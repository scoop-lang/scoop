# Scoop 异步 I/O 库规范

版本：M49／M54／M59。本文规定普通 Cone `scoop:scoop.asyncio:0.1.0` 的合同；既有能力三目标验证见 [M49 验收](../milestone49/ACCEPTANCE.md)，第 9 节扩展见 [M54 实施记录](../milestone54/PROGRESS.md)，第 10 节 TTY 见 [M59 实施记录](../milestone59/PROGRESS.md)。

第 1～10 节规定公共 I/O 语义，第 11 节规定平台后端与宿主接入合同。实施状态与验证范围见 [实施记录](../asyncio-backends/PROGRESS.md)。详细拆分、平台映射与验收见 [EventLoop 后端设计](../asyncio-backends/DESIGN.md)。

## 1. 分层与目标

源码位于 `sysroot/src/scoop.asyncio/`，显式依赖 core、std、platform、concurrent，测试依赖 scoop.test；公共声明属于 `scoop.asyncio`。不默认导入，不引入 compiler-known role、runtime 调度入口或新的 ABI。任务树与 execution 使用语言规范 11.9.1～11.9.3；阻塞池和跨 executor 同步使用[并发库](SCOOP-CONCURRENT-SPEC.md)，路径、地址、错误、时间和 buffer 直接复用平台库与标准库。

库内 C shim 的完成记录 kind 与 Scoop 解码保持一致：既有通用／网络事件保留原编号，使用 `0x000–0x0ff` 段；文件／目录事件使用 `0x100–0x1ff` 段，依次为 Metadata `0x100`、Position `0x101`、Temporary `0x102`、DirectoryName `0x103`；TTY 事件使用 `0x200–0x2ff` 段，Resize 为 `0x200`、Timeout 为 `0x201`。两类新增事件分别在自己的段内追加，保留已有编号；未分配值仍按非法完成记录处理。此编号仅用于该 Cone 的 native 接口，不属于 runtime ABI 或公共 Scoop 枚举身份。

首版支持 aarch64-apple-darwin、x86_64-unknown-linux-gnu、x86_64-unknown-linux-musl；三个 Windows target 的扩展按[Windows 进度](../windows-targets/PROGRESS.md)分批实施，未完成前不声明整个库可用。PlatformEventLoop 由目标平台选择后端：Darwin 使用系统 libdispatch，Linux 使用第 11 节规定的 io_uring，Windows 使用 IOCP。C11 shim 只管理 native I/O、稳定存储及完成记录，OS callback 不进入 managed 代码，阻塞任务由 managed worker 调用同步 shim。TLS、HTTP 属于各自库，文件监听、进程和任意 fd 接管不在本版范围；M54 为实际缓存与下载补齐第 9 节的基础能力；M59 增加第 10 节的自有 TTY。

Linux C 工具链同时需要所选 libc 的开发头文件和对应架构的 Linux UAPI 头文件（包括 linux/errqueue.h）。只带 musl libc 的 musl-gcc 包装器须补齐内核头文件搜索路径，可通过普通 --cc 包装脚本配置；不把 glibc 用户态头文件加入 musl 的搜索路径。

## 2. 根入口与事件循环

```scoop
public interface EventLoop {
    public val executor: CoroutineExecutor
    public suspend fun sleep(duration: Duration)
    public fun schedule(duration: Duration, completion: (Throwable?) -> Unit): Disposable
    public suspend fun resolve(host: OsString, port: Int, family: AddressFamily? = None): Array<SocketAddress>
    public suspend fun connectTcp(address: SocketAddress): AsyncTcpStream
    public suspend fun listenTcp(address: SocketAddress, backlog: Int = 128, reuseAddress: Boolean = false): AsyncTcpListener
    public suspend fun bindUdp(address: SocketAddress, reuseAddress: Boolean = false): AsyncUdpSocket
    public suspend fun openFile(path: Path, options: OpenOptions = OpenOptions()): AsyncFile
    public suspend fun openTty(): AsyncTty
}
public class EventLoopBuilder public constructor() {
    public fun blocking(configure: BlockingPoolBuilder.() -> Unit)
    public fun blocking(pool: BlockingTaskPool)
}
public fun eventLoop(configure: EventLoopBuilder.() -> Unit = {}): PlatformEventLoop
public class PlatformEventLoop : EventLoop {
    public fun <T> run(block: suspend CoroutineScope.() -> T): T
}
public fun currentEventLoop(): EventLoop
```

`run<T>` 是具体 final class 的同步根入口，不是 interface 的泛型虚方法；不保留 EventLoop.run 或 runWithResult 别名。调用方不得已有当前 Job 或活动 EventLoop binding。实例只接受一次运行；并发、重入、运行后重用均报 IllegalStateException，拒绝的调用不影响已接受运行。工厂只执行一次配置并保存快照，接受 run／attach 后才建立 native loop 和自有阻塞池；初始化失败清理已创建资源并关闭实例。blocking 默认自有 4 个 managed threads；两个 overload 分别配置自有池／借用池，最后一次选择生效。数量在创建线程前校验，捕获 builder 后的修改不影响实例。

运行建立精确静态类型为 EventLoop 的 Context binding，通过 startDispatchedCoroutine 在内部 LoopExecutor 上排队启动 coroutineScope。正文 receiver 是该作用域，第一次执行前已具有正确 Job 与 scope。根及全部后代完成后请求 LoopExecutor draining；已接纳的低层任务及其 child、coroutine work 和 blocking runner 排空后，再关闭本次未关闭资源、排空 I/O 操作与内部 ready。最后停止自有 blocking pool，等待其退出执行循环，关闭 native loop 并 join 已结束的 worker；借用池不关闭或 join。run 仅在全部收尾成功后返回 T；非取消异常优先于取消，正文与清理均失败按 ResourceUseException 组合。逃逸资源引用不能延长运行，run 返回后它们已经关闭。

每次用户 I/O 要求当前 Job 和所属 loop 的活动 binding；显式调用其他 loop 或已结束资源报状态错误。EventLoop binding 表示资源服务归属，不表示调用者正在 owner 线程；保留该 binding 的其他 executor worker 可建立请求，资源 phase／busy／active 和登记使用正确同步，native submit 与原生容器更新仍发给 owner。close 允许在已取消 Job 中进行，且关闭后再次 close 是空操作。`currentEventLoop` 缺少 binding 时抛 MissingContextException。executor 属性在运行／附着已建立且 LoopExecutor 未终止时返回 owner executor，之前或之后报 IllegalStateException；withExecutor(loop.executor) 可显式执行 owner 上的代码。

资源与操作由共享 `LoopOwner` 持有，不依赖具体 EventLoop 类。一次非阻塞处理只消费进入该轮时取得的命令和 native 完成快照，再恢复本轮 ready；恢复期间产生的命令和 ready 留到下一轮，嵌套处理不递归进入。同步驱动反复调用同一流程，只有无可执行工作时才等待。

命令和 ready 的批次存储由 owner 复用。外部命令在 inbox 同步边界交换接收与消费容器；本地命令在 owner 上交换，不再复制到外部命令容器。每轮先消费外部快照，再消费本地快照；完成处理后交换 ready 批次。处理期间产生的工作进入下一接收容器，不得因本轮清理而丢失。消费结束或回调抛出时清空本轮容器中的引用，容量保留给后续批次；空轮不新建容器或迭代器。可复用容器只在 owner 放弃本轮访问后才重新作为接收容器，外部生产者不访问正在消费的批次。

操作对象直接参与 ready 排队；需要跨线程提交的操作与普通命令共用队列，不为提交／交付各自建立捕获该操作的包装闭包。AsyncRuntime 在资源所属 worker 上直接登记操作并开始 native 请求，不再把本地操作排队发给自己；其他 worker／外线程以及独立 PlatformEventLoop 的操作仍经命令队列交付 owner。本地开始以当前 owner 执行点为序，不与尚未消费的普通命令建立额外总序；排队命令自身的顺序保持。直接开始不递归 pump，不同步调用 continuation 或完成回调；同步完成和失败也进入 ready，ready 消费期间产生的完成留到下一轮。原生请求仍在 worker 推进时批量提交，不逐操作强制刷新内核队列。

TCP、文件字节传输与 TTY 写入使用资源持有的可复用等待槽，实现 `SuspendRegistration<Long>`。TCP 读／写各有一槽，文件两个方向仍共用同一资源忙状态，TTY 写入有自己的槽。等待槽同时持有本次借用、continuation、结果仲裁与原生提交／取消状态；不为每次传输新建操作对象、方向释放对象或另一套清理闭包。每次调用仍独立取得和解除 buffer 借用，不把借用扩大到连接生命周期。真正退休后先提交实际进度、清空本次捕获、解除借用与方向占用，再恢复调用方；恢复可立即开始下一次调用，旧交付栈不得再访问已让出的本次状态。准备／登记失败完整释放已经取得的资源；零长度完成同样的状态、范围和取消检查，不登记 native 请求。

等待槽的取消监听按当前 Job 绑定复用；换 Job 或资源关闭时解除旧绑定。没有活动传输时，Job 取消不提交 native 撤销；新的调用仍检查当前 Job。旧绑定回调不能选择新 Job 的结果，已排队取消命令只作用于其捕获的本次传输代次，不能撤销后来复用该槽的传输。取消／关闭与完成在同一状态边界选择一次结果；错误引用先完成发布，交付方再读取，不把未完成的引用写入视为可交付结果。

等待槽首次提交后可在 owner 的完成登记中保留至资源关闭，native 编号在该槽内复用；只有前一次真实 native 退休、ready 交付及借用结束后才能提交下一次，同一编号不同时表示两笔在途请求。资源的运行／关闭、方向占用和输出结束在一个同步状态边界转换，字节传输不逐次建立独立 reservation 对象。关闭停止新接纳，等活动等待槽、原生请求和借用排空后解除完成登记及 Job 捕获；普通完成不额外排队执行无事可做的关闭检查。其他操作继续使用同一 owner、native 批次与 ready 机制，不另建调度器或资源生命周期。

完成结果由具体操作持有，公共操作基类只提供 owner、编号与排队／完成接收合同；一次性回调操作和资源持有的字节等待槽分别保存自身所需的仲裁、订阅与交付状态，不为每种操作保留完整的通用 native 结果布局。字节传输在 owner 选定结果后保存实际传输量与待交付异常；错误转换失败同样保存为异常，不能跳过借用解除。完整 native 结果只由需要它的回调操作保活。结果保存不调用 continuation 或业务回调，进度提交、借用和方向释放、恢复仍在本轮 ready 交付时按原次序执行；成功、取消和关闭的仲裁规则不变。

native 完成以有界批次跨 FFI 交付，不逐条调用 native 取出。后端把已退休请求的完成记录复制到 loop 自有、可复用的 GC-free 连续数组，并在本轮完成交付后统一释放对应请求记录；当前轮尚未执行的取消／关闭命令仍可访问请求相关端点。数组内容在下一次取批次前保持不变。managed owner 在当前 pump 中按序读取该快照，完成记录中的独立资源继续沿既有交付／迟到关闭协议处理。剩余完成留在队列，当前批次结束后重新通知；不因批次容量丢弃完成或提前解除在途 buffer 借用。每轮结束统一发布仍需推进的 managed／native 工作。阻塞请求提交后仍立即交给共享池，不能推迟到可能执行用户回调的整轮结束。

Linux 的原生提交队列和逻辑完成队列均由 owner 更新：提交、取消及完成处理不逐请求通知自己，由 pump 结束边界根据真实剩余工作统一 rearm。内核 CQE、阻塞 worker 完成及外线程命令仍各自发布持久通知；此边界不能吞掉先于等待到达的工作。Darwin／Windows 的外线程完成生产者保留原同步与唤醒，取出与消费同样使用批次接口。pump 结束、GUI 返回宿主和 worker 进入等待前均满足这个进度合同。

Linux 的一次 native 推进先消费已有 CQ 快照，再批量发布待提交请求，随后消费提交后已经可见的 CQ 快照，使本次提交的即时完成能进入同轮 managed 完成批次。两次消费均以当时 CQ 数量为界，不循环等待新完成；后一次消费产生的后续提交留给下一轮并按真实剩余工作通知。数据请求与取消控制请求仍分别退休，不能因同轮完成而提前释放在途借用或请求记录。

后端提交与完成消费在 run／attach 的 owner 线程执行；原生 I/O callback 与 worker 只生产完成。外线程提交和取消先进入受锁保护的命令队列，在同一发布边界通知后端 fd；关闭在该边界停止接受投递。LoopExecutor 的 work 使用同一内部命令队列与唤醒，不能使用可丢弃应用消息的 session.post；活动登记的工作不能丢弃。驱动交替处理命令、完成记录与本轮内部 ready，完成进度、资源交接和借用解除后在锁外调用 continuation，由 continuation 排队回自己的 executor。LoopExecutor 上的 Channel、mutex 和第三方挂起点同样回 owner；其他 executor 上的 I/O 回到原 worker executor。排队 work 的未捕获 completion 异常遵守语言规范 11.9.3，报告并终止进程。有待处理工作时不得阻塞；同步 run 通过 NativeSafe 等待通知 fd，attach 由宿主通知推进，通知可合并。

共享运行状态按职责同步：启动时一次发布、之后不变的 binding 与 execution 供 worker 以 acquire 读取，不与每次命令投递共用全局锁；结束时只停止接纳，不并发改写这些已发布引用。操作编号独立原子分配。命令队列、通知资源的有效期与停止接纳由独立 inbox 协调，竞争线程通过平台 mutex 的 NativeSafe 等待让出 CPU，不能在包含 native 唤醒的临界区外无界自旋。命令从空变为非空时发送一次通知，同一待消费批次的后续投递合并通知；入队、通知与停止接纳仍在同一边界，避免后端销毁后的访问。owner 取走一批后重新允许通知，并在本轮退出时检查真实待处理工作，防止清除旧通知时漏掉新批次。根结果／清理状态使用自己的同步，ready、操作表和资源表继续只由 owner 更新，不在其锁内执行用户回调。

### 2.1 多线程根运行

```scoop
public class AsyncRuntimeBuilder public constructor() {
    public var workers: Int
    public fun blocking(configure: BlockingPoolBuilder.() -> Unit)
}
public fun asyncRuntime(configure: AsyncRuntimeBuilder.() -> Unit = {}): AsyncRuntime
public class AsyncRuntime {
    public fun <T> run(block: suspend CoroutineScope.() -> T): T
}
```

AsyncRuntimeBuilder 的 workers 默认取逻辑处理器数的合法正 Int 表示，blocking 默认 4 个 managed threads；配置执行一次并保存快照，所有数量在创建线程前校验。工厂只保存配置，run 接受单次运行后才创建共享 BlockingTaskPool、每 worker 一个 native reactor，以及借用该池的 MultiThreadExecutor。根入口拒绝当前 Job／EventLoop binding，并发、重入和完成后重用均报 IllegalStateException，拒绝不影响已接受运行。全部 reactor 建立后才启动 coroutine workers；初始化失败回收已建立资源，部分线程启动失败先停止并 join 已启动线程，再收尾未启动的 reactor。

调用 run 的线程等待结果；用户根及默认子任务在 MultiThreadExecutor 上执行，各 worker 同时驱动自己的 reactor。运行只有一个逻辑 EventLoop binding 和一棵结构化任务树；currentEventLoop().executor 返回共享 worker executor，不提供固定 owner 线程承诺。独立 PlatformEventLoop 与 GUI attach 保持 2.1 的 owner executor 合同。普通资源创建优先选择当前 runtime worker；不在本池 worker 上的调用按轮转分配。单个 listener 接收的新连接在交付前轮转分配到各 reactor，应用无需多个端口或手工分片。

资源的 native owner 在创建时确定，已有 handle、在途请求与借用不随任务窃取迁移。accept 请求仍由 listener owner 提交并退休，新 stream 从首次登记／IOCP 关联起属于目标 owner；目标 owner 接纳资源或关闭迟到成功后才恢复调用方。每个 reactor 由自己的 worker 串行推进；本地操作直接开始，普通命令本地排队，跨 worker 提交与取消通过 inbox 发布。完成在所属 worker 上进入现有 executor 的本地队列，任务仍可被窃取。worker 在有限批任务之间处理 I/O／timer，持续可运行任务不能饿死 I/O；空闲时发布观察状态并重查任务与 I/O，再通过可被任务投递打断的 native 等待休眠。所有 reactor 和通用 blocking 共用本次 runtime 的阻塞池。

根及结构化子树结束后先请求 worker executor draining，各 worker 继续推进 I/O，并保持共享阻塞池接纳。全部执行登记归零且已接纳的队列工作排空后，各 worker 才关闭所属遗留资源、排空实际 native I/O 和 ready、销毁 reactor 并退出。全部 reactor 收尾和 coroutine workers join 后，最后停止并 join 共享阻塞池。正常结果 T 或最终异常在这些步骤之后返回，非取消异常优先于取消，清理失败遵守 ResourceUseException；低层独立任务同样计入关闭条件，不能只等根 Job。任一 backend 失败取消同一个根 Job，但仍执行全部 reactor 的收尾。AsyncRuntime 不增加 attach 入口；宿主附着继续使用 eventLoop。

## 3. 取消、完成与关闭

直接 I/O 属于提交时的当前 Job，不为每次操作创建子任务。登记前已取消则不提交。操作完成与取消在 managed 状态中选择一次结果；native 结束是独立事实。借用解除、实际进度提交及未交付资源清理完成后才恢复，包括取消结果。成功先选定则后来的取消不覆盖成功；取消先选定则迟到成功只执行清理。撤销提交或控制请求的完成不替代实际数据请求的退休。

取消须等待真实原生操作停止：Darwin 收齐 source cancel handler 与正在执行的 work；Linux 消费目标及取消控制请求的 CQE。共享定时唤醒自身也遵守该退休条件；逻辑计时等待按第 5 节从 owner 队列移除后完成，不等待其他使用者共享的唤醒关闭。单独取消 TCP read／write、UDP receive 不关闭健康资源，TCP write 保留真实部分进度。shutdownOutput 提交后的取消终止整个 stream。UDP send 提交后等待真实结果并保留已发送长度；文件／DNS work 队列可撤销，已执行 syscall 必须等 worker 完成。迟到 open／connect／accept 成功先关闭未交付资源。

close 拒绝新操作，将未选定结果的在途操作以 IoClosedException 结束；先获胜的 Job 取消不被覆盖。close 不隐含 flush、sync 或对端确认，等待所有操作、借用和 native 退休后完成；并发 close 等同一次收尾，后续重复 close 不重报旧错误。文件须先等在途请求与活动 worker，再执行一次 raw close，不能重试已消费 fd。EventLoop 收尾继续关闭所有资源，即使某项失败。

```scoop
public class IoClosedException public constructor() : Exception
public interface AsyncCloseable { public suspend fun close() }
public suspend fun <R, T> R.useAsync(block: suspend (R) -> T): T where R : AsyncCloseable
```

useAsync 与 core use 一样在正文与清理均失败时抛 ResourceUseException。同步对应协议为 core Closeable.close；异步关闭继续使用独立 AsyncCloseable，不通过同步 Disposable 表达。GC hook 不承担驱动、取消、恢复、flush 或等待；正常生命周期由 close 与 run 收尾保证。

## 4. 流、buffer 与组合

```scoop
public interface AsyncByteSource {
    public suspend fun readInto(buffer: NativeByteBuffer, count: Long): Long
}
public interface AsyncByteSink {
    public suspend fun writeFrom(buffer: NativeByteBuffer, count: Long): Long
    public suspend fun flush()
}
public interface AsyncByteStream : AsyncByteSource, AsyncByteSink {}
public suspend fun AsyncByteSource.readInto(buffer: NativeByteBuffer): Long
public suspend fun AsyncByteSink.writeFrom(buffer: NativeByteBuffer): Long
public suspend fun AsyncByteSource.readFully(buffer: NativeByteBuffer, count: Long = buffer.writableBytes)
public suspend fun AsyncByteSink.writeFully(buffer: NativeByteBuffer, count: Long = buffer.readableBytes)
public suspend fun AsyncByteSource.copyTo(destination: AsyncByteSink, bufferSize: Long = 8192L): Long
public suspend fun AsyncByteSource.readAllBytes(): Array<Byte>
public suspend fun AsyncByteSource.readAllBytes(maxBytes: Long): Array<Byte>
```

count 非负且不超过 buffer 当前可用区间。零长度字节流请求在状态及取消检查后返回零，不执行 native I/O、不表示 EOF。正 read 允许短读，零表示 EOF；正 write 允许短写，零抛平台 WriteZeroException。readFully 不足抛平台 UnexpectedEofException；各扩展保留已提交进度，不回滚或重发。copyTo 不接管任一端关闭责任，不自动 flush。

省略 count 的基本操作扩展取 buffer 当前可用长度后转发。readFully／writeFully 先检查取消与 buffer 范围，零长度不调用端点；copyTo 要求 bufferSize 为正，内部 buffer 在退出时释放。组合扩展检查端点返回的进度在本次请求范围内，不把非法返回值当作正常结束。

readAllBytes 从当前位置读取到 EOF，以至多 8192 字节的原生缓冲处理短读，返回剩余内容的独立数组；它是调用者显式要求的内存汇集，不预先读取文件大小或接管输入端关闭责任。每次读取前及接受实际结果前检查取消，输入异常及取消保留实际原因，不返回伪装成完整结果的部分数组。底层调用实际结束后才释放内部缓冲；UTF-8 或其他字符集解释由调用者使用现有文本能力完成。

带 maxBytes 的重载要求上限非负，参数错误在读取和分配缓冲前报告；无参数入口保持显式读取全部内容的行为。结果至多包含 maxBytes 个字节，空输入在零上限下成功；恰好达到上限后仍需确认 EOF。为了区分完整内容与超限，最多额外读取一个字节，超限抛 ReadLimitException 并公开 maxBytes，不交付部分数组。输入实际位置保留在已读前缀之后，不回滚或关闭来源；调用者可继续读取剩余内容。计数和探测大小避免 maxBytes + 1 的溢出，Long.MAX_VALUE 也是合法上限。接受迟到结果时若已取消，等待真实借用结束后交付取消，不把迟到 EOF 或超限改成成功。

read 推进 writer，write 推进 reader，返回前解除平台规范 4.5 的 NativeByteBorrow。取消／关闭也提交已知实际前缀。单次 native 调用选择后端可表示的合法前缀，不溢出截断。借用期间拒绝释放、扩容、清空、游标变更及重复借用；native 只保存稳定 allocation 指针，managed 表保活 owner。路径、DNS 等 managed 字节在提交 worker 前复制到请求自有 native 存储。

AsyncByteStream 允许使用不同 buffer 各一个 read 和 write 同时在途，同方向并发报状态错误。flush 负责本层已接受的积压输出向下游推进，不表示持久化；无用户态缓存的原始 TCP／文件仅检查状态及输出方向空闲。封装层可拥有自己的缓冲及关闭责任，接口不要求 fd 或 TCP 能力；可实现需要双向进展的后续 TLS stream。

## 5. 定时等待

sleep 使用 Duration，负值非法，零在取消检查后立即完成。正时长从调用时的单调时刻计起，排队时间计入等待；到期核对截止时间，不提前报告完成。不承诺精确唤醒时刻。取消与到期选择一次结果，取消先选定时由 owner 移除逻辑登记后恢复 CancellationException；不等待其他计时使用者共享的 native 唤醒关闭。

`schedule` 是框架超时使用的一次性定时通知，沿用当前活动 Job 与 EventLoop 检查。非负时长从调用时的单调时刻计起；包括零时长在内，completion 都只能在后续 owner 轮次调用，不在登记栈内调用。到期成功传入 None，native 启动或完成失败传入原始错误；最多通知一次。通知是普通非挂起函数，不建立 coroutine、execution 或结构化子 Job，不捕获 Context，在 owner 上且在所有内部锁外执行。通知应只更新状态、取消所属 Job 或通知已有等待者；抛出的异常记为 loop 失败并取消根，仍完成正常清理。

返回的 Disposable 可在任意线程幂等撤销。撤销先于回调选定则不调用 completion，并立即释放其捕获；已经选定的回调允许结束，dispose 不等待它，消费者以自己的状态边界处理竞争。撤销和所属 Job 取消都不通过抛 CancellationException 完成定时通知。通知本身不延长创建者的结构化任务；调用方在操作结束时 dispose，loop 退出时撤销全部遗留通知并排空 native 资源。使用者需要挂起等待、取消异常或继承 execution 时仍使用 sleep。

sleep 与 schedule 共用 owner 的逻辑期限队列。登记和撤销只通过命令发布，队列以期限排序并支持直接移除已撤销登记，不把失效节点及其捕获保留到原期限。每轮在处理已有 ready 之后，以一次单调时钟快照选定到期项，通知进入下一轮 ready；通知产生的新登记不能在当前轮递归执行。极长合法 Duration 不因转换为绝对时间或总纳秒数而溢出。

逻辑队列只为最早期限维护一个当前 native 唤醒，复用现有后端 timer 和 LoopOperation 退休协议。新期限更早时替换唤醒，旧唤醒取消后仍保活至实际退休；移除最早项或队列暂时变空时，可保留已经安排的较早唤醒，醒来核对真实队列，不产生提前通知或持续空转。native 唤醒不保留某个调用者的 continuation 或回调捕获。当前唤醒启动／完成失败时，尚未选定结果的逻辑登记接收该错误，已选定取消不被覆盖；已替换或关闭的旧唤醒不能结束新登记。loop 清理时移除遗留逻辑登记，取消当前唤醒并等待全部 native 退休。该共享层不改变后端 ABI，Darwin／Linux／Windows 继续分别用 dispatch timer／TIMEOUT／QPC timer 提供唤醒。

## 6. 网络

```scoop
public interface AsyncTcpStream : AsyncByteStream, AsyncCloseable {
    public val localAddress: SocketAddress
    public val peerAddress: SocketAddress
    public suspend fun shutdownOutput()
}
public interface AsyncTcpListener : AsyncCloseable {
    public val localAddress: SocketAddress
    public suspend fun accept(): AsyncTcpStream
}
public interface AsyncUdpSocket : AsyncCloseable {
    public val localAddress: SocketAddress
    public suspend fun sendTo(buffer: NativeByteBuffer, destination: SocketAddress, count: Long): Long
    public suspend fun receiveFrom(buffer: NativeByteBuffer, count: Long): Datagram
}
```

TCP EOF 不关闭输出。shutdownOutput 属于输出方向，提交后禁止新 write；成功后输入仍可读。真实读错误成为该方向终态，不重启失效的读订阅。listener 同时一个 accept，无等待者时保留后端待 accept 状态，不建立 managed 预接收队列。关闭 listener 不关闭已交付 stream。地址属性是创建时取得的值，不跨线程访问 native handle；这与 platform 同步 socket 每次调用 localAddress()／peerAddress() 查询 OS 状态的合同不同。

listenTcp 的 backlog 必须为正。监听使用 platform 的 socket_bind helper 按 reuseAddress 配置和绑定新的 owned fd，再 listen。失败时关闭尚未交付的 fd；错误保留 PosixErrno，不接管用户已有 socket。

UDP 每方向一个在途操作，发送保持完整 datagram，不拆包；空发送是真实空包。receive 要求正容量，有源地址的零字节是空包，无地址的零通知继续等待。Datagram 保留来源、实际交付长度及截断标志。超大报文保留实际错误。

bindUdp 同样先由 platform socket_bind 配置 reuseAddress 并绑定新的 fd，再交由后端持有；后端不隐式开启 SO_REUSEADDR。移交前后失败的关闭责任与 TCP 相同。sendTo 使用单次 sendmsg／SENDMSG 发送完整报文，成功完成表示报文已交给 OS；取消仍等待真实结果并提交实际发送长度，不能把取消当作撤回报文。

后端错误保留 NativeErrorDomain.PosixErrno 与实际 errno；TCP read 的零结果按 EOF 解释，库主动取消／关闭不伪造 OS 错误。Linux 网络输出使用 MSG_NOSIGNAL，Darwin 输出前设置 SO_NOSIGPIPE。不改进程 signal handler。

Windows 网络使用不可继承的 overlapped SOCKET，沿 platform 的地址与 reuseAddress 规则绑定，并关联本 loop 的 IOCP。ConnectEx 先绑定同族本地通配地址；AcceptEx 不预读业务数据，完成后分别更新 connect／accept context，再交付可查询地址的独立 stream。TCP 使用 WSARecv／WSASend，UDP 使用 WSARecvFrom／WSASendTo；同步成功仍消费实际 completion packet。包完成后用非阻塞 WSAGetOverlappedResult 取得实际 Winsock 结果和 flags，保留已知传输长度；UDP 的 WSAEMSGSIZE receive 交付已填满的前缀及 truncated，不把它当 TCP 式后缀重读。

Windows 的单次数据请求通过 CancelIoEx 撤销，真实 packet 到达后才释放 buffer 和 OVERLAPPED；取消不关闭健康 SOCKET。close 先撤销在途操作，全部退休后执行一次 closesocket；迟到 connect／accept 成功继续按第 3 节关闭未交付资源。Winsock 调用的错误保留 Winsock，IOCP 关联／取消控制的错误保留 Win32；CRT 分配错误保留 PosixErrno。未启用阻塞 linger，也不通过线程模拟整套 socket I/O。

## 7. DNS 与文件

resolve 保留 OsString 原始名称字节，port 检查 0..65535，family None 表示双族。使用共享 managed 阻塞池调用 platform 的 getaddrinfo helper，保留 AddressResolution code 和 EAI_SYSTEM 的立即 errno；不执行额外 IDNA 转换。无效内嵌 NUL 在提交 worker 前报参数错误。

Windows 的 DNS、路径与目录请求在提交时取得独立 UTF-16 副本，保留非配对 surrogate；POSIX 保持原始字节。DNS worker 复用 platform 的 GetAddrInfoW／FreeAddrInfoW，地址解码失败保留 Winsock。路径修改及目录 open／next／close 复用 platform 的 wide helper 和同次 Win32 error，目录名称按实际 UTF-16 长度复制后交付。已开始的 worker 不因取消提前释放名称或句柄，迟到结果仍按第 3 节消费／清理；排队取消不执行 OS 操作。

```scoop
public interface AsyncFile : AsyncByteSource, AsyncByteSink, AsyncCloseable {
    public suspend fun metadata(): FileMetadata
    public suspend fun size(): Long
    public suspend fun position(): Long
    public suspend fun seek(position: Long)
    public suspend fun truncate(size: Long)
    public suspend fun syncData()
    public suspend fun syncAll()
}
```

openFile 使用 Path／OpenOptions 的已有合同。Darwin 的文件操作在 managed 阻塞池调用 platform 的同步 helper；Linux 的 read／write／sync 使用 ring，其余文件操作使用同一阻塞池。错误保留 PosixErrno。顺序 read／write 共用文件位置，同一文件的读写、查询、位置／长度控制与 sync 同时最多一个。flush 不等于 sync；不增加额外的 close／sync 语义。worker 只接收 native 存储，立即记录错误。已开始的阻塞操作可能推迟取消和 run 退出，不承诺固定时延。

Windows 普通文件由 worker 以不可继承的 FILE_FLAG_OVERLAPPED HANDLE 打开并关联 IOCP；ReadFile／WriteFile 使用该 owner 的 64 位位置，真实完成时按已知传输量推进，EOF／broken pipe 读取返回零。Windows 不维护 overlapped handle 的文件位置，因此 seek／position 使用同一串行 owner 的位置，并由实际 seek 检查设备能力；truncate 不修改该位置。append 文件以同步 FILE_APPEND_DATA 句柄打开，读写进入已有 worker，保留 OS 的追加原子性和追加后的真实位置，不用另一次文件大小查询推测写入位置。metadata、truncate、sync、close 均复用 platform helper；文件错误保留 Win32。取消在途 overlapped 请求等实际 packet，运行中的同步 worker 等实际返回，close 等操作与借用退休后关闭一次句柄。

## 8. 实现边界与验收

native request 持有预留完成节点；C callback 仅存状态和进度、入 native 队列，managed 在 NativeSafe 返回握手后消费。操作编号在完成消费前不复用。managed 使用普通带类型操作与结果，不以可空字段猜测不同事件。提交同步失败走同一释放路径。后端及相关系统库按语言规范 12.2.1～12.2.3 的普通 native 输入链接。

主要验收为同 Cone scoop.test：真实 loopback 动态端口、无公网 DNS、真实文件、Context、取消／关闭竞争、部分进度、原始错误、SIGPIPE 及 moving GC。竞争用明确同步点，测试源码和工具始终存在。详细边界见 [M49 设计第 14 节](../milestone49/DESIGN.md#142-异步-io)。

## 9. M54 基础能力补齐

### 9.1 文件与目录

AsyncFile 增加 suspend 的 metadata、size、position、seek(position)、truncate(size)，语义复用 platform File；读写、位置查询／变更、truncate 与 sync 共用该句柄的串行操作边界。seek／truncate 的参数和偏移范围在提交前检查，失败不凭空改变已记录位置。不能在异步方法体内直接调用同步 File API。

具体签名为 `metadata(): FileMetadata`、`size(): Long`、`position(): Long`、`seek(position: Long): Unit` 与 `truncate(size: Long): Unit`，均为 suspend 函数；size 复用 metadata 的实际 OS 查询。seek 是绝对非负位置，truncate 不改变文件位置；seek 越过 EOF 后写入的空洞和 append 行为沿 OS 合同。负参数在原生提交前抛 IllegalArgumentException，底层错误保留 PlatformException 的 errno 和原路径。

路径查询入口为 `EventLoop.metadata(path: Path, followLinks: Boolean = true): FileMetadata`，与句柄查询复用 platform 的同一原生结构及转换。路径先取得原生副本，完成事件携带独立的结果记录，managed 复制并释放该记录后才恢复 continuation。文件查询与位置／长度操作沿 OwnedFile 的串行登记、关闭和取消路径；排队取消不执行 syscall，已开始的操作等实际完成后结束。若取消或关闭获胜，已经发生的 seek／truncate 不回滚，之后观察到的文件位置与长度以 OS 为准。

文件系统入口沿 EventLoop.openFile 的方式增加 `metadata(path, followLinks)`、`moveFile(source, destination, replaceExisting)`、`deleteFile(path)`、`createDirectories(path)`、`openDirectory(path)` 与 `createTempFile(parent, prefix)`，返回类型复用 FileMetadata／Path 等既有值。move 的排他／替换和跨文件系统错误沿 platform 合同，不暗中 copy+delete。

路径修改的具体入口为 suspend 的 `EventLoop.moveFile(source: Path, destination: Path, replaceExisting: Boolean = false): Unit`、`deleteFile(path: Path): Unit` 与 `createDirectories(path: Path, mode: Int = 0x1ff): Unit`。deleteFile 删除文件、链接本身或空目录，不递归删除目录内容；moveFile 保留源内容，默认排他，替换时沿同一文件系统 rename 合同。move 失败的 context 为 destination，其他修改为对应 path；保留本次 syscall 的 errno。

createDirectories 逐层调用共享 mkdir helper，缺少父目录时继续创建父目录；已经存在的目录成功，普通文件占用目录位置或其他错误照常报告。mode 复用 platform 的 0..0xfff 校验并受 OS umask 影响，不修改已存在目录的权限。每次 mkdir 和确认已有目录的 metadata 都通过 worker 等待，层间取消不再提交下一项；已创建目录与已生效的移动／删除不回滚。所有路径在工作排队前取得 native 副本，排队取消不执行 syscall，已经开始则等待实际完成。

createTempFile 以一次 native 排他创建返回实际 Path 与拥有句柄的 AsyncFile，避免先创建路径再重新打开；结果为有名的普通 TemporaryFile 值。取消先获胜而 native 创建成功时，关闭未交付句柄并清理本次未交付的临时文件；已交付后是否删除由调用者负责。失败或清理错误保留真实原因，不执行目录来源审计。

具体入口为 suspend 的 `EventLoop.createTempFile(parent: Path? = None, prefix: String = "scoop-"): TemporaryFile`。TemporaryFile 是带只读 path／file 字段的普通 struct，实现 AsyncCloseable，其 close 只关闭同一个 AsyncFile，不隐式删除已交付路径。父目录默认沿 platform 的 TMPDIR 选择，prefix 拒绝 NUL 与路径分隔符；文件以 0600（受 umask 影响）、读写和 close-on-exec 创建，初始位置与长度均为零。

路径模板与排他 native 创建由 platform 共享 helper 提供；创建失败保留父目录 context 和立即 errno。迟到的成功结果先纳入正常 OwnedFile 生命周期，再执行不可由本次取消中断的 close 和 delete；两项清理都尝试完成，失败以 ResourceUseException 与原取消／关闭原因组合，不能遗失清理错误或向调用方交付部分结果。成功交付后，路径和文件句柄拥有普通文件的独立生命周期。

Windows 临时文件在 worker 内规范化父目录，保留完整前缀并以随机后缀、CREATE_NEW 和 overlapped 句柄排他创建；重名重新选择，返回同一个打开的句柄及实际 UTF-16 路径，不先创建后重开。若创建后 IOCP 关联失败，内部结果保留已取得的 owner、路径和错误，由同一 managed 清理路径关闭并删除后再报告；不向应用交付失败的临时文件。随机源失败保留 NtStatus，路径／文件失败保留 Win32，mode 沿 Windows 的权限投影。

`AsyncDirectoryEntries` 为 AsyncCloseable 的单遍枚举器，`next(): DirectoryEntry?` 可挂起，正常 EOF 关闭句柄，提前退出须 close。名称在 native 返回前复制，跳过点目录，不一次性把任意大目录收集到数组。缓存淘汰或启动扫描使用此公共能力。

具体入口为 suspend 的 `EventLoop.openDirectory(path: Path): AsyncDirectoryEntries`；目录项复用 platform 的 DirectoryEntry，保持原始名称字节和 OS 枚举顺序，不额外查询每个条目的 metadata。同一目录同时最多一个 next，冲突报 IllegalStateException；正常 EOF 被记住，后续 next 等待同一次关闭后继续返回 None，显式提前关闭后的 next 报 IoClosedException。next 遵守所属 loop／Job 约束，close 沿已有资源清理合同允许取消中的 finally 和幂等关闭。

open／next／close 均由 worker 调用 platform 的目录 helper。next 在本次工作内复制一个名称，managed 取得独立快照并释放原生副本；关闭等待正在进行的 readdir 及结果消费结束。排队取消不推进目录，已经执行的 next 不回退 OS 位置，迟到名称仍被释放。open 取消后取得的句柄必须在恢复调用方前关闭；循环退出也收尾未关闭的目录，不把所有名称预先收集进数组。

上述工作使用共享 managed 阻塞池和 platform 的同步文件 helper；必要 helper 先在 platform 补齐，再由 asyncio 调度。路径、输入数据和输出存储在提交前取得稳定 native 副本，errno 在 worker 内立即记录。已开始的 syscall 取消仍等真实返回；迟到结果和 close 按第 3 节收尾。

### 9.2 地址族竞速与 SOCKS5

增加 `EventLoop.connectTcp(host: OsString, port, options)` 的域名重载，提供 RFC 8305 所需的双族解析、候选交错和延迟启动。默认 resolution delay 50 ms、connection attempt delay 250 ms，可配置；默认同时在途的 TCP 候选不超过 2 个。先获得的可用地址不必等待另一族全部解析完成，全部失败保存实际尝试错误。

选择一个成功连接后关闭其他未交付连接，等待竞速子任务和 native 结果收尾。候选普通失败在该任务内转成结果，不能传播并取消本来可能成功的候选。参数仍为 OsString，不隐式 IDNA；传入 SocketAddress 的既有接口保持不变。配置变化只影响本次连接，不建立全局 DNS 缓存、TTL 推测或 HTTPS records 实现。

具体入口为 `EventLoop.connectTcp(host: OsString, port: Int, options: TcpConnectOptions = tcpConnectOptions()): AsyncTcpStream`。TcpConnectOptions 为不可变配置，包含 `resolutionDelay: Duration = Duration.milliseconds(50)`、`attemptDelay: Duration = Duration.milliseconds(250)`、`maxConcurrentAttempts: Long = 2L`，以及默认均为 None 的 `resolveTimeout: Duration?`／`connectTimeout: Duration?`。所有配置时长必须非负，并行数必须为正。零间隔允许立即推进，零超时表示进入该阶段即到期；实际在途数仍受并行数限制，单地址入口不应用这些竞速策略。

两族分别调用既有异步 resolve，族内保留系统结果顺序并去除重复地址。IPv6 已有结果时立即开始；IPv4 先到且 IPv6 未结束时，最多等待一次 resolutionDelay，IPv6 失败或空结果提前解除等待。开始后优先选上次尝试的另一族，不因尚未返回的族阻塞已有候选。相邻尝试默认间隔 attemptDelay；任何候选失败可以提前开始下一项，达到并行上限时等待空位，迟到的旧定时通知不能越过新间隔。

全部候选失败抛 TcpConnectException，保留 host、port、`attempts: Array<TcpConnectionFailure>` 与 `resolutionFailures: Array<TcpResolutionFailure>`。两个记录分别含 address／family 及原始 cause，按各自完成顺序保存；解析成功但没有地址不伪造 OS 错误。此异常在调用方可正常捕获，不预先取消其 Job 或兄弟任务；参数及错误 loop 仍报原状态／参数异常。成功交付前发生总取消时关闭已选中的连接；清理失败以 ResourceUseException 保留，不遗失迟到连接的关闭错误。

resolveTimeout 从开始双族解析前取同一个单调时钟起点，直到两族各自交付结果。截止到期只取消尚未完成的族，并将其记为 TcpResolutionFailure，cause 为 TcpResolutionTimeoutException，保存 host／port／family、limit 与 elapsed；已经获得的候选仍可连接。迟到解析结果正常释放，不替换已结束族的候选，也不使成功的另一族失败。零解析超时不提交 DNS 工作，最终没有候选时仍通过 TcpConnectException 交付两族的明确超时原因。

connectTimeout 从第一个候选即将启动时开始，对整个候选序列共享，失败和交错启动不重置它。到期抛 TcpConnectTimeoutException，保存 host／port、limit／elapsed 和当时已记录的 attempts／resolutionFailures；零连接超时不提交 TCP 尝试。阶段完成与新尝试前也检查单调截止，已经选择的外部取消保持原异常。两类超时都取消有关任务并等待实际 native 工作与未交付 socket 清理完成，再返回调用方；计时器随竞速作用域结束，不留下长时间等待的子任务。

TcpConnectOptions.onEvent 是可选的普通 `(TcpConnectEvent) -> Unit` 回调，默认 None。事件保存 phase（TcpConnectPhase.Resolve／Connect）、kind（TcpConnectEventKind.Started／Finished）和单调 TimeMark。Resolve 覆盖双族解析，Connect 从首个候选开始；两个阶段可以重叠，各自至多通知一次开始与结束，不为每个候选保留事件历史。零截止仍表示进入过对应阶段；没有可用地址时没有 Connect 事件。成功连接导致未完成的解析／竞速任务取消时，未结束阶段的 Finished 在这些任务实际收尾后交付。

通知在 Scoop 连接流程、内部锁外执行，保留原 Context；未配置回调不增加阶段采样时钟。回调应快速返回，上层挂起 observer 可以只在这里记录状态，再于自己的调用步骤交付。回调抛出后停止后续通知，取消并等待连接任务，关闭未交付的 socket，再向调用者报告原异常；已有主失败保持在 ResourceUseException.primaryFailure，通知或实际关闭的新失败按发生顺序附加。这里不包装普通 TCP／解析错误，也不增加客户端私有 DNS 或连接竞速路径。

提供 SOCKS5 的普通异步握手适配，支持无认证和 RFC 1929 用户名／密码、CONNECT、IPv4／IPv6／ASCII 域名目的地、本地解析或代理解析。用户凭据和长度在写出前检查；reply 错误保留协议码。使用已有 AsyncByteStream，按 takeOwnership 规则接管，成功交付隧道与完整前缀，失败清理；不让 HTTP client 手写 SOCKS 报文或同步 DNS。

具体入口为 suspend 的 `AsyncSocks5Tunnel.takeOwnership(transport, destination: Socks5Address, credentials: Socks5Credentials? = None, resolution: Socks5Resolution = Proxy)`，transport 同时实现 AsyncByteStream 与 AsyncCloseable。Socks5Address 为 `Ip(SocketAddress)` 或 `Domain(String, Int)`，Socks5Resolution 为 Local／Proxy；返回的 AsyncSocks5Tunnel 实现同一异步流及关闭接口，并公开 boundAddress 和 isClosed。目标端口在 0..65535，域名为 1..255 字节的可打印 ASCII 且不含空格，不做 IDNA 或 DNS 标签规范化；IP 不能携带 SOCKS5 无法表示的非零 IPv6 scopeId。

Socks5Credentials 以 List<Byte> 接收用户名与密码，分别保存独立 Array<Byte> 快照，长度均为 1..255；`fromUtf8(username, password)` 是明确的文本便利入口，长度按 UTF-8 字节计算。无凭据时只提议无认证方法，有凭据时只提议用户名／密码方法，不自动降为无认证。协议不规定这些凭据字节的字符集，不输出凭据内容。

Proxy 模式把域名原样放入 CONNECT，Local 模式在写出握手前以当前 EventLoop.resolve 解析，并使用系统返回的第一个地址；空结果报明确失败，不伪造原生错误。调用者可显式传 Ip 选择其他候选；一次隧道握手只发送一次 CONNECT，失败后不能在同一流上改目标重试。IP 目的地不执行 DNS，Local／Proxy 均不更改已有地址。

配置、目标与活动 Job／loop 检查通过后接管传输；此前失败仍由调用者负责关闭。接管后所有解析、写入、flush、读取及取消失败都等待真实操作结束并关闭未交付传输，清理失败使用 ResourceUseException。握手使用有界缓冲及 readFully／writeFully，每一阶段排出输出；读取只消费当前回复的确切长度，因此和回复同次到达的隧道数据完整保留在底层流中，返回后直接从隧道读取，不丢弃或重复前缀。

Socks5Exception 保存 phase（Method／Authentication／Connect）及可选 replyCode；拒绝的方法、认证状态和 CONNECT REP 保留实际协议字节。非法版本、保留字段、地址类型或绑定域名报对应阶段的格式错误；底层 I/O、解析和取消保持原异常。交付后的流沿普通 AsyncByteStream 的双向、取消与背压合同；close 固定 loop 归属，可在取消 finally 中完成，并发关闭等待同一次底层收尾，之后幂等。

### 9.3 通用阻塞池与原生操作适配

领域库使用 core 的 `blocking { ... }` 或显式 `executor.blocking { ... }`，执行普通 Scoop 闭包、同步库和 FFI，返回类型化值或异常；不再提供 `scoop.asyncio.native.NativeWork`、`EventLoop.runNativeWork` 或 execute/release 指针工厂。TLS 系统信任装载和验证使用有明确释放责任的工作对象，正常结果、取消和关闭仍遵守 TLS 资源合同。

文件、目录、路径、DNS 及 Windows TTY 的阻塞部分由 EventLoop 的 managed BlockingTaskPool 执行，与通用 blocking 共享配置和线程。三个 backend 不再创建专用阻塞 worker；io_uring、IOCP、dispatch source/timer 和只发布完成记录的 OS 通知机制保持原职责。

原生 shim 保留稳定参数、同步 execute、owner 上的 finish、错误提取与释放。owner 提取待投递工作并包装为 BlockingWork；原生登记不负责启动线程。每份投递使用独立小型 native task，原生锁仲裁 Queued→Running 与取消：取消先胜时撤销 payload 关联并发布取消完成，即使 managed 池所有线程都被占用也能解除借用；已投递 task 作为空工作在实际出队时退休。Running 后等待真实 syscall 返回，owner 才处理完成、实际进度及迟到资源。取消结果选择沿第 3 节，不套用通用 blocking 的运行后保留 T 规则。

原生 idle 同时等待 operation 和所有已投递 task 退休，保证取消后尚在 managed 队列中的 task 不引用已销毁 loop。owner 在每次 operation 登记后投递其原生工作，并在完成与清理之后投递本轮新增工作；池拒绝投递时以普通 I/O 失败完成该原生工作，并沿既有收尾路径释放它，不遗留登记。finish 与 managed continuation 交付均在锁外，worker 不修改 owner-only 容器。

### 9.4 验证

文件能力独立验证真实 metadata／seek／truncate、排他临时创建、同文件系统替换、目录分批遍历、worker 取消与迟到结果；组合磁盘缓存和下载续传。连接竞速验证单族迟到／失败、不同候选交错、成功竞争、总取消及失败不污染父 scope。SOCKS5 使用独立 loopback 对端覆盖分段、认证和两种 DNS 模式。

测试覆盖真实文件／DNS／TTY、共享池排队与取消、Running 后等待真实返回、buffer 进度、迟到资源释放和 loop 关闭。通用 managed 闭包／结果及线程池生命周期由 core 和 concurrent 测试覆盖；TLS 使用真实系统装载与隔离信任验证。普通行为使用 Cone tests，平台 shim 的独立测试只验证原生操作与退休协议，不另建测试专用线程池。

## 10. 自有 TTY

```scoop
public struct TtySize(val columns: Long, val rows: Long,
    val widthPixels: Long, val heightPixels: Long)
public enum TtyRead {
    Bytes(val count: Long), Resize(val columns: Long, val rows: Long), End, TimedOut
}
public interface AsyncTty : AsyncByteSink, AsyncCloseable {
    public val isClosed: Boolean
    public suspend fun receive(buffer: NativeByteBuffer, count: Long,
        timeout: Duration? = None): TtyRead
    public suspend fun size(): TtySize
    public suspend fun isRawMode(): Boolean
    public suspend fun enableRawMode()
    public suspend fun disableRawMode()
}
```

openTty 通过 platform 的共同 owner 取得实际终端与 resize 通知，每进程最多
一个活动 owner，包括同步 terminal 会话。失败清理所有已取得的资源；取消
选定后才完成的 open 先关闭未交付 owner。库不接管调用者 fd，不改变 stdin
的 O_NONBLOCK。POSIX 不增加后台读取线程；Windows 使用共享 managed 阻塞池。

receive 同时等待字节与 resize；POSIX 使用平台后端观察自有 fd，实际 read 为
非阻塞 syscall。Windows worker 等待自有 console input HANDLE 与本次请求的取消 event，
复用 platform 的 UTF-16 输入记录到 UTF-8 字节转换；输出在 worker 内执行实际 WriteFile。
正 count 返回正 Bytes、End、Resize 或 TimedOut；count 为 0
在状态、参数和取消检查后返回 Bytes(0)，不表示 EOF。None 无限等待，非负
timeout 使用单调总时间；零不等待。resize 可合并为最后实际尺寸，通知本身
不占用字节 buffer。收到 End 后后续正 receive 继续返回 End。

同一资源允许各一个 receive 和 writeFrom 同时在途；模式和尺寸控制要求两个
方向都空闲。flush 沿无缓冲输出合同。原生状态由后端串行管理，managed 完成由所属 loop owner 消费；
不在 loop owner 上执行阻塞 poll／write。SIGWINCH 沿终端规范的
Linux self-pipe／Darwin kqueue 实现，由后端观察通知 fd，避免另装
signal handler 改变应用原 disposition。

取消停止 poll／timer 后记录一次终止，借用按实际字节进度提交后才恢复。
取消已经选定时到达的 resize 保留给下一次 receive；End 保留为输入终态。
单次取消不关闭健康 TTY。关闭拒绝新操作，结束在途等待，等待借用释放和
全部 source cancel handler／CQE／worker 实际完成，再恢复 raw 并释放共同 owner；run 收尾同样执行。
Windows 输入取消唤醒当前等待；已运行的同步输出等实际返回，保留其已知进度。
输入截止时间在提交时确定，使用与 timer 相同的 QPC 单调时钟，排队时间计入总时间。
原生错误保留 PosixErrno／Win32 来源，控制操作同次返回类型化错误；恢复失败也继续消费所有资源。

disableRawMode 和 isRawMode 允许已取消 Job 在所属活动 loop 中进行清理，
不因此接受其他被取消的 I/O。模式的幂等、快照与恢复沿终端规范第 7 节。
async raw 作用域使用这两个入口和 useAsync，GC 不承担恢复。

## 11. 平台后端与宿主主循环（迁移合同）

本节规定平台驱动与宿主接入。第 1～10 节的任务归属、资源方向独占、实际进度、路径／地址、TTY、显式关闭与 buffer 借用合同同时适用。平台后端只负责原生操作及真实完成，不另建任务树或资源语义。

### 11.1 共享层与平台后端

`EventLoop` 提供 I/O 服务与 executor 属性，同步泛型根入口归 `PlatformEventLoop`。Darwin 绑定系统 libdispatch，Linux 绑定 io_uring／固定 liburing，Windows 绑定 IOCP。所有端点、文件、目录和操作只依赖同一共享 owner；Job／CoroutineScope／execution 来自 core。native 后端管理原生资源、请求、完成记录，不拥有另一套任务树，也不直接执行 managed I/O continuation。

后端按目标构建选择，不公开后端注册表。`PlatformEventLoop` 取代 `LibuvEventLoop`，不保留 libuv 依赖或兼容别名；不修改语言、runtime ABI、IR 或产物格式。平台能力与依赖版本见[设计第 7～10 节](../asyncio-backends/DESIGN.md#7-macoslibdispatch-后端)。

### 11.2 运行与附着

`PlatformEventLoop.run` 保留同步单次根入口合同。新增 `attach(host, onClosed): EventLoopSession`，接入调用线程所属的宿主主循环；两种入口共用一次启动机会，均拒绝当前 Job／活动 EventLoop binding 中的嵌套启动。接受入口后才取得 native 资源，初始化失败回滚并关闭实例。

session 暴露 `loop: EventLoop`、`isClosed: Boolean`、`post(block: CoroutineScope.() -> Unit): Boolean` 与 `requestClose()`。后三项可跨线程；post 异步排队，关闭后拒绝且不保留闭包。接受的投递在关闭／失败时可以丢弃，内部 work、取消、完成和清理命令不能丢弃。post 的 receiver 是仍活动的 session 根作用域，子任务沿 launch／async 合同；根在没有消息时挂起，持续到请求关闭或失败。

内部根在 attach 栈内捕获 binding 快照并排队，根正文及用户投递从后续宿主通知开始；mailbox 通过根 continuation 恢复，不继承外线程 post 时的动态 Context。普通投递抛错作为根失败处理。requestClose 幂等、不阻塞，停止应用投递并把根取消交给 owner；mailbox 的取消恢复也通过 LoopExecutor 排队。主动关闭的取消不作为最终失败，其他失败沿现有组合规则保留。

两种模式共用非阻塞处理流程；run 在空闲时 NativeSafe 等待，attach 只在宿主通知时处理一轮。通知可合并，真实队列承担完整性。嵌套宿主循环不能递归进入处理流程；处理期间产生的新工作留待后续通知。库不抢占用户回调。

### 11.3 宿主接入合同

`scoop.asyncio.host.EventLoopHost` 提供 `isCurrentThread()` 与 `@Unsafe watch(notification, ready): EventLoopWatch`。`EventLoopNotification` 是带值枚举：`ReadableDescriptor(Int)` 用于 POSIX，`WindowsEvent(Ptr<Unit>)` 用于 Windows 的 manual-reset event。host 只借用并监听，不读写、重置或关闭通知资源；Windows HANDLE 不缩窄成 Int，也不是 CRT fd。ready 在 owner 线程串行执行，注册时不得内联执行。安装失败不得留下活动监听或回调。 不支持的通知种类在安装前明确拒绝；旧 watchReadable 接口不保留转发方法。

`EventLoopWatch.remove(onRemoved)` 在 owner 上异步注销，按宿主协议不得抛错；必须等待当前 ready 退出并退休排队通知，再通知移除完成，此后不得再进入 ready。loop 在移除完成前保留通知资源和 callback 上下文。

Darwin 提供主 dispatch queue 的 `MainDispatchHost`，Linux GUI 在自己的 fd watch 中监听 eventfd，Windows GUI 将借出的 event 接入自己的等待／消息循环。宿主仍负责窗口、输入、显示刷新和其连接协议。使用主队列的适配器只在宿主服务该队列时取得执行机会，不保证任意自定义 run-loop mode 都运行。具体声明与异常出口见[设计第 3 节](../asyncio-backends/DESIGN.md#3-公共入口与宿主协议)。

LoopExecutor 的全部真实挂起恢复都在 owner 线程进入 managed，包括第三方回调和跨 executor 同步。显式切换到其他 executor 后，该任务在目标 executor 上执行，返回时回原 execution；UI 对象与 host API 的线程要求仍由调用者遵守。外部应用消息可通过 session.post 进入根作用域，内部 continuation 不经该可丢弃消息通道。

### 11.4 取消、借用与最终关闭

结果选择与 native 结束分别记录。取消／close 先选择结果后请求后端撤销；必须等数据操作、相关控制请求及未交付资源清理真实结束，提交已知实际进度并解除借用，再恢复一次 continuation。dispatch cancel 请求或 io_uring 取消 CQE 不能单独证明目标操作结束；成功先选定不被后续取消覆盖。

根及其全部后代完成后，先排空 LoopExecutor 已接纳任务，再以 owner 的内部命令／ready 关闭遗留资源，排空实际 I/O 及清理。此阶段保持 blocking pool 接纳；只有无任务、无资源、无操作、无待投递／退休工作后才停止自有池。全部原生请求、worker 执行循环与 I/O callback 退出后，附着模式再注销宿主监听，最后销毁后端、join 自有 worker、关闭通知资源并在 owner 上执行一次 onClosed。requestClose 和非阻塞 pump 不等待活动阻塞函数；宿主必须继续运行到 onClosed。当前宿主移除通知的 callback lease 在其返回时释放，不等待自身退出。

onClosed 执行时 isClosed 为真，参数为运行／清理失败或 None。回调自身异常由宿主桥捕获并报告，不重复关闭、不跨 C 栈传播。GC release 不驱动循环、恢复任务或等待原生退出。阻塞 syscall 不能强制撤销时，显式关闭须等其真实返回。

### 11.5 平台能力与错误

Darwin 的 libdispatch 通过系统 libSystem 的标准 re-export 链接，不另要求 SDK 默认库目录中存在名为 dispatch 的独立库。Darwin 用 dispatch source 驱动 nonblocking 网络／TTY 与单调 timer，共享 managed 阻塞池处理普通文件、DNS、目录与路径；source 全部退休后才消费其 fd。Linux 用 ring 完成网络、文件数据 I/O、fsync、timer 与 TTY readiness，其余阻塞工作使用共享 managed 阻塞池。两者复用 GC-free 平台 helper 和同一资源语义。

Linux 新后端支持基线为内核 6.1，初始化实际探测所需操作与 feature；ring 不可用或能力不足明确失败，不静默回退其他后端。此要求仅针对 asyncio 新后端。worker 启动中途失败时，已启动的空闲线程异步退出，owner 不 join 等待；完全退休后允许后续操作重试启动。完整操作集、队列满时的处理、取消退休、提交失败与 ring 销毁见[设计第 8 节](../asyncio-backends/DESIGN.md#8-linuxio_uring-后端)。

新后端的 syscall／负 CQE 错误保留 PosixErrno，DNS 保留 AddressResolution；正常 timer 到期不是错误，取消和 IoClosedException 仍由共享层选择。平台库不再提供 Libuv 错误域。迁移验收覆盖 Darwin、GNU、musl 的同一公共行为，以及宿主监听注销、Context、取消竞争和 native 内存寿命；实际结果见实施记录。

### 11.6 Windows 完成端口

Windows 的异步数据操作使用独立、稳定的 OVERLAPPED 请求与 IOCP。每个 loop 的原生收集线程阻塞等待完成，只发布原生记录并唤醒 owner，不执行 managed 代码；owner 消费有限完成快照并复用本规范的结果选择、借用和关闭规则。同步成功但仍会产生 completion packet 的操作也等该 packet；只有明确未提交的失败可以本地结束。CancelIoEx 的返回和 ERROR_NOT_FOUND 均不能代替目标完成。

同一 manual-reset event 汇合命令、IOCP、timer 和 worker 通知。队列发布、SetEvent、owner 的 ResetEvent 与取队列在同一锁边界协调；通知只作提示，不能据其次数判断操作是否完成。宿主监听注销完成前不关闭 event。收集线程和 worker 不再访问 loop 后才销毁 native 状态；owner 的非阻塞 pump 和 close 不 join 活动线程。

timer 按 QPC 单调截止时间等待并核对实际到期，长时长分段等待，不截断为无限等待或提前完成。取消在同一原生锁下移除尚未到期的 timer，已经发布的完成仍保留且只消费一次。不可 overlapped 的操作沿共享 managed 阻塞池执行，已开始的工作等真实返回，不因取消提前释放输入。

Win32／IOCP 与 Winsock 错误分别保留真实 domain 和完整 32 位 code；CRT 的 errno 保留 PosixErrno。创建、等待和销毁的直接结果使用同次返回的错误记录，不以负 Int 编码 DWORD。详细执行与测试范围见[后端设计第 13 节](../asyncio-backends/DESIGN.md#13-windowsiocp-后端)。


### TCP 连接配置

构造配置遵守 [库 API 风格](SCOOP-API-STYLE.md)：接收者回调同步执行一次，标量赋值替换、嵌套块基于当前值修改，异常不发布部分结果。以下结果的 copy(configure) 返回独立描述，原值不变；资源、模型与普通／suspend 函数按原有引用合同保存，构造不执行这些动作。

| 结果 | 构造入口与全部配置字段 |
| --- | --- |
| TcpConnectOptions | `tcpConnectOptions(configure: TcpConnectOptionsBuilder.() -> Unit = {})`；resolutionDelay: Duration = Duration.milliseconds(50L)；attemptDelay: Duration = Duration.milliseconds(250L)；maxConcurrentAttempts: Long = 2L；resolveTimeout: Duration? = None；connectTimeout: Duration? = None；onEvent: ((TcpConnectEvent) -> Unit)? = None |

表中有新构造入口的 class 构造器内部化，不保留长参数重载、旧 companion 方法工厂或转发别名。Builder 构造器内部化。协议 struct 与少量小型完整值保留普通构造，copy 和父级配置成员直接复用同一值模型，不为它们另设顶层工厂。所有字段类型、默认策略和非法组合的领域检查仍以上文合同为准。
