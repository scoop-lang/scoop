# UI 玻璃材质与液态融合

2026-11-28。本批工作建立在已合并的下层合成 API 上。公共行为分别进入样式、主题和组件规范；这里记录实现顺序、设计取舍和实际验收。

## 实现顺序

1. 可复用材质：以资源无关的 GlassMaterial 描述模糊、折射、染色与边缘反光，由 StylePainter 管理当前程序、滤镜和轮廓；迁移独立玻璃示例。
2. 表面选择：Style／StyleOverride 支持局部选择与清除；主题通过独立于 SurfaceKind 的表面用途选择材质，组件明确传递用途。
3. GlassEffectContainer：一个容器中的稳定 key 对应独立内容与交互区域；统一计算玻璃外形和背景采样，邻近轮廓连续融合，布局目标变化沿已有动画驱动形变。
4. 状态与可访问性：处理 hover、pressed、disabled、focus、invalid，确保前景对比度，接入减少动态效果、提高对比度和减少透明度偏好；融合的桥接像素不扩大点击区域。
5. Gallery：新增材质、表面策略、融合形变与可访问性对照，验证真实滚动、输入、状态更新、缓存、DPI 和停帧。

每项完成后执行对应格式、检查与统一验证，再提交并推送工作分支。实际发现的编译器或底层库缺口另开修复分支，先完成 PR 合并、本地 main 同步、合入工作分支和工具链更新。

## 边界

材质描述不持有 Canvas、ImageFilter、ShaderProgram 或 UI 状态。背景来自当前实际目标，不保存含旧背景的像素缓存。Skia 执行滤镜和着色；普通绘制没有 CPU 读回或独立时钟。材质不改变文本排版、组件身份或子项输入语义。

融合限定在调用方显式建立的容器内。容器统一绘制材质，内容保持清晰并沿同一场景变换放置。形变由明确的几何目标驱动；不会从任意 SkSL 自动推导布局或命中区域。

## 验收记录

按已完成阶段记录范围、结果和平台限制，原始构建及运行日志保存在仓库外。

### 1. 可复用材质

新增 GlassMaterial／glassMaterial、Style.glass 及覆盖与显式清除；StylePainter 持有当前光学滤镜，独立示例已移除内嵌折射程序。主题的完整表面绘制在整体淡化玻璃时继承下层。

macOS arm64 release：格式化与 check 通过；style 的 27 个测试全部通过，覆盖 Raster／Metal、2 倍 DPI、实时背景、预乘 alpha、模糊、折射、圆角、清晰前景、缓存替换与录制资源寿命。迁移后的示例通过 10 阶段真实窗口测试，包含滚动、开关、0／50／100% 整组透明度和静止停帧。受影响的预编译 UI 库已更新；API 文档无警告生成，并通过 scoop api 检索材质及 copy 签名。本阶段未运行 Linux／Windows native 目标。

### 2. 表面选择

新增 SurfaceRole 与 ThemeTokens.glassSurfaces，组件传递实际用途；Frame 公开 surfaceRole。局部清除恢复普通填充，主题偏好关闭／恢复策略；Popover／Modal／Toast 的呈现组继承下层。

macOS arm64 release：格式与生产／测试 check 通过；四套主题 52 个测试、组件 282 个测试通过。原窗口测量用例在被遮挡时无提交帧而超时，改为与其余绘图窗口测试一致的置顶后通过，未放宽测量或帧数断言。公开 API 文档无警告生成，GlassSurfaces.get 与 FrameBuilder.surfaceRole 可检索。额外的 Popover 焦点会话测试启用了玻璃；本轮在初始窗口等待键盘焦点（phase 0）时超时，尚未进入弹层操作，后续 Gallery／交互验收继续闭合此项。

### 液态融合的底层前置

按独立分支流程补齐 shader alpha 背景遮罩，见已合并的 [PR #269](https://github.com/scoop-lang/scoop/pull/269)。Canvas.drawBackdrop(bounds, filter, mask) 只影响单次背景绘制；输入几何不随任意 shader 改写。graphics 的 88 个 Raster／Metal 等测试通过。本地 main 已更新至 d18a01f3e，合回工作分支后已重建 graphics 原生代码及全部受影响的开发 sysroot 预编译库。本次没有编译器源码变更，继续使用同一套配套编译命令。

### 3. 液态融合与形变

新增 GlassEffectPainter 与 glassEffectContainer／glassEffect。多个圆角轮廓沿共享距离场平滑融合，一次处理背景，并用相同轮廓产生折射和高光。稳定 key 保留独立内容；Surface Animatable 的同一进度插值位置、尺寸和四角，测量、绘制及命中同步。连接区域不创建输入节点；实体回退与透明材质共用几何。单表面与融合表面共用光学程序片段，并修正超过半边长度的不对称圆角判断。

macOS arm64 release：格式、生产／测试 check 与 API 文档生成通过；style 30 个测试、组件 284 个测试全部通过。新公开消费者真实窗口测试通过融合像素、整组 50% 透明度、动画中布局／圆角命中、稳定 key 重排、减少透明度与动画、隐藏／移除和资源关闭；稳定区间的 UI 更新、测量、提交帧、原生显示回调及 GPU 呈现计数不再增长，原生帧需求关闭。Raster／Metal 的 2 倍 DPI 检查覆盖平滑连接／分离、重叠 alpha、清晰前景、缓存失败及录制寿命。开发 sysroot 已更新，painter 的两个 paint 重载与容器 item API 可检索。
