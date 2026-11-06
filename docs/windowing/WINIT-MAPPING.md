# winit 能力映射与分阶段范围

状态：macOS／Windows 骨架与 Linux GNU 的 GTK4／libadwaita Wayland／X11 后端已实现；更新于 2026-11-07，同步 Linux 设计基线，未新增实现。与 [DESIGN.md](DESIGN.md) 和[规范](../specs/SCOOP-WINDOW-SPEC.md)一起阅读。基线为 winit 0.30.13；实际交付与验收边界见 [PROGRESS.md](PROGRESS.md)和 [Linux GUI 实施记录](../linux-gui/PROGRESS.md)。

当前阶段交付 macOS／Windows／Linux GNU GUI 基础：应用／窗口、proxy／关闭／lease、基础属性、显示器与 DPI、键盘／鼠标／滚轮／IME、renderer 与 asyncio 接入。Linux 使用 GTK/GDK 与 GtkGLArea，各操作仍按实际显示协议提供能力。全屏、显示模式、raw device、触摸／高级手势、光标捕获、文件拖放等继续按各自范围补充。

范围分为：本期通用、本期按平台能力、平台扩展、后续／范围外。不承诺逐字复制 Rust API、trait、Drop 或所有平台扩展。Scoop 尚未发布；本表随实际实现直接修订，不保留旧草案签名兼容层，也不为未来平台穷举无实现的变体。窗口后端使用原生系统／协议库；全项目 Rust 依赖汇总和统一 staticlib 构建属于独立流水线工作，不在本表范围内。

## 1. 应用与窗口

参照：[ApplicationHandler](https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html)、[Window](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html)、[WindowAttributes](https://docs.rs/winit/0.30.13/winit/window/struct.WindowAttributes.html)。

| winit 能力 | Scoop 目标 | 范围与平台差异 |
| --- | --- | --- |
| EventLoop／ActiveEventLoop／run_app | Application／ActiveApplication／run | 本期通用；单次、主线程 |
| EventLoopProxy／user_event | ApplicationProxy.post | 本期；类型化闭包代替泛型 user-event 通道 |
| new_events／about_to_wait | ApplicationEvent.NewEvents／AboutToWait | 本期；有限批次，等待前重新检查队列 |
| resumed／suspended／exiting／memory_warning | ApplicationEvent 对应生命周期事件 | 本期模型；启动先 Resumed，不把失焦当 Suspended；只报告系统提供的 memory warning |
| ControlFlow | Wait／Poll／WaitUntil(TimeMark) | 本期；单调来源检查 |
| run_on_demand／pump_app_events／嵌入外部 loop | 专用平台接入 | 后续；首期不作跨平台承诺 |
| create_window／WindowId | createWindow／WindowId | 本期；同一 Application 可创建多个独立非模态窗口，按 id 路由和独立销毁；创建与可绘制配置分开 |
| owner／modality（Scoop 后续需求） | 窗口归属、窗口模态／应用模态、关闭与焦点恢复 | 已有[专项设计](../desktop-ui/WINDOWS-AND-DIALOGS.md)，待实施；当前没有公共 owner／modality／showModal 入口 |
| title／visible／resizable／decorations | setTitle／setVisible／setResizable／setDecorations | 本期；Win32 无装饰且 resizable 时保留 WS_THICKFRAME，完全自绘窗框的系统操作另行补充 |
| inner／outer size、position、resize increments | metrics、outerBounds、desktopPosition、requestSize／requestPosition／setResizeIncrements | 本期按能力；Wayland 无全局窗口位置 |
| min／max inner size | setSizeConstraints | 本期按能力；逻辑尺寸，GTK 当前不支持最大内容尺寸约束 |
| enabled buttons／window level | setEnabledButtons／setWindowLevel | 本期按能力，保留按钮集合与上下层级；置顶不建立模态输入限制 |
| minimize／maximize | requestMinimized／requestMaximized | 本期；请求不等于状态确认，未知状态为 None |
| borderless／exclusive fullscreen | 后续 requestFullscreen(Fullscreen) | 后续；系统全屏过渡、模式切换与恢复一起实施 |
| focus／attention | requestFocus／requestAttention | 本期按能力；不承诺绕过 OS 激活策略 |
| window menu／drag／drag resize | 后续 showWindowMenu／beginMove／beginResize | 后续；当前原生装饰仍支持用户拖动，程序化操作须保存有效输入上下文 |
| transparency／blur／content protection | 对应属性与设置方法 | 本期按能力；不把视觉提示当作安全保证 |
| theme／preferred theme | observedTheme／requestTheme | 本期；未知与系统跟随分开 |
| window icon | 后续 WindowIcon 与 setIcon | 后续；不把 macOS 应用图标冒充单窗口图标 |
| close on Drop | requestDestroy／Closing／Destroyed | 本期改用显式寿命；GC 不调用 GUI |
| raw-window-handle／raw-display-handle | NativeWindowLease 与类型化 native handle | 本期提供 AppKit／Win32／Gtk 载荷；Linux 经 Gtk 内容容器挂载 renderer，lease 不代表待配置时即可提交帧 |

上述已实现与待补能力详见[窗口规范第 4.1～4.3 节](../specs/SCOOP-WINDOW-SPEC.md#41-多窗口)和[第 12 节](../specs/SCOOP-WINDOW-SPEC.md#12-linux-wayland-与-x11-后端)。Linux 多窗口、配置与隐藏重显已接入 GTK；新菜单、窗框、模态与独立托盘 Cone 按[桌面能力设计](../desktop-ui/DESIGN.md)实施，不能把既有基础验收当作新增能力已完成。

## 2. 事件、输入与绘制

参照：[WindowEvent](https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html)、[KeyEvent](https://docs.rs/winit/0.30.13/winit/event/struct.KeyEvent.html)、[Ime](https://docs.rs/winit/0.30.13/winit/event/enum.Ime.html)。

| winit 能力 | Scoop 目标 | 范围与平台差异 |
| --- | --- | --- |
| CloseRequested／Destroyed | 同名窗口事件，补充 Closing | 本期，意图与销毁分离 |
| Resized／ScaleFactorChanged | GeometryChanged(WindowMetrics) | 本期，完整快照替代借出的 InnerSizeWriter |
| Moved／Focused／Occluded／ThemeChanged | 位置、焦点、遮挡、主题事件 | 本期；平台不知道的状态保持未知 |
| KeyboardInput／ModifiersChanged | KeyEvent／ModifiersChanged | 本期，物理键／逻辑键／位置／repeat／synthetic 分离 |
| KeyEvent.text 与 IME Commit | TextInputEvent.Edit 批次中的 Commit 操作 | 本期统一文字入口；同批删除、提交和组合更新只调用一次 handler |
| Ime Preedit／Enabled／Disabled | 批次内 Preedit 及 Enabled／Disabled 事件 | 本期；UTF-8 字节范围、选择活动端、candidate rect；取消用空 Preedit 删除当前组合，开关变化才发 Enabled／Disabled |
| IME allowed／cursor area、dead-key reset | setImeAllowed／setImeCursorArea、失焦撤销组合 | 原生 marked text、UTF-16／ACP 转 UTF-8；快照外操作不伪造文档坐标 |
| IME surrounding 文档与重转换 | publishTextInputState／publishTextInputCaret、显式 Range replacement | macOS NSTextInputClient 与 Windows TSF 文档存储共用 revision／origin 协议；purpose 不属于当前范围 |
| CursorEntered／Left／Moved／MouseInput | 指针事件与按钮 | 本期；坐标和设备来源明确 |
| MouseWheel | LineDelta／PixelDelta、phase／momentumPhase | 本期；行数与物理像素分开，保留原生滚动方向 |
| DeviceEvent、AxisMotion、raw motion 与监听策略 | 后续设备事件扩展 | 后续；不隐式打开全局监听，设备单位不冒充像素 |
| Touch／pressure | 后续 TouchEvent／可选 pressure | 后续；macOS 无触摸屏事件时不伪造 |
| Pinch／Pan／Rotation／DoubleTap／TouchpadPressure | 后续 NativeGestureEvent | 后续；不增加通用手势识别器 |
| cursor icon／visibility | CursorIcon／setCursorIcon／setCursorVisible | 本期；原生基础图标，隐藏状态在离开窗口和销毁时恢复 |
| custom cursor | 后续 CustomCursor | 后续；自定义 RGBA 图像／hotspot 需复制并明确寿命 |
| cursor grab／warp／hittest | 后续捕获与定位接口 | 后续；按平台实际约束实施 |
| HoveredFile／DroppedFile／HoveredFileCancelled | 后续 FileDrop Enter／Drop／Leave | 后续；Path／OsString 保留平台路径，不把任意 URI 强转路径 |
| RedrawRequested／request_redraw | 同名绘制请求 | 本期；合并、无递归、零 drawable 与隐藏节流，重新 configure 后恢复 |
| pre_present_notify | prePresentNotify | 本期；为提交前通知，不是 present／GPU fence |
| monitor／primary monitor | Monitor／monitors／primaryMonitor／currentMonitor | 本期快照；毫赫兹、可选字段、桌面坐标与 DPI |
| video modes | 后续 VideoMode | 后续，与模式控制和失效规则一起实施 |
| ActivationTokenDone、startup notification | 后续 Linux 平台扩展 | 在现有 GTK/GDK 后端按实际公开能力补充；OS token 不进入普通窗口身份或权限机制 |

## 3. 平台扩展和范围外能力

| 类别 | 决策 |
| --- | --- |
| macOS activation policy、透明标题栏／content view、应用标识 | `scoop.window.platform.macos` 普通选项；不把 NSWindow style mask 放到公共属性 |
| Linux backend／display 与平台参数 | 当前以 SCOOP_WINDOW_BACKEND 选择 GTK 的 Wayland／X11；其他平台参数按实际公开能力补充，不绕过 GTK 管理 surface |
| Windows owner HWND、taskbar／DWM、Win32 样式扩展 | 后续 `platform.windows`；TSF／COM 是 IME 内部实现，不作为原生对象扩展暴露 |
| 任意 native parent／child window 嵌入、run-on-demand | 后续专项，不能用 WindowId 假装支持外部句柄所有权 |
| Android／iOS／Web／orbital 的专用入口与扩展 | 范围外 |
| clipboard、菜单、文件对话框、托盘与上层绘图 | 剪贴板／IME 等按各库当前规范；菜单与文件面板在 window，托盘在独立 scoop.tray，UI 包装与 Skia 保持分层；新增范围见桌面能力设计 |

平台能力缺失与未实施功能必须区分。实现某一交付批次时，不能把尚未写完的分支永久返回 Unsupported，声称该平台没有能力。先以真实窗口、提交一帧、asyncio 和 lease 关闭完成端到端子集，再扩大能力；每批按实际范围交付并修订 API。当前骨架阶段完成与整个能力地图完成分别记录。
