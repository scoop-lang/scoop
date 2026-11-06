# scoop.graphics

普通 Scoop Cone，提供与窗口无关的 Skia 2D 绘图。共享接口使用左上原点的 Double 逻辑坐标；目标资源覆盖 macOS ARM64／x86_64、Linux GNU x86_64，以及 Windows x64 的 Microsoft UCRT、LLVM-MinGW UCRT／MSVCRT。CPU 离屏使用不要求启动 Application。

```toml
[dependencies]
"scoop:scoop.graphics" = "0.1.0"
```

```scoop
import scoop.graphics.*

RasterSurface(320, 180).use { surface ->
    path {
        moveTo(Point(20.0, 120.0))
        quadraticTo(Point(160.0, 0.0), Point(300.0, 120.0))
    }.use { outline ->
        surface.draw { canvas ->
            canvas.clear(Color.White)
            canvas.drawPath(outline, Paint(Color(0.1, 0.4, 0.8),
                PaintingStyle.Stroke, 4.0, StrokeCap.Round))
        }
    }
    val rgba = surface.readPixels()
}
```

Canvas 支持 save／restore、仿射变换、矩形／Path 裁剪、基本形状、描边、线性／径向渐变、Image 及组透明度。`saveLayer(opacity = 1.0, filter = None, blendMode = BlendMode.SrcOver)` 对完整组应用滤镜，再以整体透明度和混合模式合成，`restore()` 结束该组。`path(fillRule = FillRule.NonZero) { ... }` 同步执行配置，返回由调用方拥有的不可变快照，并在正常／异常退出时关闭临时 builder；`path()` 创建空路径。Path 的 contains、bounds 和 PathMeasure 使用相同几何与填充规则。

增量构造可显式使用 `PathBuilder(fillRule).use { builder -> ... }`；所有追加及 `reset()` 操作返回 Unit，`build()` 返回独立快照并保留 builder，`reset()` 清空几何但保留填充规则。后续修改或关闭 builder 不改变已有 Path，PathMeasure 也可在原 Path 关闭后继续使用。

`canvas.withState { child -> ... }` 提供可嵌套的绘制作用域，子 Canvas 继承当前坐标和 clip，正常返回或异常退出都恢复全部子级保存状态。子回调只使用 child，父 Canvas 暂时不可用；退出后 child 失效，父 Canvas 恢复可用。该入口可用于自绘子组件，几何快照与绘制共享相同坐标且不会重复施加 DPI。

`ImageFilter.blur` 提供局部逻辑单位 sigma 的 Gaussian blur；`dropShadow`／`dropShadowOnly` 根据实际内容 alpha 生成带源／不带源阴影。`a.then(b)` 执行 b(a(source))，结果独立保留输入引用。滤镜不以布局矩形隐式裁剪扩展像素，源外采样透明黑；`PathBuilder.addRoundRect` 为绘制、clip 与输入提供共用圆角轮廓。详细参数、顺序、颜色和 clip 合同见[图像滤镜规范](../../../docs/specs/SCOOP-GRAPHICS-EFFECTS-SPEC.md)。该扩展已在 macOS aarch64 验证，x86_64 及 Windows 的补验状态见 UI 实施记录。

SkSL runtime effect、uniform／输入 shader、Paint 着色及自定义子树滤镜列为[后续增强设计](../../../docs/graphics/sksl/DESIGN.md)，目前未提供 Scoop 接口。计划复用 Skia 现有后端，程序与参数分开管理，动画显式驱动 uniform；持有效果不自动启动帧循环。

ImageFilter、Path、PathMeasure、Image、RasterSurface 与底层目标由使用者 close／use；显式创建的 PathBuilder 也由使用者关闭，`path { ... }` 的临时 builder 由入口关闭。原生资源限创建线程使用，Canvas 只在一次 draw 回调内有效，异常退出也会结束借用并恢复状态。Geometry 快照保留原生 Path 数据，可以在原 Path 或绘制作用域结束后继续使用；关闭快照后不可再查询。

颜色采用 sRGB，绘制内部为预乘 alpha；RGBA8 输入和 `readPixels()` 返回值采用紧密排列的直通 alpha 字节。PNG／JPEG decode 会复制输入并实际解码；surface snapshot 保留独立图像内容，后续绘制不会修改旧快照。

`canvas.recordDrawing(bounds) { record -> ... }` 返回不可变 Drawing，使用独立局部坐标、明确的 bounds 裁剪和正常 Canvas 借用规则；`canvas.drawDrawing(drawing)` 在当前坐标下重放。记录保留绘图资源而不保留用户回调，来源 Path／Image／TextLayout 包装关闭后仍可使用；重放仍会光栅化，不是像素缓存。

`drawing.canRasterizeTiles` 区分可独立裁剪的记录和需要完整输入的效果。ImageFilter 可能随裁剪区域改变内部降采样位置，因此这类记录返回 false；它们仍能正常重放，或把滤镜留在缓存内容的合成阶段。嵌套记录继承限制。Unsafe withSkiaCanvas 默认不声明独立裁剪，原生适配方可按其绘图合同显式设置 `independentTiles = true`；graphics.text 的纯字形／装饰路径使用这个声明。

`canvas.createSurface(pixelWidth, pixelHeight, scale)` 返回与当前真实目标兼容的 OffscreenSurface。其 `snapshot()` 保留 GPU 图像，不等待或读回 CPU；旧快照不随之后的 draw 改变。`canvas.isCompatible(image)`／`isCompatible(drawing)` 可在复用前判断实际 context 是否相同，不兼容的 GPU 资源拒绝绘制。普通 RenderTarget／RasterSurface 的 snapshot 仍返回 CPU 图像，readPixels 保持显式同步。

`pixelTransform` 包含目标 DPI；`localClipBounds` 只是局部裁剪的保守矩形，精确命中仍使用 CanvasGeometry。分块绘图可用 `drawImageRegion(image, source, destination)` 合成扩展图像的核心区域；Linear 允许从相邻源像素采样，应按滤镜范围及插值需要保留扩展边缘。新能力本期先验证 macOS aarch64，x86_64 统一留到 UI 基础交互功能全部完成后，Windows 验收后续进行。

GUI renderer 通过 `scoop.graphics.native` 的 RenderTarget 和 geometry 捕获接入。macOS 的 Unsafe `scoop.graphics.native.macos.MetalContext` 接受所属 device／queue 和 BGRA8 texture；Windows 的 Unsafe `scoop.graphics.native.windows.D3DContext` 接受 adapter／device／direct queue，以及同设备的单采样 BGRA8 render target。D3D12 resource 在包装前须处于 PRESENT／COMMON 状态，submit 和读回结束后恢复 PRESENT。绑定保留所需 COM 引用，原生对象仍由调用方按 GPU 使用顺序管理。

Linux 的 `scoop.graphics.native.linux.GLESContext` 使用 EGL 1.5／GLES 3。`createX11`／`createWayland` 借用显示连接，`target` 包装所属 EGLSurface；`createOffscreen` 使用 Mesa surfaceless EGL，`offscreen` 创建真实 GPU 目标。普通 close 后，已有目标和 GPU 图像继续保留 context；借用显示连接断开前，宿主必须先销毁窗口 surface，并调用 `abandon()` 释放 GPU／EGL 资源。资源必须在创建线程使用。依赖准备和两后端验收见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。

DPI 缩放由目标根部施加一次，geometry 快照仍使用逻辑坐标。普通 submit 不等待 GPU；显式读回、drain 和 context 关闭是同步点。字体、样式、Paragraph／TextLayout 和 Canvas.drawText 由独立 [scoop.graphics.text](../scoop.graphics.text/README.md) 提供；使用文本时声明该 Cone 依赖并导入其 package。

文本等原生绘图适配通过 `scoop.graphics.native` 的 `@Unsafe Canvas.withSkiaCanvas` 同步借用实际 SkCanvas。借用保留现有坐标／裁剪，正常返回和异常退出都恢复原生状态，不增加 DPI 缩放。指针仅限回调内使用；回调不能重入同一 Canvas 的 Scoop 操作、越过借用的保存边界、提交目标或变更 context。详细合同见 [Surface 规范](../../../docs/specs/SCOOP-SURFACE-SPEC.md#2-几何与绘制)。

准备 [native 资源](../../../scripts/README.md)后运行 `scoop test sysroot/src/scoop.graphics --release --sysroot sysroot`。测试通过真实 Skia 像素、几何和跨 Cone 借用验证行为；文本与字体测试归 graphics.text。整体进展见 [Surface 实施记录](../../../docs/surface/PROGRESS.md)及 [UI 实施记录](../../../docs/ui/PROGRESS.md)。
