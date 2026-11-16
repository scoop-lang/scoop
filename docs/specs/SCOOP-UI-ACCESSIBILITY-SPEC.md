# UI 无障碍接口规范

状态：S2／S5 实施中。本文规定公共语义资源、逻辑更新、内容合并、文本宿主和 Accepted 场景的接入；底层值及原生发布见 [Window 无障碍接口规范](SCOOP-WINDOW-ACCESSIBILITY-SPEC.md)。基础控件的默认接入见 [Component spec](SCOOP-UI-COMPONENT-SPEC.md#11-默认无障碍语义)，复杂控件关系、完整文本与集合、其余控件及其他平台继续按[统一计划](../ui/interaction-polish/PLAN.md)推进，不据本批基础接口声明完整 accessibility 已完成。

## 1. 资源与父子关系

`ComponentScope.semantics(role, parent = None): SemanticsHandle` 在构造／挂载阶段创建语义资源；首次登记时为当前根创建 Window provider，根关闭时关闭它，保留借用的原生 Window。没有语义声明的根不创建空树。当前工厂按 Window 能力在 Darwin 提供；SemanticsHandle、路由与场景实现不含平台分支。未来平台使用同一声明，不增加并行接口。

`SemanticsHandle` 为 scope 管理的 Closeable，提供只读 id、role、isClosed 和 geometry。id 使用 Window 的 SemanticNodeId，重排及属性更新保持身份。角色固定；替换角色创建新资源并关闭旧资源，不能让旧原生引用指向新角色。调用方可提前 close，卸载仍自动清理，重复 close 无操作。

一个挂载可以登记多个节点。缺省父节点来自最近祖先挂载的有效子项路由，最终落到 provider 根；本挂载的多个节点默认是兄弟，顺序为登记顺序。显式 parent 必须属于本挂载或其逻辑祖先、在同一活动根内；不允许指向后代挂载或其他根。自身内部嵌套通过同一挂载中先创建的父节点表达。

`ComponentScope.routeSemanticsChildren(parent: SemanticsHandle?)` 指定普通子挂载使用的语义父节点；Some 要求资源属于本挂载，None 恢复从祖先继承。它不移动本挂载中其他节点。普通 Component、布局、方向包装与没有登记节点的 Element 默认透明；阅读顺序沿实际挂载子项／数据顺序，独立于场景层叠顺序。Lazy 仅处理已挂载条目，不为发布普通树枚举未实现的数据项；完整虚拟集合另按 S7 接入。

关闭单个 UI handle 只退休这个资源。框架先把仍由其他活动 scope／handle 拥有的直接语义子节点接到其原父级，清除已关闭的子项路由，然后按当前挂载关系重新发布；不会因替换父角色而误退役仍存活的子控件。显式子节点对已关闭父资源的引用同样移向原父级。真正卸载时，该挂载及其后代各自的资源全部退休。UI 不暴露底层 AccessibilityNode，避免绕过此归属规则。

## 2. 属性、动作与逻辑发布

`handle.update(properties, actions = SemanticActions(), relations = SemanticRelations())` 替换 Window SemanticProperties、最新动作与关系；关系使用同一活动根的 handle.id，不改变角色、身份、父子归属或业务状态。检查、名称／说明／错误解析、只读／禁用／受保护文本规则沿底层公共合同。名称或值改变立即使旧几何失效；只替换同能力处理器不要求绘制，也不重复原生属性通知。render／measure／scene／draw 不能创建、更新或关闭语义资源；Scene 只登记几何，挂载构造与正常 update 可修改资源。

标签文字变化经底层关系依赖更新引用者，无需重建消费它的 Element 或请求 drawable；显式空名称优先于标签。单独关闭 UI handle 后指向它的 labelledBy／describedBy／errorMessages／controls 自动清除，handle.relations 只读返回当前关系，不按相同位置或新角色转接到替代节点。update 中关系声明与上次成功声明相同时，使用底层已清理的当前关系，避免普通 Props／模型更新重新提交退休引用；显式更换关系仍按新声明验证。bindFocus 同样保留当前关系。Scene 捕获并核对底层 contentRevision，标签依赖变化后的迟到 Accepted 场景不能恢复旧几何。内容合并／忽略策略见下节，关系本身不自动吞掉标签的交互子项。

属性、父子关系与实际焦点在根的一次正常逻辑更新结束后一起发布。独立 handle 更新也通过现有 UiHost 队列唤醒该根，不请求 drawable，不增加另一套调度器。结构未变时不重新遍历挂载树；同一轮重复修改按最终值合并。失活／关闭立即拒绝旧动作，不等待下一帧。

每个已登记动作先检查根与节点生命期、所属 InputScope 的实际输入边界和当前 UI 执行状态，再经 Surface 的同步错误边界调用现有业务 handler。没有登记动作时不公布对应能力。动作中写入 State 后执行已排队的正常更新，发布实际结果再返回；不模拟指针事件，也不绘制或 pump 原生循环。UI 正在 render／布局／宿主派发时返回 Busy；异常关闭对应根并沿现有错误路径处理。handler 可关闭自己或根，正常完成仍按已经执行的动作返回 Completed，不能继续解引用退休的节点。

`handle.bindFocus(target: FocusTarget?)` 借用本挂载的实际 FocusTarget，None 解除绑定；同一目标不能同时绑定两个活动语义节点。绑定使 provider 的 focused 反映真实活动键盘焦点，查询不改变焦点。若 actions 未显式提供 focus，绑定提供经 FocusHandle 的聚焦／解除焦点动作；需要后续布局显露的请求返回 Busy，直到实际获得焦点才可返回 Completed，不把排队成功当成聚焦完成。目标关闭后不再报告焦点，动作返回 Unavailable。显式 focus handler 仍使用绑定目标作为实际焦点事实。

## 3. 内容合并与忽略

SemanticProperties.ignored 仅忽略当前语义节点，hidden 排除其语义子树；它们不修改真实布局、输入或键盘焦点。忽略的包装保留后代的身份和阅读顺序，祖先禁用／隐藏仍约束后代动作。没有语义资源的布局继续默认透明，不需要为透明布局额外创建节点。

required 沿 Window 属性合同描述实际表单要求，不通过祖先自动继承。合并静态标签以及 updateTextEditor／updateSelectableText 的属性投影保留此事实；它不要求文本可写或公开正文，不触发编辑事务、获取焦点或重新布局。

handle.mergeLabels 默认 false，可在正常更新阶段切换。开启后，根沿该容器的语义阅读顺序吸收纯 Text 叶子的内容，并把这些叶子标为 ignored；它们继续保留资源身份和标签依赖。只有没有动作、没有绑定焦点、没有公开选区、非受保护且没有语义子节点的 Text 才能合并。显式 ignored 包装允许继续查找其后代，但不贡献包装自身的文字；hidden 子树不贡献内容。普通 Group、其他角色、带动作／焦点／选区／子控件的文本及嵌套合并容器保持独立，不遍历任意 Custom View 猜测内容。

合并名称采用显式 name、有效 labelledBy、已知内容文字的顺序，空字符串与未知分开。即使有显式名称覆盖，已经声明合并的静态标签仍避免重复进入导航。由内容声明产生的 fallback 引用不混入 handle.relations；根借用 Window contentLabels 发布并随结构、动作、焦点绑定和隐藏策略变化更新。合并、忽略和恢复均不退休节点，不请求 drawable；纯标签文字变化沿既有名称依赖更新，不重走挂载树。

动作、焦点或结构变化使文本不再适合合并时，同一次逻辑发布恢复该节点并保留其真实动作。先撤下已改变的旧内容引用，再安装本次引用，避免合法重排被中间状态的旧关系干扰；实际名称循环仍按 Window 的 publish 合同报告。UI 根通过现有错误边界处理非法声明，不发布半棵原生树。内容名称变化沿 contentRevision 拒绝不匹配的旧场景。

## 4. 场景与几何

`SceneContext.semantics(handle, bounds: Rect)` 与 Path 重载接受本挂载或其逻辑祖先的活动 handle，在当前 Canvas 坐标与裁剪中捕获独立几何和 native 命中快照。组合控件可在拥有实际 FocusTarget 的 Component 中持有语义资源，由负责绘制的后代 Element 登记位置；资源归属、焦点绑定与语义父子关系不因此改变。其他根、兄弟或后代挂载的资源均拒绝。每个节点每帧只能登记一次；不创建 InputArea。Rect 登记作为 SceneCommand 参与缓存记录和重放，重放时重新应用当前祖先变换与 clip，不保存上一帧的输入位置；Path 与现有 Path 输入放置一样使该场景不缓存，捕获后不借用调用方的 Path。opacity 为零或没有绘图不自动隐藏语义节点。

根统一在 FrameCompletion.Accepted 发布候选几何；Discarded 释放候选，保留仍匹配当前语义属性的上次几何。尚未呈现、当前 Accepted 场景未登记或名称／值已变化的节点报告 None；被自身 clip 完全裁掉时，保留完整 bounds，并以 visibleBounds=None 表示裁剪。后续匹配场景恢复几何，节点身份不变。帧生成后发生名称／值变化或节点退休，旧候选不能重新使其几何有效。

几何来自 graphics 的 CanvasGeometry，与真实绘制使用同一变换、精确 clip 和 viewport。通过 withNativeHitTest 保留不可变命中快照，再连同窗口逻辑坐标的 bounds／visibleBounds 交给 Window；原生 getter 不回到 UI 代码，也不据矩形中心承诺可点击点。登记的实际场景顺序决定命中先后，不使用挂载／阅读顺序代替层叠。滚动条留白、滚动变换、Portal 和其他场景变换均在登记点生效；完整 Reveal 与虚拟化能力继续使用既有滚动／焦点路径。

候选场景持有自己的 native 引用，Accepted 时仅把内容版本仍匹配的几何与命中一起发布；Discarded、版本不匹配、场景失败及根关闭均释放候选引用。已发布 Window 节点和原生投影各自保留所需快照，候选释放不使当前命中悬空；纯属性发布不重新捕获几何。未放置与名称／值失效同时撤下位置和命中，旧 native 对象不能继续按过期形状命中。

## 5. 文本编辑宿主

`scoop.ui.text` 的 `SemanticsHandle.updateTextEditor(host, properties = SemanticProperties(), relations = SemanticRelations())` 用于本挂载的 TextField 节点，在正常 update 中发布现有 TextEditHost 的正文、方向选区、真实 focus 和编辑动作。它借用 host，不能绑定其他挂载或改变节点角色；应用继续按普通 State／Props 更新调用，不创建另一份可写文档、IME 连接或撤销栈。properties 的名称／说明／错误／标识／隐藏／忽略及 relations 保留；正文和选区取模型，enabled 与宿主共同约束，readOnly／protectedText 任一方声明即生效。受保护时 value／selection 都为 None，与显示 projection 和复制权限无关。

动作捕获该次模型身份与 revision；模型替换、正文或选区发生尚未发布的修改时返回 Unavailable，不能修改新的焦点文档。同一模型实际获得新状态后沿正常更新发布，再返回结果。SetValue／ReplaceText 使用 TextEditSource.Accessibility，经过单行归一化、外来文本清洗、transform、accepts、长度与组合事务，并形成独立 undo 项；Reject 返回 Rejected，Transform 返回实际结果。选区修改通过同一 composition 结束与策略事务，普通只读选择允许，disabled 或输入边界不允许时拒绝。读取、选择与编辑不隐式抢焦点，Focus 仍走绑定的真实 FocusTarget。

菜单模式未禁用时提供 ShowContextMenu。该明确请求可先通过宿主的 FocusHandle 聚焦原目标，再以同一 showContextMenu 入口使用 Accepted caret／选区锚点；等待显露或实际焦点时返回 Busy，缺少有效锚点或当前资格时拒绝。已有菜单未结束返回 Busy，禁用模式不公布该动作；选区、复制策略、composition、模型换绑和关闭仍由原菜单会话管理。受保护编辑器可打开其实际允许的菜单，复制等项目遵守现有独立策略。

`SemanticsHandle.updateSelectableText(host, properties = SemanticProperties(), relations = SemanticRelations())` 将同一挂载的 SelectableTextHost 接到 Text 角色，始终只读；名称／关系、enabled／protectedText 与真实 focus 的处理沿同一原则，不提供写入动作。正文使用 StyledText.toPlainText，包括行内对象的 alternateText；选区通过现有区间替换映射在原始占位符与替代文字间转换。请求端点在替代文字内部时，非空范围向外覆盖整个对象；caret 使用下游端点。空替代文字没有文本长度，仍保留原对象边界的 affinity。范围与选区发布后反映实际映射结果，不伪造可选择半个行内对象的能力。

updateSelectableText 借用 host，不改写它的 onSelectionChanged。调用方在正常 update 以及选择通知后刷新语义；可先更新已持有的 handle，再执行业务通知。宿主按不可变 StyledText 身份复用正文与区间映射，单纯拖选不反复生成整份替代文字；内容替换、受保护发布与关闭释放不再需要的投影。正文或样式对象换绑后的旧动作返回 Unavailable，新 update 发布当前对象及动作。辅助技术选择走宿主同一 setSelection，停止拖选并保留正常选择通知；菜单复用 Copy／SelectAll 会话。

正文与选区可在无 drawable 时发布；场景只负责当前节点的 Accepted 位置和命中。完整范围几何、文字单位、保留范围及行内子控件语义后续接入，同步见 Window 文本合同。

## 6. 滚动条节点

`SemanticsHandle.updateScrollBar(viewport: ScrollViewport, orientation: SemanticOrientation, lineStep: Double = 40.0, properties: SemanticProperties = SemanticProperties(), relations: SemanticRelations = SemanticRelations())` 将本挂载的 ScrollBar 节点接到同一挂载的真实视口。scoop.ui.text 提供以 TextEditHost 为首参数的同名重载，借用其原视口；不公开或替换编辑器的滚动所有权。

调用方在正常 update 及视口的 Accepted 完成回调之后刷新。方向和有限正 lineStep 由当前控件提供，属性和关系保持调用方覆盖；scroll 由该视口的最后一次 Accepted 范围决定，尚无范围或该轴零范围时忽略条节点并撤下滚动能力。重新绑定清除原视口的就绪状态，Discarded 不发布候选范围。视觉透明度、hover 和自动隐藏不改变逻辑轴身份或动作；几何仍只来自 Scene 的真实条区域，未绘制的条不获得容器的点击点。

位置写入采用 0～1 比例，经同一 ScrollState 登记 User 来源请求；保留另一轴尚未呈现的请求位置，连续改写同一轴按最新目标替换，不叠加为相对偏移。Increment／Decrement 使用 lineStep 与现有 scrollBy，沿原动画取消、范围夹取和意图通知；边界上没有实际位移时拒绝。回调检查视口绑定和挂载寿命，不把旧条转向另一状态。请求登记后返回 Completed，原生值等待真实 Accepted 范围更新；本接口不宣称完成 Reveal 或取得焦点。

## 7. 验证边界

用真实第三方 Element 验证无 InputArea 的文本、多节点、透明挂载、子项路由、重排／角色替换、最新 handler、禁用／隐藏／模态限制、自删与关根、跨根错误和旧对象退休。无 drawable 的属性更新必须独立验证；场景缓存命中、Accepted／Discarded、裁剪、未放置和属性与候选帧不匹配必须通过实际场景验证。

原生方法的进程内调用、外部 AX 客户端与 VoiceOver 分别记录。检查与构建不能代替实窗通过；其他平台只在其 provider 与实际工具验收后声明支持。
