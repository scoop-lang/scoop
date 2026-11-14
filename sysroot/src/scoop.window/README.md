# scoop.window

面向上层 GUI／renderer 的窗口骨架，能力边界参考 winit 0.30.13。已实现 macOS AppKit、Windows Win32，以及 Linux GNU 的 Wayland／X11 后端。

库负责窗口、原生事件、重绘请求、显示器快照、原生菜单和显式生命周期，也提供 GUI 文本剪贴板。控件、文档布局、无障碍语义和图形设备由上层实现。窗口库和 `scoop.window.asyncio` 都是普通 Scoop Cone，AppKit 薄层通过普通 `.m`／ARC 接入，Win32 和 GTK 薄层使用 C native 构建。

## 依赖与入口

```toml
[dependencies]
"scoop:scoop.window" = "0.1.0"
```

在主线程、活动 Job 之外创建 `Application`，以 `run(handler)` 驱动 `ApplicationHandler.onEvent`。一个进程只能创建一个 Application，并运行一次。`Resumed` 中使用 `ActiveApplication.createWindow`；原生关闭按钮产生 `CloseRequested`，由应用决定销毁窗口或退出。

默认关闭最后一个窗口后继续运行；使用 `ApplicationOptions(LastWindowPolicy.Exit)` 可在最后窗口实际销毁后退出，也可以主动调用 `ActiveApplication.requestExit()`。运行结束会返回调用方。

[Metal 示例](examples/metal/README.md)和 Windows [DXGI 示例](examples/dxgi/README.md)给出了完整可运行入口和独立 renderer Cone。准备好[开发 sysroot](../../../scripts/README.md)后，在仓库根运行：

```sh
export SCOOP_SYSROOT="$PWD/sysroot"
target/debug/scoop run sysroot/src/scoop.window/examples/metal/app --release
```

## 当前能力

| 能力 | 接口与行为 |
| --- | --- |
| 主循环 | Wait、Poll、单调 WaitUntil；跨线程 `ApplicationProxy.post`／`requestExit`；回调串行交付 |
| 定时回调 | `Window.createDeadline` 创建窗口所属的一次性回调；共用主循环最近期限，关闭窗口即撤销 |
| 窗口 | 独立／Owned 顶层窗口、Tool 角色与一次 raise；标题、显示／隐藏、焦点请求、尺寸及约束、桌面位置、窗框及按钮、窗口级别、最小化／最大化、主题、透明／模糊、内容共享提示、注意力请求 |
| 模态 | 从隐藏窗口创建 ModalSession；Owner／Application 输入范围、SeparateWindow／Sheet 能力、嵌套、取消与原生使用退休 |
| macOS | activation policy、titlebar 属性、bundle identifier 查询 |
| 显示器与 DPI | 拥有数据的显示器快照、可选名称／桌面位置／刷新率；窗口完整 logical／physical／scale metrics |
| 输入 | 物理键、当前布局逻辑键、重复、左右修饰键、未知键码、指针位置／按钮、序列取消、行／像素滚轮及 momentum、单调输入时间、基础光标和区域内隐藏 |
| 文本 | 普通文字与 IME 共用编辑批次；marked／preedit、commit、候选窗位置、失焦及禁用时撤销组合 |
| 绘制 | 合并重绘请求；隐藏／遮挡／零尺寸时保留 dirty；显式 native lease |

`Window` 操作和原生句柄查询都要求 owner 线程。`RequestStatus.Requested` 表示已向系统提交请求，实际结果由事件或快照确认；例如焦点请求不保证系统一定授予焦点。

`window.createDeadline { ... }` 返回可反复 `schedule(time)`、`cancel()` 的 `WindowDeadline`，time 使用 MonotonicTimeSource。已经到期的时间也在后续主循环批次交付；重排、取消或关闭使旧的排队回调失效。回调在窗口隐藏时仍能执行，但不会隐式请求重绘。内部定时与应用的 WaitUntil 独立，只有后者实际到期才发出 StartCause.Deadline；不需要为每个组件另建线程或改写应用的控制流。

内容区坐标以左上为原点。`LogicalPosition`／`LogicalSize` 使用窗口逻辑单位，`PhysicalPosition`／`PhysicalSize` 使用像素。使用同一次 `WindowMetrics` 或输入事件的 scale 换算；桌面坐标不能乘某个窗口的 scale。未知的原生信息保留为 `None`。

`PointerState.time`、`KeyEvent.time` 和 `PointerEvent.Cancelled` 携带 `InputTime`，其 `mark()` 与 `MonotonicTimeSource` 同源。当前后端使用原生接收边界的 `Observed`；它区别于可可靠映射的原生发生时间 `Event`，不在业务派发时重新采样。失焦或原生捕获／跟踪中断仅取消仍活动的鼠标序列；正常离开内容区继续拖动，取消后的迟到 Up 被忽略。

全屏控制／显示模式切换、全局 raw device、触摸／高级手势、光标捕获／自定义光标、文件拖放和复杂平台窗口交互属于后续范围。完整映射见 [WINIT-MAPPING.md](../../../docs/windowing/WINIT-MAPPING.md)。

## 多窗口、模态与窗框

在 Resumed 中可以多次调用 ActiveApplication.createWindow；这些窗口共用一个 Application 和事件循环，以 ApplicationEvent.Window(id, event) 区分事件。它们各自重绘和销毁，默认是独立的非模态（modeless）顶层窗口。最后一个窗口实际销毁后的行为仍由 LastWindowPolicy 决定。

通过 windowAttributes 的 relation = WindowRelation.Owned(parent) 建立固定的顶层窗口归属；role = WindowRole.Tool 请求平台支持的工具面板角色。隐藏或最小化 owner 时暂时隐藏其后代，恢复时保留显式隐藏的意图；销毁 owner 会先关闭后代，并等待各自的 native lease 退休。AppKit 隐藏 child 时原生 parent 关系会暂时撤销，逻辑 owner 不改变，显示时重新建立关系。

先创建 visible = false 的窗口并挂载内容，再调用 window.showModal(ModalOptions(scope, presentation))。Owner 只阻塞直接 owner，Application 阻塞其他应用窗口；两者与非模态 Tool、全局 AlwaysOnTop 各自独立。session.dismiss() 正常结束并隐藏窗口，cancel() 取消，均不销毁所借 Window；结果保存在 session.result，并通过 ApplicationEvent.ModalCompleted 通知。Window.isInputBlocked 和 InputBlockedChanged 可供内容宿主同步交互状态。

desktopCapabilities().windows 分别报告相对叠放、工具角色、raise、层级传播和模态展示组合。Linux transient 关系是 WM／compositor 提示，Wayland 不提供通用工具窗角色、无激活 raise 或 topmost。AppKit 的 Automatic Owner 展示采用普通 sheet，Application 展示使用原生应用模态；不使用 critical sheet 伪装普通嵌套。详细合同与测试入口见[模态规范](../../../docs/specs/SCOOP-WINDOW-MODAL-SPEC.md)和[模态测试说明](tests-cones/modal/README.md)。

## 原生文件与目录面板

ActiveApplication.openFile／openFiles／saveFile／selectDirectory 返回 FileDialogSession<T>，分别交付 Path 或非空 Array<Path>。目标必须明确传 DialogTarget.Owner(window) 或 DialogTarget.Application；后者也支持没有业务窗口的应用。文件面板与窗口模态共用输入限制和关闭依赖，嵌套面板结束不会解除外层模态。

```scoop
val request = application.saveFile(
    DialogTarget.Owner(window),
    SaveFileOptions(
        suggestedName = Some(OsString.fromString("report.txt")),
        types = FileTypes.Extensions(["txt"])
    )
)
```

通过 ApplicationEvent.FileDialogCompleted(id) 找到对应请求，读取 request.result 的 Accepted／Cancelled／Failed。cancel() 请求取消原生展示，原生调用和回调退休后才完成 session 并释放 owner；用户取消与原生失败分别表示。路径复用 scoop.platform 的 Path／OsString，保留本地编码。保存面板只选择目标路径，应用随后执行文件 I/O。

desktopCapabilities().fileDialogs 报告入口和选项能力。AppKit 使用 NSOpenPanel／NSSavePanel，Win32 使用 Common Item Dialog，Linux 使用已有 GTK/GIO 的 GtkFileDialog，不引入托盘依赖。详细合同与测试入口见[文件对话框规范](../../../docs/specs/SCOOP-WINDOW-DIALOGS-SPEC.md)和[测试说明](tests-cones/dialogs/README.md)。

创建时使用 frame，运行中用 setFrame 切换 System(Standard／Extended) 与 Client。Client 去除宿主可见装饰，resizable／buttons 仍独立控制系统操作；System 的 Extended 保留真实系统按钮并让内容进入标题栏。frameMetrics 给出实际内容可用矩形和按钮占用，未知值为 None。desktopCapabilities().frame 分别报告窗口裁剪、独立输入区域、整窗忽略鼠标、系统拖动／缩放与阴影等能力。setWindowRegion、setInputRegion 和 setHitRegions 使用内容逻辑坐标中的自有矩形集合；None 恢复默认，空区域没有覆盖。系统 beginMove／beginResize／showWindowMenu 必须在当前真实输入事件的 handler 内调用，阻塞操作在回调退栈后执行，接管前取消原指针序列。详细平台边界见[窗框规范](../../../docs/specs/SCOOP-WINDOW-FRAME-SPEC.md)。

macOS 的 TitlebarOptions 通过 setMacOSTitlebar 设置标题栏透明和隐藏标题；内容延伸统一由 WindowFrame.System(Extended) 表达，Client 窗口仍可接收键盘焦点。Windows 的边框色、标题栏色、文字色与圆角偏好在 platform.windows 中分别查询 DWM 能力和设置。当前接口见[窗口规范第 4.1～4.3 节](../../../docs/specs/SCOOP-WINDOW-SPEC.md#41-多窗口)。

## 文本输入

`ActiveApplication.clipboard`／`Window.clipboard` 提供系统文本剪贴板：`readText(mediaType = "text/plain")` 返回 `String?`，没有该格式为 None，已有空文本为 Some("")；`writeText(text)` 替换为标准纯文本。`writeTexts` 同时发布同一项内容的多个文本表示，例如所见文字和可继续编辑的 Markdown：

```scoop
application.clipboard.writeTexts([
    ClipboardText("text/plain", "中文标题"),
    ClipboardText("text/markdown", "**中文标题**")
])
val markdown = application.clipboard.readText("text/markdown")
```

媒体类型使用无参数的具体 `text/<subtype>` MIME 名称，按 ASCII 小写归一；列表不能为空，格式不能重复。标准纯文本读写把 CRLF／CR 归一为 LF，Markdown／HTML 等其他文本格式保留换行。所有表示禁止内嵌 NUL；写入参数错误在接触剪贴板前报告，读取含 NUL 的原生文本为 Unsupported，非法 UTF-8 或原生写入失败为 NativeFailure。库不解析 Markdown 或 HTML。macOS 使用标准 pasteboard 类型与 MIME 映射，Linux 使用 GDK MIME 表示，Windows 使用 Unicode 纯文本和按 MIME 名称注册的 UTF-8 文本。

操作要求应用 owner 线程和仍运行的应用，从窗口取得的入口不会保活窗口，也不随该窗口关闭失效。剪切应在全部表示写入成功后再删除选区；原生写入失败不承诺系统无法提供的回滚。macOS 的剪贴板测试保存并恢复原有全部项目和格式，覆盖原生类型互通、失败、Unicode 与超过 1 MiB 的 Markdown。

文本输入默认禁用 IME；普通可打印键仍通过 `TextInputEvent.Edit` 交付。调用 `setImeAllowed(true)` 接入平台输入法，`setImeCursorArea` 发布候选窗矩形，坐标为内容区逻辑坐标。Enabled／Disabled 表示窗口的 IME 接入开关变化，重复设置相同值不重复发事件。

应用只从编辑批次插入文字，不再从 `KeyEvent.logicalKey` 额外插入字符。输入法消耗的候选选择／确认键不会再作为普通按键交付；普通快捷键保留 Keyboard 事件。失焦会撤销 active preedit，并为已交付的按下状态补发 synthetic release。

`KeyEvent.textCommands` 保存 window 后端解释出的移动、删除、提交等编辑语义；多条命令保留顺序，随该次按下事件进入同一输入路由。macOS 使用原生 selector，Linux 使用当前逻辑键和标准 Control 编辑键位。空列表表示本次没有编辑命令，keyup 和 synthetic 输入也没有命令。组合更新／提交／取消已消费的按键不会再触发确认或关闭动作，未知 selector 仍保留 Keyboard 回退。

`TextInputState.apply(batch)` 是可选的纯值编辑辅助：按次序应用整批操作，全部合法后才发布一次新 revision。范围使用 UTF-8 字节边界；AppKit 的 UTF-16 范围和 TSF 的 ACP 范围在 native 边界转换。取消组合的空 Preedit 会删除当前显示的组合文本；ClearPreedit 只清标记，不能代替取消。

macOS／Windows 的 Enabled／Disabled 都带有 `TextInputConnection`，原生批次的 origin 标识连接和交付顺序。编辑器用 `publishTextInputState(connection, state, after)` 立即发布已生效的文档及已处理批次；AppKit／TSF 的同步 surrounding／selection 查询读取原生不可变副本。引用该快照的显式替换带实际 baseRevision，前序批次尚未处理时不生成猜测的文档范围。CurrentSelection 批次继续不依赖 revision；无效范围拒绝整批，旧连接的编辑不能落到新连接。

`publishTextInputCaret` 单独接收当前连接和文档修订的候选框；没有匹配几何或仍有待处理编辑时，原生位置查询返回 unavailable。`None` 或 `clearImeCursorArea()` 撤回旧矩形。没有发布文档快照的基础 IME 接入只维护 marked segment；Windows 完整文档连接使用 TSF，不保留 IMM32 组合后端。

## renderer 与关闭顺序

`requestRedraw()` 合并请求；在 `RedrawRequested` 中确认 metrics 可绘制，调用 `prePresentNotify()` 后由 renderer 提交帧。窗口库不创建 Metal／D3D／Vulkan／OpenGL 设备，也不把 WaitUntil 当作显示器时钟。

renderer 在 owner 上通过 `scoop.window.native` 的 Unsafe `acquireNativeWindow()` 取得 `NativeWindowLease`。目标平台决定 handle 变体：AppKit 包含 NSWindow／NSView，Win32 包含 HWND／HINSTANCE。lease 保活原生窗口；`close()` 可跨线程且幂等，实际释放回到 owner。调用 close 后不得继续使用句柄，GC 不代替显式 close。

`requestDestroy()` 开始 Closing；Closing 后不再接受业务输入和重绘。renderer 在 Closing 中停止提交、销毁 surface 并 close lease；等全部借用和回调退休后，库才销毁原生窗口并发出最终 Destroyed。

## asyncio

同时声明 `scoop:scoop.asyncio` 和 `scoop:scoop.window.asyncio` 依赖，导入对应包，使用 `Application.runWithAsyncio(handler)`。业务回调可取得 `currentEventLoop()` 和 `currentApplicationScope()`。跨线程业务通过 proxy 返回 owner。

退出／根取消先开始窗口收尾，并继续原生循环直至子任务清理、native lease、通知 watch 和 session 全部结束。Exiting／Closing／Destroyed 在收尾 Context 中交付，没有活动业务 scope。dispatch READ 通知在 AppKit common、tracking 和 modal modes 中服务；Windows 使用 threadpool wait 和 HWND 消息服务系统 modal／menu／move-size 循环；不靠固定 tick 轮询。

## 验证

在已登录的 macOS 图形桌面及准备好的 Darwin sysroot 上运行：

```sh
target/debug/scoop test sysroot/src/scoop.window --release
target/debug/scoop test sysroot/src/scoop.window/tests-cones/deadlines --release
target/debug/scoop test sysroot/src/scoop.window.asyncio --release
target/debug/scoop test sysroot/src/scoop.window/examples/metal/app --release
```

这些测试使用 `scoop.test`，包括真实 AppKit 窗口、NSEvent 与 NSTextInputClient 协议调用、UTF 范围／编辑批次、跨线程投递和 lease、真实 modal／menu tracking 中的 I/O，以及实际 Metal drawable 提交。Metal 验收需要可用图形设备。

macOS 桌面测试须串行执行。AppKitApplicationTest 等待真实窗口焦点再验证输入协议；系统可以拒绝后台 activate 请求，此时须把测试应用置于前台。准备期为 30 秒，得到实际焦点后用独立的 5 秒期限验证事件与收尾，不能仅凭 requestFocus 返回 Requested 判断焦点成功。

显示器热插拔、混合 DPI、不同键盘布局、系统输入法候选交互和真实用户拖动仍需实际桌面／硬件验收；协议调用测试不等同于这些手工验证。实现与验证记录见 [PROGRESS.md](../../../docs/windowing/PROGRESS.md)。

## Windows

支持 Microsoft UCRT、LLVM-MinGW UCRT 和 LLVM-MinGW MSVCRT 三个 x86_64 target，要求 Windows 10 1703+ 的 Per-Monitor V2。应用可通过标准 RC manifest 声明 DPI awareness；没有声明时窗口库在创建 HWND 前设置。已经启用其他 DPI 模式时明确抛出 WindowException，不继续使用虚拟化坐标。

Win32 桌面坐标采用物理虚拟桌面单位；内容区按窗口 DPI 转换。遮挡快照报告最小化／DWM cloaked，不判断其他窗口的完全覆盖。主题与激活遵守系统限制；Windows 8+ 的 DwmEnableBlurBehindWindow 已无 blur 效果，因此 `setBlur(true)` 返回 Unsupported，创建时要求 blur 则失败。内容保护只是系统显示提示。

在已登录的 Windows 桌面串行运行以下普通 Cone 测试；真实模态／菜单／尺寸调整会使用前台窗口，不并行运行这些 GUI 测试。两个 MinGW target 另指定对应的 `--native-sysroot`：

```powershell
scoop test sysroot/src/scoop.window --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/dpi --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/input --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/errors --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/watch --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window.asyncio --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window.asyncio/tests-cones/no-window --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/examples/dxgi/app --release --target x86_64-pc-windows-msvc-ucrt
```

DPI、输入、错误收尾、watch 和无窗口退出用例各自成 Cone，因为 Application 与进程 DPI 模式不可在同一测试进程中重启。测试不会重置库的单次入口约束。输入测试通过真实 HWND 消息及与 IMM32 共用的 native UTF-16 归一路径验证批次、候选窗矩形和载荷所有权；不冒充已安装中／日输入法的候选交互。watch 测试验证 WaitUntil、借入 manual-reset HANDLE、ready 内 remove 和重复 remove；无窗口退出测试验证实际 IOCP 接收取消与异步 finally。实际显示器／输入法覆盖边界见 [PROGRESS.md](../../../docs/windowing/PROGRESS.md)。

## Linux GNU

`SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 显式选择后端。未设置时，有 `WAYLAND_DISPLAY` 则选择 Wayland，否则选择 X11；连接失败不会改连另一显示服务器。`DISPLAY`、`WAYLAND_DISPLAY` 和 `XDG_RUNTIME_DIR` 使用显示系统的普通约定。

两个显示后端都由 GTK 4.18 以上和 libadwaita 1.4 以上实现；`AdwWindow` 和 `AdwHeaderBar` 负责标题栏、按钮、圆角和阴影。原生输入法、剪贴板和显示器使用 GDK／GTK 接口，X11 的位置和窗口管理请求使用 Xlib，键符号转换使用 xkbcommon。字符比较复用目标资源 `scoop_text_backend`。开发资源按[准备脚本说明](../../../scripts/README.md)放入目标 sysroot，普通应用使用预编译 `.slib` 和系统动态库。

窗口在 GTK 首次分配内容区前为 PendingConfigure，metrics 为 None；显示后由 GeometryChanged 给出实际内容尺寸。framebuffer 按 GTK 的整数 scale factor 计算，桌面的分数缩放由 GTK／合成器处理。Wayland 桌面位置与主显示器等未提供的信息返回 None。最大内容尺寸约束、非默认 resize increments、单窗口强制明暗主题、blur 和内容保护返回 Unsupported；显式请求不支持的创建属性会报错。普通 Window／UI 接口保持，Linux 专用的 `NativeWindowHandle.Gtk` 借出 GtkWindow 与内容容器。

两个后端使用 Linux evdev 物理键和 GDK 当前键盘布局；有完整按键状态时保留左右修饰键，未知键保留原生代码。输入法由 GtkIMContext 接入当前桌面的输入服务，未启用输入法时使用 GTK 的普通字符／compose 输入。剪贴板使用 GdkClipboard，实际跨应用传递遵守显示服务器的焦点与选择规则。

`ShortcutChord` 的 Primary 为 Control，`Window.shortcutLabel` 按当前 XKB 布局显示物理键，使用 `Ctrl+Shift+…` 形式；逻辑键保留声明的文本。AltGraph 状态可确定时通过 `Modifiers.altGraph` 提供，输入 AltGraph 字符不会触发普通快捷键或 Control 编辑命令。

验证命令使用准备好的 GNU sysroot：

```sh
SCOOP_WINDOW_BACKEND=x11 scoop test sysroot/src/scoop.window --release
SCOOP_WINDOW_BACKEND=wayland scoop test sysroot/src/scoop.window --release
SCOOP_WINDOW_BACKEND=wayland scoop test sysroot/src/scoop.window.asyncio --release
```

窗口测试使用真实显示服务器，X11 输入测试还需要 libXtst 开发包；GTK 自行映射普通窗口，应用绘图由 Surface 挂载的 GtkGLArea 提供。GTK 信号仅排入窗口事件，业务回调在 GTK dispatch 返回后执行。验证范围、依赖版本及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
