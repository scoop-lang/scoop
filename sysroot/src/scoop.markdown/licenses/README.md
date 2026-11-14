# Markdown 第三方资料

解析器是纯 Scoop 实现，没有调用外部 Markdown 解析器。

- 块容器、强调分隔符栈和链接括号栈参考／移植自 [commonmark.js](https://github.com/commonmark/commonmark.js)，revision `898886961b49c1df1175666d03c3ac66c9cc9040`（2026-10-13）。保留原 [BSD-2-Clause 许可及附带声明](commonmark-js.txt)。Scoop 实现另行保存精确 UTF-8 源码映射。
- `tests/data/commonmark-0.31.2.json` 为 [CommonMark 0.31.2 官方 652 个样例](https://spec.commonmark.org/0.31.2/spec.json)，原文保留；作者 John MacFarlane，[CC-BY-SA 4.0](commonmark-spec.txt)。测试中的 HTML 投影仅用于核对这些期望。
- `tests/data/gfm-extensions.json` 从 [cmark-gfm 的 extensions.txt](https://github.com/github/cmark-gfm/blob/master/test/extensions.txt) 提取表格、删除线、自动链接、互操作和任务列表样例，转为 JSON；原作者 Yuki Izumi，原文声明 CC-BY-SA 4.0。`tests/footnotes.scoop` 的脚注容器输入也改编自同一文件，使用结构断言验证。HTML tag filter 属于 HTML 输出策略；脚注是独立 profile 开关，未混入 Gfm 的默认语义。
- `src/parser/data/entities.tsv` 从 [WHATWG HTML 命名字符实体表](https://html.spec.whatwg.org/entities.json) 的 2,125 个带分号条目生成，列为名称与十进制 Unicode 标量。来源表下载于 2026-11-14；`src/html/data/legacy-entities.txt` 从 2026-11-15 下载的同一表提取 106 个可省略分号的历史名称，复用带分号条目的字符数据。HTML 导入使用这组额外规则，Markdown 的实体规则保持不变。[WHATWG 许可](whatwg-html.txt) 允许其中纳入源码的部分按 BSD-3-Clause 使用。
- HTML 的常见块与行内转换参考 [Turndown](https://github.com/mixmark-io/turndown) 的 CommonMark rules，revision `b86e2b18539fa4922acf7001902255e2ce5bf0b9`（2026-11-04），保留 [MIT 许可](turndown.txt)。实现使用现有 Scoop HTML token 扫描、实体表和 Markdown 生成器，另外处理 profile、原始子树、UTF-8 范围及表格；不依赖 JavaScript 或浏览器 DOM。
