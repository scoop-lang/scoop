# Scoop Markdown 文档库规范

日期：2026-11-14。规范先于实施；当前交付与验证范围见[实施记录](../markdown/PROGRESS.md)，完整目标见[文档库设计](../markdown/DESIGN.md)和[编辑器设计](../ui/markdown/DESIGN.md)。

## 1. 定位

`scoop:scoop.markdown` 是纯 Scoop library Cone，package 为 `scoop.markdown`。它依赖实际使用的 core／std，提供源文档、快照、解析、结构化处理与生成，不依赖图形、窗口、UI 或 compiler crate。所有库行为由 `scoop test --release` 验证，使用本机已准备的工具链与系统库。

源文本是唯一可写文档。解析结果、预览、扩展展开和图像均为派生数据。默认保存保留原始 UTF-8 字节、换行、空白与未知语法；生成新内容和显式格式化才选择新的 Markdown 写法。

## 2. 源文本与位置

`MarkdownDocument.fromText(source)` 建立独立文档，内容 revision 初始为 0。`documentId: MarkdownDocumentId` 是该文档的普通类型化身份；快照、修改和解析结果带同一身份。文档由调用方串行修改；不可变快照可以经正常同步跨线程发布。

`MarkdownPosition(byteOffset)`、`MarkdownRange(start, end)` 使用 Long UTF-8 字节位置。范围为半开区间，必须满足 `0 <= start <= end <= byteLength` 且端点位于 Unicode 标量边界。接口在实际消费范围时检查，不把构造纯值变成读取文档的操作。

`MarkdownAnchor(position, bias)` 的 bias 为 Before 或 After。在锚点位置插入时 Before 留在新内容之前，After 移到之后；替换内部的位置分别收敛到替换开始或结束。选择方向属于上层状态，range 本身不保存方向。

## 3. 不可变快照

`document.snapshot(): MarkdownSnapshot` 返回当前不可变快照，不复制全文，不隐式解析 Markdown。快照提供 documentId、revision、byteLength、lineCount、`read(range)`、`chunks(range)`、`writeTo(writer)`、`toMarkdown()`、`lineAt(position)`、`positionOfLine(line)` 与 `lineRange(line, includeTerminator)`。

`read` 和 `toMarkdown` 显式物化请求的文本。chunks 顺序迭代范围内的字符串片段，端点均为合法 UTF-8 边界；writeTo 按片段调用 TextWriter，不先拼成全文。旧快照在后续修改、reset 和订阅取消后仍有效。

行号从 0 开始。LF、CRLF 和单独 CR 都是换行；CRLF 只算一个，位于 CR 与 LF 之间的位置仍属于前一行。空文档有一行，末尾换行产生一个末尾空行。positionOfLine 返回行首字节位置；lineRange 缺省不包含换行，有效行必须返回准确范围。

文本存储使用持久化平衡树和有界 UTF-8 叶片。正常修改只更新涉及的树路径；节点缓存长度与换行汇总，不为一次键入复制完整文档或从头建立行索引。CRLF 横跨叶片时与同叶片语义一致。

## 4. 修改事务

`MarkdownReplacement(range, text)` 表示一次源码替换；text 是已经合法的 String。`MarkdownEdit(baseRevision, replacements)` 复制操作列表，所有范围都引用该基线。列表按源码顺序排列且不重叠；同一位置的空范围插入按列表顺序出现。

`document.apply(edit): MarkdownChange` 在发布前检查版本、全部范围及结果长度，再构造新文本并一次发布。无效基线抛 IllegalStateException，非法范围／边界／顺序抛 IllegalArgumentException，实际长度溢出抛 ArithmeticException；这些失败不改变文档或通知订阅者。

`replace(range, text)` 使用当前 revision 创建单次事务。对相同范围替换相同文字的操作被省略；没有有效修改时返回未变化的 change，不推进 revision、不通知。真正发布一次修改只推进一次 revision。

`MarkdownChange` 保存 before／after 快照、实际 replacements、inverseReplacements 和 kind（Edit／Reset），提供 `changed`、`map(anchor)` 及 `mapBack(anchor)`。mapBack 使用反向差量把 after 锚点映射到 before，插入／删除边界沿同一 Before／After 规则处理；删除内部信息不会因此恢复。inverseReplacements 引用 after 的坐标，可在当前 after revision 上组成反向事务；不重复执行过滤或结构命令。仅保存实际变化所需的数据和共享快照，不为键入复制全文历史。

`reset(source)` 表示外部整体重载，保持文档身份、推进 revision 并发布 Reset，即使字节相同也通知重载。上层可据此清空旧历史；普通 replace 不暗含这种行为。

## 5. 通知

`observe(callback): MarkdownSubscription` 返回 Closeable 订阅。一次成功发布对当时仍有效的订阅通知一次，无初始补发。取消幂等，释放 callback；通知中取消尚未调用的订阅后，该订阅不再被调用；通知中新增的订阅从下一次修改开始接收。

修改回调不能重入 apply／replace／reset；该调用在改变状态前抛 IllegalStateException。回调异常不会回滚已发布的文档，其他有效订阅仍收到相同 change，最后传播首个异常。上层应区分事务准备失败与已经发布后的通知失败。

## 6. 解析与源码映射

`parseMarkdown(snapshot, profile)` 显式解析一个不可变快照，返回 `ParsedMarkdown`；`MarkdownDocument.parse(profile)` 在内容与配置不变时复用结果。默认 `MarkdownProfile.Gfm` 启用表格、任务列表、删除线和扩展自动链接；`MarkdownProfile.CommonMark` 遵守 CommonMark 0.31.2。配置是普通不可变值，不涉及 UI 或外部进程。源码修改不等待解析。

解析先识别块容器，再收集引用定义，最后解析行内内容。后置定义和 Unicode 完整大小写折叠参与引用查找；重复定义取首次出现者。代码围栏、HTML、实体、转义、强调分隔符、列表缩进和懒续行遵循对应方言。未完成输入按 Markdown 的普通恢复规则保留，不作为文档读写错误。

GFM 的裸网址在行内词法阶段识别，其正文中的实体或反斜杠不再次解码；邮箱自动链接在文字解码后识别。显式链接的 label 不再生成嵌套链接。实体解码出的空格不作为源码行尾空白删除，也不计入两个空格触发的 hard break。

`MarkdownNode` 保存类型化身份、完整源码范围、不可变子节点、语义内容和语法标记范围。`MarkdownContent` 的带数据枚举承载各类节点的必要字段：标题级别、列表起始数与紧凑性、代码 info 与正文、链接目标及引用形式等，不能让调用方通过空字段猜测节点是否合法。引用定义作为可查询节点保留，不生成可见段落。解析结果绑定 documentId／revision；节点查询只接受该结果中的身份，不以名称或数组位置冒充身份。

`MarkdownText` 保存解码后的 UTF-8 文本及分段源码映射。每段分别标识直接文本与语义转换；实体、反斜杠转义、CRLF／CR→LF、NUL→U+FFFD、部分 tab 展开等都明确映射至原始范围。只在派生语义中规范化这些字符，snapshot 及保存仍逐字节保留原文。渲染位置与源码位置在转换段内部按 Before／After 偏向落到原始／派生段边界；直接文本保持字节内的精确位置。列表／引用前缀、强调符号和链接目标不混入显示正文。代码 span 的换行、首尾空格及表格中的转义竖线也必须保留映射。

树遍历及块构建避免依赖文档嵌套深度的调用栈。解析可以在调用线程上显式物化一次输入；不得隐式变成源码键入的同步步骤。非 UI 的调用方可直接遍历节点、查询位置或读取引用定义，继续通过 source transaction 写回。

CommonMark 官方样例通过 Scoop 测试中的 HTML 投影核对语义；投影只是测试 oracle 的适配，不成为本库必需的公共 HTML API。额外测试覆盖源码映射、查询、无损保存、GFM 和真实仓库文档。

## 7. Markdown 生成

`markdownDocument(options, configure)` 使用 `MarkdownDocumentBuilder.() -> Unit` 收集内容，生成新的 `MarkdownDocument`；`markdownText(options, configure)` 返回同一生成行为的 String。生成不修改已有文档，也不隐式解析。`MarkdownWriteOptions` 选择 LF／CRLF／CR、是否保留最终换行、无序列表符号与列表缩进；选项必须能形成合法的 CommonMark 写法。

block builder 提供 paragraph、heading、blockQuote、bulletList、orderedList、fencedCode、thematicBreak、table 和 rawMarkdown。列表通过 item 的 block builder 收集内容；表格通过 header／row 的 cell inline builder 收集内容，列数与对齐数据在输出前确定。普通块间生成必要空行，容器给每条实际行添加相应前缀；代码中的空行和内容不变成容器外的段落。列表紧凑性由调用选择与条目内必要的块分隔共同决定。

inline builder 提供 text、emphasis、strong、strikethrough、code、link、image、softBreak、hardBreak 与 rawMarkdown。`text` 按字面文字转义，避免意外形成标题、列表、强调或链接语法；显式 rawMarkdown 加入调用方提供的源码，其组合效果由调用方表达。GFM 邮箱自动链接作用于解码后的文字，邮箱是否成为可点击链接由解析配置的 autolinks 决定；生成器不为屏蔽它而插入 HTML 或隐藏字符。文字中的换行可用字符实体保留在同一文字片段，真正的 Markdown 换行使用 softBreak／hardBreak；NUL 的显示语义沿 CommonMark 归一为 U+FFFD。

强调及删除线需要处理内容边界的空白、标点和相邻分隔符；必要时用实体编码相邻文字边界，保持可见文字与格式，不添加隐藏字符改变正文。空的格式片段不生成孤立定界符。code 根据正文的最长反引号串选择定界符，并处理首尾空格；行内代码的换行按 CommonMark 规范化为空格。fencedCode 选择不会与正文冲突的围栏，info 必须是单行，非空正文需要在关闭围栏之前结束一行。链接及图片分别处理显示文字、目标和 title 的转义，不能把一套文字转义直接用于所有位置。

生成器拒绝没有合法对应写法的结构参数：标题级别超出 1–6、嵌套链接、段首／段尾／相邻的显式行内换行、标题或表格内的显式换行、表格列数不匹配，以及 GFM 代码单元格中竖线前奇数个原样反斜杠。最后一种组合无法由 GFM 的反引号与表格转义共同保留，调用方可以显式提供 raw HTML 单元格。空文字和空格式不产生节点；连续同类格式可以合并，重复嵌套的同类格式按同一显示样式生成。生成配置只影响新源码，包括 rawMarkdown 中的换行；已有 snapshot 的字节不受影响。

生成的新文档与已有 snapshot 的 PreserveSource 输出是两条明确的调用路径。默认保存仍调用源文档的 writeTo／toMarkdown，不能把生成器变成保存时的全篇重排。显式格式化和结构化编辑在后续章节给出独立合同。

## 8. 结构事务

`document.transform(parsed) { ... }` 在匹配 documentId／revision 的解析结果上准备结构修改，并通过 `MarkdownEdit` 一次发布。过期或其他文档的结果抛 IllegalStateException。准备期间不修改文档；按原始坐标稳定排序后检查操作不重叠，任何非法参数或冲突都使本次准备失败。一个事务内的方法都基于同一份原始解析；不能用第一次操作尚未发布的节点作为后续操作的输入。回调返回前文档若被其他调用修改，最终 apply 仍拒绝旧基线。

`MarkdownTransformBuilder` 提供 `replaceSource`、`replaceText`、`setInlineContent`、`setHeading`、`setParagraph`、`wrapInline`、`unwrapInline`、`setLinkTarget`、`setReferenceDefinition`、`setTask`，以及单元格、表格行列和矩形修改（§8.2）。节点参数使用该解析结果的 MarkdownNodeId，错误节点种类或索引抛 IllegalArgumentException。不同节点的互不重叠修改可以组合，旧快照与反向事务遵守既有合同。

Paragraph、Heading 和 TableCell 的内容数据同时带有 `MarkdownText` 类型的行内解析输入：保留 Markdown 标记、实体和普通转义，去掉容器前缀、标题标记和任务框前缀；表格竖线先按 GFM 表格词法去转义。去前缀、换行规范化和表格转义均保留原始映射。这与叶 Text 的已解码显示文字明确区分。`ParsedMarkdown.parent(id)` 提供所属树内的父节点，根返回 None，不重新搜索整树。

`replaceText` 替换 Text 或 CodeSpan 的显示内容，按所在表格与行内语法选择转义。`setInlineContent` 用 inline builder 替换一个段落、标题或表格单元格的内容，保持外围标记和容器；在该内容之外的源码不重写。标题与表格的内容仍是单行。`setHeading` 对已有 ATX 标题只改开头级别标记；Setext 的一／二级保留原风格，更高级或普通段落转成 ATX，多行正文合并为一行。`setParagraph` 去掉标题标记，并保护会被误识别为块标记的开头普通文字。wrap／unwrap 作用于完整行内节点；选区跨节点的格式操作在后续编辑器合同中补充。

修改链接使用点的 `setLinkTarget` 保留 label 的源码并生成独立 inline target；共享引用定义不变。`setReferenceDefinition` 明确修改定义及其所有引用效果。`setTask` 只修改或增删任务框，添加任务框要求条目的第一个块是段落或空条目。源码显示的其他标记不因此统一格式化。

任务条目的首段须保持段落，转为标题前先移除任务框；无法在 Markdown 中同时表达任务首段与独立标题的参数组合直接报错。空标题的行内内容插入点位于开头标记之后，存在关闭标记时保留必要的间隔。

表格以第 0 行为 header。修改已有单元格只替换它的内容；为不齐的行补出缺失单元格时只追加必要分隔符。新 body row 的列数须匹配 header；删除不作用于 header。行修改继承所在引用／列表的续行前缀与本地换行，不把一个引用内的表格行插到引用外。整列与矩形修改见 §8.2；列表层级操作和显式格式化继续按设计补充。

### 8.1 行内显示内容的编辑

`parsed.inlineContent(id)` 为一个 Paragraph、Heading 或 TableCell 提供 `MarkdownInlineContent`。`text` 是带源码映射的显示文字：软换行是空格，硬换行是换行，图片、扩展、原始 HTML 与脚注引用是 UTF-8 三字节的 U+FFFC 原子对象。格式和链接定界符不出现在显示文字中。它属于一份不可变源快照并保留局部行内结构，不拥有另一份可保存文档。nodes 提供当前局部树的顶层行内节点，供无 UI 处理与显示投影读取格式、链接及对象；它们使用 snapshot 的源码坐标，节点身份不属于其他完整 ParsedMarkdown，不能交给另一棵解析树的结构命令。

零字节文档的 root.id 也可传给 inlineContent，得到源码位置 0 的空行内区域；只准备区域不增加段落、不修改原文或完整解析树。首次输入按普通段落文字转义，后续操作、取消和逆向事务沿同一套局部编辑规则进行。非空文档的 Document 根不能当作行内区域。

`replaceText(range, text, bias)` 的 range 使用这份显示文字的 UTF-8 坐标；可跨普通文字、行内格式和链接，但不跨所属段落／标题／单元格。插入继承所选边界处的格式与链接；Before 选择左侧，After 选择右侧，边缘使用存在的一侧。替换非空选区继承被替换的第一个内容单元。对象只能按完整标量处理，原始 HTML、扩展的内部修改继续使用源码事务。代码 span 的新内容按既有生成规则规范化换行与围栏；普通文字按字面值写入，不把输入中的 Markdown 标记直接执行为格式。

`setStyle(range, style, enabled)` 对非空显示范围设置／移除粗体、强调或删除线，保留其他样式与链接。删除或格式操作跨越行内结构时，重建该行内块内需要的结构；块外原文、引用定义、标题标记、任务标记、列表／引用前缀及换行风格保持。单个文字叶片或代码 span 内的普通修改走局部替换，保留外围定界符；若边缘物理字符的空白／标点类别变化会影响定界匹配，同样沿结构重建路径处理。隐式依赖 label 的链接变更后使用保留目标与标题的显式链接。显式结构修改可规范化受影响行内块的格式写法，不成为普通保存或模式切换时的全文格式化。

两种操作都只准备 `MarkdownInlineEdit`，包含文档身份、原始 revision、普通 MarkdownEdit、编辑后源码选区及 `after: MarkdownInlineContent`。change 是包含正反差量的完整准备结果，尚未因此发布；after.snapshot 是同一 change.after 不可变源快照，after.text 是局部显示映射；它可继续准备后续输入，文档尚未因此发布修改。applyTo 要求文档仍处于准备使用的源快照，直接发布同一事务准备结果；不重复读取或应用已准备的修改。源码差量取实际变化的公共前后缀之外部分，保持标量边界。更新的映射通过当前块的行内解析生成，复用原解析提供的链接／脚注上下文，不为一次行内操作重新解析整篇文档。参数错误或不可表达的代码单元格沿既有参数错误处理，准备失败不发布。

`content.edit { ... }` 按顺序准备一个行内编辑批次，builder 的 content 始终指向本批当前的局部内容。replaceText／setStyle 消费当前显示坐标，select 设置最终显示选区；整批只有一个对原文档的源码事务和一次内容 revision。restore 可以恢复从同一行内区域保留的内容快照，供组合取消恢复确切源码；其他文档或其他区域的内容不能充当该恢复基线。中间快照共享原文档未变化的源树，不建立另一份文档或历史，不向观察者发布。

MarkdownInlineContent.sourceRange 是局部操作会涉及的源范围，包含必要的块标记与换行；表格使用所属行以涵盖缺失单元格的补齐。该范围随准备结果更新。后台全文解析可以随后收敛；连续行内输入不依赖它按键逐次完成。外部内容修改后，旧准备结果按既有快照规则失效，由调用方使用新解析重新选择编辑区域。

### 8.2 表格列与矩形修改

insertTableColumn(id, index, alignment) 在 0 到现有列数之间插入一列，末尾 inline builder 配置新 header，默认空；已有 body row 的新单元格为空。removeTableColumn 删除指定列及对应分隔行标记，至少保留一列；删除整个表格使用明确的源码／节点替换。setTableColumnAlignment 只替换目标分隔单元格的冒号，保留原有连字符数量和周围空白。

setTableCells(id, row, column, values) 把非空、矩形的普通文字写入指定位置；起点可以是已有单元格，也可以紧接最后一行／列。矩形超出当前范围时追加必要的列与 body row，新列默认无对齐，新 header 与未覆盖位置为空。文字按现有单元格生成规则转义，Tab／换行可编码为实体，不能把文字内容当作 Markdown 执行。非矩形、负索引、起点跨过末尾、错误节点种类或重叠事务均失败且不发布。

pasteTableTsv 使用相同矩形写入路径。Tab 分列，CR／LF／CRLF 分行；首字符为双引号的字段可包含 Tab 和换行，两个双引号表示一个字面双引号，关闭引号后只能是分隔符或结尾。尾部的一次行分隔不额外增加空行；短行补空单元格，空输入表示一个空单元格。不完整引号或关闭后的非法文字拒绝整次操作。

列和矩形操作保留未编辑单元格的原始内容、引用／列表前缀和既有换行；涉及的行可以补齐两侧竖线，避免单列退化成标题或新内容成为块标记。GFM 原来忽略的多余 body 单元格保留在可见列之后，不因扩列变成可见正文。所有行与分隔行修改合并为一个普通源码事务；逆向事务恢复原始字节。

## 9. 扩展语法

`MarkdownProfile` 保存 GFM 开关、math／frontMatter／footnotes 开关及有序的 `MarkdownExtension` 注册。CommonMark 和 Gfm 保持原有语法，Editor 开启 GFM、公式、front matter 与脚注；mdBook 指令由调用方显式注册。配置及扩展的解析行为在 profile 生命周期中保持不变，改变配置时创建新 profile。解析缓存比较开关与注册对象及顺序，不以扩展名称猜测两个不同处理器等价。

`MarkdownExtension` 提供 name、非空 trigger、locations 和 parse。locations 分别选择 Inline、Block 和 Code；Block 只接管已经确定的整个段落输入，不能跨 CommonMark 容器边界，跨容器展开交给后续 processor。Code 明确启用代码块正文中的匹配，不影响 code span 或 HTML。Inline 在代码、普通转义和 HTML 词法之外匹配；相同 trigger 按 profile 注册顺序选择第一个成功结果，固定公式语法优先于自定义匹配。名称重复和保留的内置名称冲突是配置错误。

`MarkdownExtensionInput` 提供带源码映射的 source、当前位置上下文和 UTF-8 范围读取。匹配结果使用该输入内的 Long 字节位置，给出 end、body 范围、字符串属性及 closed；不把它们当作全文偏移。解析器只验证实际匹配范围、前进与标量边界，再构造完整 `MarkdownExtensionValue`。正文、完整 token 和标记均保留原始映射。无匹配返回 None，识别器自身异常照常传播；解析不隐式执行 I/O、展开 include 或生成图片。

内置公式产生 name 为 math 的 Extension 节点，location 区分行内与块级。行内支持单个 `$...$` 与 `\(...\)`；前者正文非空、开头和结尾不为空白，关闭 `$` 后不能紧跟数字，并且不跨物理换行。连续多个 `$` 不被拆成单个开关。块级支持独占行的 `$$`／`\[` 与对应关闭定界符，以及开闭定界符处在同一行的完整公式；正文不解析强调、链接或其他扩展。容器前缀和换行规范化遵守 MarkdownText 映射，TeX 字节在源 snapshot 中不变。

front matter 只在文件起始处（可在 UTF-8 BOM 之后）识别独占行的 `---`，结束标记为独占行的 `---` 或 `...`。它产生 front-matter Extension，body 保存 YAML 源码，不在本层解释 YAML 值。未关闭的块公式或 front matter 持续到所在容器／文件结束，closed 为 false，并带有范围诊断；未闭合的行内公式保留为普通文字。`ParsedMarkdown.diagnostics` 为该快照的派生诊断，不导致源文本丢失或解析失败。

`mdBookDirectives()` 是首个普通自定义注册，识别 `{{#command arguments}}`。它可用于行内、独立段落及明确启用的代码块正文；command 与 arguments 作为属性保存。没有注册时它仍是普通源文本。注册只识别语法，include 内容处理有独立入口；不能因为它位于代码围栏内而隐式解释其他已注册扩展。

## 10. 脚注

footnotes 开关识别 `[^label]:` 定义与 `[^label]` 引用。定义是可包含段落、引用、列表和代码块的容器，后续缩进行消费四列，允许首段的普通 lazy continuation。标签使用与引用定义相同的 Unicode 大小写和空白归一规则；同名定义保留在语法树中，引用选择第一个。没有对应定义的引用保持字面文本，不丢失未完成输入。

`FootnoteDefinition` 和 `FootnoteReference` 保存归一标签，完整范围及标记仍在 MarkdownNode。`ParsedMarkdown.footnoteDefinition(label)` 查询实际定义；`footnotes` 按正文首次引用顺序返回编号、定义身份与引用身份集合，再按已使用脚注中的引用继续展开编号。未使用定义不会单独占用编号，循环引用不会重复加入队列。源文档中的定义位置和拼写不因编号改变。

## 11. 无 UI 内容处理与资源

`MarkdownProcessingSession(resolver, processors)` 对不可变 snapshot 显式执行 `process(snapshot, baseUri, profile)`，返回 `ProcessedMarkdown`。它包含原始快照及其 sourceParsed、派生解析、分段来源映射、实际资源依赖和诊断；原文、revision 与订阅不改变。没有替换时 expanded 可以与 sourceParsed 共享结果，发生替换后的派生快照具有独立文档身份。默认输出仍写原始 snapshot；需要展开后的 Markdown 时显式写 `result.expanded.snapshot`。session 由调用方串行使用，处理器与 resolver 的配置在其生命周期内保持不变；配置改变时建立新 session。

`MarkdownResourceResolver` 提供 resolve(baseUri, reference)、version(uri) 和 read(uri)。URI 是调用方定义的稳定逻辑地址，resolve 负责普通相对路径解析；version 返回不读取正文的可选版本，None 表示资源不存在。read 返回同一个资源快照的 UTF-8 文本与版本；读期间发生修改时，以实际返回的版本为准。解析器仍不调用 resolver。平台文件、网络与内存适配在应用侧实现，不增加文档库的平台依赖。

session 先比较输入文档身份、revision、base URI、profile 和已用资源版本，命中直接返回既有结果，不重新读取正文或重放处理。源 snapshot／profile 没有变化时继续复用 sourceParsed，资源更新只重做相关读取及派生展开。单次处理对同一 URI 使用同一资源快照；版本改变才读新正文。缺失资源也记录依赖，因此资源出现后可重处理；普通读取或版本查询异常形成当前指令诊断，下次显式 process 可以重试。缓存只保留当前结果及相关资源，clearCache 明确释放引用。缓存不读取内容以复核摘要，不检查文件系统来源。

`MarkdownProcessor` 以扩展 name 注册，接收节点、当前派生解析、base URI 和本次资源读取入口，返回 Keep、Replace(fragment) 或诊断。重复 name 是配置错误。资源 I/O 错误转为诊断；处理器自身的编程错误照常传播，不发布部分结果。未注册的处理器不影响语法解析或原文保存。

片段由 `MarkdownFragmentPart.Text(text, resourceUri?)` 和 `Resource(resource, range)` 组成。Text 是处理器已经生成的 Markdown，映射到整个原指令；可以给出资源地址，但没有精确范围时不伪造字节对应。Resource 是本次读取的不可变资源范围，保留逐字节来源和自己的 base URI，可继续处理其中的扩展。Text 不再次执行内容处理器，避免对已经生成的结果反复替换；它仍正常参与 Markdown 解析和展示。Resource 范围在消费时检查编码边界，嵌套处理中重复相同资源选区的祖先链报告循环，不按任意深度或通用预算截断文档。

每次替换后重新解析实际上下文，再处理后面的指令；前一个展开可能打开或关闭代码围栏，因此不能按旧解析批量执行后续处理器，也不把分别解析的 include AST 拼接在一起。所属列表、引用及缩进代码的续行前缀应用到新增实际行，首行沿用所在位置。代码块内插入的是原始代码正文，不转义成普通 Markdown；随后仍按真实围栏语法解析，包含的关闭围栏会产生相应结构变化。来自不同生成上下文拼接而成的一个指令不继续隐式展开，并带有诊断；它的字面内容和解析结构仍保留。

`MarkdownSourceMap` 的 spans 使用派生快照中的范围。Original 精确映射至原始 snapshot；Generated 保存最外层原指令范围、可选资源 URI 和可选资源范围。资源原样片段保留精确范围；补出的容器前缀只映射至指令和资源地址。sourceAt／expandedAt 以 Before／After 处理隐藏、替换和删除边界，生成内容对应整个指令；origins(range) 返回裁切后的来源片段，不能据此把生成正文当成可写源文本。ProcessedMarkdown 的所有诊断范围属于其 expanded.snapshot，可通过 sourceMap 返回原指令或实际资源位置。

## 12. mdBook include

`mdBookProcessor()` 与 `mdBookDirectives()` 配套，处理 include，其他命令保留源码并给出诊断。支持完整文件、`:N` 单行、`:N:M` 包含式末行、`::M`、`:N:` 与 anchor。行号为一基，越界、零、反向和非法数字均诊断；物理文件末尾换行不额外产生可选择的末尾空行。所选行之间保持资源的实际换行，最后一行的终止符不进入替换，以复用指令所在行的终止符。空文件得到空片段。

anchor 用 `ANCHOR: name`／`ANCHOR_END: name` 成对标记，允许注释中的标记和嵌套；anchor 选中的正文忽略所有 anchor 标记行，整文件和行号选择仍按原样范围处理。重复同名的完整区段按文件顺序连接；缺失、未配对或错误关闭报告诊断。路径相对包含指令的文件解析，嵌套 include 保留自己的 URI。普通 Markdown、表格与围栏上下文均使用 §11 的派生重解析和来源合同；不以简单别名宣称兼容 rustdoc_include 的隐藏行行为。

## 13. 后续结构与处理

后续结构修改与跨修改的节点身份复用继续按设计增补。普通解析结果里的身份不构成跨独立解析的匹配猜测。保存、修改与解析各自有明确边界，parse 不成为读取快照或源码输入的隐式副作用；公式与 Mermaid 的图像工具不参与 headless 文档的解析／生成合法性。
