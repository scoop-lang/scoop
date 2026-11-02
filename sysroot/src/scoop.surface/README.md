# scoop.surface

本库提供窗口呈现、帧调度、动画与区域输入。上层的 [scoop.ui](../scoop.ui/README.md) 已在 macOS aarch64 提供不含现成控件的自绘框架；后续 `scoop.ui.component` 使用其公开能力实现 Label／Button 等基础控件，见 [UI 设计](../../../docs/ui/DESIGN.md)。

公共字体和段落 API 已迁入 [scoop.graphics.text](../scoop.graphics.text/README.md)，已实现 StyledText／SpanStyle 和行内对象，详细合同见[设计](../../../docs/graphics/text/DESIGN.md)。Surface 继续只管理呈现、动画和区域几何，文本或 UI 宿主拥有具体内容和资源。

普通 Scoop Cone，提供共享 Surface 宿主、动画、区域输入与按目标选择的平台 renderer。macOS 使用 CAMetalLayer／CAMetalDisplayLink 和 Skia Ganesh Metal；Windows 使用 D3D12、DXGI swapchain／frame-latency waitable object 和 DirectComposition。共享代码只依赖 window 的事件、配置、代理和 lease 合同。

```scoop
import scoop.graphics.*
import scoop.surface.*
import scoop.window.*

fun main() {
    val application = Application(ApplicationOptions(LastWindowPolicy.Exit))
    val host = SurfaceHost { owner, active, event ->
        when (event) {
            case Resumed -> {
                val window = active.createWindow(WindowAttributes("Scoop Surface"))
                owner.attach(window) { frame ->
                    frame.canvas.clear(Color.White)
                    frame.canvas.drawCircle(Point(80.0, 80.0), 40.0, Paint(Color(0.1, 0.4, 0.8)))
                }
            }
            case Window(id, WindowEvent.CloseRequested) -> active.requestExit()
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

关闭 SurfaceWindow 只卸载绘制层，Window 可继续存在或重新 attach。关闭 Window 或 Application 则由 SurfaceHost 自动退休 Surface 资源。SurfaceHost 事件处理器、paint 或输入回调内关闭先进入 Closing，整个业务回调和 Canvas 借用结束后，依次关闭 content、renderer 和 lease；不会在回调内等待自己。内容清理失败仍继续释放其余窗口与底层资源，并沿 Application 的既有异常通路传播。

同一个 SurfaceHost 可直接用于 `scoop.window.asyncio` 的 `runWithAsyncio`；Surface 不创建第二套主循环或任务树。应用负责 CloseRequested 策略、内容内部的资源组织及显式状态更新，最外层 host.use 覆盖所附着的全部内容根。

Windows 保留原 HWND、窗口过程、子窗口和 IME context，以 BGRA8／sRGB／预乘 alpha 呈现。DXGI 后台等待只向 owner 消息窗口投递一次通知，再经已有 proxy 请求 redraw。普通帧异步提交；尺寸变化先退休旧 frame，再释放 GPU 对旧 backbuffer 的使用并调整 swapchain。painter 内发生的原生尺寸变化会使当前帧放弃呈现和区域发布。关闭取消等待并移除待处理通知，排空 GPU 后卸载合成目标，最后释放 lease。

Windows 支持 `x86_64-pc-windows-msvc-ucrt`、`x86_64-w64-windows-llvm-mingw-ucrt` 和 `x86_64-w64-windows-llvm-mingw-msvcrt`，分别使用相容的 Skia／CRT 资源。应用消费已准备的 sysroot；维护者按[准备说明](../../../scripts/README.md)构建 native 依赖。设备或呈现错误以 GraphicsException 报告 operation 和可得的 HRESULT；不自动重建 device 或更换 renderer。

`SurfaceWindow.animations.value(initial)` 创建由窗口持有的 Animatable，支持 Double、Point、Size、Color，也可传入 AnimationConverter。`animateTo` 接受 Tween 或 Spring，可指定 Repeat；pause／resume、cancel、reverse 和 snapTo 明确控制生命周期。spring 改目标保留速度，隐藏期间不采样，恢复显示后按实际经过时间继续。窗口关闭会关闭其所有动画，并取消尚未交付的完成回调。

离屏使用 `AnimationTimeline(timeSource, invalidate)` 和 `advanceTo(time)`。同一轮全部值采样后才调用完成处理器，处理器中新启动的动画从该帧时间开始、后续帧继续。Tween 的 easing 包含线性和 cubic Bézier；spring 以质量 1 的解析解求值，不累加固定 dt。Size 和 Color 的结果限制在各自合法范围。Repeat 的次数计完整段数，Reverse 每段交换端点；零时长可以直接完成，但不能重复。

`SurfaceWindow.createArea` 创建窗口持有的稳定 InputArea，`area.scope` 是它的逻辑输入节点。`createScope` 可建立没有几何的祖先，二者的 parent 缺省为 `SurfaceWindow.input`。在 painter 内调用 `frame.place(area, rect或path)`，沿 Canvas 当时的变换和 clip 记录本帧几何；只有渲染提交成功后才整体发布。parent 在挂载逻辑中设置，与绘制顺序独立，不自动裁剪子区域。区域不再放置、scope 禁用或关闭后退出交互；所属窗口关闭时关闭其全部输入节点。

事件处理器接收 `(RoutedEventContext, InputEvent)`，返回 Unit。InputScope 的 captureHandler、handler 和 defaultAction 分别接收 capture、target／bubble 和默认动作。`stopPropagation()` 只停止传播，`preventDefault()` 只取消默认动作，`consume()` 同时执行两者。默认阶段从目标向根执行；主按钮按下的聚焦也在这一阶段，可由父级取消。窗口有 OS 焦点而没有区域焦点时，键盘事件从根 scope 开始。

上下文的 target／currentTarget 是逻辑 scope，localPosition 只在带几何的区域指针事件中存在，窗口位置保留在 InputPointer.state；上下文随同步回调结束而失效。Enter／Leave、焦点、修饰键与取消是不可取消的通知，文本只给对应 IME 会话。首次按钮按下固定整次多按钮序列的路径，空白区域按下固定根路径；拖出窗口仍可继续，取消向旧路径完整交付，剩余按钮事件不会改投其他区域。滚轮区分行与逻辑像素。`onEventFinished(InputEvent)` 在每次可路由输入之后通知原路径内仍有效节点，即使传播或默认行为被取消；控件用它结束释放序列，不执行被取消的激活。

`InputBoundary(scope)` 将输入限制到该 scope 的逻辑后代，命中、焦点、capture／bubble／default 和空白处的缺省目标都停在当前域。边界按建立顺序叠放，`isCurrent` 与 `SurfaceWindow.inputBoundary` 查询当前屏障；关闭恢复前一域。域改变会完整取消原来的按下路径，剩余按钮事件保持空路径。域外的区域不必有场景几何，但逻辑祖先的禁用／关闭仍然生效；`scope.isInputAllowed` 可在首次放置前判断准入。上层通过 `SurfaceWindow.reserveFocusRequest()` 为待处理或被拒绝的显式焦点意图推进同一个 focusRevision，不改变当前焦点，也不改写 InputArea 的成功历史。

边界所属 scope 已退休时，屏障仍持续到 InputBoundary.close，便于 UI 等待弹层移除帧成功提交。当前域根的 `beforePointerPress` 在首次按钮按下的派发前读取 InputPress，返回 true 消费整段按下序列，返回 false 沿已固定的目标和坐标继续。外部点击策略可以据此关闭最上层弹层；追加按钮不再次执行预处理，回调中改变边界也不能把旧事件投向背景。

macOS 可用 `ShortcutScope(input, bindings)` 在作用域内登记快捷键，UI 可通过 `scope.resource { ShortcutScope(scope.input, bindings) }` 管理它。ShortcutKey 显式选择 logical 或 physical；ShortcutModifiers.primary 是 Command，完整修饰组合匹配，文字大小写比较复用 Window 的 Unicode 处理。默认忽略重复和合成输入，canExecute=false 向外层继续；无焦点时可使用窗口根。每个输入节点的普通 defaultAction 和 addDefaultAction 登记先执行，ShortcutScope 最后匹配，文本编辑命令可以先消费事件。

`HostShortcutScope(host, bindings)` 为宿主当前及后续附着的窗口共用一份应用快捷键，窗口根的普通行为与本地 ShortcutScope 先处理，宿主最后回退。登记可早于 Application.run；每个宿主只允许一个活动登记，支持 enabled 和 bindings 整体更新，关闭后可重新创建。关闭单个窗口不影响其他窗口，关闭宿主立即停用。任何 InputBoundary 都阻止宿主回退，包括窗口根本身作为边界；需要在模态内执行的动作应登记到该域。

`createArea(focusable = true)` 允许逻辑焦点，`textInput = true` 同时允许焦点和 IME。区域已成功放置且有非空可见几何时，canFocus 为 true，requestFocus() 可以聚焦；它不抢占系统窗口焦点。windowBounds／visibleBounds 查询最近成功帧的窗口逻辑包围盒，后者计入路径裁剪及视口。已聚焦区域仅被 clip 裁掉时保持会话，移除、禁用或关闭仍清除焦点。`hasFocus` 表示保留的逻辑焦点，`isFocusActive` 还要求 OS 窗口当前获得焦点。文字只从 TextInputBatch 应用到编辑器自己的状态，不能再由 LogicalKey.Character 插入。用 `area.setCaret(rect)` 提交局部 caret，Surface 随已发布变换更新系统候选窗；同值设置不产生重绘。焦点切换等待旧 IME 的 Disabled 边界后再开启新接收者，已排队编辑保留原归属。

[组合示例](examples/basics/README.md)演示旋转 Path、hover、拖动 spring、文本布局及 IME 接收。它是普通 executable Cone，编辑状态由示例管理，未依赖 component。

`InputArea.focusRevision` 返回该区域最近一次成功聚焦的窗口序号；未曾聚焦或关闭后为 None。它与 SurfaceWindow.focusRevision 共用顺序，鼠标、重复成功请求及直接程序请求一致，失败和 OS 激活变化不重写成功历史，供上层导航恢复最近使用的成员。

滚轮默认行为可读取 `context.scrollRemaining`，并调用 `context.consumeScroll(lineStep) { localDelta -> ... }`，返回该容器实际消费的局部位移。lineStep 由容器提供；Surface 按本次事件固定的区域几何换算，消费后将残量还原到 Lines／Pixels，父容器再使用自己的步长和变换。Pixels 已完成一次 DPI 归一。原事件始终保留完整 delta，stopPropagation 不取消默认滚动；Contain 可在处理本层后调用 preventDefault。

连续 phase 和其后的 momentum 固定起始路由，接触结束后仍可沿原路径进入动量；普通离散 Wheel 按当次命中。源区域退休或模态切换用 ScrollCancelled 完整通知旧路径，后续动量保持空路径直到结束；它与鼠标按钮 Cancelled 分开，不会误结束另一段拖动。等待可能的动量不维持帧需求。

验证使用以下普通测试 Cone：

- 本 Cone 的 tests：动画的确定性采样；实际 Skia 离屏几何上的区域命中、传播、拖动、焦点、IME 会话和关闭；两个真实窗口的按需重绘、空闲、隐藏／最小化恢复、resize、输入宿主保留及重新 attach。`--filter Animation` 和 `--filter Input` 可分别运行不需要桌面的测试；`--filter SurfaceHostTest` 运行真实窗口测试。
- `tests-cones/asyncio`：原生 modal／menu 循环下的帧与 I/O，Windows 还覆盖系统移动／缩放循环，以及根取消后的 Surface 收尾。
- `tests-cones/content`：附着失败及错误线程的所有权边界、回调内关闭、原生关闭、重新附着、从未绘制的隐藏内容和退出收尾。
- `tests-cones/errors`：paint 与多个 content.close 同时抛错、失败附着的回滚及全部 surface／lease 释放。
- `tests-cones/input`：在真实输入宿主上组合动画帧、区域鼠标事件、平台 preedit／commit、候选窗坐标、IME 切换和回调内关闭。Windows 的协议注入复用 window 测试 Cone 的 UTF-16 边界；该测试与安装的真实 IME 桌面操作分别记录。
- `tests-cones/shortcuts`：macOS 原生 Command、修饰键精确匹配、重复抑制、禁用与默认动作取消；窗口没有区域焦点时仍可执行绑定。独立进程遵守每进程一个 Application 的合同。
- `tests-cones/host-shortcuts`：macOS 的共享宿主登记、窗口本地优先、根模态边界、关闭首窗后附着第二窗口，以及动作内关闭宿主。
- `tests-cones/scroll`：macOS 原生行／像素滚动、嵌套残量、区域移动后的连续 phase／momentum、源失效后抑制背景滚动与正常收尾。
- `tests-cones/presentation`：Windows 桌面上用两个重叠的真实窗口，采样透明区域和半透明绘图的最终合成像素。

可通过 `scoop test <Cone路径> --release --target <目标> --sysroot <sysroot>` 分别运行；Windows 安装后的命令名为 `sco`，仓库开发构建为 `scoop.exe`。真实显示测试需要已解锁、显示开启的桌面；显示休眠或锁屏时 AppKit 将窗口报告为遮挡，生产代码按合同停帧。当前交付状态与验证限制见 [实施记录](../../../docs/surface/PROGRESS.md)。

桌面测试须串行执行。`tests-cones/input` 还需要真实窗口焦点；macOS 可以拒绝后台进程的 activate 请求，此时在准备期将“Scoop region input”置于前台。测试给帧／焦点准备 30 秒，确认真实焦点后用独立的 5 秒期限验证输入会话，不用强制激活或伪造焦点绕过系统行为。超时会报告阶段、帧数、焦点和输入计数。需要桌面应用入口时，可将 `scoop test --no-run` 的测试可执行文件放入普通 `.app` 包装后从访达启动；无参数直接运行仍执行 scoop.test 的非 ignored 用例，设置 `SCOOP_TEST_RUN_DIR` 可记录原有结构化结果。

复用临时测试应用时，用新文件替换其中的可执行文件，并执行 `codesign --force --sign - /path/Test.app` 更新整个测试应用的本地签名。命令行前台启动可用 `open -n -W ... --args --scoop-test-filter TestClassName` 选择相关用例；以 scoop.test 报告中的实际 case 结束和 complete 为结果，单独的 open 返回成功不表示测试通过。
