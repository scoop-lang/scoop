# 桌面窗口与事件库设计

后续的原生菜单栏、popup menu、可定制窗框、窗口归属／模态、系统文件对话框，以及独立 scoop.tray Cone 和 UI 包装见 [原生桌面能力与 UI 组件设计](../desktop-ui/DESIGN.md)，关系与对话框细节见[配套设计](../desktop-ui/WINDOWS-AND-DIALOGS.md)。设计已纳入当前 Linux 后端，并安排最后的综合 demo；新增能力尚未实施。

后续控件基础所需的公共键位、编辑命令、剪贴板与 IME 合同整理见 [跨平台抽象设计](../ui/component-foundations/PLATFORM.md)。该轮设计于 2026-11-04 完成，尚未实施；当前先做 macOS，Windows 分支随后满足同一公共合同，上层 UI 不另写平台代码。

状态：基础架构、macOS／Windows GUI 骨架及 Linux GNU x86_64 的 GTK4／libadwaita Wayland／X11 后端已实现。更新：2026-11-07，同步 Linux 合入后的设计基线；本次仅更新文档。Linux 采用 GtkGLArea／Skia 呈现，musl 暂无 GUI。实现、验证与限制见 [PROGRESS.md](PROGRESS.md)及 [Linux GUI 实施记录](../linux-gui/PROGRESS.md)，使用方式见[窗口库 README](../../sysroot/src/scoop.window/README.md)。

本轮按 winit 的职责搭建可供 GUI 使用的骨架：真实窗口／主循环、显式生命周期与 proxy、基础窗口属性、DPI／显示器快照、键盘／指针／滚轮／文本输入、native lease、renderer 和 asyncio 接入。winit 的完整能力地图用于确定边界和后续扩展方向，不要求本轮复制它的全部平台扩展。全屏控制／显示模式切换、全局原始设备监听、触摸／高级手势、光标捕获、文件拖放及平台专有的复杂窗口交互分批补充；未实施项不以 Unsupported 伪装成平台不支持。控件、布局、菜单体系和应用编辑器仍由上层 GUI 实现。

目标是为 Skia／HarfBuzz 上层 GUI 提供 winit 对应的窗口、输入、显示器与主循环能力。公共 API 使用 Scoop，平台实现使用成熟的系统／协议库。功能基线取 [winit 0.30.13](https://docs.rs/winit/0.30.13/winit/)，另参考 0.31 beta 的演进方向，不追随其未稳定 API；逐项范围见 [WINIT-MAPPING.md](WINIT-MAPPING.md)，目标合同见[窗口库规范草案](../specs/SCOOP-WINDOW-SPEC.md)。

Scoop 尚未发布任何版本。语言、库 API、runtime ABI、native 产物组织和构建流程均可按实际需求调整；不为旧草案保留兼容别名、转发层、双轨实现或预先冻结未来平台的参数与事件。每次变更先更新规范，再同步实现、调用方和必要重建。同一次构建的类型、格式、目标与 ABI 相容性仍是正确性要求。取消历史兼容负担不扩大本次工作范围，跨流水线的新能力仍须独立设计。

## 1. 已落地 async backend 的接入结论

本设计核对主线源码，实施期间已合入 Windows 基础能力的 [b0de037002](https://github.com/scoop-lang/scoop/tree/b0de037002)。下表区分已有机制与窗口层的责任：

| 已有实现 | 可复用的事实 | 窗口层的责任 |
| --- | --- | --- |
| [PlatformEventLoop](../../sysroot/src/scoop.asyncio/src/platform_event_loop.scoop) | 单次 run／attach；attach 捕获 Context；初始化失败回滚 | 从主线程、当前 Job 外安装，不在窗口事件内再启动 loop |
| [LoopDriver](../../sysroot/src/scoop.asyncio/src/loop_drive.scoop) | 有限批次、非阻塞 pump、阻止递归进入 | 宿主保留原生输入和重绘的执行机会 |
| [LoopMailbox](../../sysroot/src/scoop.asyncio/src/loop_mailbox.scoop) | post 经根 continuation 执行；取消交给 owner | 直接复用任务、Context 和取消规则，不另建 GUI Job |
| [EventLoopSession](../../sysroot/src/scoop.asyncio/src/event_loop_session.scoop) | I/O 排空后注销 watch，再销毁并通知 onClosed | 在 onClosed 之前维持原生主循环 |
| [EventLoopHost](../../sysroot/src/scoop.asyncio/src/host/host.scoop) | 已迁入 Windows 分支的 EventLoopNotification 与 watch；remove 等 ready 退出 | 提供真实窗口宿主适配 |
| [MainDispatchHost](../../sysroot/src/scoop.asyncio/platform/darwin/main_dispatch_host.scoop) | 主队列通知及 ForeignCallback 保活／异常观察 | 继续验证菜单跟踪、拖动和模态，不把 CFRunLoop 测试当成完整 AppKit 验收 |

[实施记录](../asyncio-backends/PROGRESS.md)已报告 Darwin／GNU／musl、Linux 6.1、取消竞争和定向 moving GC 验收。本次复核 Darwin 常规 release 测试及真实 AppKit 模态／菜单 tracking，未重跑 Linux 或完整 GC stress 专测。[性能记录](../asyncio-backends/PERFORMANCE.md)是窗口接入前的基线；本阶段不预设新窗口层具有性能优势。

当前工作是窗口系统集成。主线已实现 IOCP、manual-reset event 通知及对应 attach 路径；窗口集成直接采用这组接口，不再另建 wake-source enum 或旧宿主兼容层。接口、实现和实际调用方已同步，见第 4 节。平台基础的 I/O 验收不等于对应窗口后端已经完成。

## 2. 分层与交付边界

新增两个普通 Cone：

- `scoop.window`：依赖 core／std／platform，提供应用主循环、窗口、事件、显示器、输入和受控 native 句柄借用；不依赖 asyncio、Skia 或 HarfBuzz。
- `scoop.window.asyncio`：依赖 window／asyncio，把已有 PlatformEventLoop 附着到窗口宿主，协调应用退出与 I/O 收尾。没有第二套 Job、cancel、join 或 native I/O 实现。

选择“移植能力与平台语义”，不把 Rust ABI 或 winit 类型放进 Scoop API。原生层分别使用 AppKit、Win32／TSF、libwayland-client／wayland-protocols／libxkbcommon／libdecor、Xlib／XInput2／XRandR／Xcursor／XKB／XIM。引用或改写 winit 的键映射、算法和平台规则时保留相应许可证与来源。窗口协议、键盘映射、输入法与装饰优先复用这些成熟库，不自行实现 wire protocol 或完整输入法。

本期采用上述原生后端，不新增独立的 winit Rust staticlib。现有 [text backend](../../sysroot/native/text_backend/Cargo.toml) 已包含 Rust 标准库及静态依赖；第二个独立 staticlib 会重复打包这些部分，并可能产生符号和运行库链接冲突。最终程序的按需抽取／裁剪不能作为解决多 staticlib 组合的合同。[Rust 链接规范](https://doc.rust-lang.org/reference/linkage.html#mixed-rust-and-foreign-codebases)给出的组合方式是先将 Rust 库汇总到一个 staticlib，再交给外部链接器。

把 Rust 依赖作为 Cone 可独立声明的正式能力，再为最终程序合成一个 Rust staticlib，涉及四项独立工作：Cone 的 Rust 依赖声明；PM／driver 汇总实际依赖并生成顶层 crate；Cargo 构建及依赖更新检测；在最终链接前完成统一构建并交付 native 输入。更新检测还需明确与 Cargo 缓存、lockfile、指纹的边界，不能未经设计依赖其内部格式。现有 text backend 的固定维护构建不等于具备这套全项目能力。手工把两个固定后端并入一个归档也不解决普通 Cone 的独立声明和按需系统库依赖问题。

项目未发布，允许以后调整这些边界；当前窗口任务不承担这项跨流水线改造，也不增加项目级 Cargo 合成阶段。原生路线直接满足宿主监听注销、文本快照／编辑批次和退出顺序的控制需求，同时复用成熟系统／协议库。winit 继续作为能力和平台规则的参考，不维护第二套并行窗口实现。

共享 `ApplicationOwner`／`WindowOwner` 持有事件与业务投递队列、typed id、窗口 phase、最后已公布的 metrics、dirty 状态和 native lease 数；native 后端持有真正的 NSWindow／HWND／wl_surface／X Window、协议配置与稳定事件存储。内部 C ABI 提供 create application、run／stop／wake、drain events、window control／snapshot、通知监听注册／注销、native handle acquire／release 与 teardown 等实际入口。跨 Cone 的窗口／asyncio 连接通过第 4 节列出的公开 native 包接口完成，不访问另一 Cone 的 internal owner。原生主循环由各平台驱动，共享层不能假定每个平台都有可反复调用的 poll；平台状态不再复制一份 Job 或 managed 资源表。

Widgets、布局、状态重组、字体排版、剪贴板、菜单模型、文件对话框、无障碍树、绘制 API 和通用手势识别属于后续 GUI／平台库。窗口系统原生提供的触摸和触控板手势属于本库；应用自行识别的拖拽／长按等不在本库。移动端、Web、向已有外部窗口嵌入和任意宿主轮询入口不在首期实施范围。

## 3. 公共对象与运行入口

采用 `Application` 表示窗口系统主循环，避免与 I/O 的 `EventLoop` 同名。以下为主要接口分组，相关值类型与事件字段见窗口规范和映射表；当前交付与测试见实施记录。

```scoop
public class Application public constructor(options: ApplicationOptions = ApplicationOptions()) {
    public fun run(handler: ApplicationHandler)
    public fun proxy(): ApplicationProxy
}

public interface ApplicationHandler {
    public fun onEvent(application: ActiveApplication, event: ApplicationEvent)
}

public class ActiveApplication internal constructor(owner: ApplicationOwner) {
    public fun createWindow(attributes: WindowAttributes = WindowAttributes()): Window
    public fun monitors(): Array<Monitor>
    public fun primaryMonitor(): Monitor?
    public fun setControlFlow(flow: ControlFlow)
    public fun requestExit()
}

public class ApplicationProxy internal constructor(owner: ApplicationOwner) {
    public fun post(block: (ActiveApplication) -> Unit): Boolean
    public fun requestExit(): Boolean
}

public enum ControlFlow { Wait, Poll, WaitUntil(TimeMark) }

public class Window internal constructor(owner: WindowOwner) {
    public val id: WindowId
    public val state: WindowState
    public val metrics: WindowMetrics?
    public fun requestRedraw()
    public fun requestDestroy()
    public fun requestSize(size: LogicalSize): RequestStatus
    public fun setTitle(title: String)
}
```

每进程只创建一个 Application，run 只接受一次且必须在进程主线程、当前 Job 外执行；不承诺 run 返回后再次运行。基础 window Cone 不依赖 asyncio，也不检查其 Context 类型；runWithAsyncio 另由 PlatformEventLoop.attach 拒绝活动 I/O EventLoop binding 中的嵌套启动。构造只建立 managed 状态，原生应用／显示连接在接受 run 后创建。`proxy()` 可在 run 前取得，投递在开始服务后执行；启动失败和关闭后投递返回 false，不保留闭包。

Window、ActiveApplication、Monitor 的原生活动方法只允许 owner 线程；跨线程使用 proxy，所有用户代码在内部锁外执行。只读 id、不可变事件／快照可跨线程保存，但不赋予操作原生窗口的权利。跨线程不提供同步 invoke，避免 UI 等 worker、worker 又等 UI 的互锁。

proxy.post 是异步 FIFO，跨线程以入队顺序为准；关闭会丢弃尚未执行的业务投递，true 只表示已接受。它在 run 的 Context 下执行，不捕获投递线程的动态 Context。proxy 解决 UI 消息投递，session.post 解决结构化任务入口，两者不各自实现一套任务调度。

ControlFlow 默认为 Wait；WaitUntil 只接受 `MonotonicTimeSource` 的 TimeMark，拒绝其他来源，避免把 Scoop 的民用 `Instant` 当成 Rust 的单调 Instant。Poll 是显式持续运行请求，允许占用 CPU；Wait 模式没有固定 tick。截止时间仅决定下一次唤醒，不代替显示器刷新节奏。[winit ControlFlow 基线](https://docs.rs/winit/0.30.13/winit/event_loop/enum.ControlFlow.html)。

普通入口的原生回调先复制数据，再在有限事件批次中调用 handler；同一 Application 不递归派发用户事件。需要立即返回给 OS 的询问，例如 AppKit 关闭询问、IME 状态查询、未来 Win32 hit-test，使用已发布的窗口策略／输入状态作同步回答，不持有 OS 借用数据等待异步用户回调。

首期只提供库自有 Application.run，以及该循环承载 asyncio 的集成。不把 `pumpOnce` 暴露为跨平台承诺；任意外部 NSApplication／现有 GUI toolkit 嵌入需要独立的平台扩展设计。这里的“支持 GUI 主循环”是实现并拥有真实窗口主循环，不是后台线程周期调用 poll。

## 4. 跨 Cone 的 asyncio 接入

`scoop.window.asyncio` 提供 `Application.runWithAsyncio(handler: ApplicationHandler)` 扩展，以及仅在普通业务回调期间可用的 `currentApplicationScope(): CoroutineScope`。业务 handler／proxy 消息经 EventLoopSession.post 进入根作用域，`currentEventLoop()` 可用；scope.launch／async 的取消与资源寿命完全沿 asyncio。Exiting／Closing／Destroyed 属于收尾回调，不提供活动 scope，不能从中重新启动 I/O。

runWithAsyncio 在主线程、原生应用可服务后、首个业务事件前执行 attach；不会在已有 Job 中启动它。attach 失败关闭已初始化的窗口后端并将错误从 runWithAsyncio 抛出。原始 Application.run 不启动 asyncio，因此 Linux 创建窗口不要求 io_uring 或 Linux 6.1；选择集成入口才继承 asyncio 的运行要求。

`WindowLoopHost` 由 `scoop.window.asyncio` 实现，基础 window Cone 只提供真实宿主操作。`scoop.window.native` 的公开接入合同见[窗口规范第 8 节](../specs/SCOOP-WINDOW-SPEC.md#8-asyncio-集成)：

- `Application.runAttached(handler, attach)` 与 run 共用单次入口；在原生宿主可服务后把具体 `ApplicationHost` 交给 attach，取得 `ApplicationAttachment`。此低层入口为 Unsafe，供当前集成 Cone 使用。
- `ApplicationAttachment.post` 将业务闭包交给 session.post，并在执行时绑定应用 scope；`requestClose` 转交 session.requestClose。窗口层不认识 Job 以外的 asyncio 类型，也不调用其他 Cone 的 internal 方法。
- `ApplicationHost` 提供 owner 查询、对应平台的 `watchReadable`／`watchWindowsEvent` 及 ApplicationWatch.remove。WindowLoopHost 将它们适配为 asyncio 的 watch／remove；注册、派发和退休遵守同一借用合同。
- `ApplicationHost.postControl` 在业务 proxy 关闭后继续服务根取消和清理控制；`complete(failure)` 由 session.onClosed 在 owner 调用一次。即使应用没有窗口，窗口循环也要等本次附着完成、watch 注销及已接受控制工作完成后才停止。

这些入口只连接实际窗口宿主和当前 asyncio 适配器；没有适配器注册表、通用服务容器或任意 toolkit 嵌入框架。attach 返回前失败时回滚已取得资源；业务 post 被拒绝时开始收尾，不继续运行一个没有业务接收者的应用。

集成层观察根 Job 的取消，把开始退出排入窗口内部控制队列；不能等到 onClosed 才通知窗口 Closing，否则子任务 finally 与窗口释放可能互相等待。取消订阅不执行窗口操作，只唤醒 owner；关闭订阅与其他 callback 一起退休。业务事件进入 session，收尾控制走窗口内部队列，不能因业务 post 已拒绝而丢弃。

NewEvents／AboutToWait 按外部事件或业务投递形成的一轮逻辑批次发送一次；用 session.post 交付这些通知产生的内部唤醒不再开一轮相同通知。纯 I/O／watch 维护唤醒也不自动制造新的 AboutToWait，避免 Wait 模式中“通知待等待 → post 唤醒 → 再通知”形成空转。

采用 Windows 分支已实现的通知类型及 `EventLoopHost.watch(notification, ready)`。ActiveApplication 不继承 EventLoopHost，普通窗口 API 不公开描述符；目标通知合同与主线迁移位置见[asyncio 规范 11.3](../specs/SCOOP-ASYNCIO-SPEC.md#113-宿主接入合同)：

```scoop
public enum EventLoopNotification {
    ReadableDescriptor(Int),
    WindowsEvent(Ptr<Unit>)
}
```

接口直接使用 `@Unsafe watch(notification: EventLoopNotification, ready: () -> Unit): EventLoopWatch`。迁入主线时删除 asyncio 旧 watchReadable 声明，修改 MainDispatchHost、其他实际宿主、attach 和测试；没有默认转发、兼容别名或第二个通知 enum。窗口 native 包的 watchReadable 是实际 POSIX 监听入口，不是旧 asyncio 接口的兼容方法。需要重建的库按普通依赖更新处理。

WindowsEvent 指 loop 所有的 manual-reset event HANDLE；host 只监听，不 ResetEvent／CloseHandle。分支的 IOCP 后端已在同一锁边界协调真实队列发布、SetEvent 和 owner 复位。Windows 窗口宿主复用这一通知，不重新实现 IOCP 或完成队列。

外层 [MsgWaitForMultipleObjectsEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-msgwaitformultipleobjectsex) 联合等待之外，Windows host 还用原生 wait 回调把 event 就绪转为 owner HWND 消息，使系统拖动、调整大小和菜单的模态循环也能驱动 ready。原生 wait 回调只投递消息；一个 ready 待消费期间停止重复通知，owner 消费后再按有效状态恢复监听。remove 注销等待并退休排队消息、活动回调后才报告完成，通知 HANDLE 仍由 loop 关闭。

确实会进入原生模态循环的 beginMove／beginResize／showWindowMenu 等操作先保留当前有效输入上下文，排入 native 命令队列，等当前 handler 和 session pump 返回后执行。不能在活动 pump 栈内阻塞，再依赖被 LoopDriver 禁止的递归 pump 推进 I/O。普通用户事件仍不递归派发；所用 API 本身立即返回时不人为增加模态循环。Windows [WM_ENTERSIZEMOVE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-entersizemove) 的循环持续至 DefWindowProc 返回，需要真实拖动中的 timer／网络／取消验收。

watch/remove 的所有权、非内联注册、串行通知、回调退出后注销规则保持。Darwin GUI 适配在实际服务的 CFRunLoop modes 上监听 pipe，Linux 监听 eventfd；两者都不读取 loop 的通知载荷。窗口消息唤醒和 I/O 完成分别存储真实队列，可以合并通知，不能依赖“一次唤醒等于一个事件”。

集成 scope 保存窗口 owner 的 LoopExecutor，任意真实挂起后的外部完成均排队回该 executor，包括第三方 continuation。显式后台 executor 的任务保留 I/O binding，但其恢复仍在后台；窗口操作通过 withExecutor(loop.executor) 或捕获的应用作用域返回 owner。普通线程／native callback 继续使用 ApplicationProxy.post。执行与关闭复用现有 core／asyncio 协议，详见[窗口集成规范](../specs/SCOOP-WINDOW-SPEC.md#8-asyncio-集成)。

## 5. 窗口属性、DPI 与平台差异

当前 WindowAttributes 使用普通属性／值类型，包含标题、内容尺寸及约束、可见性、装饰、可调整大小、初始位置、resize increments、透明／模糊、主题、窗口级别、按钮集合、内容共享提示及激活请求。创建时取得独立快照，之后修改原属性对象不改变已创建窗口；ApplicationOptions 同样在构造时取快照。全屏、图标和初始光标属性属于后续范围，不列为已存在的创建字段。无平台支持的必需属性使创建失败；主题／聚焦等建议性属性只表示已请求，不能把请求值伪装成 OS 已接受的状态。各方法分组见映射表。

`RequestStatus` 为 Applied／Requested／Unsupported。Applied 只用于可立即确认的设置；Requested 由后续事件／快照确认；Unsupported 不产生副作用。非法参数用通常参数异常；平台失败用 WindowException，保留 operation、错误类别、backend 和可选原生 code。当前没有通用 supports 查询；实际调用仍须处理运行环境失联、权限或 compositor 状态变化。

公共类型表达实际后端的共同语义与必要差异。窗口级别用 Normal／AlwaysOnTop／AlwaysOnBottom；后续全屏采用 Windowed／Borderless(monitor?)／Exclusive(videoMode)。本阶段直接提供已实施的方法，不预留一组没有对应方法的 WindowFeature 枚举或空平台分支。Windows 接入时复用语义相同的类型；发现当前表示不合适时直接修订 API 与调用方，不为保持旧事件匹配增加平行模型。

窗口内部坐标以左上为原点，x 向右、y 向下。LogicalPosition／LogicalSize／LogicalRect 使用有限 Double；PhysicalSize 使用非负 Int，表示 drawable 像素尺寸；PhysicalPosition 保留小数，适合物理像素中的指针位置。尺寸非负、scaleFactor 为有限正值；render target 遇到零尺寸暂停取得／提交帧。

`WindowMetrics` 同时携带逻辑内容大小、实际 drawable 大小与有效 scaleFactor；一次 GeometryChanged 发布完整快照，原生配置未完成时为 None。Linux 使用 GTK 对内容 widget 的实际分配，排除标题栏与阴影；首次映射和隐藏重显中的协议配置由 GTK/GDK 处理，不能用 requested size 冒充可绘制尺寸。GtkGLArea 物理尺寸使用 widget 的整数 scale factor，桌面分数缩放留给 GTK／compositor。

全局位置单独用 `DesktopPosition`／`DesktopRect`，表示当前显示系统的桌面坐标；不能用某一个窗口的 scaleFactor 在混合 DPI 桌面上进行全局换算。桌面位置与尺寸查询允许 None，Wayland 不伪造全局窗口位置或 primary monitor。窗口内物理／逻辑换算只在同一 metrics 快照中进行，像素分配尺寸向上取整。

MonitorId、WindowId、DeviceId 为不同值类型，库内分配且在本次 Application 内不复用；不暴露 XID、指针或数组下标作为公共 id。Monitor 是可保存的自有快照，屏幕离开后不再出现在新枚举中，旧快照内容仍可读取；名称、位置、刷新率等未知时为 None，刷新率使用可选整数毫赫兹，不能假定 60 Hz。触摸身份和显示模式控制随对应后续能力加入。

### 5.1 多窗口、模态与自绘窗框

当前已支持一个 Application 创建多个独立的非模态顶层窗口，共用原生主循环，按 WindowId 路由事件。单个窗口的重绘、输入、销毁和 lease 独立；默认关闭最后窗口后继续运行，也可设置 LastWindowPolicy.Exit。当前状态及属性对照见[窗口规范第 4.1～4.3 节](../specs/SCOOP-WINDOW-SPEC.md#41-多窗口)。

decorations／resizable／buttons／level 等已有创建及运行中设置入口。AppKit 另有标题栏透明、隐藏标题和 full-size-content 选项；Win32 关闭装饰但保留 resizable 时仍使用 WS_THICKFRAME。去掉该原生边框需要同时关闭 resizable，完整自绘窗框的系统拖移／边缘缩放接口仍待实施。

模态属于单独的待补能力：现有 WindowAttributes 没有窗口 owner 或 modality，原生模态／菜单测试只验证嵌套循环与 asyncio 配合。后续需要在 window 层明确窗口归属、窗口模态与应用模态的输入范围、取消／关闭时恢复、owner 关闭和焦点返回；UI 提供对话框内容与结果，不实现第二套原生输入阻塞规则。

多窗口、自绘窗框和模态分别交付、分别验收。多窗口 UI 可以使用现有窗口基础；模态对话框和自绘窗框系统操作在对应窗口 API 真实实现后接入，不成为先交付普通自绘组件的前置门槛。

## 6. 输入与 IME

键盘事件分别表达 PhysicalKey、LogicalKey、KeyLocation、Pressed／Released、repeat、modifiers 与 synthetic 标志。LogicalKey 区分 Named／Character(String)／Dead／Unidentified；物理键使用既有标准键位集合及 Unidentified(NativeKeyCode) 退路。NativeKeyCode 保留实际平台编码来源，Windows 后端加入时表达 Win32 编码，不能把平台 scan code 当 Unicode。modifier 包含 Shift／Control／Alt／Super、锁定状态和可取得的左右侧信息。[winit KeyEvent 参照](https://docs.rs/winit/0.30.13/winit/event/struct.KeyEvent.html)。

文字统一通过 `TextInputEvent.Edit(TextInputBatch)` 中的 Commit 操作插入：普通可打印键、dead-key compose 和 IME 都归一到该入口；LogicalKey 用于快捷键和键位语义，不再隐式插入第二份字符。TextInputEvent 只含 Enabled、Disabled、Edit，旧草案的独立 Preedit／Commit／DeleteSurrounding 事件直接替换，不提供兼容别名。失焦时撤销 active preedit、清理按下状态；需要合成 release 时标记 synthetic，不伪造真实输入时间。

输入法基础 API 为 `setImeAllowed` 和 `setImeCursorArea`，对应 winit 的 marked／preedit、commit 与候选窗位置。文档接入另通过 `publishTextInputState`／`publishTextInputCaret` 发布应用快照和精确几何；macOS 的 NSTextInputClient 与 Windows 的 TSF 文档存储共用这套协议，支持 surrounding 查询和显式范围替换。purpose 不进入当前公共 API。

`TextInputState` 保留为应用可选用的纯值编辑辅助，包含 surroundingText、带 anchor／cursor 的 selection、可选 marked range、caret rect 和 revision；所有范围以 UTF-8 字节为单位并落在字符边界，选择保留活动端方向。AppKit 转换实际 marked text 的 UTF-16 选区，产生不依赖文档 revision 的 CurrentSelection 编辑。只改变候选窗位置不推进文本 revision。

TextInputBatch 携带可选 baseRevision 和有序编辑操作，具体载荷见[窗口规范第 5 节](../specs/SCOOP-WINDOW-SPEC.md#5-输入与文本)。一批操作只调用一次 handler，在开始时检查基础快照，按批次内顺序解释后续位置，最后发布一次新快照。不能在删除 surrounding 后立即推进公开 revision，再把同批 commit／preedit 当成过期数据。范围引用失效或 native 数据非法时整批拒绝并重新同步，不部分应用；无需旧快照的普通 CurrentSelection 提交保持可用。

Linux 经 GtkIMContext 接入 commit、preedit、delete-surrounding 与已发布的 surrounding／caret 快照，实际 Wayland／X11 输入法协议由 GTK/GDK 管理，不在 Scoop 中重复解析协议批次。AppKit 一次输入解释、Windows TSF 一次原子编辑事务仍保留各自的原生更新边界；公共批次与连接／revision 合同见窗口规范。

平台使用已发布的原生文档副本和候选窗矩形回答同步询问，不在 NSTextInputClient／TSF 回调中调用 managed handler、布局或挂起。同一编辑事务使用原生临时投影，事务外存在未确认编辑时返回 unavailable。窗口层只转换文本更新和坐标，编辑器保留真实文档、布局与 grapheme 导航的责任；后续 purpose 扩展不得向平台发布密码正文。

鼠标区分按键、进入／离开、窗口内位置、相对设备位移。滚轮保留 LineDelta 与 PixelDelta，明确相位和惯性信息；不能猜测每行固定像素数。触摸携带独立 contact id、phase、位置和可选 pressure；原生 pinch／rotation／pan／double-tap／pressure 保留事件来源与单位。不支持的原生手势不靠周期采样伪造。

raw device 事件默认只在应用有焦点时请求，设备数据没有稳定身份时 DeviceId 为 None；不同平台不保证每个物理设备都可区分。光标 None／Confined／Locked 分开表达；Locked 不隐含可见性修改，鼠标 warp 不等于相对输入。Wayland 抓取、激活、移动／调整大小所需的原生 input serial 由后端保留；beginMove／beginResize 只在当前有效用户输入派发中接受，不能用任意历史点击模拟权限。

## 7. 重绘与渲染边界

requestRedraw 只置 dirty 并唤醒，重复请求可合并。RedrawRequested 在窗口可绘制时发出；绘制回调内再次请求的重绘留到下一轮，不能递归绘制。遮挡、隐藏、零尺寸期间保留 dirty，恢复可绘制后补通知，不能为不可见窗口持续忙等。

`prePresentNotify()` 告诉窗口后端即将提交帧，用于 Wayland frame callback 等节流；它不是实际 present，也不表示 GPU 完成。渲染层拥有 Metal／Vulkan／OpenGL 的 device、swapchain、surface、提交和 vsync。窗口层不把 io_uring timer 当显示时钟，不内置 Skia surface 或强制渲染线程。[winit 绘制边界](https://docs.rs/winit/0.30.13/winit/)。

`scoop.window.native` 提供 `@Unsafe Window.acquireNativeWindow(): NativeWindowLease`。lease 保活 Window 和 display connection；NativeWindowHandle 按目标提供 AppKit(nsWindow, nsView)、Win32(hwnd, hinstance) 或 Linux Gtk(window, view)。Linux renderer 在 Gtk 内容容器内挂载 GtkGLArea；需要原始显示对象时经 GTK 公共接口取得，不绕过 GTK 独立提交窗口 buffer。句柄和 borrowed 生命周期保持明确类型，不为未实现后端预留假变体。

lease 的 handle 查询在 owner 上执行；显式 close 可跨线程，原子封闭该借用后把实际释放投递到 owner 的内部收尾队列，不在调用线程执行 GUI API。该队列在停止业务 proxy 投递后仍然可用，避免后台 GPU 完成回调无法释放窗口；重复 close 为空操作，调用后不得继续使用 handle。不能从 GC release 启动这条异步路径。Skia／GPU 包装持有 lease，销毁依赖原生窗口的 surface 并结束有关 GPU／native 借用后再释放。窗口进入 Closing 后不能取得新 lease，已有 lease 阻止库主动销毁原生窗口；lease 不保证外部显示服务器永不失联，也不授权跨线程操作原生对象。渲染层需要后台线程时遵守对应图形 API 的线程规则。

平台只能查询自己的类型化 handle；native renderer 按实际平台消费载荷。新增后端可以调整该 enum 与相关 renderer，按普通依赖关系重新编译。lease 只保证对象寿命，不能据此在隐藏、待 configure 或 Closing 状态提交帧；后台 renderer 须与 owner 串行协调已有提交、unmap 和后续重新配置。

## 8. 生命周期与退出顺序

Window 状态为 PendingConfigure／Open／Closing／Destroyed；重新显示时可再次进入 PendingConfigure，隐藏／映射状态由后端单独记录。CloseRequested 表示用户意图，不自动销毁；应用可忽略、确认后 requestDestroy，或先保存数据。requestDestroy 幂等、非阻塞，进入 Closing 并排队 Closing 通知，停止新输入／重绘与句柄借用。等 Closing 已交付、现有事件派发栈与 native lease 均结束后销毁原生对象，再发一次 Destroyed；Destroyed 后只保留 id／终态快照，活动操作失败。

ApplicationOptions 的最后窗口策略为 KeepRunning 或 Exit，默认 KeepRunning，与 winit 的显式退出行为一致。macOS 关闭最后窗口不被硬编码成退出，Windows 也不硬编码成相反行为。系统退出请求携带原因，由相同应用事件表达；真正的强制终止／显示服务器消失可能不允许 veto 或完整 cleanup，不承诺从被 OS 强制杀死中恢复。

正常 requestExit 或未捕获 handler 异常按以下顺序收尾：

1. 停止接受业务 proxy 投递与创建窗口，将所有剩余窗口转入 Closing，等同分别调用 requestDestroy；当前用户派发返回后，在主线程交付一次 Exiting 和各窗口的 Closing 通知，让渲染包装开始释放。
2. 集成模式调用 EventLoopSession.requestClose，沿已有根取消等待所有子任务、借用和 I/O 清理；继续服务平台事件、watch、native cleanup 与 GPU 回调。纯窗口模式没有这一步。
3. native lease 归零且派发退出的窗口可销毁；逐一发出最终 Destroyed。退出期间这些最终通知直接在 owner 的收尾 Context 中执行，不通过已经关闭的 session mailbox，不提供活动 Job。
4. 等所有窗口销毁、集成 session 的 onClosed 调用 ApplicationHost.complete、全部 watch 移除、已接受控制工作完成以及 native callback lease 退休。没有窗口时也要等附着完成；一个条件先完成不替代其他条件。
5. 断开 display／停止 AppKit 主循环，释放应用回调，run 返回；首个业务失败与清理失败沿 ResourceUseException 组合后抛出。

步骤 2、3 可以交错推进，但实际退出必须同时满足所有条件。Closing／Exiting 之后的用户异常只记录失败并继续收尾，不重入相同通知；某个 cleanup 失败也不能跳过其他窗口。泄漏的 native lease 或不返回的外部工作可能使退出无法完成，不能假称关闭成功或由 GC 强拆仍使用的窗口。

native 事件只存稳定 native payload 与数值 id，进入 managed 前复制文本／路径／坐标。所有 user callback 通过已有 ForeignCallback，捕获异常并通知应用收尾，禁止异常跨 Objective-C／C／Win32 栈展开。main 返回前必须退出注册和活动 lease；AppKit 自身需要的 native 回调不调用 managed 时也按对应 C／ObjC 对象寿命释放。

## 9. macOS 后端

用 AppKit 的 NSApplication／NSWindow／NSView／delegate／NSTextInputClient 承载真实桌面行为，全部在主线程；每轮原生派发有 autorelease pool。run 通过 NSApplication 的正常循环服务事件，退出用可返回的 stop 与必要唤醒，不调用直接结束进程的 terminate 或永不返回的 dispatch_main。[Apple 文本输入协议](https://developer.apple.com/documentation/appkit/nstextinputclient)。

ApplicationHost 的 Darwin 监听使用 libdispatch READ source 观察 pipe，再 signal／wake 一个安装到 AppKit 所服务 modes 的 CFRunLoopSource，由该 source 在 owner 上调用 ready。等待 owner 消费期间暂停 READ source 的重复通知，处理后按有效状态恢复；取消时平衡暂停并等待 cancel handler，再在 owner 注销 CFRunLoopSource。集成 Cone 的 WindowLoopHost 只适配这组操作为 asyncio 的 watch/remove；MainDispatchHost 继续服务实际使用主 dispatch queue 的调用方。必须以真实拖动、菜单和模态验证 common／tracking／modal modes 的推进，不能只靠 mode 名称推断。

窗口位置／内容尺寸、backing scale、屏幕切换、全屏转换、occlusion、主题由原生通知生成。NSView 只提供可供 renderer 使用的宿主，不由窗口库拥有 Metal device 或 Skia；是否配置 CAMetalLayer 由渲染适配管理，并在窗口关闭前恢复／释放其持有状态。

Darwin native `.m` 已按[语言规范 12.2.1](../specs/SCOOP-SPEC.md)与[实现规范 2.7](../specs/SCOOP-IMPL-SPEC.md)正式接入：显式选择的 `.m` 交给所选 Apple Clang，以 ARC 编译，使用独立 objc_flags；`.c`、generated-C 和 runtime 保持原语言。条件筛选先行，非 Darwin 选中 `.m` 明确诊断；本期不扩展 `.mm` 或 Scoop 的 Objective-C 语法。

framework 链接复用已有合同；macOS 阶段已实际验证本仓库 Mach-O linker 对 Objective-C 元数据、类／selector 引用、ARC 外部符号和 AppKit framework 的正常链接，记录见 PROGRESS.md。Objective-C 薄层使用系统对象和消息机制，不增加 runtime 调度 ABI。

## 10. Linux 后端

### 10.1 选择与公共边界

当前目标为 x86_64-unknown-linux-gnu。`SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 显式选择协议；未设置时有 WAYLAND_DISPLAY 则选择 Wayland，否则选择 X11。已选连接失败正常报错，不静默切换。WindowBackend 报告实际显示协议，完整当前合同见[窗口规范第 12 节](../specs/SCOOP-WINDOW-SPEC.md#12-linux-wayland-与-x11-后端)。

两种协议共用 GTK/GDK 宿主：GMainContext、应用 eventfd、deadline 与借入 readable fd 组合在 owner 的一次 poll 等待中，GTK signal 只更新快照或排队，managed handler 在 GMainContext dispatch 之外执行。asyncio 消费自己的通知；不独立读取 GTK 的显示连接，不新增轮询线程或 GUI loop。

### 10.2 Wayland

使用 GTK 的 Wayland 后端。窗口由 AdwWindow／AdwToolbarView／AdwHeaderBar 承载，标题栏、按钮、圆角与阴影由 libadwaita 提供，内容仍由 Scoop UI／Skia 绘制。旧 libdecor 和直接协议窗口实现已移除；surface role、配置、缩放和提交由 GTK/GDK 管理。

窗口真实内容分配到达后发布 metrics；隐藏时暂停帧需求，重显后等待有效 GTK 分配与帧机会再呈现。renderer 使用 GtkGLArea 的真实 framebuffer 和 frame clock，不自行创建 wl_egl_window／EGL window surface。全局窗口位置与主显示器返回未知，无相应公开能力的全局定位／topmost 请求返回 Unsupported。

输入、IME、剪贴板及显示器使用 GTK/GDK。新窗框、区域、关系／模态和文件面板在这套宿主上扩展，不能为每项功能建立另一份 Wayland 显示连接或独立协议调度。

### 10.3 X11

同样使用 GTK/libadwaita 与 GtkGLArea；通过 GDK 公共入口取得需要的 Xlib Display／XID，现有全局位置、窗口层级与 attention 使用原生 X11 请求。显示事件仍由 GTK 读取，不混入另一套 XCB／Xlib 事件消费者。

WM 可以忽略状态或叠放建议，Requested 不冒充观察结果。透明需要实际 compositor；输入、IME、剪贴板和显示器继续由 GTK/GDK 处理。全屏、拖放、raw／touch 等未交付能力不因 GTK 存在就记为完成。

### 10.4 依赖与发行

最低依赖为 GTK 4.18、libadwaita 1.4，运行时使用系统动态库。维护阶段通过现有 prepare_gtk.py 与 pkg-config 准备 GNU sysroot 的头文件、native 链接输入及许可；Linux GUI 按需要声明 C++ runtime。musl 暂不包含 GUI，不要求普通无窗口程序带入桌面库。

消费预编译 Cone 不临时联网或安装依赖；缺失库、显示连接与不支持的显式创建属性正常报错。托盘额外依赖由独立 scoop.tray Cone 声明。Wayland／X11、软件 GPU 与真实硬件验证及已知 GTK/GNOME 限制分别记录于 [Linux GUI 实施记录](../linux-gui/PROGRESS.md)。

## 11. Windows 后端

本阶段以 Windows 10 1703 起的 Per-Monitor V2 为窗口 API 基线，消费现有三个 Windows x86_64 target。消息窗口和业务 HWND 均由主线程拥有，原生 C 薄层通过已有 ForeignCallback 回到 Scoop。共享生命周期、业务／收尾队列和测试辅助不复制为第二套实现。平台文件由普通 Cone source 条件选择；native handle 增加 Win32(hwnd, hinstance)，未知键码分别保留 Windows scan code 和 virtual key 来源。

普通窗口使用 Unicode Win32 API。物理键来自扩展 scan code，逻辑键来自当前键盘布局；普通 WM_CHAR／WM_UNICHAR 与 TSF 文档编辑统一为自有 UTF-8 批次。owner 线程激活 ITfThreadMgr，每窗口以 ITfDocumentMgr／ITfContext 管理焦点，应用侧实现 ITextStoreACP 和必要通知。原生文档投影把 ACP／UTF-16 范围归一为公共 UTF-8 操作，revision／origin 保证输入法编辑与上层确认一致；COM/C++ 限制在 native 后端，不保留 IMM32 组合后端或公共 TSF 扩展。raw device、触摸、全屏和捕获仍属后续扩展。

所有 HWND 创建前确认 owner 的 DPI awareness。WM_DPICHANGED 应用系统建议 rect 后发布实际 client pixel size 与 DPI 的完整快照。桌面位置采用物理虚拟桌面坐标；内容逻辑坐标按本窗口 DPI 转换，不用单个 scale 换算整个桌面。显示器枚举使用 EnumDisplayMonitors／GetMonitorInfo／当前显示设置，离线身份由共享层退休。

通知使用一次性 native threadpool wait；worker 只向 owner HWND 投递就绪消息，owner 消费后才重新 arm。remove 停止 wait、等待 native 回调退出，排队消息以 identity 查找仍存活的登记，完成通知只在活动 ready 返回后交付。借入的 manual-reset event 不由 host reset 或 close。真实系统 move／size、菜单和模态循环的 HWND 消息仍能驱动这条路径；会进入模态的测试命令必须在当前 Scoop handler 和 session pump 返回后执行。

renderer 示例使用独立普通 Cone 的 Win32／DXGI surface，持有窗口 lease，实际提交后先释放 GPU／swapchain 再结束 lease。普通行为使用 scoop.test，三个 Windows target 共用源码；桌面交互、混合 DPI 和输入法验证分别记录实际证据与环境限制。

Windows 平台移植的参考状态见[分支进度](https://github.com/scoop-lang/scoop/blob/0fa053fd11c11e641e2f17212ce4948d5d52447a/docs/windows-targets/PROGRESS.md)：目标、runtime、platform、IOCP 及 executable subsystem／RC 已有实现，整体移植仍在推进。窗口任务消费这些平台能力，补 Win32 消息、窗口、输入和显示系统后端；不重复开发另一套 target 或 I/O backend。

| Windows 窗口工作 | 设计要求 |
| --- | --- |
| HWND 与消息队列，WM_CLOSE／WM_DESTROY | WindowId、CloseRequested、Closing、Destroyed |
| 消息泵、系统模态循环与 manual-reset event | EventLoopNotification.WindowsEvent、HWND 消息唤醒及同一 watch/remove 退休合同 |
| Per-Monitor V2、WM_DPICHANGED 与建议 rect | 创建窗口前确认 DPI 模式，完整 WindowMetrics、请求与实际值分离 |
| 虚拟键／scan code／WM_CHAR／IME 或 TSF | PhysicalKey、LogicalKey、UTF-8 TextInputBatch／TextInputState |
| WM_POINTER／Raw Input／cursor clipping | Pointer／Touch／DeviceEvent 与 Confined／Locked 能力 |
| Win32／DXGI rendering surface | 实施时增加 NativeWindowHandle.Win32，并遵守 lease 寿命 |
| 系统关机、激活限制、DWM 功能差异 | 退出原因、RequestStatus、WindowFeature／WindowException |

Windows 后端要求创建窗口的 owner 使用 Per-Monitor V2。应用优先通过已有 `[windows].rc` 嵌入 DPI manifest；没有声明时，在任何 HWND 或依赖 DPI 的 UI 初始化前设置并确认实际模式。已是所需模式时直接使用；已有不同模式或设置失败明确报告 WindowException，不能用被虚拟化的坐标冒充真实物理尺寸。`subsystem = "gui"` 只决定 executable subsystem，不隐含 DPI 设置，也不需要新增窗口专用的资源配置语言。参考[默认 DPI awareness](https://learn.microsoft.com/en-us/windows/win32/hidpi/setting-the-default-dpi-awareness-for-a-process)与 [WM_DPICHANGED](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged)。

平台专有能力放 `platform.windows`；共同语义继续通过公共窗口 API 表达。真实 Windows 接入发现 API、事件、native 产物或构建合同需要变化时直接修订，并同步其他实际后端和调用方。首期不以“未来无需改 API”为验收承诺，也不创建 Windows 窗口空实现；后续验收必须包含真实模态循环、混合 DPI 和 renderer。

## 12. 实施顺序与验收

1. 按本草案定义最小端到端子集，先落实实际需要的 `.m` native 管线，并采用 Windows 分支的 EventLoopNotification／watch；同步实际宿主与调用方，不新增旧签名兼容层。
2. 用真实 window／window.asyncio 两个 Cone 完成公开宿主接入、窗口创建、输入、提交一帧及退出。持有 renderer lease 和活动 I/O 时请求退出，确认窗口、session、watch 与 lease 能依次或交错收尾；没有窗口时也能等待 session 完成。渲染使用小型实际 surface 适配，不建设完整 GUI 或通用渲染框架。
3. Linux GNU 已通过 GTK/libadwaita 接入 Wayland／X11 的窗口、内容分配、输入／IME、装饰、GtkGLArea 和 asyncio；当前验证见 Linux GUI 实施记录。拖放等未交付能力继续独立补齐，musl 不列入当前 GUI 范围。
4. 在已有基础光标、monitor、主题和窗口属性之上，按映射表分批补充 touch／gesture／raw device、全屏控制及高级可选属性；每项“平台不支持”均有真实 capability 依据和明确结果。
5. 每个阶段以配套示例、真实目标测试和明确限制独立交付。当前基础架构／macOS 骨架阶段与完整能力地图分别记录；Linux 窗口及平台专测另行安排，平台基础分支通过不等于窗口可用。

纯值／归一规则、状态、错误与普通 API 行为用所属 Cone 的 scoop.test；真实窗口与 callback 测试使用主线程测试程序，必要的 OS 进程控制／ABI／native 编译能力才用少量 fixture。当前覆盖 macOS、Windows 和 Linux GNU 的 GTK Wayland／X11；实际执行范围分别记录。Xvfb／嵌套 compositor 可覆盖自动化行为，但不能替代硬件显示器、输入法候选交互、真实用户拖动及不同桌面 Shell 的验证。

重点组合：多个窗口与任务并存；关闭／外线程投递竞争；移动 GC 下事件文本和 ForeignCallback；关闭中 surface lease 未释放；用户 handler／清理抛错；菜单、拖动、模态期间 timer／网络／取消推进；混合 DPI 跨屏与热插拔；零 drawable 与 Wayland 零尺寸提示的区别；隐藏／重显后真实 buffer 提交；中文／日文组合、emoji／UTF-16 range、dead keys、重复输入与失焦；同批 delete／commit／preedit、选择方向、协议 serial 与文本 revision 的区别；Wayland flush EAGAIN／prepared-read 取消；X11 WM 忽略请求；display 断开；退出时 I/O、window、watch 各自先完成及没有窗口的顺序。

Windows 的首期工作是审查公共语义与已实现平台基础的接缝：普通窗口参数不携带 fd／HWND，错误和 native 句柄保留实际来源与宽度；通知复用 EventLoopNotification，Windows 实施满足模态循环和 DPI 初始化合同。无需预留所有未来 enum 分支，也不把静态表示审查或空 stub 当成 Windows 窗口验收。

性能只记录空闲 CPU／唤醒、输入到 handler、redraw 合并、I/O 与 UI 同时繁忙时的延迟及关闭耗时；主循环不引入通用预算或计费。渲染性能与完整 GUI 基准留到实际 renderer 接入。当前代码与验收结果见实施记录。
