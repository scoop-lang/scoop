# Linux window、surface 与 UI

实现分支：`feat/linux-window-surface-ui`。目标为 Linux GNU x86_64，同时提供原生 Wayland 和 X11，参照现有 macOS 的公共合同。用户要求每项功能验证后提交、合并远端 main 并推送。

## 实施范围

1. 构建现有安装的系统 `.slib`；复用 `~/.scoop/bin` 的工具和本地依赖。
2. window：X11／Wayland 窗口、输入、IME／文档快照、剪贴板、显示器与 DPI、native lease、deadline、watch 及 window.asyncio。
3. graphics／surface：Linux Skia Ganesh GLES／EGL 和 Fontconfig／FreeType；同一绘图／文本实现驱动两种窗口，按需帧、隐藏／重显、resize、异常与关闭。
4. UI：共享框架、文本编辑、Linux 快捷键、控件与三套主题；以所属 Cone 测试和真实窗口示例验收。

## 当前状态

安装目录中原有的 25 个 Linux GNU 系统库已构建成功。

window 阶段已完成原生 X11／Wayland 接入、文档输入、剪贴板、借用句柄、owner loop 和 window.asyncio。原生 C 按项目警告选项构建，Scoop 源码已格式化；release 验证结果：

- `scoop.window`：X11、Wayland 各 13／13。包括真实窗口创建、configure、resize、隐藏重显、多窗口、跨线程唤醒和 lease 延迟销毁；X11 另以 X server 路由的合成事件验证键鼠和文本。
- `scoop.window.asyncio`：两后端各 1／1，覆盖 io_uring、定时器、取消和 lease 清理。
- `scoop.window.asyncio/tests-cones/no-window`：两后端各 1／1，覆盖无窗口退出时取消 UDP 接收并等待异步 finally。

测试使用独立 Xvfb／Openbox 和嵌套 Weston，未操作用户桌面。原生 IME 候选窗、实体键盘布局切换、显示器热插拔和桌面合成器间差异尚未进行人工验收；协议不提供的全局位置等能力明确返回未知或不支持。

graphics／surface 和 UI 正在接入，后续验收结果在对应功能提交时更新。

原始构建和测试日志保存在仓库外 `/home/chenxu/repos/scoop-deps/logs`。本记录只保存验证范围、结果、失败闭合与实际限制。
