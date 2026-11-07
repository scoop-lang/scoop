# 原生窗框测试

此 Cone 的普通值测试不需要桌面。原生测试在同一个 Application／asyncio session 中切换 Client、System(Standard)、System(Extended)，保留菜单栏、Surface renderer 和 native 内容宿主，检查实际按钮几何、原生区域的空集／默认值、缩放后的撤销与关闭。窗口可绘制时，请求透明并改变内容尺寸后等待对应尺寸的 Surface Accepted，再恢复不透明；缺少合成器的 X11 允许透明请求返回 Unsupported。Accepted 表示提交成功，不代表已经显示到屏幕或等待 GPU 完成。

- macOS 检查实际 NSWindow／NSView、真实 Surface CAMetalLayer 的运行中 opaque 同步，并把真实 NSEvent 对象经过原生队列和异步业务派发后用于准备系统拖动。取消回调中退出，验证尚未运行的拖动使用能安全退休；这不替代在前台桌面实际拖动／缩放的交互验收。
- X11 测试需要 libXext、libXtst 开发资源，通过 X Shape 读取真实服务器输入区域，包含 GtkNative surface 与内容坐标的偏移；通过 XTest 向服务器注入实际指针事件，检查输入使用期和退出取消。Wayland 检查 GTK 实际模式、内容几何、区域设置与失效；实际 seat serial 的拖动交互须在带输入设备的 compositor 中验收。
- Windows 完整原生用例标记 Ignore，在 SSH 中编译，交互桌面使用 `scoop test sysroot/src/scoop.window/tests-cones/frame --release --ignored --nocapture` 单独运行。它检查 HWND 的实际客户区、GetWindowRgn、WM_NCHITTEST、DWM 颜色／圆角的能力与设置值、空集恢复与系统操作退休；无桌面时的普通值测试通过不代表这些 GUI 用例通过。

每个进程只能运行一个 Application；原生测试使用该进程唯一的实例。测试所需 sysroot、目标编译器和缓存目录按当前开发安装配置；原始日志保存于仓库外。
