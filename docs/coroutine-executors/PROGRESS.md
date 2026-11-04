# 实施进度

分支：`codex/coroutine-executors`。设计：[DESIGN.md](DESIGN.md)。

| 大项 | 状态 |
| --- | --- |
| lambda 按期望类型推导挂起性、对称重载消歧、跨 Cone 定型 | 已完成本地验收 |
| core executor 协议、TaskContext、驱动边界、挂起/恢复、scope 与 blocking 入口 | 已完成本地验收 |
| Channel、AsyncMutex、AsyncConditionVariable | 已完成本地验收 |
| MultiThreadExecutor、通用 managed BlockingTaskPool | 已完成本地验收 |
| LoopExecutor、eventLoop/run 入口与调用方迁移 | 已完成库内和 consumer 本地验收 |
| AsyncRuntime 共享池与 worker 根入口 | 已完成库内和 consumer 本地验收 |
| 三平台 native work 迁移与 TLS 接入 | 已完成 Darwin 本地验收，其他平台待运行 |
| HTTP 等上层消费者审计与验收 | 未开始 |
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

## Receiver scope 与 withExecutor（2026-11-04）

- coroutineScope、nonCancellableScope、launch 和 async 改用 CoroutineScope receiver。每个子任务获得自己的 scope，保存 Job、默认 executor 与正文活动期间的父执行登记；捕获 scope 从另一个 executor 调用时仍使用原默认 executor。显式 executor 重载保留原 Job 父子关系，关闭后拒绝启动。
- 实现 withExecutor：即使目标与当前 executor 相同也排队，目标子树完成后才返回原 execution；目标失败在调用边界抛出，不提前取消可捕获该失败的 caller。登记失败沿 Job 完成路径结束已 attach 的 child。
- 迁移 asyncio、HTTP、压缩、窗口和 UI 的调用方，显式限定原有外层 receiver。ExecutorClosedException 的构造器设为 public。原 5 组普通结构化并发 fixture 移入 core consumer 与 platform Cone tests；驱动、yield 和 scope 测试按行为拆分并共享排队 executor。
- 验证：Scoop 格式化、diff 检查通过；15 项 executor/receiver/yield、8 项结构化行为、1 项非取消清理及 1 项真实线程竞争测试通过。全部系统库和 asyncio 测试程序 release 构建通过；HTTP asyncio/client/server、窗口 asyncio、Surface asyncio、无窗口宿主、下载示例的应用与测试源码，以及准备后的 SSE consumer 源码检查通过。
- 本轮在 Darwin AArch64 release 验证；Windows 专有调用方已同步 receiver 写法，实际运行随两类 worker 闭环集中验收。

## 通用 blocking 入口（2026-11-04）

- core 实现 blocking 与 CoroutineExecutor.blocking，普通闭包及返回值不受 GC-free 限制。复用 OneShot foreignCallback 捕获 binding 快照，运行时不继承 caller execution；callback 和 Context 出口结束后才通过保存的 continuation 交付 typed 结果或异常。
- 排队取消与 worker 领取由同一状态锁仲裁。取消先胜时立即释放 callback、用户捕获及 continuation，空队列工作保留必要登记；登记在队列退休与取消通知两者完成后释放。已经运行的闭包等待真实结束，运行中或结果发布后的取消不会覆盖其实际结果。
- 修复导入泛型模板中 callback 注册／操作缺失隐式协议类型的问题：在 HIR 模板物化边界按已有 core 身份请求模式、状态与 Option<Throwable>，复用类型缓存。实现规范已同步；产物格式、runtime ABI 与 callback ownership 协议不变。
- 验证：格式化、HIR lowering all-targets Clippy、5 项相关导入／native-call 测试通过；core/std/test release 构建及 27 项 executor 行为测试通过，其中新增 12 项 blocking 测试。托管结构体结果／Context 与异常交付两项用例在移动 GC／minor GC stress 下通过，平台为 Darwin AArch64 release。
- 当前验证使用可控排队 executor，覆盖同步取消安装、出队与通知交错、成功资源与失败身份、跨 executor 返回、捕获 scope 发起 child 以及 completion 抛出后的退休。实际 managed blocking pool 和多 worker 组合仍待后续实现。

## Channel 与异步同步原语（2026-11-04）

- 新增普通 Cone scoop.concurrent 与[并发库规范](../specs/SCOOP-CONCURRENT-SPEC.md)。Channel 提供会合／有限 FIFO 缓冲、非挂起尝试、关闭排空和可空消息；AsyncMutex 提供 FIFO 直接交接、共享 guard 状态与普通／挂起 receiver 的 withLock；AsyncConditionVariable 原子释放并在通知或取消后重新取得同一个 guard。
- 三个原语的等待、取消和所有权交付由各自同一状态锁仲裁，原始 continuation 保存实际 executor；锁外解除订阅并交付全部选中结果。内部可摘除链表用于实际等待队列，消息缓冲使用 ArrayDeque，终态解除 payload、continuation、订阅与节点链接。
- 验证：Scoop 格式化及源码检查通过；29 项 release Cone tests 全部通过，涵盖跨 executor／Context 恢复、关闭与 FIFO、取消和提交先后、错误 guard、异常通知、真实线程 MPMC，以及取消与发送、解锁、条件通知各 24 轮竞争。含引用的 struct／tuple 消息与跨 executor mutex／condition 两项用例在移动 GC／minor GC stress 下通过。
- 本轮在 Darwin AArch64 验证；普通线程行为与可控排队 executor 已覆盖，多 worker executor 的组合和 Linux／Windows 验收按后续大项集中进行。

## 多 worker 与通用阻塞池（2026-11-04）

- 实现 multiThreadExecutor receiver builder、MultiThreadExecutor 与 BlockingTaskPool。两类池使用固定 managed workers，复用受短 Mutex／ConditionVariable 保护的 FIFO managed work 队列；每项工作在锁外执行，coroutine 与 blocking 使用独立线程组。builder 执行一次并保存配置快照，支持自有或共享阻塞池。
- executor 在 Draining 拒绝新根、接纳活动同 executor 父登记的子任务，活动登记排空后才停止内部队列。blocking 从投递计数到实际 runner 退出；正式终止还等待 coroutine workers 和自有池退出执行循环。awaitTermination 可取消但不隐含 OS join；join 回收线程句柄，状态同步对象保留到 GC，支持并发与重复关闭。物理 worker 和活动 execution 都参与自等待检查。
- 未捕获 work／completion 异常在 worker 边界打印栈并以状态 1 退出；普通 blocking 闭包失败继续由 core 交付。部分创建失败的已建线程按关闭／join 回收。共享池 owner 负责在借用方关闭后关闭池，借用方不会关闭或 join 该池。
- 验证：格式化及源码检查、release 库构建通过；52 项 Cone tests 全部通过（本轮新增 23 项），覆盖真实并行、yield、跨 executor 同步、binding 与 Job、阻塞成功资源／异常／取消、排空中的子任务、共享池、builder 快照、自等待和并发关闭。3 项真实 worker 的托管阻塞结果、运行中取消清理、跨 executor Channel／mutex／condition 在移动 GC／minor GC stress 下通过。
- 新增一个正式 CLI fixture，消费预编译 concurrent 库并验证 queued completion 抛出后的进程退出码与栈诊断，通过。没有 runtime ABI、产物格式或 Rust 编译器改动。本轮平台为 Darwin AArch64；下一步为 asyncio 的 executor 接入、AsyncRuntime 和三平台 native work 迁移，然后集中验证 Linux／Windows。

## LoopExecutor 与事件循环入口（2026-11-04）

- BlockingTaskPool 与 MultiThreadExecutor 增加终止状态和一次性的普通终止通知；通知复用状态锁选定，在锁外执行，不继承登记方的 execution。解除订阅能撤销已选中但尚未开始的通知。新增 3 项组合测试通过，并发库共 55 项常规测试通过。
- eventLoop receiver builder 保存自有／共享阻塞池配置快照，run／attach 接受入口后才创建资源。PlatformEventLoop 的泛型 final run 使用 CoroutineScope receiver 并直接返回 T；EventLoop 接口保留服务和 executor 属性，移除旧构造调用、接口 run 与 runWithResult。仓库调用方和当前规范示例已迁移。
- LoopExecutor 排队驱动并维持任务登记及工作计数；任意真实挂起的恢复回 owner，显式 worker I/O 保留 EventLoop binding 并回调用者 executor。根结束后先排空登记与工作，再关闭遗留资源、排空原生操作，最后终止并 join 自有 managed blocking pool；借用池由其 owner 关闭。
- attach 不再额外唤醒空 mailbox；启动 work 本身负责通知宿主。普通完成只在进入最终终止时追加唤醒。已有时序测试通过明确交接，让被测子任务先提交请求，再检查占用或取消；保留真实 syscall、实际进度、迟到资源清理和宿主空闲断言。
- 验证：格式化、源码检查和 release 库构建通过；160 项 asyncio Cone tests 全部通过，包含新增 7 项 executor／阻塞池组合以及外部恢复立即回 owner 的断言。独立 consumer 的泛型托管结果与真实 UDP I/O 测试通过。新增 7 项用例另在移动 GC／minor GC stress 下全部通过。平台为 Darwin AArch64；上层库消费者与 Linux／Windows 在 AsyncRuntime 和 native work 统一后集中验收，当前系统库中尚有需要重建的旧依赖产物。

## AsyncRuntime 共享池与关闭（2026-11-04）

- asyncRuntime receiver builder 保存 coroutine／blocking 线程数快照，接受单次 run 后才创建线程；根及默认子任务运行于 MultiThreadExecutor，调用线程驱动原生 I/O owner。worker 与 LoopExecutor 共享同一 managed BlockingTaskPool，泛型 run 在完整收尾后返回 T 或异常。
- 根结束先关闭 worker executor 的新根接纳，等待所有执行登记和驱动退出，再使 LoopExecutor draining；独立 owner 任务及遗留 I/O 资源继续完成，最后停止共享阻塞池并 join 两组线程。初始化失败逆序回收；被拒绝的嵌套启动不消费实例。
- 验证：格式化、源码检查、release 库构建通过；165 项 asyncio Cone tests 全部通过。新增 5 项覆盖线程归属、binding、共享单线程阻塞池、真实文件／目录／DNS／UDP、资源自动关闭、异常 finally、独立登记排空及 builder 快照，并在移动 GC／minor GC stress 下全部通过。独立 consumer 的 2 项测试通过，包含跨产物泛型 managed 结果、worker／owner 切换与阻塞闭包。本轮平台为 Darwin AArch64；三平台旧 native work 调度仍待迁入共享池。

## 原生阻塞任务与 TLS 迁移（2026-11-04）

- 删除 Darwin／Linux／Windows 各自的阻塞 worker 调度，文件、目录、路径、DNS 和 Windows TTY 统一投递至 loop 的 managed BlockingTaskPool。原生部分保留系统调用、错误提取、取消仲裁与 owner 完成；operation 登记后立即投递，清理产生的新工作在本轮结束前继续投递。
- 独立 native task 保留已投递工作的退休责任。Queued 取消可立即释放 payload 与借用，已投递空任务实际出队后才退出 native idle 计数；Running 仍等待真实 syscall 返回。共享池拒绝时沿正常失败／释放路径退休。两项新增行为测试覆盖普通 blocking 与文件工作共用单线程池、排队取消和拒绝后的完整关闭。
- 退役公开 NativeWork、runNativeWork 及 TLS execute/release 指针工厂。TLS 使用类型化 managed 工作对象与同步 FFI；系统装载在交付前检查取消并关闭未交付结果，验证继续在引擎锁内应用结果。测试占用真实 managed 池，原生独立测试显式驱动任务，不保留另一套测试线程池。外部线程恢复的 TLS 断言同步为返回原 executor。
- 验证：Scoop／C 格式化、diff 检查、release 库构建通过；163 项 asyncio tests、23 项 TLS asyncio tests 及 2 项独立 consumer tests 全部通过。移动 GC／minor GC stress 的 10 项定向测试通过，覆盖共享池、真实 FIFO 退休、AsyncRuntime 与跨 executor TLS。原生三平台源码和 Windows 专有测试已迁移；本轮实际运行平台为 Darwin AArch64，Linux／Windows 和 HTTP 等上层消费者仍待集中验收。
