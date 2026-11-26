# scoop.window

iOS 的 `native-app` 在 `appInit` 创建 `scoop.window.ios.UIKitApplication(handler)`。场景代理在主线程转发 `connect(scene)` 和 `disconnect(scene)`。在 `ApplicationEvent.SceneConnected(window)` 中取得窗口，按 `window.metrics` 的逻辑尺寸、显示比例和 `safeAreaInsets` 布局。跨线程操作通过 `application.proxy().post` 投递。[可运行的 UIKit 宿主](../../../tests/ios-native-app/window/)

iOS 的复制粘贴使用 `window.clipboard` 或 `application.clipboard`。用户执行粘贴时再调用 `readText()`，可获取 UIKit 通用剪贴板上的纯文本。复制时可用 `writeTexts` 将纯文本和 HTML 等表示放在同一个系统条目中。iOS 的 `primarySelection.isAvailable` 返回 false。[UIPasteboard 的条目与表示](https://developer.apple.com/documentation/uikit/uipasteboard/items)

通过 `window.textCommandAvailability` 将当前选区可执行的命令交给 UIKit。系统的编辑动作进入 `WindowEvent.TextCommand`，由文档模型执行一次。可选择的只读文字也可以发布复制动作。

iPad 的外接键盘经 `WindowEvent.Keyboard` 交付。快捷键通常绑定 `ShortcutKey.Logical` 并设置 `primary = true` 表示 Command。需要固定键盘位置时使用 `ShortcutKey.Physical`，未知物理键保留其 USB HID usage。[UIKit 的键码](https://developer.apple.com/documentation/uikit/uikey/keycode)

PRIMARY 使用 `application.primarySelection`／`window.primarySelection`，独立于普通 Clipboard；先查 `isAvailable`，`claimText` 返回可关闭的拥有凭据，旧凭据不会清掉后来者。GTK 仅在真实 X11／Wayland 协议存在时提供它，macOS／Windows 返回不可用。`application.textServices` 提供只读的拼写／语法／替换结果和平台文本 Services；范围以传入正文的 UTF-8 坐标表示。`window.showTextDefinition(text, anchor)` 和 `requestHapticFeedback` 分别请求实际可用的词典与触觉入口。能力、生命周期和限制见[文本服务核查](../../../docs/ui/interaction-polish/TEXT-SERVICES.md)。

原生文本服务菜单使用 `MenuItemDescription("Services", MenuItemKind.TextServices(selectedText, acceptReplacement))`，先确认 `application.textServices.capabilities.services`。这一系统子菜单仅用于窗口 popup，每个 Menu 最多一个，子项由系统在展示时生成。服务完成后，父项的 `MenuActivation.textServiceResult` 返回正文：None 没有正文，Some("") 是空替换。调用方核对原文档、修订与选区后自行应用；只读目标传 false，禁止返回替换。不提前枚举系统菜单，不把返回值直接写入另一份编辑模型。已知名称的显式调用仍可使用 `performService(name, text)`。

面向上层 GUI／renderer 的窗口骨架，能力边界参考 winit 0.30.13。已实现 macOS AppKit、Windows Win32，以及 Linux GNU 的 Wayland／X11 后端。

库负责窗口、原生事件、重绘请求、显示器快照、原生菜单和显式生命周期，也提供 GUI 文本剪贴板与低层无障碍投影。控件、文档布局、语义内容和图形设备由上层实现。窗口库和 `scoop.window.asyncio` 都是普通 Scoop Cone，AppKit 薄层通过普通 `.m`／ARC 接入，Win32 和 GTK 薄层使用 C／C++ native 构建。

## 依赖与入口

```toml
[dependencies]
"scoop:scoop.window" = "0.1.0"
```

在主线程、活动 Job 之外创建 `Application`，以 `run(handler)` 驱动 `ApplicationHandler.onEvent`。一个进程只能创建一个 Application，并运行一次。`Resumed` 中使用 `ActiveApplication.createWindow`；原生关闭按钮产生 `CloseRequested`，由应用决定销毁窗口或退出。

默认关闭最后一个窗口后继续运行；使用 `ApplicationOptions(LastWindowPolicy.Exit)` 可在最后窗口实际销毁后退出，也可以主动调用 `ActiveApplication.requestExit()`。运行结束会返回调用方。

[Metal 示例](examples/metal/README.md)和 Windows [DXGI 示例](examples/dxgi/README.md)给出了完整可运行入口和独立 renderer Cone。准备好[开发 sysroot](../../../scripts/README.md)后，在仓库根运行：

```sh
export SCOOP_SYSROOT="$PWD/sysroot"
target/debug/scoop run sysroot/src/scoop.window/examples/metal/app --release
```

## 当前能力

| 能力 | 接口与行为 |
| --- | --- |
| 主循环 | Wait、Poll、单调 WaitUntil；跨线程 `ApplicationProxy.post`／`requestExit`；回调串行交付 |
| 定时回调 | `Window.createDeadline` 创建窗口所属的一次性回调；共用主循环最近期限，关闭窗口即撤销 |
| 系统交互偏好 | `interactionPreferences` 读取已发布快照，`observeInteractionPreferences` 观察变化；未知值与明确关闭分开，不轮询系统 |
| 窗口 | 独立／Owned 顶层窗口、Tool 角色与一次 raise；标题、显示／隐藏、焦点请求、尺寸及约束、桌面位置、窗框及按钮、窗口级别、最小化／最大化、主题、透明／模糊、内容共享提示、注意力请求 |
| 全屏 | `window.requestFullscreen(true)` 进入、`window.requestFullscreen(false)` 退出、`window.isFullscreen` 查询实际状态 |
| 模态 | 从隐藏窗口创建 ModalSession；Owner／Application 输入范围、SeparateWindow／Sheet 能力、嵌套、取消与原生使用退休 |
| macOS | activation policy、titlebar 属性、bundle identifier 查询 |
| 显示器与 DPI | 拥有数据的显示器快照、可选名称／桌面位置／刷新率；窗口完整 logical／physical／scale metrics |
| 输入 | 物理键、当前布局逻辑键、重复、左右修饰键、未知键码、指针位置／按钮、序列取消、行／像素滚轮及 momentum、单调输入时间、基础光标和区域内隐藏 |
| 文本 | 普通文字与 IME 共用编辑批次；marked／preedit、commit、候选窗位置、失焦及禁用时撤销组合 |
| 无障碍 | `scoop.window.accessibility` 的属性、状态、关系、文本／范围、集合与真实动作；AppKit、Windows UIA、Linux AT-SPI 共用模型，具体构建、客户端和设备验证范围见[实施记录](../../../docs/ui/interaction-polish/PROGRESS.md) |
| 绘制 | 合并重绘请求；隐藏／遮挡／零尺寸时保留 dirty；显式 native lease |

`Window` 操作和原生句柄查询都要求 owner 线程。`RequestStatus.Requested` 表示已向系统提交请求，实际结果由事件或快照确认；例如焦点请求不保证系统一定授予焦点。

`window.createDeadline { ... }` 返回可反复 `schedule(time)`、`cancel()` 的 `WindowDeadline`，time 是 MonotonicInstant，例如 `MonotonicTimeSource.now() + Duration.seconds(1L)`。已经到期的时间也在后续主循环批次交付；重排、取消或关闭使旧的排队回调失效。回调在窗口隐藏时仍能执行，但不会隐式请求重绘。内部定时与应用的 WaitUntil 独立，只有后者实际到期才发出 StartCause.Deadline；不需要为每个组件另建线程或改写应用的控制流。

内容区坐标以左上为原点。`LogicalPosition`／`LogicalSize` 使用窗口逻辑单位，`PhysicalPosition`／`PhysicalSize` 使用像素。使用同一次 `WindowMetrics` 或输入事件的 scale 换算；桌面坐标不能乘某个窗口的 scale。未知的原生信息保留为 `None`。

`PointerState.time`、`KeyEvent.time` 和 `PointerEvent.Cancelled` 携带 `InputTime`，其 `mark()` 直接返回与帧和窗口 deadline 共用的 MonotonicInstant。当前后端使用原生接收边界的 `Observed`；它区别于可可靠映射的原生发生时间 `Event`，不在业务派发时重新采样。失焦或原生捕获／跟踪中断仅取消仍活动的鼠标序列；正常离开内容区继续拖动，取消后的迟到 Up 被忽略。

`PointerEvent.Button` 的最后一项是至少为 1 的连续点击次数，释放沿用对应按下的次数。AppKit／GTK 使用原生 count；Windows 按系统的时间、双击矩形和拖动容差补齐三击及更多次点击，正常释放鼠标捕获不打断序列。失焦、取消、按钮变化、超时、点击越界或实际拖动结束旧计数；不要在每个编辑控件再用固定时间／距离重算。底层计数定向测试位于 `tests-cones/clicks`，实际 Win32 和 GTK X11 输入用例分别位于 `tests-cones/input`、`test-linux`；平台通过范围见 [UI 实施记录](../../../docs/ui/interaction-polish/PROGRESS.md)。

滚轮的 `PixelDelta` 使用事件 scale 对应的物理像素，`LineDelta` 不随 DPI 缩放；滚动与 momentum 各自保留开始、更新、结束或取消阶段。没有原生阶段的普通 wheel 使用 `ScrollPhase.None`。平台适配在 `native/<platform>/input/` 内完成转换、采集和资源管理，公共层只解码 Window 自己的载荷。触点与自定义手势的分层边界见[输入边界方案](../../../docs/ui/interaction-polish/INPUT-BOUNDARY.md)，当前未新增自定义手势 API。

`WindowEvent.Gesture` 提供 AppKit 已识别的 Magnify／Rotate／Swipe。前两者保留独立开始、更新、结束／取消相位，None 是单次事件；Magnify 的实际倍率为 `1 + delta`，Rotate 为顺时针弧度。Swipe 是向右／下为正的离散方向，不能当作像素位移。位置复用 PointerState，手势不改变鼠标按钮或伪造接触身份。焦点、模态及系统操作中断后不会因迟到 Changed 恢复；示例与上层组合见 [UI 手势示例](../scoop.ui/examples/gestures/README.md)。

`WindowEvent.Contact` 提供 Began／Moved／Ended(ContactState) 和 Cancelled(id, reason, time)。ContactId 在窗口内一次实际接触中稳定，原生值之后可复用；kind 区分 Touch／Pen，PointerState 保留逻辑／物理位置、缩放、修饰键、时间和可选 device，pressure 仅在系统提供规范化值时存在。单触点取消不结束其他触点或鼠标序列，关窗、失焦、模态和系统操作会统一取消。

Windows 从 WM_POINTER 和对应查询复制触点，整条消息路径一致交回 DefWindowProc，并过滤 UI 的兼容鼠标副本；GTK 使用原始 GdkEventSequence／笔工具，通过捕获阶段的 GtkEventControllerLegacy 避免 GtkGestureClick 重复交付。AppKit 继续交付已识别的触控板手势，不把 NSTouch 设备坐标假装成窗口内直接触摸。Windows／GTK 原生实现和真实设备验收范围见[实施记录](../../../docs/ui/interaction-polish/PROGRESS.md)。

独占全屏／显示模式切换、全局 raw device、更多应用手势识别、光标捕获／自定义光标和复杂平台窗口交互属于后续范围。数据拖放已提供 AppKit／OLE／GDK 来源与目标、UI source／target 及文本事务；各平台的实际验证范围见交互实施记录。完整映射见 [WINIT-MAPPING.md](../../../docs/windowing/WINIT-MAPPING.md)。

## 全屏

从 `scoop.window` 导入 `requestFullscreen` 和 `isFullscreen`。在窗口 owner 线程，对已显示的独立普通窗口调用 `window.requestFullscreen(true)`，退出时传 `false`。请求返回后，通过 `window.isFullscreen` 读取系统确认的状态。应用可以在收到 `WindowEvent.FrameChanged` 时重新读取它，随后更新按钮和布局。全屏使用窗口所在的显示器，退出后恢复窗口布局。AppKit 完成过渡前，正在进行的同向请求可重复提交，反向请求会抛出 `WindowException`（`ResourceBusy`）。

`scoop.ui.component/examples/native-desktop` 演示从 UI 发起切换并显示观察到的状态。在仓库根目录用 `scoop test sysroot/src/scoop.window/tests-cones/fullscreen --release` 运行桌面测试。Linux 可分别设置 `SCOOP_WINDOW_BACKEND=wayland` 和 `x11`。

## 系统交互偏好

在活动应用的 owner 线程，通过 `application.interactionPreferences` 或 `window.interactionPreferences` 读取 `InteractionPreferences`：自动隐藏滚动条、减少动画、高对比度、减少透明度、独立文字缩放、光标闪烁间隔／物理宽度／宽高比和双击间隔。`None` 表示没有可用的系统值，`Some(false)` 表示系统明确关闭；UI 可以为未知值选择明确默认值，不能据此伪造系统设置。

`observeInteractionPreferences { value -> ... }` 返回需要显式关闭的 Closeable。登记不触发初次回调；收到实际变化时先发布整个快照，再在普通 owner 派发中通知观察者和发送 `ApplicationEvent.InteractionPreferencesChanged`。回调中可以注销、登记或退出，重复值不通知。从 Window 登记的观察仍属于应用，使用者应在自身卸载时关闭；应用退出会清除所有登记。读取已取得的值快照不需要窗口或 owner 继续存活。

`caretBlinkInterval` 是一次显隐切换的间隔，零为常显；GTK 的整周期已除以二。`caretWidthPixels` 仍是物理像素，使用时按实际窗口 scale 转为逻辑宽度；GTK 如有公开宽高比则通过 `caretAspectRatio` 给出，不猜测字号。`textScaleFactor` 独立于 DPI。AppKit 没有公开独立数值的 caret／文字缩放保持 None；Windows 按实际可用的 UISettings 接口提供附加值，GTK 使用 4.18 基线的公开设置。原生通知经现有事件循环交付，无需安装轮询 timer。

## 无障碍内容投影

数值使用 `SemanticValueRange`，进度未知时保持 None。静态容器用 `selectionMode = Some(SemanticSelectionMode.Single)` 或 Multiple 声明真实选择模式，required 指定必选；虚拟集合改用 `SemanticCollectionSelection` 的稳定 keys，两者不同时设置。`SemanticGrid` 描述稳定列及按 key 读取的单元格，静态表格使用 `SemanticTableSize`／`SemanticTableCell` 的实际坐标和跨度。描述查询不创建条目 View。

树使用 `SemanticRole.Tree` 和先序展开行的 `SemanticCollectionSnapshot`，多列时附加原有 `SemanticGrid`。行描述用 `parentKey`、`hierarchyLevel`、`siblings = Some(SemanticItemPosition(index, count))` 表达父项、深度与同级位置，用 `checked` 和 `busy` 发布实际模型状态；叶项不声明 expanded。`SemanticCollectionActions.setExpanded`／`setChecked` 直接调用原模型，Mixed 的勾选动作请求 On。查询和动作无需创建离屏控件，模型拒绝时保留原值；删除后同 key 的新项不会复活旧代理。AppKit 使用 Outline 的披露关系，多列单元格和表头沿同一 grid 接口查询。

`SemanticRelations.activeDescendant` 可以引用 Node 或集合 Item，和实际键盘焦点分开；rowHeaders／columnHeaders 指向真实表头。macOS 虚拟表格单元格优先返回其已挂载节点的有效 `columnHeaders`，支持多级表头；未挂载或没有有效关联时使用 `SemanticGridColumn.header`。节点卸载或关联表头隐藏后，原生查询不保留旧关联。`invalid`、`busy`、`modal` 与 `headingLevel` 发布真实状态。`live` 默认为 Off，Polite／Assertive 合并同一节点的最新公告；首次挂载不自动播报，节点或祖先 busy 时延迟，隐藏、模态背景和受保护内容抑制公告。需要明确通知时调用 `node.announce(...)`。

需要保留文本范围的自绘宿主可将 `SemanticTextDocument` 绑定到 `node.textDocument`。它只保存端点和版本，普通编辑用现有偏移映射更新，全文换绑用 reset，关闭后旧范围不可用；UI 的 TextEditHost 自动维护它。`textChildren` 显式关联正文中的 UTF-8 区间与实际语义后代，供内联操作和原生文本内对象查询。`revealText(range, SemanticTextScroll(...))` 沿实际文本几何显露范围，不修改选区。

Windows 查询通过 HWND 内容根与 UIA pattern 暴露；Linux 通过 GTK 公开的 AT-SPI socket 接到原生窗口树，复用原 GMainContext。Wayland 无法提供全局屏幕原点时，该坐标查询返回 NotSupported；D-Bus 不能表达正文中的 NUL，也不会截断后冒充完整文本。完整合同见[窗口无障碍规范](../../../docs/specs/SCOOP-WINDOW-ACCESSIBILITY-SPEC.md)，平台验证范围见[实施记录](../../../docs/ui/interaction-polish/PROGRESS.md)。

导入 `scoop.window.accessibility.*`，对 PendingConfigure 或 Open 的 Window 调用 `createAccessibilityProvider()`。首次配置前即可登记和发布语义，不必先显示窗口或取得 drawable；原生动作仍要求 Open。每个窗口同时只附着一个 provider；用 `createNode(role, parent)` 创建语义资源，`update(properties, actions, relations)` 替换事实、真实回调与关系，后两项默认空值；`reparent(parent, order)` 设置阅读顺序。不同节点的 id 不复用，角色固定；需要更换角色时关闭旧节点再创建。macOS 使用 AppKit，Windows 使用 UIA，Linux GNU 使用 GTK 内容容器内的 AT-SPI 子树；实现与平台验收的状态分别记录。

角色覆盖文本、按钮、选择控件、滚动、图像、Link／Heading、数值、菜单／Tab／Dialog、List／Table／Row／Cell／表头、Tree／TreeItem、Disclosure 与 Separator。Image 使用替代名称和实际几何，不隐含点击或焦点动作。`SemanticProperties` 的可选名称、说明、标识和值区分未知和已知空串；selected／expanded／checked 只发布实际适用的状态，protectedText 不允许发布 value 明文。hidden 排除整个子树并保留身份。`SemanticActions` 只公布实际回调；setNumber／increment／decrement、setSelected、setExpanded、dismiss 与文本／滚动动作均进入已有控制器。回调只在业务已执行后返回 Completed，拒绝写入保持真实旧值；`provider.focused` 由实际键盘焦点更新，不表示焦点请求已经成功。

Text／TextField 的已知非保护 value 同时提供纯文本正文；`selection = Some(Utf8Selection(anchor, cursor))` 使用正文中的 UTF-8 边界，保留公共选区方向。AppKit 将其映射为 UTF-16 单范围，提供正文长度、选中文本、范围读取和无额外样式的 attributed string；越界或切断 surrogate 的请求返回无结果。`setSelection` 可用于只读文本，`replaceText` 只替换仍匹配的已发布选区并遵守只读状态；修改后仍需发布真实结果。选区变化不清除节点几何，正文变化使旧文本范围写入失效。`cursorAffinity: SemanticTextAffinity` 默认 Downstream，保留活动 cursor 在软换行边界的实际偏向；AppKit 光标行号读取该偏向，按 index 请求则默认 Downstream。受保护文本不提供正文、长度或选区，但可以提供实际允许的全文 setValue。

更新后调用 `publish()`，原生 getter 仅读取已发布状态。`node.geometry` 是可选的已接受布局几何，使用窗口内容逻辑坐标；可选 visibleBounds 描述自身裁剪，零面积不算可见。名称／值改变会清除旧几何，匹配布局可用后再设置；不能以外接矩形中心代替精确可点击点。发布不触发绘制，无变化不重复通知。选区的 anchor／cursor 对调即使仍覆盖同一 NSRange，也发送一次选择通知；单独方向或偏向变化不发送正文通知。

自绘宿主可用 `@Unsafe node.setGeometry(geometry, hitTest, order = 0L)` 同时提供匹配的原生命中快照与场景顺序。`scoop.window.accessibility.native.NativeSemanticHitTest` 包含 context、retain、release 和 contains；函数必须是配套的原生只读实现，不进入 Scoop 或 UI 状态。节点和已发布投影分别保留引用，替换、内容失效及关闭时释放。普通 geometry 赋值只保留矩形，并清除旧命中和文本几何。AppKit 按场景顺序查询，支持路径孔洞、裁剪、ignored 祖先映射和独立后代；阅读顺序保持不变。UI 已使用 graphics 的 withNativeGeometry 接入这条路径，普通 UI Element 直接使用 SceneContext.semantics。

自绘文本宿主可在 `setGeometry` 后用 `@Unsafe node.setTextGeometry(Some(queries))` 安装与正文和 Accepted 场景一致的原生只读查询。`NativeSemanticTextGeometry` 同属 `scoop.window.accessibility.native`，提供 UTF-8 字符／词／视觉行／逻辑行、行号、按点范围、逐片矩形和精确可见范围；节点及已发布投影各自保留原生引用。AppKit 负责 UTF-16 和屏幕坐标转换，范围查询不进入 Scoop；普通几何替换、内容失效或关闭同时撤下查询。Some 需要 Text／TextField、已知非保护 value 及节点几何；None 单独清除文本查询。UI 的两个文本宿主已自动接入，普通 UI 调用方无需自行准备函数表。

`SemanticRelations(labelledBy = [label.id], describedBy = [hint.id], errorMessages = [error.id], controls = [target.id])` 使用同一 provider 的活动节点，保持数组顺序，替换时拒绝重复与自身引用。显式 name／description／error（包括空字符串）优先，否则由对应标签文字解析；静态 Text 的非受保护 value 可以被用作标签文字，未知与已知空文字分开。实际采用的名称依赖环在 publish 时拒绝，保留之前的原生投影，可修正后重试；显式名称已覆盖的依赖不构成实际名称环。目标改名更新依赖者，关闭目标清理引用；`node.relations` 返回清理后的关系。名称依赖变化同样清除旧几何并递增 `contentRevision`，调用方不能用旧版本场景重新发布位置。hidden 标签可贡献显式引用的文字，但不恢复到导航树中。

`node.contentLabels = [text.id]` 提供独立的内容名称回退，优先级低于显式 name 和 labelledBy 的已知文字，普通 update 不清空它；内容引用同样按身份维护和清理。调用方用 `SemanticProperties(ignored = true)` 忽略当前节点并展开其后代，用 hidden 排除整棵子树。忽略／恢复保留原身份，旧引用不能执行被忽略节点的动作；后代仍受祖先禁用和隐藏约束。Window 不因 contentLabels 自动忽略目标，UI 的 mergeLabels 按实际内容决定合并。

修改已有属性可用 `properties.copy { busy = true }`，receiver 为 SemanticPropertiesBuilder；未指定的字段完整保留，返回新的不可变值。复制不发布变更，调用 `node.update` 时仍统一验证实际角色与属性组合。

这些资源要求 owner 线程。原生动作在普通 owner 回调之间同步执行，重入返回 Busy，不递归 pump 或布局；异常沿应用现有退出路径报告。节点 close 退休子树，provider close 退休全部节点；窗口进入 Closing 自动关闭 provider。客户端持有的旧原生对象不再执行动作；重新附着产生新身份。

Switch／ToggleButton 在 AppKit 使用 Button 加 Switch／Toggle 子角色，Press 进入 toggle；RadioButton 的 Press 进入 invoke 选择，RadioGroup 保留实际父子关系与阅读顺序。重复选择是否写入由真实控件决定，查询不改变值或键盘焦点。

`SemanticProperties.required` 默认 false，用于发布实际必填要求；AppKit 以公开的只读 Required 属性提供它。更新不改变正文版本、几何或焦点，也不替应用执行校验；其他状态与动作仍按各自真实条件判断。

`SemanticActions.showContextMenu` 映射 AppKit 的 accessibilityPerformShowMenu，只有存在回调且当前输入允许时才公布能力；只读不禁止实际可用的菜单。回调建立绑定目标的真实菜单会话后返回 Completed，后续菜单项执行沿该会话，不合成右键，也不在回调中 pump tracking。原生菜单的项目由系统自行提供语义。

`SemanticActions.reveal` 请求滚动显露目标，保持键盘焦点和文本选区。只有匹配当前内容的 Accepted 几何已经可见才返回 Completed，需要后续帧时返回 Busy；不能把登记定位请求当成显露完成。AppKit 通过公开的 NSAccessibilityCustomAction 提供“Scroll to Visible”，动作列表查询只读，保留旧动作不保活窗口；BOOL 回调仅在 Completed 时成功。该入口适用于普通文本和控件，不要求可写或 ScrollBar 角色。

外部 AX 客户端应读取实际结果：AppKit 可能在禁用动作拒绝后仍让 AXUIElementPerformAction 返回 kAXErrorSuccess，标准 NSButton 也有相同行为。库的 BOOL 与 SemanticActionResult 仍反映实际执行；焦点对象通过应用的 AXFocusedUIElement／AXFocusedWindow 核对，值写入、禁用与退休通过当前属性及业务结果判断。

`SemanticRole.ScrollBar` 使用 `SemanticProperties.scroll = Some(SemanticScrollRange(orientation, offset, viewport, content))` 描述一个真实轴；方向为 SemanticOrientation.Horizontal／Vertical，尺寸与位置使用逻辑单位。AppKit 读取 Orientation 和 0～1 的 Value／MinValue／MaxValue，父节点的横／纵滚动条属性连接对应子项。setScrollPosition 接收有限的 0～1 比例，scrollBackward／scrollForward 使用实际控件步长；调用方登记请求后，仍从已接受的布局发布新位置。没有范围时不提供能力，视觉隐藏不必退休逻辑轴，缺少场景登记时不提供条的点击几何。

`createAccessibilityProvider` 在 Darwin、Windows 和 Linux GNU 目标公开。UI scope／Accepted 场景、内容合并、精确命中、内置控件、文本范围、激活点和虚拟集合已通过公共资源接入，平台验收按[统一计划](../../../docs/ui/interaction-polish/PLAN.md)继续。正式合同见[无障碍接口规范](../../../docs/specs/SCOOP-WINDOW-ACCESSIBILITY-SPEC.md)，可运行的属性、关系、动作和生命周期用例见 [accessibility 测试 Cone](tests-cones/accessibility/README.md)，编辑模型组合见 [UI 文本测试](../scoop.ui/tests-cones/accessibility-text/README.md)。进程内原生方法、外部客户端和屏幕阅读器结果分别记录在[实施记录](../../../docs/ui/interaction-polish/PROGRESS.md)，不能互相替代。

逻辑列表使用 `SemanticRole.List` 与 `node.collection = Some(SemanticCollectionSnapshot(source))`。source 提供同一不可变快照的 count、稳定的 `SemanticItemKey`、反向索引和数据描述，查询不构造控件。`itemAt`／`itemForKey`／`find` 明确区分不存在、不支持和数据未知；名称与 automationId 搜索须显式声明能力。AppKit 可以按范围取得 rows；未实现条目没有估算坐标。已挂载行用 List 下的 Group 及 `collectionItem` 关联真实几何与子控件，回收 Group 不退休逻辑条目。移动 Group 到另一个父节点会清除原集合关联。重排保持已导出代理，删除使其永久失效；AppKit 将按需产生的行、单元格和列代理保留到逻辑实体退休或 provider 关闭，避免跨进程 AX 引用在查询后失效，不为此预建 View。`collectionActions = SemanticCollectionActions(realize, reveal, focus)` 登记明确操作，查询仍只读。AppKit 提供 Realize Item／Scroll to Visible 自定义动作；只有真实能力已准备或 Accepted 场景已显露时才报告完成，等待期间为 Busy。条目聚焦须同时声明 `focusable = true` 并提供真实回调；公开 setter 依据真实焦点执行。其他平台与读屏器验收继续按 S7／S10／S11 记录。

## 多窗口、模态与窗框

在 Resumed 中可以多次调用 ActiveApplication.createWindow；这些窗口共用一个 Application 和事件循环，以 ApplicationEvent.Window(id, event) 区分事件。它们各自重绘和销毁，默认是独立的非模态（modeless）顶层窗口。最后一个窗口实际销毁后的行为仍由 LastWindowPolicy 决定。

通过 windowAttributes 的 relation = WindowRelation.Owned(parent) 建立固定的顶层窗口归属；role = WindowRole.Tool 请求平台支持的工具面板角色。隐藏或最小化 owner 时暂时隐藏其后代，恢复时保留显式隐藏的意图；销毁 owner 会先关闭后代，并等待各自的 native lease 退休。AppKit 隐藏 child 时原生 parent 关系会暂时撤销，逻辑 owner 不改变，显示时重新建立关系。

先创建 visible = false 的窗口并挂载内容，再调用 window.showModal(ModalOptions(scope, presentation))。Owner 只阻塞直接 owner，Application 阻塞其他应用窗口；两者与非模态 Tool、全局 AlwaysOnTop 各自独立。session.dismiss() 正常结束并隐藏窗口，cancel() 取消，均不销毁所借 Window；结果保存在 session.result，并通过 ApplicationEvent.ModalCompleted 通知。Window.isInputBlocked 和 InputBlockedChanged 可供内容宿主同步交互状态。

desktopCapabilities().windows 分别报告相对叠放、工具角色、raise、层级传播和模态展示组合。Linux transient 关系是 WM／compositor 提示，Wayland 不提供通用工具窗角色、无激活 raise 或 topmost。AppKit 的 Automatic Owner 展示采用普通 sheet，Application 展示使用原生应用模态；不使用 critical sheet 伪装普通嵌套。详细合同与测试入口见[模态规范](../../../docs/specs/SCOOP-WINDOW-MODAL-SPEC.md)和[模态测试说明](tests-cones/modal/README.md)。

## 原生文件与目录面板

ActiveApplication.openFile／openFiles／saveFile／selectDirectory 返回 FileDialogSession<T>，分别交付 Path 或非空 Array<Path>。目标必须明确传 DialogTarget.Owner(window) 或 DialogTarget.Application；后者也支持没有业务窗口的应用。文件面板与窗口模态共用输入限制和关闭依赖，嵌套面板结束不会解除外层模态。

```scoop
val request = application.saveFile(
    DialogTarget.Owner(window),
    SaveFileOptions(
        suggestedName = Some(OsString.fromString("report.txt")),
        types = FileTypes.Extensions(["txt"])
    )
)
```

通过 ApplicationEvent.FileDialogCompleted(id) 找到对应请求，读取 request.result 的 Accepted／Cancelled／Failed。cancel() 请求取消原生展示，原生调用和回调退休后才完成 session 并释放 owner；用户取消与原生失败分别表示。路径复用 scoop.platform 的 Path／OsString，保留本地编码。保存面板只选择目标路径，应用随后执行文件 I/O。

desktopCapabilities().fileDialogs 报告入口和选项能力。AppKit 使用 NSOpenPanel／NSSavePanel，Win32 使用 Common Item Dialog，Linux 使用已有 GTK/GIO 的 GtkFileDialog，不引入托盘依赖。详细合同与测试入口见[文件对话框规范](../../../docs/specs/SCOOP-WINDOW-DIALOGS-SPEC.md)和[测试说明](tests-cones/dialogs/README.md)。

创建时使用 frame，运行中用 setFrame 切换 System(Standard／Extended) 与 Client。Client 去除宿主可见装饰，resizable／buttons 仍独立控制系统操作；System 的 Extended 保留真实系统按钮并让内容进入标题栏。frameMetrics 给出实际内容可用矩形和按钮占用，未知值为 None。desktopCapabilities().frame 分别报告窗口裁剪、独立输入区域、整窗忽略鼠标、系统拖动／缩放与阴影等能力。setWindowRegion、setInputRegion 和 setHitRegions 使用内容逻辑坐标中的自有矩形集合；None 恢复默认，空区域没有覆盖。系统 beginMove／beginResize／showWindowMenu 必须在当前真实输入事件的 handler 内调用，阻塞操作在回调退栈后执行，接管前取消原指针序列。详细平台边界见[窗框规范](../../../docs/specs/SCOOP-WINDOW-FRAME-SPEC.md)。

macOS 的 TitlebarOptions 通过 setMacOSTitlebar 设置标题栏透明和隐藏标题；内容延伸统一由 WindowFrame.System(Extended) 表达，Client 窗口仍可接收键盘焦点。Windows 的边框色、标题栏色、文字色与圆角偏好在 platform.windows 中分别查询 DWM 能力和设置。当前接口见[窗口规范第 4.1～4.3 节](../../../docs/specs/SCOOP-WINDOW-SPEC.md#41-多窗口)。

## 接收原生拖放

Darwin、Windows 和 Linux GNU 的 PendingConfigure 或 Open 窗口通过 `window.registerDropTarget(DropHandlers(offer, drop, leave))` 注册一个接收资源；声明从 `scoop.window` 导入。首次配置前可以登记和配置，实际拖放回调仍要求 Open。DropOffer 提供格式、操作、接收窗口逻辑位置、修饰键和 local；local 仅表示来源在当前进程。localData 仅在原生来源确为本 Application 当前 DragSession 时保留 beginDrag 提供的同一个进程内值，默认 None，不写入 pasteboard。offer 返回 Reject 或 Accept(format, effect)，只能选择实际可用的 Text／Url／Files 和 Copy／Move／Link；接收方自行命中自己的场景。

Enter／Over 和 prepare 不读正文。真正 Drop 时才复制所选表示为 `DragContent.Text(String)`、`Url(String)` 或 `Files(Array<Path>)`，并调用 `drop(DropRequest): Boolean`。true 表示已实际接受；尚在排队的业务不能返回 true。单项可同时提供文本和 URL；首批不合并多个文本项目。Files 要求完整的本地路径列表，拒绝混合或部分可读项目，不读取文件正文。文本保留原值，编辑目标自己的导入策略负责清洗。

返回 false、表示不可读或状态失效均拒收。已经协商 Move 时，Drop 阶段只能实际接受或拒绝，不能仅在内部改成 Copy。文件的后续打开、上传或持久化由业务负责；只有实际实现对应操作时才选择 Link。

`DropTarget.handlers` 可替换，`enabled` 默认 true，修改后旧协商失效。禁用沿已有 owner 延迟通路结束 offer；正常离开／结束／Drop 收尾至多通知一次 leave，关闭不再调用业务 leave。拥有者在自身关闭时清除高亮。close 幂等，关窗自动关闭资源，旧原生引用不调用已退休 Context。所有资源访问遵守 Window owner；回调同步完成，不等待自身队列或递归 pump。

`periodicUpdates` 默认 false；设为 true 后，AppKit 在活动拖放期间继续请求 draggingUpdated，使内容或 Accepted 场景变化可以更新静止指针下的选择。禁用或关闭目标时不请求；切换此选项本身不撤销当前协商，也不创建空闲轮询。

正式合同见[数据拖放规范](../../../docs/specs/SCOOP-WINDOW-DRAG-DROP-SPEC.md)，使用、协商与生命周期用例见[拖放测试 Cone](tests-cones/drag-drop/README.md)。UI 已通过 scope.dropTarget 接到自己的 Accepted 输入路由，见 [UI 接收示例](../scoop.ui/examples/drop-target/README.md)。UI source、文本事务与 Windows／Linux 公共后端已接入，目标平台与真实系统会话继续按交互打磨计划验收，进程内 NSDraggingDestination 方法测试不能代表跨程序拖放已通过。

## 发起原生数据拖放

`window.beginDrag(content, effects = DragEffects(true, false, false), localData = None, preview = None, onCompleted = {})` 只在当前真实鼠标或支持的 Touch／Pen owner 派发中调用，上层负责阈值。Windows 的直接接触请求等真实系统兼容鼠标消息到达才启动，等待期间仍可取消；GTK 保留真实 GdkEvent／device，macOS 使用左键 Down／Dragged。content 使用同一 DragContent；文本复制原值，绝对 URL 在同一项提供 URL／文本表示，Files 是非空绝对路径列表。localData 用于同程序实际来源关联，保留普通引用，不编码或传给外部程序。无对应输入、已有源或系统窗口操作时报告状态错误；无效数据或空操作集为参数错误，失败不接管按下序列。可选的 `preview = Some(DragPreview(image, hotSpot))` 使用 DesktopImage 的透明像素及逻辑尺寸。hotSpot 相对于图像左上角，在拖出窗口时仍与指针对齐。

DragSession.effects 可以调整后续协商，只能使用创建时提供过的操作，允许暂时为空；已有结果或关闭后不能更新。正文变化后可撤销 Move 资格，native getter 不进入 Scoop。目标已经报告的 Move 不会被后来的 effects 更新改写成取消，来源仍须核对并报告删除结果。

准备成功后结束普通鼠标和接触路由，并在已有延迟原生操作边界启动系统会话。返回的 DragSession 提供 isStarted、result、isClosed；结果为 Dropped(effect)、NotDropped 或 Failed(error)。onCompleted 在普通 owner 队列交付一次，登记和原生回调先退休。Dropped(Move) 只表示目标接受 Move，Window 不删源；实际文档或文件业务必须独立确认自己的版本和完成条件。

AppKit 保留触发事件所属的左键按下序列。快速松键或取消可能先于业务派发，这种请求正常完成为 NotDropped，isStarted 保持 false；不会因为派发延迟而抛出状态异常，也不会借用后一次按下来启动旧拖动。

`cancelPending()` 只取消未启动请求，返回 true 并产生 NotDropped；已启动返回 false。`close()` 撤销未来回调，关窗自动调用。AppKit 没有公开强制取消接口，关闭已启动资源时 native 继续持有系统使用的数据直到真实结束，清除 Scoop context；未知结果保持 None，也不会再触发来源删除。源窗口失活本身不取消跨程序拖放。

完整两窗口消费者见 [drag-drop 示例](examples/drag-drop/README.md)，原生 item 与准备通路测试见 [drag_source.scoop](tests/drag_source.scoop)。完成状态机的替身测试单独标明；系统拖动、外部程序和关闭中继续拖放的实际结果仍需桌面验收。

2026-11-22 本机 macOS release 通过原生数据／来源收尾的 2 项测试和[排队输入回归](tests/drag_source_input.scoop)：同一个 Application 内覆盖派发前释放／取消／再次按下、准备后释放／取消，共 6 次请求均完成且资源退休。分别使用 `--filter DragSourceTest` 和 `--filter DragSourceInputTest` 运行，遵守每进程一个 Application 的合同。Scoop 格式／含测试语义检查、native clang-format 与严格警告编译通过；这组输入测试不启动 OS 拖放会话。

## 文本输入

`setTextInputPurpose(connection, TextInputPurpose.Email)` 等提示沿当前文本连接交给平台，不能替代应用字符策略；受保护编辑器应使用 Password。`requestSoftwareKeyboard(connection, visible = true)` 返回 Requested、Unavailable 或 Unsupported，不请求系统前台激活；Requested 仅表示平台接受请求。`softwareKeyboard` 和 `WindowEvent.SoftwareKeyboardChanged` 区分未知、隐藏、显示但未遮挡、实际窗口逻辑遮挡矩形。Windows 使用 InputScope／InputPane，GTK 使用 input-purpose／activate_osk（没有通用隐藏与遮挡查询），macOS 返回 Unsupported／Unknown。详见[输入用途规范](../../../docs/specs/SCOOP-WINDOW-TEXT-INPUT-PURPOSE-SPEC.md)。

`ActiveApplication.clipboard`／`Window.clipboard` 提供系统文本剪贴板：`readText(mediaType = "text/plain")` 返回 `String?`，没有该格式为 None，已有空文本为 Some("")；`writeText(text)` 替换为标准纯文本。`writeTexts` 同时发布同一项内容的多个文本表示，例如所见文字和可继续编辑的 Markdown：

```scoop
application.clipboard.writeTexts([
    ClipboardText("text/plain", "中文标题"),
    ClipboardText("text/markdown", "**中文标题**")
])
val markdown = application.clipboard.readText("text/markdown")
```

媒体类型使用无参数的具体 `text/<subtype>` MIME 名称，按 ASCII 小写归一；列表不能为空，格式不能重复。标准纯文本读写把 CRLF／CR 归一为 LF，Markdown／HTML 等其他文本格式保留换行。所有表示禁止内嵌 NUL；写入参数错误在接触剪贴板前报告，读取含 NUL 的原生文本为 Unsupported，非法 UTF-8 或原生写入失败为 NativeFailure。库不解析 Markdown 或 HTML。macOS 使用标准 pasteboard 类型与 MIME 映射，Linux 使用 GDK MIME 表示，Windows 使用 Unicode 纯文本和按 MIME 名称注册的 UTF-8 文本。

操作要求应用 owner 线程和仍运行的应用，从窗口取得的入口不会保活窗口，也不随该窗口关闭失效。剪切应在全部表示写入成功后再删除选区；原生写入失败不承诺系统无法提供的回滚。macOS 的剪贴板测试保存并恢复原有全部项目和格式，覆盖原生类型互通、失败、Unicode 与超过 1 MiB 的 Markdown。

文本输入默认禁用 IME；普通可打印键仍通过 `TextInputEvent.Edit` 交付。调用 `setImeAllowed(true)` 接入平台输入法，`setImeCursorArea` 发布候选窗矩形，坐标为内容区逻辑坐标。Enabled／Disabled 表示窗口的 IME 接入开关变化，重复设置相同值不重复发事件。

应用只从编辑批次插入文字，不再从 `KeyEvent.logicalKey` 额外插入字符。输入法消耗的候选选择／确认键不会再作为普通按键交付；普通快捷键保留 Keyboard 事件。失焦会撤销 active preedit，并为已交付的按下状态补发 synthetic release。

`KeyEvent.textCommands` 保存 window 后端解释出的移动、删除、提交等编辑语义；多条命令保留顺序，随该次按下事件进入同一输入路由。macOS 使用原生 selector，Linux 使用当前逻辑键和标准 Control 编辑键位。空列表表示本次没有编辑命令，keyup 和 synthetic 输入也没有命令。组合更新／提交／取消已消费的按键不会再触发确认或关闭动作，未知 selector 仍保留 Keyboard 回退。

原生菜单的 `MenuRole.DeleteSelection` 对应 `TextEditCommand.DeleteSelection`，只删除非空选区；空选区不删除下一个字符，不等同于 `Delete(TextDeletion.Forward)`。当前文本目标通过 `TextCommandAvailability(deleteSelection = true)` 显式发布能力，默认 false；IME 启用不代表存在选区。原生文本字段按自己的可编辑性与选区验证，Scoop 文本宿主继续走正常输入路由、策略和撤销。该角色没有默认快捷键，不修改剪贴板。

`active.textContextMenuTemplate(preferSystem = true, labels = None)` 返回七种标准编辑命令的纯值模板，包含标签、快捷键及平台选区提示；不创建编辑器、IME 或菜单会话。AppKit 从公开 NSTextView.defaultMenu 保留可执行的标准项目，Windows／GTK 使用标准分组；SystemTemplate 仅表示复用了系统模板子集。未取得的系统标签使用英文默认值，`TextContextMenuLabels` 可逐项覆盖。自绘文本目标可用 Action 项绑定自身，再调用 `Window.showPopup`；不要在 PopupCompleted 中重复执行命令。`isContextMenuClick` 识别右键及平台辅助点击，保留原始物理按钮。详见[菜单规范 §6](../../../docs/specs/SCOOP-WINDOW-MENUS-SPEC.md#6-文本上下文菜单模板)。

`TextInputState.apply(batch)` 是可选的纯值编辑辅助：按次序应用整批操作，全部合法后才发布一次新 revision。范围使用 UTF-8 字节边界；AppKit 的 UTF-16 范围和 TSF 的 ACP 范围在 native 边界转换。取消组合的空 Preedit 会删除当前显示的组合文本；ClearPreedit 只清标记，不能代替取消。

macOS／Windows 的 Enabled／Disabled 都带有 `TextInputConnection`，原生批次的 origin 标识连接和交付顺序。编辑器用 `publishTextInputState(connection, state, after)` 立即发布已生效的文档及已处理批次；AppKit／TSF 的同步 surrounding／selection 查询读取原生不可变副本。引用该快照的显式替换带实际 baseRevision，前序批次尚未处理时不生成猜测的文档范围。CurrentSelection 批次继续不依赖 revision；无效范围拒绝整批，旧连接的编辑不能落到新连接。

`publishTextInputCaret` 单独接收当前连接和文档修订的候选框；没有匹配几何或仍有待处理编辑时，原生位置查询返回 unavailable。`None` 或 `clearImeCursorArea()` 撤回旧矩形。没有发布文档快照的基础 IME 接入只维护 marked segment；Windows 完整文档连接使用 TSF，不保留 IMM32 组合后端。

## renderer 与关闭顺序

`requestRedraw()` 合并请求；在 `RedrawRequested` 中确认 metrics 可绘制，调用 `prePresentNotify()` 后由 renderer 提交帧。窗口库不创建 Metal／D3D／Vulkan／OpenGL 设备，也不把 WaitUntil 当作显示器时钟。

renderer 在 owner 上通过 `scoop.window.native` 的 Unsafe `acquireNativeWindow()` 取得 `NativeWindowLease`。目标平台决定 handle 变体：AppKit 包含 NSWindow／NSView，Win32 包含 HWND／HINSTANCE。lease 保活原生窗口；`close()` 可跨线程且幂等，实际释放回到 owner。调用 close 后不得继续使用句柄，GC 不代替显式 close。

`requestDestroy()` 开始 Closing；Closing 后不再接受业务输入和重绘。renderer 在 Closing 中停止提交、销毁 surface 并 close lease；等全部借用和回调退休后，库才销毁原生窗口并发出最终 Destroyed。

## asyncio

同时声明 `scoop:scoop.asyncio` 和 `scoop:scoop.window.asyncio` 依赖，导入对应包，使用 `Application.runWithAsyncio(handler)`。业务回调可取得 `currentEventLoop()` 和 `currentApplicationScope()`。跨线程业务通过 proxy 返回 owner。

退出／根取消先开始窗口收尾，并继续原生循环直至子任务清理、native lease、通知 watch 和 session 全部结束。Exiting／Closing／Destroyed 在收尾 Context 中交付，没有活动业务 scope。dispatch READ 通知在 AppKit common、tracking 和 modal modes 中服务；Windows 使用 threadpool wait 和 HWND 消息服务系统 modal／menu／move-size 循环；不靠固定 tick 轮询。

## 验证

在已登录的 macOS 图形桌面及准备好的 Darwin sysroot 上运行：

```sh
target/debug/scoop test sysroot/src/scoop.window --release
target/debug/scoop test sysroot/src/scoop.window/tests-cones/deadlines --release
target/debug/scoop test sysroot/src/scoop.window/tests-cones/preferences --release
target/debug/scoop test sysroot/src/scoop.window.asyncio --release
target/debug/scoop test sysroot/src/scoop.window/examples/metal/app --release
```

这些测试使用 `scoop.test`，包括真实 AppKit 窗口、NSEvent 与 NSTextInputClient 协议调用、UTF 范围／编辑批次、跨线程投递和 lease、真实 modal／menu tracking 中的 I/O，以及实际 Metal drawable 提交。Metal 验收需要可用图形设备。

macOS 桌面测试须串行执行。AppKitApplicationTest 等待真实窗口焦点再验证输入协议；系统可以拒绝后台 activate 请求，此时须把测试应用置于前台。准备期为 30 秒，得到实际焦点后用独立的 5 秒期限验证事件与收尾，不能仅凭 requestFocus 返回 Requested 判断焦点成功。

显示器热插拔、混合 DPI、不同键盘布局、系统输入法候选交互和真实用户拖动仍需实际桌面／硬件验收；协议调用测试不等同于这些手工验证。实现与验证记录见 [PROGRESS.md](../../../docs/windowing/PROGRESS.md)。

## Windows

支持 Microsoft UCRT、LLVM-MinGW UCRT 和 LLVM-MinGW MSVCRT 三个 x86_64 target，要求 Windows 10 1703+ 的 Per-Monitor V2。应用可通过标准 RC manifest 声明 DPI awareness；未配置时窗口库在创建 HWND 前设置。启动环境已经使用 Per-Monitor V1 时仅把创建线程升级到 V2，既有窗口保留原 awareness；System Aware 等虚拟化模式仍明确抛出 WindowException。

Win32 桌面坐标采用物理虚拟桌面单位；内容区按窗口 DPI 转换。遮挡快照报告最小化／DWM cloaked，不判断其他窗口的完全覆盖。主题与激活遵守系统限制；Windows 8+ 的 DwmEnableBlurBehindWindow 已无 blur 效果，因此 `setBlur(true)` 返回 Unsupported，创建时要求 blur 则失败。内容保护是系统显示提示。缺少桌面合成时可按默认属性创建窗口；若系统无法启用保护，`setContentProtected(true)` 返回 `Unsupported`，创建时显式要求保护则失败。

在这个会话中仍可切换普通系统边框和不透明的自绘边框。需要 DWM 延伸标题栏或透明背景的请求会返回 `Unsupported`，创建时可据此处理 `WindowException`。

在已登录的 Windows 桌面串行运行以下普通 Cone 测试；真实模态／菜单／尺寸调整会使用前台窗口，不并行运行这些 GUI 测试。两个 MinGW target 另指定对应的 `--native-sysroot`：

```powershell
scoop test sysroot/src/scoop.window --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/dpi --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/input --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/errors --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/tests-cones/watch --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window.asyncio --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window.asyncio/tests-cones/no-window --release --target x86_64-pc-windows-msvc-ucrt
scoop test sysroot/src/scoop.window/examples/dxgi/app --release --target x86_64-pc-windows-msvc-ucrt
```

DPI、输入、错误收尾、watch 和无窗口退出用例各自成 Cone，因为 Application 与进程 DPI 模式不可在同一测试进程中重启。测试不会重置库的单次入口约束。输入测试通过真实 HWND 消息及与 IMM32 共用的 native UTF-16 归一路径验证批次、候选窗矩形和载荷所有权；不冒充已安装中／日输入法的候选交互。watch 测试验证 WaitUntil、借入 manual-reset HANDLE、ready 内 remove 和重复 remove；无窗口退出测试验证实际 IOCP 接收取消与异步 finally。实际显示器／输入法覆盖边界见 [PROGRESS.md](../../../docs/windowing/PROGRESS.md)。

## Linux GNU

`SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 显式选择后端。未设置时，有 `WAYLAND_DISPLAY` 则选择 Wayland，否则选择 X11；连接失败不会改连另一显示服务器。`DISPLAY`、`WAYLAND_DISPLAY` 和 `XDG_RUNTIME_DIR` 使用显示系统的普通约定。

两个显示后端都由 GTK 4.18 以上和 libadwaita 1.4 以上实现；`AdwWindow` 和 `AdwHeaderBar` 负责标题栏、按钮、圆角和阴影。原生输入法、剪贴板和显示器使用 GDK／GTK 接口，X11 的位置和窗口管理请求使用 Xlib，键符号转换使用 xkbcommon。字符比较复用目标资源 `scoop_text_backend`。开发资源按[准备脚本说明](../../../scripts/README.md)放入目标 sysroot，普通应用使用预编译 `.slib` 和系统动态库。

窗口在 GTK 首次分配内容区前为 PendingConfigure，metrics 为 None；显示后由 GeometryChanged 给出实际内容尺寸。framebuffer 按 GTK 的整数 scale factor 计算，桌面的分数缩放由 GTK／合成器处理。Wayland 桌面位置与主显示器等未提供的信息返回 None。最大内容尺寸约束、非默认 resize increments、单窗口强制明暗主题、blur 和内容保护返回 Unsupported；显式请求不支持的创建属性会报错。普通 Window／UI 接口保持，Linux 专用的 `NativeWindowHandle.Gtk` 借出 GtkWindow 与内容容器。

两个后端使用 Linux evdev 物理键和 GDK 当前键盘布局；有完整按键状态时保留左右修饰键，未知键保留原生代码。输入法由 GtkIMContext 接入当前桌面的输入服务，未启用输入法时使用 GTK 的普通字符／compose 输入。剪贴板使用 GdkClipboard，实际跨应用传递遵守显示服务器的焦点与选择规则。

`ShortcutChord` 的 Primary 为 Control，`Window.shortcutLabel` 按当前 XKB 布局显示物理键，使用 `Ctrl+Shift+…` 形式；逻辑键保留声明的文本。AltGraph 状态可确定时通过 `Modifiers.altGraph` 提供，输入 AltGraph 字符不会触发普通快捷键或 Control 编辑命令。

验证命令使用准备好的 GNU sysroot：

```sh
SCOOP_WINDOW_BACKEND=x11 scoop test sysroot/src/scoop.window --release
SCOOP_WINDOW_BACKEND=wayland scoop test sysroot/src/scoop.window --release
SCOOP_WINDOW_BACKEND=wayland scoop test sysroot/src/scoop.window.asyncio --release
```

窗口测试使用真实显示服务器，X11 输入测试还需要 libXtst 开发包；GTK 自行映射普通窗口，应用绘图由 Surface 挂载的 GtkGLArea 提供。GTK 信号仅排入窗口事件，业务回调在 GTK dispatch 返回后执行。验证范围、依赖版本及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
