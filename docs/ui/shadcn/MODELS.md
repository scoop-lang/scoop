# 复杂控件的数据与交互模型

日期：2026-11-04。状态：设计。范围见 [CATALOG.md](CATALOG.md)，共享输入、主题和生命周期见 [API.md](API.md)。这里定义实际控件需要的算法和业务边界，不建立跨控件的通用状态机框架。

## 1. 选择、搜索与菜单

Select、Combobox、RadioGroup、ToggleGroup 共用有序的 Choice 元数据，单选值使用 `Binding<T?>`，多选使用 `Binding<Array<T>>`；需要比较值的泛型入口明确要求 `T: Equals<T>`，不退回 Any 或显示字符串。每个 Choice 有稳定显式 ViewKey、领域值、显示文字、enabled 和必要的分组／搜索文字。显示文字不是身份，相同标题合法。

RadioGroup／ToggleGroup 的项描述在 Choice 之外保存各自的内容、样式与焦点选项，以不可变 entries 数组或 builder.item(choice) 接入；每个新 Props 快照检查一次 key／value 唯一性。Radio 使用 KeyNavigation.preferredTab 与 onNavigate 表达已选入口和方向选择，ToggleGroup 只使用同一导航的焦点行为，不共享选择策略。

其他小数据入口接收不可变 `Array<Choice<T>>`；大数据使用实际的 ChoiceSource，提供 count、按索引取得元数据、按 key 定位。导航适配现有 NavigationSource，呈现适配 ItemSource，条目 View 只在需要时创建。source 是本次元数据快照，流式正文／进度可以独立 State 更新，不为一个字的变化重新构造所有选项。

| 状态 | 行为 |
| --- | --- |
| selected | 外部 Binding 的已选择值；hover、方向导航和搜索默认不修改它 |
| activeKey | 本次候选列表中的活动 key；重排保留，过滤／禁用／删除后清空或按明确 autoHighlight 选择下一候选 |
| query | Combobox／Command 使用 TextEditorState 的已提交文字；preedit 不作为搜索结果或已选项 |
| open | 独立 Binding 或根组件自有状态；外部选择变化不隐式打开列表 |

非编辑 typeahead 使用忽略大小写的前缀，Combobox 使用忽略大小写的包含匹配；转换复用已有文本能力，匹配函数可替换以提供 locale／重音规则。Command 默认对齐上游 [cmdk 的评分算法](https://github.com/dip/cmdk/blob/main/cmdk/src/command-score.ts)：有序字符匹配、连续／词首优先和稳定的同分顺序，按其许可证复用有限的评分代码，并按 Scoop 的 Unicode 表示处理索引。它不建立持久搜索引擎。自定义结果模式直接消费调用方给出的候选和顺序，避免远端筛选后再次本地过滤。

单选 Enter 提交有效 activeKey，清空 query／关闭 popup 的规则按控件配置执行；多选保持 popup，chips 按选择顺序呈现。空输入时 Backspace 可删除最后一个 chip，chip 的删除按钮和方向导航有独立焦点；非空输入／composition 优先执行文本编辑。没有候选时 Enter 不误提交旧 key，空结果行不加入候选导航。

外部更新后的 selected 不在当前数据源中时，控件保留该值，显示调用方的值文字或明确的“不可用选项”文案；不在 render 中擅自清空 Binding。外部过滤、分页或请求未完成不代表用户撤销选择。数据源的 key／值对应关系在实际元数据接入边界检查，未变数据不每帧重新完整校验。

菜单共享具体的 MenuEntry：Action、Checkbox、Radio、Submenu、Label、Separator。MenuContent 是条目快照；同级显式 key 唯一，子级可重复父级 key。每种 entry 只携带自身需要的动作／Binding／子菜单；Label／Separator 不可激活。普通 Action 成功选择后关闭相应菜单链，Checkbox／Radio 默认保持打开并可显式配置 closeOnSelect；Checkbox 只在实际值变化后关闭，Radio 只在目标值确实选中后关闭。`radioItem<T: Equals<T>>` 把领域比较和写入保存在有类型的闭包中，不扩展统一值容器。回调失败遵循既有 UI 异常路径，不能伪造成功选择。

DropdownMenu、ContextMenu、Menubar 共用菜单行、导航和子菜单开关逻辑；不同之处是打开方式和父级导航。父／子菜单使用已提交矩形和 Hover corridor，RTL 翻转展开方向。键盘打开子菜单后保持实际焦点，悬浮打开不主动抢走父行焦点；Right／Left 进入／退出子菜单，Escape 只关闭最内层。Tab 保存方向，请求根关闭，在根 onClosed 中从恢复的入口调用 moveFocus；拒绝关闭或退出中重开都清除该请求，不合成键盘事件。Menubar 的 Binding<ViewKey?> 是唯一打开值，缺失／禁用的外部 key 不被暗写。显示快捷键不自动注册全局动作，应用注册同一个 ShortcutChord。

Popover 直接持有业务 `Binding<Boolean>`，编辑字段仍用各自 TextEditorState。打开会话与退出动画由 Portal 持有；组件不复制 open，不保存第二份焦点目标或窗口坐标。onOpenChange 仅在 setter 采纳新值后通知；onCloseRequest 可 veto，onClosed 仅在实际移除后通知。退出中重开保留同一内容实例，最终关闭再开创建新内容，外部编辑模型可继续保留草稿。

## 2. 三态、范围与拖动

CheckState 为 Unchecked／Checked／Indeterminate。用户激活规则为 Unchecked→Checked、Checked→Unchecked、Indeterminate→Checked；混合态通常由一组选项派生，不成为三次点击循环。它映射现有主题 CheckMark.Off／On／Mixed。Boolean 入口只能提供前两态，标签、指针和 Space 使用同一动作。

Slider 的 `values: Array<Double>` 按 thumb 顺序保持有序；RangeSpec 提供 min／max、step、minimumGap 与 reversed。改变 thumb 数量意味着配置改变，结束旧拖动；只移动当前 thumb，不对结果排序来悄悄交换身份。RangeSpec 的整数步号、有限性和退化区间规则直接复用。

- 主轴拖动和轨道点击使用最近 Accepted 的几何。轨道点击选择距离最近的可动 thumb，同距使用当前聚焦 thumb，再按稳定顺序决定。
- Left／Down 减小、Right／Up 增大；水平 RTL 和显式 reversed 通过同一方向映射处理，不能重复反转。Home／End 移到该 thumb 在邻居约束内的端点，PageUp／Down 默认移动十个 step；连续范围用明确的 keyboardStep。
- 用户拖动连续写 Binding，正常释放交付一次 commit。取消恢复拖动开始值；如果期间外部更新已替换受控值／range，保留外部新值并结束拖动，不用旧快照覆盖它。
- 禁用、失焦、几何丢失、默认释放被阻止和卸载均有完整取消出口。回调可以移除自身，之后不再访问退休资源。

Resizable 以有稳定 key 的 panel 和受控比例／折叠状态描述布局，min／max 使用逻辑长度。拖动只调用 resizePair 改变相邻两项，更新比例后保留总长；窗口尺寸变化重新分配但不当作一次用户提交。最小尺寸总和超出可用区域时产生明确 overflow，不能出现负 panel。折叠仅对显式允许的面板开放，恢复使用模型保留的最近展开比例；分隔把手支持键盘和可见焦点。

Carousel 使用实际测量的 item 起点作为 snap，外部选择按稳定 key 或当前索引明确映射。Drag 达到阈值后取消子项点击，结束用 snapTarget 选择锚点并驱动同一 Surface 动画；方向变化、窗口调整和外部定位使旧目标失效。循环模式维护环形顺序，回绕不清空项目状态；接缝的必要重复视觉不能产生第二个可操作实例。内容不足一屏时停止循环和无意义的上一／下一动作。自动播放是明确配置，仅在可见、未拖动／聚焦暂停且有下一目标时预约一次 deadline，不复制 Embla 的插件系统。

## 3. 文本与 OTP

Input、Textarea、InputGroup、OTP 与可编辑 Combobox 复用 TextEditorState／TextEditHost。普通输入的 required／invalid 是业务输入；onEdited 是编辑成功后的通知，不能拿它做字符过滤或修改光标补丁。过滤、长度和粘贴转换都进入已有 TextEditPolicy 事务。

OTP 默认接受 ASCII 数字，可选字母数字或显式策略；length 为正的槽数量，不是 UTF-8 字节数。每槽容纳一个被策略接受的 grapheme。粘贴按策略决定移除分隔符和截断超长内容，结果一次性写入，Undo 一次撤销整个粘贴。composition 的最终提交经过相同策略，原始 preedit 和失败恢复仍沿公共宿主规则。

OTP 的 group／separator 只影响显示。原文、复制、提交和长度判断不含显示间隙；按键导航、选区拖动、空槽点击和 IME 候选框采用 [API.md](API.md) 的同一槽几何。完成通知仅在一次被接受的编辑使正文达到完整长度时交付；外部 replaceAll、主题切换和重复 render 不重复调用。是否自动提交由应用选择。

日期输入、Combobox query 和 Questionnaire freeform 同样保存原始草稿；解析／候选／提交结果不能反向创建一份每字符同步的 String 模型。Password 显隐使用原有投影和复制策略，不因切换显示重建 editor。

## 4. Calendar 与 DatePicker

默认日期采用现有 `scoop.datetime.LocalDate`／YearMonth／DayOfWeek；它们是日期值，不带时区。选中 11 月 4 日就是这个日期，不在控件里转成午夜 UTC 再转回来。今天、周起点、月份全名／简称、星期名称、数字与日期格式由明确的日期／locale 数据输入；涉及 Instant、系统时区或 IO 的转换由调用方完成。

选择模式是三个完整入口：Single 的可选日期、Multiple 的唯一日期数组、Range 的可选完整 DateRange。DateRange 保证 start≤end，nights 为 start.daysUntil(end)，同一天为零夜；范围第一下点击保存暂定起点和预览，不向外发布缺失终点的范围。第二下点击按规则形成完整有序范围，校验后一次提交。CalendarRules 限制 required、minimumDates／maximumDates、minimumNights／maximumNights、minDate／maxDate、disabled 和 allowDisabledInside；不会静默跳过日期后形成另一段范围。

Calendar 的界面状态只包含显示月份、当前焦点日期和暂定范围。外部改 selection 时更新高亮；是否把月份移动到新值由明确的同步选项控制。方向键±1／±7 天，Home／End 到周边界，PageUp／Down 翻月，Shift+Page 翻年；所有运算复用日期库。只在本次显示页的有效日期中确定可用焦点，整页不可选时退到翻页入口，不为找到“下一个可用日”无限扫描年份。

月布局复用有限 Grid：七列、完整周、可配置 outside days、周号、多月页、月／年 Select 和日期 footer。cellSize／字体变化真实重测；选中范围的起点、中间、终点边角随逻辑方向解析。默认显示公历；周编号的 ISO／本地规则和 locale 值明确分开。

上游列出的其他历法以实际 CalendarPageSource 接入：月页给出可显示的日序、标签、周序与每格对应的 LocalDate，上一／下一页由来源决定，选择结果仍是日期。默认提供公历实现和自定义月页消费用例；这个扩展保证 UI 能展示调用方提供的 Persian／Hijri 等日期数据，**本轮不声称已实现这些历法的换算引擎**。需要内置换算时，应在日期库独立设计并选用成熟实现，不能把一套历法算法藏进 Calendar 绘制代码。

DatePicker 是 Popover、Calendar 和按钮／输入的实际组合，支持日期、范围、生日选择、预设值和时间输入。核心合同：

- 按钮模式的文字由选中值格式化；输入模式使用 DatePickerDraft 的唯一 TextEditorState 和可观察 error。默认 Enter 提交，commitOnBlur=false；显式启用 blur 时，在本次输入事件结束后提交。默认解析严格 ISO 日期，范围使用 `start .. end`；解析失败显示错误并保留草稿。
- 外部选中值变化时，非编辑中的草稿同步；编辑／composition 中的更新由明确的 accept／reset 动作收束，不能覆盖正在输入的文字。
- 日期和时间分别采用 LocalDate／LocalTime，应用需要时组成 LocalDateTime；控件不推测 timezone。
- 自然语言示例接受显式 parser、参考日期和 locale；解析只由提交动作触发，返回 DatePickerParse.Date／Range／Empty／Invalid。没有 parser 时不能把任意字符串猜成有效日期；这不要求组件库重写 chrono-node。
- Escape 先交给 IME，再关闭 popup；取消不提交暂定 range／无效草稿。点击日历提交时，输入的晚到 blur 不得覆盖刚选中的日期。

## 5. Table 与 DataTable

Table 负责有限表格布局及部件。DataTable 在此基础上提供实际行模型、查询状态和虚拟正文，范围对应官方指南的排序、筛选、分页、列可见性、行选择与 actions，不扩展为电子表格、任意查询语言或 TanStack 插件宿主。

TableColumn<Row> 必须有稳定列 id、header 和类型化 cell 渲染函数；需要本地排序／过滤时显式提供 `(Row, Row) -> Int` 与相应 Row predicate。不把字段提取为 Any 再靠字符串反射分派。TableSource<Row> 提供 count、keyAt／indexOfKey 和 rowAt；key 不是当前页索引或显示内容。

| 模式 | 谁执行查询 | 控件保证 |
| --- | --- | --- |
| Local | 基于完整本地数据，用列提供的比较／筛选函数计算 | 过滤→稳定多列排序→分页；结果按数据／查询变化复用，不每帧重排 |
| Provided | 调用方给当前页／结果和总行数／页数 | 控件更新同一 TableQuery 并通知请求；不二次排序过滤，不假装持有未加载行 |

TableQuery 包含排序描述、列过滤输入、pageIndex／pageSize；选中行 key、列可见性和列宽是各自明确状态。pageIndex 从 0 开始，UI 页码从 1 开始。变更过滤／排序默认回第一页；行数减少使页号失效时在一次模型更新中收敛，不能在 render 中反复回写。外部模式的请求、取消、loading／error 和迟到结果处理归应用，同一个最终结果经正常 Props 进入。

落地入口为应用持有的 `DataTableState<Row>(data, columns, query, selectable)` 与 `dataTable(state)`。replaceData、setQuery 和选择／显隐／宽度方法是普通应用动作，render 只读取快照。TableData.Provided 接收当前页来源与 totalRows，页数按 pageSize 推导；onQueryChange 交付成功更新后的查询。列定义固定在一个模型内，本地分页复用已有筛选／稳定排序结果。

P3 已实现的独立 Pagination 只接收 Binding<Long> 和 pageCount，不承担上述表格查询规则。`paginationItems` 用边界与邻近范围生成有序 Page／Ellipsis，只缺一页时补齐页码，多页缺口跳过；空数据和 Long 上界有定向覆盖。外部越界页码只在展示窗口计算时夹取，绑定原值保留；业务模型负责决定何时重置。实际改变后通知 setter 采纳值，拒绝写入时不通知。

表头全选明确作用于**当前结果页的可选行**，三态从该集合计算；跨页已选 key 保留。库不把“勾选本页”解释为后台所有匹配记录，更不生成抽象的“全量减例外”选择协议。删除已知行时由模型更新移除相应选择；Provided 模式中暂未出现在本页不等于已删除。

Header 和可见行消费同一个不可变列宽数组。Fixed／比例列直接解析；Auto 的宽度来自明确的表头／给定测量样本，不能为了自动列宽扫描或挂载百万行。固定 header 在正文滚动区之外，横向位移采用一份共同布局／滚动来源；拖列宽重新计算一次轨道后通知全部需要的行。正文使用 VirtualList，行内输入、菜单和按钮继续按既有活动项保留与焦点显露规则处理。

验收包含重复显示值但不同 key、稳定排序、空页、最后一页删除、部分禁用行、本页 Mixed、外部加载结果替换、隐藏列、长内容、水平滚动及虚拟行中的弹层。不得用一次漂亮的支付表格截图替代这些行为。

## 6. Chart

Chart 交付六类真实图表，与 [shadcn Chart](https://ui.shadcn.com/docs/components/chart) 的常用图表范围对应。Scoop 没有可直接运行的 Recharts，采用现有 graphics／graphics.text／math 实现这一有限绘图需求；不嵌入 WebView，不包装静态图，也不新增一个通用 renderer。

ChartModel 使用类型化系列与数据：稳定 SeriesKey／DatumKey、显示 label、可选颜色和数值／分类坐标。可空样本用 Option 表达缺值，NaN／Infinity 不是合法缺值编码。格式化是明确回调，Tooltip／Legend 使用同一系列名称和颜色。可见系列状态独立于数据，颜色按稳定系列身份分配，隐藏一条系列不使剩余颜色乱序。

| 图表族 | 本轮实际能力 |
| --- | --- |
| Area | 单／多系列、线性／阶梯／单调曲线、渐变填充、普通／归一化堆叠 |
| Bar | 纵／横、分组／堆叠、正负值、圆角、标签 |
| Line | 单／多系列、点／标记、曲线、缺值断开或明确连接 |
| Pie／Donut | 非负值比例、起止角、内半径、active sector、标签／中心内容 |
| Radar | 分类极轴、单／多系列、网格和刻度 |
| Radial | 径向数值条、起止角、单／多项、背景轨道和数值标签 |

共用有限的线性／分类尺度、tick 生成、文字测量、plot rect、路径与命中算法。相同轴域的 Bar＋Line 等组合可以共用一个 plot；多坐标系统、地图、任意缩放／刷选、3D、金融图及 Recharts 全部扩展不属于本轮。普通数值轴有明确的手动／自动 domain、零点和 nice tick 规则；常量域、空数据、全零饼图、负数与缺值都有明确结果。

算法先用成熟公开实现核对刻度、堆叠和曲线规则，实际复用代码时记录许可证；不从截图估算数值比例。geometry 与 hit-test 共用同一数据／尺度快照，文本真实排版。Legend／Tooltip 支持图标、标签、数值 formatter、隐藏标题和点／线／虚线指示；Tooltip 支持鼠标与键盘数据项导航，不能只有鼠标 hover。

数据／大小／字体／系列显隐变化才重算对应布局；pointer 移动仅更新 active datum 和提示，不能重建全部数据。动画绑定 stable key 并复用 Surface，结束／隐藏／减少动态后停帧。大输入不自动丢点；降采样若需要由显式数据准备决定并保留语义，不作为隐含绘图优化。

验收除截图外，精确检查值域到坐标、堆叠总量／正负基线、角度和命中结果；六类各有真实数据例子，并组合 Tooltip、Legend、Dark、RTL 布局及窗口缩放。

## 7. Toast

ToastState 是 owner 线程上的显式模型，一个活动 Toaster 绑定一个状态。`show` 返回 ToastId，`update(id, ...)` 替换同一条通知，`dismiss(id)` 请求退出；完成后 id 不再可更新。它不是进程级 singleton，也不沿隐式 Provider 找宿主。相同状态不能同时由两个活动窗口处理计时和关闭。

记录包含内容／动作、类型、可选位置覆盖、持续时间及明确的关闭策略。默认普通通知 5 秒，Loading 持续到应用更新或关闭；默认位置、每位置可见数量和队列顺序属于 Toaster 的布局配置。可见上限是通知展示策略，不影响程序或数据合法性。update 替换当前内容并重新开始新持续时间；排队、暂停及没有活动宿主时不消耗这段时间。

进入和退出用 Presence，剩余时间用既有单调时间及最早一次 deadline。hover、焦点进入、窗口失活／隐藏时暂停剩余时间；恢复后继续，不为暂停通知反复唤醒。排队且未显示的通知不消耗展示时长。用户滑动关闭复用 Drag，取消回到原位置；手动关闭、超时和动作关闭只选择一次结果。

异步任务在应用／组件 Job 中运行，用同一个 id 更新 Loading→Success／Error。可以提供普通组合 helper 包住现有任务，但不向 Toast 注入 HTTP 依赖或独立 executor。任务在通知被关闭后完成，更新返回无效结果，不重新创建已关闭通知；任务自己的资源收尾仍由原 scope 负责。

通知默认不夺取当前输入焦点。用户聚焦 action 后关闭时，按同一非模态焦点会话规则恢复或选择仍有效的下一通知；不得覆盖新显式焦点。Toaster 卸载取消计时／动画并解除绑定；尚未关闭的记录保留剩余时间，后续重新挂载可继续展示，已请求退出的记录完成一次关闭。外部任务只能更新仍活动的模型记录，不持有 painter。

## 8. 消息、附件与阅读位置

Message 只做行布局，Bubble 做内容表面，Marker 做状态行／分隔，Trace 做步骤行／展开组合。Attachment 的状态由应用提供，图片由调用方装载，上传／重试／取消是显式动作。几者都接收普通 View／StyledText，流式文字更新不改变消息 key，不依赖 AI SDK、Markdown 或 Code Block。

Trace 的完成态无运行标记，Running／Error／Pending 有明确文字和视觉；运行时默认展开只在用户尚未明确操作该行时生效。用户手动折叠后，流式追加不得强行重开；显式外部 open Binding 按受控规则生效。嵌套 Trace 不自行启动任务或计算业务进度。

### 8.1 TranscriptSource 与 MessageScrollState

TranscriptSource 的每行有稳定 key、正文 View 的按需构造、是否为回合锚点；消息、Marker、加载历史入口和状态行均可成为行。回合锚点由应用明确标记，不从“用户／助手”角色或消息文字推导。结构元数据可建立普通索引；当前回合查询不能每帧扫描全部历史，也不需为索引创建 View。

MessageScrollState 组合已有 ScrollState、VirtualListHandle 和阅读快照，只新增会话阅读策略。默认值跟随当前 [MessageScroller API](https://ui.shadcn.com/docs/react/message-scroller)：`autoScroll=false`、初始位置 End、边缘阈值 8、margin 0、previousItemPeek 64、preserveOnPrepend=true。示例可显式选择 LastAnchor，但不能把文档推荐配置误记为 API 默认值。

| 状态／事件 | 行为 |
| --- | --- |
| 首个非空内容 | 一次性应用 Start／End／LastAnchor；LastAnchor 找最后的显式回合锚点，缺失或该回合已可完整显示时退到 End |
| Holding | 保存最近 Accepted 的阅读 key／项内偏移；追加和增高只保持位置，不强制跳到末尾 |
| `beginTurn(key)` | 应用在发送等明确动作后请求把该回合放到顶部附近，保留 previousItemPeek；从已有请求位置接管 |
| Anchoring | 回复先长入锚点下方可用空间；逐步缩减尾部留白，保持回合起点。填满视口且 autoScroll 开启时转为 Following |
| Following | 仅在 autoScroll 开启、用户保持 live edge 时随新增内容保持末端 |
| User wheel／滚动条／翻页／正文选择／内容操作 | 立即停止自动位移并进入 Holding；包括到边界而没有实际位移的 User 意图 |
| `scrollToMessage(key)`／搜索定位／restore | 明确定位并转为 Holding，不在结束后恢复旧的自动跟随 |
| `scrollToEnd()`／跳至最新 | 定位到真实末端；autoScroll 开启时重新跟随。用户主动滚回末端且没有活动选择等保持原因时也可重新跟随 |
| prepend、图片增高、Trace 展开 | 用现有 KeepVisible 和 LayoutCorrection 保持位置；不是一次新的 User／Programmatic 意图 |

简单新增 `scrollAnchor` 元数据不会打断正在阅读的 Holding；主动发送使用 `beginTurn`，或按应用明确选择的跟随策略触发新回合定位。选择和外部搜索等无法只靠 ScrollState 发现的动作，通过组件正常输入路由或显式 `hold()` 交付，不侦听 OS 全局输入。

新回合先将 scrollMargin 与实际可显示的 previousItemPeek 合成为视口内的 anchorInset；无上一项时 peek 为零。末尾留白为 `max(0, viewport - anchorInset - turnExtent)`，turnExtent 是锚点到正文末尾的实际高度。留白通过 VirtualList 的逻辑末端空间参与范围，不伪造一条消息；增高和窗口缩放时重算。正文未填满时缩小留白，不能同时又执行 FollowEnd 把回合起点拉走。普通 Holding 不继承一份会主动移动读者的旧定位请求。

首个目标位置还未准备好时先显示静态容器／加载状态，完整场景再发布，不先闪现最老消息后跳转。按 key 定位复用现有按需测量；已知源中的未挂载项可接受定位，不存在的 key 返回 false。仅初始尚未取得数据时允许保留一次显式目标请求，源到达后解析一次，不做无限重试。

可观察结果来自 Accepted：visibleKeys、currentAnchor、atStart／atEnd、reading snapshot。程序请求、估计尺寸和 Discarded 帧不是阅读事实；不据此标记已读。模型更换会话时以显式新状态／reset 结束旧意图。会话持久化保存 key／偏移等普通数据，控件不读写存储。

验证必须覆盖：一屏以内／超长回复、用户向上读时持续追加、prepend、异步图片、嵌套 Trace、键盘和文字选择中断、缺失跳转目标、恢复期间源替换、空会话和关闭中的迟到更新。只测 VirtualList 的 FollowEnd 不足以验收 MessageScroller。

## 9. Questionnaire

QuestionnaireState 保存定义、各题的回答模型、活动题和显式 Skip／校验状态。Question 和 Choice 各有稳定的领域 key；题目顺序变化不以数组位置搬错答案。题型为单选、多选及 freeform，可在选择题中提供 freeform 选项。每个 freeform 使用唯一 TextEditorState，题目隐藏后保留；序列化时才生成文本快照，不再维护一份同步 String。

输出使用有载荷的普通类型：答案项是 Choice(key) 或 Text(value)，题目结果是 Unanswered、Skipped、Single(answer) 或 Multiple(answers)。单选 freeform 与固定选项互斥；多选最多包含该题实际声明的一项 freeform，不用分隔符字符串保存选中 key。结果快照和恢复输入遵守题型检查，不是新的 schema／反射框架。

| 操作／状态 | 规则 |
| --- | --- |
| Answered | 有有效固定选择，或已选择且满足该题要求的 freeform 内容；空白是否合法由题目规则决定 |
| Unanswered | 没有答案且没有显式 Skip；与 Skipped 不合并 |
| Skip | 只对 optional 题开放，记录明确跳过并移到下一有效题；required 题不能 Skip |
| Next | 先校验当前题；required 必须 Answered，optional 也必须 Answered 或显式 Skipped；失败留本题并聚焦有效回答入口 |
| Previous | 回上一有效题，保留答案，不因当前未答阻止返回 |
| Submit | 校验全部 enabled 题，再交付一次不可变结果；失败定位第一个无效题并显示错误 |
| Disabled 条件题 | 不计进度、不参与导航／校验／提交；保留草稿，重新启用后按当前定义检查 |
| 恢复与 reset | 初始快照含活动题、回答和 Skip；reset 恢复该基线并清空当前错误／组合与编辑历史，不隐式发送结果 |

外部校验通过题目错误数据和活动题更新接入；业务异步校验／保存由调用方执行，完成后把结果交回原 owner。默认 Next／Submit 保持可激活，让用户尝试后看到错误；应用可根据模型状态明确禁用。提交中的 busy 是显式业务输入，此时不重复交付提交，完成后是否允许再次提交仍由调用方决定；一次激活事件只产生一次结果。没有 enabled 题时进度为 0／0，无活动回答入口，Submit 可交付空结果。

字母／数字快捷键只为 enabled 固定选项按定义顺序分配；文本输入和 IME 活跃时暂停这些快捷键。快捷键选择不自动翻题。ArrowLeft／Right 仅在不会抢走文本／Radio 默认动作时前后导航；Primary+Enter 校验并继续或提交最后一题，composition 的同次提交键不继续触发问卷提交。

成功翻题把焦点放入新题，失败聚焦本题的可用回答控制；已退出题目和不可用 actions 不参与输入。需要动画时只动画新题进入，进度和导航保持稳定。Dialog 关闭表示取消当前展示，不等于 Skip、不清空答案，也不自动调用 onSubmit。

验收至少包含单／多选与 freeform 混合、required／optional／Skipped 的区别、条件题往返、外部错误回跳、恢复后的 reset、重复提交防护、中文 IME、重排／删除题目及弹层取消。具体文案由可替换的值／formatter 提供，内置英文与简体中文展示用例验证不同长度布局，不引入 locale 服务或表单网络协议。
