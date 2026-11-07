# 原生归属与模态组合测试

本 Cone 使用 scoop.test 验证 Window 的真实原生归属与输入资格。macOS、Linux 使用可创建原生窗口的会话；Linux 分别设置 SCOOP_WINDOW_BACKEND=x11／wayland 和对应 GDK_BACKEND。Linux native 查询直接读取 GtkWindow transient、窗口组与 sensitive，X11 Tool 查询实际 UTILITY 属性。

默认运行两项纯值测试和一项桌面组合：两个独立主窗口、Owned 工具窗、Owner 模态、Application 模态、嵌套与新建窗口、原有禁用状态恢复、关闭按钮资格、待展示取消、隐藏恢复以及父窗口等待子窗口 native lease。嵌套 Application 模态中执行真实 loopback TCP 传输和 asyncio 定时器。AppKit 同时验证普通 sheet 与嵌套 sheet，不调用 critical sheet。Windows 原生查询使用 HWND owner、WS_EX_TOOLWINDOW 和 IsWindowEnabled。

准备当前 sysroot 后，从仓库根目录执行：

```sh
scoop test sysroot/src/scoop.window/tests-cones/modal --release --sysroot sysroot
```

Windows SSH 会话默认只执行纯值用例，完整 GUI 用例一并编译。用户在已登录的交互桌面以 --ignored 单独执行 GUI 用例；分别指定实际的 --target，LLVM-MinGW 还需要 --native-sysroot 指向对应工具链。不能把默认测试通过记录为 Windows GUI 验收通过。

```powershell
scoop test sysroot/src/scoop.window/tests-cones/modal --release --sysroot sysroot --ignored
```

原生属性与软件显示会话验证不能代替具体桌面的视觉、交互焦点和窗口管理器叠放验收。AppKit 的隐藏 child 会暂时脱离 NSWindow.parentWindow，测试仅在实际显示／sheet 状态检查原生关系，逻辑 ownerWindow 始终保持。
