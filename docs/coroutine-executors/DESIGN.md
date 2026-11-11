# Coroutine executor、通用阻塞任务池与异步同步原语设计

状态：已实现，验收结果与图形环境限制见 [PROGRESS.md](PROGRESS.md)。日期：2026-11-04。实施起点：`8d6e2f20df`。

本文覆盖单个及多个 executor 之间的消息传递、协程互斥与条件等待、多线程 executor，以及统一承载托管闭包和现有 native 阻塞工作的线程池。公共 API 使用符合 Scoop 语言规则的 receiver DSL。项目尚未发布，直接替换旧接口，不保留兼容别名或新旧双轨。第 12 节涉及的规范与实现已同步，本文的公共 API 已可使用；work stealing 保留为第 7 节所述的后续性能演进。

## 1. 目标与设计结论

本设计提供以下完整行为：

- `Channel<T>` 能在同一 executor、不同 executor、不同 worker thread 之间传递消息；阻塞条件只挂起协程，接收者和发送者各自回到所属 executor。
- `AsyncMutex` 与 `AsyncConditionVariable` 能跨挂起点、跨 executor 同步；等待不占住 OS thread，解锁不要求回到加锁时的 OS thread。
- `MultiThreadExecutor` 使用多个已登记到 Scoop runtime 的 managed worker，并行驱动不同任务；一个任务在不同轮次可以迁移 worker，同一时刻只能有一个驱动者。
- executor 管理独立的 `BlockingTaskPool`；`blocking { ... }` 可以执行普通 Scoop 闭包、分配托管对象、调用同步库和 FFI，返回类型化结果或异常。现有 GC-free native work 迁入同一套阻塞任务设施。
- `PlatformEventLoop` 提供固定 owner 线程的 executor；`AsyncRuntime` 组合 I/O owner 和 managed worker pool，使 I/O 归属与协程执行位置分开。
- 编译器统一处理启动、挂起、结果发布、恢复入队、TaskContext 和完成边界，覆盖普通函数、接口调用、闭包调用与跨 Cone 产物。
- 普通应用通过 runtime/executor 配置 builder、带 receiver 的 coroutine scope、`blocking` 与 `withExecutor` 工作，不需要手工处理 continuation、work 或任务登记。

现有 `Continuation` 足以手工实现一个会挂起的队列，但不足以给所有挂起点统一增加 executor 归属、可靠的跨线程恢复和关闭合同。因此本方案需要修改编译器与 core 协程协议。Channel、锁、条件变量、线程池调度算法仍然是普通库实现，不成为编译器 intrinsic。

不引入新 `async` 语法、Rust 式 `Future/Poll` 类型系统、`Send/Sync` 推导、抢占式协程或通用 continuation interceptor。新增的是 lambda 字面量按期望类型确定挂起性的规则，见 §3.5。阻塞任务使用普通函数类型，挂起任务使用 suspend 函数类型；不把二者混成可以任意阻塞的同一条 runnable 队列。

## 2. 现状与调研依据

### 2.1 设计提出时的仓库基线

本节记录改造前的能力；实际实施状态以 [PROGRESS.md](PROGRESS.md) 为准。

| 层次 | 已有能力 | 本次需要补齐的部分 |
| --- | --- | --- |
| core 协程 | 单次完成 continuation、同步完成锁存、结构化 Job、可取消等待 | executor、统一驱动边界、排队启动与显式让出 |
| TaskContext | 精确类型动态绑定、任务 fork、恢复时 enter/leave | 独立于普通绑定的任务执行属性 |
| asyncio | 单 owner 驱动、跨线程命令投递、native 完成队列、资源退休 | 任意挂起点的 owner 亲和性、worker 上调用 I/O 的完整审计 |
| platform | managed `Thread`、原子量、阻塞 Mutex/ConditionVariable | 基于协程等待队列的同步对象、managed executor |
| native I/O work | 执行文件、DNS、TLS 等 GC-free native 工作 | 迁为 executor 管理的通用 managed 阻塞任务池，保留各操作必要的退休与借用合同 |

关键实现入口：

- [core 协程入口](../../sysroot/src/scoop.core/src/coroutine.scoop)、[结构化并发](../../sysroot/src/scoop.core/src/coroutine_scope.scoop)、[可取消等待](../../sysroot/src/scoop.core/src/cancellable_coroutine.scoop)。
- [suspend 调用](../../compiler/mir-lower/src/coroutine/sites.rs)、[挂起握手 lowering](../../compiler/mir-lower/src/coroutine/intrinsics.rs)、[完成发布](../../compiler/mir-lower/src/coroutine/adapters/completion.rs)、[恢复驱动](../../compiler/mir-lower/src/coroutine/adapters/drive.rs)、[启动 helper](../../compiler/mir-lower/src/coroutine_registry/start.rs)。
- [LoopOwner](../../sysroot/src/scoop.asyncio/src/loop_owner.scoop)、[loop 驱动](../../sysroot/src/scoop.asyncio/src/loop_drive.scoop)、[I/O 操作仲裁](../../sysroot/src/scoop.asyncio/src/loop_operation.scoop)。
- [managed Thread](../../sysroot/src/scoop.platform/src/concurrent/thread.scoop)、[阻塞条件变量](../../sysroot/src/scoop.platform/src/concurrent/condition.scoop)。
- 原 NativeWork 与三平台独立阻塞池已由[原生任务退休](../../sysroot/src/scoop.asyncio/native/common/work.c)、[managed 阻塞投递](../../sysroot/src/scoop.asyncio/src/native_blocking.scoop)和 [TLS 类型化工作](../../sysroot/src/scoop.tls.asyncio/src/blocking.scoop)接替。

当前没有通用 async executor。I/O 挂起点由 loop owner 恢复，但第三方 `suspendCoroutine` 可以从其他线程恢复；已有 native work 的并发执行也不代表协程正文能在多个 managed worker 上执行。

现有三个 backend 的 blocking work 都限制为 native payload 和 GC-free 执行函数；Linux/Windows 使用 native worker，Darwin 使用 dispatch jobs 排空 FIFO。Scoop 已有能进入 managed 代码的 Thread，因此不需要把旧 C worker 冒充为 managed thread；迁移重点是复用线程登记、统一任务调度并保留 I/O 收尾。

当前成功、失败和取消是一次结果竞争：成功先提交后，取消不能改写成功。此规则继续保留，尤其用于消息和锁所有权交付。I/O 的结果选择与 native 请求退休仍是两个不同事实。

### 2.2 从 Tokio 与 Kotlin 借鉴什么

| 参考 | 本设计采用的部分 | 不直接移植的部分 |
| --- | --- | --- |
| [Tokio runtime](https://docs.rs/tokio/latest/tokio/runtime/index.html) | 分离调度、I/O driver 和 timer；多 worker；后续使用 work stealing | 不假设 Tokio 必须有专用 I/O 线程；Scoop 的 owner 线程来自现有 backend 合同 |
| [Tokio spawn](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) | 排队启动不在提交者调用栈内执行任务 | 不把“没有同步执行”解释成“返回以后才执行”；其他 worker 可能已经开始 |
| [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) | 阻塞任务有独立线程；排队与运行中取消分开；关闭等待真实结束 | 线程数量独立配置，不照搬其默认上限；不提供超时后遗留运行线程的关闭模式 |
| [Tokio sync](https://docs.rs/tokio/latest/tokio/sync/index.html) | 同步对象与具体 runtime 解耦，保存等待者自己的恢复目标 | 不要求 Channel 或 Mutex 拥有一个全局 executor |
| [Tokio Mutex](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html) | 可跨 await 持锁，等待者 FIFO | 不拿阻塞 OS mutex 代替用户可跨挂起持有的锁 |
| [Tokio Notify](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html) | 登记等待与检查条件必须组成正确协议 | 不用一个通知标志拼出 MPMC Channel 或 condvar；通知本身不等于队列条件 |
| [Kotlin Channel](https://kotlinlang.org/api/kotlinx.coroutines/kotlinx-coroutines-core/kotlinx.coroutines.channels/-channel/) | rendezvous、有限缓冲、背压 | 不引入其 prompt cancellation；Scoop 保留已提交成功不被后续取消覆盖的规则 |
| [Kotlin receiver builder](https://kotlinlang.org/docs/type-safe-builders.html) | 配置与作用域使用 receiver lambda，嵌套代码直接调用 scope 成员 | lambda 挂起性按 Scoop 明确的期望类型/重载规则推导，不依赖 DslMarker、泛型虚派发或非局部 return |
| [Kotlin runInterruptible](https://kotlinlang.org/api/kotlinx.coroutines/kotlinx-coroutines-core/kotlinx.coroutines/run-interruptible.html) | 区分等待取消与底层阻塞调用实际停止 | 不假设 pthread/Win32 任意函数具有 JVM Thread.interrupt 的语义 |

资料核对日期为 2026-11-04。上表是设计取舍，不意味着 Scoop 采用对应项目的源码或 ABI。

## 3. 分层与执行模型

### 3.1 三个相互独立的概念

| 概念 | 负责什么 | 不负责什么 |
| --- | --- | --- |
| `Job` | 父子关系、取消、结果与子树完成 | 选择 OS thread |
| TaskContext 的普通 binding | Job、EventLoop、日志等动态环境 | 声明当前物理线程或执行权限 |
| `CoroutineExecution` | 一个低层任务的 executor、驱动串行化与调度登记 | 替代 Job 树或深复制共享对象 |
| 阻塞 operation | 托管闭包的单次执行、真实退休与结果交付 | 驱动等待者的 coroutine frame，或使 FFI 脱离既有 GC 契约 |

`CoroutineExecution` 是 core 的内部 managed 对象，由协程入口建立。任务创建时选择 executor，之后保持不变。direct/virtual/closure suspend 调用共享它；新的低层任务拥有新的 execution。一个结构化 Job 可能在其正文 execution 结束后继续等待子 Job，这些子任务各有自己的执行登记。

TaskContext 增加一个可空 execution 引用，独立于现有 binding tree。`context(executor) { ... }` 只是普通动态绑定，既不选择 executor，也不切换线程。读取当前 executor 使用专门入口，切换执行位置使用 `withExecutor`。

这个区别避免三个歧义：普通绑定恢复不应暗中要求线程迁移；callback 捕获的环境不应冒充正在运行的 worker；一个任务不应在任意词法 `context` 块中悄悄改变后续恢复目的地。

### 3.2 Cone 归属

| 位置 | 新增内容 |
| --- | --- |
| `scoop.core` | executor 协议、execution 支持、排队启动、receiver scope、`withExecutor`、`blocking`、`yieldCoroutine` |
| 新 Cone `scoop.concurrent` | `Channel`、`AsyncMutex`、`AsyncConditionVariable`、`MultiThreadExecutor`、`BlockingTaskPool` 与配置 builder |
| `scoop.asyncio` | `LoopExecutor` 接入、`AsyncRuntime` 与配置 builder、跨 worker I/O、现有 native work 迁移 |
| `scoop.platform` | 复用 Thread、Mutex、ConditionVariable 与原子量；只补实际缺失的平台能力 |
| compiler/runtime | typed 协程操作、TaskContext 布局、恢复驱动与 GC 合同 |

`scoop.concurrent` 依赖 core、std、platform，不依赖 asyncio；asyncio 依赖 concurrent。这样 Channel 和 Mutex 可以在没有 I/O event loop 的程序中使用，也不存在 executor 与 asyncio 的循环依赖。

### 3.3 并发与内存模型

一个 execution 任意时刻最多一个驱动者；不同 execution 可以并行。`MultiThreadExecutor` 的归属表示一个 worker 集合，不承诺线程固定；`LoopExecutor` 的归属表示固定 owner 线程。

队列提交、锁交付和恢复 payload 使用 release/acquire 或具备相同效果的锁发布。Channel 发送之前的写入对接收成功后的代码可见；Mutex 解锁之前的写入对下次取得锁后的代码可见。

这不消除共享引用的数据竞争。Scoop 当前没有 `Send/Sync`，普通 Context binding、闭包捕获和 Channel 元素都可能包含共享可变对象。消息交付不会深复制或撤销发送者的别名；跨任务继续访问这些对象仍须按语言内存模型同步。

### 3.4 公共 API：receiver DSL 与明确的执行边界

常用入口采用以下风格，并使用 §3.5 的期望类型推导：

```scoop
val total = asyncRuntime {
    workers = 4
    blocking {
        threads = 8
    }
}.run {
    val first = async { blocking { readAndParseFirstFile() } }
    val second = async { blocking { readAndParseSecondFile() } }
    first.await() + second.await()
}
println(total)
```

`readAndParseFirstFile/SecondFile` 表示应用的普通同步函数，可以包含 native 阻塞调用和托管解析代码；它们不是新库 API。配置中的 `blocking { threads = ... }` 是 builder 的真实成员，正文中的 `blocking { ... }` 是挂起等待普通闭包结果的函数，二者处于不同静态 receiver 和调用上下文。

公共入口按以下规则组织：

| 用途 | 入口与形状 |
| --- | --- |
| 独立异步程序 | `asyncRuntime(configure: AsyncRuntimeBuilder.() -> Unit = {})`，返回 `AsyncRuntime`；`run<T>(block: suspend CoroutineScope.() -> T): T` |
| 单 owner/宿主事件循环 | `eventLoop(configure: EventLoopBuilder.() -> Unit = {})`，返回 `PlatformEventLoop`；其 final `run<T>` 与 attach 使用 receiver scope |
| 手工管理 worker executor | `multiThreadExecutor(configure: ExecutorBuilder.() -> Unit = {})`，返回 `MultiThreadExecutor` |
| 子任务 | `CoroutineScope.launch(block: suspend CoroutineScope.() -> Unit)`、`async<T>(block: suspend CoroutineScope.() -> T)`；另有显式 executor 参数的 overload |
| 词法子作用域 | `coroutineScope`、`nonCancellableScope`、`withExecutor` 接受 `suspend CoroutineScope.() -> T` |
| 普通阻塞调用 | `blocking(block: () -> T): T` 或 `executor.blocking(block)`，两个入口均为 suspend |
| 简单同步对象 | `Channel<T>(capacity = 16L)`、`AsyncMutex()`、`AsyncConditionVariable(mutex)`，不为一个参数增加 builder |

配置 builder 是普通 class：worker 数 `workers: Int`、阻塞配置 `blocking(BlockingPoolBuilder.() -> Unit)` 与 `threads: Int` 是实际使用的字段。worker 默认取平台逻辑处理器数的合法正 Int 表示；阻塞线程默认 4，可独立调整。所有数量在建立任何线程之前校验，非正值报参数错误。配置函数执行一次，结束时复制为不可变配置；捕获 builder 后继续修改不影响已构造对象。重复配置同一字段按源码最后一次赋值决定，不建立隐式多份线程池。

runtime/event loop 工厂只建立配置，run/attach 接受运行后才建立资源；MultiThreadExecutor 工厂直接建立可用 executor，失败须回滚已创建资源。它提供同步 `close()`，合并 requestShutdown 和 join，并实现 Closeable，以便非 worker 的普通根入口使用 `use { executor -> ... }`。异步内部使用显式关闭请求与等待，不在自己的 worker 上调用同步 close。

`run<T>` 是具体 final class 方法，不能变成 `EventLoop` interface 的泛型虚方法。`EventLoop` 保留 I/O 服务与 executor 属性，根运行归拥有资源的具体类；退役 `EventLoop.run` 虚方法和 `runWithResult` 别名，统一使用类型化 run。不能为这个 DSL 放宽 Scoop 的泛型单态化/虚派发限制。

receiver 仅用于配置对象、CoroutineScope 和锁 guard；不会把 TaskContext 加入隐式 receiver 列表。嵌套 scope 默认绑定最近的 scope，`launch/async` 的 block 获得自己 child Job 对应的 scope，不能继续把外层 scope 冒充子任务 scope。遵守语言规范 §8.1.2 已有的 receiver 查找边界：裸名实际选中外层 receiver 时诊断，不回退另一个候选；明确使用外层 scope 时使用变量或 `this@label`。

Scoop 已有 receiver 函数类型，不需要新增 DSL 专用编译器特判；省略 suspend 则由下一节的通用语言规则支持。receiver 与展开为首参数的普通函数类型可以适配，所以不为两种写法保留成对重载。也不引入 Context 元素相加、GlobalScope、隐式全局 Dispatchers、JVM 线程中断或 Kotlin 式非局部 return。捕获外层局部 var 仍非法，需要共享状态时显式捕获引用对象并同步；不通过 builder 偷偷生成可变捕获 cell。

### 3.5 Lambda 挂起性的期望类型推导

允许调用处省略 lambda 字面量的 suspend，但函数声明和函数类型中的 suspend 仍明确表示调用效果。例如 `launch { awaitSomething() }` 可以成立，普通具名函数调用挂起函数仍须把函数声明为 suspend。

#### 3.5.1 推导边界

| 表达式与期望类型 | 处理 |
| --- | --- |
| `{ ... }`，期望 `suspend A.() -> T` 或其他 suspend 函数类型 | 作为挂起 lambda 检查正文，生成同类型的挂起函数值 |
| `{ ... }`，期望普通函数类型 | 作为普通 lambda 检查；直接挂起调用仍报错 |
| `{ ... }`，约束固定点后没有函数期望类型 | 默认普通 lambda，不通过扫描正文或外层是否 suspend 猜测其效果 |
| `suspend { ... }`、`suspend fun ...` | 明确是挂起函数值；普通函数期望类型不能接收 |
| `fun(...) { ... }` / `fun A.(...) { ... }` | 明确是普通匿名函数；用于需要锁定 ordinary 效果的场景 |
| 已定型函数值、具名函数/扩展函数引用 | 保持声明的挂起性，不隐式生成普通到 suspend 的 adapter |

这改变的是尚未定型的 lambda 字面量，不给所有普通函数类型增加到 suspend 类型的子类型关系。receiver 适配只在最终挂起性相同的前提下进行，不能被用来跨越这一边界。

```scoop
launch { blocking { readSynchronously() } }
val task: suspend () -> Int = { readAsynchronously() }
val plain = { 42 }

// The typed value remains an ordinary function; wrap its invocation explicitly.
launch { plain() }
// launch(plain) is a type error, even though the body itself cannot suspend.
```

示例中的 read 函数由应用提供。`val task` 只创建函数值，不启动任务；即使没有任何实际挂起路径，期望 suspend 类型的 lambda 也具有 suspend ABI。外层函数是 suspend 不会使它里面所有 lambda 自动变成 suspend，例如 `blocking { readAsynchronously() }` 仍非法。

期望类型可以来自变量注解、赋值、返回类型、已经确定的形参和既有的结构化表达式期望传播。对推导尚未决定的调用，把 lambda 继续作为 postponed argument；不能因为第一次尝试的候选期望 ordinary，就把 AST 永久标成 ordinary。

#### 3.5.2 同名普通/挂起回调重载

这确实会引入新的候选重叠。例如 core 的普通 use 和挂起 use 都可能接收没有挂起调用的 `{ ... }`。采用以下有限规则，不要求用户在每个资源作用域都写 suspend：

1. 先按既有候选层、参数映射、泛型约束和调用上下文筛选；ordinary 调用上下文不能选择需要实际挂起调用的入口。
2. 各候选独立提供 lambda 的期望 receiver、参数、结果和挂起性，并独立检查正文。正文中的真实挂起调用会使 ordinary 解释不适用，但不把任意候选的回调签名改写成 suspend。
3. 先应用现有最具体候选与既有附加规则；只有仍并列的候选，才适用下面的挂起性缺省规则。
4. 对 **effect 对称重载**，优先选择不必把这些隐式 lambda 解释为 suspend 的候选。对称要求声明的 receiver、类型参数及 bounds、普通参数和结果类型按类型参数重命名后相同，只允许对应 lambda 形参的顶层挂起性及入口自身的挂起性不同。只比较已通过普通检查的候选，不用本次猜出的泛型结果伪造“更具体”。
5. 多个 lambda 时按逐位置比较：A 在每个位置要求的挂起性不强于 B，且至少一处从 suspend 降为 ordinary，A 才优先。不能把位置数加总打分；`(ordinary, suspend)` 与 `(suspend, ordinary)` 无此优先关系。
6. 非对称候选、结果/receiver/参数形状不同或仍有多个不可比较候选时，继续报歧义，不按声明顺序、正文有没有某个名字或最终返回类型猜选。

因此，在允许挂起的调用上下文中，`resource.use { pureWork() }` 选普通 use；`resource.use { awaitWork() }` 只有挂起 use 的正文检查通过；`resource.use suspend { pureWork() }` 明确选择挂起 use。同理，AsyncMutex 的两个 withLock overload 可以自然使用；无歧义的 launch/async 本来只有 suspend 期望，不需要额外关键字。

这个优先级不使仅入口自身相差 suspend 的重复声明变合法，也不改变同名不同返回类型不能单独构成重载的规则。显式普通匿名函数、显式 suspend lambda 或带完整函数类型的局部 val 可以解决剩余歧义；诊断须展示各候选要求的函数效果和冲突参数。

#### 3.5.3 编译器与产物

AST 保留 lambda 的“未显式指定 / 显式 suspend”来源，普通匿名函数仍有显式 ordinary 效果；候选级检查分别持有推导结果。完成 HIR lowering 时，每个函数值必须已经具有唯一的普通或挂起类型，不把待推导 effect、可空 suspend 标记或任意默认猜测交给 MIR。

MIR/LIR 沿已定型 callable 选择现有普通 ABI 或 coroutine lowering；一个字面量不同时生成两种效果的可运行对象。跨 Cone 模板和 `.slib` 保存最终类型、调用选择及必要源码信息，消费者不能因自己的 import 或重载集合重新决定已绑定模板中 lambda 的挂起性。

改动集中在语法来源表达、HIR postponed argument/候选隔离、对称重载的缺省规则及诊断；不新增运行时效果检测。LSP 的签名、悬停和诊断应显示最终推导出的 suspend 类型。普通参数边界、构造/初始化、NoGC、FFI、函数值型变、捕获和局部 return 的既有规则继续检查；不因为 lambda 可以省略关键字就放宽这些边界。

## 4. core 协议与任务入口

以下是拟定的声明形状，省略函数体与实现私有字段；不是当前源码即可编译的示例。

```scoop
public interface CoroutineWork {
    public fun run()
}

public interface BlockingWork {
    public fun run()
}

public interface CoroutineExecutor {
    public fun registerTask(parent: ExecutorTask?): ExecutorTask
}

public interface ExecutorTask {
    public val executor: CoroutineExecutor
    public fun dispatch(work: CoroutineWork)
    public fun dispatchBlocking(work: BlockingWork)
    public fun complete()
}

public class ExecutorClosedException public constructor() : Exception

public fun currentCoroutineExecutorOrNone(): CoroutineExecutor?

public fun <T> startDispatchedCoroutine(
    executor: CoroutineExecutor,
    task: SuspendTask<T>,
    completion: Continuation<T>
)

public suspend fun <T> withExecutor(
    executor: CoroutineExecutor,
    block: suspend CoroutineScope.() -> T
): T

public suspend fun <T> blocking(block: () -> T): T
public suspend fun <T> CoroutineExecutor.blocking(block: () -> T): T

public suspend fun yieldCoroutine()
```

`startDispatchedCoroutine` 同时提供与 `startCoroutine` 对称的 suspend 函数类型 overload，普通 adapter 复用同一底层入口。`CoroutineWork`、`BlockingWork` 和 `ExecutorTask` 是 executor 实现者协议，普通应用主要使用结构化 builder。两种 work 使用不同类型，防止把 coroutine frame 的恢复直接派到阻塞 worker。

### 4.1 登记与投递合同

`ExecutorTask` 是任务生命周期的普通登记对象，不是资格凭证。它解决“任务已接纳，但后续恢复时 executor 已关闭”的实际竞争。

1. 新任务先取得登记，再发布其第一项 work。关闭拒绝发生在登记阶段，以 `ExecutorClosedException` 失败；失败的登记不执行正文、不产生完成通知。
2. 结构化 builder 优先使用所属 scope 的活动执行登记作为同 executor 的 parent；没有该父登记时，可以使用当前任务同 executor 的活动登记。两者都不属于目标 executor 时传 `None`，视为目标 executor 的新根登记。阻塞 operation 同样登记，以覆盖其运行和最终交付。
3. 内置 executor 在 `Open` 时允许根和子登记，在 `Draining` 时只允许由自身仍活动登记派生的子登记，在 `Terminated` 时全部拒绝。判断与登记计数更新在同一临界区完成。
4. 对仍活动登记的 `dispatch`，executor 必须接受、保活并恰好执行一次 work；不能因进入 `Draining` 而拒绝，不能静默丢弃，不能在调用 `dispatch` 的栈内调用 `work.run()`。
5. work 可以在另一 worker 上于 `dispatch` 返回前开始；发布前必须已经完成 payload、状态和 GC 根准备。
6. `complete` 在最终 completion 通知结束、该 execution 的驱动退出之后执行一次。重复完成或完成后投递属于协议错误；普通用户取消不会提前结束登记。

`dispatchBlocking` 对合法活动登记同样保证接受、保活和排队执行，但目的地是关联的 blocking pool。取消可以使工作包装器不再执行用户闭包，不能使已开始的工作失去生命周期登记。`BlockingWork.run` 的单次执行和用户闭包是否获准开始由 §9.6 的状态协议协调。

登记只保活有实际执行义务的任务。持有 executor 对象本身不阻止关闭。一个库不得在恢复失败时偷偷改用另一 executor 或直接在外线程驱动 continuation。

`dispatch`、`dispatchBlocking` 与 `complete` 对合法调用不得抛出普通异常；队列节点可以在发布前准备，内置池在接纳任务前完成 worker 初始化。不可恢复的分配失败遵守现有 runtime 规则，不增加假成功或丢任务的恢复路径。自定义 executor 违反协议按普通不变量错误处理，不增加来源认证或逐次完整校验。

### 4.2 入口行为

| 入口 | 首次执行 | executor 与结构化关系 |
| --- | --- | --- |
| `startCoroutine` | 当前栈内立即启动，作为明确的低层入口 | 新 execution 继承调用者 executor；不创建 Job |
| `startDispatchedCoroutine` | 总是排队 | 新 execution 选择参数 executor；不创建 Job |
| `scope.launch/async(block)` | scope 有 executor 时排队；无 executor 的低层作用域才立即启动 | 使用 scope 的默认 executor，binding 取调用处快照，登记到 scope 的 Job |
| `scope.launch/async(executor, block)` | 总是排队 | 选择参数 executor，binding 取调用处快照，父 Job 仍是该 scope |
| `coroutineScope` | 可在当前驱动内开始 | 保留现有失败边界与等待整个子树的语义 |
| `nonCancellableScope` | 同上 | 隔离父取消，继续继承 executor；清理不会切到别的线程池 |
| `withExecutor(executor, block)` | 新子任务排队，参数与当前 executor 相同也排队 | 新作用域作为失败边界；完成整个子树后返回原任务 |
| `blocking(block)` | 当前 executor 的阻塞队列 | 普通闭包在新 binding 快照中运行，当前 coroutine 等待实际结束 |
| `executor.blocking(block)` | 显式 executor 的阻塞队列 | 调度归显式目标，结果回等待者原 execution |

`startCoroutine` 只能继承正在当前调用路径中执行的 execution，因而不会把任务“立即启动到一个未进入的 executor”。普通同步 native 往返保留当前任务；`foreignCallback` 的独立 invocation 仅继承 binding snapshot，不继承 execution，其内部读取当前 executor 得到 `None`。callback 恢复既有 continuation 时，使用目标 continuation 保存的 execution；callback 可以通过仍开放的 scope 排队启动子任务，也可显式选择 executor，不能根据 EventLoop binding 推断当前物理位置。

scope 改为保存 Job、默认 executor 和正文活动期间的父执行登记。默认 executor 来自该作用域建立时的任务执行归属；普通 Context binding 仍取 launch/async 调用处快照，不把整个 TaskContext 冻结到 scope 中。这样从 foreign callback、阻塞闭包或其他 executor 调用已捕获的 scope，也不会悄悄把新任务启动到调用线程。

调用者先把 child attach 到父 Job，再登记和发布工作；登记失败必须通过既有 Job 完成路径结束该 child，不能留下永远等待的子任务。block 的 receiver 是新 child scope；该 scope 关闭时解除其父执行登记引用，关闭后仍禁止启动子任务。这是明确的 breaking change，不保留“scope 只存 Job、默认 executor 取调用线程环境”的双重规则。

结构化任务已被接纳后，开始执行时若 Job 已取消，则结束为取消并照常完成登记。调度不得为任务创建一个脱离父子关系的隐含 Job。

### 4.3 `withExecutor` 的结果与取消

`withExecutor` 复用 `coroutineScope` 的失败边界：目标侧子树失败先在该边界收尾，再从调用点抛出，不能在调用者有机会 catch 之前取消调用者 Job。

等待使用内部不可提前取消的 join；调用者取消会传给目标子树，但仍等待目标正文、finally 和全部子任务真实完成。结束后原调用 frame 通过自己的 execution 恢复，因此正常值、异常和取消都返回原 executor。

不得简单实现成 `scope.async(executor, block).await()`：公开 `await` 可因等待者取消提前结束，不能承担这个结构化切换边界。也不能把 executor 放进普通 `context` 块后直接执行 block。

### 4.4 结果提交与未捕获异常

调度不改变现有单次完成和取消规则：已提交成功，即使仍在队列中，也必须交付成功；不在每次恢复前插入会覆盖已提交结果的 `ensureActive`。

任务正文异常交给 completion/Job，不能逃逸并悄悄杀死 worker。低层用户 completion 自身抛出时，不得把它当成正文失败再次通知同一 completion：先通过 finally 完成执行清理和登记释放；同步入口向调用者传播，异步入口走明确的未捕获异常终止路径。内置 executor 不通过停止单个 worker 并遗留队列来“处理”该错误。

## 5. 编译器的挂起、发布与恢复合同

### 5.1 两层状态不能混为一层

continuation 的“结果已经确定”和 execution 的“现在可以驱动”是两个不同事实。前者保证只完成一次，后者保证一个任务不被并发或重入驱动。

| 层次 | 概念状态 | 保护的事实 |
| --- | --- | --- |
| adapter | `Registering`、`Waiting`、`Publishing`、`ReadySuccess/Failure`、`Consumed` | 一份结果只能发布和消费一次 |
| execution | `Running`、`Idle`、`Queued`、`Finished` | 同一逻辑任务只有一个驱动者、最多一个调度票据 |
| frame | 当前执行位置、挂起位置、完成位置 | typed locals、异常入口、Context 和程序计数 |

这里的状态名描述合同；最终 MIR/LIR 用封闭的 typed enum 和字段 role 表达，不使用无来源整数标记。`Publishing` 期间的结果槽不可被另一个线程读取；只有 release 发布的 ready 状态允许 acquire 消费。发布临界路径不调用用户代码、不执行可能抛普通异常的操作；需要等待该短暂状态的路径必须保留 GC 进展，不能在 managed 状态无限忙等而阻止发布线程推进。

每个 adapter 的存储形态由它的用途决定。需要跨线程保存结果的 adapter 必须具有精确 `T` 的 payload、Throwable、目标 frame 和 execution 引用；没有初始化的值使用现有合法的初始化状态表示，不能让 GC 扫描未初始化引用。

### 5.2 挂起握手

对原始 registration、direct suspend、virtual suspend 和闭包 suspend 使用同一套可证明的握手规则：

1. 保存跨挂起 live values、TaskContext 与恢复位置，建立处于 `Registering` 的 adapter，然后调用 registration 或 callee。
2. 调用尚未返回时收到 completion，先竞争一次结果发布权，只保存结果；不驱动当前仍在调用栈上的 frame。
3. registration/callee 返回后，由当前驱动者决定同步消费已锁存结果，或把 adapter 置为 `Waiting` 并返回 `Suspended`。
4. completion 与 `Registering -> Waiting` 竞争时，恰有一方负责把结果变成可执行 action。不得两边都入队，也不得双方都以为另一方会入队。
5. 整个当前驱动恢复 resumer Context、结束 native catch、退出 frame 访问后，才释放 execution 的驱动权并发布下一轮 work。

直接调用返回 `Completed(value)` 时，由当前驱动继续；callee 不得同时通过隐藏 completion 完成。callee 返回 `Suspended` 但结果已经锁存时，可以在当前驱动安全消费结果。不能像简单的“callee 调用前就设为 Waiting”那样，允许外线程在调用尚未退出时重新进入父 frame。

同步 `suspendCoroutine` 保留现有行为：registration 在返回前完成则当前栈继续；未完成便抛出则在挂起点抛出；已经完成后又抛出则以 `IllegalStateException` 失败，并使该 continuation 失效。同步锁存不产生额外 executor 投递。

### 5.3 execution 驱动门

execution 内部保存待执行 action 与驱动状态，使用短临界区协调：

- 当前为 `Running`：外部完成只提交 action，不取得驱动权。
- 当前为 `Idle`：提交者把状态改为 `Queued`，发布唯一的 work。
- 当前为 `Queued`：新的合法 action 归并到同一待执行状态，不再重复投递。
- worker 运行 work：以 acquire 取得 `Queued -> Running`，然后执行 action。
- 当前驱动退出：先完成所有 Context/异常出口，再原子检查待执行 action；没有 action 则 `Idle`，否则 `Queued` 并在锁外 dispatch。
- 最终完成：进入 `Finished`，解除不再使用的引用，再释放任务登记。

合法单任务控制流不会出现两个独立挂起分支同时继续。驱动门不是支持任意并发调用同一 continuation 的多路执行器；重复 resume 仍按原规则报错，且不能先改写 payload、Context 或队列。

TaskContext 表示逻辑任务环境，不能单凭其中的 executor 引用判定某个外部线程有驱动权。driver 只经上述获胜入口进入 frame。foreign callback 的 Context fork 和 worker 的常驻 callback 环境都不带别的任务 execution。

### 5.4 恢复、内部完成链与栈深度

有 executor 时，真实挂起后的外部恢复总是入队，包括同 executor 中另一个任务发出的恢复。这样 `send`、`unlock`、I/O completion 不会在自己的调用栈里突然运行对方任务。无 executor 时保留当前线程恢复模型，但仍经驱动门避免与尚未退出的调用并发；该模式不提供线程亲和性。

同一 execution 内，编译器生成的 callee-completion 向 caller 传播可以由内部 action 循环迭代执行：当前 driver 返回后消费下一个内部 action，不递归调用上一层 driver。它只用于已证明属于同一 execution 的编译器控制流，不把用户公开 continuation 的恢复变成任意 inline 快路径。

由此，深层 suspend 调用链的完成不必每个 frame 都回全局队列，也不会靠递归 resume 持续增长 native 栈。用户自身的普通递归仍遵守原有栈语义。

### 5.5 真正让出执行机会

`yieldCoroutine` 是新的 core intrinsic。存在 executor 时，它记录下一个恢复位置，强制结束当前驱动，在释放驱动权后把本任务放回 executor 的普通 runnable 队列；即使投递时另一 worker 立刻领取，也不能与旧驱动重叠。

不能用 `suspendCoroutine { executor.dispatch { continuation.resume(Unit) } }` 代替：另一 worker 提前 resume 时可能被同步锁存，实际根本没有让出。

让出前和重新执行该挂起点时检查取消；它不承载已经转移所有权的 payload。没有 executor 时只检查取消并返回。让出不承诺某个特定其他任务先运行，不提供时间片或硬实时保证，也不替代 GC safepoint。

### 5.6 GC 与物理 ABI

frame、execution、work、executor 队列、等待节点、结果和异常全部保持普通 managed 引用，使用精确 TD/scan 和写屏障；队列取出的值在离开锁后仍须为活跃根。消费或撤销后清空旧 payload、continuation、订阅和链表边，避免任务完成后被队列长期保活。

TaskContext 的新 execution 字段改变实际布局，必须同步编译器生成布局、runtime 后备、callback fork 和 GC 描述。binding snapshot 仍只保存不可变 binding root；fork 操作用明确的任务创建模式决定 execution，不能把 callback fork 与 coroutine fork 当成完全相同的复制。

本设计不需要 C runtime 调度器或新增保存 executor 的 TLS 根。当前 Context 根仍经既有 TLS slot 访问；新增引用由 TaskContext 扫描。worker 使用现有 managed Thread attach/detach，park/join 走 NativeSafe。若实现发现确需新增 runtime ABI，则在规范中明确增加，不以“全是普通库代码”为由绕过 ABI 更新。

## 6. 同步原语共用的等待协议

三个同步对象都不保存“自己的 executor”。它们保存每个等待操作的 continuation；该 continuation 已包含等待者的 execution，交付时自动返回正确 executor。

内部使用短持有的同步锁保护状态和等待队列。临界区只修改状态、队列与待交付列表，不执行用户代码、不调用 continuation、不 dispose Job handler、不持锁跨 suspend。现有平台 Mutex 可用于这类短临界区；它与用户层 AsyncMutex 的持锁时间、用途不同。

### 6.1 只允许一个结果仲裁点

等待节点至少区分未入队、等待中、成功提交、失败/取消提交、已交付。对消息、锁所有权和等待节点的终态，在同一个对象临界区内提交；随后在锁外解除取消订阅并恢复 continuation。

不能先在 Channel 锁内移交消息，再调用现有 `CancellableContinuation.tryResume`，指望后者一定成功：其自身的 Job handler 可能已经独立选中取消，导致消息消失。Mutex 同样可能把锁交给一个已被取消的等待者。

本设计的原语内部使用原始 `suspendCoroutine` 加 `Job.invokeOnCancellation`，让取消 handler 与匹配操作竞争同一个等待节点状态。不为此增加公开的两阶段 continuation API，也不修改普通 `suspendCancellableCoroutine` 的既有用途。

### 6.2 登记与清理顺序

1. 创建节点，保存 continuation、参数和待登记状态。
2. 登记 Job handler。已经取消的 Job 可能立即同步调用 handler；handler 在原语锁内把节点选为取消，在锁外交付。
3. 取得订阅对象后，在同一原语锁内判断节点是否已结束；未结束才保存订阅、尝试立即完成或排入队列，已结束则在锁外 dispose 迟到的订阅。
4. 正常匹配、关闭或取消在锁内选择唯一结果、摘除节点和整理要恢复的列表；在锁外执行全部通知。
5. 已开始执行的 handler 与 dispose 可竞争，因此 handler 每次都检查节点终态。解除订阅不是阻止迟到 handler 的正确性前提。

快速路径可以在入口检查取消后直接完成，避免分配等待节点；与后续取消并发时，只能由实际获胜的状态转换决定结果。同步成功不必产生一次人为挂起。

等待队列需要支持取消时摘除任意节点；可以实现内部可摘除链表，复用于这三个实际消费者。消息缓冲和 runnable 队列复用 std 的 `ArrayDeque`。不为此建立通用插件式等待框架。

## 7. Channel

### 7.1 公共形状与正常操作

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

Channel 为 MPMC；`capacity == 0` 是双方会合交付，`capacity > 0` 是有限 FIFO 缓冲，负数报 `IllegalArgumentException`。首版不使用负数魔法值表示无限、覆盖旧值或自动丢弃模式。

- `send`：有等待接收者时直接匹配，否则缓冲有空位时入缓冲，否则挂起；rendezvous 只能与接收者匹配。
- `receive`：先取最早已提交消息，并按顺序接纳等待发送者；无缓冲消息时可直接匹配发送者，否则挂起。
- `receiveOrNone`：正常关闭且缓冲排空时返回 `None`；`T` 本身可空时，`Some(None)` 仍是有效消息。
- `trySend/tryReceive`：不挂起，不安装 Job 取消监听；它们可以完成对方已经等待的操作，也可以从普通 managed thread 调用。
- `close`：第一次把状态从开放变成关闭返回 true，后续返回 false；允许任何 managed thread 调用。

FIFO 以锁内确定的匹配、入缓冲与等待登记顺序为准，不以不同 OS thread 的函数调用起始时间为准。新调用不能越过仍有效的同侧老等待者；不同 executor 的实际恢复先后不承诺一致。

### 7.2 交付、取消与关闭

| 情况 | 结果 |
| --- | --- |
| 发送节点取消先胜 | 节点移除，消息未交付；发送者收到取消 |
| 接收节点取消先胜 | 节点移除，不消费消息 |
| 双方匹配先胜 | 同一临界区提交两方成功；后续取消不撤回交付 |
| 缓冲入队先胜 | 发送成功，消息归 Channel 保存；发送任务后续取消不删除它 |
| 关闭先胜 | 拒绝新发送，让尚未提交的等待发送者以 `ChannelClosedException` 结束 |
| 已关闭且仍有缓冲 | 接收继续排空，不接受关闭前仅在等待队列中的未提交发送 |
| 已关闭且缓冲为空 | `receive` 抛 `ChannelClosedException`，`receiveOrNone` 返回 `None`；结束全部等待接收者 |

消息匹配和取消的临界区提交就是所有权交付的线性化点。成功已经确定但接收者尚未被调度时，消息仍保存在其 typed 等待结果中，不会重新放回 Channel，也不会被关闭或取消丢弃。

`close` 不丢弃已提交缓冲，不调用任意元素的清理代码。调用者应明确安排消费者排空；携带必须显式关闭的资源时，应在消费路径或应用自己的清理路径处理。首版没有 abort/drop-buffer API，因此无需引入任意线程上的 undelivered 回调。

GC 不承担 Channel 的关闭。没有“最后一个 sender 被回收时自动 close”的行为；Scoop 的 release hook 不能恢复托管协程，GC 时机也不适合作为消息协议终点。

## 8. AsyncMutex 与 AsyncConditionVariable

### 8.1 Mutex 和 guard

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

guard 只能由库内部构造。锁非重入，以 guard 表达一次持有，不以 OS thread ID 表达所有者。重复获取自己已经持有的锁可以等待自身；不增加自动死锁检测。guard 可以随任务迁移 worker，也可通过明确的交接由另一任务释放，但不得让多个任务同时把同一 guard 当作有效持有权。

等待队列为 FIFO；`tryLock` 不能插到已有等待者前面。解锁在锁内直接把持有权预留给最早有效等待者，之后锁外恢复；不能先标为空闲再唤醒，以免被新调用抢走。

等待取消先胜则不取得锁；锁交付先胜则返回有效 guard，即使恢复前 Job 已取消。`withLock` 取得 guard 后立即进入覆盖 block 的 try/finally，保证 block 抛出或在挂起点取消时释放。两个 overload 分别接受普通和 suspend receiver 函数类型，取得锁本身都可以挂起；字面量按 §3.5 推导，已定型函数值不跨效果转换。仅有 raw `lock` 的调用者须自行遵循同样规则。

guard 为引用对象，所有别名共享 `Held`、`ConditionWaiting`、`Released` 状态。`dispose` 对已释放 guard 幂等；在 condition 暂时释放期间对同一 guard 的外部操作报状态错误。GC 不自动解锁，异常不使锁永久 poisoned。

### 8.2 Condition 的关联关系

```scoop
public class AsyncConditionVariable public constructor(val mutex: AsyncMutex) {
    public suspend fun await(guard: AsyncMutexGuard)
    public fun signal(guard: AsyncMutexGuard)
    public fun broadcast(guard: AsyncMutexGuard)
}
```

Condition 固定关联一个 AsyncMutex。三个操作都要求传入该 mutex 当前有效的 guard；错误锁、已释放 guard 或正在等待的 guard 在操作前报 `IllegalStateException`。signal/broadcast 要求持锁，使谓词修改与通知使用一致协议。

应用必须循环检查谓词：

```scoop
mutex.withLock {
    while (!state.ready) {
        condition.await(this)
    }
    consumeReadyState()
}
```

代码仅示意谓词协议；`state` 是通过 val 捕获的应用状态对象，其 ready 字段和 `consumeReadyState` 均由这把锁保护，不依赖 Scoop 禁止的外层局部 var 捕获。

### 8.3 原子释放、通知与取消后的重新加锁

Condition 等待队列与关联 mutex 使用同一内部状态锁。`await` 在一个临界区完成“登记等待节点、将 guard 标为等待、释放持有权、向 mutex 下一等待者交接”；之后才在锁外通知。不存在释放锁后尚未登记的丢通知窗口。

`signal` 将最早的 condition 等待节点选为成功，并移到 mutex 的重新获取队列；`broadcast` 对当前全部等待者执行同样操作，保留其队列顺序。通知不等于已经拿到锁，只有轮到该节点取得 mutex 后才能恢复 await。

取消规则为：

- 进入 await 前已经取消：保持原 guard 有效，直接抛出取消，不释放 mutex。
- 仍在 condition 队列时取消先胜：从 condition 队列移除，携带取消结果进入 mutex 重新获取队列。
- 通知先胜：成功结果已确定，后续取消不改写该次 await；仍等待重新取得 mutex，再正常返回。
- 无论成功还是取消，重新获取都不再接受取消；原 guard 恢复 `Held` 后，才交付正常结果或抛出取消异常。

因此 await 从挂起中返回或抛出时，调用者重新持有同一个 guard；外层 finally 可以按普通规则解锁。重新获取使用内部不可取消的获取路径，不能调用会立即观察已取消 Job 的公开 `lock()`，也不能为此创建一个无关 Job。

无等待者时 signal/broadcast 不保存通知。谓词仍可能在另一个竞争者拿锁后发生变化，因此调用者必须用 while；通知不是将谓词设为真的替代操作。超时可在后续有实际 API 需求时复用同一等待仲裁，不是首版条件变量的前置功能。

## 9. MultiThreadExecutor 与通用阻塞任务池

### 9.1 公共生命周期

```scoop
public class MultiThreadExecutor : CoroutineExecutor, Closeable {
    public override fun registerTask(parent: ExecutorTask?): ExecutorTask
    public fun requestShutdown()
    public suspend fun awaitTermination()
    public fun join()
    public override fun close()
}
```

由 §3.4 的 `multiThreadExecutor` 工厂创建；配置 workers 为正，构造时建立固定数量的 managed coroutine worker 和所拥有的 blocking worker。部分创建失败时关闭已建立的 worker、join 并释放原生同步对象，再抛出原错误。实例不重启。

`requestShutdown` 幂等进入 Draining，不取消已接纳的任务；取消由拥有任务的 scope/Job 发起。raw `startCoroutine` 创建而永不完成的任务可能使关闭永远等待，这是低层非结构化使用者的生命周期责任。

`awaitTermination` 可被等待者取消，但不会撤销关闭或破坏 executor；它只表示工作排空、worker 退出执行循环，不代替最终 OS thread join。`join` 要求已经请求 shutdown，完成原生 join 并回收线程句柄，可重复和并发调用；保护最终状态的同步对象保留到对象 GC，供关闭后的并发检查使用。不得从本 executor 的活动任务或 worker 调用依赖自身结束的等待/join，应立即报状态错误；普通跨 executor 的依赖循环仍由应用避免。

正式终止必须同时满足：没有活动任务登记、没有排队 coroutine work、没有正在执行的驱动，也没有本 executor 尚未退休或交付的阻塞 operation。拥有独立 blocking pool 时还须完成该池的关闭并等待 worker 退出执行循环；其 OS join 同样由 executor.join 完成。借用共享池时只等待自己的 operation，由共享池 owner 最后关闭池。单看队列为空不够，因为挂起任务仍可能需要外部 I/O、Channel 或取消来恢复。

### 9.2 无锁任务队列与登记发布

协程 worker 各有自己的无锁 FIFO。队列采用 [Michael–Scott 算法](https://www.cs.rochester.edu/research/synchronization/pseudocode/queues.html)的 managed 实现：head、tail 和 next 使用 AtomicRef，发布者帮助推进落后的 tail，消费者以 head CAS 唯一领取节点。未链接的 next 指向节点自身，等价于算法的空链；节点不复用。在途引用由 GC 保活并随搬迁更新，因此无需裸地址计数标签或 native epoch 回收。领取成功者取出并清空 payload，避免 dummy head 保留已经完成的任务；其他消费者不读取该 payload。

已评估 [Crossbeam deque](https://docs.rs/crossbeam-deque/latest/crossbeam_deque/)的 worker／stealer 模型。直接使用其 native 存储需要另加移动 GC 根管理，而现有 AtomicRef 已提供所需 CAS 与写屏障；这里使用已知队列算法，不新增 runtime ABI。每个 worker 的队列允许外部生产者和其他 worker 消费，外部恢复直接投递任务最近执行的 worker，不再先经过一个全局 injection 锁或队列。

executor 的任务登记、complete 与关闭计数由生命周期 mutex 保护。普通 dispatch 读取 ticket 的原子活动标记，再以队列链接 CAS 发布工作，不维护另一套逐投递计数。登记排空后，每条队列以相同的末端链接 CAS 写入独立终止标记；入队先赢则工作位于标记之前，关闭先赢则投递报告登记已完成。全部队列封闭后才设置 workers 的停止状态并唤醒，直到已接纳工作排空才退出。只有与 complete 重叠的投递可能这样拒绝；仍有效的登记在 Draining 中继续正常投递。blocking 的活动检查、计数与入队仍由生命周期锁协调。

消费者推进 head 后，将旧 head 的 next 写为独立终止标记，断开退休节点指向后续活动链的引用。保留旧链接会让在途引用或分代 GC 中尚未回收的旧节点延长后继对象的存活。类似的 GC 队列节点脱链动机见 [OpenJDK ConcurrentLinkedQueue](https://github.com/openjdk/jdk/blob/master/src/java.base/share/classes/java/util/concurrent/ConcurrentLinkedQueue.java)。这里不能将旧 next 恢复为自身，因为自身表示仍可入队的空末端，迟到生产者的 CAS 会误成功。终止标记不进入 head／tail；读取者先复核 head／tail 身份，持有退休节点的竞争者只重试，不能把标记当成普通工作。

worker 是 `Thread.start` 启动的长期 managed 入口。每项 work 自行进入目标 frame TaskContext 并在退出时恢复；worker 外层环境不能残留上一任务的 Job、binding 或 execution。等待路径 NativeSafe，运行路径保留普通 GC safepoint。

任务内同步阻塞会占住一个 worker，长期不挂起的计算会阻止该 worker 调度其他任务。普通纯 CPU 循环需要主动 `yieldCoroutine`，调度器不从另一个线程注入异常或中断用户指令。长期同步工作仍应进入独立 blocking pool；协程窃取不执行 blocking work。

### 9.3 Work stealing 与停车

新根轮询分配 worker，子登记继承同 executor 父登记的最近 worker；领取工作时以 Relaxed 原子序更新该归属。归属只选择有效的目标队列，旧值仍合法；payload 的发布同步来自队列 CAS，不从归属读取推导 happens-before。worker 优先消费自己的 FIFO，空闲时按轮转顺序扫描并窃取其他队列的最早工作；每处理有限批本地工作也先检查其他来源。yield 与普通恢复同样排队，不能通过私有快捷槽反复越过已经排队的工作。窃取直接执行领取的工作，不反复搬移节点或另建转运队列。

每个 worker 有独立的 parking mutex／condition 和原子 Awake／Observing／Notified 状态。准备停车时先注册观察者并发布 Observing，然后扫描全部队列；仍无工作才在自己的 mutex 下检查状态并 await。发布者先完成队列 CAS，再把一个 Observing 改为 Notified，最后在该 worker 的 mutex 下 signal；通知与等待之间因此没有遗漏窗口。已经找到工作却收到通知的 worker 会将通知转交其他观察者，避免将未消费工作的通知吞掉。关闭通知全部观察者，worker 在无可领工作且已关闭时退出。同步只用于休眠与生命周期，入队、出队和窃取不获取 mutex。

work stealing 不改变单任务串行、排队启动、Context、取消或关闭语义。业务队列始终保留 managed 根，不借用 GC 的标记队列，也不向未扫描 native 容器写入可移动引用。

公平性保证限定为不无限忽略已有队列来源；不承诺所有任务相等的运行时间。实现中的队列轮询参数是局部调度选择，不是语言可见的通用执行预算。

### 9.4 BlockingTaskPool 的归属与调度

BlockingTaskPool 位于 concurrent，是真正可以执行 managed 代码的通用线程池。普通应用通过 `blocking` 使用它；下列低层组件接口供 MultiThreadExecutor、LoopExecutor 和 AsyncRuntime 组合，不承担 coroutine frame 的恢复：

```scoop
public class BlockingTaskPool public constructor(threads: Int) : Closeable {
    public fun submit(work: BlockingWork)
    public fun requestShutdown()
    public suspend fun awaitTermination()
    public fun join()
    public override fun close()
}
```

池使用固定数量的 `Thread.start` managed 线程、FIFO 工作队列、短持有 Mutex 和 ConditionVariable。达到线程数时新工作排队，不阻塞提交者等待空闲线程，不在 coroutine worker 或 loop owner 上执行 fallback，也不偷偷无上限创建线程。排队不设置任意任务数上限；需要业务背压时由调用方的 Channel/并发组织控制，后续若增加队列容量须设计明确的可挂起接纳语义。

线程数与 coroutine workers 独立配置。阻塞线程在一个普通闭包运行期间不会被另一任务复用；coroutine work stealing 不窃取 blocking work。长期常驻服务可以使用独立 Thread；池不承诺所有 worker 都被永不返回的调用占住时还能推进后续任务。

MultiThreadExecutor 和独立 PlatformEventLoop 默认各拥有一个池。AsyncRuntime 建立一个池，同时交给 worker executor 与 LoopExecutor 使用，避免文件/DNS/TLS 和用户阻塞调用各自建立池。ExecutorBuilder/EventLoopBuilder 的 `blocking(sharedPool)` 明确选择借用，`blocking { threads = ... }` 选择自有池，内部配置是两种互斥的完整分支；借用方不能关闭共享池。AsyncRuntime 的组合使用这条真实复用路径，不另造隐藏的全局默认池。

池的 `requestShutdown` 停止接纳新 work、排空已接纳 work 后退出；它没有 Job 树，不能自行判断谁是清理子任务。所以 executor 进入 Draining 时，池必须继续接受活动登记的 work，直到相关 executor 和 I/O owner 都不再需要它，才关闭池本身。低层 `submit` 在池已停止接纳时抛 ExecutorClosedException；有效 ExecutorTask 的 dispatchBlocking 通过正确的所有权顺序保证不遇到这条拒绝路径。

### 9.5 `blocking` 的执行与 Context

`blocking(block)` 选择当前 coroutine executor 的池；没有当前 executor 则在提交前报状态错误，不创建隐式线程池。`executor.blocking(block)` 显式选择目标的池，可以由其他 executor 的 coroutine 调用；结果始终交回等待者自己的 execution。

每次调用创建类型化 operation：普通 `() -> T` 闭包、调用时的 binding snapshot、Job 取消登记、成功 `T` 或 Throwable、目标执行登记，以及等待者 continuation。T 可以是 class、struct、enum、tuple 或包含引用的泛型值，不限制为 GC-free。已接纳后不在调用栈内执行闭包，其他 blocking worker 可以在提交函数返回前开始。

执行时从提交快照 fork 新 TaskContext，保留 Job、EventLoop 和其他 binding 的对象身份，但不共享 caller 的可变绑定根，不继承 caller 的 CoroutineExecution。此处是普通阻塞函数执行，`currentCoroutineExecutorOrNone()` 返回 None；不能因为它使用某 executor 的池，就冒充正在其 coroutine worker/owner 上运行。

捕获的 CoroutineScope 仍带默认 executor，因此闭包可以通过仍开放的 scope 排队启动子任务；这些子任务按 scope 的真实 Job 关系管理，不自动转到 blocking worker。普通 block 不能直接调用 suspend 函数，也不提供在阻塞线程内部递归运行异步 event loop 的隐藏桥接。

可以直接复用现有 `foreignCallback` 的 binding snapshot 与 managed gateway：提交侧创建捕获 typed operation 的 OneShot callback，blocking worker 在取得运行权后通过完整 function/context 调用它；已登记线程不重复 attach，callback fork 不带 execution。包装器捕获所有普通 Throwable，把结果保存在 managed operation，再正常返回 C gateway；callback 内不恢复等待者。返回到 BlockingWork runner、完成 callback 消费和 Context 恢复、确认实际收尾后，才发布结果通知。排队取消时撤销尚未调用的 callback 并清空捕获；运行权已取得时由 invocation 完成消费。不得只抽出 trampoline 丢掉 context，也不得让异常跨 C frame 展开。

这条复用路径不要求新增公开 Context snapshot API 或第二套 callback/runtime 根系统。以后若测得每次任务的 gateway 成本明显，可以把同一边界生成为直接 managed runner；其 typed Context、根、异常和执行属性合同不变，不能为优化删掉这些边界。

### 9.6 开始、取消、退休与结果交付

通用闭包可能返回文件句柄或修改外部状态。它开始以后，库既无法回滚副作用，也不知道如何销毁任意 T。因此本设计明确区分“撤销尚未开始的调用”和“让正在执行的函数协作停止”：

| 阶段/竞争 | `blocking` 的结果 |
| --- | --- |
| 进入前 Job 已取消 | 不提交、不执行 block，抛取消 |
| `Queued -> Cancelled` 先于 worker claim | block 不执行；清除捕获/取消登记，在原 executor 交付取消 |
| worker 的 `Queued -> Running` 先胜 | block 最多执行一次；caller 等待它及其 finally 真正结束 |
| 运行中 Job 被取消 | Job 的取消状态可由 block 的 `ensureActive()` 观察；不杀线程、不提前恢复 caller、不自动丢弃返回值 |
| block 正常返回 | 提交实际 T；即使运行期间收到取消，仍把 T 交给 caller，以便其安装清理/取得资源 |
| block 抛出 | 提交该 Throwable，包括 block 自己观察到的 CancellationException；执行完清理后在 caller 挂起点抛出 |
| 结果已发布但 caller 尚未恢复 | 原结果不被后来的取消改写，仍回原 executor |

`Queued -> Running/Cancelled` 在同一个 operation 状态锁或 CAS 上仲裁。worker 已从池队列取出，但尚未取得 Running 时，取消仍可以获胜。已取消的队列壳可以在出队时跳过；必须立即释放闭包、callback、binding 和 continuation 等不再需要的引用，不能让排在长阻塞操作后面的空节点长期保活整个任务。

运行中取消只停止等待的方案不采用：它会让 caller 在 finally 释放仍被 block 使用的资源，也会使 executor 误判可以销毁 runtime。等待也不使用自动抢先完成的 CancellableContinuation；复用原始挂起与同一 operation 仲裁。成功/异常、真实退休和已交付分别记录，任务登记直到真实退休、通知发布和必要清理结束才释放。

这不是保证整个父 Job 最终成功：父 Job 仍可能已取消，caller 的下一个取消检查以及父作用域的最终结果继续遵守 Job 规则。保证的是本次普通阻塞调用不会在后台继续访问已经离开调用方作用域的资源，也不会在成功返回 T 时擅自丢弃 T。

需要更积极取消的具体库，可以使用其真实原生取消 API或让 block 定期 ensureActive；停止请求本身不等于停止完成。仍须等待实际调用退出。没有通用线程强杀、JVM interruption 模拟或关闭超时后遗留 managed 线程的模式。

普通异步组合直接写 `async { blocking { operation() } }`，由现有 Deferred/Job 管理；首版不再增加一套 BlockingJob、独立 future 或只改变名字的 spawnBlocking/asyncBlocking 同义入口。

### 9.7 GC、FFI 与借用

blocking worker 在运行 Scoop 闭包时是正常 mutator：分配、异常、闭包调用、精确 roots、写屏障与 safepoint 都与普通 managed Thread 相同。池队列、工作对象和结果沿普通扫描保活；不能把 managed closure 指针直接交给未登记的原生 worker。

“允许阻塞”不是把整段闭包标成 NativeSafe。只有实际阻塞 FFI/平台同步调用经既有边界进入 NativeSafe，返回时完成握手、重读可能移动的引用后再运行 managed 代码。调用需要 managed callback 的 C 库时继续使用完整 foreignCallback 注册协议；普通 C ABI 的参数和返回类型仍必须 GC-free。

GC-free native 代码也不等于 GCLeaf：可能阻塞的 syscall、DNS、系统证书装载和长时计算必须使用正确的 NativeSafe 边界，不能为少一次切换而标 GCLeaf。真正的短小 leaf 调用遵守原合同。

`NativeByteBorrow`、pin 和临时裸指针仍有原来的作用域。不得把 caller 的栈 pin frame 或同步 `withDataPtr` 借出的指针捕获到异线程工作里；需要数据时捕获 managed owner，并在 worker 内建立和结束同步借用，或使用已有稳定 native 存储及其实际借用协议。池不会把任意逃逸 pointer 自动变为安全。

池锁不跨用户闭包、FFI、callback、结果转换或 continuation 通知。pool 空闲等待和 join 复用平台 NativeSafe 实现；线程结束前退出全部 managed/native frame，归还 callback 与任务引用，最后由线程 attachment owner detach。

### 9.8 现有 native work 的直接迁移

迁移目标是一个通用 managed blocking pool；不保留三平台旧工作池作为兼容后备。现有真正异步的 io_uring、IOCP、dispatch source/timer 仍走各自 backend，OS 通知回调也继续只生产完成记录。

| 现有工作 | 新执行路径 |
| --- | --- |
| 文件、目录、路径、DNS 的阻塞部分 | managed operation 在 owner 上建立/登记；BlockingWork 调用已有或拆出的同步 platform/native helper；owner 消费完成并处理资源/借用 |
| TLS 系统信任装载 | 普通 blocking 闭包调用同步装载，构造类型化托管结果；工作参数与未交付 native 结果在明确 finally 中释放 |
| TLS 验证状态机 | 从 engine 取出独立请求，在共享池执行；回 owner 后检查 closing、应用结果、推进 engine 并释放请求 |
| 任意应用同步库 | `blocking { ... }` 直接捕获 managed 参数并返回 T，无需 NativeWork payload 模板 |

I/O 适配的取消仍由该操作自己的选择状态决定：取消可以先选为最终失败，但必须等工作退出、提交实际进度、解除借用和关闭迟到资源后才恢复。这种带 native 退休责任的适配使用同一池的 BlockingWork 协议，不能无条件套用通用 blocking 的“开始后交付实际返回值”并改变 I/O 合同。反之，通用池也不能对所有 T 套用 I/O 的丢弃/清理规则。

具体改动包括退役 `EventLoop.runNativeWork`、公开 NativeWork 资源及仅为它存在的 execute/release 指针工厂；TLS 等消费者直接使用同步 FFI 与 typed managed 工作对象。C 中现有 execute/finish 逻辑保留必要的系统调用、错误提取和释放，但拆除 `scoop_io_work_submit/cancel` 的平台线程调度职责，仅保留原生登记、owner 提取与取消仲裁。每份已投递 BlockingWork 持有独立 native task：Queued 取消可切断 payload 并立即发布取消完成，task 实际出队后再退休；native idle 同时计入尚未退休的 task，避免阻塞池满时取消等待空槽或访问已销毁的 loop。原本由 native worker 提交的 finished 记录改由受控完成入口交给 owner，不能从 worker 修改 owner-only 容器。

关闭行为需要把两种池都算入：先让活动 coroutine、阻塞 operation、I/O 退休和清理完成，再关闭阻塞池；join 不得发生在该池自己的 worker，也不得发生在仍需推进完成消息的 owner 回调中。AsyncRuntime 的共享池直到 worker executor 与 loop 清理均不再提交工作后才停止接纳。

## 10. EventLoop 与多线程 I/O runtime

### 10.1 LoopExecutor

`PlatformEventLoop.run/attach` 为用户根任务选择 LoopExecutor。之后不只内置 I/O，Channel、Mutex、第三方 callback、普通 `suspendCoroutine` 的真实挂起恢复也都回 owner 线程。

`EventLoop` 增加只读 `executor: CoroutineExecutor`，在运行或附着已建立、尚未终止时返回该 owner executor；运行前和终止后访问报状态错误。LoopExecutor 的具体类保持内部实现。worker 需要显式执行 owner 代码时可使用 `withExecutor(currentEventLoop().executor)`；普通 I/O 调用自身已经负责把 backend 操作提交到 owner，不要求用户先切换。

LoopExecutor 使用独立的内部 work 队列或现有内部命令通道，并复用 backend wake。它必须遵守任务登记与 draining 合同；不能调用关闭时允许丢弃应用消息的 `EventLoopSession.post` 来调度 continuation。

loop 每轮处理进入本轮时取得的有限快照；work 执行产生的新命令留到后续轮。只要命令、完成或 runnable work 尚可推进，就不能阻塞或等待另一次外部事件。新 work 必须唤醒 run 和 attach 两种驱动方式。

默认 GUI/宿主入口继续在 owner 上运行。显式 `withExecutor(pool)` 才离开 owner，返回时回到原 LoopExecutor。UI 对象与 host API 的线程要求仍由调用者遵守。

### 10.2 Worker 可以使用 I/O，但不接管后端

`withExecutor` 和显式 executor 的 launch 继承 EventLoop binding，所以 worker 可以发起属于同一 loop 的 I/O。EventLoop 是资源归属与服务入口，不再被解释成“当前正在 owner 线程”。

必须逐项落实以下边界：

1. worker 只建立请求、取得必要借用、发布跨线程命令；native submit、operation/resource 容器更新和完成消费仍由 owner 执行。
2. owner 先选择结果、提交真实进度、等待 native 退休并解除借用，再调用 continuation；continuation 负责回调用者 executor。
3. I/O 的内部 ready action、关闭 callback 与资源清理仍明确归 owner，不因用户任务在 worker 上运行而把 owner-only 状态带到 worker。
4. public 资源对象的 phase/busy/active、并发 close 和操作登记必须在正确锁或 owner 命令中访问；仅检查 EventLoop identity 不足以证明线程安全。
5. 会创建 I/O 资源的请求在结果交付前取消，继续先清理未交付资源；不能为尽快恢复 worker 而跳过退休。

`LoopOwner.requireJob`、resource busy 状态、mailbox、DNS/file/native work、socket、timer、TTY 都在审计范围。HTTP/TLS/服务器等实际消费者也要检查是否依赖“同一 loop 意味着所有用户代码串行”的旧假设；必要的串行对象使用 AsyncMutex 或保留 LoopExecutor 运行位置。

### 10.3 AsyncRuntime

```scoop
public class AsyncRuntime {
    public fun <T> run(block: suspend CoroutineScope.() -> T): T
}
```

`AsyncRuntime` 位于 asyncio，由 §3.4 的 builder 创建，组合一个 PlatformEventLoop、一个 MultiThreadExecutor 和双方共享的 BlockingTaskPool。调用 run 的线程驱动 I/O owner，用户根及默认子任务在 coroutine worker pool 上执行，blocking 闭包进入独立的 blocking workers。实例单次使用，根入口拒绝已有活动 Job/EventLoop；类型化 run 在子树和全部清理完成后直接返回 T，不增加 runWithResult 同义入口。

这里暂不使多个 worker 并发 poll 同一个 backend：现有资源容器、io_uring/IOCP/宿主 watch 的 owner 合同要求串行驱动。将来即使改变 backend driver 分配，也不改变 Channel、Mutex 和协程调度 ABI。

关闭按依赖顺序推进：

1. 关闭 runtime/session 的应用入口；失败或外部关闭请求取消根 Job，正常根结束则保留正常结果。独立持有 executor 的低层调用仍按该 executor 实际进入 draining 的时机判断登记，不承诺一条关闭请求同时关闭所有相互依赖的 executor。
2. 请求 worker executor draining，继续驱动 I/O 并保持共享 blocking pool 接纳工作。已接纳任务仍可恢复、运行 finally、建立同 executor 的清理子任务或 blocking operation；根及其子树必须真实结束。
3. 等待 worker executor 的全部执行登记、coroutine work、正在执行的驱动以及它提交的 blocking operation 排空。不能只等结构化根，因为低层入口可能还有未结束登记。
4. worker executor 已完全结束后，不再有其任务需要新建 owner 清理切换，此时使 LoopExecutor draining，排空它已接纳的任务与工作。随后以 owner 内部命令／ready 关闭遗留 I/O 资源，排空 native 操作、取消和资源清理；这些库内收尾不新建用户 coroutine 登记，文件关闭等仍可使用共享 blocking pool。
5. 两个 executor、资源和 owner 清理均不再产生 blocking work 后，才使共享 blocking pool 停止接纳；排空所有已接纳 work，等待运行中的普通函数真实返回及线程退出。
6. 所有结果通知都已消费、owner 内部队列和 backend 排空后，移除宿主 watch/关闭 native loop；join coroutine/blocking worker 并回收线程句柄，状态同步对象按 §9.1 保留到 GC，最后返回结果或异常。

I/O owner 在阻塞工作实际退休和结果交付前必须继续推进，LoopExecutor 也要允许已接纳 worker 的必要清理切换；过早停止调度、blocking pool 或 I/O 任一侧都会造成永久等待。手工组合多个 executor 时，调用者同样要保持清理依赖的目标 executor 存活；向已经 draining 的另一 executor 新建根登记会正常失败，不自动“复活”它。

`attach` 的关闭完成通知必须发生在全部清理之后，host 此前仍须提供驱动机会。关闭请求本身不在 GUI owner 上等待仍在运行的 blocking 任务；完成通知驱动后续收尾，只对已退出线程执行最终句柄回收。未完成 coroutine 和 blocking completion 都不是可丢弃的应用 post。

### 10.4 两个 executor 之间的用法

以下示例使用拟议 API：生产者在 pool，消费者在 LoopExecutor，同一个 Channel 负责消息和背压。

```scoop
multiThreadExecutor {
    workers = 4
    blocking { threads = 8 }
}.use { pool ->
    eventLoop().run {
        val messages = Channel<Int>(16L)
        launch(executor = pool) {
            try {
                for (value in 0..<100) {
                    messages.send(value)
                }
            } finally {
                messages.close()
            }
        }
        var sum = 0
        var receiving = true
        while (receiving) {
            when (messages.receiveOrNone()) {
                case Some(number) -> sum += number
                case None -> receiving = false
            }
        }
        println(sum)
    }
}
```

消息匹配可以发生在任一方线程；真正继续执行的生产者和消费者分别回自己的 executor。示例中所有工作均由 root scope 管理，run 返回时该子树已经结束。

## 11. 编译器、产物与实现改动

| 范围 | 必须完成的变更 |
| --- | --- |
| AST/HIR lambda | 区分隐式/显式效果来源，完成期望类型推导、候选隔离、effect 对称重载优先级与普通边界诊断 |
| HIR core protocol | 识别实际 core 的 executor/work/登记协议与新 intrinsic，保存 selected declaration、typed identity 和派发槽；增加代表性签名错误诊断 |
| MIR 协程入口 | 建立 execution 与任务登记；立即/排队启动共用完成包装；覆盖登记失败和完成回调异常出口 |
| MIR frame/adapter | 精确 payload、execution 引用、typed 状态与 action；统一原语和 direct/virtual/closure 挂起握手 |
| MIR driver | 全驱动串行门、真正退出后发布 work、内部完成链迭代、强制 yield |
| TaskContext | 扩展 generated storage 和 fork 模式；新增读取当前 execution 的内部操作；明确 callback 只继承 binding、任务入口建立 execution |
| LIR/codegen | 原子状态发布、GC root/scan/写屏障、接口 work 调用、所有正常和异常的 Context 恢复 |
| IR/meta/slib | 保存新增 protocol role、字段 role、generated callable identity 与引用关系；受影响的 schema/profile/ABI 同步演进 |
| runtime | Context 实际布局与后备路径、GC 扫描、未捕获异步边界异常处理；复用线程登记、callback gateway 与 NativeSafe 阻塞/返回握手 |
| core 库 | receiver scope 与默认 executor、withExecutor、blocking operation、登记释放、取消与结构化失败边界 |
| concurrent/asyncio | 配置 builder、两类工作队列、共享阻塞池、loop 内部 work 通道与依赖顺序关闭 |
| native/实际消费者 | 拆除三平台旧 blocking pool，文件/DNS/TLS 迁移，保留 I/O 借用/退休/错误与 owner 合同 |

涉及的主要代码目录是 [AST 表达式](../../compiler/ast/src/expressions.rs)、[HIR 重载决议](../../compiler/hir-lower/src/overload/)、[MIR coroutine lowering](../../compiler/mir-lower/src/coroutine/)、[coroutine registry](../../compiler/mir-lower/src/coroutine_registry/)、[MIR Context](../../compiler/mir/src/context.rs)、[HIR protocol](../../compiler/hir/src/entities/core_protocols.rs) 与各自 LIR/codegen/metadata 消费方。

stage 仍只通过 IR crate 通信。协议实体、生成 adapter/work、字段和 resume point 使用不同的类型化身份，不从 FQN 猜测 executor、不把后端补查字段当作结构完备性的替代。TaskContext 的 execution 字段引用所选实际 core 内部类型，字段描述必须能表达该类型；不能为满足旧的 Context 字段枚举而复制另一套 execution 对象或擦除为无类型裸指针。

保留现有源码 suspend 调用的 `CoroutineStep<T> = Completed(T) | Suspended` 合同；execution/action 是其驱动实现，不新增用户可见 Poll 类型。必要的内部布局和产物变化直接升级并重建 core、系统库及消费者，不提供旧新 adapter 双轨或旧 ABI 转发层。

tail-suspend、内联和优化只有在保留 execution、待恢复 Context、异常/finally、任务登记与强制 yield 边界时才能生效。不能仅因最终返回类型相同就消去 dispatch 或生命周期操作；普通内存效果也不能跨原子发布和 Context enter/leave 重排。

## 12. 规范同步与实施顺序

### 12.1 先修订的规范

| 文档 | 对应部分 |
| --- | --- |
| [语言规范](../specs/SCOOP-SPEC.md) | §8.1、§8.2、§8.6：lambda 期望效果与重载；§8.3、§11.9：执行归属、callback/fork、receiver scope、blocking、启动/恢复/yield；§11.18 及引用处同步省略 suspend 的新规则 |
| [运行时规范](../specs/SCOOP-RUNTIME-SPEC.md) | §8～§9：execution 驱动、Context 布局与根；§3.5 和 FFI 边界的 blocking mutator/NativeSafe 合同，线程退出及异常交付 |
| [实现规范](../specs/SCOOP-IMPL-SPEC.md) | §2.1～§2.6、§2.10～§2.13、§2.16：lambda 来源与效果定型、状态、generated identity、GC、core 协议、meta 与 Context |
| [平台规范](../specs/SCOOP-PLATFORM-SPEC.md) | managed worker 与 NativeSafe 等待的实际复用；只有发生平台 API 变更才新增条款 |
| [异步 I/O 规范](../specs/SCOOP-ASYNCIO-SPEC.md) | §1～§3、§9.3、§11：类型化根入口、executor 归属、worker 调用、NativeWork 退役、共享池、不可丢投递与关闭顺序；TLS 规范与调用方同步 |
| 新增 concurrent 库规范 | Channel、AsyncMutex、Condition、executor builder、BlockingTaskPool 的 API、线性化与生命周期合同 |
| [路线图](../ROADMAP.md)、[backend 设计](../asyncio-backends/DESIGN.md) | 记录本次纵向能力与新旧 owner/恢复合同边界，避免同时宣称“无通用 executor”和“所有恢复回 owner” |

正文与引用方同时更新，章节变化时同步引用。当前文档阶段不把尚未实现的内容写成现有能力，也不为未发布版本维护兼容别名。

### 12.2 纵向落地

1. **语言与调度合同闭环**：修订 core/compiler/runtime 规范，完成 lambda 挂起性推导、重载和 receiver scope；实现新的 TaskContext、执行登记、挂起握手、驱动门、排队启动与 yield。贯通跨 Cone 编译与运行。
2. **普通并发库闭环**：增加 concurrent Cone；实现 Channel、Mutex、Condition，完成共同等待节点协议和独立行为测试。先在同 executor 验证，再在不同 managed 线程/执行器之间验证同一 API。
3. **两类 worker 闭环**：实现 MultiThreadExecutor、BlockingTaskPool、blocking 普通闭包/typed 结果、park/wake 和完整关闭；覆盖并行、取消、NativeSafe 与 GC，并交付配置 builder。
4. **I/O 与旧池迁移闭环**：LoopExecutor 覆盖 run/attach，完成 worker I/O 和文件/DNS/TLS 迁移，拆除三平台旧 work pool 与 NativeWork API；实现共享池 AsyncRuntime，验证 backend 与两类 worker 的关闭。
5. **性能评估与优化**：记录单线程开销、跨线程消息延迟、锁竞争、runnable 吞吐、阻塞提交/gateway 成本与空闲 CPU；确有瓶颈时按 §9.3 替换调度队列，不改前面已定的源码/编译器合同。

这是一套目标设计的实施分解。前三步不能用“已有线程池”替代第四步的 I/O 正确性；第五步也不能反过来成为 Channel 和同步功能无限延期的前置项。

## 13. 验收

### 13.1 编译器与 core

- 期望普通/挂起/receiver 函数类型、无期望默认、显式 suspend/普通匿名函数、已定型值与 callable reference；不能靠重载探测改变另一候选的 lambda 效果。
- use/withLock 的 effect 对称重载：纯正文优先 ordinary、有挂起调用只剩 suspend、多个 callback 的不可比较效果保持歧义；泛型/bounds/receiver/返回型不同不误用缺省规则。
- DSL 配置和执行作用域、嵌套 child receiver、外层 scope 显式引用、scope 从 foreign callback/blocking worker 发起子任务的默认 executor；普通回调/构造/NoGC/FFI 中越界挂起的定位诊断。
- 原语 registration 同步成功、同步失败、返回前异线程完成、返回后完成、完成后抛出、重复完成；断言一次正文驱动与一次通知。
- direct、virtual、closure 多层 suspend 组合；在 callee 返回 Suspended 前后制造完成竞争，检查不并发进入父 frame。
- start、launch、withExecutor 正常/失败/取消；其他 worker 可以在提交函数返回前执行，但不得在提交者调用栈内 inline。
- 单任务活跃驱动计数始终不超过 1；不同任务在阻塞屏障两侧确实能同时占用多个 worker。
- 强制 yield 不被同步锁存消除；内部深层完成链不产生与链长成比例的额外递归 resume 栈。
- Context 词法绑定、接口精确 key、fork、finally、callback snapshot、迁移与 resumer 恢复；普通 `context(executor)` 不触发调度。
- queued success 后取消仍交付成功；取消异常进入正确 catch/finally；非取消清理失败保持既有传播规则。
- 跨 `.slib`、单态化泛型、接口和函数值、tail-suspend、内联与必要 profile 组合；坏 core 协议给出定位清晰的编译错误。

### 13.2 Channel 与同步对象

| 对象 | 必须覆盖的竞争与行为 |
| --- | --- |
| Channel | 容量 0/1/N、MPMC、背压、FIFO、try 操作、关闭排空、关闭时等待者、可空元素 |
| Channel 取消 | send/receive 取消先胜与匹配先胜；匹配双方均仅完成一次；同步取消发生在订阅对象返回前 |
| AsyncMutex | FIFO、tryLock 不插队、跨挂起持有、迁移后解锁、handoff 与取消竞争、withLock 异常释放 |
| Condition | 登记与解锁原子性、signal/broadcast、错误 guard、通知与取消竞争、取消抛出前已重新持锁 |
| 共用协议 | handler dispose 竞争、无锁内恢复/用户回调、终态节点及时摘除、executor draining 时仍可交付 |

普通行为放 concurrent Cone tests；使用真实队列和屏障控制关键时序，不能只靠重复 sleep 等待“碰巧覆盖”。异步失败都要保证测试本身能收尾；外部超时仅用于检测死锁，不作为正确性的证据。

### 13.3 Runtime 与 I/O

- 一个 LoopExecutor、多 worker、两个独立 executor、owner 与 worker 混合；每次真实恢复检查属于预期执行位置。
- 取消期间 Channel/Mutex/Condition 清理继续得到调度；shutdown 与最后一次 dispatch、子登记、completion 同时发生时不丢任务。
- workers 空闲时能够 park，enqueue/终止唤醒无遗漏；关闭拒绝新根但接纳已有登记的合法后代；禁止自身 join。
- timer、socket、file、DNS、TLS、TTY 的实际 I/O；旧 NativeWork 消费者迁移后，取消时借用解除、真实进度和迟到资源清理仍完成。
- run/attach 都覆盖第三方挂起从外线程完成、work 唤醒、根失败、未关闭资源和退出清理；应用 post 可丢不影响内部 continuation。
- 构造部分失败、重复关闭、raw 未完成任务的关闭等待、host 继续驱动至关闭完成；最后验证无遗留 managed worker 或 native 操作。
- GC 定向压力覆盖队列中的 payload、等待节点、frame、Context、异常、迁移与取消摘除；普通全量测试不默认启用 GC stress。

### 13.4 通用阻塞池与自然 API

- `blocking` 捕获托管对象、分配、调用同步库、返回引用/值类型/泛型结果或异常；结果回原 executor，且不在提交栈内运行正文。
- 排队取消与 worker claim 两种胜负、运行中取消后的真实返回或协作抛出、callback 单次消费/撤销；不提前释放借用，不丢弃成功返回的资源。
- 多个工作期间的 Context fork/恢复、不同 Job/日志绑定不串台；worker 内 current coroutine executor 为空，捕获 scope 启动的 child 仍去 scope 默认 executor。
- 一组 blocking worker 正在等待时，coroutine worker 和 loop 仍可推进；所有 blocking worker 忙碌时后续工作排队，数量不偷偷增长。
- 长 NativeSafe 阻塞期间其他线程能完成移动 GC；FFI 返回后 managed 引用已重定位，嵌套 callback 正常；GCLeaf 误用不通过池包装被默许。
- 自有与共享池关闭、构造部分失败、运行中工作不能提前退休、清理继续提交 blocking、callback 资源最终释放、最后 join 后无残余线程；复用线程不丢工作且异常不会杀掉单个 worker。
- 完整编译运行 §3.4/§10.4 示例与 mutex/condition 示例；验证 builder 快照、无效参数在创建线程前失败、typed run 返回值和异常/清理结果。不仅检查 API 声明能解析。

普通 API 行为优先用所属 Cone tests；编译诊断、ABI/产物、必须由外部进程观察的退出和死锁才使用 fixture。实现批次先格式化和 lint，再按影响范围测试；原始并发日志与性能数据放仓库外，验收文档只保留结论和限制。

## 14. 本设计的边界

本次编译器调整一次完成 lambda 挂起性的期望推导与重载、执行归属、驱动串行化、所有 suspend 调用形态的握手、Context/callback 边界、排队启动、真正 yield、完成清理及跨产物表达。通用 managed blocking pool 与 DSL API 属于本次交付目标；Channel、Mutex、Condition、阻塞工作调度和 work stealing 不各自增加编译器特判。

`select`、广播/watch Channel、公开 Notify/Semaphore、时间超时组合、Send/Sync、抢占或任意线程强杀和无锁 work-stealing deque 都不作为首版交付条件。它们需要实际使用场景和独立 API 合同；当前设计保留普通库的扩展空间，不预置只有假想调用者的框架。
