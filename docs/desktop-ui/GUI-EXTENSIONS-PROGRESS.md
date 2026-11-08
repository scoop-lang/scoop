# GUI 扩展实施进度

实施依据：[SVG、字体与桌面集成设计](GUI-EXTENSIONS.md)。功能分支为 `codex/gui-extensions`。每个完整批次定向验证后提交、推送，再合并 `origin/main`；本地完成大项后集中进行其他平台验证，原始日志和临时产物放在仓库外。

| 批次 | 状态 | 验证范围 |
| --- | --- | --- |
| G1 Skia SVG 与独立图形库 | 已实现，全部 GUI 目标验证通过 | macOS 双架构各 14 项含真实 Metal；Linux GNU／Windows 三目标各 13 项；19 项脚本测试 |
| G2 Lucide、SvgView／Icon | 已实现，全部 GUI 目标资源验证通过 | 各目标的 31 个图标；macOS 的 4 项组件与实窗测试；既有退出错误已在 G4 闭合 |
| G3 字体目录、字面与变体 | 已实现，全部 GUI 目标验证通过 | macOS 双架构、Linux GNU 的定向字体测试；Windows 三目标各 59 项文本测试，含新增目录／变体用例 |
| G4 字体更新与选择示例 | 已实现，三平台验证通过 | 静止／隐藏窗口、独立读者、原生 preedit、选择器与完整 demo；退出错误闭合；Windows 三目标选择器均已复验 |
| G5 Windows 圆形窗口 | 已修复，Windows 11 三目标实窗通过 | 默认 demo 路径、HWND style／region、resize、最小化恢复及既有窗框／输入回归 |
| G6 Taskbar／Dock | 已实现，三平台定向测试通过 | 应用标识、原生图标与文字、菜单 tracking、窗口解绑、任务栏重放；集成呈现随 G8 验收 |
| G7 桌面通知 | 已实现，三平台定向验证通过 | macOS 实际点击冷启动；Linux 服务提交／动作；Windows NSIS／MSI 安装、替换／撤回、COM 冷启动与卸载 |
| G8 集成与发行 | 已完成，三宿主安装验收通过 | 全部 GUI 目标实窗通过；macOS／Linux／Windows 完整工具链解包安装、文档与新消费者通过 |

## G1：独立 SVG 图形库

新增 [SVG 规范](../specs/SCOOP-GRAPHICS-SVG-SPEC.md) 和 `scoop.graphics.svg`，固定 Skia 已启用 SVG／skresources，并静态合并其 DEPS 指定的 Expat。公开 SvgDocument、SvgAsset、SvgDrawOptions 和 Canvas.drawSvg；尺寸、根样式恢复、整体透明度及线程／关闭合同均有真实像素和资源测试。

2026-11-08，本机 macOS aarch64 的 14 项 release Scoop 测试全部通过，包括 2 倍 DPI 的真实 Metal 目标、非零 viewBox、百分比尺寸、渐变／clip／blur、连续颜色／线宽以及文档关闭后的 Drawing。首轮基础图形测试漏算 polyline 的 miter 尖角延伸，修正测试几何范围后全部通过，绘制实现无需补偿。

Scoop 格式化／类型检查、C++ 格式化／编译检查和 Python Ruff 通过；相关准备／打包脚本 19 项测试通过。补齐 GUI 包的 Skia／Expat 静态依赖许可收集，预编译库按普通 sysroot 方式发布。Linux／Windows MSVC 的后续验证结果见 G2／G3；G8 收尾补验两套 MinGW，均通过 13 项 SVG 测试和全部 31 个 Lucide 图标资源测试。

## G2：Lucide 与 UI 图标

固定引入 Lucide 1.70.0 的 31 个原始 SVG，维护清单、生成脚本及完整上游许可一并保存。独立资源 Cone 不依赖 UI，普通应用构建不联网。新增借用文档的 SvgView、按挂载管理文档的 Icon，以及 ActionContentStyle.icon 成员入口；现有 Spinner、导航、菜单、侧栏、日期、关闭、搜索和提示图标迁移到相同资产。

本机 macOS aarch64 release 已通过全部 31 个资源的解析／真实像素绘制，以及 4 项组件测试（尺寸、固定线宽、资源替换与关闭、builder）；相关许可收集的 6 项 Python 测试通过。新增实窗测试通过，验证主题切换、尺寸／来源更新、RTL 下普通图标不镜像、零尺寸、卸载及借用资源保持有效。Scoop 格式化／编译、Python Ruff 通过，预编译组件库已更新。

2026-11-08 的真实离屏消费者分别引用 search 和全部 31 个图标，均成功运行；当时最终可执行文件均为 19,931,032 字节，包含 Skia 和 runtime。检查最终数据段发现两者都保留了 31 份 SVG，原始资源合计 12,159 字节。这是 immortal 按需选择实现前的基线，不把整个可执行文件大小解释为图标的新增体积；独立入口不建立运行时全量枚举或启动解析。

2026-11-09，[静态资源裁剪](../program-performance/IMMORTAL-PRUNING.md)完成实现和六个 GUI 目标的实际验收。重建被测系统库后，Darwin 双架构、Linux GNU 与 Windows 三环境的零／单／全引用只保留 0／1／31 份 SVG，未使用的对象、getter 和两类登记缺席；增加第 32 个未使用图标仍只留下 search。各目标均通过全部 Lucide 资源像素测试；Darwin 各通过 SVG 14 项含 Metal，其余目标各通过 13 项。Darwin x86_64 经 Rosetta 运行；本次裁剪工作的发行打包按用户要求取消，完整范围与检查脚本问题的闭合见该项实施记录。

现有 frame-reuse 实窗测试完成了 15 帧、动态图形变化及零额外布局的断言，但退出时失败于 `input router is closed`。使用改动前的预编译组件库复现相同错误：关闭输入路由后的终结失焦事件仍触发 TextEditHost 的菜单命令更新。该既有生命周期问题随 G4 的 UI 字体／编辑状态改动一起修复并复验，避免为同一 UI 依赖重复构建；此处不将整项测试记为通过。

Linux nuc12 的独立 checkout 完成固定 Skia 构建，release SVG 13 项与 Lucide 全部 31 图标的资源测试通过。Linux UI 实窗与字体更新大项合并验证。Windows 独立 checkout 的 Skia 构建发现 PATH 中 python3 启动器已失效；准备脚本改为向 GN 显式传入正在运行的 Python，不修改用户全局环境。

## G3：字体目录与变体

FontCatalog／FontFace 现可枚举和精确打开系统字面，从内存或文件读取 TTF／OTF／TTC／OTC，查询命名实例和可变轴，按坐标创建独立字面。FontCollection 支持实际字面登记、别名整体替换和变更订阅。来源关闭不影响已有字面或段落，失败替换不发布 revision。

macOS aarch64 release 的 10 项字体测试通过，覆盖系统目录独立存活、跨目录 ID、TTC 五个真实家族、静态字体、双轴局部覆盖、隐藏轴、命名实例的实际像素变化、线程／关闭、旧段落快照和观察者异常／取消。固定 Skia 的 CoreText 工厂不接受非零 TTC 索引，CoreText 描述符列表又包含命名实例；后续字面现由 HarfBuzz face builder 提取后交给 Skia，避免将实例索引误当集合索引。未加入另一套字体解析器或系统字体安装。

Scoop 格式化／编译、C++ 格式化／定向编译检查、Python Ruff 通过，相关脚本 19 项测试通过。提交后，Linux x86_64 与 Windows MSVC UCRT 分别通过相同的 10 项 release 字体测试；Windows 也完成 SVG 13 项和 Lucide 31 图标验证。macOS x86_64 在 Rosetta 下通过 SVG 14 项（含 Metal）、Lucide 31 图标与字体 10 项。G8 收尾时，两套 MinGW 也分别通过包含新增字体用例的全部 59 项文本测试。

## G4：字体变更与选择示例

FontWatch 对实际使用者订阅 FontCollection，通过既有失效和帧请求合并唤醒窗口，最后一个读者退休后立即取消订阅。帧入口不再检查字体集合 revision；绘制缓存自己的有效性判断继续保留它。默认集合替换沿原有信号，隐藏窗口保留失效，字体目录查询不会触发字体变更。

原生桌面示例新增系统家族搜索、字面选择、TTF／OTF／TTC／OTC 文件打开、命名实例及自定义轴滑杆。hidden 轴显式展开；预览使用可整体替换的独立别名和同一个 TextEditorState。模型测试覆盖 TTC 后续字面、实例／局部轴、失败加载保留旧选择、旧字面释放及编辑内容／选区保持。

macOS aarch64 release 的 mount-fonts、font-wakeup、frame-reuse 三个独立窗口 Cone 通过。font-wakeup 通过真实 NSTextInputClient 发送 preedit，验证字体变化前后的组合范围、选区、文本与修订不变；同时验证静止唤醒、未使用集合的窗口不重排、隐藏恢复和卸载取消。该原生 preedit 场景目前限定 macOS，跨平台默认字体／挂载用例已开启 Windows 与 Linux。frame-reuse 的输入路由退出错误已通过关闭区域后停止菜单命令更新修复，稳定布局与完整退出一起通过。

完整 native-desktop smoke 通过，包括双窗口、菜单、窗框、异形／工具窗口、模态、文件面板取消、无窗口运行和再次打开。Scoop 格式化／编译通过，UI 及依赖预编译库已更新；Linux／Windows 实窗随该批次提交后集中验证。

Linux 的 mount-fonts 与 frame-demand 在 nuc12 解锁后的真实 GNOME Wayland 桌面通过；同一原实现也通过独立 Weston GLES 合成器和 Xvfb X11 的窗口验证。frame-demand 复用既有连续丢弃／隐藏恢复用例并开启 Linux GNU。首次运行时 GNOME 锁屏停止了 frame-clock 交付，造成测试等待帧超时；解锁后无需修改 renderer 即完成。本批未保留诊断日志代码或试探性的 GTK 调度改动。Xvfb 使用软件图形，不将其记为硬件桌面视觉验证。

Windows 11 登录桌面 Session 1 的 mount-fonts 已通过。选择器组合测试暴露了固定 Skia DirectWrite 的 clone 合同：传入的集合索引与原生字面不一致时会忽略变体。Windows 后端现在从实际 SkTypeface 流取得索引，系统字体也不依赖公开来源索引。命名实例测试已扩展到 TTC 的全部五个家族，验证坐标、来源不变及实际粗体像素；macOS 定向测试与 Windows 全部 59 项文本测试通过。G8 更新整条 UI 依赖后，Windows 三目标的选择器和综合 demo 均在 Session 1 复验通过。

## G5：Windows 圆形示例

默认窗口现在启用实际 window／input shape，并将没有提供边缘缩放操作的圆形示例设为不可缩放；保留仅视觉裁剪的显式对照选项。smoke 不再提前改动形状选择，和普通打开路径使用同一默认值。本批只调整 demo，没有删除普通 Client 窗口的缩放能力或改写 DirectComposition。

Windows 11 的真实登录桌面 Session 1 上，MSVC UCRT 的独立测试通过：实际调用 DocumentView.showShape，检查 HWND 不含 WS_CAPTION／WS_THICKFRAME；原生区域的中心、边中点在内，四角在外；390×390 初始窗口、480×480 调整、最小化／恢复后均正确，关闭完成 Destroyed。既有 frame 测试同时通过窗框模式切换、区域、DWM 属性和系统按钮／输入退役。G8 收尾时，两套 MinGW 也分别通过相同圆形窗口测试与完整 demo smoke。macOS 完整 smoke 通过；Windows 10 桌面不可用。

## G6：Taskbar／Dock 与应用标识

新增 `scoop.desktop` 和 [桌面规范](../specs/SCOOP-DESKTOP-SPEC.md)。ApplicationOptions.applicationId 在首个窗口前设置 Windows AppUserModelID、Wayland application id／X11 WM_CLASS，并与 macOS bundle identifier 校验。应用和窗口分别持有桌面资源；带关闭通知的 native window lease 使窗口关闭能够先解绑状态，不会被额外借用阻止销毁。

macOS 使用 NSApplication 图标、NSDockTile 文字角标、miniwindow 图标／标题与既有 delegate 的 Dock Menu；Windows 使用 WM_SETICON、ITaskbarList3 和 Jump List tasks；Linux X11 使用原生 icon list，Wayland 使用应用标识。缺失能力返回 Unsupported。Windows 保留尚未创建任务栏按钮的请求，按钮重建后重放，并在 DPI 改变时生成合适的自有 HICON；有数值进度始终保留 ULong。

macOS、Linux 真实 Wayland 与 Xvfb X11、Windows 11 Session 1 MSVC UCRT 均通过两个 release Cone 测试。它们检查重复绑定、实际 OS 图标／应用标识、窗口隔离、业务窗口保持打开及关闭幂等。macOS 另覆盖原生菜单动作、tracking 时禁止替换，以及取消后释放；Windows 通过发送 TaskbarButtonCreated 验证重放路径，没有实际重启用户的 Explorer。Linux desktop actions 的三个打包单元测试验证顺序、参数引用、Unicode、百分号、空参数与配置错误；Rust fmt／clippy 通过。

G8 补验了综合示例的 UI 生命周期、安装后的默认图标与显示名称，以及 Linux desktop actions 的实际启动参数。macOS Dock 本身的视觉效果、Windows Jump List 的人工点击和 Explorer 的实际重启仍未验证；不以编译或属性检查代替这些结果。原始日志和临时测试报告保留在仓库外。

## G7：系统桌面通知

新增 [通知规范](../specs/SCOOP-DESKTOP-NOTIFICATION-SPEC.md)、`scoop.desktop.notification` 和独立 notifications 示例。Application 创建前准备原生激活入口，Resumed 后由 DesktopApplication 接管；设置查询不触发授权。操作、事件及启动响应在应用线程交付，取消或关闭停止本地回调，已经交给系统的通知继续由 OS 管理。相同 ID 支持替换，撤回结果不承诺通知从未被用户看到。

macOS 使用 UserNotifications，Linux 使用真实会话的 freedesktop 通知服务，Windows 使用 WinRT ABI 与 COM 激活入口。macOS 的两个值测试和一个独立生命周期测试、Linux 与 Windows 登录桌面的生命周期测试均通过。实际 `.app` 经用户授权后完成提交、替换、退出；用户点击 “Open & 你好” 后冷启动并打开窗口，动作和 `payload "<>&" 你好` 原文完整。Linux GNOME 会话完成提交、同 ID 替换、默认动作与关闭事件。

Windows 11 Session 1 的 NSIS 和 MSI 都实际安装到带空格的路径；开始菜单快捷方式的 AppUserModelID／ToastActivatorCLSID、注册表和带引号的 LocalServer32 均已读回核实。两个安装形态均完成发送、替换、撤回；通过系统 COM 激活入口启动已经退出的示例，具名动作与 Unicode payload 完整。两个卸载器都移除了自身身份、激活入口和快捷方式。这里未把 COM 调用记为人工点击 toast 的视觉验收。

实际安装发现并修复两个问题：NSIS 不能调用 SDK 中仅为 inline 的 InitPropVariantFromString，现用系统 SHStrDupW 构造真实 PROPVARIANT；Windows 首次 Show 前可能没有通知设置记录，只有明确的 ERROR_NOT_FOUND 按系统默认允许提交处理，其余错误继续报告。Windows 通知准备与 Application 共享既有 DPI 初始化，避免冷启动消息窗口抢先固定 DPI 模式。Rust fmt／clippy 及四项 Windows 打包单元测试通过，Scoop 与 native 源码格式化通过。G8 收尾时，MinGW UCRT／MSVCRT 也分别完成 Session 1 的 NSIS 安装、发送／替换／撤回、COM 冷启动、Unicode 动作与参数、卸载验证。

## G8：UI 适配与综合示例

新增 `scoop.ui.component.desktop` 和 [UI 桌面集成规范](../specs/SCOOP-UI-DESKTOP-INTEGRATION-SPEC.md)。ApplicationScope 管理桌面资源，应用条目和显式窗口条目按描述变化更新；通知控制器借用中心，将回调送入原有 scope 错误边界。SVG 可显式生成多分辨率、straight-alpha 原生图标。Dock 菜单通过实际使用解绑事件退休并重新挂载，不轮询。Application 先退休原生资源时，UI 条目停止写入，随后幂等卸载。

macOS aarch64、x86_64 和 Linux GNU 各通过三项 UI 桌面测试，覆盖 SVG 图像、取消、菜单更换、窗口切换、错误边界与完整退出。新 Cone 的离线 API 文档生成通过，未产生文档警告。macOS 与 Linux 的 native-desktop 完整 smoke、字体选择器测试通过。综合示例统一 applicationId 和 Bundle.toml，提供 Lucide 图标、主题变化、系统与临时字体、命名实例／轴坐标，以及按原生能力启用的桌面与通知操作。

macOS 实际运行生成的应用包，验证图标与主题切换、系统字体预览、加载 Variable.ttf、选择 Heavy 命名实例及查看隐藏轴；预览说明保持使用正常 UI 字体。Linux 生成 Debian 包并在带空格路径解包，通过系统 DesktopAppInfo 执行其 NewDocument 动作，确认空格、中文、百分号与引号参数完整。Linux 通知也补验了同 ID 替换与撤回。当前桌面工具读取 Dock 原生进程超时，macOS Dock 本身的视觉效果仍未确认。

LLVM-MinGW 的通知后端补齐缺失的标准 COM 激活声明，并局部修正 WIDL 将 WinRT Boolean 与 Byte 混成同一 C++ 类型的问题；两套 CRT 的完整 GUI 库均编译通过。Windows 三目标各通过 Session 1 的 10 项桌面／通知／UI／字体选择器／圆形窗口测试，以及各自的完整 demo smoke；测试事件确认全部完成且无失败。两套 MinGW 另各通过 SVG 13 项、Lucide 全部 31 图标和文本 59 项测试。

综合示例的 Windows NSIS／MSI 已生成。macOS 与 Linux 的完整发行包也已生成并解包到带空格的新安装目录；五个宿主工具、bundle／example 命令、LLVM、LLDB／私有 Python、全部目标的离线文档，以及使用安装后系统库的新消费者均通过。消费者实际包含原生 C 调用、SVG／Lucide 像素、系统字体、临时字体命名实例与轴、排版、通知入口；Linux musl 验证基础库消费。

Windows 安装检查发现 NSIS 快捷方式没有引用已打包图标，现为开始菜单和桌面快捷方式显式设置图标路径，未指定图标时保持主程序默认行为。Rust fmt／clippy 与四项定向打包测试通过。修复后的综合示例重新安装到带空格目录，实际读回身份、启动路径和图标，Session 1 的完整 smoke 及卸载通过。三个宿主的 bin 组件均已刷新，已安装的 macOS／Linux bundle 命令入口复验通过。

完整工具链打包发现 Linux 的 LLDB 共享库没有记录配套非系统 Python 的运行库路径，仅选择解释器仍无法找到 libpython。打包现在从所选解释器的 sysconfig 取得 library 目录，按选定 LLVM、选定 Python、已有 LD_LIBRARY_PATH 的顺序解析依赖，再重定位到安装包。Ruff 与 macOS／Linux 的相关脚本测试通过；新增真实 ELF 用例验证无需环境覆盖也能找到共享运行库，安装目录移动后仍可执行。

安装检查还发现 Homebrew Python 的 ELF interpreter 保留了构建机绝对路径。打包将附带 executable 的 interpreter 统一到 Scoop 宿主命令的系统 loader；已有正确路径和共享库不额外修改。真实 ELF 测试保留原程序、原共享库与自定义 loader，同时验证加工后的 helper 使用系统 loader 并在移动后的安装目录运行。Linux 的九项 native 打包测试通过八项，跳过仅适用 macOS 的一项。

## 最终发行验收

2026-11-08，三宿主均完成 `0.0.0-gui.20261108` 的完整工具链组件包。编译器和系统库的源码修订为 `f1547582fa`；三个宿主的 bin 组件包含 `7de2307fe9` 的 bundle 图标修复，Linux LLVM 组件包含 `7296a95281`／`2d55e7db14` 的 Python 运行库与 interpreter 修复。

| 宿主 | 组件归档 | 随包目标资源 | 安装验收 |
| --- | --- | --- | --- |
| macOS Apple Silicon | 8 个 | aarch64、x86_64 各 26 个基础 Cone + 18 个 GUI Cone | 全部通过；x86_64 使用 Rosetta |
| Linux x86_64 | 7 个 | GNU：26 个基础 Cone + 18 个 GUI Cone；musl：26 个基础 Cone | 全部通过；GNU 消费者和 musl 基础程序均实际运行 |
| Windows x86_64 | 10 个 | MSVC UCRT、LLVM-MinGW UCRT／MSVCRT 各 26 个基础 Cone + 18 个 GUI Cone | 全部通过；三个 GUI 消费者在 Windows 11 Session 1 运行 |

三个宿主均从归档解包到独立、带空格的安装路径，验证五个宿主工具、bundle／example、LLVM 22.1.8、LLDB／随包 Python、所有已安装目标的离线文档。Windows 补验了同一安装目录的 `post-install.ps1 --all-targets` 入口和完整总索引；三目标的新消费者均使用安装后的工具链和系统库编译，运行原生 C、SVG／Lucide 像素、系统字体、临时可变字体、命名实例／轴、排版和通知准备入口。

Windows 初次完整文档检查超过了外部验收脚本的 1200 秒超时；各目标完成后复用原缓存，正式安装入口通过，无需修改产品脚本。SSH 非交互会话中的通知 DPI 初始化返回错误码 5，入口前移后仍复现；相同可执行文件在登录桌面通过，最终三个消费者均在 Session 1 验收，没有为非交互会话放宽窗口 DPI 合同。

成品还包括 macOS `.app`、Linux Debian 包，以及 Windows 综合示例的 NSIS／MSI。保留的缺测边界为 Windows 10、macOS Dock 本身的视觉效果、Windows Jump List 的人工点击和 Explorer 的实际重启；这些不计入已验证结果。原始日志与成品留在各宿主的独立工作目录，不提交到仓库。
