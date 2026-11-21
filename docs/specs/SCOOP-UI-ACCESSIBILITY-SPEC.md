# UI 无障碍接口规范

状态：功能已接通，三平台集中验收中。本文规定公共语义资源、逻辑更新、内容合并、文本宿主、虚拟集合和 Accepted 场景的接入；底层值及原生发布见 [Window 无障碍接口规范](SCOOP-WINDOW-ACCESSIBILITY-SPEC.md)。控件的默认接入见 [Component spec](SCOOP-UI-COMPONENT-SPEC.md#11-默认无障碍语义)。平台构建、原生服务、外部客户端与读屏器分别按[统一计划](../ui/interaction-polish/PLAN.md)验收，不以共用模型通过替代平台验证。

## 1. 资源与父子关系

`ComponentScope.semantics(role, parent = None): SemanticsHandle` 在构造／挂载阶段创建语义资源；首次登记时为当前根创建 Window provider，根关闭时关闭它，保留借用的原生 Window。没有语义声明的根不创建空树。Darwin、Windows 和 Linux GNU 使用同一工厂，SemanticsHandle、路由与场景实现不含平台分支。窗口处于 PendingConfigure 时也可登记和发布语义；几何等待实际 Accepted 场景，原生动作仍要求窗口 Open。

`SemanticsHandle` 为 scope 管理的 Closeable，提供只读 id、role、isClosed 和 geometry。id 使用 Window 的 SemanticNodeId，重排及属性更新保持身份。角色固定；替换角色创建新资源并关闭旧资源，不能让旧原生引用指向新角色。调用方可提前 close，卸载仍自动清理，重复 close 无操作。

一个挂载可以登记多个节点。缺省父节点来自最近祖先挂载的有效子项路由，最终落到 provider 根；本挂载的多个节点默认是兄弟，顺序为登记顺序。显式 parent 必须属于本挂载或其逻辑祖先、在同一活动根内；不允许指向后代挂载或其他根。自身内部嵌套通过同一挂载中先创建的父节点表达。

`ComponentScope.routeSemanticsChildren(parent: SemanticsHandle?)` 指定普通子挂载使用的语义父节点；Some 要求资源属于本挂载，None 恢复从祖先继承。它不移动本挂载中其他节点。普通 Component、布局、方向包装与没有登记节点的 Element 默认透明；阅读顺序沿实际挂载子项／数据顺序，独立于场景层叠顺序。未登记集合的 Lazy 仅处理已挂载条目；已登记的集合按 §6.1 发布逻辑条目，普通树更新仍不枚举未实现的数据项。

Portal 的内容默认从窗口 provider 根开始新的语义子树，不继承触发器所在的行、单元格或文本容器路由。这样弹层不会被虚拟集合的行列投影或正文内容合并吞掉；层内的普通路由与显式 parent 仍沿上述规则处理。嵌套 Portal 同样独立发布。该边界只改变默认语义父级，逻辑挂载、输入路由、模态边界、焦点恢复和资源归属保持原来的 Portal 合同。

关闭单个 UI handle 只退休这个资源。框架先把仍由其他活动 scope／handle 拥有的直接语义子节点接到其原父级，清除已关闭的子项路由，然后按当前挂载关系重新发布；不会因替换父角色而误退役仍存活的子控件。显式子节点对已关闭父资源的引用同样移向原父级。真正卸载时，该挂载及其后代各自的资源全部退休。UI 不暴露底层 AccessibilityNode，避免绕过此归属规则。

`SemanticsReference` 是跨声明位置借用同一挂载语义身份的轻量引用；`id` 可参与正常 render 依赖，未绑定或退休后为 None。`SemanticsHandle.bindReference(reference)` 只允许该 reference 绑定一个活动节点，换绑与关闭同步撤下旧引用，不产生焦点、View 或延迟重试。重复绑定同一节点不发布状态变化，也不使读取该 id 的 render 再次失效；`bindReferences` 保留、重排或重复列出同一引用同样如此。真实绑定、解绑和退休仍通知其依赖者。Component 的真实控件可以提供该引用供 controls／label／active descendant 关系使用。

## 2. 属性、动作与逻辑发布

`handle.update(properties, actions = SemanticActions(), relations = SemanticRelations())` 替换 Window SemanticProperties、最新动作与关系；关系使用同一活动根的 handle.id，不改变角色、身份、父子归属或业务状态。检查、名称／说明／错误解析、只读／禁用／受保护文本规则沿底层公共合同。名称或值改变立即使旧几何失效；只替换同能力处理器不要求绘制，也不重复原生属性通知。render／measure／scene／draw 不能创建、更新或关闭语义资源；Scene 只登记几何，挂载构造与正常 update 可修改资源。

标签文字变化经底层关系依赖更新引用者，无需重建消费它的 Element 或请求 drawable；显式空名称优先于标签。单独关闭 UI handle 后指向它的 labelledBy／describedBy／errorMessages／controls 自动清除，handle.relations 只读返回当前关系，不按相同位置或新角色转接到替代节点。update 中关系声明与上次成功声明相同时，使用底层已清理的当前关系，避免普通 Props／模型更新重新提交退休引用；显式更换关系仍按新声明验证。bindFocus 同样保留当前关系。Scene 捕获并核对底层 contentRevision，标签依赖变化后的迟到 Accepted 场景不能恢复旧几何。内容合并／忽略策略见下节，关系本身不自动吞掉标签的交互子项。

属性、父子关系与实际焦点在根的一次正常逻辑更新结束后一起发布。独立 handle 更新也通过现有 UiHost 队列唤醒该根，不请求 drawable，不增加另一套调度器。结构未变时不重新遍历挂载树；同一轮重复修改按最终值合并。失活／关闭立即拒绝旧动作，不等待下一帧。

每个已登记动作先检查根与节点生命期、所属 InputScope 的实际输入边界和当前 UI 执行状态，再经 Surface 的同步错误边界调用现有业务 handler。没有登记动作时不公布对应能力。动作中写入 State 后执行已排队的正常更新，发布实际结果再返回；不模拟指针事件，也不绘制或 pump 原生循环。UI 正在 render／布局／宿主派发时返回 Busy；异常关闭对应根并沿现有错误路径处理。handler 可关闭自己或根，正常完成仍按已经执行的动作返回 Completed，不能继续解引用退休的节点。

`handle.bindFocus(target: FocusTarget?)` 借用本挂载的实际 FocusTarget，None 解除绑定；同一目标不能同时绑定两个活动语义节点。绑定使 provider 的 focused 反映真实活动键盘焦点，查询不改变焦点。若 actions 未显式提供 focus，绑定提供经 FocusHandle 的聚焦／解除焦点动作；需要后续布局显露的请求返回 Busy，直到实际获得焦点才可返回 Completed，不把排队成功当成聚焦完成。目标关闭后不再报告焦点，动作返回 Unavailable。显式 focus handler 仍使用绑定目标作为实际焦点事实。

数值、选择、展开和关闭动作同样经 SemanticsHandle 的真实根动作边界执行，复用所属控件的业务入口。`SemanticsHandle.announce(text, priority = SemanticLive.Polite)` 只提交同一窗口的公告；实际模态／隐藏状态及同一发布周期的合并沿 Window 合同，读取节点不公告。Component 的 busy／live 仅描述应用状态，不自动创建验证器或禁止输入。

## 3. 内容合并与忽略

SemanticProperties.ignored 仅忽略当前语义节点，hidden 排除其语义子树；它们不修改真实布局、输入或键盘焦点。忽略的包装保留后代的身份和阅读顺序，祖先禁用／隐藏仍约束后代动作。没有语义资源的布局继续默认透明，不需要为透明布局额外创建节点。

required 沿 Window 属性合同描述实际表单要求，不通过祖先自动继承。合并静态标签以及 updateTextEditor／updateSelectableText 的属性投影保留此事实；它不要求文本可写或公开正文，不触发编辑事务、获取焦点或重新布局。

handle.mergeLabels 默认 false，可在正常更新阶段切换。开启后，根沿该容器的语义阅读顺序吸收纯 Text 叶子的内容，并把这些叶子标为 ignored；它们继续保留资源身份和标签依赖。只有没有动作、没有绑定焦点、没有公开选区、非受保护且没有语义子节点的 Text 才能合并。显式 ignored 包装允许继续查找其后代，但不贡献包装自身的文字；hidden 子树不贡献内容。普通 Group、其他角色、带动作／焦点／选区／子控件的文本及嵌套合并容器保持独立，不遍历任意 Custom View 猜测内容。

合并名称采用显式 name、有效 labelledBy、已知内容文字的顺序，空字符串与未知分开。即使有显式名称覆盖，已经声明合并的静态标签仍避免重复进入导航。由内容声明产生的 fallback 引用不混入 handle.relations；根借用 Window contentLabels 发布并随结构、动作、焦点绑定和隐藏策略变化更新。合并、忽略和恢复均不退休节点，不请求 drawable；纯标签文字变化沿既有名称依赖更新，不重走挂载树。

动作、焦点或结构变化使文本不再适合合并时，同一次逻辑发布恢复该节点并保留其真实动作。先撤下已改变的旧内容引用，再安装本次引用，避免合法重排被中间状态的旧关系干扰；实际名称循环仍按 Window 的 publish 合同报告。UI 根通过现有错误边界处理非法声明，不发布半棵原生树。内容名称变化沿 contentRevision 拒绝不匹配的旧场景。

当前 AllInput 模态边界也限制语义阅读：边界外的兄弟内容隐藏，连接模态子树所需的逻辑祖先仅作为忽略包装保留，边界内仍沿原阅读关系。FocusOnly 边界不隐藏背景。边界建立、退出和换绑沿现有 InputBoundary 的生命周期更新，不能等一次键盘焦点变化才生效。模态最终移除包括 UI 规范第 7.6.1 节的 Accepted 移除和不可绘制时的逻辑收尾；后者在同一更新中恢复背景阅读及动作资格，不让已经结束的模态留下空语义树。几何仍沿真实 Accepted 帧发布。保留但已折叠的内容由其真实容器发布 hidden，不把普通 disabled 控件从阅读树移除。

## 4. 场景与几何

`SceneContext.semantics(handle, bounds: Rect)` 与 Path 重载接受本挂载或其逻辑祖先的活动 handle，在当前 Canvas 坐标与裁剪中捕获独立几何和 native 命中快照。组合控件可在拥有实际 FocusTarget 的 Component 中持有语义资源，由负责绘制的后代 Element 登记位置；资源归属、焦点绑定与语义父子关系不因此改变。其他根、兄弟或后代挂载的资源均拒绝。每个节点每帧只能登记一次；不创建 InputArea。Rect 登记作为 SceneCommand 参与缓存记录和重放，重放时重新应用当前祖先变换与 clip，不保存上一帧的输入位置；Path 与现有 Path 输入放置一样使该场景不缓存，捕获后不借用调用方的 Path。opacity 为零或没有绘图不自动隐藏语义节点。

根统一在 FrameCompletion.Accepted 发布候选几何；Discarded 释放候选，保留仍匹配当前语义属性的上次几何。尚未呈现、当前 Accepted 场景未登记或名称／值已变化的节点报告 None；被自身 clip 完全裁掉时，保留完整 bounds，并以 visibleBounds=None 表示裁剪。后续匹配场景恢复几何，节点身份不变。帧生成后发生名称／值变化或节点退休，旧候选不能重新使其几何有效。

根内容与每个 overlay 的测量收尾后、开始捕获该场景几何前，先发布测量中新增挂载的逻辑语义和文本合并关系。新惰性单元格的内容名称必须先确定，再记录对应 contentRevision 的几何；不能在捕获后才安装同一批标签引用，使首次 Accepted 的可见单元格缺少位置。这里只处理既有脏语义，不重新测量、创建额外条目或请求补偿帧；后来的真实内容变化仍拒绝旧候选。

几何来自 graphics 的 CanvasGeometry，与真实绘制使用同一变换、精确 clip 和 viewport。通过 withNativeGeometry 保留不可变命中快照，再连同窗口逻辑坐标的 bounds／visibleBounds 交给 Window；原生 getter 不回到 UI 代码，也不据矩形中心承诺可点击点。登记的实际场景顺序决定命中先后，不使用挂载／阅读顺序代替层叠。滚动条留白、滚动变换、Portal 和其他场景变换均在登记点生效；完整 Reveal 与虚拟化能力继续使用既有滚动／焦点路径。

候选场景持有自己的 native 引用，Accepted 时仅把内容版本仍匹配的几何与命中一起发布；Discarded、版本不匹配、场景失败及根关闭均释放候选引用。已发布 Window 节点和原生投影各自保留所需快照，候选释放不使当前命中悬空；纯属性发布不重新捕获几何。未放置与名称／值失效同时撤下位置和命中，旧 native 对象不能继续按过期形状命中。

### 4.1 不改变焦点的显露

UI 节点默认提供 reveal 动作，包括没有 InputArea 或 FocusTarget 的 Text。显式 `SemanticActions.reveal` 可替换默认实现，继续经过相同的根与动作边界。默认能力不使普通静态标签失去合并资格；显式业务 reveal 与其他显式动作一样保留独立语义。ignored／已合并节点及 hidden／disabled 祖先的后代不能执行显露。

Scene 登记同时保存当次 ScrollViewport 路径，Rect 缓存重放重新取得当前路径。默认显露使用匹配内容版本的 Accepted bounds、精确裁剪得到的 visibleBounds 和原 ScrollReveal 算法，从最内层向外逐个提出 Nearest 定位，继续扣除 ScrollPadding；不为语义内容创建输入区域，也不计算另一份未呈现位置。每次实际移动等待下一次 Accepted 场景后再处理外层，最终必须有非空可见区域；目标大于视口时允许显示其可达部分。不可滚出的自有裁剪、空形状、未放置或内容版本失配返回 Unavailable，既不抢焦点，也不修改文本选区。

同一真实视口内存在固定表头或冻结列时，使用 UI 规范 §7.7 的 `SceneContext.withScrollReveal` 收窄相应子树的可用区域和滚动轴；普通节点及虚拟单元格的 Reveal 使用这份 Accepted 路径。中心单元格显露到冻结区之间，冻结单元格可只沿纵轴显露，仍复用同一 ScrollState、临时条目保留与取消规则。

需要后续帧的请求返回 Busy，根借用现有 requestFrame／Accepted 完成回调继续推进；重复请求同一节点沿用在途定位，另一节点的新请求取代旧请求。后续滚动意图、视口换绑、内容失效、节点退休、输入或语义资格丢失使旧请求结束，根关闭释放请求。Discarded 不推动外层或宣告成功，保留仍有效的待处理请求；停在等待状态时只等待已有的实际帧，不增加定时轮询。原生再次调用时，只有当前已接受场景确已显露才返回 Completed。查询属性或动作列表不启动显露，窗口无法呈现时不报成功。

本接口处理已经挂载、已登记几何的节点；未实例化条目的按 key 查询、Realize 与保留仍由 S7 的集合合同接入。

## 5. 文本编辑宿主

保留文本范围跟随 TextEditorState 的实际编辑事务，与哪个宿主获得编辑焦点无关。各宿主按需登记自己拥有的 SemanticTextDocument；同一模型的输入、程序修改、undo／redo 和正文重置更新所有已登记文档的端点，复用该次事务的偏移映射，不复制正文或历史。换绑、进入保护模式及关闭宿主时撤销登记并退休旧范围；模型不因此取得宿主的输入或焦点所有权。

`scoop.ui.text` 的 `SemanticsHandle.updateTextEditor(host, properties = SemanticProperties(), relations = SemanticRelations(), setExpanded = None)` 用于本挂载的 TextField 节点，在正常 update 中发布现有 TextEditHost 的正文、方向选区、真实 focus 和编辑动作。它借用 host，不能绑定其他挂载或改变节点角色；应用继续按普通 State／Props 更新调用，不创建另一份可写文档、IME 连接或撤销栈。properties 的名称／说明／错误／标识／隐藏／忽略及 relations 保留；正文和选区取模型，enabled 与宿主共同约束，readOnly／protectedText 任一方声明即生效。受保护时 value／selection 都为 None，与显示 projection 和复制权限无关。

动作捕获该次模型身份与 revision；模型替换、正文或选区发生尚未发布的修改时返回 Unavailable，不能修改新的焦点文档。同一模型实际获得新状态后沿正常更新发布，再返回结果。SetValue／ReplaceText 使用 TextEditSource.Accessibility，经过单行归一化、外来文本清洗、transform、accepts、长度与组合事务，并形成独立 undo 项；Reject 返回 Rejected，Transform 返回实际结果。选区修改通过同一 composition 结束与策略事务，普通只读选择允许，disabled 或输入边界不允许时拒绝。读取、选择与编辑不隐式抢焦点，Focus 仍走绑定的真实 FocusTarget。

菜单模式未禁用时提供 ShowContextMenu。该明确请求可先通过宿主的 FocusHandle 聚焦原目标，再以同一 showContextMenu 入口使用 Accepted caret／选区锚点；等待显露或实际焦点时返回 Busy，缺少有效锚点或当前资格时拒绝。已有菜单未结束返回 Busy，禁用模式不公布该动作；选区、复制策略、composition、模型换绑和关闭仍由原菜单会话管理。受保护编辑器可打开其实际允许的菜单，复制等项目遵守现有独立策略。

`SemanticsHandle.updateSelectableText(host, properties = SemanticProperties(), relations = SemanticRelations())` 将同一挂载的 SelectableTextHost 接到 Text 角色，始终只读；名称／关系、enabled／protectedText 与真实 focus 的处理沿同一原则，不提供写入动作。正文使用 StyledText.toPlainText，包括行内对象的 alternateText；选区通过现有区间替换映射在原始占位符与替代文字间转换。请求端点在替代文字内部时，非空范围向外覆盖整个对象；caret 使用下游端点。空替代文字没有文本长度，仍保留原对象边界的 affinity。范围与选区发布后反映实际映射结果，不伪造可选择半个行内对象的能力。

updateSelectableText 借用 host，不改写它的 onSelectionChanged。调用方在正常 update 以及选择通知后刷新语义；可先更新已持有的 handle，再执行业务通知。宿主按不可变 StyledText 身份复用正文与区间映射，单纯拖选不反复生成整份替代文字；内容替换、受保护发布与关闭释放不再需要的投影。正文或样式对象换绑后的旧动作返回 Unavailable，新 update 发布当前对象及动作。辅助技术选择走宿主同一 setSelection，停止拖选并保留正常选择通知；菜单复用 Copy／SelectAll 会话。

主题、选区前景或窗口活动外观变化不改正文、选区及语义身份，不因重绘重复发送文本／选择通知。文字缩放更新同一宿主的排版，仍只在匹配内容的 Accepted 场景发布新范围几何；逻辑焦点、模型与历史保留。

正文与选区可在无 drawable 时发布。updateTextEditor／updateSelectableText 同时绑定当前宿主的范围查询来源；宿主在真实文字绘制处登记自己的局部视口、变换和 clip，根与节点位置一起在匹配内容版本的 Accepted 场景发布，Discarded 和过期候选释放自己的引用。未登记节点位置或文字、内容失配、解除绑定及关闭时不保留旧文本几何。普通 handle.update 解除专用宿主及跟踪文档绑定，客户端的旧范围永久失效；重新调用对应宿主入口可恢复绑定，但不能复活旧范围。场景登记不创建 InputArea，也不修改正文、选区或真实焦点。

排版查询通过 graphics.text 的 withNativeLayout 保留同一 TextLayout，几何使用 graphics 的 withNativeGeometry；原生 getter 不调用 Scoop、measure 或 render。文字来源按实际布局、正文与投影复用，单纯滚动／变换仅产生新的场景几何引用，不每帧复制正文、边界表或重建排版。文本编辑的 displayRange、选区 affinity、TextProjection 区间、OTP 的实际 cell／caret，以及可选文本的 alternateText 区间都来自既有宿主数据；不另造布局或可写文本模型。

grapheme 范围使用语义正文的原边界；词、视觉行和逻辑行从实际排版映射回正文，软换行不变成正文换行。编辑器范围几何使用同一 displayRange 端点规则，投影中没有实际显示的范围不可用；可选文本的非空替代文字范围向外覆盖整个行内对象，caret 使用下游端点。按点查询返回该显示片段实际代表的正文范围，行内对象可以对应完整替代文字；无文本长度的装饰不制造正文字符。矩形保留实际 bidi 片段，原生 AppKit 按平台协议取其外接矩形；精确可见范围仍使用同一 Canvas clip，而非矩形重叠或采样点。

受保护用途既不绑定原文范围也不向 native 发布正文长度或来源表；视觉显隐不改变这一点。两个专用入口把实际 cursor 的 TextAffinity 转为 SemanticProperties.cursorAffinity，软换行前一行末与后一行首不因字节偏移相同而混淆。仅选区或 cursor 偏向变化可复用与当前正文匹配的 Accepted 排版，正文变化立即撤下旧查询，不能把新偏移应用于旧文字。保留范围与行内子控件沿 Window 的文本范围及文本子对象合同发布，各平台实际验证范围见实施记录。

## 6. 滚动条节点

`SemanticsHandle.updateScrollBar(viewport: ScrollViewport, orientation: SemanticOrientation, lineStep: Double = 40.0, properties: SemanticProperties = SemanticProperties(), relations: SemanticRelations = SemanticRelations())` 将本挂载的 ScrollBar 节点接到同一挂载的真实视口。scoop.ui.text 提供以 TextEditHost 为首参数的同名重载，借用其原视口；不公开或替换编辑器的滚动所有权。

调用方在正常 update 及视口的 Accepted 完成回调之后刷新。方向和有限正 lineStep 由当前控件提供，属性和关系保持调用方覆盖；scroll 由该视口的最后一次 Accepted 范围决定，尚无范围或该轴零范围时忽略条节点并撤下滚动能力。重新绑定清除原视口的就绪状态，Discarded 不发布候选范围。视觉透明度、hover 和自动隐藏不改变逻辑轴身份或动作；几何仍只来自 Scene 的真实条区域，未绘制的条不获得容器的点击点。

位置写入采用 0～1 比例，经同一 ScrollState 登记 User 来源请求；保留另一轴尚未呈现的请求位置，连续改写同一轴按最新目标替换，不叠加为相对偏移。Increment／Decrement 使用 lineStep 与现有 scrollBy，沿原动画取消、范围夹取和意图通知；边界上没有实际位移时拒绝。回调检查视口绑定和挂载寿命，不把旧条转向另一状态。请求登记后返回 Completed，原生值等待真实 Accepted 范围更新；本接口不宣称完成 Reveal 或取得焦点。

### 6.1 Lazy 集合的数据描述

`ItemSemantics(source, describe, search)` 把同一 ItemSource 快照与数据级描述函数、明确支持的名称／automationId 查找结合。`SemanticsHandle.updateCollection(source, items, viewport)` 用于 LazyElement 自身的 List 节点；items 的 source 必须是相同对象。可选 viewport 指定该 Lazy 挂载或同根祖先实际拥有的活动 ScrollViewport，用于远处条目的显露；兄弟和其他根的视口拒绝。Realize／Reveal／Focus 继续使用实际挂载和 Accepted 的祖先滚动路径，不从估算位置创建另一条滚动路径。没有描述源时仍提供真实 count、稳定 key、位置与集合身份，名称未知，不宣称支持属性搜索。

查询通过只读 UI evaluation 执行 count／keyAt／indexOfKey／describe，不收集依赖，不调用 build；State、滚动、焦点和语义写入遵守现有 evaluation 禁止修改的合同。正常数据更新同时替换集合快照，不等下一次滚动。元数据与具体 View 寿命分离：仅已实现行有 Group 资源和真实子节点，回收时退休它们；逻辑条目代理由 Window 的集合投影持有。该 Group 的 Accepted 几何使用真实行布局及同一裁剪／滚动变换，未呈现行没有可点击坐标。

VirtualList 提供可选 itemSemantics，并默认绑定自身实际 ScrollViewport；已有 ScrollArea、ScrollBar 和行内控件继续使用自身语义，不重复业务选择状态。普通元数据查询不会触发 Realize／Reveal／Focus。

层级行通过 `SemanticItemProperties.hierarchyLevel` 声明从零开始的深度，通过 expanded 区分可展开行与普通叶项。`ItemSemantics.setExpanded` 接受稳定 ViewKey 与目标 Boolean，经现有 owner 动作边界调用应用模型；离屏行同样可执行，不要求 build 或取得焦点。`updateDataCollection` 对应的 `SemanticCollectionActions.setExpanded` 使用相同包装与资源寿命。原生能力随当前快照更新，禁用、删除、卸载及旧代理仍由 Window 的统一边界处理。

### 6.2 Lazy 条目操作与暂时保留

集合默认提供 Realize：经正常 owner 动作边界按 key 调用既有 LazyChildren 准备、挂载和语义发布，临时保留实例直到同步回调结束。仅创建所请求的行，不改变滚动位置或焦点；尚无 Accepted 场景时几何仍不可用。回调结束后恢复普通回收规则，不因原生代理或子对象被保留而固定整个 View。

提供 viewport 时同时支持 Reveal。请求临时保留目标，将其纳入既有 LazyMeasureContext.pinnedItems；LazyElement 按原 pinned 合同测量、放置，几何继续来自 Accepted 场景。随后复用普通 SemanticRevealController 和 ScrollReveal，逐层显露并等待实际可见区域。初次布局之前也观察实际 viewport 与祖先滚动路径的定位意图，后来的滚动／换绑取消旧请求；删除、禁用、隐藏、替换操作、根关闭、失败或完成均释放保留。丢弃帧只等待新的实际提交，不提前报告成功。

存在同挂载 LazyFocus 且描述明确声明 focusable 的条目支持 Focus；沿既有按 key 焦点搜索选择首个真实目标，必要时准备和显露。Completed 只对应目标子树中真实活动的键盘焦点，已登记但未完成为 Busy；不直接激活系统窗口。清除只显式取消该 key 的在途条目请求，并向该 key 的真实目标发起 clearFocus；不把目标转发到另一行，focusRevision 与请求替换遵守既有控制器规则。行内任意控件的操作继续由实际控件自身提供，数据描述不代替其 handler。

### 6.3 表格单元格与列虚拟化

带 `ItemSemantics.grid` 的 Table 同时提供按行、列 key 的 Realize／Reveal／Focus。整行内容已挂载时按 `SemanticTableCell.columnKey` 找到实际单元格；行内还按列虚拟化时，在该列 LazyElement 的 onMount 中调用 `scope.bindGridColumns(table, rowKey)`。table 必须属于当前行的祖先 LazyElement，rowKey 必须是该实际行的 key；列 ItemSource 使用与 SemanticGrid 一致的显式列 key。登记由该 scope 拥有，卸载自动撤下，不为离屏行预建登记或单元格矩阵。

行内跨列通过 `SemanticGrid.span` 提供轻量的锚点范围，span 与 describe 都在正常只读 UI 求值边界执行。被覆盖列的动作已由 Window 快照解析成锚点 key，UI 仅准备该锚点条目；调用方以相同 columnKey 和 columnSpan 发布真实单元格，并在所覆盖区域进入视口时挂载离屏锚点。不允许用重复 View 或多个语义节点模拟同一个跨列格。

明确动作先沿现有 LazyChildren 准备目标行，再在已登记的列容器准备目标列；两个条目均在同步动作返回前保留。Realize 不滚动或聚焦。Reveal 在原 SemanticRevealController 中保留这两个实际条目，将它们纳入既有 pinnedItems，并沿目标单元格的 Accepted 滚动路径推进；后来的滚动、删除、禁用、换绑、根关闭和替换请求均执行原取消规则。Focus 优先使用该实际单元格声明的 focus 回调，未声明时使用绑定的真实 FocusTarget；未完成布局时返回 Busy，清除只影响该格子树内的焦点及请求。元数据描述继续只读，不触发准备或视口移动。

## 7. 验证边界

用真实第三方 Element 验证无 InputArea 的文本、多节点、透明挂载、子项路由、重排／角色替换、最新 handler、禁用／隐藏／模态限制、自删与关根、跨根错误和旧对象退休。无 drawable 的属性更新必须独立验证；场景缓存命中、Accepted／Discarded、裁剪、未放置和属性与候选帧不匹配必须通过实际场景验证。

原生方法的进程内调用、外部 AX 客户端与 VoiceOver 分别记录。检查与构建不能代替实窗通过；其他平台只在其 provider 与实际工具验收后声明支持。

编辑框可另传 setExpanded，用于有实际候选弹层的组合输入；expanded 属性描述当前已采纳的弹层状态，动作仍受原编辑宿主和输入边界约束，不替代编辑事务。UI 发布时为绑定的真实 InputArea 检查可点击点；未呈现、模态外、禁用、遮挡或找不到实际命中位置时不提供该点。

SelectableText 的 TextRangeAction 以稳定 key 创建 Link 语义，名称取当前公开文本范围，调用沿实际动作与 Accepted 输入资格；每个动作保留独立焦点和精确路径。内联 View 的发生位置以稳定 InlineKey 保留 Group 语义及原交互后代，替代文本给出其名称。两者在正常语义发布时与当前宿主正文建立范围映射；正文不匹配、被保护、换绑或卸载时撤下旧映射，不用场景读取修改语义树。原生文本查询可定位内联子项，且不会创建另一套输入行为。
