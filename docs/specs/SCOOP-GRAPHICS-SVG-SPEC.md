# Skia SVG 图形库规范

本库按 [GUI 扩展设计](../desktop-ui/GUI-EXTENSIONS.md) 的 G1 实施，以普通 `scoop.graphics.svg` Cone 依赖 graphics、platform 和 std；不增加语言、runtime 或编译流水线机制。后端为仓库固定版本 Skia 的 SVG DOM。

## 1. 支持范围

提供静态 SVG 解析及 CPU／现有 GPU Canvas 绘制。基本图形包括 svg、g、path、circle、ellipse、rect、line、polyline、polygon，以及 viewBox／preserveAspectRatio、变换、填充、描边、继承属性、currentColor、透明度。渐变、clip、mask、pattern 和滤镜沿用该版本 Skia 的实现，不承诺完整标准覆盖。

不增加完整 CSS、symbol、marker、vector-effect、脚本、动画、foreignObject 支持。本阶段不提供 SVG 文本的 font manager 或外部资源 provider；不会因文档中的 URL 读取文件或联网。未知元素或属性可能被 Skia 跳过，成功解析不表示完整呈现；不再实现 XML／CSS 校验器。

固定 Skia 的 SVG 模块和 Expat、skresources 静态依赖由维护脚本准备，随 GUI native 资源发布。应用离线消费预编译 Cone 和 Skia；不在运行时或普通应用构建时下载、解包或准备依赖。

## 2. 文档与纯值

```text
SvgDocument.parse(source: String): SvgDocument
SvgDocument.decode(bytes: Array<Byte>): SvgDocument
SvgDocument.intrinsicSize: Size?
SvgDocument.viewBox: Rect?
SvgDocument.isClosed: Boolean
SvgDocument.close()

SvgAsset(source: String)
SvgDrawOptions(currentColor: Color? = None,
               rootStrokeWidth: Double? = None,
               opacity: Double = 1.0)
```

SvgAsset 和 SvgDrawOptions 是不可变纯值，不持有 native 资源。SvgDocument 是 Closeable，在创建线程使用和关闭。关闭幂等；关闭后的绘制以及其他线程的绘制／关闭抛 IllegalStateException。复制出的尺寸、viewBox 和 isClosed 可查询，不返回借用的 native 指针。

解析在调用内复制输入并建立自有 DOM，不保留移动 GC 内存的裸地址。空输入、坏 XML、非 SVG 根或原生构造失败抛 GraphicsException。关闭文档释放其 DOM，但已记录的 Drawing 继续持有它所需的 Skia 绘图资源。

intrinsicSize 只在根宽、高均能独立解析为有限正数时存在；百分比、em、ex 不伪造自然大小。绝对长度换算沿固定 Skia 的长度规则。viewBox 仅返回文档显式声明的有限正尺寸矩形，原点可以非零或为负；无效或空 viewBox 不提供可用于布局的比例，其渲染仍由 Skia 决定。

## 3. Canvas 绘制

```text
Canvas.drawSvg(document: SvgDocument, viewport: Rect,
               options: SvgDrawOptions = SvgDrawOptions())
```

viewport 使用逻辑坐标，x／y 是放置位置。存在 intrinsicSize 时，先在该自然尺寸的 SVG 根 viewport 中执行文档内部的 viewBox／preserveAspectRatio 映射，再将整个结果映射到调用方 viewport；宽高比不同的目标会产生外部非等比缩放。UI 的 ContentFit 应先选择外部目标矩形，避免重复执行内部 viewBox 映射。

无自然尺寸时，以调用方 viewport 的宽高作为 SVG 容器尺寸，根的百分比长度据此解析；例如 width="50%" 占此容器的一半。绘制不隐式增加 viewport clip；需要裁剪的调用方使用现有 Canvas clip，UI 的 clip 参数也沿用该机制。DPI 仅由目标画布施加一次。

viewport 的坐标、尺寸和右／下边界必须能表示为有限 Skia 标量，宽高不得为负。opacity 必须在 0..1，rootStrokeWidth 必须为有限非负 Skia 标量，Color 沿 graphics 的 sRGB 0..1 合同。非法参数抛 IllegalArgumentException；零尺寸或零 opacity 不产生图形，但仍检查文档及 Canvas 生命周期。

currentColor 指定本次根 color 覆盖；rootStrokeWidth 指定根图形坐标中的 stroke-width 覆盖。子节点显式 color／fill／stroke／stroke-width 保持 SVG 自身的继承优先级。未指定时保留原属性，每次调用后恢复根属性和容器尺寸；不改写源码或重新解析。opacity 对整幅图形用 saveLayer 合成，重叠部分不会因逐图元透明度而加深。

接入复用 `Canvas.withSkiaCanvas` 的借用、作用域和异常恢复。默认不声明 independentTiles，带滤镜或蒙版的记录必须能取得完整输入。C++ 异常在 C ABI 内转换为 GraphicsException，调用前的 transform、clip、save depth 不受正常或失败绘制影响。

## 4. 验证职责

所属 Cone 的 scoop.test 覆盖真实解析和像素、固定／相对尺寸与非零 viewBox、基本图形和样式、整体透明度、颜色／线宽连续绘制、错误参数、线程／关闭和 Drawing 保活。GPU 测试复用 graphics 的原生测试目标，不新增图形后端。平台结果记录在 [GUI 扩展进度](../desktop-ui/GUI-EXTENSIONS-PROGRESS.md)，不以离屏结果替代窗口与桌面功能验收。

## 5. Lucide 资源 Cone

`scoop.icons.lucide` 依赖 graphics.svg，提供固定 `lucide-static 1.70.0` 的选定资源。每个图标在同名 package 下有独立的 lowerCamelCase 只读 SvgAsset 入口，例如 check、chevronLeft、search、loaderCircle、calendarDays、trash2。入口使用 includeStr 引用原始 SVG，没有全库运行时索引或初始化解析；普通消费者只引用需要的入口。

使用按实现规范 2.19 重建的库时，最终链接在所有 profile 下按实际引用选择 SVG immortal 对象与登记；未使用的独立 getter 及 callable 登记沿同一物理选择删除。跨 Cone 内联后仍引用 provider 的资源，不因此保留 getter 本体。新增未引用图标不向成品加入其数据或包装。组件内部引用的默认图标属于实际使用；应用主动保存全量资产数组或运行时索引时，该数据结构的真实引用会保留相应资源。

收录清单位于图标 Cone 的 icons.txt，原始 SVG、ISC 和适用 Feather MIT 许可一并保存。维护脚本从显式本地上游归档导入并生成独立入口；普通构建不联网。所有入选图标通过实际 SVG 解析／绘制测试，最终消费者检查未引用资源是否进入成品。
