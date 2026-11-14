# 图片与扩展图像

`MarkdownImageController(executor, application.proxy(), renderers)` 管理可见图片的后台生成和 owner 线程上的原生资源。把它传给 `markdownPreview(parser, images = Some(images), documentUri = fileUri)`，或 `MarkdownPreviewHost.update` 的同名参数。视图借用 controller；应用在组件都退出后关闭它，并在自己的退出流程中关闭 executor。executor 需要配置 blocking 线程。

内置适配的配置分别为：

- `MarkdownFileImageRenderer(baseDirectory)`：本地文件 URI 或相对引用。默认目录为创建适配时的工作目录；有 documentUri 时按文档位置解析。include 图片使用来源映射中被包含文件的 URI。百分号转义交由 `scoop.url` 解码，文件本身由 graphics 的 PNG／JPEG 解码器读取。
- `MarkdownMermaidImageRenderer(command, toolVersion, timeout)`：command 指向已安装的 mmdc，可预先配置 Puppeteer 等参数或环境。适配器只传入代码块正文，输出 PNG，使用显示比例换算逻辑尺寸。超出段落宽度的结果等比缩小。
- `MarkdownMathJaxImageRenderer(node, moduleRoot, rasterizer, toolVersion, timeout)`：node 是 Node 命令，moduleRoot 是已安装的 `@mathjax/src` 目录，rasterizer 是 rsvg-convert 命令。随 Cone 嵌入的进程入口调用 MathJax，带回 SVG、逻辑宽高与基线；Scoop 再调用光栅工具生成 PNG。基线来自 MathJax 的实际 viewBox，逻辑宽度保留为行内 advance。带编号的全宽公式沿工具的布局使用可用宽度，并保留公式的最小宽度。

外部命令使用 `scoop.platform.process.command(Path)` 构造；可执行文件路径应明确，参数经 argv 传递。toolVersion 标识这份固定的工具与配置；更换工具或配置时使用新版本和新适配器。固定配置的 refresh 直接复用成功图片；本地文件适配通过文件时间与尺寸检查更新。普通库使用不自动下载工具。Mermaid 和 MathJax 的语法支持由配置的工具版本决定。

扩展识别在文档 profile 中完成，`MarkdownParsedDocument.sourceExtensions`／`previewExtensions` 暴露原始与派生语法的索引。默认 Editor profile 用 `markdownCodeExtension("mermaid")` 识别围栏；自定义 profile 应显式加入所需的 nodeExtensions。`MarkdownParseOptions.presenters` 按扩展名称注册 `MarkdownExtensionPresenter`。`presentInline(span)`／`presentBlock(span)` 分别选择 Native、Source 或 Image；默认 math 和 mermaid 使用图像，代码保留逐行原生展示。注册只覆盖同名默认，名称必须非空且不能重复。

例如将 `diagram` 围栏交给名为 custom 的 renderer，需要在文档 profile 注册 `markdownCodeExtension("diagram", "custom")`，注册 `markdownImagePresenter("custom")`，并提供 `MarkdownImageKind.Extension("custom")` 的适配。代码语言、语法扩展与渲染器名称不再依靠 UI 自行扫描来关联。块的完整源范围仍包含围栏／分隔符，渲染输入使用语义正文和完整 info。

`markdownImageExtension(standaloneBlock = true)` 使独占段落的图片按 Block 排版；其余图片保持 Inline。默认 image 名称使用普通 Resource，指定其他名称时使用对应 Extension(name) renderer，输入仍是图片目标 URI。连续行内编辑也使用当前局部树上的规则和索引，不套用旧节点偏移。

`markdownImagePresenter(name, kind = None)` 从 Image、CodeBlock 或 Extension 提取映射目标／正文、info、attributes 和替代文字；可显式选择 Resource 或另一个 Extension 名称。`markdownNativePresenter(name)` 保留标准布局，也可展示块扩展解析出的 Markdown 子节点；`markdownSourcePresenter(name)` 在原位置显示完整语法，块源码按行布局。

应用可直接实现 `MarkdownExtensionPresenter`，从 span 的原节点语义构造 `MarkdownImagePresentation(kind, source, info, attributes, alternateText)`。引用式图片的 source 映射指向定义，可能在当前图片使用范围之外。展示选择是纯投影操作，不能读资源、执行工具、访问 UI 或改写文档；配置应不可变或自行同步。完整、局部和连续行内投影共用同一注册，绘制不会反复调用 presenter。没有实现的接口方法默认选择 Source。

```scoop
val profile = MarkdownProfile(
    nodeExtensions = [markdownCodeExtension("diagram", "custom")]
)
val options = MarkdownParseOptions(
    profile,
    presenters = [markdownImagePresenter("custom")]
)
```

自定义 renderer 声明 `MarkdownImageKind.Resource` 或 `Extension(name)`，实现：

```scoop
interface MarkdownImageRenderer {
    val kind: MarkdownImageKind
    fun version(request: MarkdownImageRequest): String?
    fun render(request: MarkdownImageRequest): MarkdownImageData
}
```

request 包含正文或资源引用、文档 URI、完整代码 info、扩展 attributes、可用宽度、显示比例、字号、前景／背景颜色和 inlineContent。渲染结果包含编码字节与实际版本。逻辑尺寸可由 `MarkdownImageSizing.Logical(Size)` 或 `PixelScale(scale)` 声明；行内扩展还须提供真实 baseline，普通图片可以按底边对齐。扩展不需要实现所见即所得输入，源码、历史与保存仍使用同一份原文。

renderer 的方法在一个 controller 的 blocking 队列上串行执行，不保证固定线程。后台返回编码字节，owner 才创建原生 Image。相同完整请求在多个可见视图间共享。图片版本变化只替换对应行资源；已接受的旧画面和独立快照仍能持有旧图片。

状态为 MissingRenderer、Pending、Ready、Failed；Pending／Failed 可以带上一次成功结果。失败详情保存在 Throwable 中，显式 `refresh()` 可重试。只有已接受帧声明的图片会启动工作，试测和丢弃帧不会启动；离开所有可见视图后退休缓存。close 不在 UI 线程等待外部进程，已运行任务正常结束并清理自己的临时文件。外部进程超时由 platform 的 communicate 终止并回收所拥有的子进程；其派生进程遵循所配置工具自身的退出行为。

原生图像的所有权由 `MarkdownImageSnapshot` 维持。需要延长快照生命时调用 `retain()` 并自行 close；`snapshot.status` 中的 Image 是借用值，不能单独 close。自绘测量通过独立 `MarkdownPreviewMeasurement` 持有图片，关闭 controller 不会使尚在使用的测量结果失效。

端到端 Scoop 测试与外部工具准备见 [images 测试 Cone](tests-cones/images/README.md)，可运行的公开 API 用法见[预览示例](examples/preview/README.md)。
