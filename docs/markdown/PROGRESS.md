# Markdown 文档与编辑器实施记录

日期：2026-11-14。目标仍为[独立文档 Cone](DESIGN.md)与[三模式编辑器](../ui/markdown/DESIGN.md)的完整设计。

## 实施约束

- 分支：`codex/markdown-editor`。每个完整变更验收后提交、推送，再 fetch 并 merge `origin/main`，继续下一项。
- 纯 Scoop 实现，使用 `~/.scoop/bin/scoop --sysroot ~/.scoop`，本机 release 验证。所有库测试使用 `scoop test`。
- 编译器／系统库 bug 或缺口直接修复并重建配套工具链，之后回到库工作；不以库内特殊分支绕过。
- 每一大项先完成实现，再统一格式化、语义检查和相关测试；保留可复用构建结果，清理已不需要的临时文件。

## 当前进度

| 项目 | 状态 | 验收范围 |
| --- | --- | --- |
| A1：源文本、持久化快照、行索引、事务与通知 | 已完成 | 本机 release：文档库 16 项、独立 consumer 2 项测试通过 |
| A2：CommonMark／GFM 解析、源码映射与查询 | 已完成 | 652 个 CommonMark、26 个 GFM 样例；文档库 23 项、consumer 3 项、真实仓库 2 项测试通过 |
| A3：结构化修改、生成 DSL、扩展语法和无 UI 内容处理 | 实施中 | 生成 DSL 与基础结构事务已完成；文档库 47 项、consumer 5 项通过；继续列表／整列操作、格式化与扩展处理 |
| B：源码编辑、虚拟布局、双栏、统一历史及图像块 | 实施中 | 共享状态与历史 9 项通过；继续原生输入、源码虚拟视图与图像适配 |
| C：普通 Markdown 所见即所得编辑 | 待实施 | 表格、列表、链接、跨块操作、中文 IME 和三模式撤销 |
| D：公式、指令和 Typora 编辑能力补齐 | 待实施 | 扩展来源映射、正确展示、错误恢复与编辑操作 |
| E：性能、生命周期、示例与编程材料 | 待实施 | 本机 1／5 MiB 基线、长段专项及完整组合 |

原设计的 Rust／Ropey／pulldown 后端已按用户要求改为纯 Scoop；文档行为、扩展和三种编辑模式的目标保持。构建／测试原始输出不提交到本记录。

## A1 验证

使用 `~/.scoop` 的 `0.0.0-dev.20261114` 工具链（`306c818a04`），本机 aarch64-apple-darwin。已完成受影响 Cone 的 fmt、check --tests --release，随后执行：

- `scoop test sysroot/src/scoop.markdown --release`：4 个容器、16 项通过。
- `scoop test sysroot/src/scoop.markdown/examples/source --release`：独立依赖消费程序的 2 项通过。
- `scoop doc` 生成本库 API，`scoop api show MarkdownDocument` 确认声明、来源及 import 可查询。

覆盖无损字节读写、混合换行与跨叶片 CRLF、UTF-8 边界、不可变快照、差量／逆向事务／锚点、失败不发布、回调重入及异常、400 次混合编辑和平衡树不变量、超过 1 MiB 中文文本的局部编辑与子树共享。这里验证的是文档层；尚未据此声称 UI 性能目标达成。

## A2 前置工具链修复

解析器的可选值分支暴露出 `Unit`／`()` 无操作语句的缺口。已先明确语言规范，再允许 HIR lowering 接受 Unit literal 语句；其他非调用值仍不因此成为合法语句。同步了 Option 示例和编程材料。

已使用仓库现有 LLVM 22.1.8 SDK 重建本机六个配套命令，安装到 `~/.scoop`；当前本机开发标识为 `0.0.0-dev (6289e706e3+unit-statements)`。不涉及 IR 格式、runtime 或现有预编译系统库的契约变更。Rust fmt／相关 clippy 完成（有变更外既有 lint 提示）；Core consumer 的 check --tests --release 通过，`scoop test ... --release --filter UnitStatementTests` 的 2 项原生测试通过，覆盖两种写法、Some／None、嵌套分支、if 和 finally 的执行行为。

## A2 验证

本机已完成受影响 Cone 的 fmt、check --tests --release，以及：

- 文档库 7 个容器、23 项 Scoop 测试通过；其中一项完整比对 CommonMark 0.31.2 的 652 个官方样例，另一项覆盖 26 个 GFM 扩展样例（包括其一个不崩溃用例）。
- 原子源码与快照测试继续通过；新增实体／转义／CRLF／NUL 映射、代码 span 规范化、Unicode 引用、解析缓存、节点查询和 3,000 层容器遍历测试。
- 独立 source consumer 2 个容器、3 项通过，验证公共语法树、映射与定义范围可以跨 Cone 使用。
- 仓库实际文档用临时纯 Scoop 测试 Cone 验收，2 项通过。`SCOOP-SPEC.md` 为 546,814 字节、3,962 行、12,591 节点、190 标题、85 代码块；`SCOOP-IMPL-SPEC.md` 为 641,186 字节、2,343 行、5,061 节点、27 标题、2 代码块。验证所有节点／标记范围、子节点包含关系和原文往返。两项包含文件读取及全树检查的本机运行分别约 66／42 ms，仅作本轮记录，不代替后续 1／5 MiB 与 UI 专项基线。
- `scoop doc` 与 `scoop api show ParsedMarkdown` 确认 API 可生成和查询。语料、移植算法及命名实体数据保留来源与许可。

本批尚不包含跨 revision 身份复用、结构化修改、builder、脚注／数学／指令扩展或 UI。它们仍按 A3 与后续项目继续实施。临时 API 站点、原始日志和仓库文档测试 Cone 验收后清理；正常 Cone 构建缓存保留。

## A3 生成 DSL 验证

已实现 `markdownDocument`／`markdownText` 与 block、inline、list、table builders。生成路径处理普通文字、强调／删除线、代码围栏、链接／图片、嵌套引用和列表、任务条目、表格以及换行选项；不影响已有文档的逐字节保存。规范、README、编程导航和独立 consumer 同步更新。

本机完成相关 fmt、check --tests --release；文档库 8 个容器、34 项 Scoop 测试通过，独立 consumer 3 个容器、4 项通过。除原有标准语料外，新增生成后解析回读、100 组 Unicode／标点／空白格式边界、围栏冲突、表格与链接组合，以及无效结构参数测试。真实 consumer 从另一个 Cone 使用 receiver DSL 生成并检查标题、格式、任务、Mermaid 源码块和表格。

`scoop doc` 与 `scoop api show MarkdownDocumentBuilder` 确认新声明、来源和 import 可查询。临时 API 站点、参考下载和本批原始日志验收后清理，保留后续脚注需要的上游扩展语料。

集成时同步修正三处解析／映射细节：GFM 网址在实体和转义解码前识别，邮箱仍在文字解码后识别；实体空格不被当作源码行尾空格删除或转成 hard break；邮箱目的地址中自动补出的 `mailto:` 前缀遵守 Before／After 源码偏向。652 个 CommonMark 和 26 个 GFM 样例继续通过。本批未改变工具链或系统库。

结构事务、显式格式化、跨 revision 复用和扩展语法仍在 A3 后续范围内，UI 仍待后续项目。

## A3 基础结构事务验证

已实现当前解析上的 `document.transform(parsed)`：标题／段落、行内内容和整节点格式、链接使用点／共享定义、任务框、表格单元格与行操作都转换为同一套源码事务。准备阶段稳定排序、检查范围与冲突后一次发布，支持原有的通知、锚点和精确逆向修改。Paragraph／Heading／TableCell 同时保存去掉容器前缀的行内源码映射；`parent(id)` 提供直接父节点查询。

本机完成相关 fmt 与 check --tests --release；文档库 10 个容器、47 项 Scoop 测试通过，独立 consumer 4 个容器、5 项通过。原有 CommonMark／GFM、生成、持久化文本与映射测试继续通过。新增覆盖同批编辑的源码顺序和原子性、其他文档／旧 revision／重叠范围拒绝、CRLF 与嵌套列表／引用、ATX／Setext／空标题、相邻格式合并、引用定义共享、空任务项、不齐表格补列以及表格行的增删与边界。

首次组合验收发现空 ATX 标题与空列表项插入时缺少分隔空格，修复后针对性及最终回归通过；一个没有被 Markdown 识别为粗体的测试输入已改成实际的嵌套粗体语法。未修改工具链或系统库。规范、README、编程导航、跨 Cone 消费测试与 API 文档同步；`scoop api show transform` 核对公开签名和 import。临时 API 站点与原始日志验收后清理。

本批不把任意选区格式化等同于完整行内节点的 wrap／unwrap。整列、列表层级、显式格式化、跨 revision 复用、扩展处理及 UI 仍继续实施。

## B 共享编辑状态验证

开始独立的 `scoop.ui.markdown` Cone，使用同一个借用文档管理双向源码选区、统一历史、结构事务及保存 revision。连续输入的历史保存改动片段，undo／redo 各通过一个文档事务恢复，并保留修改前后的选区。外部修改、Reset、关闭以及订阅抛错的行为已覆盖。

跨 Cone 接入发现 `State.checkMutation` 原为 internal，其他 UI 控件无法在修改外部模型之前检查 owner 和 UI 修改时机。先补充 UI 规范，再公开既有无副作用检查；setter 仍自行验证。本机 UI 状态观察测试 2 项通过，UI 库已重新构建并安装到 `~/.scoop`，声明来源已同步。测试需要的 Skia 开发头文件从仓库同版构建补入安装目录，没有重建 Skia 或更改 CLI。

新 Cone 完成 fmt、check --tests --release 和 `scoop test --release`：2 个容器、9 项通过。其中在超过 1 MiB 的中文文档中连续输入 1,000 次，并用一次 undo／redo 恢复，旧快照保持有效。`scoop doc`／API 查询核对公开声明。这里仍是共享状态验收；原生输入、实际三模式控件和 UI 性能继续实施。临时 slib、API 站点和本批原始日志核对后清理，安装资源与构建缓存保留。
