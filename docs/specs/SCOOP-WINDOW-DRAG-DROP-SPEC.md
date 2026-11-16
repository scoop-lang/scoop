# Scoop Window 数据拖放规范

本文规定普通 Window 的原生数据边界；完整 source、UI 路由、文本事务、其他平台与设备验收仍按 [S6 计划](../ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)交付。首批实现 AppKit destination，不把该接收边界视为三个范围的完整拖放已完成。

## 1. 内容、协商与接收

DragFormat 为 Text／Url／Files，DragEffect 为 Copy／Move／Link。DragFormats 和 DragEffects 分别以三个 Boolean 保存当前实际可用格式和允许操作；contains／allows 查询对应项。DragContent 为 Text(String)／Url(String)／Files(Array<Path>)；文本和 URL 为自有 Unicode 值，文件为真实本地路径，不读取文件内容或把进程内对象编码为数据。

DropOffer 保存 formats、effects、position: LogicalPosition、modifiers: Modifiers 和 local: Boolean。位置来自接收窗口自己的内容坐标；local 只表示原生来源在本进程，不代表某个 UI 节点、同一文档或授权。Enter／Over 只取得声明的格式和元数据，不读取正文或文件列表。AppKit 首批 Text／Url 接收单个 pasteboard item；Files 要求所有项目均声明 file URL，Drop 时取得完整列表，不把多个文本项目或混合项目静默合并、截取为一个表示。

DropDecision 为 Reject 或 Accept(DragFormat, DragEffect)。目标只能选择 offer 中实际可用的格式和操作；不满足时按拒绝处理。Copy／Move／Link 的含义由实际业务实现，不能仅因枚举存在就接受某个操作。

DropRequest 保存 offer、effect 和 content。选定表示只在 Drop 时读取；读取失败或内容无法完整转换则拒绝，不调用接收业务。Drop 时重新核对当前协商与目标状态。AppKit 一旦已协商某个操作，不在返回 BOOL 的接收阶段偷偷把 Move 改成 Copy；内容检查或策略无法完整接受时拒绝，由用户按实际允许的操作重新拖放。

## 2. Window 接收资源

Darwin 提供 `Window.registerDropTarget(handlers: DropHandlers): DropTarget`。DropHandlers 保存 `offer: (DropOffer) -> DropDecision`、`drop: (DropRequest) -> Boolean` 与 `leave: () -> Unit = {}`。一个窗口同时注册一个接收资源；重复注册报告普通状态错误。它不创建输入区域、焦点或另一棵命中树，UI／其他 renderer 在 offer 中使用自己的已接受场景和逻辑路由。

DropTarget 的 handlers 可在正常 owner 更新中替换，enabled 默认 true，close 可重复调用；修改及查询遵守 Window owner。替换或禁用使旧协商失效；原生下一次更新重新询问当前处理器。禁用后通过已有延迟原生操作通路结束尚在本目标内的 offer，清理通知不在 setter 或 render 栈内重入业务。close 立即撤销 native 登记和回调，关闭窗口先关闭该资源；关闭后的旧 native 引用不能调用已释放的 Scoop context。资源关闭不再调用业务 leave，拥有者在自己的收尾中释放高亮等 UI 状态。

leave 在原生离开、结束或 Drop 收尾时至多一次地结束当前 offer；重新进入形成新的 offer。接收回调可关闭自身或窗口；已经实际接受的数据仍以回调的 true 表达，不能因随后关闭倒推为未接受。新的处理器或重新注册不接受上一份协商。

## 3. 同步边界与 AppKit 映射

AppKit registerForDraggedTypes／NSDraggingDestination 使用系统 pasteboard 的 String、URL 和 FileURL 表示。原生 getter 不进行应用编辑；offer 和 drop 经捕获注册 Context 的 Reusable ForeignCallback 执行，只在 owner、窗口 Open 且可交互、没有正在进行的业务／watch／另一回调时进入。重入直接拒绝，不等待自身队列、不递归 pump、不请求布局；异常沿正常应用失败与退出路径收尾，不跨 Objective-C ABI。

已有 deferred native operation 在普通 pump 退栈后仍处于最外层 application native callback 中；这时没有业务／watch 在执行，允许拖放同步回调及禁用后的 leave。具体只接受 nativeOperationDepth = 1 且 nativeCallbackDepth = 1 的该边界，业务 dispatch 或 pumping 期间及更深的原生回调仍拒绝。不能用 blanket 的 nativeCallbackDepth 非零检查丢弃正常延迟收尾或阻断同 owner 的系统拖放。

prepareForDragOperation 只复核元数据和已协商选择；performDragOperation 才复制选定数据。可能触发原生延迟数据提供的读取前后都核对当前登记、窗口和协商，失败不执行 drop。drop 返回 true 必须表示目标已实际接受该内容；不能以排队等待未来处理冒充完成。Files 的后续打开、上传或持久化结果属于业务自己的入口。

原生方法在 offer／drop／leave 回调或内容读取尚未退栈时拒绝新的拖放调用，不让重入的 perform 重复插入或消费外层 offer 的收尾。AppKit 可用空 sender 结束 offer；重复结束仍只通知一次。

原生关闭先清除 callback／context，再通过已有 owner control 队列在调用退栈后释放 ForeignCallback。保留旧 destination、关闭期间读取、handler 自关与迟到方法均不能访问已退休应用状态。macOS 的真实 dragging session、跨进程接收及 source Move 删除仍须独立验证，不以直接调用 NSDraggingDestination 方法替代系统会话验收。
