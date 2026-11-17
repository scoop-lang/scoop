# Swift 原生库实施记录

分支：`codex/swift-native-libraries`。平台范围：macOS AArch64 与 x86-64。bindgen 不属于本轮。

## 当前进度

- 设计与四份相关规范已提交、推送，并合并检查 origin/main。
- 2026-11-18：清单、逻辑库投影与配套协议已接入。Swift 配方保留给 driver，compiler 仅消费库要求；PM 协议 9、compiler 协议 33，不改变 `.slib`／runtime ABI。
- 本项验证：cargo fmt；manifest／protocol／scoop 定向 clippy 通过（仅既有警告）；manifest 93 项和 protocol 47 项单元测试通过，包含新增的字段、framework、冲突及语义投影测试。
- 2026-11-18：接入 toolchain 的 Swift Build 生产内核。独立 metadata 记录在工具发现前复用结果；包级锁跨缓存、target 与 profile；实际构建计划及编译器 depfile 补充包描述；二进制、模块和资源分别保存，framework 使用已有二进制组装。附加 inputs 变化切换 Swift 工作状态，清单在失效调用中重新求值，避免 SwiftPM 内部增量缓存忽略包外生成输入。
- 本项验证：cargo fmt；toolchain 定向 clippy 通过（新代码无警告）；2 项定向测试通过，包括真实 Swift 生产、本地传递 Swift/C 依赖、头文件变化、嵌套新增与排除输入、资源变化保留库结果、静态 framework 复用、锁等待和编译失败后不返回旧结果。此时尚未接入正式 CLI 调度和最终链接；动态库／framework、插件与 artifact-only 交付仍需后续正式验收。

## 后续工作与验收

1. 将已有生产内核接入 driver ready queue；PM 共享源码标志与 CLI locked/offline 传递；补齐真实宏／插件和二进制输入交付，并执行正式构建缓存验收。
2. `.a`／`.dylib`／静态及动态 framework，autolink、Swift runtime、SDK 系统 stub 和精确库绑定。
3. `.slib` 配套目录、artifact-only 链接、动态库／资源部署、bundle 和 JSON 交接。
4. macOS 双架构正式 CLI fixture：真实 @c 调用、传递依赖变化、命中调用计数、并发与失败、资源读取和可重定位交付。
5. 同步 CLI help、编程参考与可运行示例；完成最终合同审查。

每个大项完成后进行相关格式化、lint 与定向验证，提交、推送，再合并 origin/main。原始日志和探针保留在仓库外；每轮清理不再使用的临时文件。此文件只记录范围、结果和未闭合事项。
