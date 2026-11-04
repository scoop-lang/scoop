# Linux window、surface 与 graphics

实现分支：`feat/linux-window-surface-ui`。交付目标为 Linux GNU x86_64，同时提供原生 Wayland 和 X11，遵循现有 macOS 的公共 API 与资源生命周期合同。musl 保持基础库目标范围，不包含 GUI。

## 实现范围

- window：窗口配置、输入、IME／文档快照、剪贴板、显示器与 DPI、native lease、deadline、watch 和 window.asyncio。Wayland 的 libdecor 事件与内容输入使用同一个 owner poll；不透明区域随 configure、resize 和透明设置更新。协议不提供的全局位置等能力返回未知或不支持。
- 键盘与编辑：复用 main 中归属 window 的 `ShortcutChord`、`textCommands` 和文档编辑操作。Linux Primary 为 Control；物理快捷键标签使用当前 XKB 布局；AltGraph 保留正文输入，不触发普通 Ctrl／Alt 快捷键。剪贴板归一化换行，拒绝写入 NUL，读取带 NUL 的正文返回不支持。
- graphics／graphics.text：Skia Ganesh GLES／EGL、Fontconfig／FreeType，复用共享绘图、HarfBuzz shaping 与段落布局。sans-serif、serif、monospace 通过 Fontconfig 配置解析。
- surface：首个 configure 后建立实际尺寸的 EGL surface；resize 完成旧目标的 GPU 使用后重建 backbuffer。GPU context 在借用显示连接结束前 drain／abandon；剩余 GPU 资源的操作明确失败，close 保持安全幂等。Wayland 使用本方 frame callback 和 swap interval 0 控制呈现，静止时保留尚未返回的 callback，显式隐藏、重新配置和关闭时取消。X11 使用 EGL swap interval 控制连续帧。
- 消费者：共享 UI、文本编辑、控件、样式和三套主题接入 Linux GNU；window.asyncio 和下载示例使用新版协程 receiver 与窗口 executor。Surface 基础示例也可直接在 Linux 构建。

## 工具链

使用 `/home/chenxu/repos/scoop-deps` 中已有的 LLVM 22.1.8、Skia 和其他依赖，全程离线构建，没有下载外部依赖。五个 release 命令 `scoop`、`scoopc`、`scoop-link`、`scoop-lsp`、`scoop-pm` 已重建。目标清单包含 37 个 GNU 系统库、26 个 musl 系统库，以及每个目标的四种 runtime 配置；musl 构建使用已有的 UAPI 编译器 wrapper。

链接 Skia 所需的 ELF 修正允许显式对象定义覆盖系统 C++ runtime 的动态 compiler builtin 导出，保留对象间重复定义诊断，并以 C++ 异常组合 fixture 验证。

## 验证

以下均使用 release。窗口测试运行在独立 Xvfb／Openbox 与嵌套 Weston 上，GLES 使用 Mesa 软件渲染。

| 范围 | 结果 | 关键覆盖 |
| --- | --- | --- |
| window | X11、Wayland 各 17／17 | 配置、resize、隐藏重显、多窗口、跨线程唤醒、lease、编辑命令和快捷键标签 |
| window deadline、watch | 每个 Cone 两后端各 1／1 | 无窗口等待、到期、eventfd、重复移除及回调期间注销 |
| window.asyncio、无窗口退出 | 每个 Cone 两后端各 1／1 | executor、io_uring、定时器、取消、lease 与异步 finally |
| graphics | 40／40 | GLES 离屏绘图、GPU 资源生命周期和 CPU 快照 |
| graphics.text | 52／52 | Fontconfig 字体、通用字体族与共享文字布局 |
| surface | X11、Wayland、Wayland 2 倍缩放各 77／77 | 真实帧与像素、连续失效、静止、隐藏重显、resize、重附着和关闭 |
| surface-errors | 两后端各 1／1 | 可见／未配置窗口的失败清理及主错误保留 |
| Surface 基础示例 | 4／4，独立可执行程序构建通过 | 拖拽取消、字素边界、组合文本与布局复用 |
| ui | X11、Wayland 各 87／87 | 布局、状态、文本编辑策略、输入与 GLES 缓存像素 |
| ui-frames | X11、Wayland、Wayland 2 倍缩放各 1／1 | 移动和移除内容后的 alpha，避免旧帧残影 |
| ui.component、ui.style | 53／53、19／19 | 控件、布局基础与样式 |
| 三主题共享测试、ui-dsl | 17／17、1／1 | 主题合同和独立 DSL consumer |

新 Surface 产物及其全部系统库依赖已重建，当前继续完成完整主题、下载示例与安装的最终组合验收。

独立交互程序在 X11 与 Wayland 上均通过真实显示协议输入，覆盖 Ctrl+A／C／X／V／Z／Shift+Z、运行中 US／German 布局切换、物理键标签、AltGraph 正文和跨进程 Unicode 剪贴板。Weston kiosk shell 完全遮挡剪贴板 owner 的场景也已通过，两个进程均正常退出。修复前协议与系统调用跟踪确认了约 5 秒的 EGL 内部等待；修复后使用本方 frame callback 节流并保留静止后的未完成 callback，窗口事件循环仍能响应剪贴板请求。

配套检查：

- workspace `cargo fmt --all`、`cargo clippy --workspace --all-targets --offline` 通过；Scoop 源码格式化、变更的 C／C++ 严格警告检查通过。
- `scoop-linker`、`scoop-manifest`、`scoop-toolchain` 的 release Rust 测试合计 175 项通过。
- 正式 CLI native-link fixture：55 项通过、24 项不适用，执行 56 个变体、277 个进程，检查 18 个 golden；包括 system-cxx-builtins。
- 维护脚本 Python 测试 42 项，跳过 6 项；fixture runner 公共规则测试 82 项，跳过 4 项，均无失败。相关 Python 变更使用 Ruff 0.16.10 格式化和检查。

合并 main 后发现的 Fontconfig 通用字体族缺口已经补齐；协程调用方已迁移到 receiver API。主题示例先等待实际动画稳定，再严格检查静止和隐藏期间帧数不变。多个大型消费者同时编译曾触发宿主 OOM，后续大型构建串行执行；这不改变编译器的语言、产物或缓存规则。

## 验证边界

原始日志、协议跟踪、截图和临时交互程序保存在仓库外 `/home/chenxu/repos/scoop-deps`，不提交逐次报告。测试保留源码、工具与原产物，复用共享 Cone cache。

已验证真实显示协议上的软件 GLES 与合成键鼠输入；未据此宣称硬件 GPU、实体键盘、原生 IME 候选窗、显示器热插拔或所有桌面合成器通过人工验收。
