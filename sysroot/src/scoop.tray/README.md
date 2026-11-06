# scoop.tray

独立的原生托盘 Cone，依赖 `scoop.window` 的应用、菜单和桌面坐标合同。API 与平台边界见[托盘规范](../../../docs/specs/SCOOP-TRAY-SPEC.md)。普通窗口与 UI Cone 不依赖本 Cone。

Linux GNU 需要 libdbusmenu-glib 0.4（Debian／Ubuntu 的 `libdbusmenu-glib-dev`），使用 GIO 导出 StatusNotifierItem 与 DBusMenu；运行时使用系统 `libdbusmenu-glib.so.4`。依赖准备与打包入口见[脚本说明](../../../scripts/README.md)。没有宿主时状态为 `Unavailable`，宿主恢复后重新注册当前图像和菜单；不会安装桌面扩展。Linux musl 不提供当前 GUI／托盘后端。

`geometry()` 查询当前实际图标矩形；macOS／Windows 在暂时无法定位时返回 `None`，Linux 明确报告 `Unsupported`。激活事件的 `PointHint` 只代表宿主传来的提示，窗口定位须显式允许使用提示。`placeWindow()` 默认保留尺寸并依据目标屏幕工作区翻转、滑动；缩小窗口、使用完整屏幕代替工作区也分别需要显式选择。

## 验证

使用 `scoop test sysroot/src/scoop.tray --release --sysroot sysroot` 运行所属 Cone 测试，开发树可按环境补充 `--scoopc`、`--cc` 和独立 target/cache/lockfile。

- macOS：默认覆盖无窗口应用的状态栏资源、图像、回调与菜单租约关闭，以及通用定位策略。真实状态栏菜单取消／选择在解锁的桌面以 `--ignored` 单独运行。
- Linux：需要运行中的 X11 或 Wayland 会话，并在 `dbus-run-session -- ...` 创建的独立会话总线中执行。测试宿主使用实际 SNI／DBusMenu 协议验证图像、菜单动作、更新、服务恢复和关闭中的回调；X11 验证提示位置旁的窗口请求，Wayland 验证能力拒绝。测试宿主不修改用户桌面配置。
- Windows：默认运行纯定位测试；完整原生用例编译进测试程序，在有 Explorer 的交互桌面以 `--ignored` 单独运行。该用例删除并重新注册自己的通知图标以验证 `TaskbarCreated` 处理，不重启 Explorer。

一个进程只创建一次 Application；需要桌面的忽略用例应与默认测试分开执行。
