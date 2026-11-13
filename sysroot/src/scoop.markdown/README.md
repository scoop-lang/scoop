# scoop.markdown

纯 Scoop 的独立 Markdown 文档库，不依赖 UI、window、graphics 或 native Markdown 后端。完整语法与编辑器按[设计](../../../docs/markdown/DESIGN.md)实施，已验证范围见[实施记录](../../../docs/markdown/PROGRESS.md)。

当前源码模型支持持久化快照、UTF-8 范围、CR／LF／CRLF 行索引、原子多范围替换、位置映射、逆向修改和内容订阅：

```scoop
import scoop.markdown.*

val document = MarkdownDocument.fromText("# Title\n\nText\n")
val before = document.snapshot()
val change = document.replace(MarkdownRange(2L, 7L), "Updated")
document.apply(MarkdownEdit(document.revision, change.inverseReplacements))
```

`snapshot()` 不复制全文；`read(range)` 和 `toMarkdown()` 显式生成字符串。`writeTo(TextWriter)` 顺序输出分块。回调接收差量，不能重入修改；不编辑时输出保留所有源码字节。

解析层以 CommonMark 0.31.2 为基础，默认启用 GFM 表格、任务列表、删除线和自动链接。独立程序直接取得绑定快照的完整语法树：

```scoop
val parsed = document.parse(MarkdownProfile.Gfm)
for (node in parsed.nodes()) {
    when (node.content) {
        case MarkdownContent.Heading(val level, _, _) -> {
            val line = parsed.snapshot.lineAt(MarkdownPosition(node.range.start))
            println(f"Heading ${level}, line ${line + 1L}")
        }
        else -> Unit
    }
}
```

`parseMarkdown(snapshot, profile)` 可独立解析不可变快照；`document.parse(profile)` 复用未变化内容和相同配置的结果。源码读取或修改不隐式触发解析。`MarkdownNode` 的带数据内容枚举保存完整语义，`range` 和 `markers` 定位原始写法；`MarkdownText` 的 UTF-8 映射保留实体、转义、换行与 tab 处理的位置。引用定义保留为节点，支持后置定义、首个定义优先与 Unicode 完整大小写折叠。`node(id)` 和 `nodeAt(position)` 查询同一份结果中的节点；当前身份不跨独立解析复用。

Paragraph、Heading、TableCell 的最后一个内容字段是去掉容器前缀后的行内解析输入，保留实体和 Markdown 标记；表格竖线已按表格词法去转义。叶 Text 的字段是解码后的显示文字。两者都有完整映射。`parent(id)` 在同一份结果中查询父节点，根返回 None。

已通过 652 个 CommonMark 官方样例、26 个 GFM 扩展样例、源码映射与深层嵌套测试，并验证仓库中 547／641 KB 的两份规范。解析算法与语料来源见 [许可说明](licenses/README.md)。这里不提供公共 HTML renderer；HTML 只是标准样例的测试投影。

生成新的 Markdown 使用 block／inline receiver DSL：

```scoop
val notes = markdownDocument {
    heading(1) { text("Release notes") }
    paragraph { text("Includes "); strong { text("Markdown") } }
    bulletList {
        item(checked = Some(true)) { paragraph { text("Generate files") } }
    }
    fencedCode(language = "mermaid", body = "flowchart LR\n  A --> B\n")
}
val source = notes.toMarkdown()
```

`markdownText(options) { ... }` 直接返回字符串。`MarkdownWriteOptions` 控制新源码的换行、最终换行、无序列表符号和缩进；生成不会修改旧文档或触发解析。块接口还包括 blockQuote、orderedList、thematicBreak 和 table；表格先 header，再 row，每行通过 cell 收集行内内容。完整消费代码见 [generation.scoop](examples/source/tests/generation.scoop)。

`text` 保留文字的显示内容并转义 Markdown；`rawMarkdown` 显式接收源码。GFM 的邮箱自动链接仍由解析配置控制。行内还支持 emphasis、strong、strikethrough、code、link、image、softBreak 和 hardBreak；代码和链接各自选择合适的定界与转义。源码事务与默认保存继续保留原有字节，生成器只负责新的内容。可表示的结构及参数边界见[生成规范](../../../docs/specs/SCOOP-MARKDOWN-SPEC.md#7-markdown-生成)。

现有文档通过结构事务改写：

```scoop
val parsed = document.parse()
val heading = parsed.root.children[0L]
val change = document.transform(parsed) {
    setHeading(heading.id, 3)
}
document.apply(MarkdownEdit(document.revision, change.inverseReplacements))
```

`transform` 要求当前文档、当前 revision 的解析结果。同一批命令基于同一份原始坐标，按源码顺序准备后一次发布；冲突或错误参数不留下部分修改。除了标题／段落转换，还可以 replaceText、setInlineContent、wrapInline／unwrapInline、setLinkTarget、setReferenceDefinition、setTask、setTableCell、insertTableRow 和 removeTableRow。节点未涉及的源片段继续保留原始写法，引用和列表的续行沿用本地前缀及换行。修改一个引用链接的 target 会生成独立 inline link；修改共享定义使用明确的 setReferenceDefinition。完整消费示例见 [editing.scoop](examples/source/tests/editing.scoop)，边界见[结构事务规范](../../../docs/specs/SCOOP-MARKDOWN-SPEC.md#8-结构事务)。

整列和列表层级操作、显式格式化、跨 revision 复用、脚注／公式／自定义指令处理及 UI 继续按设计实施。

完整的独立消费示例见 [examples/source](examples/source/src/main.scoop)，其 Cone 通过 path 依赖当前源码库，不要求先重新打包 sysroot。未安装本库时，应用在 `Cone.toml` 中为 `scoop:scoop.markdown` 指定实际源码目录。

本机使用已准备的工具链与系统库：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop fmt sysroot/src/scoop.markdown
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.markdown --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown/examples/source --release
```
