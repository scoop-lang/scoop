# Scoop Taskbar 与 Dock 规范

本规范对应 [GUI 扩展设计](../desktop-ui/GUI-EXTENSIONS.md) G6。实现与平台验证范围记录在[实施进度](../desktop-ui/GUI-EXTENSIONS-PROGRESS.md)。

## 1. 归属与应用标识

普通 Cone `scoop.desktop` 依赖 window／platform／std，不依赖 UI、Skia 或 tray。它使用系统已经提供的应用／窗口图标、角标、菜单和进度能力。没有相应 API 的平台返回 Unsupported，不绘制替代角标／进度，不修改标题模拟状态。

ApplicationOptions.applicationId 为可选稳定应用标识；ActiveApplication.applicationId 提供原快照。非空值不能是空串或包含 NUL，平台自身的格式限制在原生应用创建时报告。标识在第一个窗口创建前应用：Windows 设置显式 AppUserModelID，Linux 设置 Wayland application id／X11 WM_CLASS；macOS 校验已有 bundle identifier，运行期选项不改写 Info.plist。裸程序没有 bundle 时可以保留选项值，不能据此声称通知部署已完成。发行配置的 bundle.identifier 应与该值相同。

窗口标题继续使用 Window.setTitle。应用显示名和默认图标来自 bundle、PE／快捷方式和 desktop 文件；miniwindow caption、thumbnail tooltip 和 badge text 是不同的原生语义，不合并成一个跨平台 caption setter。

## 2. 资源与返回值

`ActiveApplication.openDesktop(): DesktopApplication` 在 owner 线程打开当前应用唯一的活动桌面资源，重复打开抛 WindowException(ResourceBusy)。`DesktopApplication.attachWindow(window): DesktopWindow` 绑定同一应用的现有窗口，同一窗口的重复活动绑定同样报告 ResourceBusy。关闭后可重新绑定，不改变窗口所有权。

所有查询、设置和 close 均要求 owner 线程；资源关闭后的普通操作报告 InvalidState，close 幂等。NativeApplicationLease 负责应用退出，DesktopWindow 持有带关闭通知的 NativeWindowLease。窗口进入 Closing 时排队通知绑定先停止原生使用并释放 lease，不能因桌面状态阻止窗口销毁。已有不带通知的 native lease 合同不变。

设置函数返回 Window 的 RequestStatus：Applied 表示原生 API 接收，Requested 表示保留请求等待对应 Shell 按钮，Unsupported 表示没有该能力。三者均不承诺用户正在看见图标、菜单或状态；临时服务故障和原生错误继续使用 WindowException。不会为这些状态请求 Surface 帧或启动轮询定时器。

## 3. 应用级操作

DesktopApplication 提供 capabilities()、setIcon(DesktopImage?)、setBadgeText(String?)、setDockMenu(Menu?)、setJumpListTasks(Array<JumpListTask>)、attachWindow(Window) 和 close()。None 清除本资源的覆盖。应用能力记录包含 applicationIcon、badgeText、dockMenu、jumpListTasks；能力按当前应用模式返回，不隐式改变 activation policy。

macOS 普通应用使用 NSApplication.applicationIconImage、NSDockTile.badgeLabel 和现有 NSApplication delegate 的 applicationDockMenu。文字和数字角标均由 badgeLabel 原生显示；调用方可将数量转为文字。Windows／Linux 首批没有对应运行期应用图标和文字角标入口。

Dock 菜单复用 Menu 和 NativeMenuLease，不能同时附着于菜单栏或托盘。NSMenu 自己的动作、tracking 和模型更新规则仍有效；菜单正在 tracking 时拒绝替换。关闭菜单或 DesktopApplication 时先取消 tracking，再在原生 tracking 返回后释放借用。应用关闭清除当前角标、图标覆盖和 Dock 菜单。

Windows JumpListTask 是启动任务，包含 title、arguments: Array<String>、可选 description 和可选 JumpListIcon(path: Path, index: Int)。执行目标固定为当前应用可执行文件，参数按 Windows argv 规则引用；图标路径保留原生路径编码。任务使用 ICustomDestinationList／IShellLink 发布到应用标识对应的 Jump List，不保存闭包，不转换任意 Menu 树。空数组显式清除任务；close 不删除已经发布的任务。没有显式应用标识时报告 InvalidState。

Linux 的启动任务由 bundle.linux.actions 写入 desktop actions，运行期 setJumpListTasks 返回 Unsupported。

## 4. 窗口级操作

DesktopWindow 提供 capabilities()、setIcon(DesktopImage?)、setOverlayIcon(DesktopImage?, description: String = "")、setProgress(DesktopProgress)、setMiniwindowCaption(String?)、setThumbnailTooltip(String?) 和 close()。能力记录分别包含 windowIcon、overlayIcon、progress、miniwindowCaption、thumbnailTooltip。

DesktopProgress 为 None、Indeterminate、Normal(completed: ULong, total: ULong)、Paused(completed, total)、Error(completed, total)。三种有数值状态要求 total > 0 且 completed <= total；不把整数转换为百分比或浮点近似。该状态只映射 Windows ITaskbarList3。macOS 和 Linux 首批不提供进度模拟。

Windows 的窗口大小图标使用 WM_SETICON，overlay／progress／thumbnail tooltip 使用 ITaskbarList3。窗口库记录 TaskbarButtonCreated；绑定可读取已经发生的首次创建，也在之后的按钮重建消息中重新提交最新请求。首次消息前保留最新值并返回 Requested；相同未变化请求不重复调用 Shell。Explorer 重启无需应用轮询。DPI 改变时重新选择合适的图标表征。

macOS 的窗口图标对应 NSWindow.miniwindowImage，caption 对应 miniwindowTitle，二者都属于原生最小化窗口。Linux X11 使用 GDK toplevel icon list；Wayland 首批通过 desktop／application id 关联图标，动态图标返回 Unsupported。

DesktopImage 仍为不可变 CPU RGBA 数据。原生层复制／持有所需数据或图像对象，不借用 managed 数组的临时地址；Windows 替换前生成新 HICON，WM_SETICON 不得持有已释放的旧对象。多尺寸表征按当前原生图标尺寸选择，原生层处理 alpha 格式。清除和关闭释放自有图标、overlay、进度与专用文字覆盖，不恢复另一个控制者的历史状态，也不关闭业务窗口。

## 5. 必要验证

普通 Cone 测试覆盖资源重复附着、关闭通知、参数和 Unsupported、图像复制及多窗口隔离。平台实窗测试检查系统实际属性、菜单动作及取消、任务栏消息后重放；显示能力还须经过实际桌面呈现。Jump List 和 desktop actions 使用实际启动参数验证，格式转义由各平台打包测试覆盖。缺少某个 OS／Shell 的桌面环境时明确记录缺测，不以编译成功代替。
