# Window 无障碍接口规范

状态：S0／S2／S5 实施中。本文规定已有窗口内容的低层语义节点、AppKit 投影、纯文本范围与同步动作边界；UI 资源、内容合并与场景适配见 [UI 无障碍接口规范](SCOOP-UI-ACCESSIBILITY-SPEC.md)。内置控件、复杂控件关系、完整文本几何、虚拟集合和 Windows／Linux provider 继续按[完整实施计划](../ui/interaction-polish/PLAN.md)推进，不能以本批接口代替这些完成条件。实际构建／运行结果见[实施记录](../ui/interaction-polish/PROGRESS.md)。

## 1. 包、身份与寿命

`scoop.window.accessibility` 属于 Window Cone，不依赖 Surface、UI 或控件。SemanticRole、SemanticProperties、SemanticActions、SemanticActionResult、SemanticGeometry 与 SemanticNodeId 为公共值；AccessibilityProvider 和 AccessibilityNode 为 owner 线程资源。首批 `Window.createAccessibilityProvider()` 在 Darwin 暴露并实际连接 AppKit；其他平台沿同一接口接入自己的原生实现，不放置空 provider 或成功占位返回。

每个 Window 同时至多有一个内容 provider。createAccessibilityProvider 要求 Open；重复附着报 IllegalStateException。createNode(role, parent = None) 创建节点并返回 AccessibilityNode，parent=None 表示内容根下的直接子项。角色在该节点生命期固定；更换角色需退休并重新创建。节点 id 的窗口所属标识独立于原生地址，序号在窗口生命期递增、不复用；关闭 provider 后重新附着仍延续序号。不同应用运行产生的 WindowId 数字相同也不使旧语义 id 相等。

节点的 parent 与 order 表达语义阅读顺序，同序号按创建次序排列，不依据屏幕坐标排序。reparent(parent, order = 0) 要求父节点在同一活动 provider，拒绝自身或后代为父；失败不改变原关系。close 退休整个节点子树，随后修改节点或读取 geometry 报 IllegalStateException；id、role 和 isClosed 仍可读取，重复 close 无操作。provider.close 退休全部节点；Window 进入 Closing 时立即关闭 provider，不等待 drawable 或业务回调。已退休的原生对象可以由客户端继续持有，但不再读取或保活 Window／UI，不能执行动作。

## 2. 属性与发布

首批角色为 Group、Text、Button、Checkbox、TextField、ScrollArea、ScrollBar。SemanticProperties 保存 name、description、error、automationId、value、enabled、readOnly、protectedText、checked、hidden 与 ignored；名称、说明、错误、标识和值采用可选字符串，已知空字符串与未知值不同。checked 为可选 Off／On／Mixed。只有实际文本用途才设置 protectedText；受保护时不允许通过 value 发布明文，不因屏幕暂时显示正文而解除保护。hidden 排除整个节点子树但保留身份，恢复后使用原身份；ignored 仅忽略当前节点，保留后代并把它们接到最近未忽略的语义祖先；hidden 优先排除整个子树。忽略不退休身份、不改变实际键盘焦点，旧引用不能执行被忽略节点的动作；后代继续遵守祖先的 hidden／enabled。update 在改变原状态前检查这些规则。

SemanticProperties.required 为 Boolean，默认 false，描述表单是否要求填写或选择；它不执行校验、不改变 enabled／readOnly 或文本内容。AppKit 通过公开的 NSAccessibilityRequiredAttribute／isAccessibilityRequired 读取该事实，客户端不能改写它。required 变化不改变节点身份、正文版本或几何，发布后发送适用的属性布局通知；隐藏／退休后的 selector 与其他只读属性遵守同一可访问性边界。

选择控件还提供 Switch、ToggleButton、RadioButton 与 RadioGroup。前两者在 AppKit 使用 Button 角色及公开的 Switch／Toggle subrole，checked 的 Off／On 为实际布尔值，Press 调用 toggle；Mixed 仅用于 Checkbox，其他选择角色拒绝该值；RadioButton 使用原生同名角色、Off／On 值及 invoke 选择动作，不能用反转操作取消单选。RadioGroup 使用原生 RadioGroup 角色，其条目保持普通父子／阅读顺序，组本身不凭角色获得选择或焦点动作。checked 变化发布值通知；查询不改变选择或键盘焦点。

update(properties, actions = SemanticActions(), relations = SemanticRelations()) 替换属性、最新动作实现及关系。值／名称变化使旧几何失效；新的匹配布局可通过 geometry 再发布。只读 contentRevision 随影响名称／值的变更递增，供场景比较捕获时的内容版本；关系名称的依赖变化同样使旧候选失效。SemanticGeometry 保存窗口内容逻辑坐标中的 bounds 和可选 visibleBounds；后者必须在 bounds 内，且两者坐标有限、尺寸非负。None 表示几何不可用；visibleBounds=None 表示已经有合法布局但被自身裁剪，不表示节点退休或程序化隐藏。宽度或高度为零的可见矩形不进入 visible children，仍保留合法布局与语义身份。矩形仅描述外接区域，不能据矩形中心推断任意自绘形状的可点击点；精确命中使用本节的独立 native 快照合同。

SemanticRelations 保存有序的 labelledBy、describedBy、errorMessages、controls 节点 ID 数组，默认均为空，以有序内容判断相等。每次替换在修改前检查同一活动 provider 的目标、同一数组无重复和非自身引用；controls 等关系不套用父子树的无环规则。有效名称的循环依赖在 publish 解析受影响名称时拒绝，保留上次原生投影和待发布变更，调用方可修正后重试。显式名称已截断的依赖不算有效名称环；不会为了未采用的标签重新遍历整张关系图。关系不改变父子归属或焦点，不保活已退休的目标；目标关闭时自动清除所有指向它的关系，普通重排保持引用身份。node.relations 只读返回清理后的当前关系。

node.contentLabels 是单独保存的有序内容标签 ID 数组，默认空，采用与关系相同的归属／重复／自身检查与退休清理。它只提供名称的第三优先来源：显式 name → labelledBy 的已知文字 → contentLabels 的已知文字；已知空串同样截断后续回退。普通 update 不清空 contentLabels。底层不会据内容引用自行隐藏节点，UI 等实际调用方负责声明合并与 ignored；内容引用、普通标签及其混合有效依赖共享同一次名称解析与循环检查。

各类标签按数组顺序连接已知的目标名称；被引用的静态 Text 没有名称时可用其非受保护 value 作为标签文字，但不因此为该 Text 自身重复发布 name。description 和 error 分别采用显式属性优先，否则从对应关系取得目标文字。未知条目跳过，已知空文字仍与全部未知区分；连接非空文字时使用单个空格。显式关系可以使用仍活动但 hidden／离屏的标签文字，不把被引用节点重新放回导航树，也不公开受保护 value。目标的名称或静态文字变化只更新实际依赖者，发布后原生 getter 直接读取解析结果，不进入应用代码或重复遍历名称图。

创建、修改和重排先修改 managed 状态，publish() 在正常 owner 更新完成后原子发布本批差量；未变节点不重复序列化和验证，单纯替换相同能力的 handler 不更新 native 属性。原生 getter 只读取已发布投影，不调用 Scoop handler 或布局。原生准备失败保留上一份投影，差量仍待处理；已经原子提交的结果不因随后平台通知失败回滚，错误按普通 native 操作失败报告。退休立即禁止旧对象动作，结构变更在 publish 时统一通知；新节点发布前不进入原生树。几何合法性在设置 geometry 的边界检查，失败保留原值。

focused 属性保存当前实际获得键盘焦点的活动节点或 None。它不代表辅助技术阅读位置，也不因为 Focus 请求已排队就改变。publish 先发布属性、结构和实际焦点，再发送适用的结构／值／焦点通知；无差量时不通知、不请求帧。几何仍由调用方使用匹配的 Accepted 场景提供，provider 不替调用方启动绘制。

node.geometry 保存值型 bounds／visibleBounds；只赋矩形不提供精确命中。原生 renderer 可使用 `@Unsafe node.setGeometry(geometry: SemanticGeometry, hitTest: NativeSemanticHitTest, order: Long = 0L)` 同时登记匹配几何、命中快照和实际场景顺序。NativeSemanticHitTest 位于 `scoop.window.accessibility.native`，是 context、retain、release、contains 的 CLayout 值；后三者分别为 FunPtr<(Ptr<Unit>) -> Unit>、同签名释放函数和 FunPtr<(Ptr<Unit>, Double, Double) -> Int>。调用方提供成套的原生只读实现，contains 接收窗口逻辑坐标并返回 1／0，函数不回调 Scoop、进入 UI 状态或抛出跨 ABI 异常。

setGeometry 在验证几何后为节点持有一个引用；原生投影在发布时独立保留自己的引用，调用方可在 setGeometry 返回后释放借用。替换、名称／值失效、退休和关闭释放对应引用，准备失败时旧原生投影仍有自己的有效快照。普通 geometry 赋值明确替换为仅有矩形的几何并清除旧命中；None 同时清除位置与命中。Window 不检查 Path 或重建 clip 引擎，第三方 renderer 可提供自己的实际原生命中实现。

命中顺序使用场景 order，值较大者在上，同值按节点创建顺序确定；它与阅读顺序分别保存。命中忽略 hidden 子树及已退休对象，ignored 自身命中映射到最近未忽略祖先，独立后代仍可成为结果。disabled 对象仍可被辅助技术定位和读取，动作继续按 enabled 拒绝。缺少有效命中快照时不根据 bounds 猜测结果，也不把矩形中心声明为有效激活点。

## 3. 动作与同步边界

SemanticActions 按实际实现提供 invoke、toggle、setValue(String)、focus(Boolean)、showContextMenu() 及 §3.1～§3.3 的文本、滚动与显露动作；focus 的 true 请求聚焦，false 请求解除该目标焦点。每个回调返回 SemanticActionResult：Completed、InvalidArgument、Unsupported、Unavailable、Rejected 或 Busy。缺少回调时不公布对应原生能力；enabled=false 拒绝交互，readOnly=true 拒绝值写入及切换，但不禁止有实际内容的菜单。只有实际业务已执行才返回 Completed；Binding 拒绝不能提前改写投影，异步业务结束也不属于 Invoke 的完成条件。

AppKit getter 不进入 managed 代码。同步动作使用创建 provider 时登记的 Reusable ForeignCallback，沿已有 Context 捕获规则恢复创建上下文，包括 asyncio 集成的 scope。它只在 owner、应用未退出、窗口 Open 且输入未被模态阻止、没有进行中的 Window 业务／watch 回调或另一动作时执行。重入时返回 Busy，不等待自己的队列、不 pump、不布局；UI 适配另检查宿主派发、scope 回调、render／布局和实际输入边界，见 UI 无障碍规范。普通 handler 仍沿既有异步派发路径。

动作先保存回调与所需值，再执行；回调可以退休自身、关闭 provider 或 Window。执行后仅按资源仍存活的情况发布实际结果，不继续使用旧节点。异常在 managed 边界捕获，经正常应用错误与退出路径报告，不跨 C／Objective-C ABI；UI 后续将自身动作包在既有 Surface／根错误边界中。动作中可能打开的原生菜单／面板继续使用已有延迟 native operation 通路。

NSAccessibility 的 performPress 等 BOOL 动作仅在 Completed 时返回 YES。setAccessibilityValue 与 setAccessibilityFocused 为无返回值 setter：只在实际回调中改变业务状态，失败时保持旧值；调用方必须读取结果，不能把 setter 返回解释为写入成功。原生注册在释放 callback 前撤销；动作中关闭时，callback ownership 经现有 owner control 队列在调用退栈后释放。客户端晚到查询／动作不会访问已释放 context。

AppKit 的跨进程 AX 桥由系统实现；`AXUIElementPerformAction` 的 kAXErrorSuccess 可能只确认调用已送达，即使原 BOOL 动作拒绝也返回该值，标准禁用 NSButton 同样如此。库不把这一系统状态再解释为 SemanticActionResult.Completed；客户端结合当前 enabled、实际值、焦点或对象退休状态判断业务结果，校验中也分别记录跨进程调用与真实 handler 的结果。聚焦后通过应用的 AXFocusedUIElement／AXFocusedWindow 及目标自身的 AXFocused 核对，不要求窗口自身 AXFocused 与后代节点相等。

showContextMenu 映射 AppKit accessibilityPerformShowMenu，经相同 owner／生命周期／输入边界执行真实菜单请求，不合成右键或建立第二份菜单模型。Completed 表示调用方已经创建绑定该目标的菜单会话；选中菜单项及后续业务操作属于该会话，不能在动作中递归 pump 菜单 tracking。原生菜单提供自己的语义，不把其项目复制为 UI 子节点。实际动作能力增删后发送适用的 LayoutChanged 通知，单纯替换能力相同的 handler 不通知，也不重建子项索引。

### 3.1 纯文本正文与选区

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

### 3.2 滚动范围与动作

SemanticRole.ScrollBar 表达实际滚动轴，使用 `SemanticProperties.scroll: SemanticScrollRange? = None` 发布范围。SemanticScrollRange 包含 orientation: SemanticOrientation（Horizontal／Vertical）、offset、viewport、content；后三者为有限非负 Double，offset 不超过 `maximum = max(content - viewport, 0)`。单位沿窗口逻辑坐标。None 表示尚无范围；已知零范围与未知不同。该属性仅用于 ScrollBar，不能同时提供字符串 value。范围变化不改变正文版本或节点几何。

SemanticActions 提供 `setScrollPosition((Double) -> SemanticActionResult)`、`scrollBackward()` 和 `scrollForward()`，均为可选回调。位置参数为 0～1 的有限比例；越界请求返回 InvalidArgument，不夹取为另一个请求。增减的实际步长由原控件决定。动作沿相同 owner、enabled、readOnly、模态和退休边界；没有范围或没有实现时不提供对应能力。Completed 表示实际滚动请求已登记；原生位置继续读取已发布范围，不把请求位置提前当作已呈现位置。Reveal 的已接受场景完成条件仍独立保留。

AppKit 的 ScrollBar 使用 AXScrollBar、Orientation、0～1 的 Value／MinValue／MaxValue 及 Increment／Decrement；零范围的值及上限为 0。NSNumber 写入沿真实 setScrollPosition，不能借字符串编辑接口改变滚动。容器的 HorizontalScrollBar／VerticalScrollBar 按阅读顺序引用其直接语义子项中对应轴的首个可访问节点；内置滚动容器每轴提供一个节点。自绘滚动条可在视觉隐藏时继续保留逻辑轴和动作，缺少当前场景登记时没有可点击几何；不得用容器矩形代替滚动条的位置。

滚动位置／范围变化发送对应值通知，方向或能力变化发送布局通知；原生 getter 仅读取投影，不推动布局、焦点、滚动或条的显示动画。零范围、不适用轴、disabled／hidden、重新绑定、关闭和客户端保留旧引用均沿实际状态处理。

### 3.3 显露动作

`SemanticActions.reveal: (() -> SemanticActionResult)? = None` 请求通过实际滚动路径显露本节点，不改变键盘焦点或文本选区。它不要求 ScrollBar 角色或可写文本；enabled、hidden、ignored、模态、owner 与退休检查仍适用。Completed 表示匹配当前内容的 Accepted 场景中目标已有非空可见区域，且所需的逐层滚动已呈现；只登记滚动请求或仍在等待后续 Accepted 帧时返回 Busy。没有布局、无法通过实际滚动路径显露或窗口不能呈现时，不返回 Completed。

AppKit 使用公开的 `NSAccessibilityCustomAction` 提供“Scroll to Visible”动作，BOOL handler 仍只把 Completed 映射为 YES；读取动作列表不调用业务代码。当前 SDK 虽从 macOS 26 提供 NSAccessibilityScrollToVisibleAction 常量，但未提供对应的公开 BOOL protocol 方法，因此不声明同名的私有 selector。动作对象使用 weak target，保留旧动作不能保活 Window；实际调用重新检查当前能力和节点寿命。滚动与最终几何继续发送原有发布通知，不增加轮询或嵌套事件循环。其他 provider 按其实际公开合同映射同一 reveal 回调。

## 4. AppKit 投影与验证

使用 NSAccessibilityElement 与正式 NSAccessibility protocol，连接到现有内容 NSView。原生 Window、菜单和对话框继续提供自己的树；内容 view 不重复导出同一组节点。角色、名称、说明、identifier、值、enabled、checked、protected、焦点与实际动作由已发布节点决定。窗口逻辑矩形通过 AppKit 的 view／window 转换为屏幕坐标，窗口移动无需重写所有节点几何。

AppKit 以 accessibilityLabel／Title 发布解析后的名称；只有一个有效标签且没有显式名称覆盖时，同时提供 TitleUIElement 及对应 ServesAsTitleForUIElements。多个标签保留合成名称，不伪造单一标题对象。controls 映射 LinkedUIElements，说明与错误的非空文本按顺序进入 Help。原生关系只返回可导航的活动对象；hidden 标签可提供文字但不通过对象关系重新暴露。关系或相关文字变化随同一 publish 发布并发送适用的名称／布局通知，客户端保留旧对象后查询不跳转到新身份。

本批定向验证必须覆盖实际原生查询、精确命中、动作结果、受控拒绝、发布顺序、重入拒绝、窗口／provider 关闭和客户端保留旧对象；外部 AX 客户端与 VoiceOver 的实际结果分别记录，直接调用 NSAccessibility 方法不冒充跨进程验收。完整文本范围、滚动动作、复杂控件关系、激活点、虚拟集合、三平台与真实工具任务仍执行 S2～S11 的相应验证。

忽略节点的 AppKit Children／Parent 使用同一份已发布阅读关系折叠包装，不能丢掉其独立交互后代。原生端仅在父子／阅读顺序变化时更新子项索引，名称、几何和透明度更新不重建该索引。忽略与恢复分别发送适用结构通知，不发送退休通知。

AppKit 的 accessibilityHitTest 从屏幕点经当前 NSWindow／NSView 转为窗口逻辑坐标，按已发布的场景顺序调用 native contains；内容 view 与 provider 根使用同一入口。节点内查询限于该语义子树，根查询覆盖全部内容；窗口内没有命中子项时返回内容根，窗口外或失活时无结果。更新窗口位置或 DPI 使用现有坐标换算，查询不请求帧。命中索引只在快照有无、节点增删或场景顺序变化时更新；名称、错误和普通 handler 更新不重排索引。激活点查询仍须另行满足实际输入资格和可点击位置合同，本批不开放未经验证的 ActivationPoint。
