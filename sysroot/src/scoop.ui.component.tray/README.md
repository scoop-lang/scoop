# 原生托盘组件

`scoop.ui.component.tray` 提供 `nativeTrayIcon` 应用附属项。它使用 `ApplicationElement` 的挂载、Props、State 和错误边界，依赖 `scoop.tray` 和普通组件的 `NativeMenuContent`。普通 window、UI、component 不依赖本 Cone；Linux 的 libdbusmenu-glib 依赖由[底层托盘 Cone](../scoop.tray/README.md)单独提供。

在应用 Cone 中显式声明本库，并为实际导入的 ui、component、tray、window 添加普通依赖。在 `ApplicationComponent.render` 中生成附属描述：

```kotlin
class DesktopResources(val image: DesktopImage, scope: ApplicationScope): ApplicationComponent(scope) {
    private val enabled = state(true)
    val tray = NativeTrayHandle()

    public override fun render(): ApplicationContent {
        val checked = enabled.binding()
        val target = tray
        return ApplicationContent([
            nativeTrayIcon(image, ViewKey.Text("status")) {
                tooltip = Some("Scoop desktop")
                handle = Some(target)
                menu = Some(nativeMenu {
                    checkboxItem(ViewKey.Text("enabled"), "Enabled", checked)
                })
            }
        ])
    }
}
```

应用通过 `UiHost.attachApplication` 挂载此根，无需业务 Window、Surface 或默认字体。图片与 tooltip 按变化更新，Binding 在 render 生成菜单 snapshot 时读取；同 key 菜单项更新保留原生身份。`menu = None` 请求关闭旧菜单，等真实解绑后采用最新描述，不轮询，也不把空菜单视作解除菜单。

`NativeTrayHandle.icon` 借用当前 TrayIcon，`availability` 和 `isMenuPending` 可以被 render 观察。`requestMenu()` 明确打开支持程序化展示的原生菜单，`cancelMenu()` 请求取消；`onMenuClosed` 使用最新回调且只回报该组件主动打开的 session。Linux SNI 由宿主显示导出的菜单，程序化请求保持 Unsupported。借用 icon 可以查询 capabilities／geometry 或调用 placeWindow；调用者不关闭它、不替换组件拥有的图片和菜单。

激活事件保留真实 TrayAnchor：图标矩形、PointHint 或未知。Wayland 普通窗口全局定位仍不可用；组件不会猜测任务栏位置。菜单动作按真实 NativeMenuUseId 进入所属 ApplicationScope；卸载先退订，再取消展示并关闭托盘和菜单。宿主失去服务时保留声明，服务返回由底层恢复同一图标的当前内容。

完整 API 与生命周期见[托盘组件规范](../../../docs/specs/SCOOP-UI-TRAY-SPEC.md)，平台验证见[实施记录](../../../docs/desktop-ui/PROGRESS.md)。

[原生桌面综合 demo](../scoop.ui.component/examples/native-desktop/README.md) 展示共享菜单状态、CPU 图标更新、托盘锚点／工作区查询、邻近窗口的显式降级，以及最后窗口关闭后继续更新和重新打开文档。

测试使用正式 `scoop test sysroot/src/scoop.ui.component.tray --release`。macOS 需要 Window 后端可用的桌面会话；Linux GNU 需要 D-Bus 会话及 GTK 显示会话。测试覆盖无窗口更新、菜单移除／重新附着、handle 唯一归属、scope 错误隔离、取消和卸载，不把协议可用等同于 Shell 实际显示图标。Windows 默认只执行纯状态测试，完整 GUI 程序仍会编译；在交互桌面及 Explorer 中单独追加 `--ignored` 运行原生用例。
