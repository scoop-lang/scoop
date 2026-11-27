# Scoop 图像滤镜与图层规范

更新于 2026-11-28。本文扩展 [Surface 规范第 2 节](SCOOP-SURFACE-SPEC.md#2-几何与绘制)的 graphics 合同。基础滤镜与图层优化的历史验证见 [UI 实施记录](../ui/PROGRESS.md)和[基础设施实施记录](../ui/infrastructure/PROGRESS.md)。SkSL 与背景采样的 UI 接入见[公共样式规范](SCOOP-UI-STYLE-SPEC.md)和[独立工具栏示例](../../sysroot/src/scoop.ui.component/examples/glass/README.md)。

## 1. 职责与公开对象

实际 Gaussian blur、内容 alpha 阴影、混合和图层合成属于 `scoop.graphics`，离屏绘图也可直接使用，不依赖 UI、Surface 或窗口。实现复用当前 Skia 的 SkImageFilters 和 saveLayer；基础效果不以 SkSL／SkRuntimeEffect 接入为前置。

`ImageFilter : Closeable` 是 owner 线程上的不可变原生滤镜。首批提供以下工厂与组合：

| 入口 | 对源内容 S 的操作 |
| --- | --- |
| `ImageFilter.blur(sigmaX, sigmaY)` | 对预乘 RGBA 作 Gaussian blur；源外按透明黑采样 |
| `ImageFilter.dropShadow(offset, sigmaX, sigmaY, color)` | 取 S 的 alpha，模糊、着色并平移为阴影，然后把 S 以 SrcOver 合成到阴影上 |
| `ImageFilter.dropShadowOnly(offset, sigmaX, sigmaY, color)` | 仅输出上述阴影，不包含 S |
| `ImageFilter.runtimeShader(program, sourceChild, sampleRadius) { … }` | 用 sourceChild 采样 S 并执行已绑定的 SkSL 程序，见第 5 节 |
| `filter.then(next)` | 返回 `next(filter(S))`；列表中的先后顺序就是实际求值顺序 |

sigma 是标准差，sigmaX／sigmaY 为有限非负的局部逻辑单位；offset 为同坐标系的有限 Point。零 sigma 合法，仍可产生平移、着色或恒等结果。颜色沿用 Color 的非预乘 sRGB 输入，模糊与合成使用预乘 alpha；不能把 RGB 与 alpha 重复相乘。非法参数按 graphics 的既有异常规则拒绝。

组合结果独立保留所需 Skia 引用，关闭输入包装不使结果失效。close 幂等；关闭后使用、异线程使用仍遵守 GraphicsResource 的状态与 owner 检查。组合失败不接管或关闭输入。普通 Style 不保存这些原生对象，其使用 Element 的缓存才拥有它们。

## 2. 图层与混合

`Canvas.saveLayer(opacity = 1.0, filter = None, blendMode = BlendMode.SrcOver, bounds: Rect? = None)` 在当前保存栈上建立透明图层；restore 时先处理完整图层内容，再应用整体 opacity 和指定混合模式合成回父目标。BlendMode 提供 SrcOver、Src、DstIn、Plus、Darken 和 Lighten：分别为通常覆盖、替换目标、以源 alpha 遮罩目标、预乘通道饱和相加，以及按通道取较暗／较亮色的标准 alpha 混合。Plus 不代表 HDR。Darken／Lighten 用于头像明暗边框，复用同一 Skia 图层合成，不在控件中读取目标像素。

filter 在调用时借用，图层保留实际需要的原生引用直到恢复／使用结束，因此调用者随后可以关闭该包装。组透明度不能替换成各条绘图分别乘 alpha。默认行为仍是普通 SrcOver 组透明度，既有 save／restore 和 Canvas.withState 的异常恢复合同不变。

图层不以调用方布局矩形作隐式 clip 或滤镜 crop。可选 bounds 是局部坐标中的完整源绘制范围提示，供 Skia 减少离屏分配；它不修改 Canvas 的显式裁剪，也不是滤镜输出 crop。调用方只有知道实际源绘制范围时才提供，未知时保持 None；Skia 继续结合目标、当前 clip、滤镜及采样范围安排图层。已声明有限范围的绘图记录在隔离重放时传递原范围，不为每段小内容分配整个视口大小的中间目标。离开源内容范围的阴影、光晕和模糊只受真实目标与显式祖先裁剪限制。布局尺寸不是绘制像素的边界，不能据此截断 glyph overhang 或子项效果。

GPU 目标中未提供 bounds 的普通 SrcOver 透明度组，可将本次组内绘制暂存为 Skia 命令，恢复时以命令的保守设备空间范围和抗锯齿余量分配图层。范围由 Skia 对实际路径、图片、文字及嵌套滤镜计算，不能由 UI 布局推测；未知范围保守覆盖原裁剪。每次调用仍执行绘图回调，不隐式复用上帧内容。暂存保持原变换、裁剪、混合顺序和原生依赖，显式 restore 与作用域异常收尾均完成该组；不增加 CPU 像素读回或 GPU 等待。Raster 和显式 Drawing 记录继续使用各自原有流程。

clip 建立在 saveLayer 之前时约束恢复后的输出；滤镜可能为该输出读取 clip 外所需的源样本。若要先裁源内容再产生向外效果，须在滤镜图层内部另建显式 clip。两种顺序不同。几何捕获继续使用绘制时的 transform／clip；滤镜的像素扩展本身不扩大区域几何。

`CornerRadii(topLeft, topRight, bottomRight, bottomLeft)` 保存四个物理角的椭圆半径 Size，缺省均为零；all(Double)／all(Size) 构造统一半径。normalized(size) 检查有限非负尺寸，任一半轴为零的角收敛为直角；相邻两角的横／纵半径总和超过对应边长时，对全部半径使用同一个最小缩放比例。零宽／高返回有界的退化轮廓。图形数值继续满足可表示的 native 标量范围。

`RoundedOutline(bounds, radii)` 是不拥有资源的公共物理轮廓。normalized() 解析合法外框；inset(left, top, right, bottom) 从归一化外框计算内框，横／纵圆角分别减去对应边的实际内缩。相对两边宽度超过边长时按比例收敛到该边内的一点，内框尺寸为零；不产生负尺寸或框外原点。expanded(amount) 以非负外扩量同时扩展边界和圆角，用于焦点轮廓。graphics 不依赖 UI 的布局方向。

PathBuilder.addRoundRect(rect, radii) 与 Canvas.drawRoundRect(rect, radii, paint) 使用同一 CornerRadii 归一化和 Skia 圆角矩形。原有 radiusX／radiusY 形式是统一椭圆半径的便利入口，委托同一实现。PathBuilder 追加操作返回 Unit；同一 Path 用于填充、clip、contains 和输入几何，不另用 UI 的圆角近似。作用域清理与增量构造的快照所有权见 [Surface 规范 §2](SCOOP-SURFACE-SPEC.md#2-几何与绘制)。

### 2.1 与下层共同合成

`Canvas.saveBackdropLayer(opacity: Double = 1.0, backdrop: ImageFilter? = None, bounds: Rect? = None)` 保存同一变换／裁剪状态，以当前层已经绘制的像素 D 初始化新层。提供 backdrop 时，在初始化时过滤 D；之后的普通绘图与子层在此结果上合成。因此模糊或折射只处理下层，随后绘制的文字、图标和子项保持清晰。省略 backdrop 时继承原像素，组内仍可在不同轮廓上使用 drawBackdrop、既有混合模式和 SkSL 滤镜。

restore 将完成后的组内容 R 按 `D * (1 - opacity) + R * opacity` 写回父层；插值同时作用于预乘 RGB 和 alpha。opacity 为有限的 0～1，0 保留下层，1 完整替换；没有背景滤镜的空组不改变下层。它不是将已含 D 的结果再次 SrcOver 到 D 上，半透明背景不能重复变浓。组内的 clear／Src 等操作可以替换或清除继承的像素，随后同样按此公式恢复。调用方需要透明隔离的内容组时使用原有 saveLayer；两种入口各自保持明确语义，不提供相互冲突的恢复 blendMode 参数。

下层是调用处当前合成层，不是整窗快照，也不跨过更外层的透明隔离层。希望后代玻璃在组透明度动画中仍能访问页面背景时，该动画组应使用 saveBackdropLayer；普通 save／withState、变换和子组件作用域不隔离下层像素。嵌套的 backdrop 层各自在自己的进入点取下层，并各自应用一次组 opacity。

backdrop 与普通滤镜一样只在入口借用；图层／绘图记录持有所需的实际原生依赖。bounds 沿 saveLayer 的可选范围提示规则，不建立额外 clip；提示须涵盖组的实际输出及组内背景读取所需的范围，不能只以组件布局盒子代替。进入时的 backdrop 滤镜由 graphics 计算所需外围样本，恢复输出仍受入口的显式 clip 限制。组内后续背景读取以该层实际已有的像素为源，不穿透其像素边界。

实现复用 Skia 的初始化下层与预乘混合能力，不获取 CPU 截图、不逐帧等待 GPU。含此操作的 Drawing 必须令 canRasterizeTiles 为 false；重放时才读取新的目标下层，不能将某一帧的背景固化为可复用像素块。它与 restore／withState 共用保存栈、异常收尾和输入几何，不新增时钟或持续帧需求。UI 对应入口及场景缓存见 [UI 规范 §7.2](SCOOP-UI-SPEC.md#72-场景装配与纯绘制)。

## 3. 坐标、生命周期与能耗

Canvas 根部仍只应用一次 DPI。滤镜参数保持局部逻辑单位，由当前 CTM 进入设备空间；调用方和 style 均不得预乘 DPI。嵌套图层、变换与原生 Canvas 借用保持同一坐标及 alpha 合同，不为了滤镜读回 CPU 或逐帧等待 GPU。

滤镜对象没有时钟、后台任务或帧请求。静态对象可长期复用；动画调用方使用已经采样的参数建立／替换实际变化的滤镜，及时关闭旧包装。Skia 保留尚在使用的引用，不能通过积累旧包装来延长 GPU 资源寿命。

新资源必须在构造成功后发布。创建、组合或绘制失败沿现有 graphics／Surface 路径传播，关闭完整清理；withState 在异常退出时恢复保存层级和借用状态。该层不接管 ComponentScope、挂载身份或动画。

## 4. 定向验收与后续

所属 Cone 的 scoop.test 验证真实 RasterSurface 和 Metal 像素：透明内容的 alpha 投影、源与影的合成、滤镜顺序、零 sigma、整体 opacity／混合、边界扩展、clip 顺序、变换与一次 DPI，以及组合后关闭输入、异常层恢复和错误线程。使用带容差的像素／覆盖范围断言，不要求两个后端逐位一致，也不只检查编译成功。

下层合成另覆盖半透明下层的空组恒等、预乘插值、嵌套组与异常恢复、先过滤背景再绘制清晰前景、裁剪外的入口滤镜样本、记录在不同目标背景上的重放，以及真实 UI 的背景更新、组透明度和停帧组合。保留普通 saveLayer 的隔离行为回归。

## 5. SkSL 着色与背景采样

`ShaderProgram.compile(source: String)` 在创建线程用 Skia 编译 shader 入口，失败时 GraphicsException 保留 SkSL 诊断。程序可以供多个实例复用。每次 `program.makeShader { … }` 或 `ImageFilter.runtimeShader(program, sourceChild, sampleRadius) { … }` 同步配置一份独立参数，保存不可变快照。ShaderProgram、Shader 和 ImageFilter 遵守 graphics 的 owner 线程和 close 合同。产生的对象与绘图记录持有实际原生依赖。

接收者 ShaderBindings 在配置回调期间有效。setFloat／setInt 写单个标量，setFloats／setInts 按 SkSL 声明顺序写入相应数值类别的完整分量（矩阵按列，数组连续排列）。名称、标量类型、分量数与数组长度按程序反射检查。数值默认零。setColor 将 Color 作为非预乘 sRGB 输入写入单个 layout(color) 的三或四分量 uniform。setChild 绑定一个 shader 输入。每个 shader 槽需有绑定，图像滤镜中 sourceChild 由绘制目标提供，保持空出。编译入口支持 shader 类型的 child。错误的配置报告 IllegalArgumentException，回调结束后访问绑定报告状态错误。

drawRect／drawOval／drawRoundRect／drawCircle／drawLine／drawPath 可在 paint 后借用可选的 shader。一次绘制使用一种着色来源，带渐变的 Paint 与显式 Shader 同用报告参数错误。SkSL 的坐标遵循当前局部变换，返回预乘 RGBA，Paint 的 alpha 参与绘制。`Image.toShader(sampling = ImageSampling.Linear)` 按图像像素坐标采样，边缘 clamp。shader 或滤镜持有的 GPU 图像沿用 Canvas 与 Image 的实际 context 相容性规则，组合只接受相容输入。

runtimeShader 的 sourceChild 是隐式源图像的名称。sampleRadius 是有限非负的局部逻辑长度，覆盖程序沿任一轴偏移该输入的最大距离。saveLayer(filter = …) 以当前组的绘制结果作源。`Canvas.drawBackdrop(outline: Path, filter: ImageFilter)` 取当前合成层在该绘制点之前的像素作源，按滤镜所需邻域采样，在现有 clip 内按抗锯齿轮廓覆盖率替换背景。半透明背景只计算一次。祖先隔离图层限定背景的可见范围。绘图记录在回放时读取目标背景，含 backdrop 的记录将 canRasterizeTiles 置为 false。

`Canvas.drawBackdrop(bounds: Rect, filter: ImageFilter, mask: Shader)` 在有限非负大小的局部 bounds 中，以 mask 的 alpha 作为这次背景替换的覆盖率。mask 与滤镜使用同一局部坐标及当前变换；RGB 不参与遮罩。原背景为 D、过滤结果为 R、遮罩覆盖率为 m 时，结果为预乘 `D * (1-m) + R * m`，再受已有祖先 clip 限制。bounds 限定输出，滤镜可以按其声明的邻域读取 bounds 外的背景；透明遮罩不擦除 D。遮罩只属于这次绘制，不改变 Canvas 的后续裁剪、变换或捕获的输入几何。滤镜和遮罩均在调用时借用，绘图记录保留其实际原生依赖，并在每次重放时读取当时的下层。两者持有的 GPU 图像都必须与目标 context 相容；错误路径不改变 Canvas 状态。该入口用于连续生成的光学轮廓，不需要 CPU 读回或将着色轮廓转成 Path。

UI 在 SceneContext.draw 中绘制需要读取实际背景的效果，随后呈现清晰的前景内容。程序与参数可以跨帧复用，背景在实际呈现帧中采样。动画使用 Surface 现有调度。定向验证包括错误诊断、参数和输入生命周期、记录回放、半透明合成、裁剪及滚动列表上固定工具栏的背景刷新。
