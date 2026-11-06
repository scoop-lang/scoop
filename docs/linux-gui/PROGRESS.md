# Linux window、surface 与 graphics

## GTK4/libadwaita 迁移（2026-11-07）

当前验证分支为 `verify/gtk4-adwaita-20261107`，位于仓库外的独立 worktree；基线已包含远端 main。Linux GNU x86_64 的 Wayland、X11 后端改用 GTK4/libadwaita，musl 仍只提供基础库，不包含 GUI。用户主工作区的未提交改动及原有安装保留。

- 窗口使用 `AdwWindow`、`AdwToolbarView` 和 `AdwHeaderBar`，标题栏、按钮、圆角和阴影由 libadwaita 负责。旧 libdecor、XCB/Wayland 窗口实现及其协议文件已移除。
- 内容继续使用 Scoop UI 和 Skia Ganesh GLES，经 `GtkGLArea` 呈现。GTK 负责上下文与窗口目标，原生适配层负责借用、帧完成和资源生命周期；独立离屏 EGL 能力保留。
- GTK 事件源接入同一个 owner poll，GTK signal 只排队；业务 handler 在平台派发之外执行。渲染按需申请 frame clock，静止时不保留 tick callback。
- 输入、IME、剪贴板和显示器使用 GTK/GDK；X11 位置与窗口管理请求使用 Xlib。物理快捷键标签跟随当前键盘布局，剪贴板读取保留实际字节长度，继续拒绝正文中的 NUL。
- 普通 Window、Surface 和 UI API 签名不变。Linux 专属 `NativeWindowHandle` 改为 `Gtk(window, view)`；原生用户需要适配。GTK 不提供的最大尺寸、非默认 resize increments、单窗口强制主题、模糊与内容保护按规范返回不支持；Wayland 全局位置返回未知，两个后端的 primary monitor 都返回未知。
- macOS、Windows 的原生后端未修改。基线中的共享 UI 关闭修复仍保留：退出期间不再向关闭中的 root 排入状态更新；本次没有在 macOS、Windows 上运行验证。

最低运行依赖为 GTK 4.18、libadwaita 1.4。维护准备新增 `scripts/prepare_gtk.py`，通过 pkg-config 选择开发资源，将头文件、六个动态链接输入及许可放入 GNU sysroot；运行时使用系统 GTK/libadwaita。外部开发包通过指定代理下载至 `../scoop-deps` 并解压使用，没有安装或替换系统依赖。

## 工具链与发行准备

LLVM/Clang 为 22.1.8，五个 release 命令已重建。GNU sysroot 包含 38 个系统库，musl 包含 26 个系统库，两个目标均提供四种 runtime 配置。GUI 依赖变动后，window、graphics、surface、UI、component 及样式库已按依赖顺序重新准备，完整 shadcn 示例也已重建。

GTK 动态依赖需要 GNU C++ runtime，因此 `native.cxx` 增加目标条件表达，窗口 Cone 仅在 Linux GNU 下声明 C++ 需求。compiler protocol 更新为 27，package protocol 更新为 5；PM 清单和 registry 缓存分别使用 `manifests-v4`、`registry-v2`，旧解析记录自然失效。`.slib` 仍保存所选目标的 Boolean 需求，产物格式和 runtime ABI 不变。

已安装的 Linux GNU 工具链在未传 `--cc` 时优先选择同一安装目录中的 Clang，并配对使用 clang++；显式选择和 musl 默认行为保留。独立、带空格路径的安装目录中，带 `-Wmost` 的真实 C++ consumer 已在不指定编译器的情况下构建运行成功。五个命令、泛型示例、GNU 控制台、musl 静态与动态控制台均通过运行检查；公开 LLDB wrapper 可以加载随包 Python 与 Scoop 调试支持。

正式打包脚本已生成 bin、LLVM、内置库源码、GNU 库、musl 库和 LLVM 源码六个组件。解压后的 `post-install.sh` 默认模式和 `--all-targets` 均通过，GNU、musl 离线 API 文档以 `--deny-doc-warnings` 生成。安装后的默认编译器还构建了带原生 C++ 像素探针的 GTK/Skia consumer；该程序在两个软件显示后端和下文四个硬件组合中均完成三帧绘图、像素检查及正常关闭。

## 验证结果

下表只统计当前 GTK 实现的最终结果，不将旧 EGL 后端结果或失败后重试重复计数。软件显示环境为 Xvfb/Openbox 和嵌套 Weston，GLES 使用 Mesa 软件渲染，GTK critical 视为进程失败。

| 范围 | X11 | Wayland | 关键覆盖 |
| --- | --- | --- | --- |
| window | 17/17 | 17/17 | 多窗口、resize、关闭、native lease、快捷键、剪贴板 NUL |
| surface | 83/83 | 83/83 | 实际像素、帧调度、隐藏重显、resize、输入路由与资源关闭 |
| ui | 101/101 | 101/101 | 布局、状态、编辑策略、GPU 缓存与绘图 |
| 12 个专项 Cone | 12/12 | 12/12 | deadline、watch、asyncio、无窗口退出、初始焦点、错误清理、帧清屏与缓存复用 |

合计 30 组、426 次测试执行全部通过。初始焦点回归在修复前会超时：GTK 可能先于首次内容分配报告焦点，适配层现在在首次 GeometryChanged 后交付已获得的焦点，使 Surface 输入区域直接激活，无需用户重新聚焦。

独立双进程输入程序在两个后端均通过 Ctrl+A/C/X/V/Z/Shift+Z、运行中 US/德语布局切换、物理标签、AltGraph 正文、跨进程中文/emoji 剪贴板及正常关闭。完整 shadcn 示例已检查搜索输入、菜单与键盘子菜单、执行菜单项、按钮状态、主题切换、窗口缩放和原生关闭按钮。

配套检查：

- Rust 格式化、workspace clippy 和变更范围的后续 clippy 均通过；manifest 87 项、protocol 47 项、toolchain 68 项、PM 62 项测试通过。
- Native C++ 正式 CLI fixture 中，13 个适用的 GNU 用例和 1 个 musl 目标条件用例通过，覆盖真实 C/C++ 混合、异常运行库、目标选择与输入变化。
- Python 维护脚本测试 43 项通过、8 项跳过；Ruff 0.16.10 格式化和检查通过。GTK 开发资源准备及未变化输入的复用已实跑。
- 变更的 C/C++ 使用 Clang 严格警告检查，Scoop 源码已格式化。

## 本机硬件 GPU 与上游限制

本机环境为 GNOME Shell 50.1、GTK 4.22.4、libadwaita 1.9.1、Mesa 26.0.8；硬件为 Intel Iris Xe 和 Arc A770M。X11 在本次桌面上由 XWayland 提供。

| GTK/Skia 三帧绘图与像素回读 | Iris Xe | Arc A770M |
| --- | --- | --- |
| 原生 Wayland | 通过 | 通过 |
| X11/XWayland | 通过 | 通过 |

实际程序使用 Skia 绘制文字与红色矩形，逐帧回读矩形像素为 RGBA `255,0,0,255`，三帧完成后正常关闭。当前 GL 上下文的 `GL_RENDERER` 确认了对应硬件；没有软件渲染覆盖、依赖补丁或调试器干预。这只证明表中绘图用例，不代表所有硬件窗口行为均通过。

GNOME Wayland 下另有一项上游限制：两个窗口场景中，隐藏后重显并立即请求新尺寸，GtkGLArea 可能一直保持旧分配。已用只依赖 GTK/libadwaita、标准 `g_main_loop_run` 与 `g_idle_add` 的独立 C 程序复现，无 Scoop、Skia 或自定义事件循环；单窗口正常 resize 对照通过。当前完整硬件 Surface 测试在两块 GPU 的 Wayland 路径因此超时，X11 路径通过。按用户要求，不加入延迟重试或依赖修补，保留为 GTK/GNOME 路径的已知限制；尚未进一步确定 GTK 与 Mutter 中的具体归属及修复版本。

旧 EGL/XCB 后端在 2026-11-04 还确认过 Arc+X11 的 Mesa 驱动问题：DRI3 跨 GPU 呈现时，私有 TILE4 绘制资源迁移到 LINEAR 分配，缓存的 framebuffer 状态仍指向旧 BO，导致交换缓冲后清屏正常而矩形绘制丢失。只链接 EGL/GLESv2/XCB 的独立程序同样失败，GPU 命令解码和调试器对照验证了迁移原因。旧调查保存在仓库外 `scoop-deps/work/linux-current/hardware-gpu`；当前 GTK 路径通过上表用例，不表示已经修复该上游驱动问题。

## 验证边界

当前 worktree、独立安装目录、原始日志、截图和复现程序保存在仓库外 `scoop-deps/work/gtk4-adwaita-20261107`，不提交逐次报告。测试保留源码、工具和原产物，复用共享 Cone cache。

本次覆盖真实显示协议上的软件 GLES、合成键鼠输入及上述硬件组合，未覆盖独立 Xorg、实体键盘、原生 IME 候选窗、显示器热插拔或所有 Linux 桌面环境的人工验收。
