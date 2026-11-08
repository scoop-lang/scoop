# GUI 扩展实施进度

实施依据：[SVG、字体与桌面集成设计](GUI-EXTENSIONS.md)。功能分支为 `codex/gui-extensions`。每个完整批次定向验证后提交、推送，再合并 `origin/main`；本地完成大项后集中进行其他平台验证，原始日志和临时产物放在仓库外。

| 批次 | 状态 | 验证范围 |
| --- | --- | --- |
| G1 Skia SVG 与独立图形库 | 已实现，三平台验证通过 | macOS 双架构各 14 项含真实 Metal；Linux x86_64／Windows MSVC UCRT：各 13 项；19 项脚本测试 |
| G2 Lucide、SvgView／Icon | 已实现，图形资源三平台验证通过 | 三平台的 31 个图标；macOS 的 4 项组件与实窗测试；既有退出错误留 G4 修复 |
| G3 字体目录、字面与变体 | 已实现，三平台验证通过 | macOS 双架构、Linux x86_64、Windows MSVC UCRT 各 10 项字体测试 |
| G4 字体更新与选择示例 | 已实现，macOS／Linux 验证通过 | 静止／隐藏窗口、独立读者、原生 preedit、选择器与完整 demo；退出错误闭合；Windows 实窗待完成 |
| G5 Windows 圆形窗口 | 已修复，Windows 11 MSVC UCRT 实窗通过 | 默认 demo 路径、HWND style／region、resize、最小化恢复及既有窗框／输入回归 |
| G6 Taskbar／Dock | 待实施 | 各平台已有原生能力及应用标识 |
| G7 桌面通知 | 待实施 | 提交／动作、部署注册、生命周期 |
| G8 集成与发行 | 待实施 | UI 适配、综合示例、各平台完整安装包 |

## G1：独立 SVG 图形库

新增 [SVG 规范](../specs/SCOOP-GRAPHICS-SVG-SPEC.md) 和 `scoop.graphics.svg`，固定 Skia 已启用 SVG／skresources，并静态合并其 DEPS 指定的 Expat。公开 SvgDocument、SvgAsset、SvgDrawOptions 和 Canvas.drawSvg；尺寸、根样式恢复、整体透明度及线程／关闭合同均有真实像素和资源测试。

2026-11-08，本机 macOS aarch64 的 14 项 release Scoop 测试全部通过，包括 2 倍 DPI 的真实 Metal 目标、非零 viewBox、百分比尺寸、渐变／clip／blur、连续颜色／线宽以及文档关闭后的 Drawing。首轮基础图形测试漏算 polyline 的 miter 尖角延伸，修正测试几何范围后全部通过，绘制实现无需补偿。

Scoop 格式化／类型检查、C++ 格式化／编译检查和 Python Ruff 通过；相关准备／打包脚本 19 项测试通过。补齐 GUI 包的 Skia／Expat 静态依赖许可收集，预编译库按普通 sysroot 方式发布。Linux／Windows 的后续验证结果见 G2／G3；Windows MinGW 尚待验证。

## G2：Lucide 与 UI 图标

固定引入 Lucide 1.70.0 的 31 个原始 SVG，维护清单、生成脚本及完整上游许可一并保存。独立资源 Cone 不依赖 UI，普通应用构建不联网。新增借用文档的 SvgView、按挂载管理文档的 Icon，以及 ActionContentStyle.icon 成员入口；现有 Spinner、导航、菜单、侧栏、日期、关闭、搜索和提示图标迁移到相同资产。

本机 macOS aarch64 release 已通过全部 31 个资源的解析／真实像素绘制，以及 4 项组件测试（尺寸、固定线宽、资源替换与关闭、builder）；相关许可收集的 6 项 Python 测试通过。新增实窗测试通过，验证主题切换、尺寸／来源更新、RTL 下普通图标不镜像、零尺寸、卸载及借用资源保持有效。Scoop 格式化／编译、Python Ruff 通过，预编译组件库已更新。

真实离屏消费者分别引用 search 和全部 31 个图标，均成功运行；最终可执行文件均为 19,931,032 字节，包含 Skia 和 runtime。检查最终数据段发现两者都保留了 31 份 SVG，原始资源合计 12,159 字节。因此当前不承诺逐图标链接剔除，也不把整个可执行文件大小解释为图标的新增体积；独立入口不建立运行时全量枚举或启动解析。

现有 frame-reuse 实窗测试完成了 15 帧、动态图形变化及零额外布局的断言，但退出时失败于 `input router is closed`。使用改动前的预编译组件库复现相同错误：关闭输入路由后的终结失焦事件仍触发 TextEditHost 的菜单命令更新。该既有生命周期问题随 G4 的 UI 字体／编辑状态改动一起修复并复验，避免为同一 UI 依赖重复构建；此处不将整项测试记为通过。

Linux nuc12 的独立 checkout 完成固定 Skia 构建，release SVG 13 项与 Lucide 全部 31 图标的资源测试通过。Linux UI 实窗与字体更新大项合并验证。Windows 独立 checkout 的 Skia 构建发现 PATH 中 python3 启动器已失效；准备脚本改为向 GN 显式传入正在运行的 Python，不修改用户全局环境。

## G3：字体目录与变体

FontCatalog／FontFace 现可枚举和精确打开系统字面，从内存或文件读取 TTF／OTF／TTC／OTC，查询命名实例和可变轴，按坐标创建独立字面。FontCollection 支持实际字面登记、别名整体替换和变更订阅。来源关闭不影响已有字面或段落，失败替换不发布 revision。

macOS aarch64 release 的 10 项字体测试通过，覆盖系统目录独立存活、跨目录 ID、TTC 五个真实家族、静态字体、双轴局部覆盖、隐藏轴、命名实例的实际像素变化、线程／关闭、旧段落快照和观察者异常／取消。固定 Skia 的 CoreText 工厂不接受非零 TTC 索引，CoreText 描述符列表又包含命名实例；后续字面现由 HarfBuzz face builder 提取后交给 Skia，避免将实例索引误当集合索引。未加入另一套字体解析器或系统字体安装。

Scoop 格式化／编译、C++ 格式化／定向编译检查、Python Ruff 通过，相关脚本 19 项测试通过。提交后，Linux x86_64 与 Windows MSVC UCRT 分别通过相同的 10 项 release 字体测试；Windows 也完成 SVG 13 项和 Lucide 31 图标验证。macOS x86_64 在 Rosetta 下通过 SVG 14 项（含 Metal）、Lucide 31 图标与字体 10 项。Windows GUI 实窗与 MinGW 仍在后续批次验证。

## G4：字体变更与选择示例

FontWatch 对实际使用者订阅 FontCollection，通过既有失效和帧请求合并唤醒窗口，最后一个读者退休后立即取消订阅。帧入口不再检查字体集合 revision；绘制缓存自己的有效性判断继续保留它。默认集合替换沿原有信号，隐藏窗口保留失效，字体目录查询不会触发字体变更。

原生桌面示例新增系统家族搜索、字面选择、TTF／OTF／TTC／OTC 文件打开、命名实例及自定义轴滑杆。hidden 轴显式展开；预览使用可整体替换的独立别名和同一个 TextEditorState。模型测试覆盖 TTC 后续字面、实例／局部轴、失败加载保留旧选择、旧字面释放及编辑内容／选区保持。

macOS aarch64 release 的 mount-fonts、font-wakeup、frame-reuse 三个独立窗口 Cone 通过。font-wakeup 通过真实 NSTextInputClient 发送 preedit，验证字体变化前后的组合范围、选区、文本与修订不变；同时验证静止唤醒、未使用集合的窗口不重排、隐藏恢复和卸载取消。该原生 preedit 场景目前限定 macOS，跨平台默认字体／挂载用例已开启 Windows 与 Linux。frame-reuse 的输入路由退出错误已通过关闭区域后停止菜单命令更新修复，稳定布局与完整退出一起通过。

完整 native-desktop smoke 通过，包括双窗口、菜单、窗框、异形／工具窗口、模态、文件面板取消、无窗口运行和再次打开。Scoop 格式化／编译通过，UI 及依赖预编译库已更新；Linux／Windows 实窗随该批次提交后集中验证。

Linux 的 mount-fonts 与 frame-demand 在 nuc12 解锁后的真实 GNOME Wayland 桌面通过；同一原实现也通过独立 Weston GLES 合成器和 Xvfb X11 的窗口验证。frame-demand 复用既有连续丢弃／隐藏恢复用例并开启 Linux GNU。首次运行时 GNOME 锁屏停止了 frame-clock 交付，造成测试等待帧超时；解锁后无需修改 renderer 即完成。本批未保留诊断日志代码或试探性的 GTK 调度改动。Xvfb 使用软件图形，不将其记为硬件桌面视觉验证。

Windows 11 登录桌面 Session 1 的 mount-fonts 已通过。选择器组合测试发现 TTC 后续字面的命名实例坐标不正确，正在单独修复和补充图形字体组合覆盖；不将该选择器测试记为通过。

## G5：Windows 圆形示例

默认窗口现在启用实际 window／input shape，并将没有提供边缘缩放操作的圆形示例设为不可缩放；保留仅视觉裁剪的显式对照选项。smoke 不再提前改动形状选择，和普通打开路径使用同一默认值。本批只调整 demo，没有删除普通 Client 窗口的缩放能力或改写 DirectComposition。

Windows 11 的真实登录桌面 Session 1 上，MSVC UCRT 的独立测试通过：实际调用 DocumentView.showShape，检查 HWND 不含 WS_CAPTION／WS_THICKFRAME；原生区域的中心、边中点在内，四角在外；390×390 初始窗口、480×480 调整、最小化／恢复后均正确，关闭完成 Destroyed。既有 frame 测试同时通过窗框模式切换、区域、DWM 属性和系统按钮／输入退役。macOS 完整 smoke 通过。Windows 10 桌面不可用；两套 MinGW 的整体验证在发行收尾执行，不以本次 MSVC 结果代替。
