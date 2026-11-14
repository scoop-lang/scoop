# Scoop Markdown 编辑器规范

日期：2026-11-14。完整范围见[编辑器设计](../ui/markdown/DESIGN.md)，实施进度见[记录](../markdown/PROGRESS.md)。本规范随实际能力逐项补充；文档模型遵守[Markdown 文档规范](SCOOP-MARKDOWN-SPEC.md)。

## 1. 分层

`scoop:scoop.ui.markdown` 是纯 Scoop library Cone。它依赖独立的 `scoop.markdown` 和现有 UI／窗口／文本能力；文档 Cone 不反向依赖 UI。源码、富文本和双栏共享一个 MarkdownDocument、一套文档选区和历史。不同视图的布局与滚动独立，源内容只有同一条事务写入路径。

## 2. 文档编辑状态

`MarkdownEditorState(document, selection)` 借用文档，通过普通内容订阅跟踪修改；状态绑定创建线程，UI 渲染／测量／scene／draw 期间不得修改。关闭状态释放订阅与历史，不关闭或重置借用的文档。关闭幂等，关闭后编辑接口报告状态错误。

`MarkdownSelection` 保存两个源码 UTF-8 位置及各自的 Before／After 偏向，保留活动端方向。选区端点必须是当前文档中的标量边界；不把 Long 字节位置当作字符索引。范围与插入位置沿文档差量映射。源码输入可直接 replace／replaceSelection；批量替换使用 apply，结构修改使用 transform，均调用文档的已有事务。

`apply(MarkdownInlineEdit, reversed, kind)` 接收文档 Cone 准备的行内语义修改，在发布同一个源码事务时采用其编辑后源码选区。reversed 保留选区方向；普通格式命令默认单独历史项。文档身份与 revision 必须匹配，错误准备结果不改变正文或历史；无正文变化时仍可更新选区并结束活动组合。

`apply(MarkdownBlockEdit)` 将文档 Cone 准备的块拆分／合并与结果源码选区作为一个独立历史项发布，复用行内语义操作的组合接受、观察者通知、失败原子性和 undo／redo 路径。准备本身不更改 UI 状态；后台解析未匹配当前 revision 时不能用旧 ParsedMarkdown 发起结构命令。

`apply(MarkdownChange, selection, reversed)` 发布准备好的源码事务和其 after 快照中的选区。结构命令可先用 `snapshot.prepare`／`syntax.prepareTransform` 准备差量，解析其局部结果并定位光标，再一次发布正文、历史和选区。结果选区的 UTF-8 边界及 before 快照身份必须有效；失败不发布。undo／redo 恢复命令前后的光标，不能在发布后另改选区来弥补缺失的历史信息。

状态以公开 State 读取追踪发布当前 snapshot、selection、canUndo、canRedo 与保存 revision。状态 API 先检查 UI 的修改时机，再准备与发布文档修改，不能先改文档、后因 State 禁止写入而留下部分 UI 状态。外部文档修改也必须发生在相同 owner 的正常编辑时机。

## 3. 统一历史

一次源事务保存正反替换及前后选区，普通程序化修改和结构命令形成单独历史项。输入种类 `Typing`／`Backspace`／`Delete` 仅在连续位置、折叠选区和相同种类时合并；选区移动、显式 breakUndoGroup、格式命令和 undo／redo 结束合并。合并只收集实际插入／删除的片段，不按每次按键重新拼接整个历史字符串。

富文本首次键入可以同时加入格式定界符或改写邻接转义；后续键入落在这段已修改源码内部时仍合并为一次撤销。历史只增量保存该替换片段及原始被替换文字，复用文档 Cone 的分块文本能力，不为连续按键复制全文或累计拼接字符串。跨出已修改源码范围、跨块操作及显式边界仍结束合并。

undo／redo 各通过一次源码事务恢复整个历史项，并恢复相应方向与偏向的选区。新编辑清除重做分支。Reset 表示整体重载，清空历史并将选区端点收敛到新文档中的合法位置；普通外部 Edit 形成可撤销的独立项。

`markSaved(revision)` 记录宿主已成功写出的本文件 revision；较旧的保存完成不把后续输入标成已保存。`isDirty` 按当前文档 revision 与保存 revision 比较。保存前先接受组合，再固定 snapshot；文件与异步 I/O 由宿主管理，状态不隐式写磁盘。

## 4. 源码输入片段与组合

`MarkdownSourceInput` 借用编辑状态，把活动选区及两侧上下文转换为窗口的 `TextInputState`。`sourceRange` 是当前片段在全文中的范围；片段坐标与源码坐标通过该起点换算。默认上下文为每侧 4 KiB，边界收敛到 UTF-8 标量边界；选区本身不受上下文大小限制。此对象不创建窗口或原生连接，原生宿主负责 connection／origin 检查和发布。

`apply(batch)` 使用当前片段的独立 revision 检查 baseRevision，先完成整个原生批次的范围与 Unicode 校验，再发布一次文档事务。批次中后续操作的范围指向前序操作之后的临时片段；多个不相邻修改保留中间未改正文的源码映射。失败不发布文字、选区、组合或历史。原生范围超出片段时直接报错，不截断范围。

预编辑即时写入同一个文档，并通过 `markedRange` 暴露源码中的组合范围；反复更新不逐条进入历史。非空 Preedit 记住组合前的片段、方向选区和必要来源映射；Commit／ClearPreedit 接受组合，最终净改动形成一个独立历史项。空 Preedit 或 `cancelComposition()` 恢复组合前内容和选区，不消费已有撤销项。一个批次可以接受一段组合再开始下一段；两者历史边界保持独立。取消恢复之后若同一批次仍带旧显式 Range，则报告状态错误，调用方须从新快照重新发起操作。

选区命令、普通程序化 Edit、格式命令、焦点／模式切换接受当前组合，再进行自己的操作。Reset 放弃组合并重载。undo／redo 遇到活动组合先取消它；下一次再回放历史。`acceptComposition()` 不更改已显示正文，但发布组合与历史状态；关闭编辑状态接受现有组合后释放资源。

同一编辑状态同时只允许一个输入片段拥有组合。片段内连续输入和预编辑沿用原映射；`synchronize()` 在组合之外跟随外部修改／导航，必要时重选上下文，并返回是否需要重建原生连接。宿主收到 true 必须先退休原连接再发布新映射，不能继续解释旧连接的范围。片段内选择变化只增加 native revision；caret 几何更新不增加文本 revision。批次与文档修改、选区变化均使旧 native 快照失效，原生宿主不得使用旧布局提供候选窗坐标。

输入片段的 synchronize 只更新派生快照，允许在文档变化通知中消费编辑状态已发布的 snapshot／selection；不再次编辑正文。它仍检查 owner、修改时机、片段归属和当前内容身份。正文编辑、组合提交及关闭仍禁止重入，不能将同步派生数据与第二次文档事务混为一谈。富文本 synchronize(content) 遵循相同规则。

### 4.1 富文本输入片段

`MarkdownRichInput(editor, content)` 借用同一编辑状态和当前快照上的 MarkdownTextContent，也接受单个 MarkdownInlineContent。它将文档 Cone 提供的单段或多块显示文字接入 TextInputState；公开 content 使用 MarkdownTextContent，单段结构从 content.inlineContent 读取。格式定界符不进入原生 surroundingText，图片与扩展保留 U+FFFC 原子；sourceRange 仍是文档 Cone 提供的源码上下文范围。初始源码选区须位于该上下文；跨段选区和整篇选择使用带真实文字及块分隔映射的多块上下文。它不拥有窗口、排版或另一份文档。

RichInput.setStyle 接受当前组合后修改选区格式；非空选择沿文档 Cone 的跨块 setStyle 发布一个单独历史项并保留方向。折叠选择只设置后续输入的格式覆盖，不修改 Markdown 或创建空历史；连续文字和预编辑在同一准备事务中应用覆盖，取消组合仍恢复确切原文。显式移动选择、其他来源的文档变化、输入片段关闭或模式切换结束覆盖。styleState 反映当前输入选择及覆盖；不支持的 profile 格式在修改前拒绝。setStyle 返回 true 表示宿主须退休旧原生映射，false 表示仅改变后续输入格式。

apply 依次校验原生批次并准备行内语义操作，最终只发布一次源码事务；content 立即成为该事务的 after，连续输入不等待全文解析。普通文字继承选区／光标侧样式；原生 marked、方向选区和 caret 均从当前显示映射转换为源码坐标。代码 span 换行及 NUL 沿文档 Cone 的文字规范化规则处理，并把转换后的实际文字与选择返回原生快照。取消组合保存并恢复原始行内源码，包括定界符、实体、容器换行、空标题间隔及缺失表格单元格的分隔符。

源码与富文本片段共享唯一组合、历史及发布路径；切换活动片段先接受已有组合。普通连续输入可合并历史，预编辑、提交后重开组合、取消、外部修改及 Reset 的边界与源码输入相同。批次错误不发布任何状态；订阅在发布后抛错时，片段仍更新至已发布快照。

`synchronize()`／`synchronize(content)` 在组合之外同步选择；外部源码修改后必须提供新解析得到的当前内容，不能重用旧源码映射。替换区域或快照时返回 true，宿主据此退休旧原生连接；同一区域内的本片段连续输入不更换连接。内容与 native revision 独立，旧 native revision 和其他组合拥有者的操作均按状态错误处理。关闭片段仅接受自己拥有的组合。

跨块原生输入与单段输入使用同一批次准备和发布流程，逐项校验当前 surroundingText 的 UTF-8 范围。结构替换后使用 after 映射与 insertedRange 重建组合文字、markedRange 和相对选区；保留的表格分隔符不进入组合文字。取消组合恢复开始时的完整源码上下文和定向选区，已提交文字与后续组合仍按 §3 分开进入历史。剪切、粘贴、删除及整篇选择后的输入使用相同事务。一次批次只发布一次已准备的 MarkdownChange，不在发布时重新准备另一个结果快照。

## 5. 源码视图

`markdownSourceEditor` 是普通 View 入口，`MarkdownSourceHost` 可用于自绘 Element。它借用编辑状态和字体，拥有一个文档级输入区域、原生输入片段、滚动 viewport 和可见行布局。每个源码物理行使用完整 Paragraph 排版，可自动换行或横向滚动；不按任意字符数量切断 shaping，不把每一行变成独立文本编辑器。

行高索引以未测行估值和已测行尺寸保存可见位置。局部源码修改只替换受影响行段，滚动和光标行按需测量；同时保留活动光标所在行。文档字符、超长行和大型选区不因虚拟化而截断。测量结果独立持有排版与对应源码 revision；一次试测或被丢弃的帧不覆盖已接受的输入几何。离屏布局在不再被测量结果和已接受帧借用后释放。

源码视图接收平台 TextEditCommand，使用现有文本布局的 grapheme／Unicode 单词／双向文字导航，支持方向选区、整篇选择、复制／剪切／粘贴、换行、Tab、undo／redo 与组合取消。删除跨物理行时把 CRLF 作为一个换行单元处理。只读模式保留选择、导航和复制；不接收修改正文的命令。点击与拖选使用已接受帧；当前文档 revision 不匹配时先等待新布局；选区或 marked 变化无需重排文字，但候选窗必须匹配新的输入 revision，不把旧命中位置当作新源码位置。

`hitTest` 返回 `MarkdownSourceHit`，保留源码位置和视觉 affinity；`caret` 可接受该 affinity。源码偏向决定文本事务中的位置归属，视觉 affinity 决定折行／双向边界的排版侧，不能互相代替。翻页按视口距离定位，包含同一个长物理行内的多个排版行；标量位置落在 grapheme 内部时，绘制使用边界几何，普通前后删除消费完整 grapheme。

`onChanged` 只传递文档差量。宿主销毁时退休输入连接，结束自己持有的组合，释放订阅、输入区域、滚动和布局；不销毁借用文档。卸载后的输入通知不再更新窗口或请求布局。各模式按第 9 节接入同一个编辑状态。

### 5.1 局部源码范围

`MarkdownSourceRegion(editor, range)` 借用同一编辑状态，以全局 UTF-8 源码范围限定一个源码编辑面。它沿真实文档差量映射两端，保留起点前、终点后的插入偏向，使范围内的输入、组合取消和撤销／重做继续包含在该区域；不创建第二份文档或历史。区外修改正常移动区域，Reset 使区域失效，close 释放订阅。无效 UTF-8 范围或其他 editor 的区域不能绑定输入。

`MarkdownSourceInput`、`MarkdownSourceHost` 和 `markdownSourceEditor` 可借用该 region。原生周边文字、排版、命中、方向键、整篇起止、全选、删除和缩进只消费其范围；首尾可以位于物理行中间。大型区域仍按可见行和局部输入上下文处理。外部选区位于区外时不改写共享选区，重新聚焦后将选区收敛到区域；共享撤销仍作用于同一个完整历史。关闭或 Reset 后不接受新的输入。测量结果保留自己的 sourceRange，旧帧和试测不覆盖当前输入几何。

局部面直接编辑包含围栏、语言标记、公式定界符或模板指令的原始源码，未闭合和临时无法解析的内容仍可继续编辑。解析和图像控制器照常消费同一文档的新快照，错误沿已有展示及重试路径报告；完成编辑不额外序列化或改写原文。

## 6. 共享后台解析

`MarkdownParseController(editor, executor, proxy, options)` 借用同一个编辑状态、现有 `CoroutineExecutor` 和 `ApplicationProxy`。构造不启动后台任务，start／refresh 在正常 owner 回调中请求处理；start 幂等，refresh 还用于文件通知后的资源版本检查。options 包含固定 Markdown profile 和可选的资源展开配置；configure 使用新配置后请求新结果。调用方在应用层管理 executor 的 shutdown／join，不在组件卸载时等待全文解析结束。

控制器以 Idle、Pending、Ready、Failed、Closed 表达完整状态。Pending 和 Failed 可携带上一份成功结果用于显示；Ready 的 current 提供当前完整树；Pending 时结构命令可使用第 10 节已前进到当前快照的局部树。`MarkdownParsedDocument` 同时提供原始源码解析与可选的展开结果；生成内容展示使用派生解析，结构命令使用原始解析及其节点身份。无展开时只解析一次；有展开时复用处理器已经取得的原始解析，不为同一快照重复完整解析。

文档差量先完成源码和历史发布，再将不可变 snapshot 入队，不在 UI 线程物化或解析全文。控制器最多运行一项解析，并保留一份最新待处理请求；已有任务正常完成后直接继续最新请求，连续输入不会排队所有旧快照。处理器与 resolver 在后台串行调用，不读取 UI State。语法扩展 `MarkdownExtension.parse` 只匹配输入；它既可随全文解析在工作线程调用，也可随局部结构命令在 owner 调用，共用实例须使用不可变配置或自行同步内部状态。局部解析不做资源展开或外部工具调用。

工作线程只处理值和只读快照。完成结果通过至多一个待执行消息回到 owner；后来的结果替换尚未交付的旧结果。发布时核对请求身份、文档 revision 与配置，过期成功或失败均不覆盖当前状态。选区、caret 和保存状态变化不触发解析。解析／扩展异常成为对应请求的 Failed；后续输入或 refresh 可继续处理。

close 幂等地取消订阅、清空排队请求及交付引用，结束自己的控制器生命周期；运行中的普通解析允许完成，其 executor task 无论结果是否可交付都完成登记。关闭后迟到消息不访问编辑状态或图形资源；消息队列拒绝／丢弃交付不影响后台任务收尾。close 不关闭借用的 document、editor 或 executor，不依赖下一次可见帧释放后台资源。

## 7. 富文本预览投影

后台解析同时生成不持有原生资源的显示投影。普通段落／标题保留完整逻辑段落；代码按实际行、表格按行建立虚拟化单位。列表编号、任务标记、引用层级、链接和已使用脚注作为投影数据保存；脚注按引用顺序放在正文之后，源码仍保留原顺序。原始 HTML、front matter 和没有展示适配的扩展保留可定位的源码表示。

投影直接消费 MarkdownText 的实体、转义、代码规范化与源码映射，不再次解析正文或解码实体。映射区分逐字文字、原子替换和零宽隐藏标记，保留 Before／After 两侧定位。软换行显示为空格，硬换行参与同段排版。图片及行内扩展使用真实 U+FFFC 原子对象并保留替代文字和原始范围；没有资源结果时展示其源码／替代文字，后续图像适配沿同一范围替换显示资源。

显示顺序和源码顺序各有索引，脚注搬移不会使二分定位误把显示顺序当作源码顺序。展开内容先定位派生源码，再通过 ProcessedMarkdown.sourceMap 回到原指令；生成文字不能伪装成原文件中可逐字修改的正文。

相邻解析结果在后台比较实际显示内容和影响几何的块信息，复用未变化前后段的布局身份；源码绝对偏移、链接目的地址与节点身份单独更新。身份仅服务布局复用，不作为另一 revision 的 MarkdownNodeId。UI 不在 measure／scene 中遍历整棵语法树或物化整篇源码。

`markdownPreview` 和 `MarkdownPreviewHost` 借用一个解析控制器、编辑状态和字体；不会自行创建另一份文档或解析任务。预览按可见行与必要定位行建立原生排版，用累计高度索引定位滚动位置。每个测量结果独立保持对应投影和 TextLayout；试测、丢弃帧与新 revision 不改变已接受帧的命中数据。表格采用整个表格一致的等宽列与最小列宽，超宽表格横向滚动，不根据可见行内容改变列宽。

预览读取共享源码选区，提供命中、caret、源码定位与链接动作；当前解析未就绪时可以继续显示上一份结果，但不把旧几何作为当前源码的编辑位置。普通文字的 WYSIWYG 输入另沿同一投影接入，预览自身不建立独立历史。尺寸变化和解析更新通过源码锚点保持阅读位置；关闭释放自己的布局、输入区域与滚动归属，保留借用的 editor、controller 和 fonts。

### 7.1 代码块语言与高亮

`MarkdownParseOptions.codeHighlighter` 可选择默认的纯 Scoop 词法高亮器、宿主提供的 `MarkdownCodeHighlighter`，或 None 关闭高亮。接口接收语言名与完整代码文字，返回以该文字 UTF-8 字节计量、有序且不重叠的 `MarkdownCodeSpan`。配置与提供器须不可变或自行同步；调用只发生在串行后台解析阶段。内置规则基于 Prism 的词法规则，复用 scoop.regex，不执行代码。未知语言显示完整原文。它不是语言类型检查器，字符串插值等细分样式可以由自定义提供器补充。

高亮结果按实际语言、正文和提供器身份复用未变化代码块；源位置移动时重新映射，不重新执行高亮器。展开文档与原始文档分别保持映射，无展开时共用结果。必要的范围与 UTF-8 边界检查在接收提供器结果时完成；失败仅使对应块恢复普通文字，并记录高亮诊断，不使正文解析或编辑失败。

`MarkdownParsedDocument.sourceHighlights` 提供绑定原始 snapshot 的不可变高亮范围，供独立源码视图借用。预览代码行和三模式／局部源码编辑器使用同一批高亮，源码映射分段排除容器前缀，Tab 展开沿既有 TextDisplay 映射。只为可见行和必要定位行生成样式；过期快照的高亮不套用到新源码。`MarkdownCodeTheme` 只提供颜色，不改变字符、字体度量、选区、输入 revision 或历史。owner 线程上的局部结构投影不执行高亮器，当前后台结果到达后恢复颜色。

局部代码源码面显示完整的 Language / info 输入框，可通过 Enter 或 Apply 提交普通 setCodeInfo 事务，并提供 Copy code 复制解析后的正文及 Done 返回富文本。未知语言和附加属性允许原样编辑；非代码扩展保留原有源码面。结构命令只使用当前解析，输入组合先提交；操作后保持局部源码焦点和共享历史，撤销恢复语言、围栏和正文。正文与语言字段草稿在解析等待期间仍可编辑，等待仅禁用 Apply；后台更新不取消语言字段中尚未提交的原生组合。

## 8. 图片与扩展图像

`MarkdownImageKind` 区分普通资源、指定语言的代码块及指定名称的扩展。`MarkdownParseOptions.imageCodeLanguages`／`imageExtensions` 明确哪些块投影为图像块，默认包括 Mermaid 代码与 math 扩展；其他代码仍按实际行虚拟化，front matter 仍保留源码表示。行内图片和扩展沿既有原子对象投影。图像块保留完整节点／正文映射，展示和保存分别处理，不把渲染结果写成原文中的图片链接。

应用提供普通 `MarkdownImageRenderer`，每个实例注册一种 kind。`version(request)` 查询当前资源／配置版本，`render(request)` 返回复制或不可变的 PNG／JPEG 字节及实际版本；版本查询先于内容读取或工具执行。请求只包含渲染所需的源码、信息字符串、扩展 attributes、文档 URI、宽度、比例、字号、颜色和行内／块模式，不借用 UI、Canvas 或原生图形对象。方法由借用 executor 的 blocking 队列串行执行，不保证固定 OS 线程；Markdown 解析仍使用普通工作队列。

结果的逻辑尺寸由明确的逻辑 Size 或像素比例确定，不把像素尺寸直接当作所有屏幕上的逻辑尺寸。普通图片可按底边对齐；行内扩展须提供真实的 baseline，逻辑宽度作为 advance。无效图片、尺寸、度量和普通 I/O／工具异常成为该图像的失败状态，保留源码。数学适配以排版工具的实际边界和基线换算度量，不按图片高度估算基线。MathJax 的全宽公式保留其百分比内部布局，以可用宽度与工具的最小宽度确定外层画布；编号和正文使用同一实际视口。

`MarkdownImageController` 借用 executor 和 ApplicationProxy，拥有当前缓存与 owner 线程上的解码资源。每个可见视图通过一个 lease 声明已接受帧所需的请求，多个视图的相同实际渲染输入共享结果。measure 仅读取快照；请求集合在接受帧后的普通 owner 回调中更新，丢弃帧不启动外部工作。行内／块视口以每项结果的 generation 更新自己的排版，不因无关图像完成重排所有段落。

图像状态覆盖未配置 renderer、等待、成功和失败；等待或失败可保留上一次成功图片。每次显式 refresh 检查仍被使用的请求版本，命中成功结果不重复读正文、执行工具或解码；失败可在后续 refresh 重试。待处理请求按当前使用集合合并，正在执行的普通工作允许完成；只有仍被使用且请求代次匹配的结果才在 owner 发布。工具等待不会在 UI 线程发生。

每次绘制／测量持有独立的 MarkdownImageSnapshot；lease 或 controller 关闭不会提前销毁仍被该快照借用的图片。最后一个可见使用者退休后移除缓存条目与待处理请求，运行中的过期字节结果直接退休。close 不等待外部进程，不关闭借用 executor；renderer 负责自身 I/O、进程及临时文件的正常收尾，应用在自己的退出流程中管理 executor 的 shutdown／join。

本地文件适配使用 scoop.url 解析 URI 与百分号转义。有 documentUri 时按文档位置解析；来自单个 include 资源的图片使用来源映射中的资源 URI。没有文档 URI 时使用配置的绝对目录。本地适配只读取本地文件，网络等其他资源可由应用注册 Resource renderer。

Mermaid 适配只向配置好的 mmdc 传递单个 fence body，经 argv 和临时文件得到 PNG。公式适配配置 Node、已安装的 @mathjax/src 目录与 rsvg-convert，使用随 Cone 嵌入的 MathJax 进程入口，带回 SVG 和实际排版度量，再光栅化为 PNG。固定工具配置以调用方提供的 toolVersion 标识；替换工具配置时创建相应新版本的适配。普通使用不自动安装或下载外部工具；缺少工具沿图像失败状态显示，文档的编辑和保存继续可用。


## 9. 三模式组合与视图状态

组合视图复用 `scoop.ui.component` 的标准分栏及应用的 `ThemeSource` 上下文；应用在实际 run／mount 入口提供主题。两栏文字与背景仍由各自 Markdown 样式配置。

`markdownEditor(viewState, controller, ...)` 组合原生源码与富文本视图，二者必须借用同一个 `MarkdownEditorState`。`MarkdownEditorViewState` 持有 `Source`／`Wysiwyg`／`SideBySide` 模式、活动栏、两个独立的 `ScrollState`／`FocusHandle`、源码栏比例、右侧只读与同步滚动开关。它不拥有文档或原生资源；同一 viewState 同时只能挂载一次，不同 viewState 可共享一个 editor。设置遵守 UI owner 与正常修改时机。

实际模式切换先接受当前组合一次并结束连续输入合并，保留源字节、选区与历史；设置相同模式不产生这些副作用。视图复用现有 resizable 的稳定栏 key，仍显示的栏保留挂载，隐藏栏释放自己的原生资源；各自滚动状态保留，重新显示后绑定新 viewport。`rightReadOnly` 只影响双栏右侧；`MarkdownEditorOptions.readOnly` 作用于所有模式，源码还遵守 `source.readOnly`。切换右栏只读不结束左栏的组合。

默认不抢焦点。`requestFocus(pane)` 对当前可见栏记录请求，必要时等待挂载；隐藏栏返回 false。显式 `showSource(range)` 先验证当前源码两端的 UTF-8 标量边界，再将共享选区折叠到起点；当前为 Wysiwyg 时切为 SideBySide，随后聚焦源码。无效范围不改变模式、选区或历史。`markdownRichEditor` 和三模式组件默认在富文本栏内打开 §5.1 的局部源码面，保留当前模式，完整展示原始范围；可通过完成按钮或无组合时的 Escape 关闭并返回富文本焦点。关闭先接受已有组合，组合期间 Escape 先取消组合。只读设置同样作用于局部面，生成内容编辑其来源指令。宿主显式提供 onSource 时由该回调接管展示。

组合控件仅订阅一次 `onChanged`，同一源事务不因左右两栏而重复通知。解析完成、选区与纯视图设置不产生正文差量。控件卸载关闭自己的订阅及两侧宿主，不关闭借用的 editor、controller、字体或图像控制器。

滚动同步由当前主动请求栏驱动，等待该请求的已接受帧，读取顶部命中的源码位置和相对于该行高度的偏移比例，再按另一栏的实际排版定位。图片或原子对象使用其对应行／对象几何。同步不按总滚动百分比换算，不因镜像请求或 LayoutCorrection 反向触发。进入双栏／打开同步时以活动栏对齐一次；关同步后两侧独立。待定位锚点沿文档差量映射，Reset 清除；目标解析未到当前 revision 时保留请求等待。过期请求、试测与丢弃帧不完成同步，不覆盖已接受的几何。

`MarkdownSourceHost`、`MarkdownPreviewHost`、`MarkdownRichHost` 可接收可选的 `initialScroll`，由调用方保留 ScrollState，由宿主拥有 viewport 绑定。一个 ScrollState 仍只能绑定一个活动 viewport；关闭宿主后可重新挂载。

## 10. 原生富文本输入与活动区域显示

`markdownRichEditor`／`MarkdownRichHost` 使用同一 editor、解析控制器和图像控制器，复用预览的样式、布局参数、行虚拟化、图片与滚动资源。readOnly 保留定位、选择与复制，并禁止正文输入。宿主有一个文档级输入区域，活动段落／标题／表格单元格使用 MarkdownRichInput；没有为每一段建立独立文档或历史。链接与扩展保留 onOpenLink／onSource 回调。

富文本取得焦点时，若折叠的源码光标位于当前标题／段落的外围隐藏标记，将它收敛到该区域最近的显示文字边界；未聚焦视图不因此移动共享选区。跨块选择保持原文范围，不截成单个行内区域。

活动区域直接投影 MarkdownInlineContent.nodes，原子对象与 native U+FFFC 对齐；源码标记、HTML 和脚注的内部修改继续由源码定位入口承担。普通文字输入、行内选择与格式修改立即生成局部显示。后台解析尚未完成时，已确定的其他行继续使用原来的布局和源映射，通过实际源码差量换算到当前位置；活动单元格使用新映射。只替换活动行的投影和排版，不按键遍历或重建整篇显示数组。

MarkdownRichHost.setStyle／toggleStyle 支持单段、跨块和折叠选区；styleState 提供可观察的当前格式状态，不在 render 中修改输入。Primary+B、Primary+I、Primary+Shift+X 分别切换粗体、斜体和删除线；On 时移除，Off 或 Mixed 时应用，重复按键不反复切换。只读、禁用或无可编辑内容时命令不修改文档。MarkdownEditorViewState 提供相同的命令入口，并在工具栏操作后恢复富文本焦点；源码模式不执行这些富文本命令。markdownFormattingToolbar 是可选的普通 View，复用 UI 组件按钮与宿主 ThemeSource，显示当前、混合及不可用状态；它不拥有第二份选区或历史。

`setCode`／`toggleCode`／`codeState` 沿同一输入与历史提供行内代码；Primary+E 切换。非空选区转换遵守文档 Cone 的换行与原子对象边界，折叠选区只设置后续输入模式，不先插入空代码标记。后续输入在文档插入之前确定代码模式，因此退出代码后输入的换行不被代码规范化吞掉。代码模式与粗体、斜体、删除线可组合，连续输入、组合取消和撤销仍使用同一事务规则。表格中无法由代码 span 表示的文字由文档层在发布前拒绝，RichHost／ViewState 的命令返回 false 并保留原文；已经发布后的应用观察者异常仍正常向上传递。

RichHost 与 EditorViewState 的 `canEditTarget` 可观察当前单行内块入口是否可用；`beginTarget(MarkdownTargetKind)` 接受当前组合并返回 `MarkdownTargetDraft`，字段包括 kind、label、destination、title、labelEditable 和 canRemove。`applyTarget(draft, label, destination, title)`／`removeTarget(draft)` 返回是否执行；`draft.isCurrent(editor)` 比较实际 editor、源快照及选区。含行内对象的链接 labelEditable 为 false，设置目标时保留对象；自动链接不提供移除按钮。

工具栏的 Link／Image 打开普通模态编辑面板，编辑显示文字／替代文字、URI 和可选标题；Apply 作为一次源事务，Cancel 不改正文。折叠光标位于已有链接或图片时编辑该对象；非空选区设置链接时保留未改动的文字样式；没有显示文字的新链接使用 URI 作为文字。引用链接和图片改为当前使用点的直接目标，不修改共享定义。面板保存打开时的快照和选区，文档或选区已变时拒绝将草稿写入过期范围，并提示重新打开；源码模式、只读和禁用状态不发布修改。链接移除保留文字与样式，遵守 GFM 自动链接的固有识别规则。图片插入／替换只消费宿主提供的 URI，由已有图像控制器展示。链接与图片编辑范围为一个可编辑行内块，跨块选区不创建跨结构链接；文件选择、保存或上传属于宿主应用。

MarkdownPreviewMeasurement.sourceRevision 表示实际显示／命中使用的源版本；parsed 可仍是后台的基线解析。测量持有自己的局部投影、已布局行和图像，试测与丢弃帧不替换已接受几何。选区、marked 与 native revision 一起固定；当前内容或输入 revision 不匹配时清除候选窗几何，不发布旧布局。活动行即使滚到视口之外仍保留原生输入所需排版。

RichHost 与 EditorViewState 的 `blockState(MarkdownBlockCommand)`、`executeBlock` 提供标题／段落、引用、列表类型与层级、代码块、分隔线、表格插入及行列／对齐命令。可选 `markdownBlockToolbar` 使用普通菜单组件，供应用组合；状态区分当前、混合和不可用，不在 render 时解析或修改文档。Primary-0／1…6 切换段落／标题；命令成功后恢复富文本焦点。

类型转换覆盖选区相交的完整块，新增容器要求连续同级块；已有列表的类型转换作用于当前同一列表，层级与解除列表作用于当前条目。表格行列命令要求选区位于一个单元格；第 0 行是表头，不能删除或在其前插入正文行，至少保留一列。删除后定位到相邻有效单元格，新增行列定位到新单元格，对齐保持原选区。插入表格／分隔线替换选中文字并保留两侧内容，整个操作只有一个撤销项。只读、源码模式和不相容结构禁用命令；外部修改失去有效局部结构后不使用旧节点发起变更。

块命令先接受当前输入面的组合，使用匹配当前快照的局部语法准备差量与结果选区，再沿现有结构命令发布并退休旧原生连接。后台完整解析未完成时继续复用有效的局部区域；查询状态只读取已有结构和坐标映射，不把普通键入变成同步全文解析。

原生连接的 enabled／disabled、origin、过期批次及错误恢复沿源码视图合同处理。连续行内输入不重建连接，切换映射区域时先结束组合并退休旧连接。Unicode／双向文字导航使用同一已排版文字的 grapheme、单词和视觉 caret；复制所见文字与源码字节位置保持对应。

块命令使用完整解析中取得的 MarkdownRegion；区域沿正常文档差量前进，在命令需要结构时显式解析局部。区域中的整段／列表／表格可以改变显示行数，即时显示以局部行片段替换基线的对应范围，其余行保持共享。源码到显示行、反向命中、滚动高度与选区映射均使用该片段；不以同步全文解析衔接 Enter 后的输入。后台完整结果到达当前快照后收敛到完整投影。区外修改或无法保留区域边界的源码操作退休局部结构，保留用户输入与源锚点，等待当前完整结果。

已知局部编辑尚待后台解析时，选区仍可扩大到更多块或整篇。宿主将新选区映射回基线，与已有编辑区域合并，再用已记录的真实差量推进扩大的区域；显式建立多块输入时才解析该范围。扩大选区不丢弃输入连接或等待后台结果；重新选中一个行内块时，宿主退休多块连接并回到该块的局部输入。原区域已因未知区外修改失效时仍遵守前段的完整解析边界。

Enter 拆分段落／标题，保留两侧样式、所在引用和列表语义；空列表项退出一级，空白续行可立即继续输入。普通 Backspace／Delete 在段落边界合并同容器相邻段落／标题，光标位于拼接处。Tab／Shift-Tab 在列表中缩进／提升当前项；表格内按单元格前进／后退，末格 Tab 新增正文行，Enter 保持列并前往下一行，必要时新增行。仅导航不新增历史项；新增行与结果光标作为一次事务进入历史。显式 `PasteTableTsv` 命令使用 `pasteTableTsv` 写入矩形，光标落在起始单元格新文字末尾。结构变化后仍使用真实 native 连接的退休与启用过程，不将旧连接范围解释为新段落位置。

默认复制和剪切同时写入 `text/plain` 与 `text/markdown`：富文本的前者使用所见文本，后者使用原始文档语义片段，生成内容保留原指令。源码面两种表示均为原选区源码。写入成功后才删除剪切选区。默认粘贴优先使用存在的 `text/markdown`，在富文本面调用文档 Cone 的 replaceMarkdown，在源码面原样插入；缺失时按字面文本粘贴 `text/plain`。存在但为空的格式仍是有效内容，不回退到其他格式。一次粘贴作为一个独立撤销项，复用返回的局部解析；组合先接受，禁止使用另一输入面正在持有的组合。

`MarkdownClipboardCommand` 提供 CopyText、CopyMarkdown、PasteText、PasteMarkdown 和 PasteTableTsv；RichHost、SourceHost 与 EditorViewState 的 `executeClipboard` 可供菜单／工具栏使用，不要求按钮点击后输入面仍拥有焦点。CopyMarkdown 将源码同时作为普通文本提供，PasteMarkdown 也可把普通文本显式解释为 Markdown，PasteText 始终忽略 Markdown 表示。源码面的 PasteTableTsv 返回 false。只读保留复制，禁用粘贴；禁用组件不执行命令。ViewState 路由到当前栏并在成功后恢复该栏焦点。HTML 导入另行实现，此阶段不把 HTML 格式作为 Markdown 解析。

## 11. 全文查找替换

`MarkdownSearchController(viewState, executor, proxy)` 为同一三模式视图维护查询、结果、当前匹配及原生字段状态，借用视图和 executor。configure 发起后台源码搜索，可显式 selectFirst；next(backwards) 循环导航，show(replace)／hide 控制可选查找栏。单个正在执行的任务与最新待处理请求共存，owner 发布时核对实际请求、查询和 snapshot 身份。文档变化立即废弃旧替换范围；查找栏关闭时不因正文键入继续全文搜索。选区、caret、保存状态与高亮颜色变化不重新搜索。

`markdownSearchBar(controller)` 复用原生 Input，提供查询、大小写／整词／正则选项、计数、前后导航，以及单次／全部替换。`markdownEditor(..., search = Some(controller))` 绑定其快捷键和结果高亮，controller 必须属于相同 viewState；独立源码／富文本／预览 View 和 Host 可传 searchResult。Primary-F 打开查找，Primary-Alt-F 打开替换，Primary-G／Shift-G 导航；替换组合使用 F 的物理键位，避免 Option 改变逻辑字符。查找字段 Enter／Shift-Enter 导航，Escape 在字段组合已经结束时关闭并返回原活动栏。查询字段的输入和撤销属于普通字段状态，不进入文档历史；查找结果定位保留字段焦点。

搜索范围是原始 Markdown，包括定界符、链接目标、代码与自定义指令。结果不搜索派生 include 的外部内容；高亮沿已有源码／显示映射绘制，虚拟视口只查询相交匹配，零宽结果显示细线。背景结果过期时不对新正文绘制旧范围。匹配选择以真实源码坐标保存，定位允许目标尚未挂载或完整解析尚未就绪，不隐式切换编辑模式。

replaceCurrent 和 replaceAll 只在当前结果仍匹配文档、目标视图已挂载且可编辑时执行。方法先准备文档 Cone 的替换事务，再接受文档组合并通过共享 editor 一次发布；全部替换只有一个撤销项，保留方向选区。单次替换后从插入结束位置继续搜索，跳过刚插入内容；空变化仍前进至下一个旧匹配。正则查找栏启用捕获模板，普通文本替换按字面处理；无效模式或模板在栏内显示错误，正文不部分修改。整体只读、活动栏只读或禁用时仍保留允许的查找，替换禁用。

关闭 controller 取消待处理与未发布结果、释放订阅，正在执行的纯搜索可以结束但不再发布。借用的文档、视图和 executor 仍由宿主关闭；一个查找栏独占该 controller 的原生字段。

## 12. 大纲、目录与位置导航

MarkdownParsedDocument 在后台建立 sourceNavigation 和 previewNavigation，未展开时共享同一索引；仅资源变化时复用未变的源索引。普通 caret 移动、视口变化和大纲选中状态不重新扫描语法树。大纲使用原文档的标题，预览内部链接与 [TOC] 使用最终预览语法的标题，通过既有来源映射返回原文档位置。

`MarkdownNavigationController(viewState, parser)` 借用同一编辑状态与解析器，提供当前标题索引、headingAt 对应项、goTo、goToLine、goToHeading、goToLink，以及后退／前进。goToLine 的参数与文档 Cone 一致，从零计数；原生字段按从一计数的 line:column 展示。合法跳转先接受组合，保留文档历史，以源码选区定位当前活动栏，不改变编辑模式；无效位置或旧标题索引不能改变文档和选区。后台尚未给出当前结构时，大纲保留旧项目但禁止以旧范围跳转。

导航历史与文档撤销分别管理，最多保留最近 128 个显式跳转位置；正文修改映射已保存选区，Reset 清空导航历史。后退／前进恢复源码选择方向并保持当前模式，普通光标移动不记录跳转。活动富文本栏沿既有输入边界规则，把落在标题／列表隐藏块标记上的折叠光标放到该块可编辑文字起点；源码和只读预览保留所请求的源码位置。关闭 controller 释放订阅，不关闭借用 editor 或 parser。

`markdownOutline(controller)` 使用标准 VirtualList，只挂载可见标题，按真实父层级缩进；点击标题跳转并恢复编辑焦点。`markdownNavigationBar(controller)` 提供 Back／Forward 与原生行列输入；Primary-L 聚焦行列字段，Enter 跳转，组合结束后的 Escape 返回当前编辑栏。markdownEditor 的 navigation 参数绑定同一 controller，使预览内部链接和目录跳转进入统一的导航历史。

[TOC] 保留原指令，由最终预览标题派生为可定位的目录行，生成文字不可直接写回；编辑指令仍走局部源码。预览脚注编号跳转到定义，活动编辑片段中的脚注原子仍提供源码编辑入口。未挂载的目标以源码锚点定位并等待有效布局。内部链接解析失败时交给宿主的既有 onOpenLink；普通外部 URI 保留宿主处理方式。只读仍允许导航，禁用组件不处理输入。
