# Swift 原生库第一阶段设计

状态：第一阶段已完成，macOS AArch64／x86-64 验收通过。2026-11-18 按用户决定收敛职责：SwiftPM 负责构建；Cone 显式声明缓存输入和最终链接依赖；bundle metadata 独立声明私有 framework。只在 bundle 阶段重定位动态库。验收按 AArch64 → x86-64 顺序完成。范围与结果见[实施记录](PROGRESS.md)。

本阶段优先实现系统／第三方 framework 的手写 Swift staticlib bridge：`@c` → `.a` → Scoop `@Extern` → 最终程序。bindgen、自动包装和 Swift 类型导入留到下一阶段。

权威合同见语言规范 §12.2.5、实现规范 §2.7.1／§2.8.1、包管理规范 §6.3／§11.3。

## 1. 范围

macOS 宿主，为现有 AArch64 和 x86-64 Darwin target 构建。Swift 工具链支持正式 `@c`（Swift 6.3+）和 Swift Build 后端。交付普通 `.a`／`.dylib`，或以它们组装单目标 framework。Swift 内部 LLVM 可以不同于 Scoop 的 LLVM 22.1，只交接机器码。

重点是 staticlib bridge；`.a` 不表示 Swift runtime 或最终程序全静态。SwiftPM 自己处理包内 target、远端／本地包、宏、插件、二进制输入和内部链接。Scoop 不复制这套依赖图。

## 2. Cone 声明

```toml
[[native.swift]]
package = "native/SystemBridge"
product = "SystemBridge"
linkage = "static"
inputs = ["native/BridgeSupport", "vendor/Acme.framework"]
env = ["BRIDGE_FEATURE"]
when = { os = "darwin" }

[[native.libraries]]
name = "CoreText"
kind = "framework"
when = { os = "darwin" }

[[native.libraries]]
name = "Acme"
kind = "framework"
when = { os = "darwin" }
```

`package`、`inputs` 相对 Cone 根解析。product 对应 SwiftPM 显式 static/dynamic library。`format = "framework"` 时声明 `bundle_id`，并要求实际构建产出同名 Swift 模块；默认 format 为 library。

`native.swift` 是生产配方；`native.libraries` 是 Cone 的最终链接需求。Swift 包自己的 linkerSettings 不自动转为 Cone 需求。第三方库位置沿现有 `--library-path` 提供；系统 framework 从选定 SDK 解析。相同逻辑库与 kind 按现有规则合并，冲突报错。

仅投影后的逻辑库要求进入 Scoop 编译键与 `.slib`。Swift 源码／资源／lock／本机路径不进入语言实体身份；只改 Swift 实现时复用 Scoop 编译结果。

## 3. Swift 构建与工具选择

`--swift-toolchain` 可明确选择工具链，否则使用所选 Xcode 环境。SDK、目标架构、profile、product、scratch 和 jobs 由 driver 显式传给 `swift build --build-system swiftbuild`，架构使用 `--arch arm64`／`--arch x86_64`。本次最低系统版本传入环境，SwiftPM 仍管理包自身部署设置；实际库的最低版本须与最终目标相容，不能因此抬高最终程序基线。真实产物路径通过工具结果取得。

Package.swift 是程序，由 SwiftPM 执行。Scoop 不解析它，也不调用 dump-package、describe、show-dependencies 推导缓存或链接依赖。构建失败按实际工具诊断返回；缺少要求的独立 `.a`／`.dylib` 或 framework 模块时报告所需产物。

实际 Swift Build 输出描述只用于定位产物、模块、resource bundle 和同次生成的配套原生文件，不推导 consumer 的 flags 或 target 图。

## 4. 缓存输入与失效

包根递归参与更新判断，覆盖源码、资源、清单、根 Package.resolved 和包内生成器输入；忽略 `.git`、`.build` 和本次构建缓存目录。采用整包粒度，不重建 SwiftPM 的 source/exclude 或平台选择规则；包内未选 target／测试变化也可使生产失效。

包外本地／editable 依赖、第三方 framework、宏／插件使用的额外目录和文件必须在 Cone.toml 的 inputs 中列出；env 列出消费的环境。目录递归跟踪嵌套增删、删除和重命名。Package.resolved 固定远端版本；本地修改 checkout 也要显式列入 inputs。未声明的包外输入不承诺自动发现。

记录保存文件 metadata、目录选择和所需结果路径。检查顺序为配置与输入更新 → 结果存在性 → 命中直接返回；命中不读源码正文、不执行清单、不探测工具、不复制结果。未命中才启动 Swift。附加 inputs 变化切换工作状态，保证清单／生成器重新消费它们；普通包内修改交给 SwiftPM 增量构建。

资源结果与机器库结果分别保存。只改资源且库输出未变时更新资源部署，不重编 Scoop、不重新链接。

## 5. 依赖选择与 lock

Cone.lock 只管理 Cone 图，Package.resolved 只管理 Swift 图。lock 的缺席、出现、变化进入更新输入；不因远端新版本而失效，不在命中时联网。

`--locked` 和共享 Git／registry 源码使用 SwiftPM 的 resolved-only 模式；纯本地依赖允许无 lock。普通本地包可由 SwiftPM 更新 lock；共享源码不能补写缺失选择，需作者先准备并发布。

`--offline`／`--frozen` 允许复用命中结果。当前 SwiftPM 没有完整的禁止获取依赖模式，未命中时明确拒绝离线构建；不通过分析任意清单程序推断其不会联网。`--skip-update` 不等于 offline。

## 6. 调度与失败

Swift product 是 driver 同一 jobs 调度中的独立节点。相同生产键共享，包级锁协调可更新同一个 Package.resolved 的请求；等锁不占 worker。SwiftPM 管理内部并行，不将每个 Swift target 拆成 Scoop 节点。

Scoop 源码编译与 Swift 生产可以独立调度；最终链接缓存检查晚于全部所需 Swift 节点的更新判断。失败停止 dependent，不返回旧结果作为本次成功，不发布不完整产物。

## 7. 静态库与 framework

`.a` 使用 Swift 工具的真实完整归档。最终链接按实际未定义符号抽取成员，再处理这些成员的 autolink；未选成员不引入专用依赖，不默认 whole-archive。显式 Cone framework 要求与机器码 autolink 使用同一普通 native 解析流程。

静态 framework 按内部 archive 链接。动态 framework 是普通 Mach-O 容器；生产其固定目录结构时设置与自身布局一致的 install name。资源 bundle 保留 SwiftPM 名字。C ABI 仍受现有 NativeSafe、对象生命周期及 callback 约束，不引入 Swift ABI 或 GC／ARC 协议。

## 8. 最终链接与运行

链接器保持正常职责：解析显式逻辑库与实际对象引用，选择兼容 target 的 archive／framework／动态库，调用现有 LLD。Swift runtime、系统库和 compatibility archives 按真实引用选取；不维护只含 swiftCore 的固定清单。

开发态 build/run/test 使用已解析的位置及正常加载路径，方便测试调试。链接阶段不复制私有 framework，不调整部署路径或重签第三方库。Darwin 输出预留 `-headerpad_max_install_names`，便于后续 bundle 修改 load commands。

资源按已有 CLI 相邻目录部署，run/test 执行已发布路径。系统 framework 固定由系统装载。

## 9. 发布与 bundle metadata

library build／emit-slib 发布 `.slib` 和相邻 `<stem>.native/`，保存 Swift 已产出的原生文件、资源及相对路径描述。独立链接只消费它们和显式库目录，不构建 Swift。Cone 的链接要求已经在 `.slib` 中，不在 Swift 产品描述中复制一套要求。

私有 framework 的打包清单直接写在 bundle metadata，复用已有文件映射：

```toml
[[bundle.files]]
from = "vendor/Acme.framework"
to = "Acme.framework"
kind = "framework"
when = { os = "darwin" }
```

外部 Bundle.toml 同样使用 `[[bundle.files]]`。列表只包含要随应用交付的私有代码；不读取 Cone native metadata 或 Package.swift 来推导。系统 framework 不列入清单，bundle 跳过它们。

bundle 复制私有 framework 到 `Contents/Frameworks`，保留版本目录和 symlink；按实际 Mach-O 加载信息改写副本的 rpath、install name 和私有依赖路径，保持 weak-load／re-export 种类，再由内向外签名。资源放入 `Contents/Resources`。这些输入与所需输出纳入 bundle 的已有更新记录，命中直接复用。原始编译结果、第三方输入及系统 framework 保持原样。

## 10. 诊断与验收

验收按 AArch64 → x86-64 顺序完成，定向覆盖：

- CoreText handwritten staticlib bridge 两个 @c 函数，经依赖 Cone 的 public @Extern 调用；
- 冷构建、无工具调用命中、Swift 修改复用 Scoop、显式本地依赖／头文件／资源／插件输入变化；
- 缺失声明输入、真实编译失败不发布、普通锁竞争；
- `.slib` 配套目录复制与两个独立链接入口；
- 第三方动态 framework 明确列于 bundle metadata，复制 app 后运行，路径修改及签名；系统 framework 不复制；
- 所选成员 autolink、静态 framework、普通 dylib 和必要 compatibility archive 组合。

保持源码、工具和原产物在位；不通过隐藏它们证明复用。原始日志放仓库外，进度记录只保留范围、结果和限制。

## 11. 材料同步

按[编程材料工作流](../scoop-programming/UPDATE-WORKFLOW.md)同步 CLI／编程参考、一个可运行 bridge 示例、正式 fixture 和 bundle 说明。每项完成后定向验证，提交、推送，再合并 origin/main。bindgen 不属于本阶段。

## 12. 外部依据

- [SwiftPM Product](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html#product)：显式 static/dynamic product。
- [SwiftPM 命令选项](https://github.com/swiftlang/swift-package-manager/blob/main/Sources/CoreCommands/Options.swift)：resolved-only、scratch、目标和工具链选择。
- [Apple install_name_tool](https://github.com/apple-oss-distributions/cctools/blob/main/misc/install_name_tool.c)：链接后修改加载命令，以及 headerpad 空间要求。
