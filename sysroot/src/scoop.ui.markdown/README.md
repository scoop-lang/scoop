# scoop.ui.markdown

Markdown 编辑器的纯 Scoop UI Cone。文档内容和结构操作来自独立的 `scoop.markdown`；编辑状态负责源码选区、统一撤销／重做、组合输入与保存 revision。当前提供原生源码视图、共享后台解析、富文本预览、扩展图像和独立富文本输入片段；WYSIWYG 视图与完整双栏继续按[设计](../../../docs/ui/markdown/DESIGN.md)实施，具体合同见[规范](../../../docs/specs/SCOOP-UI-MARKDOWN-SPEC.md)。

开发期间从另一个 Cone 使用 path 依赖：

```toml
[dependencies]
"scoop:scoop.markdown" = { version = "0.1.0", path = "../scoop.markdown" }
"scoop:scoop.ui.markdown" = { version = "0.1.0", path = "../scoop.ui.markdown" }
```

```scoop
import scoop.markdown.*
import scoop.ui.markdown.*

val document = MarkdownDocument.fromText("# Title\n")
val editor = MarkdownEditorState(document)
editor.setSelection(MarkdownSelection(MarkdownPosition(2L), MarkdownPosition(7L)))
editor.replaceSelection("New title")
editor.undo()
editor.redo()
editor.close()
```

选区使用 UTF-8 字节位置并保留方向与端点偏向。`replace`／`apply`／`transform` 都写入同一个借用文档；普通外部 Edit 也进入历史，Reset 清空历史。`Typing`／`Backspace`／`Delete` 只合并连续位置的同类输入，选区命令与结构事务结束合并。连续输入按片段保存历史，仅在 undo／redo 时拼接改动内容。

文档库的 `parsed.inlineContent(nodeId).replaceText(...)`／`setStyle(...)` 返回准备好的 `MarkdownInlineEdit`。把结果交给 `editor.apply(prepared, reversed = false)`，源码差量与修改后的选区会进入同一个历史项；reversed 可保留反向选择。准备结果携带当前文档身份和 revision，不能应用到别的文档或新 revision。普通源码输入、语义命令与后续 undo／redo 共用历史。

状态绑定创建线程，通过现有 `State` 参与 UI 读取追踪。编辑前检查 owner 和 UI 修改时机；关闭释放订阅与历史，不关闭文档。`markSaved(revision)` 只记录宿主实际保存完成的 revision；较旧的保存完成不会把后续修改标成已保存。

`MarkdownSourceInput(editor, contextBytes = 4096L)` 为窗口输入准备局部 `TextInputState`，包含完整选区与左右上下文。`sourceRange` 明确片段与全文的坐标关系；`apply(TextInputBatch)` 先校验整个批次，再通过一次源码事务发布。它不持有窗口，宿主仍负责原生 connection／origin 和已接受帧的 caret 几何。`synchronize()` 返回 true 时必须退休旧连接后再发布新映射；组合期间不移动片段边界。

`MarkdownRichInput(editor, parsed.inlineContent(nodeId))` 将当前段落、标题或单元格的显示文字接入同一原生输入协议；初始选区使用该内容的源码映射。连续输入直接使用更新后的 content，不等待后台全文解析；字体、图片与窗口仍由视图管理。原生范围使用显示文字的 UTF-8 坐标，隐藏标记不占位置，图片／扩展按 U+FFFC 计入。源码和富文本共享唯一组合与历史，取消组合恢复原始定界符、实体、表格补缺等确切源码。外部修改后通过 `synchronize(newContent)` 提供新解析的区域；返回 true 时退休旧原生连接。

`isComposing`／`markedRange` 反映即时预编辑；重复 Preedit 不逐条入历史，Commit／ClearPreedit／`acceptComposition()` 接受为一个历史项。空 Preedit 或 `cancelComposition()` 恢复组合前正文与选区。选择、格式命令和外部 Edit 接受组合；Reset 放弃它。undo／redo 遇到活动组合先取消，下一次再操作历史。保存前调用 `acceptComposition()` 再固定文档 snapshot。

`markdownSourceEditor(editor)` 返回普通 View，可放在 Component.render 中。`MarkdownSourceStyle` 配置字体和颜色，`MarkdownSourceOptions` 配置自动换行、只读、缩进与输入上下文。需要自绘组合时用 `MarkdownSourceHost`，以 `measure` 返回的独立结果调用 `scene`，并在 LayoutResult.dispose 中关闭结果。宿主借用 editor 和字体，由所属 scope 管理关闭；试测不改变已接受帧的命中和候选窗几何。`hitTest` 返回源码位置与视觉 affinity，`caret` 可接受同一 affinity 来保留折行／双向文字边界。

视图用物理行的高度索引定位可见区，只排版视口附近及光标行。一个很长的物理行仍完整 shaping；不把它切成会改变双向文字和连字的任意片段。支持方向选择、系统编辑命令、复制／剪切／粘贴、自动折行或横向滚动、Tab 缩进、拖选滚动、中文组合以及共享撤销。光标闪烁只请求绘制；字体、宽度和对应行内容变化才使文字排版失效。

[源码文件编辑示例](examples/source-editor/README.md) 可打开已有 Markdown，并通过 Cmd-S 保存。库不自行写文件；`onChanged` 只传递源码差量。

`MarkdownParseController(editor, executor, application.proxy(), options)` 在后台取得同一编辑文档的结构结果，供多个视图共享。executor 使用已有 `CoroutineExecutor`，由应用持有；例如应用可使用 `scoop.concurrent.multiThreadExecutor`。构造不启动任务，在正常 owner 回调或 onMount 中调用 `start()`；源码修改随后自动合并请求。`MarkdownParseOptions` 选择 profile，以及可选的 `MarkdownExpansionOptions(baseUri, resolver, processors)`。

`status` 是可追踪的 Idle／Pending／Ready／Failed／Closed 状态。`current` 只返回当前已完成的解析，`latest` 还可保留正在更新前的显示结果；过期结果不能用于结构编辑。`MarkdownParsedDocument.source` 是原文解析，`preview` 是可选的展开解析，`processed` 保留依赖、诊断与来源映射。结构修改使用 `source`，生成内容保持只读。

控制器最多执行一项后台解析、保存一份最新待处理快照和一个待交付结果；通过 ApplicationProxy 在 owner 发布，选区及保存状态变化不重解析。资源通知后显式 `refresh()`，配置变化用 `configure()`。扩展和 resolver 的方法由工作线程串行调用，其实现不应读写 UI State。处理错误进入对应请求的 Failed，下一次输入或刷新仍可恢复。

关闭控制器会停止订阅及后续交付，不等待运行中的解析，不关闭借用的 editor／document／executor。应用在自己的退出收尾中 shutdown／join 线程池；投递被拒绝或应用关闭后丢弃消息，后台任务仍会完成登记和清理。`tests/parse.scoop` 在真实工作线程与原生应用消息循环中验证合并、过期结果、错误恢复、资源刷新和这条关闭路径。

`markdownPreview(controller)` 返回共享解析结果上的原生预览 View，挂载时启动借用控制器，卸载时仅释放自己的视口资源。`MarkdownPreviewStyle` 配置文字、代码和颜色；`MarkdownPreviewOptions` 配置间距、虚拟化范围、统一表格最小列宽和缩进。`onOpenLink` 把 URI 交给应用；`onSource` 把行内扩展的原始源码范围交给应用。预览显示源码选区、支持拖选、全选和复制所见文字，修改正文继续通过同一个 editor。

自绘组合可使用 `MarkdownPreviewHost.measure`／`scene`；LayoutResult 负责关闭每次独立测量。`measurement.parsed` 表明实际显示的解析，`hitTest` 同时提供原文／预览位置与生成内容标记；`caret` 和 `reveal` 接受原文位置。旧解析等待更新时仍可显示，但不会被用作当前源码命中。表格按实际行虚拟化，代码按实际行虚拟化，普通段落保持完整 shaping。图片及扩展图像使用原子对象与原始范围；点击仍可定位源码。

向 `markdownPreview` 或宿主的 `update` 传入借用的 `images: MarkdownImageController` 与文档 URI，即可按可见范围加载图片。提供本地 PNG／JPEG、Mermaid CLI 和 MathJax＋rsvg-convert 适配，也可按 `MarkdownImageRenderer` 注册自定义图像扩展。配置、版本、基线和关闭约定见[扩展图像说明](IMAGES.md)。

[预览示例](examples/preview/README.md) 可直接打开仓库中的 Markdown；[源码示例](examples/source-editor/README.md) 提供原生编辑和保存。本机验证使用 `scoop check ... --tests --release` 与 `scoop test ... --release`。库内测试覆盖状态、历史、输入批次、组合、高度索引、显示映射和后台解析；`tests-cones/source` 与 `tests-cones/preview` 使用本机原生窗口验证大文档、滚动、几何与资源释放。完整三模式和性能结果以[实施记录](../../../docs/markdown/PROGRESS.md)为准。
