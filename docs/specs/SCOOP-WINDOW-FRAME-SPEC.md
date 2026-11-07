# Scoop 窗框、区域与系统交互规范

本文补充 [window 规范](SCOOP-WINDOW-SPEC.md)，落实[桌面设计第 7～8 节](../desktop-ui/DESIGN.md)。公共值不依赖 graphics、Surface 或 UI；三者只负责把自己的已接受布局转换成这些值。

## 1. 窗框模式与实际几何

`WindowFrame.System(WindowContentMode.Standard | Extended)` 由平台宿主绘制装饰；`WindowFrame.Client` 由应用绘制，应用也可以完全不绘制标题栏。创建属性 `frame` 默认 System(Standard)，运行中使用 `setFrame`。删除 decorations／setDecorations；macOS TitlebarOptions 只保留 transparent、titleHidden，fullSizeContent 合并为 Extended，不保留双轨配置。resizable、buttons、transparent、level 继续独立工作。

System 包含 GTK/libadwaita CSD 标题栏；Client 隐藏该标题栏和系统可见边框，保留现有内容 widget／renderer。Extended 保留系统按钮，让内容进入标题栏：AppKit 使用 FullSizeContentView，Win32 使用 NCCALCSIZE／DWM 原生按钮，GTK 使用 AdwToolbarView 的内容延伸。不能实现的后端或具体组合返回 Unsupported，不悄悄替换模式。

`Window.frameMetrics: WindowFrameMetrics?` 返回已配置窗口的实际 frame、相应 WindowMetrics、contentLayout（不被宿主标题栏占用的内容矩形）、systemButtons（Close／Minimize／Maximize 的真实占用矩形）和 visibleBorderThickness（未知为 None）。所有矩形均为内容左上角逻辑坐标；Standard 下按钮可能在内容之外。GTK 未分配、DWM 尚未给出按钮几何时，未知字段为空；不得填入常量标题栏高度或虚构按钮宽度。WindowEvent.FrameChanged 通知需要重新读取几何，GeometryChanged 仍描述内容尺寸与 DPI。

`Window.frameRevision: ULong` 是原生窗框／区域配置的短期版本快照。在同一窗口原生尺寸、DPI、窗框或内容偏移失效时，先更新该值并撤销旧区域，再排队 FrameChanged；即使通知尚未派发，值已反映失效。Surface 在采集和发布区域之间比较它，避免绘制回调内切换配置后重新发布旧几何。读取不重新计算按钮布局，只提供同一窗口内的相等比较，不用作跨窗口身份或持久版本。

`DesktopCapabilities.frame` 分别声明 client、extendedContent、systemResizeEdges、beginMove、beginResize、windowMenu、windowRegion、inputRegion、ignoreMouse、shadow、resizeHitRegions、systemButtonHitRegions。它描述后端提供的操作，具体窗口状态仍会影响请求；不表示 WM 已接受异步请求。

## 2. 原生外观

`setShadow(WindowShadow.System | None)` 只请求系统阴影；它不绘制应用阴影。后端不能独立控制时返回 Unsupported。透明背景与阴影互不隐式覆盖；AppKit 同步窗口 opaque 与现有 CAMetalLayer，renderer 不继续使用创建时的旧值。

platform.windows 提供 `WindowsFrameCapabilities` 与独立的边框色、标题栏色、文字色、圆角偏好 setter。颜色为 Default／None（仅边框）／Rgb，圆角为 Default／Square／Round／SmallRound；按实际 DWM attribute 探测，不用主题名或 OS 字符串猜测。DWM 边框宽度只查询，不提供任意宽度 setter。透明／region 等组合是否实际使用圆角由系统决定。

## 3. 自有区域

`WindowRegion(rectangles: Array<LogicalRect>)` 保存 Scoop 的不可变 Array 值；发布时 native 复制所需数据。矩形要求有限、非负尺寸且终点可表示；零面积不覆盖任何点。原生量化使用左／上 floor、右／下 ceil，再与可表示的 native 坐标范围核对。窗口内容坐标到外框／GtkNative surface 的转换由后端负责。

`setWindowRegion(region?)` 控制原生 bounding region，可能同时裁剪外观与输入；`setInputRegion(region?)` 只控制输入覆盖；`setIgnoreMouse(Boolean)` 控制整窗忽略鼠标。None 恢复默认，Some(empty) 表示空区域，二者不可混淆。Win32 SetWindowRgn 只实现前者，不将 HTTRANSPARENT 声称为跨进程穿透。AppKit 没有公开任意独立输入形状时明确 Unsupported；其 ignoresMouseEvents 提供整窗穿透。GTK 使用 GdkSurface.set_input_region，转换包含真实标题栏、菜单栏、阴影与 native surface 偏移。

区域在尺寸、DPI、窗框配置变化后失效：旧覆盖撤销并通知 FrameChanged，应用或下一帧以新几何重发；不把旧坐标自动拉伸到新窗口。GTK 实际分配可能重写输入区域，库在真实配置更新时协调覆盖，不靠每帧无条件重设。整窗 ignoreMouse 是逻辑属性，不随布局失效。

`setHitRegions(Array<WindowHitRegion>)` 发布 Client／Caption／Resize(edge)／SystemButton(role)；每项含 WindowRegion。原生按钮保留区域优先，其余按发布顺序先命中者优先，未命中为 Client。全部数据复制到 native，OS 回调只读取快照；不回调 managed layout。禁用缩放／最大化时不产生 resize 命中。后端没有对应原生命中能力时，对该角色返回 Unsupported，不接受后再忽略。

Win32 SystemButton 命中映射 HTMINBUTTON／HTMAXBUTTON／HTCLOSE；系统执行动作，managed 不再收到一次普通点击。WindowEvent.FrameButtonChanged(role, hovered, pressed) 提供真实非客户区反馈，离开、失焦、撤销区域时清除。系统按钮权限继续受 WindowButtons 控制。

视觉 Path 与上述二进制 region 不等价。graphics 显式利用现有 Skia region 栅格化得到矩形集合；Surface／UI 只有在实际帧 Accepted 后发布最终变换／clip 下的区域。Discarded 不发布候选。需要输入匹配形状的组件在缺少能力时失败；降级为矩形输入须由调用方明确选择。

## 4. 系统移动、缩放与窗口菜单

`beginMove()`、`beginResize(WindowResizeEdge)`、`showWindowMenu()` 必须在此窗口当前真实输入事件的 handler 内调用。窗口库保留该事件的 NSEvent／GdkEvent 或 Win32 消息上下文，派发结束即失效；业务 post、timer、构造的 PointerEvent 不产生有效上下文。无上下文抛 InvalidState，后端不支持返回 Unsupported。无需向应用暴露 serial 或引入额外凭据。

请求返回 Requested；它表示已把有效请求交给原生系统，不保证 WM 完成移动／缩放。可能进入嵌套原生循环的操作在 handler 和 session pump 退栈后执行；队列持有必要窗口及事件使用，关闭时取消尚未执行的请求，释放发生在真实使用结束之后。原生嵌套循环继续推进既有 I/O／有限事件批次，不递归进入同一个 managed pump。

系统接管时以 PointerCancelReason.SystemOperation 结束旧按下序列，后来的释放不得触发残留 click。AppKit 使用 performWindowDragWithEvent，公共 beginResize 在无公开等价 API 时 Unsupported；这与 AppKit 自带窗口边缘缩放能力分别报告。GTK 调用 GdkToplevel 的 begin_move／begin_resize／show_window_menu，使用原事件 device、时间、surface 坐标，由 GDK 保持 Wayland serial。Win32 使用系统 move／size／menu 与实际非客户区语义。

## 5. 验证

普通值与 API 行为在 window Cone 测试；native helper 仅用于驱动和观察实际系统行为。覆盖模式切换保留内容宿主、输入／bounding 区域默认与空集、负坐标及 DPI、配置变化撤销、真实按钮几何、命中优先级和取消序列。Surface 另验证透明 renderer 与 Accepted／Discarded 提交。Windows SSH 只构建完整 GUI 用例并运行无需桌面的测试；真实交互留桌面会话验收，记录不得将二者混淆。
