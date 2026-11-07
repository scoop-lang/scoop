# 原生桌面组件组合测试

`scoop test sysroot/src/scoop.ui.component/tests-cones/native-desktop --release` 消费准备好的系统库。组件构建后运行同一套公共 API 测试；不为此复制 sysroot 或重新冷构建其他库。

用例覆盖 NativeMenuTree 的稳定身份、同级重排、快捷键交换、tracking 中移除、最新回调、拒绝／接受 Binding 写入和禁用子树；真实平台菜单查询与动作验证应用默认／窗口覆盖及其来源。窗口部分覆盖标题栏槽位保留、窗框切换、popup 的显式取消和 handle 替换、Owned 工具窗口、Owner／Application 模态、提前回复和旧回复隔离、四种文件面板控制器、重复窗框声明及 scope 失败清理。

macOS／Linux 的入口位于 test-desktop。Linux 需要 GTK4／libadwaita、Skia 所需运行环境与 D-Bus 会话，分别用 `SCOOP_WINDOW_BACKEND=x11 GDK_BACKEND=x11` 和 `SCOOP_WINDOW_BACKEND=wayland GDK_BACKEND=wayland` 验证。软件显示会话和程序化动作不替代具体桌面中的视觉叠放、系统按钮悬停、鼠标拖动或文件选择验收。

Windows 的完整原生用例位于 test-windows，以 Ignore 保留到有 GPU 的交互桌面；SSH 中正常编译全部程序并运行纯值测试。桌面单独执行 `scoop test ... --release --ignored`；三个 Windows 目标各自消费匹配 CRT 的预编译库和原生资源。

焦点范围与标准菜单编辑命令的纯行为覆盖位于 `scoop.surface` 的 `InputMenuCommandsTest`，托盘包装及额外依赖位于独立 `scoop.ui.component.tray`。
