# Scoop 桌面窗口库规范

状态：基础架构与 macOS／Windows GUI 骨架已实现，Linux GNU Wayland／X11 后端按第 12 节接入并验证。实现基线日期：2026-10-30；第 11 节的 GUI 输入扩展于 2026-10-31 实现，Darwin 两目标验证通过，Win32 同步共享合同但本轮未执行 Windows 验证。本文件描述普通 Cone `scoop.window` 与 `scoop.window.asyncio` 的当前合同及后续平台设计方向；实际交付与验证范围见[窗口库实施记录](../windowing/PROGRESS.md)、[目标扩展实施记录](../target-refactor/PROGRESS.md)和 [GUI 实施记录](../surface/PROGRESS.md)。依据 [设计](../windowing/DESIGN.md)与 [winit 能力映射](../windowing/WINIT-MAPPING.md)。

2026-11-01 补充第 4.1～4.3 节的多窗口、模态和窗框能力边界；本次是文档更新，没有新增窗口 API 或运行验收结论。

后续公共键位、编辑命令、剪贴板与 IME 合同的演进见 [控件基础平台设计](../ui/component-foundations/PLATFORM.md)，当前仅完成设计；本文件中的现行合同在对应实施批次开始时先行修订。

## 1. 分层与目标

window 依赖 core／std／platform；window.asyncio 另依赖 asyncio。Darwin AArch64／amd64 的 AppKit 后端和三个 Windows x86_64 target 的 Win32 后端已实现；Windows 窗口 API 要求 Windows 10 1703 起的 Per-Monitor V2。Linux GNU x86_64 的 Wayland／X11 接入按第 12 节实施，进展与实际验证范围见 [Linux GUI 实施记录](../linux-gui/PROGRESS.md)。两个库及其原生薄层、测试、示例均以普通 Cone 组织，窗口逻辑不进入编译器或 runtime。

本轮范围按 winit 定位为 GUI 骨架：应用／窗口与基础属性、基础输入／IME、DPI／显示器快照、渲染句柄和 I/O 集成。下文的完整窗口层模型同时保留后续扩展方向；全屏控制／视频模式切换、全局 raw device、触摸／高级手势、光标捕获、文件拖放及复杂平台窗口交互不作为当前骨架验收项。未实施接口随实际需求再增加，不为占据 API 名称加入空实现。

公共应用对象不暴露 fd、HWND、XID、dispatch queue 或 ring。现有 Job／CoroutineScope、Context、ForeignCallback、时间、路径与异常规则继续适用；窗口库不承担布局、排版、绘制设备、I/O 调度或另一套任务树。

原生菜单、popup 及跨 Cone 菜单借用的实施合同见[菜单规范](SCOOP-WINDOW-MENUS-SPEC.md)。托盘另由单向依赖 window 的 scoop.tray 提供，普通 window 和 UI 不引入托盘系统依赖。完整桌面扩展的实际进度见[实施记录](../desktop-ui/PROGRESS.md)。

## 2. 应用与线程

Application 构造接收 ApplicationOptions 并取得独立快照，run 接收 ApplicationHandler。handler 只有 `onEvent(application: ActiveApplication, event: ApplicationEvent)` 一个分发入口。每进程一个 Application、一次 run；入口要求进程主线程和当前 Job 外。构造只建立 managed 状态，接受 run 后才建立原生应用；启动失败消费已取得资源且不能重用该 Application。基础库不认识 I/O Context，集成入口另由 PlatformEventLoop.attach 拒绝活动 I/O EventLoop binding。ActiveApplication 负责 createWindow、monitors、primaryMonitor、setControlFlow、requestExit。可选 applicationId 在首窗前设置原生应用关联，ActiveApplication.applicationId 暴露原快照；具体平台含义与部署一致性见[桌面集成规范](SCOOP-DESKTOP-SPEC.md)。

同一 Application 的 handler、proxy 与集成回调共享一个稳定的 ActiveApplication 引用；它表达相同运行 owner，不在每次派发时重新创建包装对象。引用稳定不延长运行期，关闭后各操作仍遵守原有状态检查。

ApplicationProxy 提供跨线程 post 与 requestExit；post 为 `(ActiveApplication) -> Unit`，返回 Boolean，异步排队。false 表示拒绝且不保留闭包；true 不保证之后关闭时仍执行。关闭停止业务投递，清理通知继续处理。所有原生窗口操作只在 owner 线程；WindowId／DeviceId／MonitorId 和不可变快照可跨线程保存。

ControlFlow 为 Wait／Poll／WaitUntil(TimeMark)，默认 Wait；TimeMark 必须来自 MonotonicTimeSource。每轮消费有限快照，新增事件进入后续轮；锁内不执行用户代码，不递归调用 handler。OS 必须立即得到的回答使用已发布 native 状态，不等待可挂起的用户处理。

run Context 在业务回调中恢复，外线程 post 不替换该 Context。用户异常终止业务派发并开始有序收尾；native bridge 捕获异常，不跨 C／ObjC 栈展开。最终清理通知继续执行，异常记录后继续关闭其他资源。

### 2.1 窗口所属的一次性定时回调

`Window.createDeadline(callback: () -> Unit)` 创建尚未排期的 WindowDeadline，提供 schedule(TimeMark)、cancel()、close()、isScheduled 和 isClosed。创建、查询、排期与取消使用窗口 owner 线程，时间必须来自 MonotonicTimeSource；非法时间不改变原排期。schedule 替换之前的排期，已到期的时间也异步进入后续批次，不内联 callback。cancel 与 close 幂等，close 后不能再次 schedule。

每个排期至多调用一次。进入到期批次时先清除本次排期，再按到期时间、同时间按登记顺序交付；callback 可以重新排期自己、取消别的登记或关闭窗口。已经进入当前快照但尚未调用的旧回调，在 cancel、重新 schedule 或 close 后跳过，不重放错过的次数。隐藏、遮挡或零 drawable 不阻止业务回调；回调本身不自动请求重绘。

WindowDeadline 和 ControlFlow.WaitUntil 共用现有宿主原生定时器，只安排最近的实际期限，不建立周期轮询、另一个事件循环或计时线程。内部期限唤醒使用 NewEvents(Wake)，只有应用当前 WaitUntil 真正到期才产生 NewEvents(Deadline) 并恢复 Wait；内部回调不能取消或提前触发应用期限。普通事件到来时也可消费已经到期的登记。

回调沿同一业务派发入口恢复 run／附着 I/O Context，异常按普通 handler 失败开始完整关闭。窗口进入 Closing 时立即关闭其登记，取消未调用的快照；应用退出同样撤销全部排期。登记不取得 native window lease，不延迟窗口销毁。首批用于 UI 基础交互中的离屏动画完成和后续编辑定时工作，按该批次先验证 macOS aarch64，其余目标留到对应验收。

### 2.2 GUI 文本剪贴板

公共 `ActiveApplication.clipboard` 和 `Window.clipboard` 取得同一个应用 owner 上的轻量 `Clipboard` 入口。`readText(mediaType: String = "text/plain"): String?` 读取指定文本格式；没有该格式返回 None，已有空文本返回 Some("")。`writeText(text: String)` 以标准纯文本替换剪贴板当前内容；`writeTexts(representations: List<ClipboardText>)` 同时提供同一项内容的多个文本表示，ClipboardText 包含 mediaType 和 text。调用方可提供所见纯文本及 `text/markdown` 源片段，接收方按自己的格式优先级读取，不将普通文字猜测成 Markdown。

mediaType 是无参数的具体 `text/<subtype>` MIME 名称，按 ASCII 小写规范化；空 subtype、通配符、非 ASCII、空白或参数报 IllegalArgumentException。writeTexts 要求非空列表且规范化后没有重复格式，在接触原生剪贴板之前检查所有格式和正文。所有表示采用 Unicode 文本，保留非 BMP 字符与组合序列，不做 Unicode 规范化。text/plain 的 CRLF 和独立 CR 统一为 LF，由后端处理原生行尾；其他文本格式保留实际换行。写入在修改剪贴板之前以 IllegalArgumentException 拒绝内嵌 U+0000；读取到属于文本内容的 NUL 时抛出 WindowException(Unsupported)，不能截短。原生转换或写入失败抛出 WindowException(NativeFailure)，不能把失败当成成功的空文本。

取得入口及每次读写均要求应用 owner 线程和仍运行、未开始退出的 Application；线程错误为 IllegalStateException，应用已退出为 WindowException(InvalidState)。Clipboard 不取得 native window lease，也不需要 close；从窗口取得的入口不依赖窗口寿命，窗口关闭后，只要应用仍运行就可继续使用。它不解析 Markdown／HTML，不提供终端 OSC52、剪贴板历史或跨线程代理；需要跨线程请求时使用现有 ApplicationProxy。

公共声明、owner 检查、文本归一化和 NUL 合同位于共享源码；后端只转换及读写原生格式，不向上层暴露借用。读取结果独立复制，成功与异常路径均释放临时存储。macOS 通过 NSPasteboard 接入，纯文本使用标准字符串类型，其他 MIME 经 UniformTypeIdentifiers 映射为有效 UTI；Linux 使用 GdkClipboard 的 MIME 格式；Windows 纯文本使用 CF_UNICODETEXT，其他文本使用 MIME 名称注册的 UTF-8 文本格式。多个表示先在 native 临时对象中准备，再替换系统剪贴板；不存在进程私有的内容回退。剪切的上层编辑必须先成功写出，再修改文档；参数或写前转换失败不改变剪贴板，实际原生写入失败不承诺 OS 无法提供的回滚。

Windows 写入使用应用已有的隐藏宿主窗口取得 clipboard owner，不使用空 HWND 执行 EmptyClipboard／SetClipboardData；因此该入口无需依赖仍存活的业务窗口。

### 2.3 窗口活动状态观察

`Window.activity` 返回自有 `WindowActivity(focused, visible, occluded)` 快照；`observeActivity(callback)` 返回 Closeable 登记，创建时记录当前快照，不内联调用 callback。窗口 owner 线程在原生 Focused／Occluded／GeometryChanged 事件派发前、setVisible 实际操作后比较当前快照，仅值变化时同步通知。回调可以读取状态、取消自身登记或关闭窗口；取消后不再接收当前批次余下通知，后注册的观察者不加入正在派发的批次。回调异常遵守所在普通业务调用的失败处理。

观察不增加窗口 lease、线程、定时器或 redraw。close 幂等；窗口进入 Closing 时撤销全部登记并释放回调，不再读取已销毁窗口。此入口服务需要准确暂停剩余时间的 UI 通知，不能用焦点控件或绘制是否发生来推测窗口活动状态。

## 3. 事件模型

ApplicationEvent 类别为：NewEvents(StartCause)、Resumed、Suspended、MemoryWarning、Window(WindowId, WindowEvent)、MonitorsChanged、AboutToWait、ExitRequested(ExitReason)、Exiting。启动在创建业务窗口之前发送一次 Resumed；失焦不等于 Suspended，AppKit 不捏造没有收到的 suspended／memory warning。proxy 使用类型化闭包，不另设无类型 user-event payload。全局 Device 事件属于后续设备输入扩展。

WindowEvent 类别为：CloseRequested、Closing、Destroyed、GeometryChanged(WindowMetrics)、Moved(DesktopPosition)、Focused(Boolean)、Occluded(Boolean)、ThemeChanged(Theme?)、Keyboard(KeyEvent)、ModifiersChanged(Modifiers)、Pointer(PointerEvent)、TextInput(TextInputEvent)。RedrawRequested 为独立窗口事件。payload 为自有值，不携带 OS 回调栈的借出指针。触摸、手势、文件拖放和平台专有激活结果随对应能力后续增加。

NewEvents 原因区分 Init、Poll、Wake 与 Deadline；平台没有信息时不伪造输入硬件时间。允许合并 redraw、连续 geometry 和 pointer move，但不得跨过按键／按钮、IME 编辑、焦点、close 等顺序边界；Commit、drop、button、touch 的开始／结束及 lifecycle 不得丢弃。

## 4. 窗口与显示器

WindowAttributes、Window 的能力与方法分组见映射表，当前阶段只提供其中标为本期的已实施接口。请求返回 Applied／Requested／Unsupported；Requested 不保证系统采纳，实际状态由通知／快照决定。查询无法知道时返回 None；不把 desired state 写成 observed state。创建所需属性不支持时明确失败，不悄悄改变含义。

WindowException 保留 WindowErrorKind、operation、backend 和可选 native code；类别包含 Unsupported、Unavailable、PermissionDenied、InvalidState、ResourceBusy、Disconnected、NativeFailure。参数错误使用现有参数异常；backend 随实际后端提供，按实际显示系统为 AppKit／Win32／X11／Wayland，不假定原生 code 都是 errno；Win32 保留 DWORD／HRESULT 的 32-bit 数值。能力提示不替代实际操作错误。

WindowId、DeviceId、MonitorId 为不同值类型，只在一次 Application 生命周期内比较，不暴露原生身份，不复用活过的编号。本阶段 Monitor 为不可变快照，保存 id、可选 name／desktopBounds／workArea／refreshRateMilliHertz、physicalSize 与 scaleFactor；不持有原生指针。workArea 只保存平台实际给出的工作区，不以完整屏幕范围填补缺失值。ActiveApplication.monitors／primaryMonitor 与 Window.currentMonitor 在 owner 上获取快照，后者及 primaryMonitor 可为 None。AppKit 用 NSScreen 的桌面矩形和 backing 转换，CoreGraphics 未给出正刷新率时返回 None，不把最大可用刷新率当作实际刷新率。显示器变更通知刷新当前身份集合，已观察到离开的身份退休；再次出现分配新编号。后续加入视频模式及控制请求时再定义对应活动句柄的失效行为。

LogicalPosition／LogicalSize／LogicalRect 采用有限 Double，内容坐标左上原点、向右／向下递增。PhysicalSize 为非负 Int 像素尺寸；PhysicalPosition 允许小数。DesktopPosition／DesktopRect 仅用于同一显示系统的全局桌面坐标，不允许通过单个窗口 DPI 在混合 DPI 桌面上换算。

WindowMetrics 包含 logicalSize、physicalSize、scaleFactor，scaleFactor 必须有限且为正；首个原生配置前 metrics 为 None，GeometryChanged 原子发布完整快照。请求与实际配置分离；渲染所需像素向上取整，零 drawable 大小不得申请或提交帧。

窗口级别为 Normal／AlwaysOnTop／AlwaysOnBottom。后续全屏控制采用 Windowed／Borderless(Monitor?)／Exclusive(VideoMode)，并与显示模式切换和恢复一起实施；相同平台语义复用现有表示。平台专有选项放在 platform 子包。

尺寸约束分别保存可选 min／max，非负且 min 不大于 max；创建尺寸必须满足约束。运行中收紧约束时请求调整当前内容大小，实际结果由 GeometryChanged 确认。resize increments 为可选的有限正尺寸，None 恢复平台默认。属性在创建返回前复制并应用，再按 visible／activation 请求显示窗口；不会先显示一个忽略所需属性的窗口。

AppKit 使用原生内容尺寸／桌面坐标、titlebar 按钮、window level、miniaturize／zoom、appearance 与 NSVisualEffectView。改变装饰、resizable 或 full-size-content 的 style mask 后恢复输入 view 为 first responder，保证键盘路由连续。模糊背景放在 renderer 宿主 view 后面，关闭模糊时恢复独立的 transparent 设置。content protection 使用系统分享提示，返回 Requested，不保证屏幕捕获系统遵守该提示。macOS 的 activation policy 和 titlebar／full-size-content 选项位于 platform.macos；应用标识只查询实际 main bundle，命令行进程没有 bundle identifier 时返回 None。

### 4.1 多窗口

多窗口在 AppKit、Win32、X11 和 Wayland 后端均已实现。一个 Application 可以多次调用 ActiveApplication.createWindow，创建共享同一 owner 线程和原生事件循环的多个独立窗口；每个窗口有自己的 WindowId、状态、输入、重绘需求和 native lease。ApplicationEvent.Window(id, event) 指明事件归属，不为每个窗口另建 Application 或主循环。

createWindow 默认创建非模态的独立顶层窗口，也可通过 WindowRelation.Owned 建立固定的原生归属关系，通过 WindowRole.Tool 请求工具面板。关系、相对叠放、可见性意图与后代先销毁的合同见[归属与模态规范](SCOOP-WINDOW-MODAL-SPEC.md)。最后一个窗口实际 Destroyed 后，LastWindowPolicy.Exit 才触发应用退出；默认 KeepRunning，详见第 7 节。

现有 [Darwin 测试](../../sysroot/src/scoop.window/tests/application.scoop)与 [Win32 测试](../../sysroot/src/scoop.window/test-windows/application.scoop)包含双窗口、不同 WindowId、分别请求销毁和重复销毁的用例。它们不构成模态窗口或父子窗口关系的实现证明。

### 4.2 装饰与窗口外观

当前创建属性与运行中操作包括：

| 能力 | WindowAttributes | Window 操作 |
| --- | --- | --- |
| 原生／自绘窗框与内容延伸 | frame | setFrame |
| 交互式调整大小 | resizable | setResizable |
| 关闭／最小化／最大化按钮可用性 | buttons | setEnabledButtons |
| 普通／置顶／置底层级 | level | setWindowLevel |
| 透明背景 | transparent | setTransparent |
| 系统阴影 | shadow | setShadow |
| 显示／隐藏 | visible | setVisible |

WindowFrame 与 resizable 是独立属性；Client 去除宿主可见装饰并保留必要系统交互。窗框模式、实际几何、区域、外观和系统操作以[窗框规范](SCOOP-WINDOW-FRAME-SPEC.md)为准。

macOS 的 TitlebarOptions 提供 transparent、titleHidden，通过 WindowAttributesBuilder.macOSTitlebar 配置创建值，或通过 Window.setMacOSTitlebar 更新现有窗口；内容延伸统一使用 WindowFrame.System(WindowContentMode.Extended)。无装饰窗口仍可成为 key／main window，沿用第 5 节的输入规则。透明背景属性不替代 renderer 的实际 alpha 呈现能力。

窗口层级与模态是不同属性：AlwaysOnTop／AlwaysOnBottom 只改变叠放策略，不禁止其他窗口输入，也不建立窗口归属。

### 4.3 模态与系统窗框操作

showModal 从隐藏的既有窗口创建 ModalSession，区分 Owner／Application 输入范围与 Automatic／SeparateWindow／Sheet 展示。实际能力、原生输入限制、唯一完成、焦点请求及关闭依赖见[归属与模态规范](SCOOP-WINDOW-MODAL-SPEC.md)。第 8～10 节的原生循环接入继续承担嵌套展示期间的 I/O 与控制推进，不能递归执行 managed pump。

原生文件／目录面板由 ActiveApplication.openFile／openFiles／saveFile／selectDirectory 提供，目标明确区分 Owner 与 Application，返回 FileDialogSession<T> 和 Path 结果。选项、无损路径、原生取消与关闭依赖见[文件对话框规范](SCOOP-WINDOW-DIALOGS-SPEC.md)。

自绘窗框的 beginMove／beginResize／showWindowMenu 使用当前真实输入上下文；原生命中、区域失效与能力差异见[窗框规范第 3～4 节](SCOOP-WINDOW-FRAME-SPEC.md#3-自有区域)。

窗口归属、模态输入限制、焦点与系统拖移／缩放由 scoop.window 提供；UI 负责内容、控件及结果交付。不得在组件中通过临时屏蔽几个输入回调或设置置顶来代替窗口模态。实施时分别验证窗口模态／应用模态、其他窗口的交互、关闭及焦点恢复、自绘窗框拖移／缩放，以及真实原生循环中的 asyncio 推进。

## 5. 输入与文本

KeyEvent 分离 PhysicalKey、LogicalKey、KeyLocation、state、repeat、modifiers 和 synthetic，并保存有序 `textCommands: Array<TextEditCommand>`（手工构造时缺省为空）；LogicalKey 支持 Named／Character(String)／Dead／Unidentified，未知 native 键码保留实际编码来源，Darwin keyCode、Windows 扩展 scan code 或 Windows virtual key。修饰键含 Shift、Control、Alt、Super、锁定状态和可取得的左右侧信息。`Modifiers.altGraph: Boolean?` 保存已确定的文本修饰语义，None 表示未知，macOS 为 Some(false)；Linux 从当前 XKB 的 LevelThree／Mode_switch 文本修饰状态产生 Some(true) 或 Some(false)，缺少 keymap 时为 None。Super 始终是实际 Command／Win 键，不能用 Windows Control 冒充 Super。Function 表示实际 Fn 状态，不把 AppKit 为方向键等附加的功能键类别标志作为额外修饰键。

TextEditCommand 表达 Move(TextMovement, extendSelection)、Delete(TextDeletion)、Copy／Cut／Paste／SelectAll、Undo／Redo、InsertNewline／InsertTab／InsertBacktab 和 Cancel。TextMovement 区分逻辑 Backward／Forward、视觉 Left／Right、Up／Down、逻辑／视觉词移动、视觉行的逻辑／视觉两端、逻辑段落和文档两端及 PageUp／PageDown；TextDeletion 表达前后 grapheme、词、当前视觉行或逻辑段落的两端。Window 只保存原生语义，不执行排版、选择或文档修改。

AppKit 的 doCommandBySelector 在当前 keyDown 解释中产生上述命令；多个 selector 按原顺序放在同一 Keyboard 事件内。命令只出现在真实按下／重复事件，keyup、synthetic release 与修饰状态事件没有命令。Window 在原生命令为空且该键未被输入法消费时，补充宿主的标准编辑映射；macOS 包括 Command 的复制／剪切／粘贴／选择／撤销／重做和行／文档导航、Control 编辑键、Option 词／段落导航以及普通编辑键。未知 selector 不伪造命令，但仍可使用该键已确定的标准映射。Linux 在 window 层产生 Ctrl 的复制／剪切／粘贴／选择／撤销／重做、词和文档导航以及普通编辑命令；Home／End 对应行边界，Ctrl+Home／End 对应文档边界。AltGraph 文本输入不产生 Ctrl 编辑命令或快捷键。空列表表示没有编辑含义，上层不得再添加平台键位表；Windows 标准映射由该平台后续批次实现。所有命令沿同一 capture／target／bubble／default 路由，preventDefault 同时阻止对应默认编辑，字符插入仍只由 TextInputEvent 交付。

Window 公共 `LogicalKey.matchesShortcut(other)` 比较已收到的逻辑键，不监听或注册系统 hotkey。Character 使用平台的 Unicode 无 locale 大小写比较，保留重音区别并识别规范等价；Named 和 Unidentified 按值比较，Dead 不作为快捷键字符。此查询只消费值，不要求活动窗口或抢占焦点，也不从文本编辑结果反推按键。

`ShortcutKey` 为 Logical(LogicalKey)／Physical(PhysicalKey)。不可变 `ShortcutModifiers` 保存 primary、shift、control、alt、superKey、function 六个布尔值，默认均为 false。`ShortcutChord(key, modifiers = ShortcutModifiers())` 保存完整描述，构造时拒绝 Dead 和空 Character；`shortcutChord(key) { ... }` 用 ShortcutModifiersBuilder 同步构造同一描述，配置块不执行动作。Primary 在 macOS 展开为 Super，在 Linux 和 Windows 合同中展开为 Control；解析后按集合合并，不能要求同一个修饰键按两次。完整匹配忽略锁定及左右侧信息，未指定的其他修饰键不得出现；明确的 AltGraph 文本修饰不匹配普通快捷键。

`ShortcutChord.matches(event)` 比较键和解析后的完整修饰集合，不判断 keydown、repeat、synthetic 或路由；`equivalentTo(other)` 比较同一种 logical／physical 语义下的解析后组合。两个操作复用同一平台解析，surface 负责绑定、冲突、输入资格和动作。

`Window.shortcutLabel(chord): ShortcutLabel` 在 owner 线程和活动窗口上查询当前键盘布局，返回只读 `parts: Array<String>` 与 `separator: String` 值快照；不保留 native 借用。macOS 修饰键按 Control、Option、Shift、Command、Fn 排列，使用 ⌃／⌥／⇧／⌘／fn，separator 为空串，重复的 Primary／Super 只出现一次。Logical 字符保持绑定字符；Physical 字符按当前键盘布局查询，无字符映射时使用明确物理键名称；已知非字符键使用本平台标签，未知键使用 Unknown。查询不改变输入源、焦点或当前组合。Linux 修饰键按 Ctrl、Alt、Shift、Super、Fn 排列，separator 为 "+"；Physical 字符通过当前 XKB keymap 的无修饰布局查询，非字符键使用 Linux 标签。macOS 和 Linux 均实现标签查询，Windows 后端按同一入口补齐。

AppKit 使用实际 NSEvent keyCode 和当前布局产生的字符；Command／Control 快捷键不产生普通文本提交。无装饰窗口仍可成为 key／main window，接收键盘不依赖原生标题栏。已交付的按下状态在失焦时补发 synthetic release，清空按住的修饰键并保留已知锁定状态；没有硬件来源的设备 id 为 None。CapsLock 通知表达锁定状态，不伪造物理按键释放。指针同时携带同次 native 事件的逻辑／物理位置与 scale，两种位置均以内容区左上为原点，物理位置由该次窗口 backing scale 换算；不能直接混入 backing surface 的底部原点。精确滚轮的 AppKit points 转换为物理 PixelDelta，普通滚轮保持 LineDelta，普通 phase 和 momentumPhase 分别保留。光标基础图标使用 NSCursor，可见性只在本窗口内容区域生效，退出该区域、失焦或销毁时平衡系统 hide／unhide。

文字统一由 `TextInputEvent.Edit(TextInputBatch)` 中的 Commit 操作交付；普通键、compose、IME 不得重复插入。TextInputEvent 只有 Enabled(connection?)／Disabled(connection?)／Edit。TextInputConnection 由 WindowId 与该窗口递增的输入连接 generation 组成；macOS／Windows 开启时均交付 Some(connection)，重复开关不重建连接，Disabled 保留结束的连接身份。一次 AppKit 输入解释或一个 Windows TSF 原子编辑事务形成一个编辑批次，以一个事件调用 handler。范围为 UTF-8 字节区间且位于字符边界，平台 UTF-16 索引在 native 边界转换；非法 native 范围触发状态重同步，不越界访问。

owner 线程上的 setImeAllowed 与 setImeCursorArea 保留无需文档快照的基础接入。公共 `publishTextInputState(connection, state, after: TextInputOrigin? = None): Boolean`，把 TextInputState 的文本、selection、marked range 和 revision 独立复制给当前原生连接；caretRect 不在此调用中发布。连接已结束或被替换时返回 false，窗口关闭及线程规则与其他窗口操作一致，跨窗口连接或非法范围属于参数错误。相同连接的 revision 不倒退，相同 revision 不改变文档内容、选择或组合；重新开启连接可发布新的文档基线。公共检查和声明位于共享源码，后端负责复制与原生查询；macOS 通过 NSTextInputClient、Windows 通过 TSF 文档存储、Linux 通过 GtkIMContext 接入同一合同。没有发布快照时，原生 surrounding 查询返回 unavailable。

原生 Edit 的可选 origin 为 TextInputOrigin(connection, sequence)，sequence 只表达同一输入连接中已经排队的批次顺序。应用处理完一批后，把它的 origin 随已生效 state 传入 after；已经处理的序号不倒退、不越过该连接实际排队的末项。非原生纯值批次及不参与文档事务的普通 Win32 字符批次为 None。native 只保存最新已发布文档、当前原生事务的临时投影以及已经排队和已经处理的两个序号，不保留文档历史或建立另一套编辑状态。普通 CurrentSelection 批次可连续排队；在较早批次尚未处理完时，无法确定的 surrounding 查询返回 unavailable，不能把预测的未来 revision 当作已应用版本。

NSTextInputClient／ITextStoreACP 的同步查询读取原生不可变文档副本；同一次原生解释或 TSF 编辑事务内允许对其临时副本依次投影本批操作。macOS attributedSubstring 返回已知、完整 UTF-16 标量范围的纯文本 attributed string 及 actualRange；Windows GetText 返回已知 ACP 范围的纯文本。selection／marked 的文档范围保留实际 native 组合信息。未知范围、尚未同步的 surrounding、切断 surrogate 或缺少布局时返回平台 unavailable 结果，不回调 managed handler、不布局、不挂起。

IME 接入默认关闭；setImeAllowed 成功改变开关时产生 Enabled／Disabled，重复设置相同值不重复通知，Enabled 先于该次开启产生的编辑。关闭接入先撤销组合；失焦也撤销组合，但保留接入开关。setImeCursorArea 接受有限、非负尺寸的内容区逻辑矩形，包含矩形端点溢出检查；公共 clearImeCursorArea 撤销它。候选窗使用原生客户区到屏幕坐标转换后的位置。公共文档接入使用 `publishTextInputCaret(connection, revision, area: LogicalRect?): Boolean`：只有当前连接、所有较早原生编辑已处理且 revision 与已发布文档一致时才接收，None 明确撤销几何。文档改变、待处理原生编辑和连接结束立即使旧几何不可用。macOS firstRect 与 Windows GetTextExt 只报告确知的活动 cursor 位置；macOS characterIndex 与 Windows GetACPFromPoint 只在该有效 caret 矩形内报告其索引，其他范围／位置返回 unavailable，不编造整个文档的布局。

接入开启后，普通 keyDown 先调用 AppKit 的 interpretKeyEvents，再决定是否交付 Keyboard。组合处理、确认及取消已消费的键不再交给应用执行普通编辑；doCommandBySelector 可要求交付应用命令，但同次解释已经改变／提交／取消组合时不再次交付确认或取消键，即使命令早于该次组合编辑产生也如此。Command／Control 快捷键继续直接交付 Keyboard，缺省编辑映射使用实际修饰信息，不能产生第二次文本插入；keyup 只释放此前已交付的按下状态。

TextInputState 是应用可选用的纯值编辑辅助，包含 surroundingText、selection、markedRange?、caretRect、revision；只有调用上述发布入口才成为 OS 可查询的文档副本。selection 保存 anchor／cursor，不能排序后丢失选择活动端；markedRange 为 Utf8Range。只修改候选窗位置不推进应用文本 revision。`String.utf8Slice(Utf8Range)` 复用同一范围检查返回文本片段，供选区读取和编辑差量使用。组合与真实编辑状态由上层编辑器持有，窗口库不负责排版或 grapheme 导航。

TextInputBatch 保存可选 baseRevision、origin 与有序的 TextInputOperation 数组；纯值 apply 只解释编辑和 baseRevision，连接归属与原生批次完成由宿主处理。Utf8Range 是非负、半开的 Long 字节区间，start 不大于 end；Utf8Selection 的两端均非负。操作如下：

| 变体 | 载荷与语义 |
| --- | --- |
| ClearPreedit | 清除当前组合状态 |
| Preedit | text、可选 selection（相对 preedit，保留活动端）、replacement；空 text 清除组合 |
| Commit | text、replacement；replacement 为 CurrentSelection 或 Range(Utf8Range)，范围引用该批前序操作产生的临时文本 |
| DeleteSurrounding | beforeBytes、afterBytes；相对当前 selection 两侧，须校验长度与字符边界 |

纯值批次开始时核对 baseRevision，按顺序在临时文本上解释范围，整批通过后发布一次新快照。任何范围或 revision 失效均拒绝整批，不能部分应用。AppKit 的 CurrentSelection Preedit／Commit 保持 baseRevision=None；只有引用实际已发布文档的显式范围使本批携带该文档的 baseRevision。取消组合产生空 Preedit，删除应用正在显示的组合内容；上层编辑器可据其组合事务恢复替换前基线。unmarkText 表示接受已经显示的组合，产生 ClearPreedit 而保留正文；它与 cancelComposition 不同，不能把接受组合变成删除或第二次插入。原生 insertText 接受实际纯文本及显式替换，包括空字符串和换行，普通未解释按键的 fallback 仍过滤控制键。原生 UTF-16 选区必须合法且不切断 surrogate pair，非法批次拒绝并撤销组合。

AppKit 的 NSNotFound replacement 表示替换当前 marked segment 或应用选区。没有文档快照时，显式 replacement 只能引用已知 native marked segment，归一为整段 CurrentSelection 替换，选区先平移再转 UTF-8。有文档快照时，显式 replacement 使用文档坐标，按本批前序操作的临时文本转成 UTF-8；若较早排队编辑尚未处理或范围未知，则拒绝整批，不能退回 marked 局部索引或重新解释旧范围。过期的 managed baseRevision 同样拒绝整批，由宿主关闭到 Disabled 后用当前 state 重开连接。无效 native 批次丢弃全部待交付操作；仅在批次开始前已有 native preedit 时产生一次空 Preedit 撤销它，不能因拒绝一个新组合而误删应用原选区。

`TextInputState.apply(batch)` 提供上述字节编辑规则的纯值实现，供输入归一和应用同步快照使用，不承担文档布局。CurrentSelection 在存在 marked range 时替换组合文本，否则替换 selection；ClearPreedit 只清除 marked 标记。Preedit 插入后设置 marked range，并把相对 selection 平移到新文本；Commit 清除组合，光标落在插入末尾。DeleteSurrounding 删除 selection 之前／之后指定字节，保留选区正文与活动端方向，清除组合标记。成功批次统一使 revision 加一，空批次保持原值；过期 revision 报 IllegalStateException，非法范围、尺寸或 revision 溢出报 IllegalArgumentException，原快照保持不变。

Pointer 区分位置、按钮、进入／离开、滚轮；滚轮区分行／像素并保留相位。DeviceId 不可得时为 None。失焦清理组合与按下状态，必要的补发 release 标记 synthetic。后续 raw motion 使用设备单位，Touch 保留 contact id、phase 和可选压力，Gesture 只报告原生手势；这些扩展不是当前交付接口。

ScrollDelta 的 x／y 表示内容在窗口坐标系中的位移：正 x 向右，正 y 向下。滚动容器将其取反为 offset，因此查看更右方／下方的内容分别对应负 x／y。后端以 OS 已应用用户滚动方向设置的事件为准，只归一坐标含义，不再次应用自然滚动设置；GTK 两轴取反，Win32 水平轴取反、垂直轴保留，AppKit 保留原生内容位移。LineDelta 和 PixelDelta 的单位／比例与相位合同不变。

后续光标捕获使用 None／Confined／Locked，warp 与相对输入分离；custom cursor 的 RGBA 与 hotspot 在创建时复制。移动／resize／系统窗口菜单请求需要有效输入上下文的平台，由后端保留原生 serial 并核对，不能靠定时回调伪造用户操作。

## 6. 绘制与 native 句柄

requestRedraw 合并 dirty，RedrawRequested 不递归触发；隐藏、遮挡或零尺寸时保留 pending redraw，恢复后再处理。prePresentNotify 是提交前通知，不能等价为 present、vsync 或 GPU 完成。图形设备与渲染管线由上层 renderer 管理。

native 子包提供 Unsafe 的 acquireNativeWindow，返回 NativeWindowLease；handle 按目标声明为 `NativeWindowHandle.AppKit(nsWindow: Ptr<Unit>, nsView: Ptr<Unit>)` 或 `NativeWindowHandle.Win32(hwnd: Ptr<Unit>, hinstance: Ptr<Unit>)`。当前目标不声明其他后端的句柄变体。取得 lease 和查询 handle 只在 owner 上进行，lease 保活窗口和 display connection；窗口 Closing 后不能取得新 lease。

lease.close 可跨线程且幂等，原子结束借用并把实际释放排入 owner 内部收尾队列，不直接在调用线程操作 GUI，不依赖已经停止接受业务的 ApplicationProxy。调用后不能再使用 handle；队列实际消费前继续保活原生对象。GC release 不得代替这个显式操作。

已有 lease 阻止库主动销毁原生窗口，但不保证显示服务器永远可用，也不扩展平台图形对象的线程权限。renderer 先结束 surface 及有关原生使用，再释放 lease；GC release 不调用窗口 API。lease 不授权在隐藏、零尺寸、待配置或 Closing 时提交帧。

NativeApplicationLease.acquireWindow 可接收可选 requestClose 回调，服务独立桌面资源的窗口绑定。窗口进入 Closing 时将回调排入 owner 收尾队列，最多交付一次；借用者必须停止原生使用并 close lease。显式先关闭 lease 会取消未交付回调，回调只负责现有资源收尾，不取得新的窗口借用。原有不带回调的调用行为保持不变。

## 7. 窗口及应用关闭

`Window.requestClose(): Boolean` 从业务代码排队一次普通 CloseRequested，供自绘窗口关闭按钮沿同一退出确认入口工作；它不直接 requestDestroy。返回 true 表示已接受排队。窗口受模态限制、关闭按钮禁用或应用不再接受业务工作时返回 false；交付时再次检查窗口仍可处理关闭请求，失效后不交付旧请求。实际销毁继续由应用明确决定。

独立托盘 Cone 所需的 NativeApplicationLease、显示器 workArea 及系统栏查询合同见[托盘规范](SCOOP-TRAY-SPEC.md)。这些入口复用既有 owner／mailbox／关闭通路；window 不引入托盘类型或 Linux 托盘依赖。

状态为 PendingConfigure／Open／Closing／Destroyed。CloseRequested 只表示意图；requestDestroy 幂等、非阻塞，先进入 Closing／通知并拒绝新业务，等 Closing 已交付、callback 和 lease 均退休后实际销毁并发一次 Destroyed。Closing 后不再交付业务输入／重绘；迟到原生事件按 id 和 phase 丢弃，Destroyed 为该窗口的最后事件。

ApplicationOptions.lastWindowPolicy 为 KeepRunning／Exit，默认 KeepRunning。退出先停止业务投递，发送 Exiting／Closing；随后继续服务 native cleanup、窗口销毁和可选 I/O 根收尾。所有窗口、watch、native callback、集成 session 均完成后，才停止主循环并返回／抛错。清理错误不阻止其他资源关闭，按 ResourceUseException 保留正文与清理失败。

在用户显式退出、handler 失败、安装失败和显示连接失联中都必须消费已经取得的资源。OS 强制终止和无法返回的 native 工作不承诺完整或有界关闭；不能通过 GC 或提前关闭句柄伪装成功。

## 8. asyncio 集成

Application.runWithAsyncio(handler) 是独立 Cone 的扩展，复用 eventLoop().attach 与 EventLoopSession。业务 handler 及 proxy 消息在 session 根 Context 和 owner 的 LoopExecutor 上运行，currentEventLoop 与 currentApplicationScope 可用。应用作用域保存该 executor；从其他 executor 调用捕获的应用作用域 launch／async，默认子任务仍排队到 owner。调用方须在发布前准备好任务状态，不把 launch 返回当成正文已经执行。根取消及时触发窗口收尾控制，不等 onClosed 才开始释放窗口；退出时 requestClose，继续原生循环至 onClosed。Exiting／Closing／Destroyed 在 owner 收尾 Context 执行，没有活动业务 scope。用于派发通知的内部唤醒不得循环生成 NewEvents／AboutToWait。

原始 Application.run 不初始化 io_uring；集成入口继承 asyncio 的平台要求。应用作用域的协程在任意真实挂起后都由保存的 execution 排队回 owner，包括第三方 continuation、Channel 和 blocking 的完成。显式 withExecutor／launch(executor) 可将工作交给后台 executor；EventLoop binding 仍表示 I/O 服务归属，其完成返回调用者的 executor。后台协程通过 withExecutor(loop.executor) 执行 owner 操作；普通线程／native callback 使用 proxy，或向已捕获的应用作用域启动任务。普通 Context binding 本身不切换线程，Canvas、窗口和可变 UI 状态仍只能由 owner 操作。

`scoop.window.native` 为当前集成 Cone 提供以下公开低层接口，不暴露 internal owner：

- `@Unsafe Application.runAttached(handler, attach: (ApplicationHost) -> ApplicationAttachment)` 与 run 共用单次入口；原生宿主可服务后、首个业务事件前调用 attach。安装失败回滚所有原生资源。
- ApplicationAttachment 是普通接口，`post(block: () -> Unit): Boolean` 投递业务闭包，`requestClose()` 请求现有 session 收尾；基础库不依赖 CoroutineScope／EventLoop 类型。
- ApplicationHost 提供 `isCurrentThread()`、`@Unsafe watchReadable(descriptor: Int, ready: () -> Unit): ApplicationWatch` 和 `@Unsafe watchWindowsEvent(event: Ptr<Unit>, ready: () -> Unit): ApplicationWatch`；选择与实际后端不匹配的通知在借用前报参数异常。监听不读取、复位或关闭借入 fd／HANDLE，不内联 ready。ApplicationWatch.remove 在 owner 上注销，当前 ready 退出、排队通知退休后回调一次。
- `postControl(block: () -> Unit): Boolean` 在关闭业务队列后继续接受收尾工作；控制闭包用 `requestExit()` 在 owner 开始关闭。`complete(failure: Throwable?)` 由 session.onClosed 在 owner 调用一次。无窗口时仍等待附着完成、watch 注销和已接受控制工作执行完。

WindowLoopHost 将这些实际操作适配为[asyncio 规范 11.3](SCOOP-ASYNCIO-SPEC.md#113-宿主接入合同)的 EventLoopNotification／watch。根取消订阅只排入控制队列，不跨线程直接操作窗口；集成 scope 仅在业务回调期间绑定。通知派发与纯 I/O／watch 维护唤醒不自行产生新一轮 NewEvents／AboutToWait，避免空闲自唤醒。

## 9. 构建与验收边界

Darwin 使用语言规范 12.2.1、实现规范 2.7 的普通 native Objective-C `.m`／ARC 与 framework 声明。AppKit backend 由 NSApplication 正常循环驱动，以 stop 和唤醒返回；不调用 terminate。通知监听以 dispatch READ source 观察 fd，通过 common／tracking／modal modes 的 CFRunLoopSource 在 owner 派发；等待消费时暂停重复通知，注销先平衡暂停并等待 cancel handler，再退休 owner source。渲染层自行管理实际 layer／surface。

本阶段以所属 Cone 的 scoop.test 验证值、输入归一、API、真实 AppKit／Win32、native callback 与 asyncio 收尾；native 编译／链接／诊断使用必要的 compiler fixture。真实 surface 示例用单独 renderer Cone 消费 native lease。显示器热插拔、混合 DPI、输入法候选交互与真实用户拖动等依赖实际硬件／桌面输入的验收单独记录，不能用合成事件冒充。Linux／其他平台验收留待后续。

## 10. Win32 后端

Windows 窗口使用 Unicode HWND，创建前确认 Per-Monitor V2；已声明其他 DPI 模式时明确失败，不改变现有 UI 的坐标合同。每次 WM_DPICHANGED 使用建议窗口矩形并重新读取内容像素尺寸，原子发布完整 metrics。内容逻辑坐标以窗口 DPI 换算；DesktopPosition／DesktopRect 使用物理虚拟桌面坐标。最小化提供零 drawable，隐藏／最小化期间保留 dirty；Windows 不提供普通顶层窗口完整遮挡查询，isOccluded 只报告系统可确认的最小化或 DWM cloaked 状态。

通知的启动入口也会在 Application 之前创建 COM／消息窗口，因此与应用创建共用原生 DPI 准备函数，先确定 Per-Monitor V2 再创建任何 HWND。后续应用创建读取同一个线程／进程模式，不以已有隐藏窗口为理由改变 DPI 合同；见[系统通知规范](SCOOP-DESKTOP-NOTIFICATION-SPEC.md)。

窗口属性通过正常 Win32／DWM API 实施，原生返回值在边界保留并转换为 WindowException。native 薄层的创建结果同时返回资源和状态码；setter 以状态码报告失败，不使用另一个错误缓存。Windows 8 起 DwmEnableBlurBehindWindow 不产生模糊效果，setBlur(true) 返回 Unsupported 且无副作用；要求 blur 的创建失败。主题建议使用 DWM 支持的暗色标题栏属性；缺少该属性时返回 Unsupported。透明背景供实际 renderer 使用，不承诺所有渲染设备都有 alpha 支持。内容保护为 WDA_MONITOR 系统提示，仍返回 Requested。

键盘的物理位置来自扩展 scan code，NumLock 与 Pause 按 Win32 的扩展位区分；Lang1／Lang2 不混同 Kana／转换键，当前键集合没有对应项时保留 WindowsScanCode。同值的 VK_KANA／VK_HANGUL 只在 scan code 明确识别 Kana 时命名，否则保留 WindowsVirtualKey。逻辑文字查询当前布局且不改变系统 dead-key 状态。普通文字由 WM_CHAR／WM_UNICHAR 交付；完整 IME 文档连接只使用 TSF，不保留 IMM32 组合／候选窗后端或运行时降级。TSF 提交、WM_CHAR 与 WM_IME_CHAR 不得重复交付同一文本；失焦／禁用取消组合并补发已交付按下键的 synthetic release。

owner 线程持有 ITfThreadMgr 和 TfClientId，每窗口管理 ITfDocumentMgr、context stack 与焦点。应用实现 ITextStoreACP 及必要的 composition／edit sink，系统提供 ITfContext。COM/C++ 对象不跨 Scoop ABI。连接启用、焦点、禁用、restart 与销毁均核对 connection generation；旧连接的迟到回调不能修改新状态。当前文档模型为纯文本、单选区；不为 TSF 富文本属性、embedded object 或多选区增加另一套公共模型，无法由公共批次表达的操作原子拒绝。

ITextStoreACP 的 ACP 范围是 UTF-16 单元范围，在 native 边界转换为 Scoop UTF-8 字节范围，切断 surrogate 或越界时拒绝整个操作。RequestLock 遵守 TSF 同步／异步锁规则；锁内只访问原生快照或临时投影，不调用 managed handler、布局或挂起。每个原子编辑事务按实际顺序投影文本操作，再生成一个带 connection／sequence 的 TextInputBatch；显式范围携带快照 baseRevision，无快照或范围未知时返回 unavailable，不能退回局部猜测。无效事务不部分交付；publishTextInputState(after=origin) 后才推进已处理序号，事务外存在未确认编辑时 surrounding／caret 查询不可用。

composition 的开始、范围更新、提交、取消和 unmark 归一到现有 Preedit／Commit／ClearPreedit 操作；取消删除应用正在显示的组合，unmark 保留正文并仅清除组合标记。一个连接只有一个 composition 写入者。候选窗和点命中使用当前 revision 对应的已发布 caret，经客户区／DPI 转换回答 GetTextExt／GetACPFromPoint；文档变化、待确认编辑、连接结束、窗口尺寸或 DPI 变化立即使旧几何失效。

宿主使用 MsgWaitForMultipleObjectsEx 与 owner HWND。线程池 wait 一次只投递一个 HWND 就绪消息，owner 回调退出后重新监听；remove 停止 native wait 并等候 worker 退出，在 owner 排队消息和活动 ready 退休后报告完成，HANDLE 始终由 asyncio loop 持有。截止时间只安装所需原生 timer，Wait 不做周期轮询。真实系统 dialog、menu、move／size 循环继续派发 HWND 消息，因此能推进 asyncio；会进入模态的操作排在当前业务回调和 session pump 返回后。

## 11. Surface 基础层所需输入扩展

本节是 [Surface 基础架构](../surface/DESIGN.md)所需的输入合同。共享表示、AppKit 后端及 Cone 测试已实现；已有 Win32 后端同步 ABI 和事件映射，验证范围见 [实施记录](../surface/PROGRESS.md)。绘图、段落布局、动画、区域命中和帧源仍属于 graphics／surface，边界见 [Surface 规范](SCOOP-SURFACE-SPEC.md)。

### 11.1 鼠标序列取消

新增 PointerEvent.Cancelled，携带 PointerCancelReason 与 InputTime。当前窗口鼠标模型只有一个聚合指针序列，取消结束该窗口已经交付的整个按下按钮集合，不伪造未知位置、按钮 Up 或 touch contact id。理由覆盖 FocusLost、CaptureLost、SystemOperation 与 Modal；窗口 Closing 的上层清理由 GUI 生命周期通知处理，不在 Closing 后另发业务输入事件。

失焦或原生侧确认捕获／跟踪中断时，先取消仍活动的鼠标序列，再交付相应的后续状态通知。正常最后一个 Up 已结束序列时不重复取消；原生取消和失焦归一只结束一次。PointerEvent.Left 仅表示离开内容区，滚轮 ScrollPhase.Cancelled 仅结束滚轮相位，两者均不替代鼠标序列取消。

AppKit 至少覆盖窗口／应用失焦；对系统可观察的原生跟踪中断报告 CaptureLost。mouseDragged 与正常 mouseUp 继续原按下序列，不能用 mouseExited 代替中断。Win32 将 WM_CAPTURECHANGED／WM_CANCELMODE 中的异常丢失映射为取消；正常 ReleaseCapture 也会触发消息，必须区分正常结束，避免抢在最后一个 Up 前取消。同一窗口已经持有捕获时，继续按下其他鼠标按钮沿用该捕获，不重复调用 SetCapture 而打断现有按钮集合。这里不增加 Confined／Locked 光标能力，也不把鼠标序列固定目标的 GUI 路由移入 window。

### 11.2 输入单调时间

新增 InputTime，区分 `Event(TimeMark)` 与 `Observed(TimeMark)`：前者是可靠映射到 MonotonicTimeSource 的原生事件时间，后者是原生接收边界采样的同源时间。两者均可与该单调源的 TimeMark 比较，但调用方可以识别其来源；Event 不等于保证硬件采样精度。

PointerState 与 KeyEvent 均携带 InputTime，取消事件独立携带；平台缺少时间信息、无法可靠归一或生成 synthetic 事件时使用 Observed。不能用晚到业务 handler 的处理时刻替代原生接收时间，不用墙上时钟，也不把预测呈现时间当作输入时间。

原生秒／纳秒读数通过既有 Duration、TimeMark 与 MonotonicTimeSource 构造，不新增 runtime 时钟或窗口专用 TimeSource。NSEvent.timestamp、Windows 消息时间等只有完成来源与原点转换后才能成为 Event；不能把外观相似的计数直接互减，不能假定系统睡眠前后仍使用同一未经验证的偏移。首批后端明确交付 Observed 也符合合同。

输入时间随原始 payload 保存；合并连续移动时保留最终被交付样本的位置、scale 与时间，不跨按钮／焦点／取消等顺序边界合并。相关验证覆盖来源标记、同源运算和排队延迟，不以精确计时断言替代事件顺序验收。


## 12. Linux Wayland 与 X11 后端

Linux 桌面接入首先面向 `x86_64-unknown-linux-gnu`，复用第 2～8 节的应用、事件、关闭、lease、deadline 和 asyncio 合同。`SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 显式选择后端；未设置时有 WAYLAND_DISPLAY 则选择 Wayland，否则选择 X11。显式选择或已选显示连接失败正常报错，不静默改用另一个显示服务器。`WindowBackend` 报告实际 Wayland／X11；原始 Application.run 不初始化 io_uring。

Linux 窗口由 GTK 4.18 及以上与 libadwaita 1.4 及以上承载，X11／Wayland 仍是实际显示协议。装饰窗口使用 `AdwWindow`、`AdwToolbarView` 与 `AdwHeaderBar`；标题栏、圆角、阴影与桌面窗口按钮遵循 libadwaita。内容区保留 Scoop 自绘，不把普通组件改成 GTK 控件。窗口库本身不创建 GPU context，渲染器在取得 native lease 后挂载内容 widget。

GTK 的 GMainContext、eventfd 唤醒、最近 deadline 和借入的 readable fd 共用 owner 上的 poll 等待，不增加轮询线程。GTK signal 只更新原生快照或排队事件；业务 handler 在 GMainContext dispatch 之外执行，剪贴板的同步读取也不递归进入业务 handler。watch 不读取或关闭借入的 descriptor，remove 在活动回调及本轮就绪快照退休后完成。

GTK 在首次内容分配前取得的焦点先保存在原生快照中；首次 GeometryChanged 使窗口进入 Open 后，再交付当前 Focused 状态。Surface 的初始逻辑焦点因此能与 OS 焦点一致，不要求用户先切换到其他窗口再返回。

窗口 metrics 使用 GTK 实际分配的内容尺寸，排除标题栏和阴影；首次分配前为 None。物理尺寸采用内容 widget 的整数 scale factor，与 GtkGLArea framebuffer 相同；分数桌面缩放由 GTK／compositor 合成，不把它再次乘入 framebuffer。显示器信息来自 GdkMonitor，输出移除时身份退休。Wayland 没有全局窗口位置或主显示器约定，相应查询返回 None。

X11 的 outerBounds 查询实际原生顶层外框：沿 X11 窗口树取得 WM frame，读取其当前几何并转换到 GDK 桌面坐标；不能取得时返回 None。位置请求也以外框左上角的桌面坐标表达，在 WM 声明支持时使用 EWMH `_NET_MOVERESIZE_WINDOW` 的 NorthWest gravity，否则沿现有 X11 移动请求交给 WM。请求坐标先转换为 X11 原生单位，成功仍只代表 Requested。托盘定位使用这份实际外框尺寸，不以内容区冒充窗口尺寸。

GTK 不提供的窗口控制明确返回 Unsupported，包括内容保护、背景模糊、独立窗口的强制明暗主题、非默认 resize increments、最大内容尺寸约束。默认创建值不要求这些能力；调用方显式要求不支持的创建属性则创建失败。Requested 只表示已发出有效原生请求，不能把 desired state 当成观察结果。X11 的全局位置、层级与 attention 使用其原生窗口管理协议；Wayland 对没有对应协议的操作仍返回 Unsupported。

默认内容背景由 libadwaita 提供，透明窗口通过去掉该窗口和内容区背景实现；窗口圆角和阴影仍由 GTK 合成。renderer 负责正确的预乘 alpha 像素。X11 透明需要实际合成器支持，window 不自行启动合成器。

物理键使用 Linux evdev 编码，未知物理键为 NativeKeyCode.LinuxEvdev，未知逻辑键保留 XkbKeysym。逻辑键和布局来自 GDK 键盘事件，普通组合键与输入法使用 GtkIMContext；正文只经 TextInputBatch 交付一次。失焦取消组合和鼠标序列，并补发已经交付按下键的 synthetic release。重复键和指针滚动节奏由 GTK 处理，不另起重复计时器。输入保留原生接收边界的 Observed 单调时间。

文本剪贴板使用 GdkClipboard 的 MIME 表示，读取实际所有者的内容，不以本地副本伪造成功。多个文本表示由同一个 content provider 提供，纯文本同时使用 GDK 的标准字符串转换。GtkIMContext 的 commit、preedit 与 delete-surrounding 接入已有编辑批次；surrounding、selection 与 caret 只使用匹配连接／revision 的已发布快照。GTK 仅提供其实际输入法能力，Scoop 不重写 XIM 或 Wayland text-input 协议。

Linux native handle 为 `Gtk(window: Ptr<Unit>, view: Ptr<Unit>)`，分别是 GtkWindow 与可挂载渲染 widget 的内容容器。它们是借用对象，lease 保活窗口与显示连接；需要原始 GDK／X11／Wayland 对象的调用方通过 GTK 公共接口取得，不能绕过 GTK 向其 surface 单独提交 EGL buffer。renderer 先卸载内容 widget 和帧回调，再释放 lease。macOS AppKit 和 Windows Win32 的句柄及实现不变。

按真实桌面协议分别验证创建／属性／多窗口、DPI、输入／编辑、剪贴板、deadline、watch、跨线程投递和延迟 lease 关闭；合成事件仅证明归一与路由。显示器热插拔、真实输入法候选窗与硬件刷新行为单独记录，不能用合成事件声称完成。

### 创建配置的接收者入口

构造配置遵守 [库 API 风格](SCOOP-API-STYLE.md)：接收者回调同步执行一次，标量赋值替换、嵌套块基于当前值修改，异常不发布部分结果。以下结果的 copy(configure) 返回独立描述，原值不变；资源、模型与普通／suspend 函数按原有引用合同保存，构造不执行这些动作。

| 结果 | 构造入口与全部配置字段 |
| --- | --- |
| WindowAttributes | `windowAttributes(configure: WindowAttributesBuilder.() -> Unit = {})`；title: String = "Scoop"；size: LogicalSize = LogicalSize(800.0, 600.0)；visible: Boolean = true；frame: WindowFrame = WindowFrame.System(WindowContentMode.Standard)；resizable: Boolean = true；transparent: Boolean = false；requestActivation: Boolean = true；constraints: WindowSizeConstraints = WindowSizeConstraints()；position: DesktopPosition? = None；resizeIncrements: LogicalSize? = None；buttons: WindowButtons = WindowButtons()；level: WindowLevel = WindowLevel.Normal；theme: Theme? = None；blur: Boolean = false；contentProtected: Boolean = false；shadow: WindowShadow = WindowShadow.System |
| ApplicationOptions | `applicationOptions(configure: ApplicationOptionsBuilder.() -> Unit = {})`；lastWindowPolicy: LastWindowPolicy = LastWindowPolicy.KeepRunning；applicationId: String? = None |

表中有新构造入口的 class 构造器内部化，不保留长参数重载、旧 companion 方法工厂或转发别名。Builder 构造器内部化。协议 struct 与少量小型完整值保留普通构造，copy 和父级配置成员直接复用同一值模型，不为它们另设顶层工厂。所有字段类型、默认策略和非法组合的领域检查仍以上文合同为准。

WindowAttributes 和 ApplicationOptions 发布只读值；完整修改通过 copy 产生新配置。macOS 的 ApplicationOptionsBuilder.macOSActivationPolicy 属性与 WindowAttributesBuilder.macOSTitlebar(TitlebarOptionsBuilder.() -> Unit) 配置平台字段；macOSTitlebar 块基于当前 flags，三个布尔字段默认 false。创建配置上的 setMacOSActivationPolicy／setMacOSTitlebar 删除，Window.setMacOSTitlebar 仍是对现有窗口的显式操作。配置不激活应用、不请求焦点、不创建窗口；Application／ActiveApplication.createWindow 的执行合同不变。
