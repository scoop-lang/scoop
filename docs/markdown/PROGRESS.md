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
| A2：CommonMark／GFM 解析、源码映射与查询 | 待实施 | 完整解析语义与真实仓库样本 |
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
