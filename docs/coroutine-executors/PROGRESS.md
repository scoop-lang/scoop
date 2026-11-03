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

## 执行协议与恢复驱动（2026-11-04）

- core 提供 CoroutineWork、BlockingWork、CoroutineExecutor、ExecutorTask 及内部 execution 驱动；startCoroutine 继承当前 executor 并立即开始，startDispatchedCoroutine 登记后排队。完成通知及驱动退出后释放登记，回调抛出也只释放一次。
- direct、virtual、closure 和原语挂起统一使用 Registering/Waiting 握手与成功/失败 latch；公开恢复先提交结果，再交给 continuation 保存的 execution。内部完成链迭代消费，外部恢复回到 executor 队列，驱动交接前完成 Context 和异常清理。
- 编译器生成具有真实 managed 字段的 StartWork 和 adapter run 方法。跨 Cone 普通协议方法引用原定义，不重新单态化；仅实际协程选择这些依赖。对应 HIR/MIR/LIR 产物能力版本已同步，runtime ABI 保持 33。
- 验证：Rust 格式化及受影响 crate 的 all-targets Clippy 通过；identity 361 项、core surface 10 项、MIR adapter 4 项和 coroutine 14 项、MIR lowering 22 项、HIR coroutine 3 项及 ordinary-core 导入 35 项通过。11 个 CLI fixture 覆盖同步/异步恢复、异常、泛型/成员、Context/外部线程、GC 根及跨 Cone 导出，全部通过。
- 新增 5 项排队 executor 行为测试通过；其中挂起恢复、启动 work 和上下文保活用例单独开启移动 GC／minor GC stress 后通过。既有 nonCancellableScope 的取消与清理组合回归通过。本轮在 Darwin AArch64 release 验证。
- 强制 yield、receiver scope、withExecutor、通用阻塞任务入口和实际 executor 仍待接入；上述验证使用手动排队的测试 executor。

## 强制 yield（2026-11-04）

- 实现 yieldCoroutine：有 executor 时退出当前驱动并重新排队，无 executor 时直接返回，前后均检查取消。内部 Yield 节点独立于 registration，不被同步恢复锁存或协程尾转交优化消去。
- 复用 managed adapter、保存的 TaskContext 和 execution 门；Wait 状态下提交 Unit，所有调用层退出后再发布下一驱动。注册握手代码拆为独立模块，普通挂起行为保持同一实现。
- 新 intrinsic 使用 tag 37，HIR core-bootstrap-interface／cross-cone-interface 更新为 22／92；配套 CLI 与 core/std/platform/test 产物已重建，MIR/LIR 和 runtime ABI 无新增变更。
- 验证：格式化、受影响 crate 的 all-targets Clippy 通过；23 项协程 lowering、27 项 intrinsic 和 10 项 core surface 测试通过。4 项 yield 行为及既有 5 项 executor 驱动回归通过，覆盖排队、Context、无 executor、取消和 catch/finally 挂起。排队与 Context 用例在移动 GC／minor GC stress 下单独通过，平台为 Darwin AArch64 release。
- 下一步为 receiver scope、withExecutor 和通用阻塞任务入口；多 worker 及其他平台验收仍未开始。
