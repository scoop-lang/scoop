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

## 6. 后续语法与生成合同

解析、节点查询、结构修改、Markdown builder 和扩展处理按设计逐项增补本规范后实现。必要的语法树字段由完整数据结构表达；恢复节点保留源范围和真实文本，不以空处理分支或“尚未支持”占位符代替已声明能力。

保存、修改与解析各自有明确边界，parse 不成为读取快照或源码输入的隐式副作用。公式与 Mermaid 的图像工具不参与 headless 文档的解析／生成合法性。
