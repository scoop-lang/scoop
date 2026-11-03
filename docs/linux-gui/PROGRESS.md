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

窗口组合补验已闭合默认不透明区域与 libdecor 独立事件 fd：区域随 configure／resize／setTransparent 更新，装饰栏输入与内容输入共享 owner 的 poll 等待。扩展后的 window 测试仍在两后端各 13／13；X11 从服务器查询透明区域及尺寸，Wayland 以实际合成像素验证不透明／透明切换，并通过装饰栏按钮正常关闭。公开 UI consumer 还在两种后端验证了真实显示协议路由的合成键鼠、编辑选区／撤销／提交、按钮激活和跨进程 Unicode 剪贴板。

graphics／surface 阶段已完成：Linux 使用 Skia Ganesh GLES／EGL，文本使用 Fontconfig／FreeType。首次 configure 后创建实际尺寸的 EGL surface；resize 排空旧目标的 GPU 使用并重建 EGL surface，避免 Wayland 首帧及 X11 resize 的旧 backbuffer。GPU context 在窗口 lease 结束前 drain／abandon，剩余 GPU 资源操作明确失败，close 仍安全。按需帧在隐藏和 idle 时停止，PendingConfigure 保留内容及 dirty。验证结果：

- `scoop.graphics`：37／37，包含真实 GLES 离屏绘图、GPU 资源生命周期与 CPU 快照。
- `scoop.graphics.text`：49／49，包含 Fontconfig 字体和共享文字布局合同。
- `scoop.surface`：X11、Wayland、Wayland 2 倍缩放各 70／70；真实多窗口像素读回覆盖首次配置、resize、隐藏重显、idle、重附着及关闭，Linux Primary 为 Control。
- `surface-errors`：X11、Wayland 各 1／1；绘制抛错时关闭可见与未配置窗口，并保留主错误及多个 cleanup 错误。
- 工具链打包脚本：5／5；Linux GNU 包含 Skia archive／headers，musl 保持原目标范围。Python 使用 Ruff 0.16.10 格式化和检查；原生 C／C++ 完成格式化与警告检查。

上述呈现测试通过真实显示协议，GPU 为 Mesa 软件渲染，未据此宣称硬件 GPU、真实 IME 或所有合成器验收通过。

链接 Skia 时发现显式对象的 compiler builtin 与系统 C++ runtime 动态导出冲突，已修正 ELF 符号选择并补充 C++ 异常组合 fixture。`scoop-linker` 22／22，使用 fixture 默认 GCC 的原生链接验收 45 项通过、19 项不适用；workspace fmt／clippy 通过。五个 release 工具命令已重建并安装至 `~/.scoop/bin`。

UI 正在完成组合验收，结果在对应功能提交时更新。

原始构建和测试日志保存在仓库外 `/home/chenxu/repos/scoop-deps/logs`。本记录只保存验证范围、结果、失败闭合与实际限制。
