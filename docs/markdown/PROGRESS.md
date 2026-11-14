# Markdown 文档与编辑器实施记录

日期：2026-11-15。本轮目标收敛为[独立文档 Cone](DESIGN.md)、[核心 Markdown 编辑与扩展框架](../ui/markdown/DESIGN.md#11-本轮交付边界)。实现已按用户要求暂停；最新批次只更新设计范围，具体扩展按需后续增加。

## 实施约束

- 分支：`codex/markdown-editor`。每个完整变更验收后提交、推送，再 fetch 并 merge `origin/main`，继续下一项。
- 本轮只交付核心 Markdown 功能和扩展框架；具体扩展示例不成为新增功能清单。保留已完成能力，用必要的代表用例验证接口，不逐项追齐 Typora，不再扩张 HTML 导入。
- 纯 Scoop 实现，使用 `~/.scoop/bin/scoop --sysroot ~/.scoop`，本机 release 验证。所有库测试使用 `scoop test`。
- 编译器／系统库 bug 或缺口直接修复并重建配套工具链，之后回到库工作；不以库内特殊分支绕过。
- 每一大项先完成实现，再统一格式化、语义检查和相关测试；保留可复用构建结果，清理已不需要的临时文件。

## 当前进度

| 项目 | 状态 | 验收范围 |
| --- | --- | --- |
| A1：源文本、持久化快照、行索引、事务与通知 | 已完成 | 本机 release：文档库 16 项、独立 consumer 2 项测试通过 |
| A2：CommonMark／GFM 解析、源码映射与查询 | 已完成 | 652 个 CommonMark、26 个 GFM 样例；文档库 23 项、consumer 3 项、真实仓库 2 项测试通过 |
| A3：结构化修改、生成 DSL、扩展语法和无 UI 内容处理 | 实施中 | 生成、结构事务、行内／跨块文字和格式、Markdown 片段、块拆分／合并、列表／表格／引用转换及扩展处理已接入；共享不可变快照与局部结构，支持先准备／定位再发布；剩余工作围绕核心读写与扩展框架闭环，不追加非必要辅助功能 |
| B：源码编辑、虚拟布局、双栏、统一历史及图像块 | 已完成 | 三模式、右侧只读、源码锚点滚动同步、共享历史和扩展图像已接入；编辑器库 89 项通过，真实图像工具及原生视图通过 |
| C：普通 Markdown 所见即所得编辑 | 实施中 | 行内／跨块输入与格式、多格式剪贴板、链接／图片面板、代码语言／高亮、块／列表／表格菜单及三模式共用历史已接入；真实 IME、菜单、字段和焦点回归见下文，待完成核心编辑的剩余问题及必要组合验收 |
| D：扩展框架 | 实施中 | 复用已有图片、代码、公式、Mermaid、指令映射与显式局部源码；统一片段、索引、多行块接入与光标自动源码编辑已更新设计，尚待实现；具体扩展不新增为本轮交付项 |
| E：核心与框架的组合、性能及材料 | 实施中 | 已有真实语料与长段测量、示例及编程材料；最终本机 1／5 MiB 交互基线、长段专项及核心与框架的组合验收待完成 |

原设计的 Rust／Ropey／pulldown 后端已按用户要求改为纯 Scoop；三种编辑模式、独立文档与大文档目标保留。历史记录中的后续扩展计划以最新范围为准，不继续作为本轮验收要求。构建／测试原始输出不提交到本记录。

## A1 验证

使用 `~/.scoop` 的 `0.0.0-dev.20261114` 工具链（`306c818a04`），本机 aarch64-apple-darwin。已完成受影响 Cone 的 fmt、check --tests --release，随后执行：

- `scoop test sysroot/src/scoop.markdown --release`：4 个容器、16 项通过。
- `scoop test sysroot/src/scoop.markdown/examples/source --release`：独立依赖消费程序的 2 项通过。
- `scoop doc` 生成本库 API，`scoop api show MarkdownDocument` 确认声明、来源及 import 可查询。

覆盖无损字节读写、混合换行与跨叶片 CRLF、UTF-8 边界、不可变快照、差量／逆向事务／锚点、失败不发布、回调重入及异常、400 次混合编辑和平衡树不变量、超过 1 MiB 中文文本的局部编辑与子树共享。这里验证的是文档层；尚未据此声称 UI 性能目标达成。

## A2 前置工具链修复

解析器的可选值分支暴露出 `Unit`／`()` 无操作语句的缺口。已先明确语言规范，再允许 HIR lowering 接受 Unit literal 语句；其他非调用值仍不因此成为合法语句。同步了 Option 示例和编程材料。

已使用仓库现有 LLVM 22.1.8 SDK 重建本机六个配套命令，安装到 `~/.scoop`；当前本机开发标识为 `0.0.0-dev (6289e706e3+unit-statements)`。不涉及 IR 格式、runtime 或现有预编译系统库的契约变更。Rust fmt／相关 clippy 完成（有变更外既有 lint 提示）；Core consumer 的 check --tests --release 通过，`scoop test ... --release --filter UnitStatementTests` 的 2 项原生测试通过，覆盖两种写法、Some／None、嵌套分支、if 和 finally 的执行行为。

此修复已独立抽取到基于 main 的 `codex/fix-unit-statements`，经 [PR #133](https://github.com/scoop-lang/scoop/pull/133) 合并至 main；Markdown 工作分支已合入并发布新的 main。修复 PR 仅包含编译器、规范、编程参考与相应测试。

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

## B 图片与扩展图像验证

已实现 MarkdownImageRenderer、共享图像 controller、可见 lease 和独立资源快照。接受帧才声明工作，多视图共享相同请求；版本检查先于读取或外部渲染。后台只传回编码字节，owner 创建原生 Image；旧帧保持自己的图片，退休缓存或关闭 controller 不使仍在使用的测量结果失效。等待和失败保留上一次成功结果，失败可重试，关闭不在 UI 线程等待工具进程。

本地图片使用真实 file URI、百分号解码和 include 来源位置。Mermaid 按带完整源码范围的扩展图像块处理；公式、自定义行内／块扩展保留正文与 attributes。内置外部适配调用配置好的 mmdc，以及 MathJax 与 rsvg-convert；公式使用排版工具的实际逻辑宽高和基线。带编号的整行公式保留 MathJax 内部百分比布局，正文与编号共同光栅化。库运行时不自动下载工具。

本机完成相关 fmt、check --tests --release；编辑器库 6 个容器、30 项 Scoop 测试通过。独立 images Cone 的 2 项测试通过，包含真实 PNG 解码与像素绘制、公式正文／编号像素和行内 caret advance、include 相对图片、自定义扩展属性、Mermaid 正文输入，以及队列共享、版本复用、失败重试、旧帧持有、试测不启动任务和非阻塞关闭。使用 mmdc 12.2.0、Chrome 157.0.8103.78、Node 26.13.0、MathJax 4.1.3、rsvg-convert 2.63.4；两项运行合计约 2.5 秒。外部工具预先显式准备，Mermaid 通过 PUPPETEER_EXECUTABLE_PATH 使用现有 Chrome。

原有预览窗口的 12 步组合复验通过，约 3.0 秒。此只读用例通过公开命令接口操作，准备条件改为组件的逻辑焦点归属，保留重绑与焦点断言；macOS 拒绝后台窗口激活不再阻塞没有原生键盘输入的用例。原生输入／IME 的测试仍要求真实窗口焦点。

公开预览示例支持显式配置 Mermaid／公式工具，release build 和 API 查询通过。规范、IMAGES.md、README、示例与编程导航同步；无编译器、runtime 或系统库修改。临时 API 站点与本批日志核对后清理，保留后续验收需要的 MathJax 工具安装及正常构建缓存。普通 Markdown 的所见即所得输入、图像块内源码编辑与三模式联动继续实施。

## A3／C 行内语义编辑验证

已提供 parsed.inlineContent、MarkdownInlineContent.replaceText／setStyle 和 MarkdownInlineEdit。段落、标题、表格单元格的显示范围可跨普通文字、嵌套格式、代码与链接；图片、HTML、公式及自定义扩展保持原子源码对象。准备结果带普通源码差量、编辑后选区和局部显示映射；不修改文档、不持有 UI 资源，也不重新解析整篇文档。单叶片普通修改保留外围标记，结构重建保持块外源码、换行和容器前缀；隐式引用改名保留原目标，不齐表格复用现有补列路径。

MarkdownEditorState.apply 接受该结果，源码和语义命令共享同一历史，undo／redo 恢复正反选区。错误文档或旧 revision 不发布。局部行内解析保留原块使用的引用、脚注与扩展上下文，include 仍不在解析时执行。

本机完成两库和独立 consumer 的 fmt、check --tests --release。文档库原有 74 项回归通过；新增行内用例定向完成最终 16 项验证，覆盖源码最小差量／精确逆向、Unicode／实体、定界两侧插入、跨样式替换、嵌套格式、代码围栏／表格竖线、隐式及显式链接、原子对象、空标题、缺失单元格、CRLF 容器、软／硬换行和脚注／公式／指令组合。另用 1,320,009 字节、40,001 个段落的文档确认操作保留块外源码；含首次完整解析的此项用例约 0.42 秒，不作为交互延迟基线。

首次验证发现对相邻粗体内的普通字母使用实体编码会改变关闭定界符的匹配。局部写回改为保留稳定的物理字符分类，必要时走结构重建；补齐普通文字邻接下的空格、标点、中文和 emoji 组合后通过。格式变化同时保持软／硬换行，不让换行落到新格式片段的非法边缘。

编辑器库 7 个容器、32 项测试全部通过；headless consumer 6 个容器、9 项通过，实际跨 Cone 使用准备结果及格式命令。API 文档生成与 MarkdownInlineContent／MarkdownEditorState.apply 查询通过；规范、README、消费示例和编程导航同步。本批无编译器或系统库修改，临时站点与日志核对后清理。原生富文本输入片段、跨块事务和完整三模式仍继续实施。


## C 连续行内批次与富文本输入片段

MarkdownInlineEdit 提供已准备的 change 和 after 内容；applyTo 发布同一不可变源快照，连续操作不等待全文解析。content.edit 按当前显示坐标准备一批修改，合为一次源码事务；restore 恢复同一行内区域的确切源码。局部上下文独立保存容器前缀、换行、空标题间隔和表格所属行，继续输入或取消不会丢失这些源信息。

MarkdownRichInput 将显示文字接入窗口原生批次协议，复用同一 editor 的组合与发布路径。源码／富文本只有一个组合拥有者，预编辑、接受、取消、提交后重开组合和外部修改使用同一历史。普通连续输入立即复用 after 的新映射；原生 Unicode 范围错误不留下半次修改，代码换行与 NUL 规范化同步更新 native 选区。源文档订阅在发布后抛错时，片段仍跟随已发布快照。

本机使用 ~/.scoop 工具链，fmt、受影响 Cone 的 check --tests --release 通过。文档库 17 个容器、96 项全部通过（新增 6 项）；编辑器原有 32 项回归通过，新增富文本输入 11 项通过；独立 headless consumer 6 个容器、9 项通过。覆盖连续准备／发布、同 revision 分支失效、精确恢复、空标题、缺失表格单元格、格式边界双向偏向、组合与外部编辑、观察者异常以及 Unicode／原子范围。API 生成与 MarkdownRichInput、MarkdownInlineEditBuilder、MarkdownInlineEdit.after 查询通过；规范、README、消费示例与编程导航同步。

本批没有编译器、runtime 或系统库修改。临时 API 站点与日志核对后清理；普通构建缓存保留。MarkdownRichInput 是不拥有窗口的输入片段；原生所见即所得视图、块级命令与三模式联动继续实施。

## A3 局部语法读取、反向锚点与空文档输入

MarkdownInlineContent.nodes 公开当前局部树的顶层行内节点，供处理程序与视图取得最新样式和对象；源码范围属于该内容的快照，不改变完整 ParsedMarkdown 的节点身份。MarkdownChange.mapBack 复用正向映射规则及反向差量，将更新后的坐标映射到基线，保留 Before／After 偏向并检查 UTF-8 边界。

零字节文档可通过 parsed.inlineContent(parsed.root.id) 准备一个空行内区域。创建区域不增加文档节点或源码；首字输入按普通段落文字转义，取消和再次输入仍走同一准备／发布路径。非空 Document 根继续拒绝作为行内目标。

使用 ~/.scoop 完成 fmt、check --tests --release；已有 97 项文档库回归及随后新增的 2 项空文档测试全部通过。覆盖反向锚点、Unicode 边界、空文件首字转义、局部恢复、重新输入和完整旧树不变。API 文档生成与 mapBack／MarkdownInlineContent.nodes 查询通过。规范、README 同步；无工具链或系统库修改。图形宿主的原生窗口验收独立继续，不计入本批已完成范围。

## A3 表格列操作与矩形粘贴

MarkdownTransformBuilder 支持插入／删除列、设置列对齐、替换文字矩阵和粘贴 TSV。未编辑单元格保留原始行内 Markdown，GFM 忽略的多余单元格随列移动，扩列不会意外显示它们。矩形操作按需要补齐行列，复用普通源码事务并可精确撤销；引号字段可包含 Tab、换行和双引号。表格写回保留容器前缀、局部换行和文件末尾状态，必要时在分隔符前补空格，避免吞掉原单元格末尾的反斜杠。

使用 ~/.scoop 完成文档库及独立 consumer 的 fmt、check --tests --release；文档库 21 个容器、113 项 Scoop 测试全部通过，consumer 6 个容器、10 项全部通过。新增 14 项覆盖列边界、对齐、缺失及多余单元格、CRLF 引用／列表、Unicode 与转义、矩形扩展、TSV 引号规则、非法参数原子性及精确逆向。原 CommonMark／GFM 语料及下游 12 项显示投影测试继续通过。API 文档生成及 insertTableColumn／setTableCells 查询通过，规范、README、消费示例和编程导航同步。

本批没有编译器、runtime 或系统库修改。临时 API 站点与日志核对后清理，普通构建缓存保留；富文本原生输入及完整三模式交互继续实施。

## C 原生富文本宿主与活动行显示

已提供 markdownRichEditor 与 MarkdownRichHost，复用预览虚拟视口、样式、图片和资源所有权。活动段落／标题／单元格直接显示局部语法树的新内容，其他行共享后台投影；新旧源坐标沿实际差量映射。一个文档级输入区域连接 MarkdownRichInput，共享现有选区、组合与历史；空文档可直接输入。活动行离开视口仍持有组合排版，试测和丢弃帧不污染已接受的命中及候选窗几何。

使用 ~/.scoop 完成 fmt、库与原生测试 Cone 的 check --tests --release；编辑器库 48 项测试已分批通过，表格解析调整后的 12 项显示投影继续通过。覆盖局部投影与完整树分离、相邻表格映射、HTML／脚注原子坐标、外围标记选区和空文件。原有预览窗口的 12 步回归通过。API 生成及 MarkdownRichHost／markdownRichEditor 查询通过，规范、README 和编程导航同步。

富文本窗口的 24 步真实输入测试通过，约 3.66 秒：大于 1 MiB 文档、中文／emoji 组合及取消、同一连接跨帧与离屏保持、格式修改、所见复制、undo／redo、连续退格、组合字符删除、单元格竖线转义、只读切换、RTL 与 Page 导航、公共 View 生命周期及空文件首字输入。后台裸程序启动时 macOS 未给予焦点；使用 scoop test 生成同一测试程序，再由 scoop bundle 打包、Launch Services 启动后通过。验收保留真实系统焦点及输入连接要求，核对 scoop.test 的 begin／end／complete 与空 failures，未以启动成功代替测试结果。复现步骤见富文本测试 Cone 的 README。

本批无编译器、runtime 或系统库修改。局部 Enter／Tab、跨块替换、完整指针交互与三模式继续实施；本记录不将行内编辑等同完整 WYSIWYG 交付。临时应用包、API 站点及原始日志核对后清理，正常构建缓存保留。

## A3 列表层级与标记转换

已提供 indentListItem、outdentListItem、unwrapListItem、setBulletList 和 setOrderedList。列表解析保留实际前缀位置与部分 Tab 消费信息；结构命令按行修改前缀、任务框及必要分隔，不重新生成正文。嵌套提升保留父条目，后续兄弟与父条目尾部内容归到提升后的条目；解除列表保留所在引用等容器。显式有序转换负责连续编号及续行宽度，普通层级移动不重编号其他条目。

使用 ~/.scoop 完成文档库与 consumer 的 fmt、check --tests --release。文档库已有 113 项回归通过，新增列表 16 项最终全部通过；全量首次执行 128／129 通过，剩余用例误将 `* *` 当作普通段落，改为按 CommonMark 验证其嵌套列表后，16 项定向复核通过。独立 consumer 11 项及下游 12 项显示投影通过。覆盖源码锚点、精确逆向、任务与空项、同一物理行的父子标记、Tab／懒续行、CRLF 引用、代码／Mermaid／脚注、引用定义、编号跨位数和范围上限、分隔线歧义及失败原子性。API 生成与 indentListItem／setOrderedList 查询通过，规范、README、消费示例及编程导航同步。

本批没有工具链或系统库修改。临时 API 站点和日志核对后清理，正常构建缓存保留；原生块命令与三模式组件继续接入这些文档操作。

## B／C 三模式组件与联动

已提供 MarkdownEditorViewState、MarkdownEditorOptions 和 markdownEditor。源码、所见即所得和双栏使用同一文档、选区、组合与历史；切换模式接受当前组合一次，不重新序列化源码。两侧保留独立滚动和焦点状态，支持右侧只读、可调栏宽、显示指定源码范围和可选双向滚动同步。同步以已接受帧的源码锚点和行内比例定位，等待匹配的 revision；布局修正不反向驱动另一侧。源码可见行测量继续迭代至高度稳定，避免窄栏换行后视口缺行。

组合视图只订阅一次正文通知，宿主回调不因两栏重复。原生验证发现文档通知中更新输入快照会误触正文事务的重入检查，已将这种派生同步改为检查 owner、存活与当前快照；新的正文修改和关闭仍禁止重入。独立观察者测试覆盖程序修改、撤销／重做和非法重入。

使用 ~/.scoop 完成 fmt、库与测试 Cone 的 check --tests --release；编辑器库 11 个容器、55 项 Scoop 测试全部通过。三模式原生窗口的 29 步验证通过，约 3.50 秒：大于 1 MiB 文档、双侧真实输入、中文组合切换、撤销／重做、右侧只读、源码定位、双向同步与关闭同步、栏宽变化、丢弃帧、更换 view state、单次通知和借用资源卸载。使用 scoop test 生成测试程序，scoop bundle 打包并由 Launch Services 启动，核对 begin／end／complete 与空 failures。

新增带模式工具栏、保存和退出快捷键的公开示例，实际打开 641,186 字节的仓库规范文件并检查双栏显示。三份示例统一按 Scoop main 的 argv[0] 约定读取文件参数，格式化与 release 检查通过。API 生成及 MarkdownEditorViewState／markdownEditor 查询通过；规范、设计、README、测试复现说明和编程导航同步。

本机现有 ui.style、ui.component、ui.style.shadcn 预编译产物与已安装 ui 的语义指纹不一致，已用 ~/.scoop 中的工具链和原有安装源码重建配套产物；未修改这些系统库源码，也没有新增编译器或 runtime 修复。临时应用包、API 站点、诊断用文件读取 Cone 和原始日志验收后清理，保留正常构建缓存。块级 Enter／Tab、跨块替换、完整指针交互和最终性能验收继续实施。

## A3／C 块拆分与合并事务

已提供 ParsedMarkdown.splitBlock、joinBlocks 和 MarkdownBlockEdit。拆分使用显示字节选区，闭合两侧行内样式并保留标题、引用与列表容器；列表延续原标记，新任务框未勾选，编号跨位时调整必要续行。空条目退出一级。相邻同容器段落／标题可合并，保留首块种类，ATX 标题中的换行转为空格；代码、扩展和引用定义等块边界保持明确。准备结果带源码差量与结果 caret，不发布文档或重新解析整篇；MarkdownEditorState.apply 将结果作为独立历史项。

使用 ~/.scoop 完成相关 fmt、check --tests --release。文档库首次全量 148 项中 147 项通过，唯一失败样例缺少空行而被 CommonMark 识别为 Setext 标题；修正样例并补充多行 HTML 边界后，20 项块编辑定向测试全部通过，连同已通过的 129 项回归共覆盖当前 149 项。编辑器库 12 个容器、58 项全部通过；独立 consumer 6 个容器、12 项全部通过。覆盖 Unicode／实体／原子对象、引用与脚注、CRLF、标题标记、任务与嵌套列表、编号上限、过期或其他文档事务、一次通知、组合输入及精确撤销。另以大于 1 MiB 文档验证块外源码保持，约 0.36 秒包含首次完整解析，不作为交互延迟指标。

API 生成及 MarkdownBlockEdit／splitBlock／joinBlocks／MarkdownEditorState.apply 查询通过；规范、设计、README、consumer 示例与编程导航同步。本批无工具链或系统库修改，临时 API 站点和日志核对后清理，保留正常构建缓存。此批完成文档操作及共享历史，原生 Enter／Tab 和跨块连续输入继续接入。

## A3／C 局部结构区域与空白输入

已提供 MarkdownSyntax、MarkdownRegion 和 ParsedMarkdownRegion。完整解析选择包含完整顶层块及相邻块的区域，区域沿源码差量推进后按需解析，保持原文档身份与全局字节坐标。引用定义按原顺序解析，删除区内定义后可采用区外后续定义；区外位置偏移延迟到解析时处理，键入不遍历所有定义。相同 revision 数字的未发布分支不能作为当前结构事务。空列表项和普通空白行可通过 inlineContentAt 取得输入片段，保留任务前缀及精确恢复。

使用 ~/.scoop 完成相关 fmt、check --tests --release。文档库首次全量 159 项中 157 项通过；修复空任务项的插入点，校正代码块范围断言后，8 项区域和 24 项行内测试通过。随后针对延迟定义映射、未发布分支和文件末尾中文／emoji 字节定位的区域复验通过，原有 149 项回归保持通过。大于 1 MiB、80,000 段尾部的用例以扩展实际调用次数确认局部命令只解析小于 100 字节的区域，约 0.31 秒包含首次完整解析，不作为 UI 交互指标。

编辑器原有 58 项通过，新增区域事务与共享撤销组合的 4 项定向复核通过，当前共 59 项分批通过；独立 consumer 6 个容器、13 项通过。覆盖区域内拆分／合并与连续文字修改、引用和脚注、列表／表格／数学组合、CRLF、文档中段 front matter 排除、空文件首字、代码及扩展内部拒绝普通输入、区外更新／Reset／过期分支和精确撤销。UI 用例遵守既有源码锚点规则，未把整段替换误作前缀插入。

API 生成及 MarkdownRegion／MarkdownSyntax.nodeAt／inlineContentAt／MarkdownEditorState.transform 查询通过；规范、设计、README、消费测试与编程导航同步。无编译器或系统库修改。临时站点与原始日志验收后清理；原生视图的局部行片段、Enter／Tab 与跨块选择继续实施。


## A3／C 原生块命令与局部多行显示

Enter 已接入段落／标题拆分、列表续项和空项退出，边界 Backspace／Delete 合并同容器相邻段落。Tab／Shift-Tab 调整列表层级或导航表格，末格 Tab／末行 Enter 新增表格行，含 Tab 的剪贴板内容按 TSV 矩形写入。命令先准备源码差量、解析局部结果并定位光标，再一次发布到统一历史；撤销／重做恢复结果光标。相应公开入口为 MarkdownSnapshot.prepare、MarkdownSyntax.prepareTransform、MarkdownChange.applyTo 及 MarkdownEditorState.apply(change, selection)。

局部区域沿真实差量链保持当前快照，多行投影只替换覆盖的行片段，保留区外布局及未变化的 mdBook 展开；脚注位于文档末尾的显示顺序仍正确。命中、复制、源码定位、双向滚动及高度索引消费同一投影；普通键入只更新活动行。新段落、空任务退出和引用空行可立即输入，后台完整解析接替时不会用空光标行覆盖前一段。全文和局部解析可并发使用语法扩展，文档与编程导航明确其不可变配置／内部同步要求；资源处理器与 renderer 继续后台串行。

使用 ~/.scoop 完成 fmt、相关 Cone 的 check --tests --release。文档库 28 个容器、162 项全部通过；独立 consumer 6 个容器、14 项全部通过。UI 首次全量 67 项通过，补充交接回归后 9 项结构测试通过，当前 68 项分批通过。覆盖事务准备不发布、相同 revision 的分支身份、UTF-8 选区失败原子性、EOF／引用空行、样式拆分与合并、表格导航／TSV／撤销、脚注非连续显示行和 mdBook 展开复用。

真实窗口的 33 步富文本验收通过，约 1.95 秒；包含大于 1 MiB 文档中的连续分段、输入、边界合并、立即翻页、列表调整、表格新增行与撤销，以及已有 IME／RTL／只读／试测／卸载组合。首次新增用例在原生输入事件交付前断言，按现有事件与布局确认规则修正后通过。三模式原有 29 步回归通过，约 3.35 秒。两者均使用 scoop test 生成的程序、scoop bundle 和 Launch Services，核对真实 begin／end／complete 与空 failures。

API 生成及 prepare／prepareTransform／MarkdownEditorState.apply 查询通过，规范、README、消费示例与编程导航同步。本批无编译器或系统库修改。原始日志、临时应用包和 API 站点验收后清理，正常构建缓存保留；跨块替换、局部源码编辑、其余 Typora 编辑操作与最终性能验收继续实施。

## A3／C 跨块文字事务与原生选区

已提供 MarkdownSyntax.textContent、MarkdownTextContent 和 MarkdownTextEdit。单段复用行内编辑，多块区域按源码顺序提供显示文字与结构分隔映射；段落／标题及同级列表项支持跨块替换，保留两端未选内容和样式。不同容器与表格网格保留边界，代码、扩展与定义为原子对象。整篇选中可替换或清空正文，顺序批次只发布一次，restore 与逆向事务恢复原始字节。补齐段落合入标题后仍保留后续块边界，以及局部删除后空标题的插入点映射。

MarkdownRichInput 共用上述文档操作，按准备结果的真实 insertedRange 处理输入法选区；跨块中文预编辑、接受、取消、剪切、普通文字粘贴、删除和撤销使用同一历史。没有活动行时仍保留局部块显示。待后台解析时可将当前区域扩大到全篇，结束全选后立即回到单段输入；普通局部修改不展开整篇原生文字。

使用 ~/.scoop 完成受影响 Cone 的 fmt、check --tests --release。文档库首次全量 182 项中 180 项通过，两项断言误锁定序列化标记拼写；改为核对显示文字和格式语义后，20 项文字编辑、9 项块合并、最终 5 项文字事务测试通过，当前 184 项分批通过。UI 首次全量 77 项中 76 项通过；修正组合期间 Undo 先取消组合的既有合同断言，并补充历史分组、大文档局部输入与立即全选后，最终 37 项富文本定向测试通过，当前 80 项分批通过。独立无 UI consumer 15 项全部通过，实际消费跨块准备、发布和恢复接口。

原生富文本窗口的 47 步通过，约 2.86 秒，包含超过 1 MiB 文档中输入后立即全选并回到单段、跨段中文组合与取消、格式保留、剪切／粘贴、末块为代码时整篇替换、清空后继续输入、表格跨单元格预编辑及公开 View 的既有组合。首次运行遇到等价实体转义的固定源码断言，改为验证文字、格式和区外原文后通过。三模式窗口 29 步回归通过，约 3.32 秒；两者均使用 scoop test 程序、scoop bundle 和 Launch Services，核对 begin／end／complete 与空 failures。另以超过 1 MiB 文档确认小跨段输入上下文小于 4 KiB，含完整解析的测试约 0.30 秒；这些时间仅记录功能验收，不代替最终交互延迟基线。

API 生成与 textContent／replaceText／restore 查询通过；规范、设计、README、consumer 示例和编程导航同步。本批无编译器、runtime 或系统库修改。临时日志、应用包与 API 站点验收后清理，正常构建缓存保留。跨块格式化、Markdown 片段粘贴、代码／扩展局部源码面、查找替换及最终 1／5 MiB 性能验收继续实施；普通文字粘贴尚不等同于结构化 Markdown 片段插入。

## C／D：代码与扩展的局部源码面

富文本组件默认在同一栏内打开代码、公式、Mermaid、HTML 和自定义指令的局部原始源码。它借用同一文档、选区、撤销与后台解析；保留围栏／定界符，未闭合内容不会使面板消失，Done 或无组合时 Escape 返回富文本焦点。显式 onSource 仍由宿主接管，showSource 仍显式切换到完整源码栏。独立富文本组件沿用显式样式合同，不新增主题上下文要求。

新增 MarkdownSourceRegion 跟随源码差量移动；源码输入、命中、排版及键盘命令按该范围裁剪，首尾可在物理行中间。全选、删除、缩进和翻页保留区外源码；外部编辑、组合取消、撤销重做沿同一历史处理，Reset 或关闭使区域失效。大区域仍只排版可见行和光标行，原生输入只保留局部上下文。代码与原始扩展行保存完整节点的源码入口，显示行复用后也更新其实际范围。

使用 ~/.scoop 完成 fmt、受影响 Cone 的 check --tests --release。UI 库 89 项全量通过，补齐显示行复用的源码入口后 8 项投影定向回归通过。原生源码窗口 52 步通过，约 2.06 秒；局部源码窗口 35 步通过，约 1.00 秒；原有富文本 47 步回归通过，约 2.82 秒；三模式 29 步回归通过，约 3.60 秒。全部使用 scoop test 程序、scoop bundle 与 Launch Services，并核对 begin／end／complete 和空 failures。源码首轮测试在撤销退休连接后过早注入文字，补充连接与布局交接后通过；局部源码首轮发现后台行复用丢失入口范围，修复及回归后通过。

API 生成及 MarkdownSourceRegion／MarkdownSourceInput／markdownSourceEditor 查询通过，规范、设计、README、原生测试复现说明及编程导航同步。本批无编译器、runtime 或系统库变更。原始日志、临时应用包和 API 站点核对后清理，保留正常构建缓存。跨块格式化、Markdown 片段粘贴、代码语言选择／高亮、查找替换与最终大文档性能验收继续实施。

## C：选区格式与后续输入样式

文档 Cone 新增 MarkdownStyleState，以及 MarkdownTextContent／edit builder 的跨块 setStyle 与 styleState。粗体、斜体和删除线按相交行内块一次准备，保留显示选区、容器、表格网格、原子扩展与区外源码；批次给新文字加格式时保留 insertedRange。格式状态区分 Off／On／Mixed／Unavailable，跳过纯空白和结构分隔，独立程序可直接消费。

MarkdownRichInput 支持折叠光标的后续输入样式，不预先写入空标记；格式命令接受当前组合，组合取消恢复原始源码并保留待输入样式。RichHost 与 viewState 提供格式命令和可追踪状态，Primary-B／I／Shift-X 分别切换粗体／斜体／删除线。可选 markdownFormattingToolbar 已接入三模式示例，显示混合状态、尊重只读与纯源码模式，并在点击后恢复富文本焦点。连续键入引入格式定界符或邻接转义时，历史复用文档分块文本保存已修改片段，仍可一次撤销；不逐键复制全文或拼接累计字符串。

使用 ~/.scoop 完成受影响 Cone 的 fmt 与 check --tests --release，文档库 191 项、UI 库 96 项、独立无 UI consumer 16 项全量通过。UI 首轮发现新建格式定界符会拆开连续输入历史，修复后补充非空原文的精确恢复并全量通过；另一项误锁定等价实体拼写的断言改为核对语义与已接受快照。超过 1 MiB 的 40,000 段样本只格式化小于 256 字节的局部上下文，包含完整解析的测试约 0.43 秒，不作为交互延迟指标。

原生格式窗口 35 步通过，约 1.97 秒；局部源码 35 步回归通过，约 0.97 秒；原有富文本 47 步通过，约 2.76 秒；三模式 29 步通过，约 3.40 秒。全部使用 scoop test 生成程序、scoop bundle 与 Launch Services，核对 begin／end／complete 和空 failures；覆盖真实快捷键、按钮点击、反向跨块选择、连续中文／emoji、组合接受与取消、只读、模式切换和卸载。

API 生成及 MarkdownStyleState／MarkdownTextContent.setStyle／styleState、MarkdownRichInput.setStyle、MarkdownEditorViewState.toggleStyle 和 markdownFormattingToolbar 查询通过。规范、设计、README、消费示例、原生复现说明及编程导航（含本机工具链副本）同步。本批无编译器、runtime 或系统库修改；临时日志、应用包与 API 站点核对后清理，正常缓存保留。行内代码转换、Markdown 片段粘贴、代码语言选择／高亮、链接和图片操作、查找替换及最终 1／5 MiB 性能验收继续实施。

## C：Markdown 片段的系统剪贴板前置能力

片段复制需要同时向其他应用提供所见纯文本和可解析 Markdown，原来的窗口剪贴板仅支持纯文本。已在 scoop.window 补齐 ClipboardText、writeTexts 与按媒体类型读取，先完成原生多格式、UTF-8／NUL、失败与大文本验收，再用 ~/.scoop 重建受影响的 14 个 Cone。具体合同与平台边界见[窗口实施记录](../windowing/PROGRESS.md#多格式文本剪贴板)。

Markdown 富文本原生测试直接消费安装库的新 API，并保存恢复原系统剪贴板全部格式。Markdown 文档片段的独立导出、保留结构的插入及相应编辑器命令继续在文档／UI Cone 中实施；本批的多格式传输不计为这些编辑操作已经完成。

使用 ~/.scoop 完成 fmt、受影响测试 Cone 的 check --tests --release，UI 库 16 个容器、96 项 release 测试全部通过。原生富文本窗口 47 步通过，约 2.98 秒，包含安装库多表示读写、默认纯文本粘贴、剪切与撤销，以及超过 1 MiB 文档的原生输入／IME／虚拟化回归。程序由 scoop test 生成，经 scoop bundle 与 Launch Services 启动；begin／end／complete 完整，failures 为空。此批修正系统库缺口，没有新增编译器或 runtime 变更。

## C：Markdown 片段与多格式剪贴板

文档 Cone 提供 MarkdownSyntax.exportMarkdown／replaceMarkdown，两个接口均使用源码 UTF-8 坐标。局部导出补齐样式、直接链接／图片目标和传递脚注定义，保留列表、引用、表格及代码／扩展源码；整篇选择保留原始字节。插入保留两侧内容，继承容器、按矩形写表格，脚注冲突一并改名。MarkdownFragmentEdit 带一次 change、结果 selection 和已准备的局部 after；发布复用同一快照，精确撤销，未引入 UI 依赖或第二份历史。

UI 默认复制／剪切同时提供所见 text/plain 与语义 text/markdown，生成内容保留原指令；默认粘贴优先 Markdown，源码面原样插入，普通文本按字面编辑。Host／ViewState 的 executeClipboard 提供 CopyText、CopyMarkdown、PasteText、PasteMarkdown、PasteTableTsv；TSV 为显式矩形命令，菜单按当前栏路由并返回焦点。剪切先成功写出再删除，一次粘贴独立撤销；只读保留复制。

使用 ~/.scoop 完成受影响 Cone 的 fmt 与 check --tests --release。文档库 208 项全量通过，修正源码关闭标记边界后新增一项并回归全部 18 项片段测试，当前 209 项分批通过；UI 库 98 项全量通过，最终修复后 2 项片段历史回归通过；无 UI consumer 17 项全量通过。超过 1 MiB、60,000 段的样本只修改小于 128 字节的范围，局部解析小于 256 字节；包含完整解析的测试约 0.48 秒，不作为交互延迟基线。

验证修正了删除后空单元格裁剪引起的粘贴落点、空输入位置的节点归属，以及从源码模式选择关闭标记时误合并下一段的问题。脚注循环、区外定义、名称冲突、单元格内脚注、原子末块、空文档／列表／表格、CRLF 容器、格式和精确恢复均有 Scoop 行为测试。

最终富文本原生窗口 61 步通过，约 3.23 秒；三模式窗口 35 步通过，约 3.55 秒。使用 scoop test 生成程序、scoop bundle 与 Launch Services，核对 begin／end／complete 和空 failures。覆盖实际剪贴板多表示、纯文本／Markdown／TSV、空格式、只读、模式路由、一次撤销、输入连接退休及既有大文档／IME 场景；两个测试保存恢复原系统剪贴板。首次富文本运行在撤销后尚未接受新上下文时执行下一命令，拆为两个已就绪帧后通过；三模式发现的标记边界缺陷已修复并用最终代码复验。

API 严格生成与 exportMarkdown／replaceMarkdown／MarkdownFragmentEdit／executeClipboard 查询通过；规范、设计、README、无 UI 示例、原生复现说明和编程导航（含安装副本）同步。本批没有编译器、runtime 或系统库变更。原始日志、临时应用包及 API 站点核对后清理，正常缓存保留。HTML 导入、行内代码转换、代码语言选择／高亮、链接和图片操作、查找替换及最终大文档性能验收继续实施。

## C：行内代码与链接／图片编辑

文档 Cone 新增 setCode／codeState；单段和跨块转换保留显示文字、换行边界、其他样式、链接及原子对象。replaceText 的可选 code 在插入规范化前生效，显式退出代码后保留换行，两种 builder 保留实际 insertedRange。行内 setLink／removeLink／replaceImage 以显示范围准备一次源码事务，保留两侧、共享引用定义与精确逆向；图片结果为一个映射的 U+FFFC 对象，URI 转义沿用生成器规则。

RichInput、RichHost 和 viewState 提供行内代码状态、命令和后续输入模式，Primary-E 与 Code 按钮使用同一选区和历史。代码可与粗体等样式组合；组合取消、换行规范化及反向跨块选区保持一致。表格中无法表示为代码 span 的转换在发布前拒绝，高层命令返回 false；已经发布后的观察者错误仍正常传递。

可选格式工具栏增加 Link／Image 模态面板，编辑显示文字／替代文字、URI 和标题，支持取消与移除，关闭后释放草稿并恢复富文本焦点。RichHost 与 viewState 的 canEditTarget、beginTarget、applyTarget、removeTarget 可供自建面板使用。MarkdownTargetDraft 保存实际快照与选择；文档、选区或输入状态已变时不应用旧范围。修改引用只作用于当前使用点；未改动的文字样式和行内对象保留，文件读写由宿主负责。入口限于一个行内块，自动链接不提供移除按钮。

使用 ~/.scoop 完成 fmt、受影响 Cone 的 check --tests --release：文档库 219 项、UI 库 107 项和独立无 UI consumer 17 项全量通过。新增测试覆盖代码／粗体／链接组合、反引号、中文、换行与组合区、不可表示的表格内容、引用使用点、图片外层链接、URI／标题转义、光标偏向和草稿失效。最初几处测试误锁定 URI 空格及等价 Markdown 拼写，改为规范化 URI 与实际解析语义后通过。

最终原生格式窗口 56 步通过，约 2.60 秒；链接／图片窗口 35 步通过，约 1.94 秒；既有富文本 61 步回归通过，约 3.01 秒；局部源码 35 步回归通过，约 0.99 秒。全部通过 scoop test 构建、scoop bundle 与 Launch Services 启动独立进程，核对 begin／end／complete 和空 failures。覆盖真实快捷键和工具栏键盘导航、面板原生字段、一次撤销、取消、恢复焦点、草稿失效、只读，以及接受原生组合后退休连接并继续编辑；宿主最终拒绝路径和面板收尾也在这些窗口中验证。

公开 API 生成及 setCode、MarkdownTargetDraft、canEditTarget、applyTarget、replaceImage 查询通过。规范、设计、库 README、编辑器示例、原生复现说明及编程导航（含本机工具链副本）同步。本批无编译器、runtime 或系统库修改；原始日志、临时应用包与 API 站点核对后清理，正常构建缓存保留。代码语言选择／高亮、其他块操作入口、查找替换和最终 1／5 MiB 综合性能验收继续实施。


## C：代码块语言、信息字段与语法高亮

文档 Cone 提供 setCodeInfo，编辑完整单行信息字段。已有围栏仅修改必要字段与冲突定界符，保留正文、容器、换行和未闭合状态；缩进代码可转换为安全围栏，处理列表、引用、脚注、Tab 与 CRLF。相同解码信息保留原拼写，逆向事务恢复精确源码。转换只替换实际代码缩进及节点内容，不改同一行的祖先标记。

UI 增加 MarkdownCodeHighlighter／MarkdownCodeSpan／MarkdownCodeKind。默认纯 Scoop 实现复用 scoop.regex，以 Prism 的 MIT 词法规则为参考，覆盖 Scoop、Kotlin、C／C++、Rust、Shell、JSON、TOML、Python、JavaScript／TypeScript、YAML、SQL 及常见别名。它提供平坦词法颜色，字符串插值等细分语义可由宿主提供器扩展；未知语言仍显示完整原文。高亮仅在串行后台解析阶段调用，未变化代码块按提供器身份、语言和实际正文复用，接收结果时检查必要的 UTF-8 范围；失败仅产生该块诊断和普通文字回退。

原始与展开文档各有绑定 snapshot 的 sourceHighlights／previewHighlights，无展开时共用结果。映射逐段避开列表／引用前缀，Tab 沿既有 TextDisplay 处理。预览、三模式源码和局部源码共用颜色数据，独立 Source View／Host 可传 highlights；过期结果不染色新正文。MarkdownCodeTheme 提供 Light、Dark 与自定义颜色，不改变文字、字体度量、选区或历史。

局部代码面提供原生 Language / info 字段、Apply、Copy code 和 Done。Enter 与 Apply 提交普通文档事务；Copy code 仅复制正文；提交后恢复源码焦点。字段草稿在正文修改与后台解析期间保留，包括尚未提交的原生组合；等待只禁用结构提交。外部信息改变而草稿已编辑时显示冲突提示，不覆盖新信息。非代码扩展沿用局部源码面。

使用 ~/.scoop 完成 fmt、相关 check --tests --release 和示例 check。文档库全量 223 项首轮 221 项通过，修正信息字段转义预期与脚注代码样例后，4 项代码信息回归全部通过；其余实现未变。UI 库 117 项全量通过，随后 10 项代码高亮／草稿回归全部通过；独立无 UI consumer 17 项通过。6,000 行中英混合代码的两次解析与投影测试约 64.5 毫秒，正文未变时高亮器仅调用一次；这不是键入延迟或最终 1／5 MiB 综合基线。

最终局部源码原生窗口 54 步通过，约 1.48 秒；富文本 61 步约 3.07 秒、格式 56 步约 2.57 秒、链接／图片 35 步约 1.95 秒均通过。窗口以 scoop test 构建、scoop bundle 和 Launch Services 串行启动，核对 begin／end／complete 与空 failures，剪贴板恢复原内容。验收修复了未变化显示状态反复通知造成的布局等待，并确保后台更新不取消语言字段的 IME；源码 Shift-Tab 按既有取消缩进语义验证，工具栏从字段通过 Tab／Space 操作。

公开 API 严格生成及 setCodeInfo、MarkdownCodeHighlighter、sourceHighlights 查询通过；规范、设计、README、编辑器说明、原生复现说明、Prism 许可及编程导航同步。本批没有编译器或 runtime 修改。原始日志、临时应用包和 API 站点在核对后清理，正常构建缓存保留。其他块操作入口、查找替换、文档导航与最终 1／5 MiB 综合性能验收继续实施。

## C：原生块、列表与表格菜单

文档 Cone 提供连续同级块的 wrapBlockQuote、wrapBulletList、wrapOrderedList、wrapTaskList 和 setFencedCode，以及单层 unwrapBlockQuote。包装保留原始正文、行内写法、代码围栏、扩展与本地换行；转换代码时以行内 Markdown 作为字面正文。相邻块保留必要分隔，标题转段落不会吸收前后正文。解除引用按各层实际前缀处理缩进及部分 Tab；引用与列表标记同一行时把子块放到续行并调整本条目的前缀，保留代码正文、后续块和精确逆向。

RichHost／viewState 的 blockState、executeBlock 接收 MarkdownBlockCommand。可选 markdownBlockToolbar 提供 Block／List／Table 菜单，接入三模式示例；Primary-0／1…6 转换段落／标题。命令覆盖引用、列表类型与任务状态、层级、解除条目与重编号、代码、分隔线、表格插入及行列／对齐。表格保留表头与至少一列，增删后定位到有效单元格；代码转换进入局部源码。命令先准备当前局部结果，再接受组合并一次发布，菜单关闭后恢复焦点。状态读取已有投影并复用节点及选区上下文，普通状态查询不触发同步全文解析。

使用 ~/.scoop 完成 fmt、相关 check --tests --release 和编辑器示例 check。文档库当前 231 项分批通过，最终 29 项块操作回归通过；UI 库当前 126 项分批通过，最终 13 项块命令／结构历史回归通过；独立无 UI consumer 18 项全量通过。验收修复空列表项、空单元格与文字末尾的节点归属，以及引用解除后的缩进误判。超过 1 MiB、60,000 段样本的标题转换只使用小于 256 字节的局部树、修改小于 128 字节的原范围；含全文解析的测试约 261 毫秒，不作为交互延迟基线。

最终原生窗口串行通过：块操作 42 步约 1.63 秒，富文本 61 步约 2.99 秒，格式 56 步约 1.94 秒，链接／图片 35 步约 1.87 秒，局部源码 54 步约 1.39 秒。程序由 scoop test 构建，经 scoop bundle 和 Launch Services 启动；每项 begin／end／complete 完整且 failures 为空。覆盖真实快捷键、列表／表格菜单、中文组合、空行／列中的继续输入、对齐和删除定位、一次撤销、只读／源码模式拒绝及现有大文档、焦点和卸载回归。

公开 API 严格生成及 MarkdownBlockCommand、markdownBlockToolbar、wrapBlockQuote、executeBlock 查询通过；规范、设计、README、消费示例、原生复现说明及编程导航（含安装副本）同步。本批没有编译器、runtime 或系统库修改。原始日志、临时应用包及 API 站点核对后清理，保留正常构建缓存。查找替换、文档导航、源码辅助、HTML 导入与最终 1／5 MiB 综合性能验收继续实施。

## C：全文源码查找与原生替换（2026-11-15）

文档 Cone 提供 MarkdownSearchQuery、snapshot.search 和 MarkdownSearchResult，复用 scoop.regex，支持字面／正则、大小写、Unicode 整词、循环导航和按范围惰性查询。结果保存实际快照身份与 UTF-8 范围，prepareReplace／prepareReplaceAll 生成普通 MarkdownChange；正则捕获模板在准备时展开，全部替换只消费原匹配集合，精确 inverse 保留原始换行与指令。搜索不依赖解析或 UI，不展开 include。

MarkdownSearchController 借用 viewState／executor／proxy，合并后台查询，废弃变化后的替换范围；隐藏后正文键入不继续全文扫描。markdownSearchBar 与三模式组件接入原生字段、大小写／整词／正则选项、计数、快捷键及可见范围高亮，查询字段保持独立历史。替换遵守活动栏只读，经共享 editor 一次发布；全部替换独立撤销。验收修正 Shift-Enter 的单行字段路由，并用明确的物理 F 键绑定替换组合，处理 macOS Option 改变逻辑字符的行为。后台测试独立启动 Application，遵守一进程一次主循环的合同。

使用 ~/.scoop 完成 fmt、相关 check --tests --release 与编辑器示例 check。新增文档搜索 7 项通过，文档库当前 238 项分批通过；UI 库 127 项全量通过，无 UI consumer 19 项全量通过。超过 5 MiB 的源码完成跨分块搜索、6,599 处稀疏替换和精确恢复，整项测试约 241 毫秒；此数值包含准备、发布和撤销，不是交互延迟基线。原生后台查询测试通过约 131 毫秒，覆盖连续 200 次配置、过期结果、失败恢复及运行中关闭。

最终原生窗口串行通过：查找替换 41 步约 1.65 秒，富文本 61 步约 3.01 秒，格式 56 步约 2.44 秒，链接／图片 35 步约 1.93 秒，局部源码 54 步约 1.39 秒，块操作 42 步约 1.60 秒。查找用例覆盖真实中文组合取消、逐字输入、正反导航、替换字段与按钮、统一撤销重做、模板错误、三模式只读及超过 1 MiB 文档末尾在两侧的实际滚动。程序由 scoop test 构建，经 scoop bundle 与 Launch Services 启动；各项 begin／end／complete 完整且 failures 为空。

API 严格生成及 search、prepareReplaceAll、MarkdownSearchController、markdownSearchBar 查询通过；规范、设计、README、消费示例、原生复现说明与编程导航同步。本批没有编译器、runtime 或系统库修改。原始日志、临时应用包及 API 站点核对后清理，正常构建缓存保留。文档导航、源码辅助、HTML 导入与最终 1／5 MiB 综合性能验收继续实施。

## C／D：标题索引、大纲、目录和原生位置导航（2026-11-15）

文档 Cone 提供 MarkdownNavigation／MarkdownHeading、Unicode 标题锚点与章节范围、脚注 fragment、独立目录生成，以及零基物理行／标量列转换。Editor 默认识别 [TOC]，保存保留原指令。UI 在后台建立原文与最终预览索引，未展开时共享，资源更新复用未变的原文索引；目录包含扩展生成标题，跳转沿来源映射回所属指令。

MarkdownNavigationController 管理标题、行列、内部链接及 128 项导航历史，普通修改映射已有选区，Reset 清空；后退恢复方向，与正文撤销独立。markdownOutline 使用 VirtualList，markdownNavigationBar 使用原生字段和 Primary-L；目录与脚注接入内部跳转。旧大纲在解析待完成时禁用，三模式及只读共用源码定位。链接点击保留跳转前选区；活动富文本标题遵守既有可编辑文字边界，源码与只读保留原始目标。

使用 ~/.scoop 完成相关 fmt、check --tests --release 和示例 check。文档库全量 245 项、UI 库全量 130 项、无 UI consumer 全量 20 项通过。超过 5 MiB、6,000 个标题的解析／索引／行列往返／目录生成用例约 238 毫秒，仅作功能测试记录，不替代最终交互基线。验收校正 include 来源范围不包含终止换行，以及富文本隐藏标记归一到文字边界的测试预期。

最终原生窗口 319 步通过：导航 30 步约 1.85 秒、富文本 61 步约 3.11 秒、格式 56 步约 1.60 秒、链接／图片 35 步约 1.92 秒、局部源码 54 步约 1.39 秒、块操作 42 步约 1.75 秒、查找 41 步约 1.63 秒。另有导航历史／生命周期及后台查询各 1 项通过。程序由 scoop test 构建，使用 scoop bundle／Launch Services 按独立进程串行启动；各项 begin／end／complete 完整且 failures 为空。新增覆盖原生行列输入、错误与组合取消、大纲／目录点击、只读后退起点、旧索引拒绝，以及超过 1 MiB 文档在两侧的末尾滚动。

规范、设计、README、示例、原生复现说明及编程导航同步；严格 API 文档生成和 MarkdownNavigation、MarkdownNavigationController、markdownOutline、tableOfContentsDirective 查询通过。编程导航安装副本已同步。本批没有编译器或 runtime 修改。原始日志、临时应用包及 API 站点核对后清理，保留正常构建缓存。标题／front matter 折叠、源码辅助、HTML 导入和其余 Typora 扩展、最终 1／5 MiB 综合性能验收继续实施。

## C／D：标题、代码与 front matter 折叠（2026-11-15）

UI 在后台建立 MarkdownFoldIndex，MarkdownFoldingController 管理每个视图的折叠状态；markdownEditor、独立 View／Host 和 markdownFoldingToolbar 共用该状态。标题与 front matter 同时折叠源码和预览，代码围栏仅折叠源码，局部源码面保留完整内容。折叠不改正文或撤销；修改头部或 Reset 展开，展开父项保留子项状态，搜索／导航／程序化光标自动展开目标。中文组合范围不能被收起，边界删除先展开隐藏正文，显式跨区选区仍操作完整范围。

布局保留展开时的持久高度树，通过整段零高度范围跳过隐藏行；预览行索引处理 include、文末脚注和局部结构替换。旧快照的预览沿用自身折叠几何，过期命中不能改变当前状态。大于 1 MiB、30,000 段的原生样本收起后仅测量不超过 6 个源码行和 3 个预览块，展开后仍只测量视口附近内容。另有百万行高度树及大段局部修改组合覆盖；这些验证不替代最终 1／5 MiB 交互延迟基线。

使用 ~/.scoop 完成相关 fmt、check --tests --release 与编辑器示例 check。UI 库全量 138 项通过。原生窗口共 450 步通过：新增折叠 44 步、富文本 61 步、格式 56 步、链接／图片 35 步、局部源码 54 步、块操作 42 步、查找 41 步、导航 30 步、源码 52 步、三模式 35 步；另有导航状态和后台查询各 1 项通过。全部由 scoop test 构建，scoop bundle／Launch Services 串行启动，begin／end／complete 完整且 failures 为空。新增折叠用例约 1.47 秒。验收修复逻辑字符移动在同一字节位置因 affinity 改变而提前停止、导致富文本退格合并隐藏正文的问题，并固定测试搜索栏的容器以保持编辑器焦点。

严格 API 文档生成及 MarkdownFoldIndex、MarkdownFoldingController、sourceFolds、markdownFoldingToolbar 查询通过；规范、设计、README、示例、原生复现说明与编程导航（含安装副本）同步。本批没有编译器、runtime 或其他系统库修改。原始日志、临时应用包和 API 站点核对后清理，正常构建缓存保留。源码辅助、HTML 导入、其余 Typora 扩展和最终 1／5 MiB 综合性能验收继续实施。

## C／D：源码高亮、行号与输入辅助（2026-11-15）

MarkdownParsedDocument.sourceSyntax 在后台从已有语法与代码 token 派生源码颜色，MarkdownSourceHighlights 提供原始 UTF-8 范围和相交查询；嵌套样式、标记、代码颜色保持优先次序，过期结果不用于新正文。源码 View／Host、三模式和局部源码面共用结果，MarkdownSourceTheme 支持浅色、深色及自定义颜色。可选物理行号只为可见行生成布局，折行只显示一次，局部面保持全文行号；测量、命中、caret 和折叠按钮共用正文起点。编辑器示例已开启行号。

autoIndent／autoPairs／autoFences 分别控制保留缩进、括号配对和空围栏补全。括号支持包裹反向 Unicode 选区、跳过本视图补出的右括号和空配对退格；边界判断读取光标附近原文，不依赖原生片段大小。代码围栏仅在当前后台语法确认尚无正文且未闭合时补齐，不同步解析或等待后台。辅助与原输入一次发布、统一撤销，粘贴、显式范围、多项批次、IME 和底层 MarkdownSourceInput.apply 保持原样输入。配对状态随位置映射，离开范围、标记修改、撤销、Reset、失焦或禁用后退休。

使用 ~/.scoop 完成 fmt、相关 check --tests --release、编辑器示例检查和 UI 库全量 155 项测试。新增 17 项覆盖缓存与颜色映射、原生片段为零／一个字节、长转义串、Unicode、反向选区、撤销、局部范围、CRLF／Tab 和围栏边界；大于 5 MiB 的文档验证配对仍使用局部片段，大于 1 MiB 的文档验证末尾围栏单点修改。验收修复同一语法快照上的失败高亮被误复用、导致刷新不重试的问题；只读原生输入的测试预期校正为拒绝。

原生窗口共 488 步通过：新增源码辅助 38 步，以及既有源码、三模式、富文本、局部源码、格式、链接／图片、块操作、查找、导航和折叠 450 步；另有导航状态与后台查询各 1 项通过。新增窗口覆盖真实输入和中文组合、一次撤销、各辅助开关、字面粘贴、行号像素、折叠命中、末尾导航、试测释放、卸载与借用资源。大于 1 MiB、36,000 段的样本只测量少于 100 个源码行，收起后不超过 6 行；这些是功能验证，不替代最终交互性能基线。程序均由 scoop test 构建，通过 scoop bundle／Launch Services 串行运行，begin／end／complete 完整且 failures 为空。

严格 API 文档生成及 MarkdownSourceHighlights、MarkdownSourceTheme、MarkdownSourceOptions、sourceSyntax 查询通过；规范、设计、README、示例和编程导航（含安装副本）同步。本批没有编译器、runtime 或其他系统库修改。原始日志、临时应用包和 API 站点核对后清理，正常构建缓存保留。HTML 导入、其余 Typora 扩展和最终 1／5 MiB 综合性能验收继续实施。

## C／D：独立 HTML 导入与原生富文本粘贴（2026-11-15）

文档 Cone 提供 importMarkdownHtml、MarkdownHtmlImportOptions 和 MarkdownHtmlImportResult，复用现有 HTML token 扫描、WHATWG 实体表与 Markdown 生成器。支持常见块、文字格式、链接图片、代码、列表任务框和普通矩形表格；片段标记与诊断使用原 HTML 的 UTF-8 范围。HTML 空白、历史无分号实体、数值 C1 映射及常见省略结束标签按定义恢复；显式 baseUri 只解析目标，不访问资源。CSS 仅转换明确的文字语义；未知元素、profile 关闭的语法、复杂表格与无法表示的编号保留 HTML。实现不依赖 UI、浏览器或外部转换程序，Turndown 转换规则参考及许可已记录。

富文本默认粘贴按存在性选择 Markdown、HTML、普通文本；显式 PasteHtml 在两种输入面均可用，源码默认继续原样插入 Markdown／普通文本。转换结果通过已有语义事务发布，空表示不回退，组合提交与粘贴分别可撤销，菜单路由遵守只读和生命周期。复制、剪切与三模式保存继续使用同一原始文档。

使用 ~/.scoop 完成 fmt、相关 check --tests --release、编辑器示例 check。文档库全量 269 项、UI 库全量 155 项和独立 consumer 21 项通过；表格复查修复后，HTML 24 项定向复验通过。新增模型覆盖大于 1 MiB、22,000 段的导入／语义插入／精确撤销，以及 2,048 层布局容器和重复格式、UTF-8 片段、实体、畸形 HTML、无头表格与保留原始子树。复查将含不能转换的删除线或无 src 图片的表格保留为整张 HTML，避免其中原始竖线破坏 GFM 单元格。

原生窗口 235 步通过：新增 HTML 剪贴板 33 步，以及源码 52 步、富文本 61 步、局部源码 54 步、三模式 35 步。新增用例验证真实 Cmd-V、三格式优先级、显式 HTML／纯文本、空表示与反向选区、一次正文通知、精确撤销／重做、源码模式、两类表格、只读、中文组合、CommonMark profile 和卸载；系统剪贴板在每次命令前设置并核对，结束恢复原内容。验收校正段首空格由生成器编码为实体的显示语义预期，并将字符索引的测试预期改为 UTF-8 范围。新增窗口再次独立复跑通过；所有窗口由 scoop test 构建、scoop bundle／Launch Services 串行启动，begin／end／complete 完整且 failures 为空。

严格 API 文档生成及 importMarkdownHtml、MarkdownHtmlImportOptions、MarkdownHtmlImportResult、MarkdownClipboardCommand／PasteHtml 查询通过。规范、设计、README、消费示例、原生复现说明及编程导航（含安装副本）同步。本批没有编译器、runtime 或其他系统库修改。原始日志、临时程序、应用包和 API 站点核对后清理，保留正常构建缓存。其余 Typora 扩展、HTML 展示和最终 1／5 MiB 综合交互性能验收继续实施。

## 范围确认：核心 Markdown 与两种扩展展示（2026-11-15）

按用户明确的范围，重点是 Typora 自带的 Markdown 编辑与展示能力，不要求完整复刻或逐个内置其额外语法。已有公式、Mermaid 和 mdBook 能力保留；上下标、高亮、emoji 简码等通过扩展接口接入，历史记录中的“其余 Typora 扩展”不再表示逐项内置交付要求。

展示合同采用 Inline／Block 两种形态。行内对象需要真实宽度与基线，块对象需要宽度约束、可见范围和高度变化处理；同一种公式或图片可提供两种展示。原生代码保留文本选择、复制、高亮和按行虚拟化，图像 renderer 复用为一种适配。文档层的语法与处理接口保持无 UI；代码正文的识别位置属于语法上下文。已核对现有 MarkdownImageRequest.inlineContent、图像基线、原生代码投影及 MarkdownExtension／MarkdownProcessor，设计与当前进度同步；公共展示入口的归一和核心整体交付继续实施。本次只修改设计与范围记录，未运行无关的全量测试。

## 设计更新：扩展识别与自动源码编辑（2026-11-15）

文档设计明确两种识别入口：匹配标准 Image／CodeBlock 等节点，或在 parser 的合法行内／块上下文识别新增语法；两者输出同一类扩展片段。片段保留 Inline／Block 形态、完整源码范围、结构化数据与来源，引用式图片的定义依赖不混入当前使用点。新增按位置／相交范围查询的索引设计，并明确多行块的开始、继续、结束与容器规则。已有单段落匹配不是完整块扩展接入点，统一片段和索引尚未实现。

UI 设计明确 Rendered／SourceEditing 两种交互状态，由编辑器统一处理光标进入／离开、边界偏向、IME 和共享历史。Inline 在原段落展开源码，Block 在原块展开；不完整语法和迟到渲染不打断输入。补齐双栏、只读、跨范围选择、嵌套、显式完成与原子对象的进入规则；图像与原生代码分别复用现有展示能力，不增加通用 UI 组件插件要求。

本批只更新两份设计及本记录，检查 Markdown 结构、相对链接和差异空白；未修改实现或已交付 API 规范，未运行编译和库测试。后续按文档识别与索引、展示适配归一、自动编辑投影的顺序实施；实现任务保持暂停。

## 范围收敛：核心功能与扩展框架（2026-11-15）

按用户最新要求，本轮完成条件限定为独立文档核心、三模式 Markdown 编辑与展示，以及可实际接入的扩展框架。保留标准节点／新增语法两种入口、统一片段与索引、Inline／Block 展示、自动源码编辑、无 UI 内容处理、来源映射与资源生命周期等抽象，以便以后独立增加具体扩展。

公式、Mermaid、mdBook、脚注／目录、front matter 和 HTML 导入的已有实现保留；只做框架调整所需的适配与回归，不扩充公式、图表或模板命令兼容。取消设计中把具体 renderer、完整扩展集或新增 HTML 能力作为本轮完成条件的表述。框架用已有图片／代码、行内语法和自定义指令的代表用例验证，不按内置扩展数量验收。非必要辅助功能与其他平台验证也不阻塞本轮。

本批仅更新设计、交付清单及本记录，检查文档结构、链接与差异；未修改实现或 API 规范，未运行编译和库测试。实现任务继续暂停。
