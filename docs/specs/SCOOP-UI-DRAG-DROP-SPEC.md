# Scoop UI 数据拖放

本文为交互打磨 S6 的 UI 数据拖放合同，复用 [Window 数据拖放](SCOOP-WINDOW-DRAG-DROP-SPEC.md)与 [Surface 命中](SCOOP-SURFACE-SPEC.md#6-区域输入与焦点)。触摸来源和三平台系统会话继续按[统一计划](../ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)实现。

## 1. 接收区域

Darwin 的 `ComponentScope.dropTarget(area: InputArea, handlers: DropAreaHandlers, enabled: Boolean = true): DropArea` 在当前挂载登记接收行为。area 必须属于该挂载；同一区域同时只登记一个。借用已有 InputArea，不新增命中树、不取得焦点或普通鼠标 capture。scope 自动关闭行为，关闭行为不关闭借用的 area。每个 UI 根按需创建一个 Window DropTarget，根关闭时撤销原生登记。

DropAreaHandlers 保存 offer: (DropOffer, Point) -> DropDecision、drop: (DropRequest, Point) -> Boolean 和 leave: () -> Unit = {}。Window offer／request 的位置仍为窗口坐标；第二个参数为区域局部坐标。offer 仅协商元数据，不读取正文，不修改业务集合；drop 接收已复制的实际数据，true 只表示已实际接受。

DropArea 的 update(handlers, enabled = true) 使用当前处理器与资格；普通属性更新无需重注册原生目标。isClosed 可查询，close 幂等。hover: DropHover? 表示当前被选中的接收区域，DropHover 保存 position、format、effect；它可在 render 中追踪读取，变化同时请求该挂载的帧。被尝试但拒绝的区域不获得 hover。DropArea 的创建、update、close 与通知遵守普通 UI owner 及评估阶段限制。

## 2. 路由和实际接收

每次原生 offer 只命中最近 Accepted 场景一次，冻结该次路径与局部坐标；从前景区域沿逻辑父链找到首个仍有资格且接受实际格式／操作的 DropArea。子目标拒绝可尝试该链中的父目标，不扫描其他窗口或穿透无关兄弟。AllInput、Portal 的逻辑路由、当前禁用和卸载沿 Surface 处理，FocusOnly 不拦截指针拖放。

一个根同时只有一个选中的区域。目标变化或正常离开会先清除旧 hover，再对仍存活的旧目标通知一次 leave。禁用后的 leave 延后到正常 UI 更新收尾，不在 update／render 栈中调用业务；关闭和卸载只清理状态，不调用已经退休的业务。窗口失活本身不是数据拖放终止，不能以普通 PointerEvent.Left 或 FocusLost 取消合法的跨窗口会话。

offer 同步写入反馈值并安排后续 UI 更新，不在查询中重建控件；因此反馈自身引起的 render 不会在同一次协商中替换处理器。实际 Drop 与正常 leave 按既有根更新通路收尾。

Drop 重新确认实际命中、坐标、选中区域、处理器修订、格式和操作。读取期间更新处理器、关闭区域、改变落点或使其退出当前输入范围，均拒绝旧提案；不将旧落点应用到新目标。只有一个 drop 处理器接收数据，拒收不在 Drop 阶段改投父目标。接受后自身或根关闭仍保留 true 的实际结果。Root 的错误收尾和回调隔离复用 SurfaceWindow.dispatch，忙于 UI 派发／评估时拒绝同步原生进入，不 pump 或等待自身队列。

UI 根请求 AppKit 的周期 draggingUpdated，在活动拖放期间继续使用当前 Accepted 路由／局部落点更新或撤下反馈；Discarded 不采用新几何。没有活动 offer 时不为拖放请求帧或 timer，没有登记区域时禁用原生接收。外部系统的光标与拖放图像仍由 OS 拥有，UI 不重复绘制原生光标。

## 3. 拖出区域

Darwin 的 ComponentScope.dragSource(area, options = DragSourceOptions(), prepare): DragSource 借用本挂载的 InputArea，作用域退休时自动关闭；不创建第二份输入区域或路由。DragSourceOptions(threshold = 4.0, enabled = true) 的阈值为有限非负窗口逻辑距离，任一轴达到阈值即可开始，窗口缩放不重复应用。零阈值允许在按下的默认阶段开始。普通数据拖出只考虑非上下文菜单的左键单击；多击继续由文本选择等原行为处理。

prepare: (DragSourceStart) -> DragSourceData? 只在达到阈值且默认动作仍有资格时调用一次。DragSourceStart 保存当前 Accepted 几何下的区域局部 start／position，以及原始 press 和当前 pointer 两个 InputPointer；正文准备可据此核对原按下位置、修饰键和当前业务状态。按下时只记录候选，不读取数据或消费点击。prepare 返回 None 时放弃该候选，不接管普通点击，祖先默认动作仍可继续；下一次按下可重新尝试。准备回调中更新、禁用或关闭该行为，会撤销本次准备。

DragSourceData(content: DragContent, effects = DragEffects(true, false, false), localData: Any? = None, onCompleted: (DragResult) -> Unit = {}) 描述本次具体数据和结果处理。localData 沿 Window 的真实来源关联传给同程序接收端，不写入原生数据表示。普通属性更新为下一次准备替换闭包；已准备数据的 onCompleted 始终与该次来源配对，不被下一行、下一文档或下一次 render 的处理器替换。需要 Move 的消费者在准备时捕获来源及修订，并在 Dropped(Move) 后核对来源仍可删除，再提交一次删除；库不自动删除业务数据。

有数据后沿 Surface.claimPointer 接管原按下路径，再由当前 Window.beginDrag 复制表示并延后启动系统会话。子项先获得默认动作；事件被阻止或已被其他作用域接管时，来源不另建竞争会话。准备异常或 Window 启动入口异常沿正常 UI 错误边界处理，已经取得的普通 claim 会收尾。窗口系统操作发出的 Pointer.Cancelled 只结束普通点击／拖动候选，不撤销已准备的数据会话；窗口失焦、capture 结束也不能被解释成系统 Drop 失败。

DragSource.update(options, prepare) 使用最新配置和准备函数；阈值改变取消尚未达到阈值的候选，不改变已启动的系统会话。来源在开始、输入通知、正常 UI 更新和完成派发前核对当前区域及 AllInput 资格，不靠额外 timer 轮询。禁用、发现区域已关闭或不再允许输入、挂载退休及根关闭时，清除反馈并关闭关联 DragSession，撤销已退休业务的完成回调。FocusOnly 不改变指针拖出的资格。AppKit 活动拖放仍按 Window 合同保留 native 数据直到真实结束，不能把清理反馈记作 OS 已取消。

isDragging 是可追踪的 UI 状态，表示该行为当前持有准备中或活动中的 Window 会话，不保证系统已经确认启动。状态只在开始和终止时变化，不请求持续动画。cancelPending(): Boolean 先清除未接管的候选，再请求取消尚未启动的 Window 会话；true 仅表示取消了一个候选或未启动请求，活动系统会话返回 false。取消已准备请求的 NotDropped 结果仍经正常 owner 完成通路送达。close 幂等，撤销所有订阅和结果回调，不关闭借用区域。

创建、更新、显式取消与关闭遵守 UI owner 和评估阶段限制，不在 render／measure／scene／draw 内修改。正常完成先清除来源登记及 isDragging，再在本根普通派发边界中调用该次 onCompleted；根错误收尾复用 SurfaceWindow，不建立额外事件循环。

拖动不自动产生另一棵语义树。消费者仍应提供同一业务模型的复制／粘贴、菜单或键盘入口，并以已有 semantics 发布这些入口。拖放成功不能替代非拖动入口及辅助技术操作的验收。

## 4. 文本 Copy 接收

TextEditOptions.acceptDrops 默认为 true，Input／Textarea／OTP 及其 builder 提供同名开关并透传到宿主。macOS 的 TextEditHost 在自己的区域接入 DropArea，接收 Text／Url；文件列表和只有 Link 的来源拒绝。Copy 的基础接收遵守本节，完整 Move 扩展见第 5 节。其他平台在 Window 接收后端具备后接入同一文本实现；当前没有原生接收后端的平台不分配接收资源，不影响已有键盘、粘贴或鼠标拖选。完整系统拖放仍按 S6 单独验收。

接收资格同时要求宿主、挂载、区域有效，acceptDrops／enabled 为 true 且不是 readOnly。protectedText 和遮罩不禁止导入，但不因此开放任何导出能力。offer 只选择格式和当前落点；不读取正文、不改变文档选区、composition、焦点或历史。仅使用属于当前 TextEditorState、修订和投影的最近 Accepted 排版；重绑定、正文变化或投影变化后等待匹配的新帧，不能把旧视图的落点套在新文档上。空文本仍使用真实空段落的 caret。

接收位置显示独立的临时 caret，使用同一投影、文本槽位、滚动变换和裁剪；临时 caret 存在时不重复绘制普通闪烁 caret，但不改变原选区或向输入法发布新 caret。进入有效视口内侧 16 个逻辑像素的边缘带时，沿现有滚动模型及拖选步长自动滚动；窄视口的每侧边缘带不超过该轴有效尺寸的一半。普通拖选仍在越出视口后滚动，数据接收不要求先离开目标。每次实际滚动等待下一 Accepted 帧后更新落点；无可用残量、离开、禁用、卸载或关闭后停止 deadline。临时 caret 不触发常规焦点显露去抢回原文档光标的位置。

实际 Drop 再次确认宿主、模型身份、修订、Accepted 帧及局部位置。TextEditSource 新增 DragDrop，默认进入与 Paste 相同的导入清洗，再执行通用变换、字符与 grapheme 长度限制；保持来源可辨认，不绕过已有 paste 策略。Copy 可以接受经过清洗或截断的结果。被拒绝的 Drop 不发布候选，保留正文、选区、composition 和撤销历史；策略异常沿普通根错误边界处理。

接受时按现有输入批次在同一编辑事务中结束 marked 状态并在落点插入；旧坐标在准备阶段使用，不在 composition 结束后重新解释。只发布一次结果，实际编辑形成独立撤销项，空结果不制造冗余历史；正文及已提交回调、原生文本快照和 accessibility 通知沿既有模型路径。实际编辑生效后才请求该区域的普通焦点；业务回调关闭宿主或重绑定时，不把焦点交给新内容，已经接受的结果仍返回 true。

## 5. 文本选区拖出与移动

TextEditOptions 与 SelectableTextOptions 增加 drag: TextDragOptions。TextDragOptions 默认 enabled=true、threshold=4.0、effects=DragEffects(true, true, false)、copyPolicy=TextCopyPolicy.Default、onCompleted={}；阈值使用窗口逻辑距离。Link 不是文本移动，配置不允许它，也不允许初始空操作集。只读宿主与 SelectableTextHost 仅使用其中的 Copy，未允许 Copy 则不拖出。Input／Textarea／OTP、可选择 StyledText／Typography 及其 builder 和包装透传同一选项。

默认导出策略同时遵守宿主复制策略与 protectedText／遮罩限制；显式 Allow／Deny 单独控制拖出原文。导出开关与 acceptDrops 独立，关闭任何一端不关闭普通点击、拖选、多击、键盘编辑或复制／粘贴。正在预编辑的宿主先沿普通点击／编辑路径结束 composition，该次按下不启动数据来源。

左键单击真实选区绘制范围时先保存当前模型、正文修订与范围，保留选区；只有达到阈值后才准备文本并让原 DragSource 接管。按下位置必须来自匹配当前模型／投影的 Accepted 帧，不能把行外留白的最近字符当成命中选区。未成为拖放的点击在释放时完成普通定位，取消则保持选区；选区外、Shift 和多击继续原拖选规则。候选期间换绑、正文或选区变化使候选失效，拖动开始后单纯选区变化不改变已捕获来源范围。SelectableTextHost 导出 StyledText 的普通复制表示，保留行内对象替代文字。

每次编辑来源保存独立的进程内上下文，包含具体 TextEditorState、正文修订、范围与原文。正文修订只在实际正文变化时推进，Undo 恢复旧文字也不会恢复旧来源资格。来源正文变化、改为只读或关闭导出时收窄后续 Move 资格；已复制的文本仍可用于允许的 Copy。只读、禁用、卸载和关闭遵守原 DragSource 的业务回调退休边界。

目标通常优先 Move，平台复制修饰键或只有 Copy 可用时选择 Copy。同文档身份来自本次真实 localData 的模型引用；外部或其他来源不按文本内容推断。已知本地来源失去 Move 资格时改为可用的 Copy，否则拒绝。实际 Drop 重新确认来源、目标、策略与落点；Move 经 Paste 清洗／一般策略后必须完整保留预期移动正文，有截断或正文变换则拒绝，不能在 BOOL 接收阶段偷换 Copy。

同一文档 Move 以一次候选编辑完成源删除、落点校正、插入和最终选区，发布一次、一个独立 undo 项；落点位于原范围内（含两端）保持正文与历史。目标记录已经完成该来源移动，原生结束回调不再删除。同程序不同文档及外部目标由源在 Dropped(Move) 后检查当前宿主、模型、正文修订、范围和删除策略，成功后独立删除一次；目标已经接受而来源不能删除时保留来源。来源删除沿普通编辑命令策略执行，不把删除当成第二次 Paste；拒绝或变换不能误删其他文字。

TextDragResult 为 Copied、Moved、SourceRetained、NotDropped、Failed(WindowException)。SourceRetained 表示目标报告 Move 成功但源未完成删除，不能解释成完整移动。完成回调属于开始时的 TextDragOptions，只在来源仍有合法业务回调时调用；该回调不对已经关闭的控件复活。复制、菜单、键盘与语义编辑仍操作同一模型，移动结果沿现有正文／选区通知发布。

根停止后，焦点和文本输入仍按原合同发送终止通知，宿主可以结束 composition 并发布最终交互状态。文本来源的内部刷新遇到已退休的挂载或绑定时只收束自身资源，不再更新已关闭的 DragSource，也不重新登记来源或恢复业务完成回调。

## 6. 验证范围

Surface 以真实离屏几何覆盖路径、变换、裁剪、边界和无副作用查询。UI 逻辑测试使用真实挂载、InputArea 和资源，分别验证候选路径、最新处理器、实际接受与生命周期。完整 GUI 场景单独构建和运行；这些分层结果不能代替真实 Accepted 命中与独立进程拖放会话。
