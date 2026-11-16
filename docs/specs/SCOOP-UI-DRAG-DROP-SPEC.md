# Scoop UI 数据拖放

本文为交互打磨 S6 的 UI 数据拖放合同，复用 [Window 数据拖放](SCOOP-WINDOW-DRAG-DROP-SPEC.md)与 [Surface 命中](SCOOP-SURFACE-SPEC.md#6-区域输入与焦点)。文本事务、触摸来源和三平台系统会话继续按[统一计划](../ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)实现。

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

DragSourceData(content: DragContent, effects = DragEffects(true, false, false), onCompleted: (DragResult) -> Unit = {}) 描述本次具体数据和结果处理。普通属性更新为下一次准备替换闭包；已准备数据的 onCompleted 始终与该次来源配对，不被下一行、下一文档或下一次 render 的处理器替换。需要 Move 的消费者在准备时捕获来源及修订，并在 Dropped(Move) 后核对来源仍可删除，再提交一次删除；库不自动删除业务数据。

有数据后沿 Surface.claimPointer 接管原按下路径，再由当前 Window.beginDrag 复制表示并延后启动系统会话。子项先获得默认动作；事件被阻止或已被其他作用域接管时，来源不另建竞争会话。准备异常或 Window 启动入口异常沿正常 UI 错误边界处理，已经取得的普通 claim 会收尾。窗口系统操作发出的 Pointer.Cancelled 只结束普通点击／拖动候选，不撤销已准备的数据会话；窗口失焦、capture 结束也不能被解释成系统 Drop 失败。

DragSource.update(options, prepare) 使用最新配置和准备函数；阈值改变取消尚未达到阈值的候选，不改变已启动的系统会话。来源在开始、输入通知、正常 UI 更新和完成派发前核对当前区域及 AllInput 资格，不靠额外 timer 轮询。禁用、发现区域已关闭或不再允许输入、挂载退休及根关闭时，清除反馈并关闭关联 DragSession，撤销已退休业务的完成回调。FocusOnly 不改变指针拖出的资格。AppKit 活动拖放仍按 Window 合同保留 native 数据直到真实结束，不能把清理反馈记作 OS 已取消。

isDragging 是可追踪的 UI 状态，表示该行为当前持有准备中或活动中的 Window 会话，不保证系统已经确认启动。状态只在开始和终止时变化，不请求持续动画。cancelPending(): Boolean 先清除未接管的候选，再请求取消尚未启动的 Window 会话；true 仅表示取消了一个候选或未启动请求，活动系统会话返回 false。取消已准备请求的 NotDropped 结果仍经正常 owner 完成通路送达。close 幂等，撤销所有订阅和结果回调，不关闭借用区域。

创建、更新、显式取消与关闭遵守 UI owner 和评估阶段限制，不在 render／measure／scene／draw 内修改。正常完成先清除来源登记及 isDragging，再在本根普通派发边界中调用该次 onCompleted；根错误收尾复用 SurfaceWindow，不建立额外事件循环。

拖动不自动产生另一棵语义树。消费者仍应提供同一业务模型的复制／粘贴、菜单或键盘入口，并以已有 semantics 发布这些入口。拖放成功不能替代非拖动入口及辅助技术操作的验收。

## 4. 验证范围

Surface 以真实离屏几何覆盖路径、变换、裁剪、边界和无副作用查询。UI 逻辑测试使用真实挂载、InputArea 和资源，分别验证候选路径、最新处理器、实际接受与生命周期。完整 GUI 场景单独构建和运行；这些分层结果不能代替真实 Accepted 命中与独立进程拖放会话。
