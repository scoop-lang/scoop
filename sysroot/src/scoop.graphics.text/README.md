# scoop.graphics.text

独立的字体与段落排版 Cone，依赖 graphics 的几何、颜色与画布，不需要 window、surface 或 UI。字体来自显式数据和系统 fallback；shaping／排版复用 SkParagraph、HarfBuzz 与 SkUnicode／ICU。

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

公共编辑查询同样使用这份布局：`wordRange` 取得 ICU 分词段，`moveWord` 按逻辑方向跳过沿途纯空白段，`moveGrapheme` 返回相邻 grapheme 边界。`moveCaret(position, CaretMovement, preferredX)` 支持视觉字符／词移动、上下、逻辑／物理行边界及文档起止，返回新 TextPosition 与可传给下一次上下移动的 preferredX；经过短行时不会丢失原列。WordLeft／WordRight 依实际字形方向遍历 ICU 词段；LineLeft／LineRight 使用物理左右边缘，与 LineStart／LineEnd 的逻辑起止分开。`logicalLineRange` 合并软换行片段并包含末尾显式换行，供整行选择使用。查询保留混合 bidi 与软换行 affinity，不建立选择状态或编辑历史；完整规则见[导航规范](../../../docs/specs/SCOOP-GRAPHICS-TEXT-SPEC.md#61-公共编辑导航)。

字体、Paragraph 和 TextLayout 限创建线程使用及关闭，通过 Closeable／use 管理。文本库拥有自己的 native 资源；Canvas.drawText 只在同步回调内借用 graphics 的公开原生画布接口，正常返回或异常退出后恢复调用方状态，不依赖 graphics 的私有包装或 internal 成员。

准备[目标资源](../../../scripts/README.md)后运行 `scoop test sysroot/src/scoop.graphics.text --release --sysroot sysroot`。测试覆盖真实离屏文本、固定字体、Unicode／bidi、独立布局和资源边界；macOS 另验证真实 Metal 目标。字体来源与许可见 [FONTS.md](tests/data/FONTS.md)，运行结果和平台范围见 [UI 实施记录](../../../docs/ui/PROGRESS.md)。
