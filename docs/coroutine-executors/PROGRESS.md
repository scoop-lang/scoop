# 实施进度

分支：`codex/coroutine-executors`。设计：[DESIGN.md](DESIGN.md)。

| 大项 | 状态 |
| --- | --- |
| lambda 按期望类型推导挂起性、对称重载消歧、跨 Cone 定型 | 已完成本地验收 |
| core executor 协议、TaskContext、驱动边界、编译器挂起/恢复握手 | 实施中 |
| Channel、AsyncMutex、AsyncConditionVariable | 未开始 |
| MultiThreadExecutor、通用 managed BlockingTaskPool | 未开始 |
| asyncio 接入、AsyncRuntime、三平台 native work 迁移与 API 收敛 | 未开始 |
| Linux/Windows 组合验证及最终清理 | 未开始 |

每个完成的大项先本地验证、提交并推送，再合并 `origin/main`；跨平台验证集中到完整能力闭环后进行。未完成项不代表已可使用的 API。

## Lambda 期望挂起性（2026-11-04）

- 未显式标记的 lambda 按期望函数类型检查挂起性；没有期望时仍为 ordinary。显式匿名函数、函数引用和已定型函数值保留效果边界。编辑器恢复使用同一规则。
- 对称重载以声明类型及精确 binder 重命名比较，复用既有最具体候选规则；多 lambda 按位置偏序选择。实现覆盖本地/导入 callable、成员、构造器和成员要求，不按当前推导结果猜选。
- 规范与 API 风格已同步；AST 标记仍记录源码显式性，成功 HIR 的效果已确定，MIR/LIR、产物格式及 runtime ABI 没有新增动态效果机制。
- 验证：`cargo fmt --all`、workspace/all-targets Clippy 通过；HIR 全量运行覆盖 2144 项，其中 6 项旧诊断断言的问题修复后，69 项相关回归全部通过。10 个定向 CLI fixture 均通过，包含真实挂起/恢复、候选歧义诊断及跨 Cone 泛型模板。platform 的 5 个资源作用域测试通过，覆盖多次挂起、失败恢复、清理异常及 ordinary 重载。
- 本轮运行平台为 Darwin AArch64，Scoop 程序使用 release；Linux/Windows 验收随执行器闭环集中进行。Rust 测试及 CLI 链接保留工具链已有的巨大 `__eh_frame` 警告，测试运行未受影响。

## TaskContext 执行归属（2026-11-04）

- 完成 execution 的独立 managed 字段、生成字段身份与精确扫描；TaskContext 由 24 bytes 扩为 32 bytes，runtime ABI contract 更新为 33，metadata 记录格式保持 10。
- typed fork 支持显式 execution 或独立入口；读取当前 execution 先判空，普通 binding snapshot/push/restore 保持原有职责。多操作数的访问、活跃度和 CFG 降低已同步。
- 定向验证覆盖 debug/release 的分配后引用恢复、空入口判空，以及真实移动 GC 下父子任务的 binding/execution 保活。相关 Clippy 与 runtime/codegen Context 测试通过。
- 这一子项建立表示与 GC 基础；协程入口、恢复 work 与 core 驱动状态仍在接入，尚不表示多 worker executor 已可用。
