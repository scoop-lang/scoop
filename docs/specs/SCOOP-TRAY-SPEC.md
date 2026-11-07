# Scoop 托盘规范

日期：2026-11-07。本文为[桌面能力设计](../desktop-ui/DESIGN.md) D3 的实施合同；实际完成和验证状态见[实施记录](../desktop-ui/PROGRESS.md)。窗口线程、桌面坐标和退出规则沿用[窗口规范](SCOOP-WINDOW-SPEC.md)，图像和菜单借用沿用[菜单规范](SCOOP-WINDOW-MENUS-SPEC.md)。

## 1. Cone 与资源归属

scoop.tray 是独立普通 Cone，单向依赖 scoop.window／std／platform。普通 window、Surface、UI 和通用组件不声明托盘类型或托盘依赖。scoop.ui.component.tray 随组件批次另行包装。

TrayIcon.create(application, options) 创建该 Application 的应用级资源；TrayOptions 包含 DesktopImage、可选 tooltip 与 Menu。TrayIconId、TrayMenuSessionId 为独立 Long 值类型，编号不复用。所有读取、修改、订阅与关闭在 owner 线程进行。最后一个业务窗口关闭不直接删除托盘，应用原有 LastWindowPolicy 仍有效。

TrayIcon 提供 setImage、setTooltip、setMenu、subscribe、capabilities、availability、geometry、showMenu、placeWindow 和 close。订阅返回 Closeable；退订或 TrayIcon 关闭后不交付排队的旧业务回调。业务回调异常沿应用已有失败与退出通路处理。close 幂等，先取消原生展示、解除菜单与系统注册，再归还应用资源；GC 不替代显式 close。

scoop.window.native 提供 registerNativeResource(requestClose)，返回 NativeApplicationLease，服务这个独立 Cone 的实际原生资源。lease 提供 owner 线程检查、业务 post、关闭控制 postControl、退出当前回调后执行原生操作的 postNative，以及同应用的菜单／窗口借用。业务投递复用现有 mailbox 和 asyncio attachment；关闭控制在业务队列停止后继续。应用退出通知每个资源一次，原生宿主等全部 lease 归还后才能销毁，不占用 asyncio 的单一 ApplicationAttachment。

## 2. 能力、事件与菜单

TrayCapabilities 区分模板图像、主／上下文／键盘激活、绑定菜单后的激活通知、图标矩形查询、事件锚点、宿主菜单、程序化菜单与邻近窗口定位。它描述协议能力；TrayAvailability 的 Available／Unavailable 描述当前注册宿主状态。注册或宿主接受图标不保证图标在主托盘栏中可见。

TrayEvent 包含 Activated、AvailabilityChanged、MenuCompleted、MenuBindingChanged(NativeMenuUseId?)。激活值区分 Primary／Context／Secondary，来源为 Pointer／Keyboard／Unknown；按钮、点击次数、锚点只在系统实际提供时存在。键盘激活不伪造鼠标位置。绑定 Menu 后使用平台的宿主菜单行为；各平台能同时提供的激活通知由能力值表达。

Menu 通过 NativeMenuLease 借用，同一 Menu 的其他使用按 ResourceBusy 处理。TrayIcon.menuUseId 提供当前 NativeMenuUseId，供应用把 ApplicationEvent.Menu 的 Native 来源关联到托盘；不再生成一份执行相同动作的 TrayEvent。Menu.close 只解除该托盘的菜单使用，图标本身可以继续存在。

setMenu 对相同菜单无副作用；原生菜单仍在 tracking 时报告 ResourceBusy，应先取消并等待展示结束。接受的替换取消尚未启动的旧展示。取消已经开始的程序化 session 会退休旧菜单使用，结束后重新借用仍有效的同一 Menu，避免旧动作落到下一次展示。

MenuBindingChanged 在实际附着、解绑或取消后的重新借用完成时交付，以当前 NativeMenuUseId（解绑为 None）标记结果；相同菜单的无操作更新不通知。它也覆盖宿主直接打开菜单后因 Menu.close 引发的延迟解绑，供组件等待旧使用退休并应用最新声明，不增加轮询。事件是变化时的快照，处理时仍可查询 menuUseId 确认当前使用。

支持程序化显示的后端上，showMenu 返回 TrayMenuSession；状态为待完成或 Selected(MenuItemId)、Cancelled、Failed(WindowException)。一次请求只记录一次终态；关闭、替换菜单与应用退出取消展示。MenuCompleted 通知用于展示状态，不重复执行条目动作。展示从 postNative 边界启动，原生 tracking 中继续处理现有 owner 和 I/O。Linux 首批只支持由宿主显示导出的菜单，程序化 showMenu 返回 Unsupported。

## 3. 几何与窗口定位

geometry 返回 TrayGeometry?，包含当前图标 DesktopRect 和可识别的 MonitorId。支持查询但当前没有位置时返回 None；不支持该查询时报告 Unsupported。TrayAnchor 区分图标矩形快照与宿主 PointHint，不把提示点包装成精确图标矩形。所有值是查询或事件发生时的快照，未知的可见性、面板范围和坐标不猜测。

Window 的 Monitor 增加 workArea: DesktopRect?。AppKit 使用 NSScreen.visibleFrame，Win32 使用 MONITORINFO.rcWork；X11 将当前虚拟桌面实际发布的 EWMH 工作区与显示器范围相交，未发布时为 None。Wayland 保持 None，不以输出矩形代替工作区。DesktopCapabilities.positioning 区分普通窗口全局定位、工作区查询、系统任务栏矩形和系统菜单栏高度。systemTaskbarBounds 仅描述 SHAppBarMessage(ABM_GETTASKBARPOS) 报告的系统任务栏，不声称枚举副屏或第三方 appbar；systemMenuBarHeight 返回 AppKit 提供的高度，不声称存在精确的每屏菜单栏矩形。

placeWindow 只定位同一 Application 内已有的 Window，返回 RequestStatus，不隐式创建、显示、激活或置顶。TrayWindowPlacement 选择方向、对齐、逻辑间距、是否翻转与平移、是否允许缩小；使用 PointHint 或完整显示器边界均需调用方明确允许。默认要求实际图标矩形与工作区。位置计算使用目标显示器的坐标单位和实际窗口外框；Windows 跨 DPI 移动后按同步得到的新外框重新对齐。不能满足尺寸约束或缺少所需锚点／工作区时报告相应错误。

Wayland 的普通 xdg_toplevel 不能任意全局定位，placeWindow 为 Unsupported；SNI 的提示点不改变这一限制。默认只在打开或明确重新定位时计算，不以轮询承诺永久跟随系统栏。

## 4. 平台实现与依赖

AppKit 使用 NSStatusBar／NSStatusItem.button，保留已有 activation policy。图像使用多分辨率 NSImage 和 template 提示。图标矩形由 button.bounds 经所属 NSWindow 转换到屏幕，再转换为 Window 的桌面坐标。绑定菜单采用 NSStatusItem.menu，程序化展示锚定实际按钮。

Win32 在同一 owner 线程使用托盘 Cone 自有的隐藏顶层 HWND，声明 shell32 依赖，接收 NOTIFYICON_VERSION_4 回调与 TaskbarCreated。图像复制为 HICON，菜单借用 HMENU。Explorer 恢复后只重新注册仍存活的图标及其最新内容；Shell_NotifyIconGetRect 查询矩形，事件中的锚点单独保存。C 回调仅排队，TrackPopupMenuEx 从原生操作边界启动，完成后按实际 command 编号查询 MenuItemId。

Linux GNU 在既有 GLib 主上下文导出 StatusNotifierItem，使用 GIO 的总线、名称观察和对象注册。DBusMenu 复用发行版维护的 libdbusmenu-glib 0.4，独立于 GTK3；菜单适配读取借用的 GMenuModel／GActionGroup。此依赖的头文件、链接输入、许可与准备入口归 scoop.tray。GLib 版 Ayatana 新接口导出的是 org.gtk.Menus／org.gtk.Actions，不能代替本合同的 com.canonical.dbusmenu。

Watcher／Host 消失时发布 Unavailable。Watcher 恢复时重新注册仍存活的图标；仅 Host 恢复时复用 Watcher 保留的注册，宿主读取当前导出的图像和菜单。异步回复只作用于发起请求时的服务状态，不得用旧回复覆盖后来的宿主变化。总线断连报告不可用，不自行建立重连轮询，不自动安装桌面扩展。SNI 没有图标矩形查询，只交付宿主实际提供的激活提示。图像 pixmap 使用协议要求的 ARGB 字节顺序；菜单图标、动作资格和标准角色仍来自 Menu 的当前原生描述。Linux musl 不交付当前 GUI／托盘资源。

依赖选择依据：[Debian libdbusmenu 维护包](https://sources.debian.org/src/libdbusmenu/)及当前 Ubuntu GNU 开发包；[libayatana-appindicator-glib 的协议说明](https://github.com/AyatanaIndicators/libayatana-appindicator-glib)用于确认协议边界。该选择不把旧上游发布日期误写为近期功能发布。
