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

本机 `~/.scoop` 已同步命令、内置库源码、原生资源、系统 `.slib` 和 runtime；五个命令的代码版本为 `35fd1e9d7`。安装后使用配套命令与系统 GCC 消费预编译 GUI 库，在 X11、Wayland 上显示中文／emoji、连续呈现三帧并正常关闭；musl 控制台程序也编译、运行成功。两个目标的离线 API 文档均以 `--deny-doc-warnings` 生成通过，入口为 `~/.scoop/docs/index.html`。原安装备份保存在仓库外的 `scoop-deps/work/linux-current/installation-backup`。

链接 Skia 所需的 ELF 修正允许显式对象定义覆盖系统 C++ runtime 的动态 compiler builtin 导出，保留对象间重复定义诊断，并以 C++ 异常组合 fixture 验证。

## 验证

库测试与实际程序均使用 release。下表为此前在独立 Xvfb／Openbox 与嵌套 Weston 上完成的验证，GLES 使用 Mesa 软件渲染；本机硬件补验单独记录在后文。

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
| 完整主题示例 | X11、Wayland 各 3／3 | 主题／明暗切换、模型保留、隐藏重显和静止停帧 |
| 下载示例 | X11、Wayland 各 5／5 | 真实 HTTP、后台 SHA-256、取消、隐藏期间完成、重显及关闭收尾 |

新 Surface 产物及其全部系统库依赖已重建，完整主题、下载示例、最后的连续帧回归及安装后的消费检查均已通过。

独立交互程序在 X11 与 Wayland 上均通过真实显示协议输入，覆盖 Ctrl+A／C／X／V／Z／Shift+Z、运行中 US／German 布局切换、物理键标签、AltGraph 正文和跨进程 Unicode 剪贴板。Weston kiosk shell 完全遮挡剪贴板 owner 的场景也已通过，两个进程均正常退出。修复前协议与系统调用跟踪确认了约 5 秒的 EGL 内部等待；修复后使用本方 frame callback 节流并保留静止后的未完成 callback，窗口事件循环仍能响应剪贴板请求。

配套检查：

- workspace `cargo fmt --all`、`cargo clippy --workspace --all-targets --offline` 通过；Scoop 源码格式化、变更的 C／C++ 严格警告检查通过。
- `scoop-linker`、`scoop-manifest`、`scoop-toolchain` 的 release Rust 测试合计 175 项通过。
- 正式 CLI native-link fixture：55 项通过、24 项不适用，执行 56 个变体、277 个进程，检查 18 个 golden；包括 system-cxx-builtins。
- 维护脚本 Python 测试 42 项，跳过 6 项；fixture runner 公共规则测试 82 项，跳过 4 项，均无失败。相关 Python 变更使用 Ruff 0.16.10 格式化和检查。

合并 main 后发现的 Fontconfig 通用字体族缺口已经补齐；协程调用方已迁移到 receiver API。主题示例先等待实际动画稳定，再严格检查静止和隐藏期间帧数不变。多个大型消费者同时编译曾触发宿主 OOM；改为串行构建后，系统库和完整示例均成功，未改动编译器的语言、产物或缓存规则。

安装后的严格文档检查还发现 HTTP README 指向仓库测试脚本的相对链接无法解析。发行组装现将 `tests/` 引用与既有 `docs/`、`scripts/` 引用一起重定位到源码 revision 的 GitHub 链接；打包回归保留库内源码的相对链接，实际 GNU／musl 文档生成均通过。

### 本机硬件 GPU 补验（2026-11-04）

在本机 GNOME Shell 50.1 会话中，分别指定 Intel Iris Xe（Alder Lake-P GT2，PCI `8086:46a6`）与 Intel Arc A770M（DG2，PCI `8086:5690`），运行原生 Wayland 和 X11 后端；本次 X11 服务由 XWayland 提供。内核为 `7.0.0-42-generic`，两块 GPU 均使用 i915，用户态为 Mesa `26.0.8-1ubuntu0.3`、EGL 1.5、OpenGL ES 3.2。

运行时未设置软件渲染覆盖，分别以 `DRI_PRIME=pci-0000_00_02_0` 和 `DRI_PRIME=pci-0000_03_00_0` 选择设备。仓库外的 EGL 观测代码从被测程序当前上下文读取 `GL_RENDERER`：分别得到 `Mesa Intel(R) Iris(R) Xe Graphics (ADL GT2)` 与 `Mesa Intel(R) Arc(tm) A770M Graphics (DG2)`；窗口 display 的 EGL 设备查询分别对应 `/dev/dri/renderD128` 与 `/dev/dri/renderD129`。

| 范围 | Iris Xe | Arc A770M |
| --- | --- | --- |
| 已安装工具链 GUI consumer | Wayland、X11 均成功提交三帧并正常关闭 | Wayland、X11 均成功提交三帧并正常关闭 |
| graphics，含 GLES 离屏像素与资源生命周期 | 40／40 | 40／40 |
| ui，含 GLES 缓存像素 | 87／87 | 87／87 |
| surface，Wayland | 77／77 | 77／77 |
| surface，X11 | 77／77 | 77／77 |
| ui-frames，Wayland | 1／1 | 1／1 |
| ui-frames，X11 | 1／1 | **0／1，上游 Mesa 驱动已知问题** |

上述 Cone 回归直接运行已有 release 测试程序，合计 566 次测试执行，565 次通过、1 次失败；另有四次已安装 GUI consumer 检查。Surface 覆盖双窗口像素、连续帧、静止、隐藏重显、resize、重附着和关闭。测试进程均确认使用指定的硬件渲染器，未观察到 EGL 交换缓冲失败；成功提交帧不替代像素断言。

上游驱动故障最初表现为 Arc A770M＋X11 的透明 UI 窗口重绘：`ui-frames/tests/frames.scoop` 的 alpha 断言期望 255、实际为 0。开启观测和移除观测各复现五次，均失败。后续独立复现确认范围包括不透明窗口：第一次交换缓冲后，清屏仍正确，但普通矩形绘制失效。

### Arc＋X11 原因调查（2026-11-04）

原因已定位到当前 Mesa Iris 驱动的缓冲迁移与绘制状态缓存不同步。独立 C 程序只链接 EGL、GLESv2 和 XCB，在默认 framebuffer 上清屏、画矩形、读回并交换缓冲；不依赖 Scoop 或 Skia，仍复现相同故障。32 位透明和 24 位不透明窗口均失败，Iris Xe 的相同用例均通过。窗口尺寸从 160×80 到 640×480 的多组对照结果一致。GL framebuffer 完整、无 GL 错误，遮挡查询确认绘制产生片元；X11 对测试窗口的像素读取也确认呈现内容缺失。

证据链如下：

- 当前桌面由 Iris Xe 驱动，Arc 的 X11 客户端走 DRI3 跨 GPU 呈现。Mesa 为其创建私有绘制 image，再复制到可共享的线性呈现缓冲。
- Mesa `26.0.8` 的 `iris_flush_resource` 会通过 `iris_reallocate_resource_inplace` 将私有绘制资源迁移到可共享分配，更新资源的 BO 和布局。Gallium 跟踪显示，后续复用同一资源时没有重新设置 framebuffer 状态。
- `INTEL_DEBUG=bat` 的 GPU 命令解码直接显示：第一帧的绘制和读回使用相同的 TILE4 地址；迁移后，第二帧矩形的渲染目标仍使用旧 TILE4 地址，读回已使用新的 LINEAR 地址。这解释了清屏正常而矩形消失的现象。
- 在 GDB 中仅改变测试进程的初始 image 分配，为私有绘制 image 加上 `__DRI_IMAGE_USE_SHARE`，避免后续迁移：独立 C 程序从四处像素失败变为六帧全部通过，原始 Scoop `ui-frames` 用例也从第 82 行失败变为通过。同一调试器下不改变参数的对照仍失败。
- 另一组独立对照保持原分配方式，在不同的完整 framebuffer 上实际绘制后切回窗口，六帧也通过；仅切换空 framebuffer、重设 viewport／program／顶点或关闭混合无效。新的 XWayland 连接同一 GNOME 会话仍复现；Weston 对照中观察到更频繁的缓冲重分配，说明呈现调度会影响复用问题是否出现。

该问题按已确认的上游依赖缺陷处理，不纳入 Scoop 的修复范围，不在项目中加入针对性的绕过逻辑，也不修改或替换系统 Mesa。上表保留原始 Arc＋X11 失败，调试器干预后的通过仅作为诊断证据。尚未验证哪个上游版本已修复；当前已通过的硬件路径为 Arc＋原生 Wayland，以及 Iris Xe＋Wayland／X11。独立复现源码、运行说明、GDB 对照脚本及原始跟踪保存在仓库外 `scoop-deps/work/linux-current/hardware-gpu`。

## 验证边界

原始日志、协议跟踪、截图和临时交互程序保存在仓库外 `/home/chenxu/repos/scoop-deps`，不提交逐次报告。测试保留源码、工具与原产物，复用共享 Cone cache。

已验证真实显示协议上的软件 GLES、合成键鼠输入，以及上文列出的本机硬件组合；Arc＋X11 的缓冲复用故障保留为上游 Mesa 的已知限制，不作为 Scoop 的待修复项。本次没有覆盖独立 Xorg 会话，也未据此宣称实体键盘、原生 IME 候选窗、显示器热插拔或所有桌面合成器通过人工验收。
