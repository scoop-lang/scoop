# scoop.surface

本库提供窗口呈现、帧调度、动画与区域输入。上层的 [scoop.ui](../scoop.ui/README.md) 提供自绘框架，`scoop.ui.component` 使用其公开能力实现 Label／Button 等基础控件；Linux GNU 的 Wayland／X11 共用这些接口，见 [UI 设计](../../../docs/ui/DESIGN.md)。

公共字体和段落 API 已迁入 [scoop.graphics.text](../scoop.graphics.text/README.md)，已实现 StyledText／SpanStyle 和行内对象，详细合同见[设计](../../../docs/graphics/text/DESIGN.md)。Surface 继续只管理呈现、动画和区域几何，文本或 UI 宿主拥有具体内容和资源。

普通 Scoop Cone，提供共享 Surface 宿主、动画、区域输入与按目标选择的平台 renderer。macOS 与 iOS 的 Metal 渲染器共用 CAMetalDisplayLink 和 Skia Ganesh Metal。UIKit 场景借用视图自带的 CAMetalLayer，随场景激活和帧需求启停。Windows 使用 D3D12、DXGI swapchain／frame-latency waitable object 和 DirectComposition；Linux GNU 的 Wayland／X11 使用 EGL／GLES 和 Skia Ganesh。共享代码只依赖 window 的事件、配置、代理和 lease 合同。

```scoop
import scoop.graphics.*
import scoop.surface.*
import scoop.window.*

fun main() {
    val application =
        Application(applicationOptions { this.lastWindowPolicy = LastWindowPolicy.Exit })
    val host =
        SurfaceHost {
            owner,
            active,
            event -> when (event) {
                case Resumed -> {
                    val window =
                        active.createWindow(windowAttributes { this.title = "Scoop Surface" })
                    owner.attach(window) {
                        frame ->
                        frame.canvas.clear(Color.White)
                        frame.canvas.drawCircle(
                            Point(80.0, 80.0),
                            40.0,
                            Paint(Color(0.1, 0.4, 0.8))
                        )
                    }
                }
                case Window(val id, WindowEvent.CloseRequested) -> active.requestExit()
                else -> {}
            }
        }
    host.use { current -> application.run(current) }
}
```

`SurfaceWindow.requestFrame()` 合并重绘需求。空闲、隐藏、系统报告遮挡或无有效配置时暂停原生帧请求；恢复可绘制状态后继续处理 dirty。Windows 的遮挡状态遵守 window 合同，只报告最小化或 DWM cloaked，不承诺检测其他顶层窗口的完整覆盖。帧内再次请求重绘会保留到下一帧。`FrameContext.time` 使用平台单调时钟，一帧只采样一次；Canvas 仅在 painter 内有效。

`frame.withState { child -> ... }` 在同一帧中借出隔离的子 Canvas，保留窗口、尺寸、采样时间和输入收集器。child.place 使用子坐标与 clip，输入几何仍随整帧提交；正常或异常退出均恢复父绘制状态，子 Canvas 随即失效。

持有 Path／Image／文本等资源的内容可实现 `SurfaceContent : Closeable`，通过 `attach(window, content)` 把整个内容根交给 SurfaceWindow。content 提供 `paint(FrameContext)` 和幂等 `close()`；通过 owner 线程检查后即转移所有权，后续附着失败也由 Surface 关闭 content 并回滚资源。错误线程调用在接收所有权前拒绝。直接 painter 重载仍可用于不需要拥有内容资源的绘图。

content 可实现 `onFrameComplete(FrameCompletion)`，读取该帧的 time、窗口内容区 size 和 Accepted／Discarded 结果。回调位于 Canvas／frame 借用结束后；Accepted 时输入几何已发布，可请求焦点或下一帧。没有进入 paint、绘制异常或窗口关闭时不发通知，资源仍在 close 中释放。它不表示 GPU 已完成，也不会自行续订帧。

一次 paint 内的输入 scope 更新按同一个批次收尾。逻辑启用、关闭、重挂和焦点请求立即生效，路由协调与 `observeActivity` 通知在 paint 退出后交付稳定状态；不会在逐个创建或卸载虚拟项时重复通知。抛错仍执行输入收尾，成功提交后再发布本帧几何。需要读取提交结果的业务工作继续放在 `onFrameComplete` 中。

macOS 应用在附着窗口前调用 `scoop.surface.configureFpsReportingFromEnvironment()`，并以 `SCOOP_SURFACE_FPS=1` 启动，即可在 stderr 查看每个窗口实际呈现的帧率。未调用时环境变量无效。每次调用读取当前值，设置适用于随后附着的窗口。

`scoop-surface-fps` 样本使用 Metal 回执时间，`frames / seconds` 为该区间的 `fps`。样本的 `start` 和 `end` 可用于判断区间是否跨过静止期。窗口静止时探针随呈现停采，关闭时输出累计有效回执数 `presented_total`、无有效时间的回执数 `without_time_total` 和 `last_presented`。

关闭 SurfaceWindow 只卸载绘制层，Window 可继续存在或重新 attach。关闭 Window 或 Application 则由 SurfaceHost 自动退休 Surface 资源。SurfaceHost 事件处理器、paint 或输入回调内关闭先进入 Closing，整个业务回调和 Canvas 借用结束后，依次关闭 content、renderer 和 lease；不会在回调内等待自己。内容清理失败仍继续释放其余窗口与底层资源，并沿 Application 的既有异常通路传播。

同一个 SurfaceHost 可直接用于 `scoop.window.asyncio` 的 `runWithAsyncio`；Surface 不创建第二套主循环或任务树。应用负责 CloseRequested 策略、内容内部的资源组织及显式状态更新，最外层 host.use 覆盖所附着的全部内容根。

Windows 保留原 HWND、窗口过程、子窗口和 IME context，以 BGRA8／sRGB／预乘 alpha 呈现。DXGI 后台等待只向 owner 消息窗口投递一次通知，再经已有 proxy 请求 redraw。普通帧异步提交；尺寸变化先退休旧 frame，再释放 GPU 对旧 backbuffer 的使用并调整 swapchain。painter 内发生的原生尺寸变化会使当前帧放弃呈现和区域发布。关闭取消等待并移除待处理通知，排空 GPU 后卸载合成目标，最后释放 lease。

Windows 支持 `x86_64-pc-windows-msvc-ucrt`、`x86_64-w64-windows-llvm-mingw-ucrt` 和 `x86_64-w64-windows-llvm-mingw-msvcrt`，分别使用相容的 Skia／CRT 资源。应用消费已准备的 sysroot；维护者按[准备说明](../../../scripts/README.md)构建 native 依赖。设备或呈现错误以 GraphicsException 报告 operation 和可得的 HRESULT；不自动重建 device 或更换 renderer。

`SurfaceWindow.animations.value(initial)` 创建由窗口持有的 Animatable，支持 Double、Point、Size、Color，也可传入 AnimationConverter。`animateTo` 接受 Tween 或 Spring，可指定 Repeat；pause／resume、cancel、reverse 和 snapTo 明确控制生命周期。spring 改目标保留速度，恢复显示后按实际经过时间继续。窗口关闭会关闭其所有动画，并取消尚未交付的完成回调。

`animateTo(target, spring, initialVelocity = Some(AnimationVector(vx, vy)))` 可把交互测得的速度传给首段 spring，单位为转换器向量分量每秒。None 保留当前采样速度，Some(零向量) 从静止开始；显式速度必须有限，Tween 不接受该参数。即使当前位置等于目标，非零速度仍会先运动再收敛。完成处理器使用 named onFinished 或最后的 trailing lambda。

已知内容不参与当前场景时可设 `animation.isFrameEnabled = false`，时间仍然推进。有限动画通过窗口所属的一次性期限完成，无限重复动画不产生离屏周期唤醒；恢复 true 时直接采样当前进度并请求帧。窗口整体隐藏／遮挡时也使用相同的有限完成机制。`hasWork` 包含所有活动动画，`hasFrameWork` 只表示启用帧的工作；Surface 只按后者要求连续帧。

`timeline.group(frameEnabled)` 创建同一时间轴所属的 AnimationGroup，再用 `group.value(initial)` 创建其动画。组的 isFrameEnabled 与各动画自己的标记共同决定帧需求，切换组不会覆盖动画的本地选择。关闭组会关闭全部成员，关闭时间轴也会关闭仍为空的组；组不创建独立时钟或事件循环。

离屏使用 `AnimationTimeline(invalidate)`，通过 `advanceTo(time) { ... }` 在给定的 MonotonicInstant 启动或调整动画，再用 `advanceTo(next)` 采样下一帧。同一轮全部值采样后才调用完成处理器，处理器中新启动的动画从该帧时间开始、后续帧继续。Tween 的 easing 包含线性和 cubic Bézier；spring 以质量 1 的解析解求值，不累加固定 dt。Size 和 Color 的结果限制在各自合法范围。Repeat 的次数计完整段数，Reverse 每段交换端点；零时长可以直接完成，但不能重复。

`SurfaceWindow.createArea` 创建窗口持有的稳定 InputArea，`area.scope` 是它的逻辑输入节点。`createScope` 可建立没有几何的祖先，二者的 parent 缺省为 `SurfaceWindow.input`。在 painter 内调用 `frame.place(area, rect或path)`，沿 Canvas 当时的变换和 clip 记录本帧几何；只有渲染提交成功后才整体发布。parent 在挂载逻辑中设置，与绘制顺序独立，不自动裁剪子区域。区域不再放置、scope 禁用或关闭后退出交互；所属窗口关闭时关闭其全部输入节点。

`SurfaceWindow.hitInput(point)` 以窗口逻辑坐标查询同一份 Accepted 输入场景，返回前景目标到当前 AllInput 域根的 `Array<InputHit>`。每项保存 scope 和该次冻结的 localPosition；没有几何的 scope 为 None。它遵守真实路径、变换、裁剪和当前资格，忽略普通鼠标 capture，也不派发事件、移动焦点或更新 hover。非有限坐标、未命中及已关闭窗口返回空数组；FocusOnly 不限制此查询。数据拖放等独立输入协议可以复用它，不能把业务拒绝解释为穿透同级背景。

`SurfaceWindow.createWindowRegion(kind, parent)` 登记原生窗口区域，kind 为 `WindowRegionKind.Shape`、`Input` 或 `Hit(WindowHitRole)`。在同一 FrameContext 中用 `place(handle, rect或path)` 捕获最终变换及 clip，按当前 scaleFactor 转为 WindowRegion；只在 RenderFrame Accepted 后发布，Discarded 保留上一有效快照。Shape／Input 每个 Surface 各至多一个，不能传输入 parent；Hit 可归属于同窗口 InputScope，禁用、AllInput 边界或卸载立即撤销其资格。未放置恢复默认覆盖，放置空覆盖则发布空区域；尺寸、DPI 和窗框改变会撤销旧快照。Surface 关闭恢复自己曾发布的默认区域，借用的 Window 仍可存活。能力限制与完整合同见[原生区域规范](../../../docs/specs/SCOOP-SURFACE-FRAME-SPEC.md)，真实 UI／renderer 组合在 [UI desktop 测试](../scoop.ui/tests-cones/desktop/README.md)中覆盖。

事件处理器接收 `(RoutedEventContext, InputEvent)`，返回 Unit。InputScope 的 captureHandler、handler 和 defaultAction 分别接收 capture、target／bubble 和默认动作。`stopPropagation()` 只停止传播，`preventDefault()` 只取消默认动作，`consume()` 同时执行两者。默认阶段从目标向根执行；主按钮按下的聚焦也在这一阶段，可由父级取消。窗口有 OS 焦点而没有区域焦点时，键盘事件从根 scope 开始。

上下文的 target／currentTarget 是逻辑 scope，localPosition 在带几何的区域指针或原生手势事件中存在，窗口位置保留在 InputPointer.state 或 GestureEvent.state；上下文随同步回调结束而失效。Enter／Leave、焦点、修饰键与取消是不可取消的通知，文本只给对应 IME 会话。首次按钮按下固定整次多按钮序列的路径，空白区域按下固定根路径；拖出窗口仍可继续，取消向旧路径完整交付，剩余按钮事件不会改投其他区域。滚轮区分行与逻辑像素。`onEventFinished(InputEvent)` 在每次可路由输入之后通知原路径内仍有效节点，即使传播或默认行为被取消；控件用它结束释放序列，不执行被取消的激活。

`InputEvent.Gesture` 保留原生识别结果；Magnify／Rotate 的 Began 各自固定起始命中路径，后续更新使用当前 Accepted 坐标，None 相位与 Swipe 独立命中。禁用、卸载、模态、失焦或关闭取消活动路由，迟到更新不会转交兄弟区域。手势不移动鼠标 hover 或焦点；普通 Capture／Target／Bubble／Default 与完成通知仍适用。具体单位见 Window README，公共内容变换见 [UI 手势示例](../scoop.ui/examples/gestures/README.md)。

`InputBoundary(scope)` 将输入限制到该 scope 的逻辑后代，命中、焦点、capture／bubble／default 和空白处的缺省目标都停在当前域。边界按建立顺序叠放，`isCurrent` 与 `SurfaceWindow.inputBoundary` 查询当前屏障；关闭恢复前一域。域改变会完整取消原来的按下路径，剩余按钮事件保持空路径。域外的区域不必有场景几何，但逻辑祖先的禁用／关闭仍然生效；`scope.isInputAllowed` 可在首次放置前判断准入。上层通过 `SurfaceWindow.reserveFocusRequest()` 为待处理或被拒绝的显式焦点意图推进同一个 focusRevision，不改变当前焦点，也不改写 InputArea 的成功历史。

边界所属 scope 已退休时，屏障仍持续到 InputBoundary.close，便于 UI 等待弹层移除帧成功提交。当前域根的 `beforePress` 在首次鼠标按钮按下或每个触点开始的派发前读取 InputPress，返回 true 消费整段按下序列，返回 false 沿已固定的目标和坐标继续。InputPress.source 区分 Mouse 和 Contact。外部按下策略可以据此关闭最上层弹层；同一鼠标序列的追加按钮不再次执行预处理，回调中改变边界也不能把旧事件投向背景。

macOS／Linux 可用 `ShortcutScope(input, bindings)` 在作用域内登记快捷键，UI 可通过 `scope.resource { ShortcutScope(scope.input, bindings) }` 管理它。ShortcutKey 显式选择 logical 或 physical；ShortcutModifiers.primary 在 macOS 为 Command、Linux 为 Ctrl，完整修饰组合匹配，文字大小写比较复用 Window 的 Unicode 处理。默认忽略重复和合成输入，canExecute=false 向外层继续；无焦点时可使用窗口根。每个输入节点的普通 defaultAction 和 addDefaultAction 登记先执行，ShortcutScope 最后匹配，文本编辑命令可以先消费事件。

`HostShortcutScope(host, bindings)` 为宿主当前及后续附着的窗口共用一份应用快捷键，窗口根的普通行为与本地 ShortcutScope 先处理，宿主最后回退。登记可早于 Application.run；每个宿主只允许一个活动登记，支持 enabled 和 bindings 整体更新，关闭后可重新创建。关闭单个窗口不影响其他窗口，关闭宿主立即停用。任何 InputBoundary 都阻止宿主回退，包括窗口根本身作为边界；需要在模态内执行的动作应登记到该域。

`createArea(focusable = true)` 允许逻辑焦点，`textInput = true` 同时允许焦点和 IME。区域已成功放置且有非空可见几何时，canFocus 为 true，requestFocus() 可以聚焦；它不抢占系统窗口焦点。windowBounds／visibleBounds 查询最近成功帧的窗口逻辑包围盒，后者计入路径裁剪及视口。已聚焦区域仅被 clip 裁掉时保持会话，移除、禁用或关闭仍清除焦点。`hasFocus` 表示保留的逻辑焦点，`isFocusActive` 还要求 OS 窗口当前获得焦点。文字只从 TextInputBatch 应用到编辑器自己的状态，不能再由 LogicalKey.Character 插入。

`area.setCaret(rect, revision)` 把可空的局部 caret 和文档修订随放置捕获；成功帧才发布其变换与实际区域／路径裁剪。新 caret 尚未成功呈现、几何失效或全部被裁掉时，macOS 候选框查询返回 unavailable；`visibleCaretBounds` 查询同一结果，`toLocal`／`toWindow` 用当前成功放置映射子区域。只读切换可设置 `textInput = false`，结束 IME 并保留键盘焦点；`restartTextInput()` 等待实际属于该区域的旧连接 Disabled 后重新开启，返回是否存在这一旧会话。返回 false 时，新区域的首次 Enabled 可以直接使用。焦点切换和重启期间的已排队编辑保留原归属，旧连接尾部不会交给新接收者。

[组合示例](examples/basics/README.md)演示旋转 Path、hover、拖动 spring、文本布局及 IME 接收。它是普通 executable Cone，编辑状态由示例管理，未依赖 component。

同一文本从浏览区域移交给新编辑区域时，可显式调用 `target.continueTextInputFrom(source)`。来源须是同窗口当前已开启连接的活动文本区域；返回 true 只登记交接，不移动焦点。下一次普通聚焦到目标时，目标先收到同一连接的 Enabled，来源再收到 Disabled，原生连接与组合继续有效。双方须共享文档及 revision，Surface 不复制正文。其他焦点去向、失焦、禁用、注销和 restartTextInput 取消旧登记；普通无关联编辑器仍先关闭旧连接。UI 的 TextInputStarter 封装了文档与宿主交接，本机使用见 [text-start](../scoop.ui/tests-cones/text-start/README.md)。

`InputArea.focusRevision` 返回该区域最近一次成功聚焦的窗口序号；未曾聚焦或关闭后为 None。它与 SurfaceWindow.focusRevision 共用顺序，鼠标、重复成功请求及直接程序请求一致，失败和 OS 激活变化不重写成功历史，供上层导航恢复最近使用的成员。

滚轮默认行为可读取 `context.scrollRemaining`，并调用 `context.consumeScroll(lineStep) { localDelta -> ... }`，返回该容器实际消费的局部位移。lineStep 由容器提供；Surface 按本次事件固定的区域几何换算，消费后将残量还原到 Lines／Pixels，父容器再使用自己的步长和变换。Pixels 已完成一次 DPI 归一。原事件始终保留完整 delta，stopPropagation 不取消默认滚动；Contain 可在处理本层后调用 preventDefault。

`consumeScroll` 与 `consumeContactMovement` 可在 apply 之前提供 `onRemainder = Some { remainder -> ... }`。最后一次成功的非 None 登记，在全部普通默认消费之后、事件完成通知之前运行一次，使用该接收者冻结的几何与单位，返回真正吸收的同向位移；零余量也通知。普通 None 消费者不撤销登记，Contain 的 preventDefault 保留本层已登记回调。路径失效／关闭或普通处理抛错时跳过，完成清理仍执行；不能保留 RoutedEventContext 给这个回调使用。UI 用此入口在父级消费之后处理最终弹性边界。

连续 phase 和其后的 momentum 固定起始路由，接触结束后仍可沿原路径进入动量；普通离散 Wheel 按当次命中。源区域退休或模态切换用 ScrollCancelled 完整通知旧路径，后续动量保持空路径直到结束；它与鼠标按钮 Cancelled 分开，不会误结束另一段拖动。等待可能的动量不维持帧需求。

验证使用以下普通测试 Cone：

- 本 Cone 的 tests：动画的确定性采样；实际 Skia 离屏几何上的区域命中、传播、拖动、焦点、IME 会话和关闭；两个真实窗口的按需重绘、空闲、隐藏／最小化恢复、resize、输入宿主保留及重新 attach。`--filter Animation` 和 `--filter Input` 可分别运行不需要桌面的测试；`--filter SurfaceHostTest` 运行真实窗口测试。`--filter SurfaceFpsReportingTest` 在独立 macOS 进程验证 FPS 的显式配置和真实呈现输出。
- `tests-cones/animation-deadlines`：隐藏窗口的有限完成、可见窗口中关闭动画帧需求、真实 display link 停止、恢复采样，以及完成回调内关闭 Surface；只运行 macOS 定向用例。
- `tests-cones/asyncio`：原生 modal／menu 循环下的帧与 I/O，Windows 还覆盖系统移动／缩放循环，以及根取消后的 Surface 收尾。
- `tests-cones/content`：附着失败及错误线程的所有权边界、回调内关闭、原生关闭、重新附着、从未绘制的隐藏内容和退出收尾。
- `tests-cones/errors`：paint 与多个 content.close 同时抛错、失败附着的回滚及全部 surface／lease 释放。
- `tests-cones/input`：在真实输入宿主上组合动画帧、区域鼠标事件、平台 preedit／commit、候选窗坐标、IME 切换和回调内关闭。Windows 的协议注入复用 window 测试 Cone 的 UTF-16 边界；该测试与安装的真实 IME 桌面操作分别记录。
- `tests-cones/text-caret`：macOS 原生文档查询与成功帧几何，覆盖文档修订、缩放／裁剪、Discarded、只读及连接重启。
- `tests-cones/shortcuts`：macOS 原生 Command、修饰键精确匹配、重复抑制、禁用与默认动作取消；窗口没有区域焦点时仍可执行绑定。独立进程遵守每进程一个 Application 的合同。
- `tests-cones/host-shortcuts`：macOS 的共享宿主登记、窗口本地优先、根模态边界、关闭首窗后附着第二窗口，以及动作内关闭宿主。
- `tests-cones/scroll`：macOS 原生行／像素滚动、嵌套残量、区域移动后的连续 phase／momentum、源失效后抑制背景滚动与正常收尾。
- `tests-cones/presentation`：Windows 桌面上用两个重叠的真实窗口，采样透明区域和半透明绘图的最终合成像素。
- `tests-cones/frame-demand`：Windows 桌面的单个 Application／asyncio session 中连续隐藏并丢弃帧，再恢复重绘，验证 DXGI 已取得的帧机会不会在停止需求时丢失；正常提交、超时和资源收尾均有断言。

可通过 `scoop test <Cone路径> --release --target <目标> --sysroot <sysroot>` 分别运行；Windows 安装后的命令名为 `sco`，仓库开发构建为 `scoop.exe`。真实显示测试需要已解锁、显示开启的桌面；显示休眠或锁屏时 AppKit 将窗口报告为遮挡，生产代码按合同停帧。当前交付状态与验证限制见 [实施记录](../../../docs/surface/PROGRESS.md)。

桌面测试须串行执行。`tests-cones/input` 还需要真实窗口焦点；macOS 可以拒绝后台进程的 activate 请求，此时在准备期将“Scoop region input”置于前台。测试给帧／焦点准备 30 秒，确认真实焦点后用独立的 5 秒期限验证输入会话，不用强制激活或伪造焦点绕过系统行为。超时会报告阶段、帧数、焦点和输入计数。需要桌面应用入口时，可将 `scoop test --no-run` 的测试可执行文件放入普通 `.app` 包装后从访达启动；无参数直接运行仍执行 scoop.test 的非 ignored 用例，设置 `SCOOP_TEST_RUN_DIR` 可记录原有结构化结果。

复用临时测试应用时，用新文件替换其中的可执行文件，并执行 `codesign --force --sign - /path/Test.app` 更新整个测试应用的本地签名。命令行前台启动可用 `open -n -W ... --args --scoop-test-filter TestClassName` 选择相关用例；以 scoop.test 报告中的实际 case 结束和 complete 为结果，单独的 open 返回成功不表示测试通过。

## Linux GNU

在已准备 GNU sysroot 的图形会话中，使用 `SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 运行应用及 `scoop test sysroot/src/scoop.surface --release`。Wayland 使用 frame callback 和 EGL swap interval，X11 使用 EGL swap interval；没有帧需求时停止请求，不建立固定刷新定时器。Wayland 的 PendingConfigure 窗口可以先附着，首次 configure 后才取得可绘制目标，隐藏重显同样等待有效配置。

窗口 surface、GPU 工作和 native lease 按关闭顺序释放，同一宿主的窗口共享 GLES context。原始 GPU 资源不能越过借用 display 的生命周期；SurfaceHost 收尾时显式释放该 context 的 native 资源。两种显示协议的真实呈现、像素读回及限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。

## 原始触点的路由与归属

`InputEvent.Contact` 保留 Began／Moved／Ended 和不可取消的 Cancelled。每个 ContactId 在首次命中时固定 Accepted 路径，移到兄弟区域也不重投，不改变鼠标 hover。`InputArea.hitTest(windowPoint)` 查询当前精确命中路径，供抬起时确认 tap；它使用形状、裁剪、变换、前景及输入边界。

`InputScope.claimContacts(ids)` 要求非空且无重复的身份数组；整组均为活动且原路径包含该 scope 才能取得归属。新归属先全部发布，再通知原后代取消。回调失败或收尾时归属失效会取消本次新取得的触点，保留调用前已经拥有的其他触点；正在 Ended 中的接触不可新接管。`cancelContactClaim(id)` 只撤销本 scope 拥有的一个触点。普通阶段仍允许多个候选观察同一序列，识别策略由 UI 决定。

嵌套直接滚动在 Contact.Moved／Ended 的 Default 阶段调用 `context.consumeContactMovement { localDelta -> ... }`，返回本层实际消费的局部接触位移。每次事件须由当前接触归属者先开放共享，祖先再消费 `contactRemaining`；普通 claim 不自动开放共享，因此子级 Slider／Drag 不会同时带动父滚动。首次消费包含从 Began 累计的阈值内距离，随后使用相邻输入样本的增量；每层沿本次固定的 Accepted 几何转换起止点，使用与滚轮相同的方向、有限值和不超量检查。Contain 在处理本层后 preventDefault；取消立即停止后续消费。具体消费者见 [UI ScrollPan](../scoop.ui/README.md)。

`InputActivity.hasContact` 及 observeActivity 跟踪触点身份和所属 scope，让虚拟子项在活动期间保持；移动不触发归属通知。取消、模态、失焦、路径失效和关闭均从原路径收尾，迟到更新不重新命中。实际使用与组合测试见 [UI Press／Drag](../scoop.ui/tests-cones/press-drag/tests/contact.scoop)。
