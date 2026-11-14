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
| A3：结构化修改、生成 DSL、扩展语法和无 UI 内容处理 | 实施中 | 生成、基础结构事务、行内语义编辑、表格与列表结构、扩展语法及内容展开已完成；文档库 129 项分批通过，consumer 11 项通过；继续分段／跨块、格式化与跨 revision 复用 |
| B：源码编辑、虚拟布局、双栏、统一历史及图像块 | 实施中 | 源码、富文本预览与扩展图像已接入；编辑器库 48 项分批通过，真实图像工具及原生视图通过；继续双栏 |
| C：普通 Markdown 所见即所得编辑 | 实施中 | 行内语义事务、连续输入与原生富文本视图完成；24 步原生输入通过；继续表格／列表块命令、跨块及三模式交互 |
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
