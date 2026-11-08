# scoop.graphics.svg

静态 SVG 图形库，使用仓库固定版本的 Skia SVG DOM；支持范围及资源合同见 [SVG 规范](../../../docs/specs/SCOOP-GRAPHICS-SVG-SPEC.md)。它不提供完整浏览器 SVG、脚本、外部资源或 SVG 文本字体装载。

```scoop
import scoop.graphics.*
import scoop.graphics.svg.*

SvgDocument.parse("<svg viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"8\" fill=\"currentColor\"/></svg>").use { document ->
    RasterSurface(48, 48).use { surface ->
        surface.draw { canvas ->
            canvas.clear(Color.Transparent)
            canvas.drawSvg(document, Rect(0.0, 0.0, 48.0, 48.0),
                SvgDrawOptions(currentColor = Some(Color(0.2, 0.4, 0.8))))
        }
    }
}
```

文档限创建线程使用和关闭，纯值 SvgAsset 仅保存源码。改变颜色、根描边宽度或整体透明度不需要重新解析。固定根尺寸先形成自然 viewport，再映射到目标矩形；等比展示由调用方选择合适矩形，不重复处理内部 viewBox。已有 Drawing 保留绘图资源，不依赖文档持续存活。

维护者先用 `scripts/build_skia.py` 准备 SVG／Expat 静态资源，再按开发 sysroot 流程构建本 Cone。普通应用只消费已准备的库。
