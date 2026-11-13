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

`MarkdownChange` 保存 before／after 快照、实际 replacements、inverseReplacements 和 kind（Edit／Reset），提供 `changed` 及 `map(anchor)`。inverseReplacements 引用 after 的坐标，可在当前 after revision 上组成反向事务；不重复执行过滤或结构命令。仅保存实际变化所需的数据和共享快照，不为键入复制全文历史。

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

## 8. 后续编辑与扩展合同

结构修改和扩展处理按设计逐项增补本规范后实现。跨修改的节点身份复用在后续增量解析接口中规定；普通解析结果里的身份不构成跨独立解析的匹配猜测。必要的语法树字段由完整数据结构表达；恢复节点保留源范围和真实文本，不以空处理分支或“尚未支持”占位符代替已声明能力。

保存、修改与解析各自有明确边界，parse 不成为读取快照或源码输入的隐式副作用。公式与 Mermaid 的图像工具不参与 headless 文档的解析／生成合法性。
