# scoop.ui.markdown

Markdown 编辑器的纯 Scoop UI Cone。文档内容和结构操作来自独立的 `scoop.markdown`；编辑状态负责源码选区、统一撤销／重做与保存 revision。当前已实现共享状态，视图和原生输入正在按[设计](../../../docs/ui/markdown/DESIGN.md)接入，具体合同见[规范](../../../docs/specs/SCOOP-UI-MARKDOWN-SPEC.md)。

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

本机验证使用 `scoop check ... --tests --release` 与 `scoop test ... --release`。状态测试覆盖中文与 emoji、双向删除合并、反向选区、原子结构事务、外部修改、重载、订阅异常和超过 1 MiB 文档中的连续输入。这些验证不代表视图或输入法已实现。
