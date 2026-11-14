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

`snapshot()` 不复制全文；`read(range)` 和 `toMarkdown()` 显式生成字符串。`writeTo(TextWriter)` 顺序输出分块。回调接收差量，不能重入修改；不编辑时输出保留所有源码字节。`change.map(anchor)` 将 before 锚点移到 after；`change.mapBack(anchor)` 用同样的偏向规则反向映射，删除内部的位置不能被反向恢复。

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

`MarkdownProfile.Editor` 另外开启公式、YAML front matter 和脚注；CommonMark／Gfm 的默认含义保持。公式支持 `$...$`、`\(...\)`、块级 `$$` 和 `\[...\]`，返回带有 TeX 和源码映射的 Extension。front matter 同样保留原始 YAML；缺少关闭定界符的块在 `diagnostics` 中报告范围，原文仍完整可保存。脚注定义和引用是独立节点，`footnotes` 给出使用顺序、编号及节点身份，`footnoteDefinition(label)` 查询定义。

自定义语法通过有序的 `MarkdownExtension` 列表加入 profile：

```scoop
val profile = MarkdownProfile(
    math = true, frontMatter = true, footnotes = true,
    extensions = [mdBookDirectives()]
)
val parsed = document.parse(profile)
```

`mdBookDirectives()` 只识别 `{{#command arguments}}`，同时支持行内、独立段落和代码块正文。解析不会读取 include 文件或启动渲染工具。自定义实现提供 name、trigger、locations 和 parse，输入及返回范围均采用该映射片段中的 UTF-8 字节坐标；完整用法见[无 UI 扩展示例](examples/source/tests/extensions.scoop)。Block 注册只能接管一个完整段落，不能跨 CommonMark 容器；代码块正文必须单独启用，code span 和 HTML 保持自己的词法。未注册的内容仍按普通 Markdown 解析，配置变化应使用新的 profile／扩展实例。解析在调用线程执行；UI 的全文解析与局部结构命令可能并发使用同一 profile，语法扩展应使用不可变配置或同步自己的内部状态。资源处理器与渲染器仍由宿主管理调用。

内容展开使用独立的 `MarkdownProcessingSession`。应用实现 `MarkdownResourceResolver` 的路径解析、版本查询和读取后，可以显式处理同一个不可变 snapshot：

```scoop
val session = MarkdownProcessingSession(resolver, [mdBookProcessor()])
val result = session.process(document.snapshot(), sourceUri, profile)
val expanded = result.expanded.snapshot
```

`document` 与 `result.source` 保持原始 Markdown；`sourceParsed` 保留这份原文的解析及节点身份，写 `expanded` 才输出展开内容。没有实际替换时，两份解析复用同一个结果；发生替换的派生快照具有独立文档身份。`mdBookProcessor()` 支持整文件、单行、一基闭区间、开放起止行号和成对 anchor，嵌套路径相对各自文件解析。每次替换后在实际上下文重新解析，因此包括表格续行、引用／列表和代码围栏的变化。缺失、越界、未配对 anchor 和循环包含保留指令并给出诊断；未知命令同样保留，未实现的 rustdoc_include 不被当作普通 include。

`MarkdownProcessor` 也可以由另一个 Cone 实现，返回 Keep、Replace 或诊断。片段的 Text 是已经合成的 Markdown；Resource 引用本次读取资源的原样范围，并允许继续处理其嵌套指令。`sourceMap.origins(range)` 给出派生范围的来源：Original 精确对应原文，Generated 定位到原指令，能确定资源字节时额外保留 URI 和范围。补出的容器前缀与合成文字不声称精确资源偏移；`sourceAt`／`expandedAt` 用偏向处理替换和删除边界。所有结果诊断范围属于 `expanded` 快照。

session 先查询版本，输入未变时直接复用已有结果，不再次读正文或运行处理器；缺失资源也参与更新检测。配置在 session 生命周期内保持不变，`clearCache()` 释放缓存引用。完整的跨 Cone 自定义处理器和真实文件展开、输出、更新验证见[无 UI 文件示例](examples/source/README.md)。纯解析和生成仍不需要 resolver 或渲染工具。

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

`snapshot.prepare(edit)` 与 `syntax.prepareTransform { ... }` 返回尚未发布的 `MarkdownChange`。可以先以 `region.after(change).parse()` 检查结构或定位结果光标，再调用 `change.applyTo(document)`；文档必须仍持有同一个 before 快照。准备不发送观察通知，不改变 revision；其 after 正是成功发布后的文档快照。完整消费用例见 `examples/source/tests/editing.scoop`。

`transform` 要求当前文档、当前 revision 的解析结果。同一批命令基于同一份原始坐标，按源码顺序准备后一次发布；冲突或错误参数不留下部分修改。除了标题／段落转换，还可以 replaceText、setInlineContent、wrapInline／unwrapInline、setLinkTarget、setReferenceDefinition、setTask，以及表格单元格、行列和矩形修改。节点未涉及的源片段继续保留原始写法，引用和列表的续行沿用本地前缀及换行。修改一个引用链接的 target 会生成独立 inline link；修改共享定义使用明确的 setReferenceDefinition。完整消费示例见 [editing.scoop](examples/source/tests/editing.scoop)，边界见[结构事务规范](../../../docs/specs/SCOOP-MARKDOWN-SPEC.md#8-结构事务)。

表格的 header 是第 0 行。`setTableCell` 修改一个单元格，`insertTableRow`／`removeTableRow` 修改 body row；`insertTableColumn(id, index, alignment) { ... }` 用 inline builder 提供新 header，body 新列为空；`removeTableColumn` 至少保留一列，`setTableColumnAlignment` 只修改分隔行的目标冒号。列变化保留原有单元格源码，必要时补齐外侧竖线；GFM 原先忽略的多余 body 单元格继续留在可见列之后。

`setTableCells(id, row, column, values)` 接受非空矩形的 `List<List<String>>`，把值作为普通文字写入；超出表格时追加必要行列。`pasteTableTsv` 使用同一路径，支持引号中的 Tab／换行和成对双引号，短行补空；非法引号不发布修改。空 TSV 清空一个单元格，一次末尾换行不生成额外空行。整次操作只有一个源码事务，其 inverseReplacements 可精确恢复原始文件。

`indentListItem(id)` 将条目移到前一个兄弟条目下，首项不能缩进。`outdentListItem(id)` 提升一级，原来的后续兄弟与父条目尾部内容留在它下面；当前列表不直接属于条目时，解除当前项的列表标记。`unwrapListItem(id)` 明确解除当前项，移除任务框并用必要空行分隔前后列表。它们只调整实际容器前缀，保持正文、代码围栏及扩展内容；Tab、懒续行和引用容器由解析时的来源信息处理。

`setBulletList(id, marker = '-')` 修改整个 ListBlock 的无序标记；`setOrderedList(id, start = 1L, delimiter = '.')` 转为有序列表并显式顺序编号，可选 `)` 分隔符。编号限于 0～999999999，宽度变化同步调整续行；普通层级操作不重编号其他条目。错误参数或同批重叠编辑不发布修改，所有命令继续提供精确 inverseReplacements。详细边界见[列表规范](../../../docs/specs/SCOOP-MARKDOWN-SPEC.md#83-列表层级与标记修改)。

`parsed.splitBlock(id, range)` 删除段落／标题中的显示选区后拆分块，返回带结果源码光标的 `MarkdownBlockEdit`。左侧保留标题种类，右侧为段落；直接在列表项中的块延续下一项，新任务框未勾选，空列表项退出一层。`parsed.joinBlocks(first, second)` 合并同一容器中相邻的段落／标题并把光标放在连接处。准备不修改文档，调用 `applyTo(document)` 一次发布；行内格式在分界处正确闭合，反向事务恢复原字节。完整范围与容器规则见[块编辑规范](../../../docs/specs/SCOOP-MARKDOWN-SPEC.md#84-块拆分与合并)。

完整解析的 `region(range)` 提供包含完整顶层块及相邻块的 `MarkdownRegion`。用 `after(change)` 跟随原文档的区内修改，再用 `parse()` 显式解析当前区域，得到 `ParsedMarkdownRegion`；这些调用不另建可保存文档或历史。局部树与完整树都通过 `MarkdownSyntax` 提供节点查询、行内编辑和结构事务，范围始终是原文档的全局字节位置。区域保留区外引用定义与脚注标签，但不承担任意源码变化的跨区域传播；调用方只在编辑保持区外块边界时沿用区域，其他情况等待完整解析。

普通文字也可以按显示内容的 UTF-8 范围编辑。`inlineContent` 接收段落、标题、表格单元格或空列表项的节点身份，返回带源码映射的 `text`；隐藏标记不计入显示范围，图片／扩展等按 U+FFFC 原子对象计入：

零字节文档可用 `parsed.inlineContent(parsed.root.id)` 取得位置 0 的空编辑区域。`syntax.inlineContentAt(position)` 还可以定位已有行内块或普通空白行的输入区域；代码和扩展内部返回 None。创建区域不修改原文或解析树；首次输入按普通段落文字写回，空列表项保留标记并补齐必要分隔空格。

```scoop
val parsed = document.parse()
val inline = parsed.inlineContent(parsed.root.children[0L].id)
val prepared = inline.replaceText(MarkdownRange(0L, 5L), "Hello")
val change = prepared.applyTo(document)
val caret = prepared.selection.end
```

`replaceText` 将输入作为普通字面文字，处理实体、Unicode、格式边界、链接与代码 span；`setStyle(range, MarkdownInlineStyle.Strong, true)` 对非空显示范围设置格式，false 则移除相应样式。两者只准备 `MarkdownInlineEdit`，不修改文档；其中的 change 是包含正反差量的准备结果，edit 是普通源码事务，selection 是修改后的源码选区。after 是更新后的可编辑内容，after.text 提供局部显示映射；可直接继续准备后续输入。`applyTo` 要求文档仍处于准备所用的同一快照，发布后 document.snapshot() 与 after.snapshot 相同。

`styleState(range, style, bias)` 查询粗体、斜体或删除线，返回 `MarkdownStyleState.Off`、`On`、`Mixed` 或 `Unavailable`。非空范围忽略纯空白、换行与结构分隔；无可格式化内容时为 Unavailable。折叠位置按 bias 查询相邻内容，空行内区域为 Off。`profile` 提供内容的解析配置，删除线修改要求该配置启用 strikethrough。

单个文字叶片内的修改保留外围标记；会改变定界匹配的边界或跨行内结构的操作，重建当前行内块需要的结构。代码按既有围栏与换行规则写入，隐式引用链接改名保留原目标；不齐表格会补齐编辑位置所需的单元格。局部解析不会执行 include 或重新解析整篇文档。`inline.edit { replaceText(...); setStyle(...); select(...) }` 按当前显示坐标顺序准备操作，最终只发布一次源码事务。builder.content 提供中间内容，restore(savedContent) 恢复同一区域的确切源码，可用于组合取消。nodes 提供当前局部树的顶层行内节点，范围属于本内容的 snapshot；它们的 id 不能交给另一份完整解析。sourceRange 保留必要容器上下文，表格包括整行，确保补缺单元格也能恢复。外部修改后须从新解析重新取得内容。此接口范围是一个段落、标题或单元格。Mermaid 仍保留普通代码围栏，图像生成由 UI／应用适配承担。

跨块选择使用 `syntax.textContent(sourceRange)` 取得 `MarkdownTextContent`。它按源码顺序连接当前区域的显示文字，块间为换行，同一表格行的单元格之间为 Tab；格式标记不进入文字，代码与扩展块作为 U+FFFC 原子，内部仍从源码编辑。普通单段选区自动复用行内快速路径。返回的 `MarkdownTextEdit` 包含准备好的 change、结果源码 selection、实际插入文字的显示 insertedRange，以及可继续编辑的 after：

```scoop
val document = MarkdownDocument.fromText("ab\n\ncd\n\nTail\n")
val content = document.parse().textContent(MarkdownRange(1L, 5L))
// The source selection spans b, the paragraph boundary, and c.
val prepared = content.replaceText(MarkdownRange(1L, 4L), "中文")
prepared.applyTo(document)
// Source: a中文d\n\nTail\n; displayed text: a中文d\nTail.
```

同容器段落和同级列表项可合并，保留起始块种类和两端未选文字；不同容器及表格网格保留结构，只清除选中内容。整篇选择可替换整个文档。`content.edit { ... }` 按当前显示坐标准备多个操作，`restore(savedContent)` 恢复同一上下文原来的完整块和确切字节。`inlineContent` 在可用时提供当前插入边界的行内结构，`region` 提供局部树；它们不意味着整个 surrounding text 只有一段。

`MarkdownTextContent.setStyle(range, style, enabled)` 按相交行内块准备一次格式事务，支持跨段落、标题、列表、引用和表格单元格。显示文字、选区范围与表格网格保持，代码／扩展等原子块原样保留；`styleState` 对同一范围汇总样式。单独 setStyle 的 insertedRange 为空；在 `content.edit { replaceText(...); setStyle(insertedRange, ...); select(...) }` 中给新文字加格式，builder.insertedRange 仍对应实际插入文字。详细边界见[多块编辑规范](../../../docs/specs/SCOOP-MARKDOWN-SPEC.md#85-多块显示内容的编辑)。重排全文源码的显式格式化继续按设计实施。

完整的独立消费示例见 [examples/source](examples/source/src/main.scoop)，其 Cone 通过 path 依赖当前源码库，不要求先重新打包 sysroot。未安装本库时，应用在 `Cone.toml` 中为 `scoop:scoop.markdown` 指定实际源码目录。

本机使用已准备的工具链与系统库：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop fmt sysroot/src/scoop.markdown
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.markdown --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown/examples/source --release
```

## Markdown 片段

`syntax.exportMarkdown(sourceRange)` 把源文档中的选择导出为独立 Markdown：部分粗体、行内代码和链接会补齐结构，引用式链接／图片转为直接目标，脚注补上有效定义。跨块选择保留列表、引用和表格；代码、公式、Mermaid 及自定义指令保留源码，不读取资源或调用渲染器。整篇选择原样返回，任意源码原样裁切仍用 snapshot.read。

`syntax.replaceMarkdown(sourceRange, markdown)` 解析片段并准备 `MarkdownFragmentEdit`，包含 change、结果源码 selection 和 `after: ParsedMarkdownRegion`。用 `edit.applyTo(document)` 发布；after.snapshot 与 change.after 相同，后续局部编辑可直接复用。单段保留两侧格式，多段继承容器，表格矩形保留格式并按需增长；脚注冲突一起改名。表格中的普通多段保留行内换行，无法放入单元格的块结构保留为源码字面文字。此接口使用源码 UTF-8 坐标，区别于 replaceText 的显示坐标；不执行文件 I/O，不持有 UI 或独立撤销历史。

[无 UI 消费示例](examples/source/tests/fragments.scoop) 展示跨文档复制、准备、一次发布和精确恢复。对大文档使用 `parsed.region(range).parse()` 获取局部语法；片段只读取自身和需要的区外定义。
