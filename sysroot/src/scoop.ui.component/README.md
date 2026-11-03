# 自绘基础控件

`scoop.ui.component` 是普通 Scoop Cone。当前提供 Label、Button、Checkbox、Input／Textarea、Row／Column／Stack、Frame、Absolute、ScrollArea 和 VirtualList，使用公开的 Component／Element／LazyElement、ParagraphHost／TextEditHost、Theme／ThemePainter、InputArea 和 Surface 动画；UI 框架没有这些控件的特殊节点。

应用选用一个主题实现，并在实际运行入口绑定稳定 ThemeSource：

```kotlin
import scoop.ui.*
import scoop.ui.component.button
import scoop.ui.style.ThemeMode
import scoop.ui.style.ThemeSource
import scoop.ui.style.macos.macosTheme

class Counter: Component {
    private var count: Int by state(0)
    public override fun render(): View =
        button(f"Count: ${count}", { count += 1 })
}

fun main() {
    val source = ThemeSource(macosTheme(ThemeMode.Light))
    context(source) {
        runUiApplication("Counter", { Counter() })
    }
}
```

控件挂载时保存来源，在 render 中订阅 `source.current.value`。应用在 UI owner 线程更新这一个 State 即可切换风格或明暗；工厂、内部 State 和焦点保留，尺寸及文字按新主题重新布局。可通过控件的 `theme` 参数明确使用另一来源。临时 Context 不会自动成为 View 子树的主题 Provider。

Checkbox 可接收 `State<Boolean>.binding()`，也可用 `checkbox(initialChecked = false)` 保留自己的状态。Button.enabled 是普通 Props，可在组合组件的 render 中从同一 checked State 计算。更换外部 Binding 后交互使用新绑定；initialChecked 只决定自有状态组件首次挂载的值。

`input(editor, placeholder = "名称")` 和 `textarea(editor, rows = 4L)` 接收调用方持有的 `scoop.ui.text.TextEditorState`。通过 editor.text／selection 读取同一模型，外部整体替换使用 replaceAll；不需要在 onEdited 中把字符串写回控件。Input 的 Enter 提交，Textarea 的普通 Enter 换行、⌘Enter 提交，Tab 默认移动焦点。只读仍可聚焦和复制，enabled=false 停用输入。

两个输入控件在有界父容器中占满宽度，可用 Frame 指定尺寸；Textarea 的 rows 决定建议高度。description 放在边框外，invalid 使用当前主题的错误色，业务自行决定验证结果。TextFieldStyle 可覆盖表面、字体、选择及光标颜色；caretBlink=Duration.milliseconds(0L) 保持光标可见。Textarea 的 Auto／Always 滚动条预留稳定空间，Hidden 不预留；paintCache 独立控制正文像素缓存。`tests-cones/editing` 覆盖实际控件、三套主题、模态 IME、滚动条和虚拟编辑项的保留／卸载。

Button／Checkbox 默认参与 Tab／Shift+Tab 遍历。`focus = FocusOptions(Some(handle), tabStop = true, order = 0)` 可提供外部 FocusHandle、Tab 资格及作用域内顺序；handle.requestFocus() 也可聚焦退出 Tab 的控件。禁用仍由原有 enabled 控制，布局和绘制顺序不决定 Tab 顺序；更换 handle 不重建区域或重置已有焦点。

Label 的 String 入口接受可选 SpanStyle，只覆盖其中明确的字体、字号、颜色等属性；完整 StyledText 使用公共富文本模型。Button 的标签复用 Label 与主题 action 字体，其基线来自实际文字布局。外部 FontCollection 只借用，默认字体资源由挂载作用域管理。

ButtonStyle／CheckboxStyle 使用公共的尺寸 Options 和 StyleOverride。Inherit 保留本轮主题值，Set(None) 明确清除可空属性。InteractionStyles 按 hovered、pressed、focused 顺序组合，禁用时只应用 disabled。`styled(content, style)` 可以给完整控件添加有序 glow、drop shadow 或 blur；空效果与有效果使用同一个持久装饰节点。

主题 painter、字体、段落和滤镜归 Element 的 ComponentScope 所有，应用无需为每个控件添加 use。主题和效果没有自己的时钟，控件动画只使用 Surface Animatable；静止时不安排更新或绘制。

Row／Column 按主轴排列，spacing 与交叉轴 AxisAlignment 通过 Props 更新；Stack 按描述顺序叠放。Frame 提供明确的尺寸、padding、两轴对齐和公共装饰，也可以选择 SurfaceKind 使用主题表面。未选表面时无需主题来源；从普通 Frame 切换为主题表面或反向切换都保留内容身份。Frame 透传经实际偏移后的基线，其他容器首期没有基线对齐策略。

`absolute([AbsoluteChild(view, horizontal, vertical, zIndex), ...], width, height, padding)` 在有限容器内定位。AbsoluteAxis 提供 Start／End／Center 的偏移与可选 tight 尺寸，以及 Stretch 两端定位；允许负偏移，无界父轴需要显式容器尺寸。容器不由子项撑开，每项只测量一次，不隐式裁剪。zIndex 控制同层呈现和命中，同值按描述顺序；子项 key、逻辑挂载和 Tab 顺序保留。布局错误在参数／测量边界报告，外观可组合 Frame。

有限宽度的 Label 使用定宽段落排版，无界宽度使用 layoutUnwrapped。行内按钮需要限定宽度时，可用 Frame.width 明确指定；文本行内子项仍只测量一次，不能要求先得到所在行的剩余宽度。

`scrollArea(content, state = Some(scroll), height = Some(240.0))` 给普通内容提供垂直滚动；ScrollState 可由组件或业务 Store 持有。axes 可选 Horizontal／Vertical／Both，滚动轴没有有限父约束时须提供明确尺寸。lineStep 默认 40，链式滚动在各轴边界把未用的原始残量交给父容器，`chain = ScrollChain.Contain` 可停止交接。整个内容树保持挂载，offset 变化不重建其描述。

滚动条使用当前主题，默认覆盖在视口边缘，支持拖 thumb、轨道翻页和主题可选箭头。ScrollbarVisibility.Auto／Always／Hidden 控制显示；点击条保留内容焦点，视口自身聚焦时可以用 PageUp／PageDown／Home／End。程序可用 scrollTo／scrollBy、内容局部 Rect 的 reveal，或显式 animateTo；用户滚动会取消旧平滑动画。`tests-cones/scroll` 通过真实窗口验证嵌套残量、双轴、取消、滚动条、主题切换、焦点和裁剪。

ScrollArea 和 VirtualList 均可设置 `paintCache = PaintCachePolicy.Tiles(tileSize = 256, overscan = 1)`，默认 None。策略只缓存 UI drawLayer 声明的有限内容，公共 ParagraphHost 正文自动使用该入口；普通 draw 保持直接执行。绘制缓存与 retention 独立，切换不会替换 ScrollState 或子项实例。含滤镜的记录按 Graphics 的独立裁剪能力正常重放，滤镜也可放在完整内容合成阶段。`tests-cones/paint-cache` 验证四种缓存组合、真实输入、文字更新和停帧。

已放置的内容目标通过 FocusHandle、Tab 或 KeyNavigation 聚焦时会自动显露；嵌套 ScrollArea 每次调整最近一层，等成功帧后继续外层。滚动条与 Portal 按自己的实际场景路径处理。`tests-cones/reveal` 用真实键盘和窗口覆盖这条路径、失败帧、不可显露目标、新位置请求与隐藏时取消。

`virtualList(source, itemExtent = ItemExtent.Fixed(24.0), state = Some(scroll), height = Some(240.0))` 使用 ItemSource 按需创建固定高度项，orientation 可选 Vertical／Horizontal；两轴须有界或提供明确尺寸。overscan 是视口前后额外实现的逻辑长度，默认 0。列表复用同一套滚轮、键盘、主题滚动条与视口 clip，只为可见范围、overscan 和活动交互实现条目。离开范围且无活动交互的项正常卸载，再进入时创建新实例；数据本身可以保存在外部模型。

条目的焦点、按下序列和等待惯性的路由在滚出屏幕后继续有效，交互结束后恢复淘汰；真实删除不受这种保留阻止。固定范围算法、零视口和参数检查在本 Cone 测试；`tests-cones/virtual` 用一亿条的数据源验证实际构建数、Dispose、横纵轴、缩小范围以及跨帧的指针／焦点／惯性行为。`tests-cones/variable` 覆盖可变 extent、按 key 对齐、阅读锚点、FollowEnd 与失败帧。

`retention = ItemRetention.KeepAlive(32L)` 额外保留最近使用的 32 个离屏实例，复用其局部 State 与资源。它不改变 viewport／overscan 和交互固定范围，也不创建绘制缓存；默认 Dispose。容量缩小和真实删除在隐藏窗口中也会处理。`tests-cones/retention` 覆盖缓存淘汰、状态及 shape 更新、嵌套缓存、动画停帧和模态输入边界的保留。

[主题展示示例](examples/themes/README.md)并列展示公共绘图部件，提供三风格、明暗、系统跟随和真实控件／效果切换。

[下载与后台校验示例](examples/download/README.md)展示 UI owner 的异步 HTTP 与真实后台 Thread 如何协作，通过 ApplicationProxy 合并发布进度；覆盖共享业务 State、组件自有状态、取消／替换以及应用退出后的线程回收。

规范见 [基础控件规范](../../../docs/specs/SCOOP-UI-COMPONENT-SPEC.md)和[主题绘图规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md)。普通覆盖／来源／布局测试在本 Cone，真实 Metal／输入／主题切换组合位于 `tests-cones/interaction`，容器与富文本组合位于 `tests-cones/layout`；实施范围与运行结果记录在 [UI 实施记录](../../../docs/ui/PROGRESS.md)。

`tests-cones/absolute` 验证显式子项呈现顺序、真实像素与命中、keyed 更新和 Tab。焦点、组内导航与新增基础容器的本期结果见 [基础交互实施记录](../../../docs/ui/foundations/PROGRESS.md)。

`ItemExtent.Variable(estimate = 24.0)` 按视口逐项测量可变高度内容，允许实际高度为零。尺寸索引只保存访问过的 key，未知高度仍为估计；前方插入或测量变化时保留首个可见 key 和项内位置。`anchor = VirtualListAnchor.FollowEnd` 只在原先已到末端时跟随新增内容，向上阅读时保持阅读位置。

把稳定的 `VirtualListHandle` 传给 `handle`，挂载后可调用 `handle.scrollToItem(key, ScrollAlignment.Center)`。列表用实际测量修正位置，`isPending` 在成功帧后清除；更新的普通 ScrollState 定位优先。handle 不另存 offset，ScrollState 继续用于观察位置、滚轮和滚动条。

`handle.focusItem(key, ItemFocusEdge.First)` 按需实现并定位条目，再聚焦行内首个可用 Tab 目标；Last 选择末个目标。isPending 也包含这个聚焦过程。未知 key、没有可用目标、真实删除、禁用或更新的焦点／滚动请求会结束旧等待；外层缓存的列表可以因待聚焦归属恢复布局。`tests-cones/focus-items` 检查真实按 key 聚焦与实例数量。

`navigation = ItemNavigation.Sequential` 启用逐行 Tab 访问；`ItemNavigation.Roving(direction = TextDirection.Ltr, loop = false)` 让列表占一个 Tab 停靠点，并用方向键和 Home／End 导航。轴沿 orientation，入口顺序沿 focus.order；启用时视口自身不再额外占一个 Tab 停靠点，None 默认模式保持普通滚动视口行为。选择／激活由上层组件决定。`tests-cones/navigation-items` 覆盖真实键盘、嵌套列表与焦点作用域、禁用／空项、实例卸载后的返回、RTL、循环和默认动作优先级。
