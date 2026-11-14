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
| A3：结构化修改、生成 DSL、扩展语法和无 UI 内容处理 | 实施中 | 生成、基础结构事务、扩展语法与带来源内容展开已完成；文档库 74 项、consumer 8 项通过；继续列表／整列、格式化与跨 revision 复用 |
| B：源码编辑、虚拟布局、双栏、统一历史及图像块 | 实施中 | 原生源码视图完成；库内 22 项和 43 步实际窗口用例通过；继续预览、双栏与图像适配 |
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

## B 源码输入片段验证

`MarkdownSourceInput` 将完整活动选区和左右局部上下文接入现有窗口输入协议；native revision 独立于文档 revision。多个原生操作先在临时片段中校验，再发布一次多范围源码事务，保留未改正文的位置映射。组合状态与历史由共享 editor 管理，预编辑更新不单独入栈，接受形成一次记录，取消恢复原文与方向选区；同批次接受后重新开始组合也保持正确历史边界。

本机完成 fmt、check --tests --release 和 `scoop test --release`：3 个容器、19 项通过。新增 10 项覆盖超过 1 MiB 文档中的局部片段、中文与 emoji 预编辑、接受／取消、旧 revision 与非法标量边界、批次原子性、多点修改及中间锚点、周边删除、外部 Edit／Reset、大选区、单一组合归属和订阅异常后的已发布输入。首次 check 修正了可选值嵌套的解包和测试中缺少的扩展方法 import，未修改工具链或系统库。

本批验收窗口协议的状态处理，未据此声称已完成真实窗口／输入法或三模式控件。API 文档与库说明同步；原始日志和临时 API 输出核对后清理，下一项继续实际源码视图。

## B 原生源码视图验证

已提供 `markdownSourceEditor`、可自绘组合的 `MarkdownSourceHost`、独立测量结果、样式与选项。物理行使用完整 Paragraph 排版，以持久化行高索引定位视口；滚动只测可见区域、少量前后行和活动光标行。原生连接绑定整个视图，输入仅发布当前片段；同一共享状态管理选择、组合、历史与重载。保留折行／双向文字的视觉 affinity，支持系统编辑命令、拖选滚动、缩进、长行翻页和只读模式。

跨 Cone 绘制需要 UI 的既有文字记录路径，已先补充规范，再公开 `SceneContext.drawTextLayout` 并由 ParagraphHost 共用。UI 文字缓存 5 项 Scoop 测试通过，重新构建的 UI slib 及声明安装到 `~/.scoop`；没有重建 CLI、runtime 或 Skia。

本机完成相关 fmt、check --tests --release：编辑器库 4 个容器、22 项测试通过，覆盖高度索引的局部插删与旧帧保持、状态／历史／输入批次等。独立 source window Cone 的一个原生窗口用例通过 43 个顺序步骤，使用 1,900,000 字节、100,001 行的中文源码，排版行数始终少于 200；验证系统拼音输入、emoji、离屏组合、换宽试测、丢弃帧、远处光标定位、CRLF 与组合字符删除、RTL 选区、只读、像素绘制、公开 View 的文档重绑及关闭后借用资源仍有效。该用例本机运行约 1.8 秒，是功能验收时间，不作为输入延迟或完整 1／5 MiB 性能基线。

验收发现并修复了输入片段重定位清掉待执行光标显露请求，以及窗口卸载通知继续访问输入路由的问题；补齐长物理行内翻页和折行／RTL 光标方向。`examples/source-editor` 经 check 与 release build，使用公开 View 打开文件并按 Cmd-S 保存；API 文档及查询确认入口、命中结果和绘制方法。规范、库说明和编程导航同步。临时 slib、备份、API 站点与本批日志验收后清理，保留正常构建缓存。

此批完成源码视图；语法高亮、折叠、查找、大纲、富文本、双栏和扩展图像仍随后续完整编辑器继续实现。

## A3 扩展语法验证

已接入带位置开关和固定顺序的 `MarkdownExtension`，提供映射输入、范围匹配、正文属性及未关闭语法诊断。内置公式保存 TeX；front matter 保留 YAML、BOM 与定界符；脚注保留定义／引用，并按正文及已使用脚注中的实际引用分配编号。`mdBookDirectives()` 是普通自定义实现，在行内、整段及明确启用的代码块正文识别指令。识别不执行 include 或图像工具，CommonMark／Gfm 默认语义保持原状。

本机完成 fmt、check --tests --release。文档库 12 个容器、59 项 Scoop 测试通过，包括原有 652 个 CommonMark 和 26 个 GFM 样例；新增 12 项覆盖公式、转义／代码／HTML 隔离、CRLF／BOM、容器边界、未关闭语法、自定义优先级与范围错误、脚注重复／后置定义、Unicode 标签和循环引用。独立 headless consumer 5 个容器、6 项通过，真实跨 Cone 实现一个新识别器，并与公式、mdBook 和脚注组合。编辑器状态层 22 项回归继续通过。

首次 fmt 指出一个 when 分支内 if 缺少分支块，修正后语义检查和运行测试通过；本批没有工具链或系统库修改。规范、README、来源许可、编程导航同步，`scoop doc`／API 查询确认扩展接口和 profile 的公开声明。临时站点与日志验收后清理。

本批完成语法识别和查询。include 的资源读取、带来源展开与 UI 中公式／图像的实际展示继续实施，不将识别成功记为展示完成。

## A3 无 UI 内容处理验证

已实现 `MarkdownProcessingSession`、普通应用 resolver、自定义 `MarkdownProcessor`、资源／合成片段、双向来源定位和带版本的实际依赖。展开返回独立快照及解析，不修改原始文档。每次替换后按实际语法上下文重新解析，后续指令不沿用可能已被围栏改变的旧上下文。精确资源片段保留各自 URI／范围，合成文字与新增容器前缀定位原指令；嵌套 include 的失败同样可回到最外层原指令。

`mdBookProcessor()` 覆盖整文件、一基单行／闭区间／开放区间、成对和重复 anchor、相对路径及嵌套，保留实际换行。缺失、非法范围、未配对 anchor、循环包含和未知命令保留源码并诊断。session 在读取正文前查询更新，命中复用既有结果；缺失资源出现及 I/O 失败重试有独立覆盖。

本机完成相关 fmt、check --tests --release；文档库 15 个容器、74 项 Scoop 测试通过，包括原有标准语料和新增 15 项处理／缓存／来源测试。组合覆盖引用、列表、表格、脚注、缩进与围栏代码、前一个 include 改变后续词法上下文、中文／emoji／混合换行、生成及删除边界、30 层嵌套和允许的局部自包含。独立 consumer 6 个容器、8 项通过，真实文件适配验证读取、独立输出、原文件保留和更新；另一个 Cone 实现自定义 topic 处理器。consumer 的 release build、API 文档及 `MarkdownProcessingSession`／`MarkdownSourceMap` 查询均通过。

规范、README、可运行文件示例和编程导航同步；没有编译器、runtime 或已安装系统库的修改。临时 API 站点、参考下载和本批日志验收后清理。该批完成内容处理层，UI 中的预览、公式／图像与三模式编辑继续实施。

## B 共享后台解析验证

已实现借用 editor、CoroutineExecutor 和 ApplicationProxy 的 MarkdownParseController，以 Idle／Pending／Ready／Failed／Closed 发布状态。工作线程串行解析不可变快照，同时只保留一项运行请求、一份最新待处理请求和一个待交付消息；owner 只接受当前文档与配置的结果。关闭取消订阅并释放排队引用，不在 UI 线程 join，也不关闭借用的 executor。异常可通过新输入或 refresh 恢复。

展开结果新增 sourceParsed，供原文结构编辑直接使用；真实替换后的派生快照具有独立文档身份。资源更新复用未变化原文的解析，纯解析 refresh 复用相同快照与配置的结果，避免把上一份配置的成功结果误作失败重试结果。

本机完成两库 fmt、check --tests --release；编辑器库 5 个容器、23 项 Scoop 测试通过，缓存完善后的后台解析定向复验通过，文档处理 3 个容器、15 项复验通过。新用例运行真实原生 Application 消息循环和独立工作线程，覆盖超过 1 MiB 的快照、连续 1,000 次输入的合并、旧结果退休、选区／保存不触发解析、配置失败及重试、资源刷新与原文解析复用、当前解析上的结构事务，以及非阻塞关闭和应用退出后交付被拒绝时的任务收尾。

规范、README、编程导航与 API 文档同步，公开 MarkdownParseController 声明可查询。本批没有工具链或系统库修改；临时 API 站点和原始日志核对后清理。此批完成解析调度，原生富文本预览、双栏和 WYSIWYG 编辑继续实施。

## B 原生富文本预览验证

已提供 markdownPreview、MarkdownPreviewHost、独立测量结果和预览样式／选项。显示投影在后台生成，保留标题、嵌套文字样式、列表／任务／引用、表格列对齐与引用顺序的脚注；代码按真实行虚拟化，普通段落整体 shaping。实体、转义、隐藏标记、换行和行内原子对象具有双向 UTF-8 映射。源码顺序独立于脚注的显示顺序，生成内容继续沿已有来源映射回到原指令。

相邻结果在后台复用未变化前后块的布局身份，源码范围与节点身份独立更新；资源未变化的 refresh 直接复用整个解析与显示投影。原生视口只持有可见及定位所需排版，表格列宽由整表列数与统一最小宽度决定。滚动锚点保留源码位置和可复用块内文字位置；试测、丢弃帧和旧解析不覆盖当前命中几何。预览借用同一 editor／controller，支持选区、复制所见文字、链接动作与扩展源码定位。

本机完成库、原生测试 Cone 和示例的 fmt、相应 check --tests --release／check --release。库内 6 个容器、29 项 Scoop 测试全部通过；新增覆盖源码映射、脚注排序、表格／列表、CRLF／Tab／围栏、重复内容的独立布局身份、十万行代码、3,000 层引用及资源刷新。首次复用测试替换了包含换行的段落范围，导致真实段落合并；改成只替换链接源码后通过，未修改文档语义来适应错误测试。

独立原生窗口用例通过 12 个连续步骤，包含超过 1 MiB 的中英文文档、十万行代码、远处定位、编辑后的布局复用、换宽试测与丢弃帧、滚动锚点、像素绘制、行内扩展来源、生成内容复制，以及公开 View 的文档重绑和借用资源生命周期；实际排版单位始终少于 100。用例本机运行约 3.1 秒，属于功能验收时间，不代替最终 1／5 MiB 交互延迟基线。公开 examples/preview 已通过 release build，API 文档及 MarkdownPreviewHost 查询通过。

规范、README、示例和编程导航同步；本批无工具链或系统库修改。临时 API 站点和本批日志验收后清理。此批完成普通 Markdown 的富文本预览；图片／公式当前在无展示适配时仍显示替代文字或源码对象，真实扩展图像、WYSIWYG 输入和完整双栏继续实施。
