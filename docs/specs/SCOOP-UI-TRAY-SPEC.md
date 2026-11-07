# UI 托盘组件规范

`scoop.ui.component.tray` 是独立 Cone，单向依赖 `scoop.ui.component`、`scoop.ui`、`scoop.tray` 和必要基础库。普通组件不导入托盘；Linux 额外 DBusMenu 链接与运行依赖仍只由底层 `scoop.tray` 带入。原生几何、宿主能力和退出规则遵守[托盘规范](SCOOP-TRAY-SPEC.md)。

## 1. 声明与归属

`nativeTrayIcon(image, key, configure = {}): ApplicationItem` 是 ApplicationElement，接受稳定的 ViewKey。NativeTrayIconBuilder 提供 tooltip = None、menu: NativeMenuContent? = None、handle: NativeTrayHandle? = None，以及 onActivated、onAvailabilityChanged、onMenuClosed，回调默认空。图片由调用者提供 DesktopImage，菜单 snapshot 在应用 render 内读取 Binding 并收集依赖；不需要 Window、Surface、字体或隐藏的业务窗口。

NativeTrayIconProps 保存 image、tooltip、menu: NativeMenuSnapshot?、handle 和三个回调。挂载创建一个 TrayIcon，更新只写变化的图像／tooltip，菜单按完整 key 路径协调同一个 NativeMenuTree。相同 key 的显示内容和回调更新保持原生身份，不因更新或宿主暂时消失重新注册整个组件。

## 2. 菜单与事件

菜单动作通过 scope.observeMenu 按实际 Menu 登记，并要求 MenuSource.Native 的 NativeMenuUseId 等于当前 TrayIcon.menuUseId。TrayEvent.MenuCompleted 只表示展示完成，不再执行一次动作。所有托盘订阅先进入所属 ApplicationScope.dispatch，回调使用最新 Props；不会覆盖 ApplicationHandler，也不会把应用错误伪装成窗口错误。

menu = None 关闭旧 NativeMenuTree，立即停止旧动作并请求取消原生 tracking；等 MenuBindingChanged 确认实际解绑后释放登记。等待期间只保留最新声明；如果又提供菜单，旧使用退休后创建并绑定新树。宿主菜单和程序化菜单采用相同退出规则，不以空菜单伪装解除菜单，也不轮询原生 busy 状态。

## 3. 显式控制与卸载

NativeTrayHandle 可选，且同一时间只能绑定一个挂载。icon: TrayIcon? 和 availability: TrayAvailability? 可被 render 观察；icon 只借用，调用者不得关闭它或替换组件拥有的菜单／图像。几何、能力查询及邻近窗口定位通过该 icon 的既有 API 进行，保留 Unsupported、未知锚点和 PointHint 的区别。

requestMenu(): Boolean 明确发起一次程序化展示；未挂载、无菜单或本次请求尚未完成时返回 false，原生不支持则报告真实错误。isMenuPending 保持到对应 TrayMenuSession 完成；cancelMenu 请求取消而不自动重开。onMenuClosed 只回报组件通过 handle 发起的展示，宿主直接显示菜单不虚构 session。菜单动作先沿其菜单事件执行，完成回调可以明确打开下一次。

卸载先解绑 handle 和注销业务事件，再取消 session、关闭 TrayIcon、关闭菜单树；底层按原生使用退休释放。AppRoot 失败或最后一个业务 Window 关闭不改变托盘的所属边界，Application 退出仍等待其资源。卸载后的延迟事件不交付给新挂载或旧回调。
