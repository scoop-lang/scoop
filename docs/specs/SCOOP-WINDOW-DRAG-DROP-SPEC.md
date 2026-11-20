# Scoop Window 数据拖放规范

本文规定普通 Window 的原生数据边界。Darwin、Windows 和 Linux GNU 提供同一 destination 和 source 接口，分别使用 AppKit、OLE、GDK／GTK 公共 API。接口、准备／收尾测试和实际系统会话的完成状态按 [S6 计划](../ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)分别记录。

## 1. 内容、协商与接收

DragFormat 为 Text／Url／Files，DragEffect 为 Copy／Move／Link。DragFormats 和 DragEffects 分别以三个 Boolean 保存当前实际可用格式和允许操作；contains／allows 查询对应项。DragContent 为 Text(String)／Url(String)／Files(Array<Path>)；文本和 URL 为自有 Unicode 值，文件为真实本地路径，不读取文件内容或把进程内对象编码为数据。

DropOffer 保存 formats、effects、position: LogicalPosition、modifiers: Modifiers 和 local: Boolean。位置来自接收窗口自己的内容坐标；local 只表示原生来源在本进程，不代表某个 UI 节点、同一文档或授权。Enter／Over 只取得声明的格式和元数据，不读取正文或文件列表。AppKit 首批 Text／Url 接收单个 pasteboard item；Files 要求所有项目均声明 file URL，Drop 时取得完整列表，不把多个文本项目或混合项目静默合并、截取为一个表示。

DropOffer.localData: Any? 默认为 None。只有 native 的真实 draggingSource 与本 Application 当前尚未关闭的 DragSession 对应时，才携带 beginDrag 提供的同一个进程内值；同程序的其他原生控件、外部来源及已经退休的源均为 None。它不写入 pasteboard、不序列化，也不读取外部正文。UI 用该值关联本次来源文档与范围，不能凭 local 或相同文本猜测来源。该值依普通引用规则存活，接收者保留 offer 不会延长源的原生登记。

DropDecision 为 Reject 或 Accept(DragFormat, DragEffect)。目标只能选择 offer 中实际可用的格式和操作；不满足时按拒绝处理。Copy／Move／Link 的含义由实际业务实现，不能仅因枚举存在就接受某个操作。

DropRequest 保存 offer、effect 和 content。选定表示只在 Drop 时读取；读取失败或内容无法完整转换则拒绝，不调用接收业务。Drop 时重新核对当前协商与目标状态。AppKit 一旦已协商某个操作，不在返回 BOOL 的接收阶段偷偷把 Move 改成 Copy；内容检查或策略无法完整接受时拒绝，由用户按实际允许的操作重新拖放。

## 2. Window 接收资源

三个桌面后端提供 `Window.registerDropTarget(handlers: DropHandlers): DropTarget`，允许在 PendingConfigure 或 Open 时登记和配置资源；实际拖放回调仍要求窗口 Open。DropHandlers 保存 `offer: (DropOffer) -> DropDecision`、`drop: (DropRequest) -> Boolean` 与 `leave: () -> Unit = {}`。一个窗口同时注册一个接收资源；重复注册报告普通状态错误。它不创建输入区域、焦点或另一棵命中树，UI／其他 renderer 在 offer 中使用自己的已接受场景和逻辑路由。

DropTarget 的 handlers 可在正常 owner 更新中替换，enabled 默认 true，close 可重复调用；修改及查询遵守 Window owner。替换或禁用使旧协商失效；原生下一次更新重新询问当前处理器。禁用后通过已有延迟原生操作通路结束尚在本目标内的 offer，清理通知不在 setter 或 render 栈内重入业务。close 立即撤销 native 登记和回调，关闭窗口先关闭该资源；关闭后的旧 native 引用不能调用已释放的 Scoop context。资源关闭不再调用业务 leave，拥有者在自己的收尾中释放高亮等 UI 状态。

leave 在原生离开、结束或 Drop 收尾时至多一次地结束当前 offer；重新进入形成新的 offer。接收回调可关闭自身或窗口；已经实际接受的数据仍以回调的 true 表达，不能因随后关闭倒推为未接受。新的处理器或重新注册不接受上一份协商。

`DropTarget.periodicUpdates` 默认 false；true 在活动拖放期间更新协商，使静止指针下的内容变化和自动滚动能重新路由。AppKit 使用原生 draggingUpdated；OLE／GTK 在持有当前活动 offer 时使用窗口 owner 的有限周期回调。没有活动 offer、正在接收数据、窗口不可用、禁用或关闭时停止周期回调，不安装空闲轮询。切换该提示不改变已协商的格式与操作。

## 3. 同步边界与 AppKit 映射

AppKit registerForDraggedTypes／NSDraggingDestination 使用系统 pasteboard 的 String、URL 和 FileURL 表示。原生 getter 不进行应用编辑；offer 和 drop 经捕获注册 Context 的 Reusable ForeignCallback 执行，只在 owner、窗口 Open 且可交互、没有正在进行的业务／watch／另一回调时进入。重入直接拒绝，不等待自身队列、不递归 pump、不请求布局；异常沿正常应用失败与退出路径收尾，不跨 Objective-C ABI。

已有 deferred native operation 在普通 pump 退栈后仍处于最外层 application native callback 中；这时没有业务／watch 在执行，允许拖放同步回调及禁用后的 leave。具体只接受 nativeOperationDepth = 1 且 nativeCallbackDepth = 1 的该边界，业务 dispatch 或 pumping 期间及更深的原生回调仍拒绝。不能用 blanket 的 nativeCallbackDepth 非零检查丢弃正常延迟收尾或阻断同 owner 的系统拖放。

prepareForDragOperation 只复核元数据和已协商选择；performDragOperation 才复制选定数据。可能触发原生延迟数据提供的读取前后都核对当前登记、窗口和协商，失败不执行 drop。drop 返回 true 必须表示目标已实际接受该内容；不能以排队等待未来处理冒充完成。Files 的后续打开、上传或持久化结果属于业务自己的入口。

原生方法在 offer／drop／leave 回调或内容读取尚未退栈时拒绝新的拖放调用，不让重入的 perform 重复插入或消费外层 offer 的收尾。AppKit 可用空 sender 结束 offer；重复结束仍只通知一次。

原生关闭先清除 callback／context，再通过已有 owner control 队列在调用退栈后释放 ForeignCallback。保留旧 destination、关闭期间读取、handler 自关与迟到方法均不能访问已退休应用状态。macOS 的真实 dragging session、跨进程接收及 source Move 删除仍须独立验证，不以直接调用 NSDraggingDestination 方法替代系统会话验收。

## 4. Window 拖出源

三个桌面后端提供 `Window.beginDrag(content: DragContent, effects: DragEffects = DragEffects(true, false, false), localData: Any? = None, preview: DragPreview? = None, onCompleted: (DragResult) -> Unit = {}): DragSession`。只允许在该窗口当前真实左键或受支持的直接接触输入的 owner 派发中调用，平台来源限制见第 5 节；上层负责实际拖动阈值。无对应输入、窗口不可交互或已有活动源／窗口系统操作时报告状态错误。一个 Application 同时只有一个源请求；不同目标根仍各自协商接收。

数据在返回前复制给 native。Text 发布一个原值文本项；Url 要求绝对 URL，在同一项发布 URL 与其文本表示；Files 要求非空绝对本地路径列表，每个路径发布一个 FileURL 项，不读取文件内容、检查来源或创建文件承诺。格式无法表示、空操作集及无效数据为参数错误，失败不接管普通按下序列，也不留下半份来源。源不改系统普通剪贴板，拖放 pasteboard 归系统会话。

准备成功后沿已有 SystemOperation 取消通知结束普通鼠标与接触路由，再在 owner 的原生操作边界启动；Darwin 调用 beginDraggingSessionWithItems。左键拖动保留对应 NSEvent，使阈值后的接管不用伪造按下；系统移动／调整窗口的事件限制不随此扩展改变。准备到实际启动之间的取消、模态、关窗或退出不启动会话。未提供预览时沿用各后端的默认拖放反馈。

GTK 的左键按下与按住左键的移动事件都必须将真实 GdkEvent 附件保留到 owner 派发结束；不能只保留按下而丢弃达到上层拖动阈值的移动。没有按住左键的移动、释放和取消不取得拖出输入资格，不用伪造旧按下弥补丢失的附件。

DragSession 的 isStarted 表示原生已确认开始，result: DragResult? 保存终态，isClosed 可查询；所有访问要求 owner。DragResult 为 Dropped(DragEffect)、NotDropped 或 Failed(WindowException)。Dropped 仅表示系统报告目标接受了允许的操作，绝不隐含删除来源；NotDropped 合并系统未接受、拒收及用户取消，因为 AppKit 不区分其原因。Failed 表示实际启动失败。完成结果通过普通 owner 队列交付 onCompleted 一次，交付前释放源登记；处理器仍沿正常应用错误边界执行。Copy 不删源，Move 的文档版本确认与一次删除由实际 UI／业务实现；Window 不擅自删除文字或文件。

DragSession.effects 可在正常 owner 更新中改变后续协商允许的操作，只能使用创建时提供过的操作；更新可暂时为空，表示不再允许任何目标接收。已关闭或已有终态时拒绝更新。native 查询只读取已复制的当前值；不因此重入 Scoop 或重新读取正文。原生结束回报仍按创建时的操作集合解释：如果目标已经实际完成 Move，随后收窄 effects 不能把它伪装成 NotDropped；来源最终仍须核对实际文档。

`cancelPending(): Boolean` 只取消尚未启动的请求，成功后产生 NotDropped；已启动、已有终态或已关闭返回 false。`close()` 幂等，撤销后续业务回调，关窗自动执行；尚未启动时撤销请求。AppKit 的公开 NSDraggingSession 没有强制取消方法，因此关闭已启动资源不宣称系统会话已取消，也不合成 Escape 或调用私有接口：native 保留系统仍在使用的数据与源对象直到真实 ended 回调，只清除 Scoop context。此时尚无实际终态的 result 保持 None；后续没有来源删除回调。已经保存的终态在关闭后保留。

原生 sourceOperationMask 查询只读取复制后的允许操作，不进入 Scoop。系统实际结束才报告 Dropped／NotDropped；窗口失活或普通鼠标 capture 结束本身不结束跨程序会话。关闭期间的 native 引用不能保留业务 closure，待系统结束后释放剩余原生数据。实际会话、关闭中继续拖放、同程序接收与外部 Move 均须独立桌面验收，不能用协议方法测试替代。

### 4.1 自定义预览

`DragPreview(image: DesktopImage, hotSpot: LogicalPosition)` 指定整次拖动的一张透明彩图。热点以图像左上角为原点，用逻辑单位表示鼠标抓住的位置，可以在图像之外。Window 按源窗口缩放选择像素表征，保持逻辑尺寸；原生层在准备期间取得自有像素，输入离开窗口后继续随系统会话显示。多文件仍逐项传递，整组只显示一张自定义预览。图片保持原色和 alpha，外观与 Text／Url／Files、localData 及 Copy／Move／Link 协商相互独立。

AppKit 使用 dragging item 的图像与 frame，GTK 4 使用定尺寸 paintable 与热点，Windows 使用 Shell drag image 和 OLE 源／目标辅助接口。目标窗口可按平台惯例调整反馈；外部 Windows 目标对 Shell 拖放图像的支持由目标决定。

## 5. Windows／GTK 与直接接触

以上公共入口在三个桌面后端具有相同的签名、完成、业务线程与关闭合同。Darwin 段落中的 pasteboard item 和 NSDraggingSession 限制是该后端的具体映射。Windows 使用 IDataObject／IDropSource／IDropTarget 和 DoDragDrop，应用 owner 初始化 OLE STA；GTK 使用 GdkContentProvider／GdkDrag 与 GtkDropTargetAsync，不建立第二个事件循环。

Windows 的标准表示为 CF_UNICODETEXT、UniformResourceLocatorW 和 CF_HDROP；路径采用目标系统的原生字符串，不经 UTF-8 丢失文件名。GTK 使用 UTF-8 文本和 text/uri-list；URL 接收要求一个绝对 URI，Files 要求完整列表均为本地 file URI，不能只取可转换的部分。GTK 在 Drop 后异步读取完整表示，读取期间保留该次 GdkDrop 和登记代次；完成后在同一 owner 上重新核对目标、格式和操作，再执行一次实际 drop 并向 GDK 报告完成。旧登记、禁用、关闭或取消不得向新目标交付迟到数据。读取失败拒绝，不报告成功，也不建立轮询或嵌套等待。

Windows 的 Unicode 文本和 URL 读取到 HGLOBAL 范围内的首个 UTF-16 终止符；其后的分配填充不属于正文，不要求填充为零或恰好占用完整字符。范围内缺少终止符、正文编码无效或 URL 不合法时仍拒绝。

OLE 在 DragEnter 查询并保留本次 IDataObject 的格式声明，离开或结束时清除；重新进入重新查询。DragOver 和周期更新使用这份声明，按当前坐标、修饰键、允许操作及目标状态重新协商，不对未变化的格式重复调用外部数据对象。格式查询可能发生跨 STA 调用并派发嵌套消息，查询期间保持原生重入隔离，目标被替换、禁用或关闭后不发布旧查询结果。仅 Drop 读取选定内容，读取失败仍拒绝，保留格式声明不代表已经接受数据。

本进程来源关联使用真实 native 对象身份：Windows 比较当前 IDataObject 的 IUnknown 身份；GTK 比较 gdk_drop_get_drag 与当前 GdkDrag。不向外部格式添加进程对象标识。OLE／GTK 的周期目标更新只在已进入的活动拖放及 periodicUpdates=true 时使用 owner 原生定时源，离开、Drop、禁用或关闭立即撤销；不在空闲时轮询。

beginDrag 除鼠标左键之外，也接受后端支持的当前真实 Touch／Pen Began 或 Moved。Linux 保留该输入的 GdkEvent 与设备，使用公开 GDK 拖放接口。Windows 保留真实 contact 身份，触摸或笔的 OLE 会话只从系统为该输入生成的鼠标消息处理栈启动；兼容鼠标不再交付 UI 点击。尚未抵达该入口的请求仍可 cancelPending；接触结束、设备取消或窗口退休后不补发拖动。不得合成鼠标消息、伪造事件或从 WM_POINTER 处理器直接调用 DoDragDrop。原生启动确认前 isStarted 保持 false。

数据拖放准备成功后的 SystemOperation 取消同时结束当前 UI 鼠标和 contact 路由，防止系统接管后残留按压、滚动或选区把手；它不销毁原生拖放所借用的真实输入，也不把接触转换为鼠标。第二触点、几何失效或默认动作被阻止时，上层未识别候选按普通手势合同取消。AppKit 没有直接触屏 contact 来源；触控板系统手势不是数据拖出输入。
