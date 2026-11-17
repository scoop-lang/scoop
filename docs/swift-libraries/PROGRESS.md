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

## 后续工作与验收

2026-11-18：app 打包保留 weak-load 的可缺省语义，缺席路径参与更新判断；可选库出现后重新打包，重定位保持弱加载。bundle 格式化与严格 clippy 通过；新增 AArch64 实机测试覆盖缺席返回 42、热复用、新库出现返回 43、整体复制运行，以及复制后仍为 LC_LOAD_WEAK_DYLIB。未运行 x86-64 用例。

1. 补齐真实宏、prebuild 诊断，并执行共享源码、锁定／离线验收；跨包 build plugin、传递生成工具及静态 binary target 交付已验证。
2. 补齐动态依赖的部署验收与 compatibility archive 组合覆盖；weak-load、re-export／rpath 的链接行为已定向通过，基础精确绑定、按成员 autolink、SDK Swift 系统库和两类 framework 已接入。
3. 动态库部署和 bundle 交接；配套目录的非系统依赖补齐实际传递加载输入。静态资源的 CLI／app 交付及静态 binary target 配套目录已完成。
4. macOS 双架构正式 CLI fixture：真实 @c 调用、传递依赖变化、命中调用计数、并发与失败、资源读取和可重定位交付。
5. 同步 CLI help、编程参考与可运行示例；完成最终合同审查。

每个大项完成后进行相关格式化、lint 与定向验证，提交、推送，再合并 origin/main。原始日志和探针保留在仓库外；每轮清理不再使用的临时文件。此文件只记录范围、结果和未闭合事项。
