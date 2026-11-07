# 原生文件对话框测试

本 Cone 的桌面组合从真实的 openFile／openFiles／saveFile／selectDirectory 入口展示原生面板，检查 Owner 与 Application 的输入范围、两个独立 owner、窗口模态内的文件面板、新建窗口、显式／用户取消、owner 关闭和无窗口应用。文件面板存活期间执行真实 loopback TCP 与 asyncio 定时器；取消外层应用模态后再结束内层文件面板，验证两个原生循环均能退出。

Linux 测试还通过 GtkFileChooser 的公共选择接口触发接受，检查单选／多选／保存／目录的类型化 Path 结果、取消不能覆盖已完成结果，以及保存接受不创建文件。多选用例确认多选模式并接受一个文件；多个文件的实际选择、Shell 呈现和编辑行为继续由交互桌面验证。AppKit 与 Win32 的自动组合使用展示和取消，不把自动取消记录为真实选择／编辑验收通过。

准备当前 sysroot 后，从仓库根目录执行：

```sh
scoop test sysroot/src/scoop.window/tests-cones/dialogs --release --sysroot sysroot
```

Linux 分别配置 SCOOP_WINDOW_BACKEND=x11／wayland 和对应 GDK_BACKEND，在有相应 display／seat 的会话中运行。本地选择器测试需要 `GDK_DEBUG=no-portals`。真实 portal 另行运行，避免把独立进程中的选择器当作应用内 GtkWindow 查询：

```sh
GDK_DEBUG=portals scoop test sysroot/src/scoop.window/tests-cones/dialogs --release --sysroot sysroot --ignored --filter NativePortalTests
```

该用例观察 GTK 实际连接的 FileChooser 请求、系统 portal 返回的 request handle 和 Close；四种入口均在受理后执行真实 TCP，再取消并等待 session 退休，最后覆盖无窗口 Application 目标。原生观察器不替换服务或生成选择结果。需要已安装且可用的 xdg-desktop-portal 及桌面后端；协议受理和取消不等同于跨进程面板的前台选择／视觉验收。验证结果见[实施记录](../../../../../docs/desktop-ui/PROGRESS.md)。

原生路径的独立 ABI 测试位于所属 Window Cone，可以在没有 GUI 的进程中运行。它从 C 返回包含非法 UTF-8 字节或 Windows 非配对 surrogate 的路径，经过实际对话框编码／解码函数后，再由 C 核对原始字节和长度。

```sh
scoop test sysroot/src/scoop.window --release --sysroot sysroot --filter FileDialogPathTests
```

Windows SSH 只执行非 GUI 用例并完整编译桌面组合。在已登录桌面运行以下入口；分别指定实际 --target，LLVM-MinGW 另传 --native-sysroot。

```powershell
scoop test sysroot/src/scoop.window/tests-cones/dialogs --release --sysroot sysroot --ignored
```

测试创建自己的临时目录，并在退出后删除。文件面板的创建、路径转换和资源关闭均使用当前公共库入口，不替换生产实现的 native 结果。
