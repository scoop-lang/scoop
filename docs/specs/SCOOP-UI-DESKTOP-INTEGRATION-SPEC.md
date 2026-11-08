# Scoop UI 桌面集成规范

本规范落实 [GUI 扩展设计](../desktop-ui/GUI-EXTENSIONS.md) G8。独立 Cone `scoop.ui.component.desktop` 依赖 `scoop.desktop`、`scoop.ui.component`、`scoop.graphics.svg` 与已有基础库；UI 核心和普通组件不反向依赖桌面扩展。底层能力见 [桌面规范](SCOOP-DESKTOP-SPEC.md)和[通知规范](SCOOP-DESKTOP-NOTIFICATION-SPEC.md)。

## 1. 应用资源与声明

`ApplicationScope.desktopController(launch: NotificationLaunch? = None): NativeDesktopController` 在现有应用 scope 中创建并持有一个 DesktopApplication；提供启动资源时接管 NotificationCenter。controller.application 和 controller.notifications 只借用，生命周期由 controller 所属 scope 管理。创建失败关闭已取得的原生资源；关闭 controller 不关闭业务窗口、不撤回系统通知。

controller.isClosed 同时反映显式关闭和底层随 Application 退出而完成的关闭。底层先于 UI scope 退休时，条目停止写入已关闭的原生对象，随后卸载仍可幂等清理自身观察与菜单树。

`nativeDesktop(controller, key, configure): ApplicationItem` 使用 ApplicationElement 声明应用图标、badgeText、dockMenu 和 jumpListTasks。menu 在 render 中转换为既有 NativeMenuSnapshot，动作读取最新描述；Jump List 保持原生启动任务的独立 Array<JumpListTask>。同一 controller 同时只允许一个应用状态条目。jumpListTasks 的 None 表示不发布新任务，空数组明确清空；其他可选显示值的 None 清除本条目拥有的状态。卸载清除条目拥有的动态显示状态和 Dock 菜单，不删除持久的 Jump List 任务。

`nativeDesktopWindow(controller, window, key, configure): ApplicationItem` 明确绑定 Window，声明图标、overlayIcon／说明、progress、miniwindowCaption 和 thumbnailTooltip。不根据焦点猜测目标窗口。更换 Window 先释放原绑定，窗口关闭后停止写入，条目卸载只释放绑定；它不关闭 Window。业务窗口数量可以是零。

两种条目按实际值变化更新，重复 render 不重新注册资源、不重发相同进度或 Jump List。原生 Unsupported 保持不支持，包装层不绘制角标、进度条或替代菜单。描述重建不发送通知、不创建 Surface，不请求绘制帧。

## 2. Dock 菜单退休

Dock 菜单复用 NativeMenuTree。动作通过 ApplicationScope.observeMenu 和实际 NativeMenuUseId 路由。移除菜单先关闭树，立即停止旧动作并请求取消 tracking；等底层报告使用已经解绑后，才能绑定等待中的最新描述。期间不轮询、不重复调用会返回 ResourceBusy 的替换入口。

为这一实际调用增加 `DesktopApplication.observeMenuBinding(callback: (NativeMenuUseId?) -> Unit): Closeable`。设置或解绑实际菜单使用后，在现有应用队列交付新 ID／None；没有变化不发送，登记不重放当前状态。取消订阅或关闭 DesktopApplication 后不进入业务回调。底层仍独立完成 tracking 退出和 native lease 退休。

## 3. scope 中的通知操作

ApplicationScope 与 ComponentScope 均提供 `notificationController(center): NativeNotificationController`，借用已有 NotificationCenter，不创建第二个通知中心。它提供 settings、requestPermission、post、withdraw 与 observe，参数和 Result 类型沿底层；返回可提前取消的 Closeable。

操作完成后移除本地登记。回调进入创建它的 scope.dispatch，使用原有应用／窗口错误边界；显式取消、scope 卸载或 controller 关闭后，排队的结果和事件不再调用业务代码。关闭仅取消本 scope 的 operation／观察，不关闭借用的中心或撤回 OS 通知。发送和撤回只能由明确的业务动作调用，构造、render 和声明更新均不自动发送。

## 4. SVG 图标输入

`SvgAsset.toDesktopImage` 与 `SvgDocument.toDesktopImage` 接受正的有限 LogicalSize、明确的物理分辨率列表、SvgDrawOptions 和 template 标志。它们在 RasterSurface 上透明清屏、按每份分辨率绘制同一逻辑 SVG，再以 straight-alpha RGBA8 生成独立 DesktopImage。asset 重载仅在调用期间解析并关闭文档；document 重载借用文档。返回图像不依赖文档或 Surface 的寿命。

转换是显式 CPU 操作，不藏在 render、原生 DPI 回调或任务栏进度更新里。调用方复用生成的多分辨率图像；后端仍选择适合实际 DPI 的 representation。该转换只提供原生图标输入，不用于补齐 OS 缺失的桌面功能。

## 5. 验证

普通 Cone 验证图像颜色／透明度／多分辨率和关闭后的独立存活，以及无业务窗口的应用条目挂载、最新 Props、菜单移除与重新绑定、显式窗口切换、通知取消及 scope 退休。综合示例使用统一 applicationId／Bundle.toml，按实际能力显示操作，并与已安装的应用包一起检查桌面呈现和启动动作。未实际观察的平台行为在实施记录中明确列出。
