# 实施进度

分支：`codex/coroutine-executors`。设计：[DESIGN.md](DESIGN.md)。

| 大项 | 状态 |
| --- | --- |
| lambda 按期望类型推导挂起性、对称重载消歧、跨 Cone 定型 | 已完成实现与跨平台验收 |
| core executor 协议、TaskContext、驱动边界、挂起/恢复、scope 与 blocking 入口 | 已完成实现与跨平台验收 |
| Channel、AsyncMutex、AsyncConditionVariable | 已完成实现与跨平台验收 |
| MultiThreadExecutor、通用 managed BlockingTaskPool | 已完成实现与跨平台验收；调度基线为共享 FIFO |
| LoopExecutor、eventLoop/run 入口与调用方迁移 | 已完成库内和 consumer 跨平台验收 |
| AsyncRuntime 共享池与 worker 根入口 | 已完成库内和 consumer 跨平台验收 |
| 三平台 native work 迁移与 TLS 接入 | 已完成 Darwin、Linux、Windows 验收 |
| HTTP 等上层消费者审计与验收 | 已完成；图形相关运行限制见末节 |
| Linux/Windows 组合验证及最终清理 | 功能验证已完成，图形限制已记录 |

每个完成的大项先本地验证、提交并推送，再合并 `origin/main`；跨平台验证集中到完整能力闭环后进行。以下分阶段记录保留当时的完成范围，最终结果以首表和末节为准。

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

## 上层库的排队启动与所有权（2026-11-04）

- HTTP/1、HTTP/2、WebSocket、Client 和 Server 的工厂使用明确的初始化通知，等待所有者状态及取消监听安装完成后再交付对象。启动前取消与正文进入竞争同一个接管状态，避免 launch 排队后被误判为未启动，以及两方重复清理资源；HTTP/2 空闲计时器使用相同的单次清理原则。
- 更新 HTTP 相关规范及调用方；共享认证、SSE、压缩和 WebSocket 的时序测试通过实际开始通知或显式 yield 建立被测操作的先后关系。新增启动前取消后 transport/listener 仍可复用的测试，以及真实多 worker HTTP/1、HTTP/2 consumer。
- 验证：Scoop 格式化、diff 检查及全部系统库 release 构建通过。HTTP asyncio 146 项、Client 254 项、Server 151 项分批覆盖全部通过；Server consumer 6 项通过，其中真实 runtime 用例另在移动 GC／minor GC stress 下通过。Compression 49 项、Terminal asyncio 3 项、真实 PTY fixture 2 项、Window 宿主 1 项和系统 TLS/mTLS fixture 1 项通过。
- Surface asyncio 源码检查通过，运行验收被本机已有 Skia archive 缺少 PathOps 符号阻断；仓库已有对应 vendor 补丁，本轮未扩展修改图形库。实际执行平台为 Darwin AArch64；Linux／Windows 按完整能力闭环集中验证。

## Linux 基础闭环（2026-11-04）

- 在独立工作目录构建 release 工具与新 ABI 运行时，复用已准备的目标 native 依赖。27 项 executor、8 项结构化并发、55 项 concurrent、23 项 TLS asyncio 和 2 项 asyncio consumer 测试通过。
- asyncio 165 项分批覆盖全部通过。删除已退役 C worker 的创建失败测试；原生排队拒绝／取消及完整关闭由现有 managed pool 组合用例覆盖。Linux 文件 CQE 测试补上排队启动后的交接，继续验证真实内核完成先于取消时的字节数和文件偏移。
- Linux HTTP 等上层验证与 Windows 构建验收仍在进行；本节不代表两平台全部完成。

## Windows 基础闭环（2026-11-04）

- 使用维护的 LLVM 22.1.8 SDK 构建 release 工具与 MSVC UCRT 运行时，复用目标 native 依赖。27 项 executor、8 项结构化并发、55 项 concurrent 以及完整 189 项 asyncio 测试通过。
- 独立 Windows 原生测试复用显式 work 驱动，覆盖文件、DNS 和临时文件；TTY 检查先等待排队子任务提交操作。生产侧继续使用统一 managed 阻塞池，未恢复原生线程池。
- 托管阻塞结果与跨 executor Channel／mutex／condition 两项用例在移动 GC／minor GC stress 下通过。三个 CLI fixture 覆盖省略 suspend、跨 Cone 推断及完成回调异常退出，全部通过。
- TLS、独立 consumer、HTTP 和窗口宿主验证继续进行；本节不代表 Windows 全部验收结束。

## 跨平台验收收尾（2026-11-04）

- Linux x86-64 GNU：除上述基础测试外，HTTP asyncio 146 项、Client 254 项、Server 151 项、Server consumer 6 项、Compression 49 项和 Terminal asyncio 3 项全部通过。Client 分批覆盖；请求正文测试增加对端已接收 HTTP/2 stream 的通知，再取消上传，消除把尚未交付的 reset stream 当成必然可接收的假设。
- Linux 定向移动 GC／minor GC stress 3 项通过，覆盖托管阻塞结果、跨 executor 同步及 runtime 共享池。5 组正式 CLI fixture、7 个变体通过，覆盖 lambda 推断、跨 Cone、混合 profile、跨线程尾挂起及未捕获 completion 的退出路径。
- Windows x86-64 MSVC UCRT：TLS asyncio 23 项、独立 asyncio consumer 2 项、Terminal asyncio 3 项和 HTTP Server consumer 6 项全部通过，包含真实多 worker HTTP/1、HTTP/2。独立 consumer 的泛型 runtime 结果另在移动 GC／minor GC stress 下通过，加上前述 concurrent 用例，共 3 项定向 GC 验证。未重复运行已在 Darwin／Linux 覆盖的全部纯 HTTP 测试。
- 无窗口宿主测试补齐 CoroutineScope receiver 内对外层 handler 的显式限定，编译通过。两个 Windows GUI 用例未计为通过：均在进入事件循环前以 `create application` 失败；独立的普通 `Application.run` 程序同样返回 Win32 错误 5。原生 DPI 探针设置 Per-Monitor V2 也返回错误 5，给临时测试副本嵌入 V2 manifest 仍未解除失败。本轮保留现有窗口合同，没有把初始化失败当成协程验收通过；Windows 原生 I/O host／attach 行为已由完整 asyncio 测试覆盖。
- Darwin Surface asyncio 的运行验收仍受旧 Skia archive 缺少 PathOps 符号限制，源码检查通过。以上两项图形限制不影响已通过的 core、concurrent、asyncio、TLS 与 HTTP 结果；本轮不扩展修改图形后端或构建第三方 Skia。
- 普通常规行为使用 release；GC 压力仅用于上述定向验证。原始日志、临时探针、测试副本与 fixture 工作目录在汇总后清理，保留开发 sysroot、构建缓存和可复用的测试产物。

## 2026-11-11：每 worker 无锁队列与 work stealing（B13）

协程调度从共享带锁 FIFO 改为每 worker 独立的 Michael–Scott managed CAS 队列。正常 dispatch 不再进入 executor 全局锁；恢复优先回最近执行的 worker，空闲时窃取其他队列，每处理有限批本地工作也检查其他来源。节点及在途引用使用既有 AtomicRef／GC 根，不新增 native 容器或 runtime ABI。每个 ticket 的原子发布状态协调 complete 与在途入队；休眠／唤醒使用各 worker 自己的同步对象，生命周期和独立 blocking pool 仍使用锁。

先更新规范和设计，再使用已安装 `~/.scoop` 工具链构建独立 concurrent／asyncio 产物。相同源码、CPU 配额和 runtime，对固定新旧二进制随机交错各测三次；测量与编译、测试串行。新增纯调度基准，以 64 个任务、每任务 32,768 次操作隔离队列及协程恢复成本。下表为中位数。

| 纯调度负载 | workers | 改前操作／秒 | 改后操作／秒 | 变化 |
| --- | ---: | ---: | ---: | ---: |
| CoroutineWork dispatch | 1 | 3,358,635 | 6,045,277 | +80.0% |
| CoroutineWork dispatch | 4 | 1,165,277 | 8,115,486 | +596.4% |
| yieldCoroutine | 1 | 1,790,952 | 2,659,999 | +48.5% |
| yieldCoroutine | 4 | 968,029 | 3,217,713 | +232.4% |

裸 TCP echo 复用 B12 的客户端与服务端业务代码，1 KiB 消息、64 KiB 复用缓冲、默认 GC；每项预热 1 秒、测量 6 秒。服务端所有线程共享一个或四个物理 P 核，客户端绑定独立 CPU。64 连接的结果如下。

| 配置 | echo／秒，前 → 后 | CPU μs／echo，前 → 后 | P99 μs，前 → 后 | RSS MiB，前 → 后 |
| --- | ---: | ---: | ---: | ---: |
| 仅 event loop | 110,821 → 110,423 | 9.02 → 9.04 | 770 → 778 | 31.02 → 30.60 |
| 1 worker | 77,641 → 80,510 | 12.88 → 12.42 | 1,278 → 1,311 | 31.27 → 30.25 |
| 4 workers | 85,338 → 106,477 | 20.35 → 16.45 | 1,393 → 885 | 31.04 → 30.84 |

四 worker 的纯 dispatch 为原来的 6.96 倍，yield 为 3.32 倍；真实并发 echo 吞吐提高 24.8%、CPU 成本下降 19.2%。这确认共享调度路径是实质瓶颈，但新实现从一核扩展到四核的收益仍有限，尚不能认为底层瓶颈已经解决。

单连接回退完整保留：吞吐基本持平（1 worker 46,173 → 45,413，4 workers 46,680 → 46,866 echo／秒），P99 分别为 31.49 → 39.94、34.82 → 53.76 μs；RSS 从 12.42／13.07 增至 25.75／25.93 MiB。尚未定位这些回退的根因，不将其直接归因于 GC。后续继续检查调度主路径的扩展性、任务分配与唤醒成本。

格式与静态检查通过。并发库共覆盖 62 个不同用例：首次全套 60 项，新增专用搬迁／公平性用例后分别通过最终队列 3 项与 stealing 4 项；生产代码未再变化。asyncio 175 项通过。覆盖多生产者／消费者恰好一次领取、FIFO、窃取、公平轮询、空队列唤醒、投递与 complete／shutdown 竞争，以及既有 Context／取消／共享阻塞池。两个定向用例在 moving＋minor GC stress 下通过；初次将 8,192 项普通并发测试用于逐分配搬迁压力时，耗时集中于全堆验证，已明确中止，不计为通过。保留普通 8,192 项测试，并以专用 256 项并发测试验证压力搬迁。

短传输的三个配置全部通过；36 个正式 echo 样本含预热校验 18,229,563 次完整响应，24 个纯调度样本核对 50,331,648 次正式操作，全部无错误。echo 为闭环延迟，未校正 coordinated omission；每组仅三次。CPU／RSS 取预热后窗口，GC／分配包含启动、预热和收尾；上下文切换不等同于锁等待次数。仅实测 Linux x86-64 GNU，本轮未重测 HTTP、Darwin 或 Windows。

固定二进制、全部原始数据、报告、源码快照及复现脚本保存在仓库外 `/home/chenxu/benchmarks/http-improvements-20261111-round13`。本轮生产改动仅限 concurrent 调度；asyncio、HTTP、JSON 和 GC 实现未变。

## 2026-11-11：队列发布、关闭与节点退休（B14）

对 B13 的纯调度主路径采样，四 worker 的 `drive` 与 `endPublication` 各占约 19% 用户态 self，指令样本分别集中在归属的强序写入及 ticket 原子减计数附近；单 worker 还存在明显的 GC 标记／复制成本。本轮将接纳与关闭的仲裁合并到队列末端 CAS：正常投递只读取 ticket 活动状态，移除逐投递的 CAS／原子减计数；活动登记归零后封闭各队列，再通知 worker 排空。与 complete 重叠的投递可接纳或拒绝，成功返回的工作必须执行，已完成之后的新投递必须拒绝。语言与并发规范先明确该并发顺序。

任务归属只作为选队列的提示，改用 Relaxed 原子序；payload 的发布仍由队列同步。消费者领取后把旧 head 的后继改为独立终止标记，断开退休节点对后续链的引用。不能恢复为代表开放末端的自链接，否则持有旧 tail 的生产者可能误发布到退休节点。GC、asyncio、HTTP 和 runtime ABI 实现均未变。

沿用 B13 的固定二进制为对照，已安装 `~/.scoop` 编译器、相同 runtime、独立 concurrent／asyncio 产物；CPU 配额、负载、预热／测量时长及每组三次的口径保持，改用随机种子 20261114。构建、测试、正式测量、诊断采样串行。下表为中位数。

| 纯调度负载 | workers | 改前操作／秒 | 改后操作／秒 | 变化 |
| --- | ---: | ---: | ---: | ---: |
| CoroutineWork dispatch | 1 | 6,677,442 | 21,234,460 | +218.0% |
| CoroutineWork dispatch | 4 | 8,294,093 | 18,688,820 | +125.3% |
| yieldCoroutine | 1 | 2,618,715 | 3,273,269 | +25.0% |
| yieldCoroutine | 4 | 3,711,614 | 5,530,544 | +49.0% |

| TCP echo 配置 | echo／秒，前 → 后 | CPU μs／echo，前 → 后 | P99 μs，前 → 后 | RSS MiB，前 → 后 |
| --- | ---: | ---: | ---: | ---: |
| 1 worker，1 连接 | 45,023 → 46,210 | 22.21 → 21.54 | 39.94 → 31.49 | 25.76 → 12.49 |
| 4 workers，1 连接 | 46,648 → 47,392 | 27.73 → 27.35 | 32.26 → 36.35 | 25.91 → 11.84 |
| 1 worker，64 连接 | 80,572 → 81,287 | 12.41 → 12.31 | 1,311 → 1,245 | 30.30 → 31.15 |
| 4 workers，64 连接 | 105,207 → 110,531 | 16.43 → 16.26 | 1,163 → 803 | 30.90 → 31.67 |

单连接的内存回退已消退，1／4 worker 的生命周期 GC 收集时间分别为 86.33 → 38.16、112.36 → 49.05 ms，分配字节／echo 基本不变；退休链接改变了存活关系，本轮未更改 GC 算法。四 worker 单连接 P99 仍回退约 12.7%，不将其描述为尾延迟全面改善。64 连接四 worker 吞吐提高 5.1%、CPU 成本下降 1.0%，纯调度收益没有等比例转成网络吞吐；同期 owner-only 吞吐变化约 +0.4%。三次范围及全部回退保存在报告中。

改后四 worker 纯 dispatch 的 `CoroutineQueue.take`／`tryPut` 分别占用户态 self 64.92%／15.35%；take 内样本集中在 head CAS 与退休链接写入附近。四 worker yield 的一个 `CoroutineLock.withLock` 实例占 21.59%，还有其他实例。四 worker dispatch 仍未超过单 worker，后续继续定位本地领取／跨线程窃取和协程驱动同步。独立 echo 采样中 owner 约占一个核的 81.8%，四个 worker 各约 22%；不据此认定 owner 已满核。采样存在扰动与指令 skid，与正式吞吐／延迟结果分开解读。

格式、静态检查、concurrent 全套 64 项和 asyncio 全套 175 项通过；新增关闭后排空及关闭与多生产者／消费者竞争，既有公平性、唤醒、Context 和关闭组合全部通过。两个队列用例显式启用构建期 GC stress 支持，在 moving＋minor 压力下通过。三个短传输配置通过；36 个正式 echo 样本含预热校验 18,993,107 次完整响应，24 个纯调度样本核对 50,331,648 次正式操作，全部无错误。仅实测 Linux x86-64 GNU，未重新测 HTTP 或其他平台。

完整报告、固定二进制、前后采样、源代码与复现命令保存在仓库外 `/home/chenxu/benchmarks/http-improvements-20261111-round14`。CPU／RSS 取预热后窗口；GC／分配包含预热及收尾。闭环延迟未校正 coordinated omission，每组仅三次；不将短微基准或 self 采样比例等同于应用加速比。

## 2026-11-11：本地环形 FIFO 与批量 work stealing（B15）

B14 改后四 worker 的纯 dispatch 热点集中在通用链表队列的领取和发布。本轮将每个 worker 的单一 MPMC 链表改为单生产者本地环形 FIFO 与外部无锁 inbox。同池物理 worker 内部投递复用 256 个 managed 槽位；外部投递及本地溢出仍使用 inbox。空闲 worker 窃取对方至多一半的本地工作，第一项立即执行，其余批量进入自己的本地 FIFO。内部投递不再分配链表节点，领取不再退休链表链接；owner 领取仍有一次 head CAS。

窃取范围在复制和清空完成前保持预留，防止 producer 提前复用槽位。持有预留的 worker 暂停时，其他 stealer 不等待它；owner 仍可处理未预留工作和 inbox。标量 TLS 识别实际物理 worker，跨池投递进入对方 inbox，不保存 managed 指针。保留 yield、公平轮询、任务 Context、取消及关闭排空语义；没有新增 runtime ABI 或改变 GC 实现。规范和设计已先行同步。

使用固定 B14 二进制、已安装 `~/.scoop` 编译器和相同 runtime 对比，构建、验证、正式测量和采样串行。沿用 24 个短调度样本；为确认不足一秒的样本波动，另做 24 个长样本，每项 64 个任务、每任务预热 256 次、正式 1,048,576 次。新旧各三次随机次序，中位数如下；两组数据独立保留。

| 长运行纯调度 | workers | 改前操作／秒 | 改后操作／秒 | 变化 |
| --- | ---: | ---: | ---: | ---: |
| CoroutineWork dispatch | 1 | 21,166,872 | 23,240,728 | +9.8% |
| CoroutineWork dispatch | 4 | 21,269,377 | 30,740,849 | +44.5% |
| yieldCoroutine | 1 | 3,335,270 | 3,450,826 | +3.5% |
| yieldCoroutine | 4 | 5,919,060 | 5,288,880 | −10.6% |

四 worker dispatch 的短样本提升 38.1%，yield 回退 10.5%，与长运行方向一致。本地队列改善了低层调度，但完整协程的多核回退尚未闭合，不能认为主路径整体优化完成。

裸 TCP echo 保持 1 KiB 消息、64 KiB 缓冲区、1 秒预热／6 秒测量和原 CPU 配额；36 个样本的新旧各三次中位数如下。

| TCP echo 配置 | echo／秒，前 → 后 | CPU μs／echo，前 → 后 | P99 μs，前 → 后 | RSS MiB，前 → 后 |
| --- | ---: | ---: | ---: | ---: |
| 1 worker，1 连接 | 46,458 → 45,851 | 21.52 → 21.77 | 32.00 → 32.26 | 12.56 → 12.31 |
| 4 workers，1 连接 | 47,299 → 47,529 | 27.38 → 27.42 | 32.00 → 35.84 | 11.83 → 11.89 |
| 1 worker，64 连接 | 81,289 → 81,125 | 12.32 → 12.31 | 1,245 → 1,262 | 30.94 → 31.06 |
| 4 workers，64 连接 | 109,657 → 111,433 | 16.13 → 16.09 | 852 → 844 | 31.65 → 31.88 |

64 连接四 worker 吞吐变化 +1.6%，同期不使用业务 worker 的 event loop 对照变化 +1.4%，不能将前者直接归因于队列优化。单连接四 worker P99 回退约 12.0%；64 连接改后另有一次 95,881 echo／秒、P99 3,964.93 μs 的异常样本，全部保留，尚未定位根因。每 echo 分配基本不变，外部 I/O 恢复仍走 inbox；64 连接的生命周期 GC 收集时间在 1／4 worker 下为 294.42 → 360.05、893.17 → 903.35 ms，不将其变化直接解释为尾延迟回退的原因。

改后独立用户态采样：四 worker dispatch 的 `CoroutineLocalQueue.take`、`CoroutineWorkers.findWork`、`submit` 分别为 26.16%、23.63%、10.02% self；完整 yield 的各 `CoroutineLock.withLock` 实例合计 27.88%，最大实例为 21.62%。后续重点仍是协程驱动／恢复同步和 I/O 的 owner／worker 交接；这些比例定位调查方向，既不是墙钟分账，也没有证明 yield 回退的根因。本轮未继续添加条件 I/O 快路径。

格式与静态检查、concurrent 全套 69 项、asyncio 全套 175 项通过；三个专用队列用例在 moving＋minor GC stress 下通过。覆盖槽位复用与 FIFO、批量转移的目标容量、8,192 项恰好一次领取、1,024 个子任务的窃取和溢出、关闭时排空本地工作，以及既有公平性／唤醒／Context 组合。三个短传输配置通过；正式 echo 含预热校验 18,950,465 次响应，短／长调度分别核对 50,331,648／1,610,612,736 次操作，全部无错误。

报告、固定二进制、源码、全部范围和异常样本保存在仓库外 `/home/chenxu/benchmarks/http-improvements-20261111-round15`。CPU／RSS 取预热后的测量窗口，GC／分配包含预热和收尾；闭环延迟未校正 coordinated omission，每配置仅三次。只实测 Linux x86-64 GNU，本轮未重测 Go、HTTP、JSON 或其他平台。
