# Window 无障碍接口规范

状态：功能已接通，三平台集中验收中。本文规定窗口内容的低层语义节点、原生投影、文本范围、虚拟集合与同步动作边界；UI 资源、内容合并与场景适配见 [UI 无障碍接口规范](SCOOP-UI-ACCESSIBILITY-SPEC.md)。平台构建、原生服务、外部客户端与读屏器分别按[完整实施计划](../ui/interaction-polish/PLAN.md)验收，实际结果见[实施记录](../ui/interaction-polish/PROGRESS.md)。

## UIKit 内容视图

iOS 连接的场景窗口可创建同一个 `AccessibilityProvider`。内容视图按语义顺序公开有可见几何的节点，复用稳定的 `UIAccessibilityElement`。名称、值、已选、禁用、标题和按钮等特征来自发布的属性，元素位置和激活点使用已提交的视图逻辑点。滑动、增减、激活和关闭动作只进入对应的 Scoop 回调一次。关闭节点或断开场景后，系统持有的旧元素不能再触发动作。静态文本未指定名称时以公开正文作为朗读标签。编辑器成为第一响应者时，容器在同一阅读位置公开实际的 UITextView，沿用语义节点的标识、名称和已呈现几何。结束编辑后恢复该节点。受保护文本不对外提供正文。读屏焦点与键盘焦点分开，布局变化与实时公告按 UIKit 通知交付。[UIKit 无障碍元素](https://developer.apple.com/documentation/uikit/uiaccessibilityelement)

UIKit 在阅读顺序中保留提供名称、说明或值的滚动区域和滚动条。纯布局用的滚动区域和滚动条由其内容承接读屏焦点。从子元素发起的读屏滚动会查找最近的可滚动祖先。

## 1. 包、身份与寿命

`scoop.window.accessibility` 属于 Window Cone，不依赖 Surface、UI 或控件。SemanticRole、SemanticProperties、SemanticActions、SemanticActionResult、SemanticGeometry 与 SemanticNodeId 为公共值；AccessibilityProvider 和 AccessibilityNode 为 owner 线程资源。`Window.createAccessibilityProvider()` 使用统一的差量发布与 owner 动作接口；Darwin 连接 AppKit，Windows 连接 UIA，Linux GNU 连接 AT-SPI。后两者的实现与验收仍按进度记录独立核实，不放置空 provider 或成功占位返回。

`SemanticProperties.copy { ... }` 用 SemanticPropertiesBuilder 修改一份完整属性快照，未指定的字段保留原值，返回值仍不可变。复制本身不发布或执行动作；角色及属性组合在节点 update 的同一边界检查。UI 宿主和组合控件用此入口保留应用提供的状态与关系之外的属性，避免只更新文本或可见性时遗漏其他字段。

每个 Window 同时至多有一个内容 provider。createAccessibilityProvider 接受 PendingConfigure 或 Open；重复附着报 IllegalStateException。首次配置前允许创建、更新和发布语义及读取已发布集合，不要求 drawable、可见窗口或几何；原生动作仍要求 Open。createNode(role, parent = None) 创建节点并返回 AccessibilityNode，parent=None 表示内容根下的直接子项。角色在该节点生命期固定；更换角色需退休并重新创建。节点 id 的窗口所属标识独立于原生地址，序号在窗口生命期递增、不复用；关闭 provider 后重新附着仍延续序号。不同应用运行产生的 WindowId 数字相同也不使旧语义 id 相等。

节点的 parent 与 order 表达语义阅读顺序，同序号按创建次序排列，不依据屏幕坐标排序。reparent(parent, order = 0) 要求父节点在同一活动 provider，拒绝自身或后代为父；失败不改变原关系。close 退休整个节点子树，随后修改节点或读取 geometry 报 IllegalStateException；id、role 和 isClosed 仍可读取，重复 close 无操作。provider.close 退休全部节点；Window 进入 Closing 时立即关闭 provider，不等待 drawable 或业务回调。已退休的原生对象可以由客户端继续持有，但不再读取或保活 Window／UI，不能执行动作。

## 2. 属性与发布

Tree／TreeItem 表达树容器和树项。Tree 可发布普通层级集合，也可同时提供 SemanticGrid 和表格尺寸，表示多列树；列、表头和单元格继续复用同一表格合同。AppKit 的 Tree 映射 Outline，TreeItem 映射 Row；Windows 映射 Tree／TreeItem，Linux GNU 映射 Tree／TreeItem，多列树仍保留表格能力。静态树项沿既有节点父子关系；惰性树项以同一集合中的稳定 key 表达层级，不为每个逻辑项创建实际控件。

首批角色为 Group、Text、Button、Checkbox、TextField、ScrollArea、ScrollBar。SemanticProperties 保存 name、description、error、automationId、value、enabled、readOnly、protectedText、checked、hidden 与 ignored；名称、说明、错误、标识和值采用可选字符串，已知空字符串与未知值不同。checked 为可选 Off／On／Mixed。只有实际文本用途才设置 protectedText；受保护时不允许通过 value 发布明文，不因屏幕暂时显示正文而解除保护。hidden 排除整个节点子树但保留身份，恢复后使用原身份；ignored 仅忽略当前节点，保留后代并把它们接到最近未忽略的语义祖先；hidden 优先排除整个子树。忽略不退休身份、不改变实际键盘焦点，旧引用不能执行被忽略节点的动作；后代继续遵守祖先的 hidden／enabled。update 在改变原状态前检查这些规则。

SemanticProperties.required 为 Boolean，默认 false，描述表单是否要求填写或选择；它不执行校验、不改变 enabled／readOnly 或文本内容。AppKit 通过公开的 NSAccessibilityRequiredAttribute／isAccessibilityRequired 读取该事实，客户端不能改写它。required 变化不改变节点身份、正文版本或几何，发布后发送适用的属性布局通知；隐藏／退休后的 selector 与其他只读属性遵守同一可访问性边界。

选择控件还提供 Switch、ToggleButton、RadioButton 与 RadioGroup。前两者在 AppKit 使用 Button 角色及公开的 Switch／Toggle subrole，checked 的 Off／On 为实际布尔值，Press 调用 toggle；Mixed 仅用于 Checkbox，其他选择角色拒绝该值；RadioButton 使用原生同名角色、Off／On 值及 invoke 选择动作，不能用反转操作取消单选。RadioGroup 使用原生 RadioGroup 角色，其条目保持普通父子／阅读顺序，组本身不凭角色获得选择或焦点动作。checked 变化发布值通知；查询不改变选择或键盘焦点。

Image 表示具有整体替代名称的图像或自绘图形，AppKit 映射为 NSAccessibilityImageRole。name／description 使用同一属性与关系合同，几何沿当前已呈现场景发布；角色本身不隐含点击、选择或键盘焦点。装饰图形仍应忽略，实际可操作图形通过显式动作或独立控件提供等价入口。

update(properties, actions = SemanticActions(), relations = SemanticRelations()) 替换属性、最新动作实现及关系。值／名称变化使旧几何失效；新的匹配布局可通过 geometry 再发布。只读 contentRevision 随影响名称／值的变更递增，供场景比较捕获时的内容版本；关系名称的依赖变化同样使旧候选失效。SemanticGeometry 保存窗口内容逻辑坐标中的 bounds 和可选 visibleBounds；后者必须在 bounds 内，且两者坐标有限、尺寸非负。None 表示几何不可用；visibleBounds=None 表示已经有合法布局但被自身裁剪，不表示节点退休或程序化隐藏。宽度或高度为零的可见矩形不进入 visible children，仍保留合法布局与语义身份。矩形仅描述外接区域，不能据矩形中心推断任意自绘形状的可点击点；精确命中使用本节的独立 native 快照合同。

SemanticRelations 保存有序的 labelledBy、describedBy、errorMessages、controls 节点 ID 数组，默认均为空，以有序内容判断相等。每次替换在修改前检查同一活动 provider 的目标、同一数组无重复和非自身引用；controls 等关系不套用父子树的无环规则。有效名称的循环依赖在 publish 解析受影响名称时拒绝，保留上次原生投影和待发布变更，调用方可修正后重试。显式名称已截断的依赖不算有效名称环；不会为了未采用的标签重新遍历整张关系图。关系不改变父子归属或焦点，不保活已退休的目标；目标关闭时自动清除所有指向它的关系，普通重排保持引用身份。node.relations 只读返回清理后的当前关系。

node.contentLabels 是单独保存的有序内容标签 ID 数组，默认空，采用与关系相同的归属／重复／自身检查与退休清理。它只提供名称的第三优先来源：显式 name → labelledBy 的已知文字 → contentLabels 的已知文字；已知空串同样截断后续回退。普通 update 不清空 contentLabels。底层不会据内容引用自行隐藏节点，UI 等实际调用方负责声明合并与 ignored；内容引用、普通标签及其混合有效依赖共享同一次名称解析与循环检查。

各类标签按数组顺序连接已知的目标名称；被引用的静态 Text 没有名称时可用其非受保护 value 作为标签文字，但不因此为该 Text 自身重复发布 name。description 和 error 分别采用显式属性优先，否则从对应关系取得目标文字。未知条目跳过，已知空文字仍与全部未知区分；连接非空文字时使用单个空格。显式关系可以使用仍活动但 hidden／离屏的标签文字，不把被引用节点重新放回导航树，也不公开受保护 value。目标的名称或静态文字变化只更新实际依赖者，发布后原生 getter 直接读取解析结果，不进入应用代码或重复遍历名称图。

创建、修改和重排先修改 managed 状态，publish() 在正常 owner 更新完成后原子发布本批差量；未变节点不重复序列化和验证，单纯替换相同能力的 handler 不更新 native 属性。原生 getter 只读取已发布投影，不调用 Scoop handler 或布局。原生准备失败保留上一份投影，差量仍待处理；已经原子提交的结果不因随后平台通知失败回滚，错误按普通 native 操作失败报告。退休立即禁止旧对象动作，结构变更在 publish 时统一通知；新节点发布前不进入原生树。几何合法性在设置 geometry 的边界检查，失败保留原值。

AppKit 对已发布节点的纯几何更新复用现有逻辑属性、文本和关系，并在同一事务中批量替换边界、可见区域、命中、场景顺序、激活点和文本范围几何。同轮有逻辑变化的节点以完整记录发布。完成提交后，原生查询和几何通知使用本轮接受的差量。

focused 属性保存当前实际获得键盘焦点的活动节点或 None。它不代表辅助技术阅读位置，也不因为 Focus 请求已排队就改变。publish 先发布属性、结构和实际焦点，再发送适用的结构／值／焦点通知；无差量时不通知、不请求帧。几何仍由调用方使用匹配的 Accepted 场景提供，provider 不替调用方启动绘制。

node.geometry 保存值型 bounds／visibleBounds；只赋矩形不提供精确命中。原生 renderer 可使用 `@Unsafe node.setGeometry(geometry: SemanticGeometry, hitTest: NativeSemanticHitTest, order: Long = 0L)` 同时登记匹配几何、命中快照和实际场景顺序。NativeSemanticHitTest 位于 `scoop.window.accessibility.native`，是 context、retain、release、contains 的 CLayout 值；后三者分别为 FunPtr<(Ptr<Unit>) -> Unit>、同签名释放函数和 FunPtr<(Ptr<Unit>, Double, Double) -> Int>。调用方提供成套的原生只读实现，contains 接收窗口逻辑坐标并返回 1／0，函数不回调 Scoop、进入 UI 状态或抛出跨 ABI 异常。

setGeometry 在验证几何后为节点持有一个引用；原生投影在发布时独立保留自己的引用，调用方可在 setGeometry 返回后释放借用。替换、名称／值失效、退休和关闭释放对应引用，准备失败时旧原生投影仍有自己的有效快照。普通 geometry 赋值明确替换为仅有矩形的几何并清除旧命中；None 同时清除位置与命中。Window 不检查 Path 或重建 clip 引擎，第三方 renderer 可提供自己的实际原生命中实现。

命中顺序使用场景 order，值较大者在上，同值按节点创建顺序确定；它与阅读顺序分别保存。命中忽略 hidden 子树及已退休对象，ignored 自身命中映射到最近未忽略祖先，独立后代仍可成为结果。disabled 对象仍可被辅助技术定位和读取，动作继续按 enabled 拒绝。缺少有效命中快照时不根据 bounds 猜测结果，也不把矩形中心声明为有效激活点。

SemanticRelations 还提供 `activeDescendant: SemanticDescendant?` 以及 rowHeaders／columnHeaders。SemanticDescendant.Node 引用同一 provider 的活动节点，Item(collection, key) 引用同一 provider 中集合的稳定逻辑条目；它们不改变键盘焦点或选择。节点退休自动清除引用，条目删除后不解析成同位置的新项。表头关系只引用实际语义节点，重排更新顺序但保留身份。原生端以各平台的活动后代与关联元素合同表达，不能为表达活动项发送虚假键盘焦点通知。

### 2.1 控件状态与数值范围

角色还包括 Link、Heading、Slider、ProgressBar、Menu、MenuItem、TabGroup、Tab、Dialog、Table、Cell、RowHeader、ColumnHeader、Disclosure 与 Separator，用于实际存在的相应控件。角色本身不授予动作。SemanticProperties 的 selected／expanded 为可选布尔值，None 表示不适用；invalid、busy、modal 默认 false，分别表达实际校验结果、业务忙碌与真实模态边界，不自动建立验证器、禁用控件或阻塞输入。headingLevel 为 1～6 的可选整数，仅用于 Heading；url 是 Link 的可选目标，不因提供 URL 自动执行导航。

`SemanticValueRange(minimum, maximum, value, step = 0)` 表达有限数值，minimum ≤ value ≤ maximum、step ≥ 0。SemanticProperties.range 用于 Slider／ProgressBar／可调整 Separator 的实际数值；ProgressBar 的 None 表示进度未知，不伪造零进度。范围与文本 value 不同时发布。数值写入／增减只在真实 handler 存在时公布；请求不改变键盘焦点。范围、选择和展开变化发送相应原生通知，静止状态不重复通知。

`SemanticActions.setNumber(Double)`、increment／decrement、setSelected(Boolean)、setExpanded(Boolean)、dismiss 分别对应实际数值、选择、展开和关闭业务入口。写入要求适用状态存在，readOnly／disabled／模态／退休检查与原动作相同。数值越界或非有限返回 InvalidArgument；实际控件仍按原步长和 setter 采纳规则处理。Completed 只表示请求被实际业务入口采纳；拒绝受控 setter 返回 Rejected。

### 2.2 公告与动态区域

`SemanticLive.Off／Polite／Assertive` 表达公告优先级，SemanticProperties.live 默认 Off。`AccessibilityNode.announce(text, priority = Polite)` 是显式公告，不隐含焦点移动；同一 owner 发布周期内同一节点的新公告替换旧公告。live 非 Off 的节点在已首次发布后名称／公开值变化时公告最新公开值或名称；节点或祖先 busy 期间合并，解除 busy 后发布最新消息。隐藏、退休、模态背景和 protectedText 不公告；首次挂载与普通几何／hover／动画更新不公告。文本不得包含未公开的受保护内容。原生端按公开公告／通知接口映射优先级，不增加定时轮询。

## 3. 动作与同步边界

SemanticActions 按实际实现提供 invoke、toggle、setValue(String)、focus(Boolean)、showContextMenu() 及 §3.1～§3.3 的文本、滚动与显露动作；focus 的 true 请求聚焦，false 请求解除该目标焦点。每个回调返回 SemanticActionResult：Completed、InvalidArgument、Unsupported、Unavailable、Rejected 或 Busy。缺少回调时不公布对应原生能力；enabled=false 拒绝交互，readOnly=true 拒绝值写入及切换，但不禁止有实际内容的菜单。只有实际业务已执行才返回 Completed；Binding 拒绝不能提前改写投影，异步业务结束也不属于 Invoke 的完成条件。

AppKit getter 不进入 managed 代码。同步动作使用创建 provider 时登记的 Reusable ForeignCallback，沿已有 Context 捕获规则恢复创建上下文，包括 asyncio 集成的 scope。它只在 owner、应用未退出、窗口 Open 且输入未被模态阻止、没有进行中的 Window 业务／watch 回调或另一动作时执行。重入时返回 Busy，不等待自己的队列、不 pump、不布局；UI 适配另检查宿主派发、scope 回调、render／布局和实际输入边界，见 UI 无障碍规范。普通 handler 仍沿既有异步派发路径。

动作先保存回调与所需值，再执行；回调可以退休自身、关闭 provider 或 Window。执行后仅按资源仍存活的情况发布实际结果，不继续使用旧节点。异常在 managed 边界捕获，经正常应用错误与退出路径报告，不跨 C／Objective-C ABI；UI 后续将自身动作包在既有 Surface／根错误边界中。动作中可能打开的原生菜单／面板继续使用已有延迟 native operation 通路。

NSAccessibility 的 performPress 等 BOOL 动作仅在 Completed 时返回 YES。setAccessibilityValue 与 setAccessibilityFocused 为无返回值 setter：只在实际回调中改变业务状态，失败时保持旧值；调用方必须读取结果，不能把 setter 返回解释为写入成功。原生注册在释放 callback 前撤销；动作中关闭时，callback ownership 经现有 owner control 队列在调用退栈后释放。客户端晚到查询／动作不会访问已释放 context。

AppKit 的跨进程 AX 桥由系统实现；`AXUIElementPerformAction` 的 kAXErrorSuccess 可能只确认调用已送达，即使原 BOOL 动作拒绝也返回该值，标准禁用 NSButton 同样如此。库不把这一系统状态再解释为 SemanticActionResult.Completed；客户端结合当前 enabled、实际值、焦点或对象退休状态判断业务结果，校验中也分别记录跨进程调用与真实 handler 的结果。聚焦后通过应用的 AXFocusedUIElement／AXFocusedWindow 及目标自身的 AXFocused 核对，不要求窗口自身 AXFocused 与后代节点相等。

showContextMenu 映射 AppKit accessibilityPerformShowMenu，经相同 owner／生命周期／输入边界执行真实菜单请求，不合成右键或建立第二份菜单模型。Completed 表示调用方已经创建绑定该目标的菜单会话；选中菜单项及后续业务操作属于该会话，不能在动作中递归 pump 菜单 tracking。原生菜单提供自己的语义，不把其项目复制为 UI 子节点。实际动作能力增删后发送适用的 LayoutChanged 通知，单纯替换能力相同的 handler 不通知，也不重建子项索引。

### 3.1 纯文本正文与选区

`SemanticProperties.multiline` 指明 TextField 接受多个逻辑行，默认 false。空正文和仅含一行的正文也可声明为多行。AppKit 将普通多行编辑区投影为 AXTextArea，受保护与组合框继续使用相应的原生角色。UIA 在 Edit 的 AriaProperties 发布 multiline，AT-SPI 使用单行或多行状态。

Text／TextField 的已知、非受保护 value 同时是其纯文本正文，不再保存另一份可写文档。SemanticProperties.selection: Utf8Selection? 默认 None；Some 只适用于这两个角色，要求 value 已知、protectedText=false，端点必须是正文范围内的 UTF-8 标量边界。None 表示没有公开选区，不能与零长度 caret 混淆；原生单选区协议返回排序后的范围，公共值保留 anchor／cursor 方向。仅选区变化不使节点边界或精确命中失效。

SemanticProperties.cursorAffinity 使用 SemanticTextAffinity（Upstream／Downstream），默认 Downstream，描述已公开选区活动 cursor 的实际排版偏向；没有公开选区时不使用它。光标在软换行的同一字节边界时，Upstream 指向前一行末，Downstream 指向后一行首。仅偏向改变仍保留匹配正文的文本几何，发布选择变化而非正文变化；AppKit 的 InsertionPointLineNumber 同时读取 cursor 与该偏向。按 index／range 的原生请求没有提供偏向时使用 Downstream。

SemanticActions 增加 setSelection(Utf8Selection) 与 replaceText(Utf8Range, String)，返回同一 SemanticActionResult。前者允许只读的可选择文本；后者以及 setValue 遵守 readOnly／enabled。原生选区修改进入实际 handler，文本替换不直接修改投影。AppKit 的 SelectedText 写入使用已发布选区，选区或正文已变化时拒绝旧请求；范围动作携带当前投影的内容版本，不能把旧 UTF-8 偏移应用到新正文。普通全文 setValue 继续使用实际控件的值替换入口。

AppKit 在发布时将合法 UTF-8 选区转换为同一 NSString 的 UTF-16 范围；正文长度、选中文本、SelectedTextRange(s)、StringForRange 和纯文本 AttributedStringForRange 只查询已发布副本。参数越界或切断 surrogate 时返回 unavailable，不夹取到别的文本。SelectedTextRanges setter 只接受一个合法 NSValue 范围，不伪造多选区能力。值和实际选区发布后发送适用 ValueChanged／SelectedTextChanged 通知；anchor／cursor 对调也是实际选择变化，即使排序后的 NSRange 相同，仍发送一次选择通知，以便客户端重新读取活动 cursor 的行号。仅端点方向或偏向变化不发送正文变化，重复发布相同选区不重复通知。getter 不回调 Scoop、不触发布局。

受保护文本不发布正文或选区，不提供上述范围／长度查询；显式可用的全文替换仍可进入真实编辑器。文本位置与单位查询遵守下节的已呈现布局合同；retained range、密码平台专有能力和其他 provider 继续按 S5／S10 接入，不以正文、单选区和当前布局查询声明它们已完成。

#### 3.1.1 已呈现的文本范围与几何

`@Unsafe node.setTextGeometry(geometry: NativeSemanticTextGeometry?)` 在已有、匹配当前正文的 Accepted 节点几何上安装原生只读文本查询；None 撤下旧查询。Some 要求 Text／TextField、已知非受保护 value 和有效节点 geometry。NativeSemanticTextGeometry 位于 `scoop.window.accessibility.native`，包含配套的 opaque context、retain／release、range、lineRange、lineIndex、rangeAtPoint、boxes 与 visibleRange 函数指针；Window 不依赖某个排版库、不重新布局，也不在原生 getter 中进入 Scoop。

range(context, offset, affinity, unit) 使用已发布正文的 UTF-8 字节；affinity 为 0（Upstream）或 1（Downstream），unit 为 0（grapheme）、1（词）、2（视觉行）、3（逻辑行）。lineRange 使用零起始视觉行号，lineIndex 把正文位置映射到实际视觉行；软换行保留 affinity，末尾空行是独立的空范围。rangeAtPoint 使用窗口逻辑坐标，必须经过实际文本区域的变换与精确 clip。boxes 用同步 native visitor(context, x, y, width, height) 返回同一范围的全部窗口逻辑矩形，保留 bidi 及跨行的独立片段；空范围返回实际 caret。visibleRange 返回当前精确裁剪中可见片段的最小覆盖正文范围，离散片段之间的不可见字符可以包含在这一单范围中；完全不可见时不可用。坐标不是像素或屏幕坐标。

NativeSemanticTextRange 保存 start、end、status；其 status 和 boxes 返回值为 1（成功）、0（不可用）、-1（参数无效）、-2（原生失败），只有成功结果中的数据有效。lineIndex 成功返回非负行号，-1／-2／-3 分别表示不可用、参数无效、原生失败。返回范围处于正文的合法 UTF-8 标量边界，返回矩形有限、尺寸非负；visitor 不进入 Scoop、不保留借用参数、不抛出跨 ABI 异常。调用方负责提供与正文及节点几何相符的查询，不把未呈现位置或省略号作为原文位置。

安装时节点保留引用，发布时原生投影独立保留引用；候选、节点、投影分别释放自己的引用。正文／名称使内容版本失效、普通 geometry 替换或清除、退休和关闭时一起撤下文本查询；单独的选区、只读或 enabled 更新不重建排版。原生发布失败保留上一份完整投影。保留的 native 查询只持有实际布局及几何资料，不保活 UI 或 Window；客户端保留旧节点不能读到它们退休前的范围。

AppKit 通过公开的 RangeForIndex、RangeForPosition、BoundsForRange、LineForIndex、RangeForLine、InsertionPointLineNumber 和 VisibleCharacterRange 读取上述投影。平台边界与同一已发布 NSString 做 UTF-8／UTF-16 转换，拒绝越界和切断 surrogate 的参数；BoundsForRange 取所有实际矩形的外接矩形并经当前 NSView／NSWindow 转为屏幕坐标。getter 不请求帧、不抢焦点、不改选区或文档；没有匹配文本几何时相关 selector 不公布，正文读取继续可用。文本几何从不可用变为可用或被撤下时发送布局变化，普通相同帧重放不因此重复通知。

#### 3.1.2 客户端保留的文本范围

`SemanticTextDocument` 只跟踪文档身份、UTF-8 长度、版本及仍被客户端持有的端点，不保存正文或编辑历史。编辑器在实际事务提交时复用偏移映射调整端点；插入点的 Upstream 留在插入内容之前，Downstream 跟随其后；被删除范围内的端点折叠到替换边界，倒置的范围折叠为 caret。折叠后两端共用起点 affinity，后续插入不将 caret 意外扩展为选区。全文替换、文档换绑、受保护用途切换及宿主关闭使旧范围永久失效。文档版本与节点已发布正文不一致时返回暂不可用，不能在旧正文上解释新端点；几何仍要求同一正文的 Accepted 排版。

`AccessibilityNode.textDocument` 绑定这份跟踪状态；不绑定时仍可读取当前快照的文本，但保留范围在正文版本变化后失效。原生范围引用不保活 Window、Scoop 文档或编辑宿主。native 更新偏移的同步回调是只读映射，不重入跟踪对象或调用 UI 行为；窗口关闭撤销提供者，晚释放的范围只清理自己的原生资源。

模型事务可先于下一次逻辑发布推进跟踪文档，期间仍允许发布上一正文版本的场景或其他属性；这属于正常的暂不可用阶段，不能因此使窗口发布失败。原生投影保留该节点对应的文档版本，不把它改为当前版本来解释旧正文。只有正文与文档版本相同时才核对长度并允许保留范围操作；零版本或尚不存在的未来版本仍属无效参数。这个过渡不需要正文历史、强制绘制或输入焦点。

### 3.2 滚动范围与动作

SemanticRole.ScrollBar 表达实际滚动轴，使用 `SemanticProperties.scroll: SemanticScrollRange? = None` 发布范围。SemanticScrollRange 包含 orientation: SemanticOrientation（Horizontal／Vertical）、offset、viewport、content；后三者为有限非负 Double，offset 不超过 `maximum = max(content - viewport, 0)`。单位沿窗口逻辑坐标。None 表示尚无范围；已知零范围与未知不同。该属性仅用于 ScrollBar，不能同时提供字符串 value。范围变化不改变正文版本或节点几何。

SemanticActions 提供 `setScrollPosition((Double) -> SemanticActionResult)`、`scrollBackward()` 和 `scrollForward()`，均为可选回调。位置参数为 0～1 的有限比例；越界请求返回 InvalidArgument，不夹取为另一个请求。增减的实际步长由原控件决定。动作沿相同 owner、enabled、readOnly、模态和退休边界；没有范围或没有实现时不提供对应能力。Completed 表示实际滚动请求已登记；原生位置继续读取已发布范围，不把请求位置提前当作已呈现位置。Reveal 的已接受场景完成条件仍独立保留。

AppKit 的 ScrollBar 使用 AXScrollBar、Orientation、0～1 的 Value／MinValue／MaxValue 及 Increment／Decrement；零范围的值及上限为 0。未发布范围时不提供数值、上下限或滚动动作。NSNumber 写入沿真实 setScrollPosition，不能借字符串编辑接口改变滚动。容器的 HorizontalScrollBar／VerticalScrollBar 按阅读顺序引用其直接语义子项中对应轴的首个可访问节点；内置滚动容器每轴提供一个节点。自绘滚动条可在视觉隐藏时继续保留逻辑轴和动作，缺少当前场景登记时没有可点击几何；不得用容器矩形代替滚动条的位置。

滚动位置／范围变化发送对应值通知，方向或能力变化发送布局通知；原生 getter 仅读取投影，不推动布局、焦点、滚动或条的显示动画。零范围、不适用轴、disabled／hidden、重新绑定、关闭和客户端保留旧引用均沿实际状态处理。

### 3.3 显露动作

`SemanticActions.reveal: (() -> SemanticActionResult)? = None` 请求通过实际滚动路径显露本节点，不改变键盘焦点或文本选区。它不要求 ScrollBar 角色或可写文本；enabled、hidden、ignored、模态、owner 与退休检查仍适用。Completed 表示匹配当前内容的 Accepted 场景中目标已有非空可见区域，且所需的逐层滚动已呈现；只登记滚动请求或仍在等待后续 Accepted 帧时返回 Busy。没有布局、无法通过实际滚动路径显露或窗口不能呈现时，不返回 Completed。

AppKit 使用公开的 `NSAccessibilityCustomAction` 提供“Scroll to Visible”动作，BOOL handler 仍只把 Completed 映射为 YES；读取动作列表不调用业务代码。当前 SDK 虽从 macOS 26 提供 NSAccessibilityScrollToVisibleAction 常量，但未提供对应的公开 BOOL protocol 方法，因此不声明同名的私有 selector。动作对象使用 weak target，保留旧动作不能保活 Window；实际调用重新检查当前能力和节点寿命。滚动与最终几何继续发送原有发布通知，不增加轮询或嵌套事件循环。其他 provider 按其实际公开合同映射同一 reveal 回调。

### 3.4 虚拟集合的数据查询

`SemanticRole.List` 是逻辑集合容器。`SemanticItemKey` 以 Number(Long)／Text(String) 表达集合内稳定 key，区分数字和文本。`SemanticCollectionSource` 提供同一不可变快照的 count、keyAt、indexOfKey、describe 与 search；不从 View 构造函数推导元数据。`SemanticCollectionSnapshot(source)` 只保留描述源和已校验的实际 count，不枚举数据或创建每项对象。

`SemanticItemProperties` 提供 name、description、automationId、value、enabled、selected、expanded、focusable 与可选 hierarchyLevel；另提供 parentKey、siblings、checked 与 busy。可选文本的 None 是未知，Some("") 是已知空文本，selected／expanded／checked 的 None 表示没有该状态。hierarchyLevel 是从零开始的非负层级深度，None 表示未声明层级；不从条目位置推导深度。parentKey 是同一集合中的已知逻辑父 key，None 表示没有声明父项，不能由当前挂载或屏幕位置猜测。siblings 使用 `SemanticItemPosition(index, count)` 表达当前同级中的零基位置与数量，要求 `0 <= index < count`，不以全展开集合的位置冒充；未提供时继续使用普通集合位置。checked 为 Off／On／Mixed，busy 默认 false。focusable 缺省 false；只有数据源明确知道该项有真实聚焦目标时设为 true，不从名称或角色猜测。`describe(index)` 的 None 表示该项描述当前不可用。实际 count 不包含尚未加载、尚无 key 的远端数据。

Tree 的数据顺序为当前可阅读展开行的先序，parentKey 必须指向本快照中位于当前项之前的父项。单项描述验证父 key 的普通引用及顺序即可，不为查询一项重新扫描或验证整棵树。读取已知父项、深度、同级位置、checked／busy 或查询多列单元格不触发加载或 View 构造。AppKit 沿 Outline 的 disclosedByRow／disclosedRows 显示父子关系，逻辑条目仍只拥有一份原生代理；披露、勾选与几何变化不会另建吸顶副本。UIA／AT-SPI 的惰性树项导航按同一 parentKey 返回父项和直接子项；多列行先列出单元格，再列出展开的直接子项。显式枚举子项允许读取对应展开范围的描述，不创建整棵树的代理。

快照的 itemAt／itemForKey 返回 Found(SemanticItem)／NotFound／Unavailable；`find(SemanticItemQuery, after)` 另可返回 Unsupported。SemanticItem 含 key、从零开始的 index、count、properties、缺省为 true 的 hasCells 与缺省为 1 的 span；普通条目的 span 为 1，表格单元格用它表达连续列跨度。Name／AutomationId 查找仅在 SemanticItemSearch 对应能力明确开启时执行，使用精确字符串匹配和当前快照的轻量描述；after 是排除的稳定 key，不是旧 index。未知 after、途中不可用的描述或被查询的未知属性返回 Unavailable，不能假报 NotFound。明确的全量查询可以有线性数据成本，但不执行 build。

`AccessibilityNode.collection` 暂存一个可选快照，用于 List 或 Table；publish 成功后才替换原生查询使用的快照。`AccessibilityNode.collectionItem` 将一个具体 Group 子节点绑定到其 List 父节点中的 key，用于已挂载行的几何与真实子控件；回收这个 Group 不删除逻辑条目；Group 更换父节点时清除原集合关联。原生集合对象按需产生并缓存轻量条目代理，不预建 count 个对象或完整子 ID 数组。发布数据变化时只复核已经产生的代理 key，重排保留对象，删除／过滤退休旧对象，后来重用相同 key 也不复活退休对象。

AppKit 的 List 提供 rows、visibleRows、按 index 取 row 和逻辑子项；行的元数据通过已发布快照的只读 owner 查询获得。普通原生查询在非 owner、重入更新或关闭时返回适用的暂不可用结果；内部发布对保留 key 的复核只读本次候选快照，不进入 UI 更新。描述失败交回现有 owner 错误处理。缺失具体实例时不报告估算坐标和虚假的子控件；已实例化行借用其 Accepted 几何和实际挂载语义子树。查询本身不 build、滚动或聚焦。

跨进程 AX 引用不会替应用保留提供方的 Objective-C 对象。AppKit 按需保留已经导出的行、单元格和列代理，直至对应逻辑实体退休或 provider 关闭，不能在返回属性数组后立即释放。代理对集合、行和树的反向引用不形成所有权环；退休时撤销缓存并发布适用的销毁通知。未查询项目仍不创建代理，显式枚举全部数据可以产生相应数量的轻量代理，但不能构造全部 View。

集合选择由同一发布快照的 `SemanticCollectionSelection(keys, multiple, required)` 描述。keys 只列属于该快照的已选稳定 key；原生查询不为取得选中项扫描整个集合。`SemanticItemProperties.selectable` 缺省 true，允许总计、说明等条目明确设为 false，同时保留其 enabled、focusable 和阅读能力。false 优先于 selected 和集合选择快照：该条目不发布 selected 状态，不出现在原生选中项列表中，选择／取消选择返回 Unsupported 且不调用业务回调。选择列表发布只检查已选 key 的描述，描述暂不可用的项也不发布；不会为检查可选性枚举整个集合或创建 View。`setSelected(key, value)` 经原业务选择入口确认采纳结果；没有选择模型时不声明 Selection 能力。AppKit 的选择 setter 只在该行可用、可选择且有选择模型和回调时公布。

`SemanticGrid` 给 Table／Tree 集合提供稳定列 key、标题及可选实际表头引用，并通过 `(rowKey, columnIndex)` 查询轻量单元格描述。行身份仍属于集合，单元格身份由行 key 和列 key 组成；索引只表示当前坐标。查询不构造 cell View，未知描述返回 Unavailable；列或行删除永久退休对应旧代理。已挂载单元格用 `SemanticTableCell` 声明真实行列位置与跨度，`SemanticTableSize` 描述静态表格尺寸。窗口层不缓存第二份业务表格。Table、Tree 和 List 共用集合的发布、按需代理及明确操作边界。

`SemanticGrid.hasCells = Some { rowKey -> Boolean }` 可明确表示详情、说明等结构行不属于列空间；省略时所有行均有单元格。该回调只读当前行 key，不读取各列描述、不创建 View。行查询返回的 `SemanticItem.hasCells` 保存同一快照中的结果；false 时 `cellForKey` 返回 NotFound，单元格动作不可用且不调用 span／describe 或业务动作。原生表格保留该行的正常身份、层级及行操作，子项来自已挂载的实际语义子树；离屏时没有伪造的单元格或子控件。它与 describe 暂不可用的 Unavailable 含义不同。发布中从有格改为无格永久退休已建立的旧单元格代理，随后恢复有格使用新代理；不枚举行列矩阵。

跨列集合可传 `SemanticGrid.span = Some { rowKey, columnIndex -> SemanticGridSpan(column, columnSpan) }`。缺省每列独立；回调给出包含被查询列的锚点索引和正跨度，同一跨度内所有列须返回同一范围。范围不能超出当前 columns，查询时检查负值、零跨度、溢出、当前边界及包含关系，不枚举行列矩阵。`cellForKey` 返回锚点的 key、index、span 和 describe 元数据；describe 只读取锚点。行列重排或跨列关系变化通过新快照发布。此能力只跨列，不合并不同行。

AppKit 按被查询列解析到同一锚点代理，行子项只列出一次锚点；离屏 ColumnIndexRange 也使用声明的真实跨度。被覆盖的旧独立单元格退休，取消跨列后不会复活旧代理。挂载内容仍以锚点 columnKey 对应唯一的真实单元格，几何与关联表头沿实际挂载读取；未挂载时没有估算 Frame。未声明 span 的集合沿原有按需单列查询路径，不为枚举列代理额外读取每格描述。

### 3.5 虚拟条目的明确操作

`AccessibilityNode.collectionActions` 接受 `SemanticCollectionActions(realize, reveal, focus, setSelected)`，用于 List 或 Table。前两项是可选 `(SemanticItemKey) -> SemanticActionResult` 回调，focus 为可选 `(SemanticItemKey, Boolean) -> SemanticActionResult` 回调。能力随 collection 一同发布；动作重新按当前有效快照解析 key，经过相同 owner、模态、祖先 enabled／hidden 和退休边界。描述暂不可用或 key 已删除时返回 Unavailable，disabled 返回 Rejected；没有相应回调或条目未声明 focusable 时返回 Unsupported。

可选 `SemanticCollectionActions.setExpanded` 接收 `(SemanticItemKey, Boolean)`，复用同一动作边界；只有当前条目明确声明 expanded 时可执行。它调用应用的实际展开模型，不隐式挂载、滚动或聚焦条目。AppKit 以公开 `setAccessibilityExpanded:` 映射展开／收起，以 `accessibilityDisclosureLevel` 发布声明的深度；setter 只在节点可用、已声明 expanded 且提供回调时公布。回调拒绝时保留原状态，旧代理不能操作后来重建的同 key 条目。

`SemanticCollectionActions.setChecked` 同样接收 `(SemanticItemKey, Boolean)`，只有声明 checked 的可用条目和真实回调可执行；Mixed 的 Toggle 请求为 On。原生查询读取当前快照，动作经原模型接受后更新，不因数据项尚未挂载而创建 Checkbox 或夺取焦点。AppKit 提供 Check／Uncheck 自定义动作及 checked 值，UIA／AT-SPI 使用对应切换能力；没有 checked 状态的普通行保持原 value。busy 不自动拒绝选择、折叠或业务允许的勾选，禁用仍由 enabled 与具体回调决定。

表格可另传 `SemanticCollectionActions.grid = Some(SemanticGridActions(...))`：realize／reveal 回调接收行 key、列 key，focus 再接收 Boolean。每次调用同时检查当前行和列，单元格禁用或未声明 focusable 时沿相同结果规则处理。单元格动作不会降级为同一行的动作；列重排保留 key，删除后复用同 key 不复活旧代理。Realize 的 Completed 要求目标单元格及其语义子树已经发布，Reveal 和 Focus 仍分别等待真实可见几何与活动焦点。

跨列单元格的动作先在当前快照解析锚点，再把锚点列 key 交给上述回调；对被覆盖列的查询和操作不能构造第二个控件或改变被覆盖列的独立业务值。

Realize 的 Completed 表示实际行及其当前语义子树已挂载并发布，原生同步调用返回之前不得回收；它不承诺几何、滚动或焦点。Reveal 只有真实 Accepted 场景中目标已具有有效可见区域时才 Completed，准备或等待呈现期间为 Busy。Focus 选择该项实际支持的焦点目标，以实际活动键盘焦点完成；清除请求仍指向该 key 的真实目标，请求之间的先后与取消遵循原焦点控制器。窗口不在前台时，不把逻辑焦点或排队请求当作实际活动焦点。

AppKit 用公开 NSAccessibilityCustomAction 提供“Realize Item”和“Scroll to Visible”，分别对应实际已登记能力，BOOL 只把 Completed 映射为 YES。聚焦使用公开 setAccessibilityFocused:；条目 getter 依据真实挂载子树中已发布的实际焦点，不借读取启动任何操作。旧动作、旧代理以及删除后重新使用同 key 均不能越过退休边界；动作对象不保活实际行或 Window。

表格单元格的 AppKit 自定义动作使用“Realize Cell”和“Scroll to Visible”，只公布 grid 回调实际提供的能力。isAccessibilityFocused 读取该单元格实际子树中的活动焦点；setAccessibilityFocused: 指向同一行列身份，不能返回或改写该行其他单元格的焦点。

## 4. AppKit 投影与验证

使用 NSAccessibilityElement 与正式 NSAccessibility protocol，连接到现有内容 NSView。原生 Window、菜单和对话框继续提供自己的树；内容 view 不重复导出同一组节点。角色、名称、说明、identifier、值、enabled、checked、protected、焦点与实际动作由已发布节点决定。窗口逻辑矩形通过 AppKit 的 view／window 转换为屏幕坐标，窗口移动无需重写所有节点几何。

AppKit 以 accessibilityLabel／Title 发布解析后的名称；只有一个有效标签且没有显式名称覆盖时，同时提供 TitleUIElement 及对应 ServesAsTitleForUIElements。多个标签保留合成名称，不伪造单一标题对象。controls 映射 LinkedUIElements，说明与错误的非空文本按顺序进入 Help。原生关系只返回可导航的活动对象；hidden 标签可提供文字但不通过对象关系重新暴露。关系或相关文字变化随同一 publish 发布并发送适用的名称／布局通知，客户端保留旧对象后查询不跳转到新身份。

本批定向验证必须覆盖实际原生查询、精确命中、动作结果、受控拒绝、发布顺序、重入拒绝、窗口／provider 关闭和客户端保留旧对象；外部 AX 客户端与 VoiceOver 的实际结果分别记录，直接调用 NSAccessibility 方法不冒充跨进程验收。完整文本范围、滚动动作、复杂控件关系、激活点、虚拟集合、三平台与真实工具任务仍执行 S2～S11 的相应验证。

忽略节点的 AppKit Children／Parent 使用同一份已发布阅读关系折叠包装，不能丢掉其独立交互后代。原生端仅在父子／阅读顺序变化时更新子项索引，名称、几何和透明度更新不重建该索引。忽略与恢复分别发送适用结构通知，不发送退休通知。

AppKit 的 accessibilityHitTest 从屏幕点经当前 NSWindow／NSView 转为窗口逻辑坐标，按已发布的场景顺序调用 native contains；内容 view 与 provider 根使用同一入口。节点内查询限于该语义子树，根查询覆盖全部内容；窗口内没有命中子项时返回内容根，窗口外或失活时无结果。更新窗口位置或 DPI 使用现有坐标换算，查询不请求帧。命中索引只在快照有无、节点增删或场景顺序变化时更新；名称、错误和普通 handler 更新不重排索引。`AccessibilityNode.activationPoint` 可登记经过真实输入路由确认的窗口逻辑坐标；它必须位于当前 visibleBounds 和 native hit 快照内，改变或撤下几何时同步清空。调用方负责当前 enabled、模态边界和前景遮挡的输入资格。平台只读取已发布的点并转换屏幕坐标，不在 getter 中尝试点击或布局；未知时不声明 ActivationPoint／ClickablePoint。UI 在正常发布时对已接受的实际 InputArea 做有界候选点命中，确认失败则保持 None。

## 平台原生投影与线程边界

`SemanticProperties.selectionMode: SemanticSelectionMode?` 为实际挂载子项的选择容器声明 Single／Multiple；required 表示是否必须保留选择。它只用于 Group／List／RadioGroup／TabGroup／Table，不能与同一节点的虚拟集合选择快照同时声明。没有声明的普通列表不自动变成选择容器，平台不能从 role 猜测是否必选或多选。静态容器只遍历已挂载的选择后代，嵌套选择容器具有独立边界；虚拟集合仍从已发布的 selected keys 查询，不创建条目 View。

Windows 使用 HWND 的 WM_GETOBJECT 返回 UIA Simple／Fragment／FragmentRoot。静态事实在原生不可变快照内读取；Action、按需集合描述和显式集合操作通过窗口 owner 消息调用原入口，不在 COM 线程调用 Scoop。同步等待不启动私有事件循环；owner 重入、发布中和关闭分别按既有 Busy／Unavailable 拒绝。COM 对象仅保留原生树和文档范围，窗口关闭使后续调用返回元素不可用。直接宿于 HWND 的根在 GetRuntimeId 返回成功及空数组指针，由 UIA host 提供标识；其余元素使用 UiaAppendRuntimeId、窗口投影实例与稳定节点／按需代理身份，删除后复用 key 不复活旧对象。

Windows 的屏幕坐标使用当前 HWND 客户区原点和 DPI 转换；UIA ClickablePoint 只转换已验证的 activationPoint。TextRange 使用 UTF-8 文档端点，UIA 与 AT-SPI 的文本参数按各自单位转换；普通 retained range 在同文档编辑时更新端点，替换或关闭后不可用。范围查询只读同一发布正文和匹配的原生布局；没有相应布局时，不假造字符边界或位置。

Linux 的内容投影使用 GIO 的 AT-SPI D-Bus 协议，在既有 GMainContext 处理请求，未变化时不轮询。GTK 4.18 的 GtkAccessibleText 不支持 SetSelection，Component 不支持 GrabFocus；这些能力必须通过真实输入／编辑入口接入，不能用可见 role 代替完整 provider。

Linux 使用 GTK 4.14 起公开的 gtk_at_spi_socket_new 和 gtk_accessible_set_accessible_parent，把 GDBus 内容根作为原 GTK 内容容器的 accessible child。根处理 Socket.Embedded 记录 GTK 提供的父引用；Application 引用沿 GTK 根返回，菜单／文件面板继续使用其原 GTK provider。该衔接不调用 GTK 私有 API，也不注册第二个表示相同窗口的应用根。

AT-SPI 的 D-Bus 对象路径没有逐对象 Release。Linux 只保留已请求或已通知的导出对象，按客户端总线退出释放相关引用，窗口关闭时撤销导出并关闭该窗口的协议连接。逻辑集合更新后的缓存通知在既有 GMainContext 中合并处理，只刷新已导出的条目，不在 publish 的回调中查询业务数据。名称、状态或索引变化不退休稳定 key，删除则发送 defunct 和 RemoveAccessible。

桌面未提供会话／accessibility bus 或明确设置 GTK_A11Y=none 时，不导出 AT-SPI 内容，普通窗口仍可运行；其他连接失败照常报告。X11 屏幕原点使用实际原生坐标，Wayland 无法提供全局屏幕原点时明确返回 NotSupported，窗口和父节点坐标仍可查询。D-Bus string 无法表达内含 NUL 的文本，此类查询返回 NotSupported；无法表达的文本通知不截断或伪造正文，也不导致整个应用退出。

文本动作 `revealText(range, scroll)` 显露指定 UTF-8 范围而不改变选区或焦点。SemanticTextScroll(horizontal, vertical) 使用 Start／Center／End／Nearest 描述最近滚动视口的对齐，其外层视口只做必要显露。动作使用同一内容版本校验、真实已呈现排版与滚动路径；等待新 Accepted 几何时返回 Busy，期间文档变化、隐藏、输入边界或新滚动意图使请求取消。正文版本独立于名称和说明版本；仅名称变化不会使未跟踪文档的文本范围失效。

`AccessibilityNode.textChildren` 用 `SemanticTextChild(node, range)` 将当前公开正文中的 UTF-8 范围关联到实际语义后代，用于内联控件与文本内动作。每个后代只能登记一次，范围必须属于当前正文；正文改变时清空旧映射，后代退休时移除对应条目。查询只沿已发布映射，不从控件矩形猜测字符位置。Windows TextPattern.RangeFromChild／TextRange.GetChildren 和 Linux Hypertext 使用这份映射；正文、独立交互后代和替代文本保持各自真实身份。
