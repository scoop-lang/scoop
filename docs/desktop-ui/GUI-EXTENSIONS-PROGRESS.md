# GUI 扩展实施进度

实施依据：[SVG、字体与桌面集成设计](GUI-EXTENSIONS.md)。功能分支为 `codex/gui-extensions`。每个完整批次定向验证后提交、推送，再合并 `origin/main`；本地完成大项后集中进行其他平台验证，原始日志和临时产物放在仓库外。

| 批次 | 状态 | 验证范围 |
| --- | --- | --- |
| G1 Skia SVG 与独立图形库 | 已实现，本机验证通过 | macOS aarch64：14 项 Scoop 测试，含真实 Metal；19 项准备／打包脚本测试 |
| G2 Lucide、SvgView／Icon | 待实施 | 主题、尺寸、线宽、挂载与现有图标迁移 |
| G3 字体目录、字面与变体 | 待实施 | 系统枚举、临时字体、命名实例／轴、精确选择 |
| G4 字体更新与选择示例 | 待实施 | 实际使用者唤醒、编辑状态、多窗口隔离 |
| G5 Windows 圆形窗口 | 待实施 | 先检查默认 demo 路径及真实原生区域 |
| G6 Taskbar／Dock | 待实施 | 各平台已有原生能力及应用标识 |
| G7 桌面通知 | 待实施 | 提交／动作、部署注册、生命周期 |
| G8 集成与发行 | 待实施 | UI 适配、综合示例、各平台完整安装包 |

## G1：独立 SVG 图形库

新增 [SVG 规范](../specs/SCOOP-GRAPHICS-SVG-SPEC.md) 和 `scoop.graphics.svg`，固定 Skia 已启用 SVG／skresources，并静态合并其 DEPS 指定的 Expat。公开 SvgDocument、SvgAsset、SvgDrawOptions 和 Canvas.drawSvg；尺寸、根样式恢复、整体透明度及线程／关闭合同均有真实像素和资源测试。

2026-11-08，本机 macOS aarch64 的 14 项 release Scoop 测试全部通过，包括 2 倍 DPI 的真实 Metal 目标、非零 viewBox、百分比尺寸、渐变／clip／blur、连续颜色／线宽以及文档关闭后的 Drawing。首轮基础图形测试漏算 polyline 的 miter 尖角延伸，修正测试几何范围后全部通过，绘制实现无需补偿。

Scoop 格式化／类型检查、C++ 格式化／编译检查和 Python Ruff 通过；相关准备／打包脚本 19 项测试通过。补齐 GUI 包的 Skia／Expat 静态依赖许可收集，预编译库按普通 sysroot 方式发布。Linux／Windows 与 macOS x86_64 的验证将在 SVG 组件大项完成后集中进行，当前不宣称这些目标已实测通过。
