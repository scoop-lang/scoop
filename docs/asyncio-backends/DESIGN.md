# EventLoop 平台后端与 GUI 主循环接入

状态：Darwin、Linux GNU／musl 后端、宿主接入和消费者迁移已交付；Windows 扩展实施中。日期：2026-10-28。原有验证范围见[实施记录](PROGRESS.md)，性能与回退见[性能记录](PERFORMANCE.md)；Windows 的分批验收见[Windows 进度](../windows-targets/PROGRESS.md)。

本设计对应[异步 I/O 库规范第 11 节](../specs/SCOOP-ASYNCIO-SPEC.md#11-平台后端与宿主主循环迁移合同)。沿用[语言规范](../specs/SCOOP-SPEC.md) 11.9～11.9.2 的协程与任务树、[运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)第 4 章的 native／callback 边界，以及[平台库规范](../specs/SCOOP-PLATFORM-SPEC.md)的错误、buffer 与 TTY 所有权。编译器、runtime ABI 和 `.slib` 格式不变。

## 1. 决策与范围

公共 `EventLoop` 继续表示异步 I/O 服务。提供 `PlatformEventLoop` 作为默认实现，macOS 使用系统 libdispatch，Linux 使用 io_uring 与 liburing。任务树、取消结果选择、buffer 借用、资源关闭和 continuation 恢复共用一份 Scoop 实现；平台后端只负责原生操作及其真实完成。

同一个实现支持两种驱动方式：命令行程序用现有同步 `run`；GUI 用 `attach` 把非阻塞处理接入宿主主循环。AppKit／其他 GUI 主循环继续负责窗口、输入、菜单和显示刷新，EventLoop 不接管这些职责。

首版覆盖现有全部 asyncio 能力：定时器、TCP、UDP、DNS、文件、目录、路径操作、TTY 和 `NativeWork`。TLS、HTTP、地址竞争、SOCKS5 等继续组合公共 API。文件监听、子进程、任意 fd 接管、通用任务执行器、协程拦截器、渲染线程与帧调度不在本次范围。

平台由构建目标决定，不增加运行时后端注册表或公共后端枚举。迁移时通过内部 libuv 适配验证共享逻辑；现已移除 `LibuvEventLoop` 与 libuv 依赖，并更新仓库内调用方。

## 2. 迁移前状态与拆分位置

以下保留迁移前的耦合与对应拆分，当前实现不再依赖 `uv_run`：

| 迁移前位置 | 迁移前职责／耦合 | 实现方式 |
| --- | --- | --- |
| `src/libuv_event_loop.scoop` | 根入口、Job／Context 检查、具体资源构造 | `PlatformEventLoop`，持有共享 owner |
| `src/loop_state.scoop` | 命令、完成、ready 队列及运行收尾 | 重整为 `LoopOwner`，支持阻塞驱动与宿主驱动 |
| `src/loop_operation.scoop` | 选择成功／取消／关闭结果，等待原生完成 | 保留同一算法，消除后端类型依赖 |
| `src/owned_endpoint.scoop`、`owned_file.scoop`、`owned_directory.scoop` | 具体持有 `LibuvEventLoop` | 只依赖共享 owner，保留各资源的并发规则 |
| `src/native_loop.scoop`、`native/loop.h` | 完成记录、uv 错误与 uv handle | 平台无关记录与不透明 native handle |
| `native/work.c`、`native/resolve.c`、文件／TTY helper | libuv worker／poll，部分已复用 platform | 复用 GC-free 平台 helper，由各后端提交 |

表内路径均相对 [scoop.asyncio](../../sysroot/src/scoop.asyncio/)。当前 native 代码按 `common/`、`darwin/`、`linux/` 分目录，以真实职责拆文件。

分层边界如下：

| 层 | 持有的状态 | 不承担的职责 |
| --- | --- | --- |
| core `Job`／`CoroutineScope` | 任务父子关系、取消传播、join、结果 | I/O、线程选择 |
| asyncio 共享层 | 根任务、操作表、资源表、队列、借用、结果选择 | dispatch source／SQE 的内部状态 |
| native 后端 | fd、请求、稳定内存、平台等待／取消、完成记录 | managed 对象、Job、用户闭包 |
| 宿主适配 | 唤醒 fd 监听、owner 线程入口、监听注销 | I/O 语义、资源关闭策略、另一套任务树 |

## 3. 公共入口与宿主协议

保留 `EventLoop` 全部现有方法、`currentEventLoop` 和 `runWithResult`。新增入口的公共声明如下：

```scoop
public class PlatformEventLoop public constructor() : EventLoop {
    public fun attach(host: EventLoopHost,
        onClosed: (Throwable?) -> Unit): EventLoopSession
}

public class EventLoopSession internal constructor(owner: LoopOwner) {
    public val loop: EventLoop
    public val isClosed: Boolean
    public fun post(block: (CoroutineScope) -> Unit): Boolean
    public fun requestClose()
}

public enum EventLoopNotification {
    ReadableDescriptor(Int), WindowsEvent(Ptr<Unit>)
}

public interface EventLoopHost {
    public fun isCurrentThread(): Boolean
    @Unsafe public fun watch(notification: EventLoopNotification,
        ready: () -> Unit): EventLoopWatch
}

public interface EventLoopWatch {
    public fun remove(onRemoved: () -> Unit)
}

public class MainDispatchHost public constructor() : EventLoopHost
```

`PlatformEventLoop`／`EventLoopSession` 属于 `scoop.asyncio`；宿主接口与仅在 Darwin 提供的 `MainDispatchHost` 放在 `scoop.asyncio.host`。`LoopOwner` 是库内共享状态，应用不能直接创建 session。普通 I/O 调用方不接触 fd、dispatch queue、ring 或 `pump`。

### 3.1 单次生命周期

`run` 与 `attach` 竞争同一个单次启动状态：只能接受其中一个，不能并发启动、重入或关闭后重用。两者调用时均不得已有当前 Job 或活动 EventLoop binding。构造不取得 native 资源，接受启动后才初始化；失败回滚已取得资源，实例进入关闭状态。

`run` 的调用线程成为 owner，保留既有同步返回／抛异常与结构化收尾合同。`attach` 要求 `host.isCurrentThread()` 为真，该线程成为 owner；先建立后端与内部根，再安装监听并投递启动通知，成功后立即返回 session。内部根在 attach 栈内建立 Context 并立即挂起，尚不执行应用投递或提交 I/O；用户代码从后续宿主回调开始。监听安装不得同步调用 `ready`；安装失败时内部根没有子任务或在途 I/O，可以取消并清理初始化资源。

`post`、`requestClose` 与 `isClosed` 可跨线程使用，发布与读取具有必要同步。其余驱动／宿主操作只在 owner 上执行。资源仍要求当前 Job 和所属 EventLoop binding；没有 binding 的 GUI 事件通过 `session.post` 进入，而不是直接调用挂起 I/O。

### 3.2 GUI 根任务与投递

`attach` 内部建立精确静态类型为 `EventLoop` 的 Context binding，用现有 `startCoroutine` 与 `coroutineScope` 建立一个根任务。根正文消费 GUI 投递的 mailbox，没有消息时挂起，直到请求关闭或任务树失败；不能在启动回调返回时结束这个根作用域。每条 `post` 回调得到同一个仍活动的 `CoroutineScope`，可用 `scope.launch`／`async` 启动有父任务的挂起工作。

`post` 总是异步排队，按入队顺序执行；跨线程之间以入队锁的顺序为准。返回 `false` 表示已停止接受应用投递，且不保留闭包；`true` 表示接受入队，不保证在之后的关闭／失败中仍会执行。关闭开始时丢弃尚未执行的应用投递，但必须继续消费完成、取消和清理命令。普通回调抛错按根正文失败处理，取消子任务并收尾。

mailbox 的执行通过根协程自己的 continuation 恢复完整 Context；不能仅临时绑定 Job 和 EventLoop，也不能假定保存一个闭包就保存了调用方的动态 Context。根继承 `attach` 时的 Context，外线程调用 `post` 不把自己的 Context 偷换进去。

`requestClose` 幂等且不阻塞：原子地停止接受应用投递，入队根取消命令并唤醒 owner，由 owner 调用根 Job 的 cancel。mailbox 等待的取消 handler 也只把恢复放入 owner 的 ready 队列，不在任意取消线程直接恢复根。即使关闭发生在首次宿主回调之前，也必须走完关闭流程且不执行已排队的应用回调。主动关闭引起的取消不作为 `onClosed` 的失败；独立的非取消异常及清理失败照常保留。

宿主必须继续运行至 `onClosed`。该回调恰好一次，在 owner 上、I/O 清理和监听注销完成后执行，此时 `isClosed` 为真；参数是运行／清理结果，组合规则沿既有 `ResourceUseException`。回调本身抛错不重新打开 loop、不再次通知；native 桥接用现有 `ForeignCallback` 捕获异常，适配器负责取出并报告，内置适配器写 stderr，不让异常跨 C 栈传播。

### 3.3 监听的具体合同

`watch` 只借用 `EventLoopNotification` 的资源：POSIX 可读 fd 或 Windows manual-reset event；不读、写、重置、关闭或更改其模式，loop 自己消费通知。HANDLE 保留指针宽度，不映射为 fd。`ready` 必须在 owner 上串行执行，可有合并或多余通知；不得在注册栈内同步执行。安装失败抛出后，host 不得再持有回调或活动监听。 不支持的种类在安装前拒绝。

`remove` 在 owner 调用，按宿主协议不得抛错，异步注销监听；`onRemoved` 恰好一次，并且此后不会再进入 `ready`。如果在 `ready` 内开始注销，必须先退出当前 `ready`，再通知移除完成。注销负责退休已经排队的通知，不能只从 poll 集合删掉 fd 就允许旧闭包继续运行。loop 在 `onRemoved` 前保留 fd 和相关上下文。

该协议只要求宿主提供对应原生通知资源的监听和注销完成，不要求 timer、worker pool 或通用 `dispatch` 接口。后端用自己的统一通知资源把命令、完成和计时到期接入这个入口。

Darwin 的内置 `MainDispatchHost` 用主 dispatch queue 上的 READ source 观察通知 pipe，沿已有 FFI callback 桥进入共享处理；cancel handler 负责移除完成。它适用于正常服务主 dispatch queue 的 AppKit／CFRunLoop 宿主，不调用永不返回的 `dispatch_main()`。自定义 run-loop mode、模态和嵌套循环的服务时机由宿主决定；需要其他 mode 行为的 GUI 层实现同一 host 协议，不能宣称任意 mode 都自动获得主队列服务。Apple 对主队列与主循环的关系见 [queue.h](https://github.com/apple-oss-distributions/libdispatch/blob/main/dispatch/queue.h)。

Linux 的 GUI 层把通知 eventfd 加入自己已有的 fd watch。Wayland 连接继续由 GUI 层遵守 libwayland 的 prepare／read／cancel 协议；X11、窗口事件和渲染也不转给 asyncio。参见 [Wayland 外部事件循环接口](https://wayland.freedesktop.org/docs/html/apb.html)。

## 4. 共享驱动与线程模型

`LoopOwner` 是命令和完成的单一消费者。I/O 提交、外线程取消和 GUI `post` 只发布命令；取消结果的竞争选择及投递入口关闭在短锁内完成。任何共享锁都不包围用户代码、continuation 恢复、等待 I/O 完成、等待 worker 退出或宿主注销；竞争同一 mutex 的线程可按平台协议等待取得该锁。

启动配置与命令通道分开：binding／execution 一次 release 发布后不再改写，worker 查询使用 acquire，不获取全局 owner 锁。操作编号使用独立原子计数。`LoopInbox` 以平台 mutex 保护命令批次、native 通知指针和停止接纳；只有批次从空变为非空才调用 native wake，竞争者通过 NativeSafe mutex 等待，不围绕包含 native 调用的临界区自旋。取批次后释放队列锁再执行命令；driver 退出前对新批次补通知。根结果单独同步，owner 的 ready、操作及资源容器不由 worker 直接访问。停止接纳在 inbox 锁下清除通知指针，所有已经开始的通知返回后才允许销毁后端。

一次内部 `pump` 非阻塞处理以下内容：消费唤醒提示；取得这一轮命令及已有 native 完成的有限快照；提交操作、解码完成；把可恢复项放入 ready 队列；执行本轮 ready；推进收尾。有新工作在处理期间到达，留到下一轮并保留／重新发送唤醒，不通过“持续直到所有队列为空”霸占 GUI 线程。每轮至少推进已排队的取消和关闭；不建立逻辑成本计费或跨阶段资源预算。

B19 将 native 完成改为每批一次 FFI 交付：后端复用一个 64 条记录的连续数组，取出时复制已退休请求的结果，本轮命令和完成消费结束后才统一释放请求记录；数组只在下一次取批次时改写，managed pump 直接读取 GC-free 记录。超过容量的完成保留在队列。每轮末尾统一 rearm 尚未处理的命令、ready 和 native 工作。阻塞请求仍在提交时交给共享池，避免后续 owner 回调等待尚未启动的工作。Linux 的提交和逻辑完成只有 owner 生产，移除它们逐请求的自唤醒；内核 eventfd、外线程命令和阻塞完成继续负责异步通知。Darwin／Windows 保留外线程生产者的同步边界。批次容量是调度实现选择，不限制程序可建立的请求数量。

B20 收拢 managed 操作生命周期：`LoopOperation` 承担结果仲裁与 owner 上的提交／交付，命令和 ready 以普通闭包或操作引用排队。字节传输操作同时实现已有 `SuspendRegistration<Long>`，保留实际 borrow、方向登记和 continuation，完成时在恢复调用方前结束借用和方向占用；TCP、文件和 TTY 写共用这一真实生命周期。读写入口只做普通准备和一次原语挂起，移除跨挂起 finally、单独 progress 对象与 start／cancel／finish 捕获链。其余操作通过同一基类的回调实现继续表达不同资源交付规则。提交、取消、ready 次序和 GUI 分轮合同保持，未增加 public API 或 compiler/runtime 协议。

B21 让每个 owner 复用外部命令、本地命令和 ready 各自的接收／消费容器。inbox 在已有锁内交换，owner 本地容器直接交换；不再把本地命令逐项复制进外部批次，也不在每轮重新分配容器及遍历器。消费次序仍为外部命令、本地命令、native 完成、ready。本轮退出时清空消费容器，下一轮才交回接收端；新发布工作保持在独立的接收容器中。这里只复用 loop 自己的队列容量，不缓存操作对象、借用或 continuation。

B29 将 TCP 字节请求的 native 提交也归并到 owner 的轮次边界。每个后端 loop 持有固定容量的提交数组，逐请求的 GCLeaf 入口只写入 GC-free 的 endpoint、操作编号、借用地址、长度和方向；它不分配、不加锁、不调用操作系统、不回调、不改变浮点环境。资源、borrow、Job 和 continuation 仍由原 ByteTransfer 保活，只有真正完成才解除。数组满时先以一次 NativeSafe 调用提交旧批次，再登记当前请求；容量不是并发请求上限。所有实际平台调用、请求分配及同步失败处理均在 NativeSafe 批量入口内进行，不能把原有读写后端直接标为 GCLeaf。

owner 在 poll／等待前、pump 退出前、取消及普通控制操作开始前提交已有批次，保持读写与 shutdown／close 的次序。暂存请求也计入 native loop 的非空闲状态，不能在尚有提交时销毁后端。批量入口把同步提交失败及平台直接完成保存到独立的连续结果数组，managed 侧按原操作编号送入完成仲裁；数组在下一次提交时才改写，结果交付只进入 ready，不在读取数组期间恢复用户代码。异步请求的后续结果继续走原完成队列。TCP 提交不逐次提取阻塞工作；可能产生阻塞工作的文件、TTY 与普通操作在提交后立即派发，轮次边界仍统一派发剩余工作，保留已有回调等待及排空合同。Linux、Darwin、Windows 共用批次协议，各自实际后端和退休规则保持。

处理过程中 GUI 可能进入嵌套主循环；`pumping` 标志阻止递归进入状态机，嵌套通知只记录需再次处理，外层退出时重新唤醒。普通用户回调仍可能运行很久，本库不抢占它，也不把 CPU 计算自动搬到后台。

`run` 反复调用同一个 `pump`，无可执行工作时在统一通知 fd 上阻塞等待；等待使用 `NativeSafe`，不能标为 `GCLeaf`。`attach` 由 host 可读回调调用 `pump`，内部不等待 I/O、worker 或监听注销。正时长 timer 的到期由后端通知，宿主不需要周期轮询；空闲时没有常驻 idle 回调。

协程恢复遵守语言规范 11.9.3 的 execution 归属：LoopExecutor 上的 I/O、Channel 和第三方挂起点均排队回 owner，AsyncRuntime worker 上的操作回共享 worker executor。独立 PlatformEventLoop 的 `withExecutor(loop.executor)` 显式切回 owner；AsyncRuntime 的 `loop.executor` 则是共享 worker executor，不承诺固定物理线程。普通外部应用消息可通过 `session.post` 进入 GUI 根作用域。frame、Job、取消和 executor 登记仍由 core／concurrent 的同一实现负责；帧请求与 vsync 由 GUI 层安排。

### 4.1 worker 与 reactor 整合（B18 已实现）

裸 TCP 架构对照保留同一组固定库、runtime、进程级 GC 和 echo 正文，把“单 owner＋四 worker”改为同进程四个独立 owner，各自直接执行所属连接的业务。四核配额、64 连接下的三次测量显示明显收益，完整记录见[实施进度 B17](PROGRESS.md#第十七轮reactor-分布与本地执行的架构对照)。这个实验同时改变 reactor 数量、执行位置和连接分配，证明组合方向值得实施，不能把全部收益分摊给某一次队列或锁；多端口均匀分片也不能代替正式 API 的验收。

B18 已以 `AsyncRuntime` 为实际消费者整合现有调度器，并同步 asyncio／concurrent 规范。实现如下；B17 仍只作为架构诊断实验，正式结果见[实施进度 B18](PROGRESS.md#第十八轮正式-workerreactor-整合)。

- 保留一个逻辑 `EventLoop` binding 和一棵结构化任务树；每个 coroutine worker 驱动自己的 native reactor。caller 等待最终结果，不再额外承担唯一 I/O owner。独立 `PlatformEventLoop.run`／GUI attach 继续保持原有线程归属。
- 复用 MultiThreadExecutor 的本地队列、外部 inbox、批量 work stealing 和任务登记。worker 在有限批可运行任务之间推进自己的 I/O；空闲时重新检查队列，再等待 reactor 通知。任务投递和 I/O 完成共同唤醒该 worker，不能引入丢失唤醒或纯忙轮询。驱动接入只服务这个实际 asyncio 调用方，不建立通用插件体系。
- 普通 I/O API 与单 listener 保持。新连接在交付用户前分配到目标 reactor；既有活动资源保持明确、稳定的 native owner。接收端创建、目标 reactor 登记及失败关闭构成一次完整交接，不能把已绑定 IOCP／已有在途请求的 handle 任意搬到另一个 loop。客户端不需要多端口或手工分片。
- 每个资源仍由所属 reactor 串行提交／消费 native I/O，跨 worker 的操作发给该 owner。完成在相应物理 worker 上恢复排队，利用现有同池本地投递规则让后续业务与 I/O 保持局部性；stealing、跨 executor 恢复和调用方 Context 合同保持，不能靠禁止任务迁移获得实验收益。
- 共享根结束后先排空已接纳任务，再由各 reactor 清理遗留资源、取消与实际 native 请求。共享 blocking pool 在所有 reactor 收尾完成前继续接纳必要工作，最后才停止并 join。初始化部分失败、backend 失败、取消与关闭复用同一生命周期，不创建 detached 根任务或提前释放借用。

`CoroutineWorkerDriver` 把每个已创建的 reactor 接入现有 worker；每处理至多 32 个队列任务推进一次非阻塞 pump，空闲发布观察状态后再 pump 并重查任务。任务投递通过同一 native 通知打断等待。`RuntimeWorkerContext` 的 TLS 只保存标量身份。B24 将资源所属 worker 上的操作登记与 native 开始放到本地调用栈，去掉操作发给自身命令队列的往返；普通本地命令仍进入复用数组，跨 owner 操作与命令仍使用现有 inbox。同步结果也只排入 ready，交付不进入提交栈、不递归 pump；内核请求继续批量推进。独立 PlatformEventLoop／宿主接入保持原有命令提交合同。

Linux 的 B24 native 推进分为已有 CQ 消费、待提交 SQ 批量发布、提交后 CQ 消费。后一次消费让提交时已完成的请求进入同轮 managed 完成批次；两次读取各自以可见 CQ 数量为界，不循环等待新事件。后一次完成产生的重试／取消提交留到下一轮，并由已有 rearm 保证进度。请求和控制 CQE 的退休条件保持，C 层只发布完成记录，continuation 仍由 managed ready 阶段恢复。独立 event loop 共用此 native 推进，但保持原有 managed 提交队列。

B18 使用普通 `AsyncRuntime` 和一个监听端口，测量单／四 worker、单／64 连接的吞吐、延迟、CPU、分配与 GC，保留独立 event loop 对照。新增持续 runnable 负载下的四 reactor timer 进度、跨 executor I/O、单 listener 分配、部分传输与移动 GC 交接测试，并通过既有取消、单次完成和关闭排空回归。仅 Linux/io_uring 已实测；Darwin／Windows 的目标 owner 参数已同步但尚未实机验证，不均匀连接与混合 CPU 负载的性能仍待验证。多 reactor 下仍有显著用户态和托管分配成本；继续按实际采样处理，不通过放宽取消、GC 或资源规则换取吞吐。

### 4.2 Mio、Tokio 与 tokio-uring 的架构参照

为 4.1 的正式改造核对了官方文档与源码。三者承担的职责和线程模型不同，不能把它们合并称为“每 worker 一个 reactor 的 Tokio 架构”。以下区分已实现机制、RFC 和 Scoop 的设计判断；不引入这些 Rust 库作为 Scoop native 后端依赖。

| 参照 | 已核对的机制 | 对 Scoop 主路径的启示 |
| --- | --- | --- |
| Mio `Poll`／`Waker` | `Poll` 返回 readiness，`Token` 关联事件源；通知可能多余，实际 I/O 结果才决定进度。`Waker` 负责跨线程唤醒 selector，不是任务队列或调度器。 | 把“有工作可能需要处理”的通知与真实命令／完成记录分开；任务与 I/O 共用 worker 的等待入口。Scoop 已有 io_uring／IOCP completion 合同，保持该模型，不额外模拟一层 readiness。 |
| Tokio 多线程调度器 | 每 worker 有本地任务队列并支持 stealing；已检查的 `multi_thread/park.rs` 用 `Arc<Shared>` 共享一个 `TryLock<Driver>`，取得它的 worker 推进 driver，其他 worker 可等待条件变量。 | I/O 推进纳入 worker 调度循环，不必专设一个只驱动 I/O 的 caller；已交付的本地队列和 stealing 继续复用。共享 driver 与每 worker 独立 reactor 是两种不同选择，Tokio 的实现本身不能证明哪种更适合 Scoop。 |
| tokio-uring | 已发布的 runtime 使用 Tokio current-thread runtime 与 `LocalSet`；多线程靠创建多个 runtime。线程进入等待前 flush SQ，通过观察 ring 的可读事件消费 CQ；任务通过 `spawn_local` 执行。 | 借鉴任务、提交和完成在同一线程推进的局部性，以及批量提交／完成的处理位置。它没有多线程 work stealing，不能直接用它替换 Scoop 的多线程 executor。 |

来源：[Mio Poll](https://docs.rs/mio/latest/mio/struct.Poll.html)、[Waker](https://docs.rs/mio/latest/mio/struct.Waker.html)及 [Linux Waker 实现](https://github.com/tokio-rs/mio/blob/master/src/sys/unix/waker/eventfd.rs)；[Tokio runtime](https://docs.rs/tokio/latest/tokio/runtime/index.html)与 [park.rs](https://github.com/tokio-rs/tokio/blob/master/tokio/src/runtime/scheduler/multi_thread/park.rs)；[tokio-uring 文档](https://docs.rs/tokio-uring/latest/tokio_uring/)与 [runtime 实现](https://docs.rs/tokio-uring/latest/src/tokio_uring/runtime/mod.rs.html)。这些是本轮查阅的实现，不能依赖其队列容量或检查间隔作为上游永久合同。

B18 已移除 `worker → 单 owner → worker` 的必经往返。选择每 worker reactor 的依据是 B17 的本机组合实验与 owner 集中负载，不是“Tokio 也这样做”。正常局部路径由当前 worker 提交、消费完成并把 continuation 放回本地队列；被窃取任务访问其他 owner 的资源时才跨 worker 交付。保持资源 owner 稳定、让可运行任务迁移，是 Scoop 为同时保留局部性与 stealing 做出的设计选择。普通单 listener 已完成实测，不均匀负载的性能仍待验证。

整合时遵守以下直接约束：

- worker 在有限批任务之间推进 I/O 与 timer；队列持续非空也不能饿死 reactor。等待前先提交待发操作、发布等待状态并重查队列，外部发布必须能打断等待。Mio 的 `wake` 是通知机制；减少系统调用的合并决策应由 Scoop 的队列／等待状态承担，不能假定每次 `wake` 免费。
- I/O owner 与当前执行任务的 worker 分开记录。新 accepted 连接在交付前分配 owner，现有请求、buffer 借用和 native handle 不随 stealing 搬迁；完成在 owner worker 上进入既有调度器。不能依赖任务永不被窃取，也不让用户手工管理端口或线程。
- 操作取消和 native 退休仍是两件事。tokio-uring 的 [driver 实现](https://github.com/tokio-rs/tokio-uring/blob/master/src/runtime/driver/mod.rs)在 future 被放弃后保留在途数据，取消操作自身的 CQE 不代替原操作完成。Scoop 继续以真实完成解除 `NativeByteBorrow`，统一排空后再销毁 reactor；线程局部执行不能取消这个生命周期约束。

tokio-uring 的 [DESIGN.md](https://github.com/tokio-rs/tokio-uring/blob/master/DESIGN.md)是 RFC，可用于理解 thread-per-core 和在途操作所有权的动机；其中示例 API 与拟议机制不能直接当成已发布功能。这里仅采用与当前源码一致、直接服务 worker／reactor 整合的部分，不扩展到固定 buffer、零拷贝、其他后端重写或 HTTP／JSON 优化。

## 5. 一份操作与资源生命周期逻辑

`sleep` 与一次性 `schedule` 共用 owner 的逻辑期限队列，原生层只维护最早期限所需的共享唤醒。逻辑登记携带可直接移除的队列位置；普通撤销立即清空通知捕获，owner 再移除登记，不创建逐通知的 native cancel 或资源表记录。sleep 的取消结果同样等 owner 移除后才恢复；它不借用 native 存储，无需等待其他使用者共享的唤醒退休。

每轮已有 ready 执行后以一次单调时钟快照选定到期项，通知进入下一轮 ready，锁外调用。更早的新期限才替换当前 native 唤醒；原最早项被取消后保留已经安排的较早唤醒，到时重新检查队列。当前唤醒的失败交付给仍等待的逻辑登记；旧唤醒的迟到结果不能影响替换后的登记。共享唤醒继续用 `LoopOperation` 跟踪真实 native 退休，不保留调用方捕获；清理关闭逻辑队列、取消当前唤醒并排空全部操作。期限比较保留 Duration 的完整范围，不强制转换为总纳秒数或可能溢出的绝对 TimeMark。具体结果选择与生命周期见 asyncio 规范第 5 节。

已选定回调与撤销竞争时仍由消费者自己的状态决定是否有效；HTTP 的连接锁与请求期限状态承担这一职责。计时不由辅助 coroutine 持有 native 资源，也不为每个通知重复向平台提交 timer。

### 5.1 操作有两个独立事实

共享 `LoopOperation` 分别跟踪“已选定的用户结果”和“native 是否结束”。Job 取消、资源关闭、native 完成竞争前者；后者只能由后端完成记录确认。所有路径只选择一次结果、只恢复一次 continuation。

| 时序 | 对外结果 | 仍必须完成的工作 |
| --- | --- | --- |
| 登记时 Job 已取消 | 取消 | 不提交 native，不建立长期借用 |
| 原生完成先被共享层接受 | 成功或该 I/O 错误 | 提交进度、解除借用、撤销取消订阅 |
| Job 取消先选定 | 取消 | 请求撤销，等真实结束，提交已知实际进度 |
| close 先选定未决结果 | `IoClosedException` | 同上，不覆盖更早的取消／完成 |
| 取消后迟到 open／accept／connect 成功 | 保留取消 | 先关闭未交付资源，才报告完成 |

“先”以共享状态中的选择顺序为准，不以墙上时钟或 callback 创建时间推断。native cancel 的返回值、dispatch source cancel 请求、取消 SQE 的 CQE 都不能单独证明数据操作已经结束。

原生操作尚持有 `NativeByteBorrow` 时，不用会立即交付 Job 取消的通用可取消挂起包装它。沿现有 `LoopOperation` 的延迟完成方式，等真实借用结束后才恢复；部分读写进度即使最终抛取消／关闭也照常提交。原生只保存稳定 allocation、native request 和数值 id，managed 表保活 buffer owner、continuation 与资源。

### 5.2 资源与 loop 收尾

保留 `Open → Closing → Closed`：关闭入口拒绝新操作，取消尚未选定结果的操作；在途操作、buffer 借用与原生注销全部结束后才消费 fd／owner 并完成 close 等待者。并发 close 共享一次收尾；再次 close 不重复关闭或重抛旧错误。单独取消 TCP read／write、UDP receive、TTY receive 不关闭健康资源；`shutdownOutput` 提交后取消仍终止整个 stream。

loop 生命周期为 `Created → Running → Stopping → Draining → Detaching → Closed`；正常根完成可直接进入 Draining，同步运行跳过 host 注销。Stopping 停止应用投递并取消根；根及子任务执行各自 finally／close。根完成后 Draining 关闭所有遗留资源，即使某项失败也继续，并排空清理命令、完成和 ready。这里复用 core 的结构化 join，不另建 detached job 或公共 root-job 构造入口。

只有 managed 操作／资源／ready 已空、native 请求及控制请求已退休、worker 和 I/O callback 不再生产完成时，才进入 Detaching。先注销 host watch；确认 `onRemoved` 后再销毁后端、关闭通知 fd 并发布 `onClosed`。`run` 以同样的条件销毁并返回／抛错。应用停止入口与 native 通知 fd 的寿命在同一发布边界协调，避免外线程向已关闭甚至已复用的 fd 写通知。

当前执行的 host 移除通知持有自己的 callback lease，返回后释放；它不能阻塞等待自己退出，也不能在返回前释放仍使用的 C callback context。I/O 后端的退出条件与这个最后的宿主通知分开计数。

显式关闭是唯一正常生命周期。GC `release` 不启动 pump、调用宿主、恢复协程、等待 worker 或补做异步关闭。worker 已进入不可中断的 syscall 时，关闭耗时由其返回决定，不能为了 GUI 快速退出而提前释放它使用的内存。

## 6. 后端边界与唤醒

内部 native 接口保留按实际操作划分的 submit／cancel／close 函数，增加共用的初始化、通知 fd 获取、非阻塞完成批次获取与销毁入口。C 头文件使用不透明 loop／resource handle 与普通数据记录，不出现 uv、dispatch 或 io_uring 类型；Scoop 侧无需为每个平台复制资源类。

完成记录继续携带操作 id、结果类别、实际进度、错误 domain／code 以及必要的 native payload。沿现有事件编号分段扩展；不属于 runtime ABI。每个已接受操作最终产生一条逻辑完成；取消控制请求、两个 TTY poll 及其 timer 等内部请求由后端聚合并退休，不分别恢复同一 managed 操作。

通知是“可能有工作”，不是完成计数。Darwin 使用非阻塞 close-on-exec pipe；Linux 使用同属性 eventfd，并把它注册为 ring 的完成通知，同时用于命令／worker 完成唤醒。队列先发布、再通知；写满意味着已有未消费通知，重试 EINTR，其他 I/O 错误进入运行失败收尾。读端清空通知后检查真实队列／CQ；若本轮留下工作则重新唤醒，关闭检查也依据队列与活动请求，不依据 fd 是否可读。[liburing 明确允许通知合并及多余通知](https://man7.org/linux/man-pages/man3/io_uring_register_eventfd.3.html)。

错误统一保留实际来源：POSIX syscall 立即保存 errno，io_uring 的负结果取相反数形成 `PosixErrno`；DNS 保留 `AddressResolution` 与 `EAI_SYSTEM` 的 errno。正常 timer 到期的 `-ETIME` 解码为到期，不报 I/O 失败；Job 取消和 `IoClosedException` 仍由共享层决定。最终后端不产生 `.Libuv` 错误；清理平台库中该枚举项须与全部实际消费者迁移一起进行，不伪造 UV 错误码来维持内部测试。

## 7. macOS：libdispatch 后端

每个 loop 有一条串行 I/O dispatch queue，负责 endpoint、source 与请求的原生状态。串行 queue 不等于固定线程；其 event／cancel handler 都只执行 C、发布完成，不运行 Scoop 用户代码。owner 的 submit／cancel 通过 `dispatch_async_f` 等 C 入口入队，不用 `dispatch_sync` 等待 owner 与 I/O queue 互相返回。

Mac 同机测量显示四 worker 的内核态 CPU 和通知／等待交接成本明显高于 Go。沿用 Linux 的 owner 本地推进与批量边界，TCP 字节读写在已有 NativeSafe flush 中先由资源 owner 尝试非阻塞 syscall；字节数、EOF 和非 EAGAIN 错误进入原提交批次的结果数组，和提交失败一起交给 managed 完成仲裁。立即完成不分配 native 请求，不经过 dispatch queue，也不写通知 pipe；EAGAIN／EWOULDBLOCK 才创建原请求，由 dispatch source 继续等待。不是将 syscall 放进 GCLeaf，也不直接恢复 managed continuation。

这条直接路径仅用于已经交付并按原协议占用方向的 TCP 端点。同方向的前次 handler 在发布完成之前结束其读写与 EOF／错误状态更新，owner 在消费完成后才允许下一次提交；反向传输不访问读方向状态。端点的 fd 在未关闭期间固定，关闭停止新接纳，并在 native close 入队前先 flush 已暂存传输。直接完成只写结果与读方向状态，不修改 I/O queue 独占的 active／input／output 或 source 容器。异步与直接路径共用同一 TCP syscall／EOF／错误处理函数，不能维护两套不同的传输语义。取消选择已经发生时仍保存实际写入进度，待原 ready 清理借用后恢复。

通知使用 loop mutex 保护的一个未消费标记。生产者先发布实际工作，仅在标记为 false 时向 nonblocking pipe 写入一个提示；owner 在同一锁内读取该提示并清除标记，随后检查真实工作。没有提示时跳过原来的 FIONREAD／read 调用。控制动作结束仅在 actions 降到零时通知排空进度，避免每次把请求挂上就绪监听后都产生一次空唤醒；真正完成的通知保持。外线程不能插入到清除提示与标记之间，返回宿主或等待前仍按真实剩余完成／managed 工作 rearm。正常排空、取消、源退休及阻塞池完成使用同一通知规则，保留原 close-on-exec、EINTR、I/O 失败及无忙轮询合同。

TCP／UDP／TTY 使用 nonblocking fd、READ／WRITE source 与实际 syscall：先尝试操作，遇到 EAGAIN 再等待就绪；只在有等待操作时启用相应兴趣，避免可写 socket 造成空转。连接就绪后检查 `SO_ERROR`；listen／accept、地址复用与 SIGPIPE 处理沿 platform 的既有 POSIX helper。source suspend／resume 必须平衡，取消 suspended source 时也须使 cancel handler 能到达。

TCP 的 source 在端点的读／写方向上懒创建并复用，避免每次 EAGAIN 都重新向 kqueue 注册、随后注销。source context 只指向端点内的方向记录，handler 查找当前 input／output，不捕获前次请求或借用地址。完成和取消在串行 queue 中暂停该方向 source、清除等待请求，再发布完成；后续请求需要等待时重新启用 source。空闲 source 不交付事件，但仍计入 loop 存活工作。close 先结束两个方向，再取消其 source 并平衡暂停，所有 cancel handler 退休后才关闭 fd。owner 直接路径不读取或修改这些 queue 专属字段。

timer 用单调时钟 dispatch timer；到期仍核对截止时间，不能提前完成。timer 及其他一次性 fd 等待在正在执行的 handler 与所需 cancel handler 结束后发布终止；TCP 复用 source 的请求按上述暂停和解除引用规则退休。一个 fd 被多个 source 引用时，资源关闭等所有 source 都退出再关闭 fd，不能把 fd 释放交给第一个 cancel handler。[Apple source 合同](https://github.com/apple-oss-distributions/libdispatch/blob/main/dispatch/source.h)要求这种顺序。

普通文件、目录、路径、DNS 和 `NativeWork` 复用现有 GC-free C helper，投递到共享 managed 阻塞池。文件操作保持每文件串行，不能在串行 I/O queue 或 GUI 主队列上执行可能阻塞的文件访问。尚未执行的 work 可以观察取消而跳过；已经执行的 work 必须完成并发布结果，取消不会强制终止 `getaddrinfo` 或文件 syscall。

使用系统 libdispatch 的 `_f` API，不要求 Scoop 支持 Objective-C、Blocks 或 `.m`／`.mm` native 编译；也不引入 `dispatch_io` 的 buffer／channel 所有权模型来复制当前资源合同。主循环进入 managed 的入口只在 host 适配层使用既有 `ForeignCallback`。

## 8. Linux：io_uring 后端

### 8.1 依赖与最低能力

使用固定源码版本 liburing 2.15，随 Cone 的 native 输入与许可证一起发布；该版本已有 [2026-06-29 上游发布记录](https://github.com/axboe/liburing/releases/tag/liburing-2.15)。GNU／musl 分别按现有目标工具链构建，不依赖系统 liburing，也不手写 ring 的内存屏障和 SQ／CQ ABI。

首版支持与验证基线选择 **Linux 6.1 或更新内核**；这是 asyncio 新后端的运行要求，不改变其他 Scoop 程序的最低内核要求。初始化实际建立 ring、probe 所需操作并检查必要 feature，不仅判断版本字符串；所需操作为 ACCEPT、CONNECT、RECV、SEND、RECVMSG、SENDMSG、READ、WRITE、FSYNC、TIMEOUT、ASYNC_CANCEL、POLL_ADD，要求 `IORING_FEAT_NODROP`。操作存在也不保证任意 fd 类型／文件系统支持，提交错误仍按操作报告。探测依据见 [liburing probe](https://man7.org/linux/man-pages/man3/io_uring_get_probe.3.html)。

ring 被系统策略禁用、初始化失败或缺少所需能力时，启动报告明确的平台错误并回滚；不静默退回 libuv、epoll 或线程模拟整个后端。部署验证覆盖实际容器／运行用户的 io_uring 可用性；如果以后必须支持旧内核，再以具体部署需求单独设计后备。

### 8.2 提交与完成

每个 loop 一个 ring，仅 owner 线程提交 SQE、消费 CQE。首版使用普通 interrupt-driven ring 和单次请求，不开启 SQPOLL／IOPOLL、multishot、注册 buffer 或零拷贝发送，避免引入尚无收益依据的额外借用周期。

`user_data` 指向稳定 C request，内含共享操作 id 及内部请求类别；请求对象只在其所有 CQE 退休后释放。待提交操作由后端队列持有；SQ 满时保留并稍后提交，不丢请求，不把队列暂满作为业务 I/O 失败。部分提交按实际已提交数推进；取消／关闭请求应能随 SQ 空间释放继续提交，不能被新业务提交一直挤在后面。

每轮先消费已有 CQE，再推进待提交项；CQ 满时沿 NODROP／overflow 机制继续取回完成，不默默丢记录或只等下一次 eventfd 边沿。若 SQ/CQ 暂时需要重试，安排后续处理；无工作时不忙等。通知 eventfd 使用普通注册而非仅异步完成通知，队列本身始终是完成来源。

READ／WRITE 的文件 offset 使用 `-1`，保留真实 fd 位置，并维持现有每文件操作串行规则；不新建一套逻辑 offset 或绕过 append 语义。单次长度取平台可表达的合法前缀，短读写与取消的实际进度原样交给共享层。[liburing read 文档](https://man7.org/linux/man-pages/man3/io_uring_prep_read.3.html)说明了共享 fd offset 的同步要求。

普通文件的数据 I/O 与 fsync 首版设置 `IOSQE_ASYNC`，交由内核异步执行路径，减少 GUI owner 上直接尝试文件访问的延迟；网络等待不一律使用该 flag。这个选择有 worker 调度开销，需在验收中测量，不宣称提交 syscall 完全没有延迟。[liburing 对 IOSQE_ASYNC 的说明](https://man7.org/linux/man-pages/man3/io_uring_sqe_set_flags.3.html)给出了其执行方式与成本取舍。

### 8.3 取消和复合等待

按目标 request 的 `user_data` 提交 ASYNC_CANCEL，取消请求拥有独立 request。目标数据 CQE 与取消 CQE 都必须消费，顺序不限；取消返回 `-ENOENT`／`-EALREADY` 是正常竞争，不能代替目标结果。若目标仍在用户态待提交队列、尚未发布至 SQ，则移除并直接完成本地收尾；已发布的 SQE 继续跟踪其提交与取消，不能当作未提交请求直接释放。后端在所有相关请求退休后只发布一次逻辑完成，避免请求地址或 id 过早复用。参考 [liburing cancel 合同](https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html)。

TTY 用单次 POLL_ADD 观察自有终端 fd 和 platform resize fd，read／write 使用 nonblocking syscall；timeout 用独立 TIMEOUT。任一结果选定后取消其余等待并消费它们的 CQE，全部退出才释放 buffer／owner；迟到 resize 仍为下一次 receive 保留。`None` 无限等待，零 timeout 只做当下查询，不创建无限 poll。现有 SIGWINCH self-pipe 仍归 platform，不安装第二个信号处理器。

DNS、目录遍历、路径操作、open、seek／truncate 和 `NativeWork` 使用按需启动的有限 native worker 集合。它只接受本库已有的 GC-free 工作与完成记录，不提供公共线程池 API；队列可撤销，运行中的 syscall 等真实返回。常规文件数据 read／write／fsync 和网络 I/O 实际走 ring，不能把新后端实现成全部交给 worker 的包装。

### 8.4 提交失败与 ring 销毁

操作 CQE 错误只结束对应操作。提交失败须区分未发布、已发布与内核已接受的请求；只有明确尚未提交的请求可以本地结束。EINTR 等可重试情况保留原请求，不重复发布 SQE。运行层失败停止新业务、记录错误并进入取消／排空流程，仍按目标 CQE 和 worker 完成释放借用，不能用一次 submit 错误假定所有在途操作都停止。

`io_uring_queue_exit` 仅在无在途请求时调用，不能当作同步“取消全部并立即释放 buffer”的入口：它解除映射并关闭 ring fd，内核部分退出工作异步执行。参见 [queue_exit 合同](https://man7.org/linux/man-pages/man3/io_uring_queue_exit.3.html)与 [Linux 6.1 退出实现](https://github.com/torvalds/linux/blob/v6.1/io_uring/io_uring.c)。无法确认原生结束时继续保留借用，不发布关闭完成；本设计不承诺内核／运行环境失效后的有界恢复，也不另建灾难恢复线程来绕过这个边界。

## 9. 现有能力到后端的完整映射

| 能力 | 共享语义 | Darwin | Linux |
| --- | --- | --- | --- |
| `run`／`attach`、任务投递 | 同一 root／owner／pump | pipe 通知 | eventfd 通知 |
| `sleep`、连接总期限 | 逻辑取消等 owner 移除，共享唤醒等 native 退休 | dispatch timer | TIMEOUT |
| TCP connect／accept／read／write | 地址、方向独占、短读写、部分进度 | socket syscall + source | CONNECT／ACCEPT／RECV／SEND |
| listen、bind、地址查询、shutdown | 既有 platform 规则；shutdown 提交后取消终止流 | owner 的原生控制队列 | owner 的短 POSIX 控制操作 |
| UDP send／receive | 完整数据报，含零长度、截断和地址 | sendmsg／recvmsg + source | SENDMSG／RECVMSG |
| 文件 read／write／sync | 每文件串行、真实位置、append、借用 | 后台 work | READ／WRITE／FSYNC |
| open／metadata／移动／删除／建目录／临时文件 | 路径、错误、迟到资源关闭 | 后台 work | native worker |
| seek／truncate／目录 open／next／close | 现有平台表示与枚举寿命 | 后台 work | native worker |
| DNS | `getaddrinfo`、原始 OsString、EAI 错误 | 后台 work | native worker |
| TTY bytes／resize／timeout | 单 owner、raw 恢复、非阻塞、End／resize 保留 | sources + timer | POLL_ADD + TIMEOUT |
| `NativeWork` | 单次 execute／release、等待借用结束 | 后台 work | native worker |
| socket／文件／TTY close | 等在途完成，消费资源一次 | 等 source／worker 退出 | 等 CQE／worker 退出 |

表中的短 POSIX 控制操作不得包含可能阻塞的 regular-file I/O 或默认带阻塞 linger 的 socket close。EAGAIN 按相应等待路径重新订阅；不把它当 EOF。UDP 不按字节流重发剩余后缀。网络写沿 platform 的 SIGPIPE 防护，不修改进程全局 signal disposition。现有 Happy Eyeballs、SOCKS5、TLS 和 HTTP 不另写平台分支。

## 10. FFI、GC 与构建

native I/O callback／worker 不执行 managed 代码，参数在提交前复制到请求自有 native 存储，或使用仍有效的 `NativeByteBorrow`。共享层的操作表保活 managed owner；native callback context 不能保存 movable 对象裸地址。

唯一新增的异步 native→managed 入口是宿主通知桥，使用已有 `ForeignCallback` 的 retain／release、线程 attach、Context 与异常捕获合同，不手工扩展 runtime 调度 ABI。注销完成后才释放注册持有的 callback 引用；当前调用的 lease 在回调返回时退出。宿主侧可能长时间运行／等待的 FFI 调用也遵守 NativeSafe 边界，不能阻塞在 `GCLeaf` 中。

Darwin 链接系统 libdispatch；Linux 固定 liburing 源码与普通 C helper 随现有 Cone native 管线构建，GNU 与 musl 不混用头文件。目标选择沿现有 manifest 条件输入与预编译 sysroot 机制，不修改编译阶段职责或增加构建来源证明。发布时重建受公共类名与 native 输入变化影响的普通 Cone；缓存仍只按已有输入更新规则决定复用。

## 11. 实施顺序与兼容性

1. 先按本设计更新正式生效的 asyncio 规范，保留本次提案与现状的区分；实施前不把新 API 描述为已可用。
2. 抽出共享 `LoopOwner`、操作／资源状态和一次非阻塞 pump；暂用内部 libuv 适配运行现有行为测试，确认这是同一套任务／取消算法。
3. 完成 Darwin libdispatch 的全部操作及 `run`，再加入 session mailbox、host 注销合同与 `MainDispatchHost`。关闭与回调寿命必须一起交付。
4. 完成 Linux ring、eventfd、worker 与取消退休，覆盖 GNU／musl；把同一个宿主协议接入真实 fd-watch 测试宿主。
5. 迁移现有 TLS／HTTP／工具、示例、测试和文档的 `LibuvEventLoop()` 构造；移除 libuv adapter、vendor 输入和失去消费者的 UV 错误转换，重建预编译资源并做三目标定向验收。

公共 I/O 方法与成功／取消／关闭语义保持；构造类名变化是显式源码迁移，Linux 的 io_uring 运行要求也是部署变化。底层错误 domain 从 Libuv 迁移到真实 POSIX 来源，不能把原始 UV 数字或文案作为兼容承诺。迁移不提高任何无关编译器或库的最低系统要求，不新建长期双实现选择开关。

## 12. 验收与设计限制

普通库行为继续在所属 Cone 的 `scoop.test` 中验证：所有现有 I/O 功能、同方向并发拒绝、各方向同时在途、短读写、零长度、EOF、数据报、timer、文件位置、目录、DNS、TTY、`NativeWork` 与错误路径。跨 TLS／HTTP 的组合用例复用现有测试，不为每个后端复制测试实现。三目标使用相同公共语义断言，只有后端原生错误与资源退休测试按平台区分。

竞争测试用明确屏障安排提交前取消、已提交取消、完成后取消、close／取消竞争、SQ 暂满、迟到资源成功、部分进度、源／取消 CQE 的不同顺序，以及关闭期间外线程投递；不靠极短 sleep 猜时序。GC 定向测试覆盖移动期间的 buffer owner 保活和 host callback 引用，不把 GC stress 施加到所有普通 I/O 测试。

宿主接入验证覆盖：安装不内联、首次回调前关闭、从外线程 post／close、根 Context 保留、回调抛错、嵌套 pump、空闲无轮询、连续 I/O 下宿主事件能继续推进、移除后无 callback、通知 fd 不提前关闭。真实外部主循环与 native callback 也通过测试 Cone 的 native helper 和 `scoop.test` 验证，不新增 fixture；保留源码与工具，复用正常系统库构建。

Darwin 用实际主 dispatch queue／CFRunLoop 验证主线程进入、等待与退出；未来 GUI 库用实际 AppKit 再覆盖菜单跟踪、窗口拖动和模态行为。Linux 先用真实 poll 型 fd-watch 宿主与 ring 共存验证，GUI 库接入时再测试实际 Wayland／X11 循环。前者不是完整窗口系统验收，不宣称本任务交付了 GUI 库。

Linux 在 6.1 基线与当前发行内核各验证 GNU／musl 所需行为；未开放 io_uring 的机器应报告未满足验收环境，不能把目标初始化失败当测试通过。补充 ring 禁用／缺能力的启动错误与回滚覆盖。

性能记录只用于判断后端是否适合 GUI：空闲 CPU／唤醒次数、post 到执行延迟、timer 偏差、连续 I/O 时宿主事件延迟、吞吐与关闭延迟，并与迁移前 libuv 对照。不预设新后端必然更快，也不为这些测量增加公共资源预算或程序合法性规则。

实现按第 11 节分批提交；各批的验证范围、失败闭合与设计限制见实施记录。

## 13. Windows：IOCP 后端

Windows TCP 的直接完成沿用第 7 节与 Linux 优化已经建立的 owner 批量提交边界。ConnectEx／AcceptEx 的 context 更新成功后，为交付的 stream 设置 FIONBIO；此时尚无业务传输，不改变已关联的 IOCP 或资源 owner。NativeSafe flush 先用不带 OVERLAPPED 的 recv／send 尝试传输；字节数、EOF 和实际错误写入批次结果，不分配请求、不经过收集线程、不通知 event。未就绪读取回到 WSARecv 请求及真实 packet 退休路径，不启用 skip-completion 模式。直接与等待完成共用 TCP 结果转换和读方向 EOF／错误状态，反向写入不改写它们；managed 仲裁与 ready 继续负责实际进度、buffer 借用、方向和 Job 的收尾。关闭与取消先 flush 暂存传输；native 直接完成不等于已经允许业务复用 buffer。UDP、连接、文件和 timer 的等待协议保持。

TCP 写入遇到 WSAEWOULDBLOCK 时，懒创建该请求的 WSA event，订阅 FD_WRITE／FD_CLOSE，并在清除旧事件后重试 send，避免注册边界丢失就绪。仍未就绪才安装一次性 thread-pool wait。callback 先释放自己的 wait 对象，再在既有 loop mutex 下发布原生完成节点；解锁后不再访问请求和 loop。owner 清除网络事件后再次尝试 send，必要时安装下一次等待。取消只标记原有请求并设置等待 event，等待 callback 的真实退休，不取消含用户数据的 WSASend，也不猜测取消后的发送量。owner 完成请求时解除 WSAEventSelect 并关闭 event；socket 关闭仍等待 active 请求归零。这样背压与取消共用既有请求登记、完成队列和生命周期，不使用轮询或专用阻塞 I/O 线程。

每个 loop 拥有一个 IOCP、一个不可继承的 manual-reset event 和一个原生收集线程。收集线程使用 GetQueuedCompletionStatus 等待，直接把请求内预留的完成节点入队，随后 SetEvent；无 I/O 或 timer 时没有周期轮询。owner 的提交、取消、解码和 continuation 恢复继续使用共享驱动。IOCP 句柄不直接交给 GUI；GUI 监听 event，按既有 remove 完成合同退出监听。

OVERLAPPED 在请求内稳定存放，owner 在提交前登记请求。收集线程只写完成 code／count 和原生队列，owner 取出后才执行具体操作的收尾。同步成功不启用 skip-completion 模式，仍等 packet；同步失败仅在 API 明确不会投递 packet 时本地收尾。`CancelIoEx(handle, &overlapped)` 只请求取消，不关闭健康资源，也不退休借用；ERROR_NOT_FOUND 是与真实完成的正常竞争。依据 [IOCP](https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports)、[完成获取](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getqueuedcompletionstatus)和[取消](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)合同实现，不用取消成功冒充数据完成。

原生队列发布、SetEvent、ResetEvent 与队列取快照在同一 SRW lock 下完成，外线程命令唤醒也经过该边界；owner 清除旧通知后产生的新通知不会丢失。取走的一批有限工作处理完后返回宿主，期间产生的工作保留下一轮通知。借出的 event 由 loop 独占重置和关闭，host 不消费其状态；不能使用会被 host 等待自动消费的 auto-reset event。

timer 使用 QPC 截止时间，由收集线程的有界毫秒等待推进；插入更早 deadline 时通过 PostQueuedCompletionStatus 唤醒收集线程。超时后重新核对 QPC，不提前完成；超长等待按 DWORD 可表示的有限片段推进。timer 移除与到期入队受同一锁保护，取消没有额外 callback 或悬空 context。收集线程只做内存／队列工作，不能在其上执行阻塞文件、DNS、用户 NativeWork 或 managed continuation。

不可 overlapped 的操作使用与 Linux 相同宽度的按需 native worker 集合，队列取消跳过尚未执行的工作；运行中的工作真实返回后发布结果。worker 启动失败回滚已启动的空闲线程，通过通知异步退休，不在 owner 上 join。IOCP 请求、timer、work 与完成全部排空后再请求收集线程／空闲 worker 退出；它们最后一次访问 loop 的发布边界结束后才允许销毁。宿主注销、event 关闭和 onClosed 保持共享顺序。

直接创建／poll／销毁错误使用普通 C-layout 的 domain／code 结果；异步完成用已有完成记录的错误 kind 和 code。Win32 DWORD 原位保存，Winsock 提交错误保留 Winsock，CRT errno 保留 PosixErrno；不把 HRESULT／NTSTATUS 猜测成 errno。平台错误记录与事件分支的扩展不改变 runtime ABI。

Windows 普通测试复用同一 Cone，原生定向测试覆盖真实 overlapped 完成、取消与迟到成功、借用退休、timer、空闲不唤醒和 worker 屏障。宿主测试使用真实 Windows event 等待验证安装不内联、重入、跨线程 post、注销前保留 HANDLE 与注销后无回调。端口收集、文件／socket／TTY 和完整消费按 Windows 进度分批记录，不能由底层组件通过推断整个库已可用。

网络端点保留完整 SOCKET 与各一个输入／输出请求。bind／listen 复用 platform 的 owned socket helper，connect 自建同族 socket 并先 bind；所有 socket 都以 overlapped、不可继承方式创建。ConnectEx 与 AcceptEx 使用实际 provider 的扩展入口，accept 的 receive length 为零，接收地址缓冲留在稳定请求内；在 SO_UPDATE_CONNECT_CONTEXT／SO_UPDATE_ACCEPT_CONTEXT 成功后才交付 stream。数据完成在 owner 用 WSAGetOverlappedResult(FALSE) 取得 Winsock code／flags，失败时保留 IOCP 已给出的进度；UDP 截断按 WSARecvFrom 的完整缓冲前缀规则处理。参见 [ConnectEx](https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nc-mswsock-lpfn_connectex)、[AcceptEx](https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nf-mswsock-acceptex)、[WSARecvFrom](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsarecvfrom) 与 [WSAGetOverlappedResult](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsagetoverlappedresult)。

DNS、路径修改和目录枚举继续使用共享 worker 生命周期。平台条件的字符串入口在 Windows 借用 UTF-16，在 POSIX 借用原始字节；请求在提交内复制，完成后释放。原生目录复用 platform owner，next 按实际长度复制独立、终止的名称，允许 managed 在原目录关闭后消费；错误同次取回，Windows 为 Win32。DNS 复用 platform resolver，结果地址的 C-layout 同次带回 Winsock／PosixErrno，取消后的地址链仍经正常消费释放。

文件 owner 保存独立 HANDLE，普通文件的串行位置显式写入 OVERLAPPED，按完成长度推进；同步成功仍等 packet，EOF 不变成失败。append 使用同步句柄与现有 worker，从而由 OS 同时维护追加位置，避免在异步追加后查询 EOF 误认其他写入的位置。其余 metadata／truncate／sync／close 通过 platform 的结果 helper 执行。参见 [ReadFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile)、[WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile) 和 [同步／异步文件位置](https://learn.microsoft.com/en-us/windows/win32/fileio/synchronous-and-asynchronous-i-o)。

Windows 临时创建在 worker 内取得规范父目录，拼接完整前缀与随机后缀，再以 CREATE_NEW 创建 overlapped 文件；创建后复用同一 HANDLE，不重新打开路径。内部临时结果包含 owner、UTF-16 路径及关联状态，创建已成功但 IOCP 关联失败时交由原有 close／delete 清理流程报告错误；成功交付的 close 仍不删除路径。熵源沿 platform 的 BCrypt helper，完成记录增加 NtStatus 分支，保持原码。

TTY 复用 platform 的独占 console owner；输入 worker 用 WaitForMultipleObjects 同时等待 console input 与单次取消 event，再用非等待输入转换取得字节、resize 或 EOF。截止时间与 timer 共用 QPC 算法，在提交时计算，超时唤醒后重新核对。输出复用已有 worker 的同步 WriteFile，取消排队工作可跳过写入，已运行工作实际返回后才释放借用。close 取消输入等待和排队输出，等待两个方向完成后恢复模式并关闭句柄；worker 不调用 managed 代码。控制操作直接返回 Win32／PosixErrno 记录，不用 captureErrno 捕获 Win32 错误。参见 [console 输入等待](https://learn.microsoft.com/en-us/windows/console/low-level-console-input-functions)与 [console handles](https://learn.microsoft.com/en-us/windows/console/console-handles)。

### 4.3 B26：资源持有字节传输等待状态

B25 的分层实验把主要用户态差距定位到通用 managed I/O 和协程恢复。字节传输首先改为资源持有的等待槽：复用操作身份与完成登记，按 Job 复用取消监听，每次只填入实际 buffer 借用和 continuation。资源的方向接纳／释放直接改变同步状态，去掉每次 reservation 的对象与正常完成后的关闭检查投递；真正关闭仍等所有借用与 native 请求退休。

槽的本次传输代次与 Job 绑定代次分别承担实际的竞争边界：旧取消回调检查绑定，排队撤销检查传输代次。取消错误通过选定／发布状态交付；正常完成选定后不再被迟到取消覆盖。槽在解除借用及资源忙状态前清空本次捕获，允许恢复回调马上启动下一次使用，不把旧操作的尾部清理留到复用之后。native 编号在实际退休后复用，资源关闭时清除持久登记。公开 suspend API、每次 buffer 借用、原生真实退休以及结构化任务合同保持。

此次只重构字节传输的 managed 生命周期，后端共享原来的真实 I/O 实现。以完整 TCP 程序与 B24 固定产物、Go 的吞吐和 CPU／echo 比较，不以减少某个字段或分配小项作为完成依据。

### 4.4 B27：并行 reactor 上受控恢复

B26 完整协程的用户态 CPU 仍明显高于直接使用同一字节等待槽的回调程序。隔离实验分别展开多层 suspend 调用、在 reactor 完成栈内恢复、增加 owner 本地恢复槽：本地槽只改善单连接，高并发没有收益；就地恢复改善并行场景，但单 worker 回退。因此先改变并行恢复交接，不继续微调队列槽，也不把展开后的诊断程序作为公开 API 的替代品。

core 的 ExecutorTask 增加普通 dispatchResume 方法，默认仍排队；只有旧 drive 已完成 frame／Context／catch 收尾并交出执行权的发布者使用它。startDispatchedCoroutine、withExecutor、yield 及旧 drive 退出时已经锁存并由其领取的外部恢复仍用强制排队的 dispatch。编译器生成的 adapter、execution 原子发布协议和 runtime ABI 不变。

MultiThreadExecutor 只在本池多个 worker 的 driver.poll 窗口内直接执行恢复，每轮最多 32 次，执行中的嵌套恢复继续进入原有无锁 FIFO／inbox。单 worker、外线程、其他 executor、普通用户任务和非 poll 的 driver 阶段都继续排队。直接恢复复用相同执行入口、ticket、TaskContext 和 managed 根；不增加生命周期或私有优先队列。I/O 在交付前已经结束本次借用及资源占用，产生的新 ready 留到下一轮，因此直接恢复不递归 pump，也不从提交栈交付完成。

此变化减少完成事件到可运行任务的入队、唤醒和迁移，但保留协程 frame／adapter 的成本。验收同时比较单／多 worker、单／多连接的完整公开 API 程序、固定回调程序和 Go，后续依据剩余 CPU 差距选择下一大项。
