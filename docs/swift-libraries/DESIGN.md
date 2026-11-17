# Swift 原生库第一阶段设计

状态：实施中。设计日期：2026-11-17。2026-11-18 已接入清单解析、逻辑 native 库投影、Swift 生产内核、driver 调度、库绑定／autolink／最终链接、`.slib` 配套目录和独立链接，以及 CLI／app 的 resource bundle 部署。AArch64 静态库已使用 CoreText bridge 完成实际调用；可重定位动态部署和完整验收尚待完成，见[实施记录](PROGRESS.md)。

本阶段闭环为：Swift package 中已有的 `@c` 函数 → Swift 工具生产原生库 → Scoop 最终链接 → 手写 `@Extern` 调用。Swift API 的自动分析、Swift 包装代码生成和 Scoop binding 生成属于下一阶段 bindgen。

权威合同同步到[语言规范 §12.2.5](../specs/SCOOP-SPEC.md#1225-swift-包原生库第一阶段设计待实现)、[实现规范 §2.7.1](../specs/SCOOP-IMPL-SPEC.md#271-swift-包生产与更新记录设计合同)、[实现规范 §2.8.1](../specs/SCOOP-IMPL-SPEC.md#281-swift-原生库的链接与发布设计合同)和[包管理规范 §6.3](../specs/SCOOP-PACKAGE-SPEC.md#63-swift-依赖选择与锁文件设计合同)。

## 1. 范围与已有基础

首期支持 macOS 宿主为 `aarch64-apple-darwin` 和 `x86_64-apple-darwin` 构建普通机器码库，按 AArch64、x86-64 顺序验收。Swift 编译器至少支持 Swift 6.3 的正式 `@c`；SwiftPM、编译器、SDK 和资源目录使用同一选定工具链。最低部署版本采用 Scoop 本次目标配置，不能采用 Swift 工具的宿主默认值。Linux、Windows、移动平台、Universal／XCFramework 和任意 Xcode 工程构建不属于本阶段。

交付包括：

- Swift package 的选定 library product 及其真实传递依赖构建；
- 静态 `.a`、动态 `.dylib`，以及由这两类库形成的单目标 `.framework`；
- Swift 生产节点的更新判断、普通并发协调、失败不发布和向最终链接的失效传播；
- Swift autolink、runtime／compatibility library、系统库和 framework 需求；
- source Cone 的传递消费，以及已有 `.slib` 配合已生产原生库的显式链接；
- 动态库及 SwiftPM resource bundle 的本地运行与交付。

已有 C ABI、NativeSafe、函数指针和 managed callback 合同继续适用。没有 `abi = "swift"`、Swift 类型导入、新的 GC／ARC 协议、Swift 异常或协程 ABI。C/C++／Objective-C 的逐文件对象编译路径保持原职责。

2026-11-17 的本机小验证使用 Apple Swift 6.4.2 和仓库现有 release 命令：SwiftPM 显式静态、动态 products 都生成成功；Scoop 调用动态库中的 `@c` 函数，Swift 内部执行 `String(2026).utf8.count`，返回 4。静态链接先缺少 Swift 标准库符号，手工补库后遇到 SDK 的 arm64e 系统 stub 被按普通 arm64 输入选择的问题。这只证明最小动态调用可行，不代表下文构建集成或全部目标已验收。原始日志留在仓库外。

## 2. 清单与工具选择

### 2.1 Swift 生产声明

在普通 Cone 中增加 `[[native.swift]]`：

```toml
[[native.swift]]
package = "native/swiftmath"
product = "SwiftMath"
linkage = "static"
format = "library"
when = { os = "darwin" }
inputs = ["native/swiftmath/config/build.json"]
env = ["SWIFTMATH_FEATURE"]
```

| 字段 | 合同 |
| --- | --- |
| `package` | 必需，包含 `Package.swift` 的目录；按现有 native 路径规则相对 Cone 根解析，允许共用包的 `..` |
| `product` | 必需，SwiftPM 已声明的 library product 名，同时是 `@Extern(lib = ...)` 使用的逻辑库名 |
| `linkage` | 必需，`static` 或 `dynamic`，选择生产形式，不表示整个程序或 Swift runtime 的静态链接 |
| `format` | `library`（默认）或 `framework`；与 linkage 正交 |
| `bundle_id` | 仅 framework 必需，作为其 `CFBundleIdentifier`；library 禁止该字段 |
| `when` | 复用语言规范 §12.2.2 的目标条件 |
| `inputs` | 可选文件／目录路径数组，默认空；相对 Cone 根，补充清单、插件等使用的外部输入，不替换自动收集的依赖 |
| `env` | 可选环境变量名数组，默认空；记录声明所消费变量的未设置、空值或具体值 |

字段、枚举值、路径形式及条件先作普通清单检查；未匹配条目不访问包目录、不读取 Swift 输入、不定位工具。匹配非首期目标的条目给出明确目标不支持诊断。单文件模式、virtual workspace 和 workspace 字段继承不承载 Swift 生产声明。

SwiftPM product 必须显式采用与 linkage 相同的 `.static` 或 `.dynamic`。automatic library 不承诺生成独立归档／动态库，本阶段报告需要明确 product 类型；不改写第三方 `Package.swift`，不生成替代 package 或 Swift 空壳源码。framework 还要求该 product 只有一个直接 Swift target，且 target/module 名等于 product 名；其 target 可以有正常依赖。

每个选中条目向当前 Cone 的既有 native library 表投影一个要求：library/static 为 StaticArchive，library/dynamic 为 Dynamic，framework 为 Framework。同名的普通 `native.libraries` 声明只能采用相同 kind，并合并要求；不相容声明报错。无对应 Scoop extern、仅由 native 代码使用时同样有效。

package locator、附加输入、环境及 Swift 工具选项属于 driver 的生产配置。只有投影后的逻辑库要求进入 Scoop 编译输入和 `.slib`；Swift 源码、锁文件和本机构建路径不进入 Scoop 语言实体身份。

### 2.2 工具与参数

`scoop build/run/test` 接受 `--swift-toolchain <path>`，选定含 `usr/bin/swift`、`usr/bin/swiftc` 和配套资源的工具链根；省略时沿本次选定 Xcode 开发环境定位默认工具链。已有 `--native-sysroot` 的 Darwin SDK 和部署配置同时用于 Swift 目标构建。编译器至少满足包的 tools version 及 `@c` 要求；不要求其内部 LLVM 与 Scoop 的 LLVM 22.1 同版本。

首期明确选择 SwiftPM 的 Swift Build 后端（`--build-system swiftbuild`），工具链须具备该能力；输入发现和资源布局先覆盖这一实际后端，不随宿主默认值切换另一套构建输出。canonical target 映射为带部署版本的 Swift triple：`aarch64-apple-darwin` → `arm64-apple-macosx<version>`，`x86_64-apple-darwin` → `x86_64-apple-macosx<version>`。包要求的最低 macOS 版本高于本次目标时报告不相容，不能静默提高最终程序的部署基线。

工具选择文本、相关选择环境和本次目标配置进入更新键；实际定位及版本／target-info 查询只在未命中且确有需要时执行。不新增工具文件或整份 SDK 的巡检、摘要体系；原位置替换整套工具而沿用旧缓存不成为新的恢复合同。工具链切换应通过明确配置／发行身份切换，遵守实现规范 §2.7。

driver 管理 package、product、configuration、target、SDK、部署版本、scratch、输出和 jobs 参数，直接传 argv，不经过 shell。Swift flags、依赖 traits 和 package 内链接设置由 `Package.swift` 表达；本阶段不增加任意 shell hook 或通用构建任务 DSL。普通机器码是交接格式，不交换不同 LLVM 版本的 bitcode，也不引入跨语言 LTO。

## 3. 两套依赖图的职责

`scoop-pm prepare` 仍负责 Cone 图，仅解析并交付展开的 Swift 生产声明，不执行 Swift、读取 Swift 源码或求解 Swift 版本。Swift package 不伪装成 Cone，不取得 ConeIdentity，不向 `Cone.lock` 复制 Swift 依赖图。

SwiftPM 负责自己的依赖选择、target 条件、host 宏／插件、混合语言编译和包内构建顺序。`scoop` 使用一个选定 product 的生产节点封装这次构建，不把每个 Swift target 复制进 Scoop 的语言编译图。正常 build 不运行 `swift test`。

Swift 构建根使用它自己的 `Package.resolved`；作为 SwiftPM 传递依赖的包，其 lock 是否采用继续由 SwiftPM 的根规则决定。源码发布时，作为 Scoop 原生构建根的这个 lock 是普通构建输入，可以随 Swift 目录交付；它不是消费者的 `Cone.lock`。锁文件中的远端版本／revision 固定依赖选择，本地 path 依赖仍需跟踪实际输入。

PM 的共享 Git／registry 源码沿现有规则不接受构建回写。这类 Cone 中的 Swift 根若有远端依赖，须随源码提供有效 Package.resolved，构建只消费该选择；不足时要求在本地修正并发布。纯本地依赖可无 lock。Swift scratch、下载和产物放在正常构建缓存中。普通工作区／path Cone 则可由 SwiftPM 更新根 lock；此区别采用 PM 已知的来源信息，不新增文件系统来源审计或源码复制层。

普通构建优先复用有效选择，不以远端出现新版本为缓存失效条件，不在命中时联网。首次选择或清单要求变化时才交给 SwiftPM 解析。显式 `swift package update` 修改本构建根的 lock 后，下一次 Scoop build 观察更新；`scoop pm update` 仍只更新 Cone 图。

`--locked` 同时要求已有 Swift 选择满足请求，不改写它；没有需要锁定的远端依赖时允许不存在 Package.resolved。`--offline` 命中时直接复用；未命中时只允许 SwiftPM 使用已经在本地准备的来源、二进制依赖和构建工具。`--skip-update` 不是完整离线合同，适配器不得把它当作禁止下载的保证。若选定 SwiftPM 无可保证只用本地输入的调用路径，则在启动可能获取依赖的步骤前报告该离线构建不支持，不能联网完成后声称 offline。`--frozen` 合并两种约束。lock 格式错误或当前声明不满足 locked 要求，均不能因旧二进制存在而忽略。

SwiftPM 可以在后续编译失败时留下已经正确解析的 lock；Scoop 不把它当作生产成功记录。直接 Swift 命令与 Scoop 都可能修改 lock，实际 I/O 错误正常报告，不建立源码隔离副本或文件系统来源审核。

## 4. 更新记录与失效规则

### 4.1 配置键与生产结果

一个 Swift 节点的配置键由实际 package locator、product、linkage、format／bundle_id、target／部署、SDK／工具链选择、debug/release、附加输入声明、有效环境和 adapter 格式版本组成。不同 Cone 请求相同配置时共享生产；不同路径的同名包、不同目标或配置不因 product 同名而共享。

记录采用已有普通构建记录模式，保存输入 metadata、所需输出及结果描述。Swift 编译结果和 framework 组装／最终输出发布可分别复用：只切换 library/framework 交付布局，可以复用相同配置已经生成的 Swift 二进制。工作目录按 package 和实际 Swift 编译配置区分，不能让不同 target/profile 互相覆盖。

结果区分链接输入和部署输入：前者包括实际库、传递库及链接需求；后者包括要随程序发布的动态库、framework、resource bundle。只改变资源部署内容且实际库与链接需求未变时，允许复用最终机器链接结果，仅更新部署。结果标识来自正常生产时取得的数据和更新记录；命中不重新读取库计算摘要。

### 4.2 必须观察的输入

1. **解析输入**：实际使用的 Package.swift（含版本专用清单的选择候选）、根 Package.resolved 的存在状态、已使用的 SwiftPM 配置及影响选择的环境；选定传递包的清单、实际版本／revision 和本地路径。
2. **编译输入**：选定 product 的实际 Swift/C/C++ 源码、头文件、module map、必要二进制输入和资源；包含根目录外的本地 path 依赖、editable 依赖和实际选中的远端 checkout。
3. **输入集合**：源码／资源自动发现目录及实际遍历的子目录，保留已存在文件的 metadata 和目录项／目录更新信息；附加 inputs 的目录也递归覆盖实际文件和子目录。必须发现嵌套目录下的新增、删除、重命名以及缺失候选变为存在；不能只 stat 包根目录或上次已有 `.swift` 文件。目录状态变化时先重新枚举并应用已记录的选择规则，只有有效集合变化才使生产失效，排除的输出／无关文件不算源码变化。
4. **host 构建输入**：实际使用的宏、插件和生成工具源码、声明输入、配置及附加 inputs/env。host 工具使用宿主目标，产出的库仍使用所选 target。

自动记录使用 SwiftPM 的包／target 信息及真实构建计划、编译依赖输出。`show-dependencies --format json`、`describe --type json` 可提供包路径和源码发现基础，但不能代替编译器头文件依赖、宏／插件输入和资源集合。适配所选 Swift 工具的正常构建输出，不固定假设 `.build/release`、`description.json` 或某个旧 backend 的私有目录恒定存在。

中间 `.build`、scratch、日志和本节点输出不作为自身源码输入；选中的远端 checkout 即使物理位于 scratch 下，仍按其真实依赖角色观察。只收集本次生产相关的输入，不因无关 Swift 测试或未选 product 的源码改变而重建当前 product。

上述排除以独立输入文件和选定 target／product 为粒度。已经选中的 Swift 文件内即使只改未激活的 `#if` 分支，或在已使用的 Package.swift 中只改其他 target 的声明，仍属于该文件变化，须重新进入 SwiftPM；不为避免这次调用另建 Swift 语义比较器。旧自动发现文件消失时重新发现并交给 SwiftPM 更新计划；仍被显式声明或实际依赖的输入缺失才按正常缺失输入报错。

Package.swift 和插件可以执行代码。它们读取的包外文件、非自动发现目录及业务环境必须由正常依赖输出或 `inputs`／`env` 声明覆盖；声明者承担依赖完整性的责任，Scoop 不声称能静态推导任意 Swift 代码的文件访问。build command 的声明输入输出可以直接采用；prebuild command 若依赖未列出的外部状态，必须补充声明。要求每次执行、输出又没有可表达更新输入的 prebuild 不在本阶段缓存生产合同内，给出不支持诊断。不得用每次启动 SwiftPM 的“空构建”代替命中判断。[SwiftPM 插件合同](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0303-swiftpm-extensible-build-tools.md)

### 4.3 命中、失效与复用

先按有效配置找到记录，再检查已记录输入的修改时间／大小、目录选择状态及所需输出存在性。未变化直接返回已保存的结果描述；此时不读取源码正文、不重新执行清单、不调用 Swift／xcrun／Xcode、不复制输入或输出、不读取命中库的内容、符号或摘要。framework 的 required outputs 包括实际二进制、Info.plist 和已交付的模块／资源文件，不能仅凭外层目录存在就算结果存在。

| 变化 | 必须更新的部分 |
| --- | --- |
| package 清单、有效解析配置、lock 的选择变化 | SwiftPM 解析／计划及受影响的 product；然后更新链接／部署结果 |
| 本地或传递依赖源码／头文件修改，源码增删 | 当前 Swift product；最终链接消费新的原生结果 |
| 资源、生成器输入或声明环境变化 | 相应 Swift 生产；按实际二进制和资源输出更新链接／部署 |
| target、部署版本、profile、工具链选择变化 | 使用另一配置记录，不能复用不相容二进制 |
| 所需 `.a`／`.dylib`／framework 文件或资源输出缺失 | 重新生产／发布缺失结果；不把 SwiftPM 的临时目录当成唯一交付 |
| 只有 Scoop 源码／手写 extern 声明变化 | 复用 Swift 结果，更新相应 Scoop 编译与链接 |
| 只有未选测试／target／product 或未匹配平台条目的独立文件变化 | 当前生产结果保持；已选文件或 Package.swift 内部修改仍使其失效 |
| 只有远端存在新版本 | 当前锁定选择保持，不访问远端 freshness |

解析图没有变化时，不重新选择已满足要求的远端版本；SwiftPM 可在失效后的实际调用中复用自己的计划和对象缓存。Scoop 负责决定是否需要调用，SwiftPM 负责调用内部的增量粒度。

遵守实现规范已有的普通输入变化合同：不建立构建期间文件替换检测／重试、源码快照、产物损坏恢复或通用计费机制。实际缺失输入报错；不能把旧成功结果返回给本次已失败的请求。

## 5. 调度、并发和发布顺序

完整顺序为：

1. PM 完成 Cone 图及清单展开，driver 形成 Scoop 编译节点和选定 Swift product 节点。
2. 各节点先做自己的更新检查。Scoop 编译只依赖投影后的 native 库要求，手写 `@Extern` 不要求先读 Swift 二进制，因此两类节点可以独立就绪。
3. Swift 命中交付已有结果；未命中取得普通构建锁并再次检查，再进行 SwiftPM 解析／构建和必要的 framework 组装。
4. 汇总根及所有 source Cone 所需的 Swift 结果，将每个逻辑库绑定到本次实际产生的原生文件；所有这些节点完成后，才判断最终程序是否可复用。
5. 最终链接未命中时读取 `.slib` Link 闭包和所需原生输入，解析 Swift 及其他 native 依赖，生成现有 Scoop startup/image tables，调用目标 linker。
6. 成功后发布最终程序及必需的动态库／framework／资源，保存更新记录。`run` 只执行本次成功结果。

Swift 外部构建加入现有 ready queue 和 `--jobs` 调度；一次 Swift 构建占用父调度任务，并将分配给它的正常并行度传给 SwiftPM，不能让每个子进程各自无限扩张 worker 数。这只管理构建并发，不引入语言／产物资源预算。

相同 package 的可写 SwiftPM workspace／Package.resolved 采用普通包级锁，同一生产配置另按已有结果锁共享。不同 scratch 使用者也须协调同一个根 lock 的写入；等待者释放调度执行位，获得锁后重查。package 只在同一节点内部由 SwiftPM 递归处理，避免父调度持锁等待自身其他 product 节点。

SwiftPM scratch 是可变工作状态，不直接作为已发布产物。未命中时可增量更新 scratch；构建及必要组装成功后，将所需结果一次性发布到新的结果目录，再更新记录。旧结果目录在仍有使用者时保持，不让另一输入的构建覆盖它。失败不发布部分 framework、不推进成功记录、不继续链接旧 Swift 库。无需对完成结果做写后读回或完整语义重验。

## 6. 三种产物的生产合同

### 6.1 `.a` 与 `.dylib`

driver 以 `swift build --build-system swiftbuild --package-path … --product … --configuration … --scratch-path …` 为主要入口，传入明确的目标、SDK、部署和 jobs 配置。输出位置从所选工具的正常输出／`--show-bin-path` 得到，只在未命中时查询，不拼接旧版 SwiftPM 的固定输出路径。

`.a` 是 SwiftPM 实际生产的完整原生归档，不是把它当成一个 `.o` 传给 scoopc，也不盲目拼接所有 target 的对象。静态 product 的传递动态库、系统库、资源和 runtime 需求仍需单独交付，不能假设“静态 product 包含所有依赖”。`.dylib` 使用工具实际产物和传递装载依赖；部署阶段在发布副本上处理加载路径。

### 6.2 `.framework`

SwiftPM 的 library product 没有通用的 framework 输出开关。本阶段对 §2.1 限定的单模块 product，先取得对应 `.a`／`.dylib` 和编译器已经生成的模块文件，再执行固定的 framework 组装；这一步不生成 Swift API 包装、不重新编译源文件、不接管任意 Xcode 工程。

```text
SwiftMath.framework/
  Versions/A/SwiftMath
  Versions/A/Resources/Info.plist
  Versions/A/Modules/SwiftMath.swiftmodule/...
  Versions/Current -> A
  SwiftMath -> Versions/Current/SwiftMath
  Resources -> Versions/Current/Resources
  Modules -> Versions/Current/Modules
```

实际二进制为静态 archive 或动态 Mach-O，类型来自生产结果，不从 `.framework` 后缀猜测。Info.plist 保存给定 bundle_id、product 名、CFBundleExecutable 和 FMWK 包类别。复制 Swift 工具实际提供的模块文件，不为 C ABI 消费强制开启 library evolution 或伪造 `.swiftinterface`；多个直接模块的 product 仍可用 library 形式，但本阶段不能组装为该单模块 framework。

动态 framework 的 install name 为 `@rpath/SwiftMath.framework/Versions/A/SwiftMath`，按实际 product 替换名称。通过目标平台工具修改发布副本，处理正常加载路径和开发运行所需的 ad-hoc signing；正式应用签名由既有 bundle 流程在最后完成。静态 framework 不作为动态装载项发布到应用中。

SwiftPM resource bundle 保留实际名称和内容，作为配套资源输出；不能仅把它移进 framework 的 Resources 就假定 `Bundle.module` 能找到。首期所选 Swift Build 访问器支持主程序资源位置、所属 framework 资源位置和 CLI 主 bundle 位置：CLI 将 bundle 放在程序相邻位置，`.app` 放在 `Contents/Resources`。framework 自身目录、模块和 Info.plist 与这些资源 bundle 分开描述，静态库的资源同样需要交付。

上述访问器布局已在本机 Swift 6.4.2 的实际资源 package 输出中确认；尚不代表 Scoop 部署集成已通过。实施验收必须真正读取已部署资源，并确认没有回退到绝对 scratch 路径。其他 SwiftPM 后端的访问器可能只有不同的主 bundle 路径和构建目录回退，不能假定通用或静默切换；增加后端时需同步输入发现与资源部署合同。

## 7. `.slib` 与原生结果的交接

`PreparedNativeObjects` 继续表示 C/C++／Objective-C 的逐文件 `.o`。新增 Swift 生产结果表示逻辑 product、实际文件／framework、二进制 linkage、链接需求、资源与更新结果，driver 将其交给最终链接和部署；不塞进 scoopc 的有序 native-object 数组。

`.slib` 继续保存逻辑 native library requirements 和手写 extern 的实际 C ABI 合同，不嵌入 Swift 源码、构建配方、Package.resolved 或本机 scratch 路径。首次编译会加入 §2.1 的逻辑要求；之后仅 Swift 实现、配置、lock 或生成库内容变化，且这份投影未变时，复用原 Scoop `.slib`。Swift 结果进入原生链接／部署更新输入，不进入 HIR/MIR/LIR 语义键。

source Cone 闭包中的生产结果由 driver 自动传递，应用无须重复配置依赖 Cone 的 Swift 包。显式生产绑定直接选中该结果，不把它变成全局搜索目录后再次任选同名库。相同生产键可共用；同一最终闭包对相同逻辑库名绑定不同 product 结果时报告冲突，不能按路径顺序覆盖或按相同 bytes 静默合并。

library build 和 `--emit-slib` 同样完成声明的 Swift 生产，交付 `.slib` 及相邻 `<slib-stem>.native/`，其中保留实际库／framework、配套非系统依赖和资源。该目录是正常配套原生文件集合，不是新的语言产物容器。发布记录保存哪些文件由本次生产，命中不反复复制；只更新本次已声明输出，不清空用户目录。

配套目录内的 `native-outputs.json` 保存正常链接／部署所需描述：逻辑库名、产物种类和 linkage、目录内相对路径、不能从库文件恢复的实际链接要求，以及 resource bundle 的相对路径与部署角色。特别是静态归档未必保留 Package.swift 的 linkedLibrary／linkedFramework 设置，不能仅靠对象 autolink 重建全部需求。描述不包含 Swift 配方、源码、lock、摘要或成功凭证；只在实际消费这些链接／部署输入时读取并作普通格式检查，命中沿已记录的 metadata 复用。

发布时将实际非系统依赖落实为配套文件，SDK／工具链依赖保留正常逻辑库要求；不能把源码或 scratch 的绝对搜索位置作为可移植交付。独立链接只激活已选逻辑库对应的描述，不因目录中还有其他条目就引入它们的库或资源。

显式 artifact-only `scoop link`／`scoop-link` 消费已有 `.slib` 和调用方通过 `--library-path <slib-stem>.native` 提供的原生文件，并采用该显式目录中的 `native-outputs.json` 补充链接要求与资源部署，仍不构建 Swift 包。该描述不替换既有显式库绑定或放宽冲突规则；普通没有该文件的 library roots 按原规则使用。`.slib` 单文件不承诺包含这些原生库。原生文件自身及普通链接需求提供 autolink、目标和装载信息；必要 compatibility archive 从显式 native roots 或所选 Swift 工具链取得，缺失时报告实际库名，不回源重建。直接链接入口也接受同一 `--swift-toolchain` 资源选择，但不执行 Swift 源码编译。

构建结果的 JSON 输出增加结构化 `native_outputs`，复用上述链接／部署描述，给出实际产物、linkage、framework 二进制位置及部署资源路径；缓存命中返回相同已存描述。进程交接使用已定位路径，配套文件序列化为相对路径以支持整体搬迁。bundle 工具据此消费本次结果，文件路径只属于工具交接，不进入 ConeIdentity 或语言 ABI。

## 8. 最终链接与运行

### 8.1 Swift 原生依赖闭合

Darwin 继续使用现有 LLD 最终链接器、Scoop startup/image tables、GC metadata 和 unwind provider。Swift 编译器不取代 Scoop 的最后链接阶段。

在实际需要读取原生输入时，采集 Mach-O 对象的 `LC_LINKER_OPTION` 以及 Swift 生产提供的正常链接需求，转成已有的库／framework 要求，补齐所选对象的符号引用。选入新的 archive member 后继续处理其实际 autolink，直到所需依赖闭合；未选 member 不强制引入其专用依赖。保留 Swift metadata、初始化记录和实际 relocation 所需成员，按平台正常 archive／dead-strip 规则处理，不一律 whole-archive 所有 Swift 库。

SwiftPM 实际构建计划中的库／framework 设置、搜索位置及其他仍作用于最终 consumer 的 native 链接需求须随生产结果传递。已由动态库自身闭合的要求不重复提升为主程序直接依赖。无法在既有目标链接合同中表达的参数，或企图覆盖最终输出／target 的冲突参数，明确指出原选项并报错，不能静默丢弃。

Swift runtime 库名不写成仅有 `swiftCore` 的固定名单；Concurrency、StringProcessing、compatibility archives 等由实际输入决定。为已经要求的 Swift 库加入所选 SDK 的 `usr/lib/swift` 和所选工具链正常资源目录，遵守现有显式／系统 provider 规则。SDK 中可供 arm64 使用的 arm64e 系统 stub 必须走已有系统兼容路径；这不允许把任意 arm64e 第三方机器库当成 arm64 输入。

普通链接边界检查目标、最低系统版本、实际定义／引用与库归属，不证明外部实现的函数类型。由 LLD 消费的 autolink 与 Scoop 预解析必须采用同一已选输入，不能在最后一步按宿主搜索路径重新选库。静态 framework 内的 archive 按同一规则读取，动态 framework 按实际 Mach-O 装载依赖读取；必要的 weak-load、re-export 和 rpath 采用正常平台语义，不能无条件升级为强依赖或静默忽略。

`.a` 的静态性只属于选定 Swift product；首期 Swift runtime 采用目标支持的正常动态配置，实际需要的 compatibility archive 可静态加入。不承诺 macOS 上全静态 Swift runtime，也不修改 Scoop runtime ABI。

### 8.2 链接与部署缓存

所有 Swift 节点的更新判断先于最终结果复用。最终链接记录加入实际 Swift 二进制结果、链接需求、依赖选择及输出布局；Swift 文件路径未变不能替代生产结果的更新判断。未变化命中仍先于 Link 闭包解码、工具定位、startup 生成和 linker 调用。

程序本地输出包含需要加载的非系统 dylib／framework，保留可重定位的相对 rpath／install name 和 SwiftPM resource bundle 名称。`run` 不靠开发机器绝对 `.build` 路径或临时 `DYLD_LIBRARY_PATH` 才能成功。系统 runtime 留由目标系统提供；需要随应用交付的工具链兼容资源按实际依赖收集，不能复制整个 SDK／Swift 工具链。

库读取与最终链接成功后才发布所需文件和新记录。只缺少部署副本时可以从已有成功结果发布，不启动 Swift 编译器或重做最终链接；命中已有正确位置的输出不重读、哈希或覆盖。相同目标输出名的冲突在发布前报告，不覆盖其他 producer 的文件。

`.app` 的库路径调整、资源放置与签名复用 scoop-bundle；静态 archive／静态 framework 只参与链接，所需资源仍须部署。此处动态库指正常链接依赖，不引入 dlopen 卸载或新的 Scoop 动态模块机制。

## 9. `@c` 调用示例与 ABI 边界

Swift package：

```swift
// swift-tools-version: 6.3
import PackageDescription

let package = Package(
    name: "SwiftMath",
    platforms: [.macOS(.v14)],
    products: [.library(name: "SwiftMath", type: .static, targets: ["SwiftMath"])],
    targets: [.target(name: "SwiftMath")]
)
```

`Sources/SwiftMath/Math.swift`：

```swift
@c(swiftmath_digits)
public func digits(_ value: Int32) -> Int32 {
    Int32(String(value).utf8.count)
}
```

Scoop Cone：

```toml
schema = 1

[cone]
group = "dev.example"
name = "swift-client"
version = "0.1.0"
kind = "executable"

[[native.swift]]
package = "native/swiftmath"
product = "SwiftMath"
linkage = "static"
```

```scoop
package dev.example

@Extern(lib = "SwiftMath", name = "swiftmath_digits")
private fun digits(value: Int): Int

fun main() {
    @Unsafe { println(digits(2026)) }
}
```

预期输出 `4`。动态库将 SwiftPM product 和 linkage 同时改为 dynamic；framework 再设置 `format = "framework"` 与 `bundle_id = "dev.example.swiftmath"`，使用同一份手写声明。

`@c` 的名字是普通 C symbol，Darwin 对象前缀仍由 Scoop 目标规则添加；不把 Swift mangled symbol 当作 C 入口。Swift Int32 对应 Scoop Int，Int64 对应 Long；Swift Int 是平台字长，不能按名称映射为 Scoop Int。可用签名是 Swift `@c` 与 Scoop C-FFI-safe 分类的交集，包含支持的定宽标量、指针、C 函数指针及双方支持的导入 C aggregate；一般 Swift struct／enum／class／String／closure 不直接穿越该边界。[SE-0495](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0495-cdecl.md)

普通调用走既有 NativeSafe。Swift ARC 与 Scoop GC 各自管理对象；手写 API 对 buffer、opaque pointer 和回调的所有权负责。`throws` 在 Swift 侧按该 C API 的约定处理；不得让异常穿越 C frame。Swift 分配并不自动意味着调用满足 `@GCLeaf`，Swift 对象 release 也不自动满足 Scoop STW release hook 的限制。此阶段不为这些规则生成包装代码。

## 10. 诊断与验收

诊断按实际边界区分：清单字段／条件、package/product 选择、locked/offline、Swift 构建、缺失结果、目标／部署不相容、原生符号／库冲突和运行资源缺失。清单错误指向 native.swift 条目；Swift 编译错误保留其原路径、位置和 stderr，附所属 Cone/product；不将其伪造成 Scoop 源码错误。失败请求不运行旧程序。

| 验收项 | 必须观察的结果 |
| --- | --- |
| `.a`、`.dylib`、静态／动态 framework | 两个 macOS target 均从真实 Swift package 链接并调用 `@c`；普通用例 release |
| Swift runtime 真实引用 | 使用字符串或其他实际标准库操作，验证静态库 autolink 与系统 stub 选择 |
| Scoop library 间接消费 | 应用自动获得 source 构建的 Swift 结果；再用 `.slib`＋native roots 显式链接，覆盖配套描述中的链接设置和资源 |
| 无变化命中 | Swift 工具、scoopc、linker 均零新增调用，输出保留；源码、工具及原产物始终存在 |
| 本地传递依赖与发现集合 | 修改包外 path 依赖、头文件；新增／删除嵌套源码，真实返回值相应变化 |
| 锁定选择 | 有效 lock 不查新版本；显式更新 lock 后采用新版本；locked 拒绝所需改写 |
| 共享源码消费 | 已发布 Swift 根按随包 lock 构建，scratch／产物不写回共享来源；缺失所需选择给出修正诊断 |
| 资源与插件输入 | 真正读取 Bundle.module 资源；修改声明的生成输入／环境后结果变化；资源部署布局实际运行 |
| 失效分层 | 只改 Swift 实现不启动 scoopc；只改 Scoop 实现不启动 Swift；未选独立输入不失效，已选文件内部变化仍更新 |
| 并发与失败 | 同配置只生产一次；不同配置不覆盖；Swift 失败不发布、不回退执行旧结果 |
| 普通离线行为 | 命中零工具／网络；本地可执行的失效构建不发请求；不能保证离线的路径在获取前明确失败 |
| 动态部署 | 在新的正常输出位置发布并运行，原输入保持在位；检查实际装载位置及 Bundle.module.bundleURL，确认使用已部署库与资源 |
| ABI 错误与边界 | 代表性的非法 C ABI 声明、缺符号、wrong target、最低系统版本和同名 producer 冲突 |

构建调度、缓存、产物、链接和诊断使用少量专门 CLI fixtures；普通调用行为优先放测试 Cone。不会为每个 Swift API 复制冷／热构建 fixture，也不把 GC stress 扩大成普通验收模式。已有移动 GC／callback 合同只在本阶段确实改变其实现时增加对应定向回归。

## 11. 实施顺序与材料同步

1. 实现清单及 PM 展开字段，明确 driver-only 配置与编译投影；更新实际跨进程协议版本，旧未发布缓存自然失效，不预分配未来 wire tag。
2. 接入 Swift product 更新记录与调度，先用 `.a`／`.dylib` 真实生产闭合输入收集、文件增删和并发。
3. 完成 autolink、Swift 系统 provider、两种 framework 二进制读取和固定组装；接通最终链接与部署缓存。
4. 完成 `.slib` 配套原生文件、artifact-only 链接、resource bundle 和 bundle 工具交接，执行第 10 节验收。
5. 按[编程材料更新工作流](../scoop-programming/UPDATE-WORKFLOW.md)同步 CLI help、projects-and-tools／language 参考、一个可运行主题、正式 fixture 和必要发行工具说明，然后才把状态改为已实现。本次设计不将未来字段写成当前 skill 能力。

下一阶段 bindgen 消费 Swift API 信息，生成 Swift `@c` 包装和 Scoop 声明。生成的 Swift 文件与生成器真实输入接入本阶段的普通输入／生产关系；不在本阶段预建 Swift AST 导入、类型映射框架或自动所有权桥接。

## 12. 外部依据

- [SwiftPM Product](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html#product)：显式 static/dynamic 与 automatic product 的区别。
- [SwiftPM 构建命令](https://github.com/swiftlang/swift-package-manager/blob/main/Sources/Commands/SwiftBuildCommand.swift)：选定 product、输出位置查询及实际 build system。
- [SwiftPM 命令选项](https://github.com/swiftlang/swift-package-manager/blob/main/Sources/CoreCommands/Options.swift)：scratch、resolved、skip-update、目标及工具链选项；不据 skip-update 宣称完全离线。
- [Apple framework／XCFramework 文档](https://developer.apple.com/documentation/xcode/creating-a-multi-platform-binary-framework-bundle)：framework 的静态／动态属性与平台产物；本阶段不实现 XCFramework。
- [Swift library evolution](https://www.swift.org/blog/library-evolution/)：Swift 模块／二进制分发与 C ABI 导出的区别；本阶段不要求所有包开启 library evolution。
