# scoop.desktop

提供系统原生的 Taskbar／Dock 状态，依赖 `scoop.window`、platform 和 std，不依赖 UI、Skia 或 tray。应用通过 `ActiveApplication.openDesktop()` 取得唯一活动资源，窗口通过 `desktop.attachWindow(window)` 单独绑定。

应用级接口包括图标、Dock 文字角标、Dock 菜单及 Windows Jump List 启动任务。窗口级接口包括图标、overlay、任务栏进度、miniwindow caption 和 thumbnail tooltip。窗口标题仍使用 `Window.setTitle`；安装后的显示名称和默认图标由 bundle、快捷方式或 desktop 文件提供。

| 功能 | macOS | Windows | Linux GNU |
| --- | --- | --- | --- |
| 应用图标／文字角标 | NSApplication／NSDockTile | 无对应入口 | 无对应入口 |
| 窗口图标 | miniwindowImage | WM_SETICON | X11 icon list；Wayland 使用 desktop id |
| 菜单／启动任务 | 原生 Dock Menu | Jump List tasks | 安装期 desktop actions |
| overlay／进度 | 无对应入口 | ITaskbarList3 | 无对应入口 |
| 专用文字 | miniwindowTitle | thumbnail tooltip | 无对应入口 |

先查询 `capabilities()`。没有原生能力时 setter 返回 `RequestStatus.Unsupported`；不会画一套替代进度条、角标或菜单。`Applied` 表示原生 API 接收，`Requested` 表示等待任务栏按钮创建，均不承诺 Shell 正在显示该状态。

`ApplicationOptions.applicationId` 应与发行配置 `bundle.identifier` 相同，并在创建第一个窗口前设置。Jump List 要求显式应用标识，任务目标固定为当前应用，参数使用独立字符串数组。Linux 启动任务配置在 `[[bundle.linux.actions]]`，由 scoop-bundle 写入 desktop 文件。

所有操作和关闭都在应用 owner 线程进行。绑定关闭会清除自己设置的覆盖，并释放原生图标／菜单借用；它不关闭业务窗口。业务窗口关闭时自动释放对应绑定。`DesktopImage` 使用不可变 CPU RGBA 表征，调用方不需要保持临时字节指针或图形设备存活。菜单复用 `scoop.window.Menu` 的动作路由与 tracking 合同。

完整合同见 [Taskbar 与 Dock 规范](../../../docs/specs/SCOOP-DESKTOP-SPEC.md)，实施与平台验收见 [GUI 扩展进度](../../../docs/desktop-ui/GUI-EXTENSIONS-PROGRESS.md)。
