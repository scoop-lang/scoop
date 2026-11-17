# Swift 原生库实施记录

分支：`codex/swift-native-libraries`。平台范围：macOS AArch64 与 x86-64。按最新安排，先完成 AArch64 的全部能力，再统一验证 x86-64。bindgen 不属于本轮。

## 当前进度

- 设计与四份相关规范已提交、推送，并合并检查 origin/main。
- 2026-11-18：清单、逻辑库投影与配套协议已接入。Swift 配方保留给 driver，compiler 仅消费库要求；PM 协议 9、compiler 协议 33，不改变 `.slib`／runtime ABI。
- 本项验证：cargo fmt；manifest／protocol／scoop 定向 clippy 通过（仅既有警告）；manifest 93 项和 protocol 47 项单元测试通过，包含新增的字段、framework、冲突及语义投影测试。
- 2026-11-18：接入 toolchain 的 Swift Build 生产内核。独立 metadata 记录在工具发现前复用结果；包级锁跨缓存、target 与 profile；实际构建计划及编译器 depfile 补充包描述；二进制、模块和资源分别保存，framework 使用已有二进制组装。附加 inputs 变化切换 Swift 工作状态，清单在失效调用中重新求值，避免 SwiftPM 内部增量缓存忽略包外生成输入。
- 本项验证：cargo fmt；toolchain 定向 clippy 通过（新代码无警告）；2 项定向测试通过，包括真实 Swift 生产、本地传递 Swift/C 依赖、头文件变化、嵌套新增与排除输入、资源变化保留库结果、静态 framework 复用、锁等待和编译失败后不返回旧结果。此时尚未接入正式 CLI 调度和最终链接；动态库／framework、插件与 artifact-only 交付仍需后续正式验收。

- 2026-11-18：Swift product 加入 driver 的同一 jobs 调度，锁等待不占 worker；所有选定 product 完成后才生成 build outcome。PM 协议 10 携带共享源码标志，driver 将 CLI locked/offline/frozen 传给 Swift 生产。check/doc 保持前端职责。
- 本项验证：定向 clippy 通过（新代码无警告）；7 项并发调度测试和 2 项 PM 来源测试通过；使用本地 LLVM 22.1.8 构建配套命令后，真实 CLI 的冷构建、命中和 Swift 源码修改三轮均成功。冷构建新增 1 次 scoopc 调用，后两轮均无新增调用且 `.slib` mtime 保持，Swift 修改产生新的原生库结果。最终链接与配套目录尚未接入，不能据此宣称完整 Swift 调用交付已完成。

- 2026-11-18：接入 source 最终链接。生产结果直接绑定逻辑库，普通库搜索不替换它；选中 Mach-O 成员的 autolink 递归加入库要求，未选成员不引入依赖，LLD 使用同一输入且关闭再次自动搜库。SDK Swift stub 沿系统兼容路径解析；缺失的 Swift compatibility 库才查询所选工具链。静态 framework 按 archive 读取；最终缓存加入 Swift link key，生产记录与普通 native output 描述分开。
- 本项验证：定向 clippy 通过（新代码无警告）；1 项真实链接测试覆盖精确绑定、静态 framework、后续成员引入库及未选依赖排除；2 项 Swift 生产缓存测试通过。真实 CLI 的 `.a`、`.dylib`、静态／动态 framework 均调用 `@c`，Swift 内部使用 String；静态 framework release 热构建与 Swift 修改两轮无新增 scoopc，修改后程序更新且返回新结果。并发测试共享记录造成的初次调用计数干扰已通过单独复核闭合。此时动态程序仍使用缓存库位置，不能据此宣称可重定位部署完成。

- 2026-11-18：library／emit-slib 发布相邻 `.native` 目录及普通链接描述，独立链接只启用实际选定的库；路径按相对 HostPathCarrier 保存，目录整体复制后可消费。发布命中不重写文件，资源更新不重复复制未变化的库，淘汰文件只限上次记录的生产结果；构建／链接 JSON 返回结构化 `native_outputs`。
- 本项验证：定向 clippy 和 Swift 生产／发布测试通过，包含相对路径、热发布、资源单独变化、framework 布局复用和保留用户文件；链接测试覆盖未选描述不引入依赖。正式 [CoreText staticlib fixture](../../tests/fixtures/compiler-correctness/native-link/swift-system-framework/fixture.toml) 在 AArch64 通过 10 个进程步骤：独立 bridge Cone 导出字体大小和 glyph 数量，应用通过 `@Extern` 调用；Swift 修改无需重编 Scoop；复制 `.slib`＋`.a` 后 `scoop link` 和 `scoop-link` 均可运行。初次 fixture 的 public 声明遗漏已修正。x86-64 探针更新旧 core／runtime 后仍遇到 Swift 归档成员目标不匹配，留待 AArch64 完成后统一处理，此处不计为通过。

- 2026-11-18：静态 Swift 库资源接入 CLI 与 app。resource bundle 保留原名，部署记录独立复用，source run/test 执行已发布路径；两个 artifact-only 链接入口同样部署资源。scoop-bundle 消费 native_outputs，并同步 PM 成员协议 10；只有实际私有动态依赖才添加 Frameworks rpath。
- 本项验证：相关 Rust 格式化／clippy 通过；Swift 生产／发布／资源热部署测试、bundle 15 项单测和 1 项 macOS 集成测试通过。AArch64 [资源 fixture](../../tests/fixtures/compiler-correctness/native-link/swift-resources/fixture.toml) 通过 10 个进程步骤，覆盖 source build/run、资源变化、整体复制运行和两个独立链接入口；Swift 函数检查 Bundle.module 位于主程序资源目录，未回退到 scratch。真实 bundle 项目热构建复用，资源修改后原 executable mtime 不变，新 app 读取新值；复制 app 后运行成功，不设置 DYLD_LIBRARY_PATH。初次 run 用例误传 -o、打包器旧协议与多余 rpath 问题已闭合。

- 2026-11-18：补齐跨包 build plugin 脚本的更新输入；生成工具及其传递源码、声明输入仍由真实构建计划收集。consumer 链接设置只沿实际产品对象／归档收集，排除宿主生成工具；配置与 traits 条件采用 SwiftPM 已解析选择。
- 本项验证：定向格式化／clippy、真实 Swift 生产缓存测试通过。AArch64 [插件 fixture](../../tests/fixtures/compiler-correctness/native-link/swift-plugins/fixture.toml) 通过 9 个进程步骤，覆盖三个本地包、插件脚本变化、生成工具的传递支持库变化、声明输入变化及 frozen 热复用；输出依次为 52、62、63、62，均无需重编 Scoop。宿主工具专用 linker flag 与 z 库未传播给最终程序。初次用例误用 clang 的 -Wl 参数已改为 Swift 的 -Xlinker。

- 2026-11-18：Mach-O 动态依赖区分普通 load、weak-load 与 re-export；弱依赖无候选时可缺席，实际候选仍检查目标与版本。缺席查找进入缓存输入，新增可选库会重新链接；link plan 升为 v8，source final cache 升为 v4。
- 本项验证：相关格式化／clippy 通过；AArch64 [weak-load fixture](../../tests/fixtures/compiler-correctness/native-link/dynamic-weak/fixture.toml) 通过 12 个进程步骤，覆盖缺席运行、热复用、实际库出现、版本不相容与强依赖缺失。已有 loader-path 与 re-export 两项 fixture 的 3 个变体、22 个进程步骤通过。初次回归遗漏 LLD 环境及复用残留 work 目录导致的准备失败均已闭合；没有开始 x86-64 验证。

- 2026-11-18：实际 ProcessXCFramework 的选中 slice 纳入更新记录，consumer 的 binary target 作为独立原生依赖快照及配套文件交付；宿主工具依赖不提升为 consumer 库。修正 Darwin archive 惰性候选处理：SwiftPM 已合入归档的二进制依赖与保留的 autolink 可以共存，两个成员实际抽取后的重复强定义仍报错。使用维护中的 plist 库读取容器描述，并同步主 workspace 与 runtime_builder 的锁文件。
- 本项验证：相关格式化／clippy、2 项真实 archive 链接测试和 6 项发行许可测试通过，runtime_builder 的 locked 依赖解析通过。AArch64 [binary target fixture](../../tests/fixtures/compiler-correctness/native-link/swift-binary/fixture.toml) 通过 14 个进程步骤，覆盖跨包静态二进制依赖、frozen 命中、替换归档后 42→43、Scoop 编译复用、配套目录复制及两个独立链接入口。另用真实 versioned 静态 framework／XCFramework 验证相同交付，两入口复制后均返回 42；输入和原产物保持存在。初次探针暴露的 autolink 重复候选及单测对象命名导致的搜索歧义已闭合。

## 最新职责收敛与验证

2026-11-18 用户决定：最终 framework／库依赖显式写在 Cone.toml；SwiftPM 负责包自身构建；私有 framework 清单独立写在 bundle metadata，rpath／加载名调整及签名仅由 bundle 在副本上完成。系统 framework 跳过。上文自动包图／target／plugin／linkerSettings 推导属于早期实现，已由这一合同取代，不再保留两套路径。

本批删除 Package.swift 描述求值、target／traits／plugin 依赖推导及 consumer 链接设置传播；包根递归观察，包外输入由 Cone 显式声明，实际 Swift Build 输出只用于定位产物。去掉无需使用的 plist 依赖并同步两个 Cargo.lock。配套描述 schema 2 仅保存实际文件／资源；Swift adapter 5、source final cache v5、link plan v9。链接器只增加正常 headerpad 预留，撤掉未提交的 CLI 动态部署尝试。

验证：相关格式化／clippy 通过（新代码无警告，bundle 严格 clippy 通过）；2 项生产／缓存测试、2 项实际链接测试通过。AArch64 四组正式 Swift fixture（CoreText、资源、显式插件输入、显式 binary 输入）全部通过，共 43 个进程步骤。新 bundle 实机测试用 Swift staticlib 调用第三方 Answer.framework 和系统 Foundation：显式 bundle.files 清单、自足已有程序入口、热复用、复制 app 后运行、相对加载路径、签名及源文件不变均通过，Frameworks 目录仅含 Answer。未运行 x86-64。初次测试保留旧 offline 冷构建用法及把等价 @loader_path 误写成 @executable_path 的断言已修正。

当前 offline／frozen 仅支持 Swift 生产命中；未命中在调用 Swift 前明确报错，因为不再分析任意清单来猜测是否会获取远端依赖。locked 仍交给 SwiftPM 的 resolved-only 模式。需要本地更新的外部目录必须显式列入 inputs。

## AArch64 收尾与发布示例

2026-11-18：新增随工具链发布的 `language/swift-bridge` 主题，完整提供 CoreText 两个函数的 Swift package、Cone 和 Scoop 调用。同步示例索引、阅读器检查、runner 与编程参考；清理规范中失效的“待实现”状态和旧链接设置描述，更新 runtime 章节引用与 PM members 协议号。

验证：格式化／lint 通过，示例阅读器 9 项测试通过；从正式 `scoop example` JSON 还原工程后运行通过。新增真实本地 Git Swift 依赖测试通过，覆盖共享源码缺少 resolved 选择失败、本地首次生成 lock、共享源码按 lock 构建、远端新增版本不失效及不满足锁定选择时失败且不改 lock。

收敛后的普通 dylib、动态 framework 和使用 Foundation `#Predicate` 宏的 staticlib，在 AArch64 冷构建、热复用与运行均通过；三种程序显式 bundle 后，热复用、复制 app 运行和签名验证通过。系统库未复制；动态产物只部署显式列出的私有文件。示例首次验证遇到开发版 scoopc 的 LLVM 加载环境问题，已使用既有测试启动器闭合；阅读器旧主题数量断言与临时验证脚本的 JSON 层级已修正。

实际 Swift compatibility archive 探针暴露了原生 Mach-O 读取器的问题：Apple 归档成员同时含 macOS 与 Mac Catalyst 的 LC_BUILD_VERSION 时被拒绝。当时未计作兼容库验证通过；修复与复测见下节。

## AArch64 验收完成

2026-11-18：按普通 Mach-O 多平台记录规则修复对象／归档及 dylib 的部署选择，消费 macOS 记录，忽略不相关的平台记录。规范先行同步；没有 macOS 或 macOS 最低版本高于当前部署仍拒绝。没有引入 compatibility 库名单或平台来源豁免。

验证：格式化和定向 clippy 通过（仅既有警告）；2 项 native object 测试通过，其中新测试用真实 clang 产出普通对象／dylib，覆盖多平台输入、仅 Catalyst 和 macOS 最低版本过高。实际 Swift staticlib 使用工具链的 runtime compatibility 选项生成真实引用，最终程序成功解析并链接 Compatibility50、51、56、Concurrency、DynamicReplacements，热复用和两个 CoreText @c 调用均通过。AArch64 本阶段能力与组合验收完成，下一批才开始 x86-64。

## 后续工作与验收

1. 统一修复和验证已记录的 x86-64 Swift target 问题。
2. 最终合同与变更审查；bindgen 留到下一阶段。

每个大项完成后定向验证，提交、推送，再合并 origin/main。原始日志在仓库外，每轮清理不再使用的临时文件。
