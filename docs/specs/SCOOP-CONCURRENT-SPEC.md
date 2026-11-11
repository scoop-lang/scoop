# Scoop 并发库规范

本文规定普通 Cone `scoop:scoop.concurrent:0.1.0` 的合同。源码位于 `sysroot/src/scoop.concurrent/`，显式依赖 core、std、platform，测试依赖 scoop.test；公共声明属于 `scoop.concurrent`，不默认导入。实施与验证范围见[实施记录](../coroutine-executors/PROGRESS.md)，整体设计见[设计文档](../coroutine-executors/DESIGN.md)。

## 1. 分层与等待协议

语言规范 11.9 的 continuation、Job、execution 与 executor 负责执行归属。Channel、AsyncMutex 和 AsyncConditionVariable 不保存一个固定 executor；每个等待者保存自己的 typed continuation，因此同一对象可以由单个或多个 executor、普通 managed thread 共同使用。同步 API 不要求调用者具有 Job；挂起 API 在有当前 Job 时参与其取消，无 Job 时仍可等待。

内部平台 Mutex 只保护状态、等待队列和待交付结果。锁内不执行用户回调、不恢复 continuation、不 dispose Job 订阅、不跨挂起点。等待队列支持取消时摘除任意节点；摘除后清空节点链接，终态后解除 continuation、取消订阅和未交付参数引用。消息缓冲复用 std 的 ArrayDeque。

挂起操作先检查当前 Job，再使用原始 suspendCoroutine 与 invokeOnCancellation。消息、锁持有权、取消和关闭竞争同一对象锁下的状态转换；不得在提交所有权之后再用独立仲裁的 CancellableContinuation 决定是否接收结果。

登记顺序为：建立未入队节点；在锁外订阅取消；取得订阅后在对象锁内检查节点终态、保存订阅并尝试完成或入队。已取消 Job 可同步执行 handler；此时迟到的订阅在锁外 dispose。已开始的 handler 可以与 dispose 竞争，handler 必须在对象锁内检查节点状态。快速路径可在入口取消检查后直接提交成功。

结果在锁内提交一次，在锁外通知。一个操作选中多个等待者时，某个低层无 executor 的 completion 即使抛出异常，其余已选中通知也必须执行，之后才传播首先捕获的异常。通知顺序不承诺不同 executor 的实际执行顺序。GC 不自动 close Channel 或释放 async guard。

## 2. Channel

```scoop
public enum TrySendResult { Sent, Full, Closed }
public enum TryReceiveResult<T> { Value(T), Empty, Closed }
public class ChannelClosedException public constructor() : Exception
public class Channel<T> public constructor(capacity: Long) {
    public suspend fun send(value: T)
    public suspend fun receive(): T
    public suspend fun receiveOrNone(): T?
    public fun trySend(value: T): TrySendResult
    public fun tryReceive(): TryReceiveResult<T>
    public fun close(): Boolean
}
```

Channel 是 MPMC。capacity 为 0 时双方会合；正数表示有限 FIFO 缓冲；负数抛 IllegalArgumentException。没有负数魔法值、无限缓冲或自动丢弃模式。

send 优先匹配最早等待接收者，否则在缓冲有空位时提交消息，否则挂起。receive 先取最早缓冲消息，并接纳最早等待发送者补入缓冲；没有缓冲消息时可直接匹配等待发送者，否则挂起。FIFO 按锁内的匹配、入缓冲和等待登记顺序确定，新调用不能越过同侧有效等待者。

trySend 与 tryReceive 不挂起、不订阅取消，可以完成对方已等待的操作。Full／Empty 表示本次未发生交付；Closed 表示发送已关闭或接收已关闭且缓冲排空。

发送或接收节点取消先胜时将其移除，既不交付也不消费消息；双方匹配在一个临界区内提交两方成功，后续取消不能撤回。缓冲入队即为发送成功。结果已确定、等待者尚未运行期间，消息由 typed 结果持有，不重新放回 Channel。

close 首次把开放状态改为关闭返回 true，之后返回 false。它拒绝新发送，并让所有尚未提交的发送者收到 ChannelClosedException；不会将待发送节点补入已关闭的缓冲。已提交消息仍可按序排空。关闭且缓冲为空时，receive 抛 ChannelClosedException，receiveOrNone 返回 None，所有等待接收者结束。T 本身是 Option 时，Some(None) 仍是有效消息。

close 不执行消息元素的清理代码。携带需显式关闭的资源时，由应用安排消费或清理；没有 abort、drop-buffer 或 undelivered 回调。

## 3. AsyncMutex

```scoop
public class AsyncMutex public constructor() {
    public suspend fun lock(): AsyncMutexGuard
    public fun tryLock(): AsyncMutexGuard?
    public suspend fun <T> withLock(block: AsyncMutexGuard.() -> T): T
    public suspend fun <T> withLock(block: suspend AsyncMutexGuard.() -> T): T
}
public class AsyncMutexGuard : Disposable {
    public override fun dispose()
}
```

锁非重入；guard 仅由库构造，表示一次持有权，不绑定 OS thread。guard 可随 coroutine 迁移或由应用明确交给另一任务释放，但不能由多个任务同时作为持有权使用。重复获取已持有锁可以等待自身，不自动检测死锁。

等待者 FIFO，tryLock 不能插队。释放在内部锁下直接把持有权交给最早等待者，随后恢复；不会先变为空闲再竞争获取。取消先胜则不取得锁，交付先胜则返回有效 guard，即使调度前已取消。

withLock 取得 guard 后用 try/finally 覆盖完整 block，普通或 suspend block 返回、抛出或取消均释放。两个 overload 的字面量挂起性遵循语言规范的期望类型与对称重载规则；已有函数值不跨效果转换。

guard 的所有别名共享 Held、ConditionWaiting、Released 状态。对 Released 的 dispose 幂等；ConditionWaiting 期间 dispose 抛 IllegalStateException。没有 GC 自动解锁或 poison。

## 4. AsyncConditionVariable

```scoop
public class AsyncConditionVariable public constructor(val mutex: AsyncMutex) {
    public suspend fun await(guard: AsyncMutexGuard)
    public fun signal(guard: AsyncMutexGuard)
    public fun broadcast(guard: AsyncMutexGuard)
}
```

Condition 固定关联一个 AsyncMutex；全部操作要求该 mutex 的当前有效 guard，错误锁、已释放或正在等待的 guard 抛 IllegalStateException。signal／broadcast 也必须持锁。无等待者时不保存通知；应用在 while 中检查由该 mutex 保护的谓词。

Condition 队列和 mutex 获取队列共用一把内部状态锁。await 在一个临界区登记节点、将 guard 标为 ConditionWaiting、释放持有权并交给下一获取者，然后才在锁外通知，不存在释放后尚未登记的窗口。

signal 将最早 condition 节点选为成功并移到 mutex 重新获取队列；broadcast 对当前全部等待者按顺序执行。通知不等于已取锁，取得锁之前不会恢复 await。

入口已取消时保留原 guard 并直接抛取消。登记期间取消先胜、尚未释放锁时同样保留 guard；已在 condition 队列的取消则摘除节点，携带取消结果进入 mutex 重新获取队列。通知先胜时，后续取消不能覆盖成功。重新获取不再接受取消，不调用公开 lock，也不创建额外 Job。仅在原 guard 恢复 Held 后交付成功或取消，所以 await 返回或抛出时仍持有同一个 guard，外层 finally 可以正常释放。

## 5. 多线程 executor 与通用阻塞池

```scoop
public class MultiThreadExecutor : CoroutineExecutor, Closeable {
    public val isShutdown: Boolean
    public val isTerminated: Boolean
    public fun invokeOnTermination(handler: () -> Unit): Disposable
    public override fun registerTask(parent: ExecutorTask?): ExecutorTask
    public fun requestShutdown()
    public suspend fun awaitTermination()
    public fun join()
    public override fun close()
}
public class BlockingTaskPool public constructor(threads: Int) : Closeable {
    public val isShutdown: Boolean
    public val isTerminated: Boolean
    public fun invokeOnTermination(handler: () -> Unit): Disposable
    public fun submit(work: BlockingWork)
    public fun requestShutdown()
    public suspend fun awaitTermination()
    public fun join()
    public override fun close()
}
```

MultiThreadExecutor 由第 6 节工厂建立固定数量的 managed coroutine workers；BlockingTaskPool 建立独立的固定 managed workers，执行任意普通 Scoop 闭包所产生的 BlockingWork，不限制为 GC-free。协程 worker 各有一个单生产者本地环形 FIFO 和一个接收外部投递的 CAS 无锁 inbox。物理 worker 内的同池投递进入自己的本地队列，外部恢复进入目标 inbox；本地空间不足时溢出到 inbox，不限制实际可接纳工作总量。worker 定期检查外部来源，空闲时批量窃取其他 worker 的本地工作；阻塞池保持独立的 Mutex／ConditionVariable FIFO，不参与窃取。没有隐式全局池、无限线程或调用线程 fallback。

每个 worker 使用平台 Thread.start 的长期 managed 入口，等待路径为 NativeSafe，执行路径保留普通 safepoint。每个 CoroutineWork 负责进入及恢复自己的 TaskContext；BlockingWork 遵守语言规范 11.9.3 的 binding 快照、独立 invocation 与结果交付合同。work 抛出的未捕获 completion／协议异常在 worker 边界打印异常栈并以退出码 1 终止进程，不能只结束一个 worker；普通任务正文／blocking 闭包的异常仍由 core 交给 completion／Job。

executor 的 Open 接纳根和子任务，Draining 只接纳自身仍活动登记的 child，Terminated 全拒绝。登记、complete 与总计数使用生命周期锁；ticket 的原子活动标记供普通 dispatch 读取，投递不更新 ticket 计数、不取得 executor 全局锁。活动登记归零后，关闭在每条外部 inbox 末端以 CAS 发布终止标记，与正常入队争用同一链接；所有 inbox 封闭后才通知 worker 排空退出。本地队列只由其物理 owner 发布，owner 返回调度循环后须先排空已有工作，不能因停止标记跳过仍有工作的本地队列。与 complete 重叠的投递按语言规范 11.9.3 接纳或拒绝，成功投递的工作不能遗失。有效登记的 coroutine／blocking work 必须排队，Draining 也不得拒绝；已完成登记上的投递和重复 complete 报协议状态错误。blocking 工作单独跟踪从投递至实际 runner 退出的义务，不能仅因 ticket 已 complete 就忽略仍在执行的工作。

新根分散到 worker 队列，同 executor 的子登记继承父登记最近执行的 worker，外部恢复优先回该 worker；被窃取执行后更新归属。这只是内部局部性选择，不提供固定线程承诺，也不承担 payload 发布或关闭同步，归属读取／写入可使用 Relaxed 原子序。同池物理 worker 内部投递优先使用本地 FIFO，并先将已有 inbox 工作移入可用槽位；没有空间时继续排到 inbox，不能让 yield 越过已观察到的外部工作。队列不承诺不同来源之间的全局 FIFO，但须定期处理每个来源，不能用私有优先槽无限抢占旧工作。

本地槽位使用 managed 数组，每项工作领取后清空引用；在途窃取保留所占范围，完成复制和清空之前不能把槽位交回 producer。窃取中的 worker 暂停时，owner 可继续处理未预留工作，空间不足的新工作进入 inbox，其他窃取者不在该预留上自旋等待。inbox 的节点和在途读取仍是正常 managed 根，领取后清除 payload，并将旧 head 后继替换为独立终止标记；不能恢复为表示开放末端的自链接。标量 thread-local 状态仅识别物理 worker 所属池和序号，不保存 managed 指针或提供公开线程亲和保证。队列算法不使用 mutex，分配和 GC 仍遵守通常运行时合同。

休眠与队列分开：worker 先发布观察状态，再扫描全部工作来源，最后在自身 parking mutex 下检查通知后等待；生产者先发布工作，再以原子状态通知观察中的 worker，并在该 worker 的 mutex 下 signal。扫描已找到工作时到达的通知须转交其他观察者，不能因为本线程不再等待就丢弃其唤醒义务。关闭广播通知全部 worker；实际执行、入队、出队与窃取不持有 parking 或生命周期锁。

requestShutdown 幂等请求排空，不取消 Job。executor 的活动登记为零后才停止 coroutine 队列和自有 blocking pool 的接纳；正式 Terminated 还须全部 coroutine workers 退出执行循环、自己的 blocking 工作实际退出，以及自有 blocking pool 全部退出执行循环。共享池不关闭，只等待本 executor 的工作完成。悬挂且永不恢复的原始任务可以使关闭无限等待，这是其调用者的生命周期责任。

BlockingTaskPool 的 requestShutdown 停止接纳新 work，排空已接纳队列后退出；关闭后 submit 抛 ExecutorClosedException。借用共享池的 owner 必须在全部借用方关闭并 join 后关闭该池；提前关闭违反实际资源生命周期合同，不能指望活动 executor 的后续 blocking 投递仍可执行。借用方本身不关闭或 join 共享池。

isShutdown 表示已经请求关闭，isTerminated 表示逻辑排空并退出执行循环；两者为线程安全的单调只读状态，供实际 event loop／runtime 组合使用，不代替正确的共享池归属顺序。awaitTermination 可以被等待者取消，不撤销关闭，也不代替 OS thread join。join 要求先请求 shutdown，等待逻辑终止，再完成原生 join 和句柄回收；重复及多个外部线程并发 join 均安全。close 合并请求和 join。不得从本 executor 的活动任务、其 coroutine worker 或依赖本次阻塞工作退出的 worker 上等待自身终止；阻塞池也拒绝其 worker 的自等待／join。检查包含物理线程身份，不能只依赖在 blocking invocation 中为空的 currentCoroutineExecutorOrNone。

invokeOnTermination 是组合组件使用的普通一次性通知，不创建 coroutine，也不捕获登记者的 execution。逻辑终止时在内部锁外于完成线程调用；已终止时登记则在调用栈同步调用。Disposable 撤销尚未开始的通知，与已开始的 handler 竞争时不等待其结束；通知或撤销后解除函数捕获。一个 handler 抛出也须执行其余通知和等待者交付，之后传播首先捕获的异常；worker 边界按上述未捕获异常规则处理，同步的迟到登记直接向登记方抛出。内部 I/O owner 可以据此唤醒驱动，避免用继承 caller executor 的等待 coroutine 建立关闭循环。

逻辑终止在最后一个 work 返回且 worker 不再领取 work 后发布；线程的 managed gateway 尾部由 join 等待。因此 awaitTermination 对自有 blocking pool 也不隐含 OS join。join 回收线程句柄，保护最终状态的 Mutex／ConditionVariable 则随对象 GC 回收，关闭后的并发幂等操作和拒绝投递仍在有效同步对象上协调。部分线程创建失败时请求已创建 worker 退出、join 它们并传播原失败；不留下后台 worker。

## 6. Executor 配置

AsyncRuntime 通过以下窄接口将 reactor 接到 coroutine worker，不再另建调度队列。每个 driver 只交给一个 worker；资源由调用方在工厂调用前准备，调用方保留初始化失败后的清理责任。

```scoop
public interface CoroutineWorkerDriver {
    public fun enter()
    public fun poll()
    public fun park()
    public fun wake()
    public fun close()
}
```

enter 在 worker 执行任务前调用一次；poll 在有限批任务之间及空闲重查时非阻塞推进事件；park 仅在发布观察状态并重查后等待。wake 可跨线程并发调用，须让已开始或即将开始的 park 返回，不能丢失先于等待到达的通知。除 wake 外的方法均由所属 worker 串行调用。已启动 worker 在队列排空后调用一次 close，再发布 worker 终止；close 排空 driver 自有工作，不能等待其他尚未退出的 coroutine worker。wake 与 close 的竞争由 driver 的资源生命周期边界协调。driver 内部异常应交给所属 I/O 运行的结果；未捕获异常沿第 5 节 worker 协议处理。未启动 worker 的 driver 由调用方清理。

无 driver 时继续使用原有条件变量等待。自有 blocking pool 的停止推迟到全部 coroutine workers（含 driver 收尾）退出后；借用池仍由其 owner 管理。该接口只连接实际事件驱动与现有调度器，不提供后端注册表、第二套任务树或任务执行替代入口。

```scoop
public class BlockingPoolBuilder public constructor() {
    public var threads: Int
}
public class ExecutorBuilder public constructor() {
    public var workers: Int
    public fun blocking(configure: BlockingPoolBuilder.() -> Unit)
    public fun blocking(pool: BlockingTaskPool)
    public fun drivers(drivers: Array<CoroutineWorkerDriver>)
}
public fun multiThreadExecutor(configure: ExecutorBuilder.() -> Unit = {}): MultiThreadExecutor
```

workers 默认取平台逻辑 CPU 数的合法正 Int 表示，blocking 默认 4 threads。配置函数执行一次，结束后复制为不可变配置；再次修改捕获的 builder 不影响已建立对象。blocking 的两种配置分别选择自有池／借用池，最后一次配置生效，不能同时存在两组池。全部数量在创建任何线程前校验，非正值抛 IllegalArgumentException。工厂立即创建可用 executor；失败清理自身创建的池与 worker，不关闭借用池。BlockingPoolBuilder 是可跨 Cone 复用的普通配置类，后续 event loop／runtime 使用同一配置语义。

drivers 可选，数组长度必须等于 workers，否则在线程创建前抛 IllegalArgumentException。配置快照复制数组元素，不受之后修改原数组影响；driver 本身由各 worker 独占使用，不复制其资源。

## 7. 验证边界

普通行为在本 Cone 测试：容量与 FIFO、关闭排空、可空消息、取消与交付竞争、跨 executor 恢复、mutex 直接交接、condition 通知／取消后的重新获取、错误 guard 和异常清理。真实 managed threads 验证并发状态转换；定向 GC stress 验证挂起消息、guard 与 continuation 的托管根。两类线程池验证固定线程与真实并行、上下文隔离、排空期间的子任务、阻塞取消与退休、共享池归属、自等待拒绝和关闭并发；进程终止合同使用外部进程 fixture。实际可用范围以实施记录为准。
