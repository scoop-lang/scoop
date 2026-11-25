# scoop.graphics.text

独立的字体与段落排版 Cone，依赖 graphics 的几何、颜色与画布，不需要 window、surface 或 UI。字体来自显式数据和系统 fallback；shaping／排版复用 SkParagraph、HarfBuzz 与 SkUnicode（macOS 使用 Apple 后端）。

```toml
[dependencies]
"scoop:scoop.graphics" = "0.1.0"
"scoop:scoop.graphics.text" = "0.1.0"
```

```scoop
import scoop.graphics.*
import scoop.graphics.text.*

FontCollection().use { fonts ->
    Paragraph(fonts, "Hello，是世界。مرحبا", textStyle { fontSize = 20.0 }).use { paragraph ->
        paragraph.layout(280.0).use { layout ->
            RasterSurface(320, 180).use { surface ->
                surface.draw { canvas ->
                    canvas.clear(Color.White)
                    canvas.drawText(layout, Point(12.0, 12.0))
                }
                val position = layout.hitTest(Point(30.0, 10.0))
                val caret = layout.caret(position)
                val selection = layout.selection(TextRange(0L, paragraph.text.byteLength))
            }
        }
    }
}
```

FontCollection 默认使用系统 fallback，也可以通过 `FontCollection(false)` 关闭并用 `register(bytes, family)` 登记自有字体。只读 `revision` 从 0 开始，每次成功登记增加一次，失败不改变版本。Paragraph 在构造时保存文字、样式和字体来源快照；`layout(width)` 与 `layoutUnwrapped()` 每次产生独立结果，后续登记字体、关闭来源或重新布局不改变旧结果。

需要延迟创建多个段落，同时保持字体输入一致时，调用 `fonts.copy()` 并持有返回的独立集合。副本保留已登记字面、别名、系统通用字体选择和 fallback 设置，不重新扫描或复制字体文件；之后两边的登记、观察者和关闭互不影响，副本的 revision 从 0 开始。副本仍是可变资源，宿主要保持其不变才能将它用作字体快照，并在 owner 线程关闭。完整的延迟排版示例见 [font_copy.scoop](examples/font_copy.scoop)。

内容通过 `styledText { ... }` 组装，`text(value)`、`span { ... }`、`inlineObject(key, alternateText)` 和 `add(existingSpan)` 是按顺序追加并返回 Unit 的真实成员。`textSpan { ... }` 创建可复用的不可变片段。配置同步执行一次，不创建 native 或 UI 资源；已生成描述不随捕获的 builder 后续修改而变化。Paragraph 也接受 String／TextStyle 的便利构造。

`textStyle { ... }`、`spanStyle { ... }` 和 `paragraphStyle { ... }` 分别构造完整文字样式、继承覆盖和段落选项。内容中的 `style { ... }` 基于当前设置修改，重复块累积、空块不重置；`style = existingStyle` 整体替换。SpanStyle 未指定属性继承父样式，数组属性整体替换，`lineHeight = Some(None)` 恢复自然行高。公开 `SpanStyle.resolve(parent)` 允许主题及普通消费者使用同一继承计算，返回完整 TextStyle，不创建 native 资源。TextStyle 支持 family 顺序、字号、颜色、weight／width／slant、locale、字距、词距、行高及 OpenType feature／variation；ParagraphStyle 指定方向、对齐、maxLines 和 ellipsis。TextRun 仅为内部展开结果。

```scoop
val content = styledText {
    style { fontSize = 20.0 }
    text("Hello, ")
    span {
        style { fontSize = Some(28.0) }
        text("世界")
    }
}
```

Paragraph／TextLayout 的 `content` 保留同一描述。`content.toPlainText()` 导出全文，也可以传入 UTF-8 标量边界的 TextRange。

`inlineObject(key, alternateText)` 声明行内对象；InlineKey.Text／Number 在同一 StyledText 中唯一。`content.inlineObjects` 提供按逻辑顺序保存的 key、UTF-8 range 和 alternateText；规范化文字以一个 U+FFFC 表示对象，导出纯文本时才替换为 alternateText。

`paragraph.layout(width, inlines)` 或 `layoutUnwrapped(inlines)` 接收 `Array<InlineMetrics>`，必须按 key 完整覆盖对象，可以以任意顺序提供。metrics 包含 Size 和 InlineAlignment；默认底边贴 alphabetic baseline，也可选显式 baseline offset、AboveBaseline／BelowBaseline、Top／Bottom／Middle。尺寸允许为零，超宽对象不自动缩小。TextLayout.inlineObjects 保留全部 key；其 box 为 None 表示被 maxLines／ellipsis 截掉，Some 则给出局部矩形、行号和方向。

Canvas.drawText 只绘制文字；调用方按 box 绘制 Image 或放置自绘控件。文本层不拥有图片或 UI 对象。两类基线与行内对象的详细合同、后续高级排版范围见[文本排版设计](../../../docs/graphics/text/DESIGN.md)；当前基线行为需要目标资源准备时应用 [Skia 补丁](../../vendor/skia/README.md)。

`TextLayout.paintBounds` 按需返回实际文字绘图的保守边界，包含字形 overhang，排除由外部绘制的行内对象；空文字为零矩形。它由同一 Skia 绘图命令计算并随不可变布局复用，不等同于 layout.size。使用 Drawing 或分块像素缓存时，在此边界之外继续预留抗锯齿／采样边缘；纯几何查询和正常绘制不因该边界改变。

TextRange 和 TextPosition 使用 **UTF-8 字节偏移**，与 String 的标量索引不同。selection 的端点必须是标量边界，caret 的端点必须是 `graphemeBoundaries` 中的位置。hitTest 返回可见内容的 grapheme 边界；affinity 区分软换行和双向 run 的两侧。被截断的位置没有 caret；selection 可以返回多个带方向的矩形。各行保存 range、contentRange、visibleRange、baseline 与尺寸，查询结果相对布局原点。

公共编辑查询同样使用这份布局：`wordRange` 取得 平台 Unicode 分词段，`moveWord` 按逻辑方向跳过沿途纯空白段，`moveGrapheme` 返回相邻 grapheme 边界。`moveCaret(position, CaretMovement, preferredX)` 支持视觉字符／词移动、上下、逻辑／物理行边界及文档起止，返回新 TextPosition 与可传给下一次上下移动的 preferredX；经过短行时不会丢失原列。WordLeft／WordRight 依实际字形方向遍历 Unicode 词段；LineLeft／LineRight 使用物理左右边缘，与 LineStart／LineEnd 的逻辑起止分开。`logicalLineRange` 合并软换行片段并包含末尾显式换行，供整行选择使用。查询保留混合 bidi 与软换行 affinity，不建立选择状态或编辑历史；完整规则见[导航规范](../../../docs/specs/SCOOP-GRAPHICS-TEXT-SPEC.md#61-公共编辑导航)。

原生宿主可导入 `scoop.graphics.text.native.*`，在布局的 owner 线程调用 `@Unsafe layout.withNativeLayout { native -> ... }`，借出包含 context、retain／release、hit、caret、boxes、range、lineRange／lineIndex 的 CLayout 值。它读取同一份已排版结果；需要跨借用保存时先 retain，每份拥有的引用最后 release，普通值复制不增加引用。保留引用在原 TextLayout／Paragraph／字体来源关闭后仍有效，native 线程可以并发查询；boxes 使用同步的纯 native visitor，不回调 Scoop。偏移仍为原布局 UTF-8 字节，转换为编辑文档、替代文字及屏幕坐标由宿主负责。状态码、单位和寿命见[原生查询规范](../../../docs/specs/SCOOP-GRAPHICS-TEXT-SPEC.md#62-原生只读查询)。

字体、Paragraph 和 TextLayout 的普通方法限创建线程使用及关闭，通过 Closeable／use 管理。原生查询与这些方法共用排版缓存的串行访问边界，不将普通 Skia 句柄直接交给其他线程。文本库拥有自己的 native 资源；Canvas.drawText 只在同步回调内借用 graphics 的公开原生画布接口，正常返回或异常退出后恢复调用方状态，不依赖 graphics 的私有包装或 internal 成员。

准备[目标资源](../../../scripts/README.md)后运行 `scoop test sysroot/src/scoop.graphics.text --release --sysroot sysroot`。测试覆盖真实离屏文本、固定字体、Unicode／bidi、独立布局和资源边界；macOS 另验证真实 Metal 目标。字体来源与许可见 [FONTS.md](tests/data/FONTS.md)，运行结果和平台范围见 [UI 实施记录](../../../docs/ui/PROGRESS.md)。

## Linux GNU

系统字体使用 Fontconfig／FreeType，共享 SkParagraph、HarfBuzz 与 SkUnicode 排版实现，可绘制到 RasterSurface 或 Skia Ganesh GLES 目标；文本库本身不依赖显示服务器。使用已准备的 GNU sysroot，运行 `scoop test sysroot/src/scoop.graphics.text --release --sysroot ~/.scoop`。依赖、测试范围及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
