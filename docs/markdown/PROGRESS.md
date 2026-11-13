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
| A3：结构化修改、生成 DSL、扩展语法和无 UI 内容处理 | 待实施 | 读取→修改→生成及 mdBook 风格处理 |
| B：源码编辑、虚拟布局、双栏、统一历史及图像块 | 待实施 | 两份最大 spec 编辑保存，真实 Mermaid 图像适配 |
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
