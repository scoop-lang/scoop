# scoop.ui.markdown

Markdown 编辑器的纯 Scoop UI Cone。文档内容和结构操作来自独立的 `scoop.markdown`；编辑状态负责源码选区、统一撤销／重做、组合输入与保存 revision。当前提供原生源码视图，富文本、双栏和扩展展示继续按[设计](../../../docs/ui/markdown/DESIGN.md)实施，具体合同见[规范](../../../docs/specs/SCOOP-UI-MARKDOWN-SPEC.md)。

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

状态绑定创建线程，通过现有 `State` 参与 UI 读取追踪。编辑前检查 owner 和 UI 修改时机；关闭释放订阅与历史，不关闭文档。`markSaved(revision)` 只记录宿主实际保存完成的 revision；较旧的保存完成不会把后续修改标成已保存。

`MarkdownSourceInput(editor, contextBytes = 4096L)` 为窗口输入准备局部 `TextInputState`，包含完整选区与左右上下文。`sourceRange` 明确片段与全文的坐标关系；`apply(TextInputBatch)` 先校验整个批次，再通过一次源码事务发布。它不持有窗口，宿主仍负责原生 connection／origin 和已接受帧的 caret 几何。`synchronize()` 返回 true 时必须退休旧连接后再发布新映射；组合期间不移动片段边界。

`isComposing`／`markedRange` 反映即时预编辑；重复 Preedit 不逐条入历史，Commit／ClearPreedit／`acceptComposition()` 接受为一个历史项。空 Preedit 或 `cancelComposition()` 恢复组合前正文与选区。选择、格式命令和外部 Edit 接受组合；Reset 放弃它。undo／redo 遇到活动组合先取消，下一次再操作历史。保存前调用 `acceptComposition()` 再固定文档 snapshot。

`markdownSourceEditor(editor)` 返回普通 View，可放在 Component.render 中。`MarkdownSourceStyle` 配置字体和颜色，`MarkdownSourceOptions` 配置自动换行、只读、缩进与输入上下文。需要自绘组合时用 `MarkdownSourceHost`，以 `measure` 返回的独立结果调用 `scene`，并在 LayoutResult.dispose 中关闭结果。宿主借用 editor 和字体，由所属 scope 管理关闭；试测不改变已接受帧的命中和候选窗几何。`hitTest` 返回源码位置与视觉 affinity，`caret` 可接受同一 affinity 来保留折行／双向文字边界。

视图用物理行的高度索引定位可见区，只排版视口附近及光标行。一个很长的物理行仍完整 shaping；不把它切成会改变双向文字和连字的任意片段。支持方向选择、系统编辑命令、复制／剪切／粘贴、自动折行或横向滚动、Tab 缩进、拖选滚动、中文组合以及共享撤销。光标闪烁只请求绘制；字体、宽度和对应行内容变化才使文字排版失效。

[源码文件编辑示例](examples/source-editor/README.md) 可打开已有 Markdown，并通过 Cmd-S 保存。库不自行写文件；`onChanged` 只传递源码差量。

本机验证使用 `scoop check ... --tests --release` 与 `scoop test ... --release`。库内测试覆盖状态、历史、输入批次、组合和高度索引；`tests-cones/source` 通过本机原生窗口验证大文档、系统拼音、滚动、几何与资源释放。完整三模式和性能结果以[实施记录](../../../docs/markdown/PROGRESS.md)为准。
