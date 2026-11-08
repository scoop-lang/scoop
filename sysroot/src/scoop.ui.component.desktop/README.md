# UI 桌面适配

`scoop.ui.component.desktop` 把 `scoop.desktop` 接入既有应用挂载与组件 scope。普通组件库不反向依赖这个 Cone；原生菜单、窗口和绘制继续使用各自的 owner。

```scoop
val controller = scope.desktopController(Some(notificationLaunch))
val notifications = scope.notificationController(controller.notifications!!)
```

`notificationLaunch` 来自 main 中、Application 创建前调用的 `prepareNotifications(applicationId)`；`desktopController()` 也可仅创建 Taskbar／Dock 控制器。控制器由 ApplicationScope 关闭，其公开的 `application` 与 `notifications` 为借用对象。

在 ApplicationComponent 的 render 中返回 `nativeDesktop(controller, key) { ... }`，可描述应用图标、文字角标、Dock 原生菜单及 Windows Jump List tasks。每个 controller 同时只有一个应用状态条目。`nativeDesktopWindow(controller, window, key) { ... }` 显式绑定业务 Window，描述窗口图标、overlay、进度和平台专用 caption；卸载只清理桌面绑定，业务窗口保持打开。能力查询由底层 `capabilities()` 提供；缺失功能遵循原生 Unsupported 结果。

菜单沿用 NativeMenuSnapshot 和 NativeMenuTree，动作在应用事件路径执行。撤下 tracking 中的菜单时先取消并等待实际解绑，再应用最新描述。没有用轮询或额外渲染帧等待原生菜单退休。相同描述不会重复发布 Jump List；`None` 不发布，空数组显式清空。卸载不会删掉已经发布的持久任务。

`ApplicationScope.notificationController(center)` 与 `ComponentScope.notificationController(center)` 借用通知中心。settings、requestPermission、post、withdraw 及 observe 都返回可关闭的本地操作／订阅；scope 卸载会关闭未完成项，回调异常进入该 scope 的错误边界。发送只放在显式动作中，不能放进 render。控制器关闭不撤回系统通知，也不关闭借用的通知中心。

`SvgAsset.toDesktopImage` 和 `SvgDocument.toDesktopImage` 接受逻辑尺寸、明确的像素尺寸数组、SvgDrawOptions 与 template 标志。调用时用 Skia RasterSurface 生成独立的 RGBA 图像表示，之后复用结果，不在每帧转码。Asset 入口自行关闭解析文档，Document 入口借用调用者文档。

完整示例见 [native-desktop](../scoop.ui.component/examples/native-desktop/README.md)，详细合同见 [UI 桌面适配规范](../../../docs/specs/SCOOP-UI-DESKTOP-INTEGRATION-SPEC.md)。
