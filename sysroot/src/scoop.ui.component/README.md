# 自绘基础控件

基础输入、选择、数值、菜单、Tab、模态、日期、表格、虚拟集合及会话控件使用同一默认语义入口。通过 Props 或 builder 的 `semantics = ComponentSemantics(...)` 覆盖 name、description、error、automationId、relations、hidden／ignored、busy／live；角色、checked／selected／expanded、禁用、只读、正文与动作取实际控件。三个桌面平台共用组件声明，AppKit／UIA／AT-SPI 的实际验收范围见[实施记录](../../../docs/ui/interaction-polish/PROGRESS.md)。完整消费者见[无障碍设置示例](examples/accessibility/README.md)。

已有覆盖值可用 `semantics.copy { busy = true }` 派生，receiver 为 ComponentSemanticsBuilder；未指定字段与原 reference 完整保留。复制不会创建节点或替控件执行业务动作。

WindowControls 的 `semantics(button, maximized)` 回调可为系统窗口按钮命名；只覆盖部分按钮时，其余按钮可返回 `windowControlSemantics(button, maximized)` 保留默认名称。

```kotlin
import scoop.ui.View
import scoop.ui.component.*

fun saveButton(onSave: () -> Unit): View =
    button("保存", onSave) {
        semantics = ComponentSemantics(
            description = Some("保存当前文档"),
            automationId = Some("save")
        )
    }
```

Button 默认从自己可合并的静态 Label 取得名称，独立交互后代继续可访问；显式 name（包括空串）优先于 labelledBy 和内容标签。Label 正文保留 StyledText 的行内 alternateText，覆盖名称不替换正文。Checkbox 的原生切换使用真实 Binding，拒绝写入不会先翻转语义状态。ButtonGroup、InputGroup、Command、Combobox、Sidebar 和原生文件按钮的相关包装继续传递覆盖值。

`ComponentSemantics.reference = Some(SemanticsReference())` 可给关系的目标提供稳定引用；其 id 在真实挂载后可用，卸载时清空。Field、弹层触发器、Tab、候选项、表头等内置连接自行维护引用，通常无需手动管理。Dialog／Sheet／Drawer 的语义遵守同一模态输入边界，背景旧引用不能执行动作，dismiss 仍经过关闭 veto。

Slider 发布真实数值范围、步长与调整动作；Progress 的未知值不伪装成零。Select／Combobox／NavigationMenu 和 DatePicker 发布实际 expanded 与 controls；Tabs、Calendar、Pagination 发布真实选择状态。OTP 只有一个 TextField，视觉槽位不拆成多个输入。Calendar 的 daySemantics 可本地化完整日期名称；静态 Table 发布行、列、跨度与表头关系。DataTable 的 semanticText／条目描述读取数据，不调用 cell View 构造函数；Command、Combobox、VirtualList 和 MessageScroller 使用同源集合快照。

Toast 默认 live=Polite，首次实际显示后公告一次；Loading 暂缓，关闭、遮挡于模态背景或退休后不发迟到公告。普通 Label 等控件的 live 默认为 Off，应用可为实际状态消息明确启用。ImageView／PathView／Spinner／Separator 默认装饰，只有显式提供 semantics 才导出；Avatar 默认保留图像替代名称。可点击 Badge、Item、Bubble、Attachment、Trace 和问卷选项复用原操作与受控绑定，不通过模拟鼠标触发。

Switch／Toggle 读取实际布尔绑定；RadioGroup 通过原选择入口选择，重复激活已选项不会取消或重复调用 setter。RadioGroup／ToggleGroup 的组和条目均可设置 semantics；重排沿原 key 保留节点，删除后旧引用失效，单选／多选规则不变。Switch 和 Radio 条目沿 FieldTarget 接入标签、说明、错误与 required。用法及运行范围见[选择控件语义测试](tests-cones/choice-accessibility/README.md)。

输入控件直接复用 TextEditHost 的正文、选区、策略、撤销、焦点与菜单，placeholder 不作为 label；既有 description 提供默认说明，invalid 时也提供默认错误文字，显式覆盖可替换或清空它们。受保护输入不公布正文／长度／选区。用例和当前验证边界见[内置控件语义测试](tests-cones/accessibility/README.md)；字段状态和其他控件继续按计划推进。

原生桌面包装提供 `nativeApplicationMenuBar`、`nativeWindowMenuBar`、`nativeMenuButton` 和 `nativeContextMenu`，使用独立的 `NativeMenuContent`、稳定 key 与 Binding。应用菜单在 `ApplicationComponent.render` 中声明；窗口菜单包装正文。菜单值和回调更新不依赖绘制，popup 使用一次性 session；`NativePopupHandle.requestOpen()` 发起新请求，取消不会自动重开。标准编辑角色沿当前输入范围进入现有文本编辑器。详见[原生菜单规范](../../../docs/specs/SCOOP-UI-NATIVE-MENUS-SPEC.md)。

`windowChrome` 区分视觉 clip、原生 windowShape 与 inputShape，按实际窗框和系统按钮几何布局。`windowDragRegion`、`windowResizeRegion`、`windowControls` 复用原生移动／缩放和系统按钮能力；关闭保留 `CloseRequested` 退出确认。`scope.nativeToolWindowController()`、`nativeDialogController<T>(target)`、`fileDialogController()` 管理明确拥有的窗口／session，普通 `UiHost.attach` 继续借用 Window；`nativeOpenFileButton`／`nativeSaveFileButton` 复用 Button 并返回类型化 Path。模态窗口与文件面板的原生创建失败通过现有结果回调返回 Failed，清除 pending 后仍可继续操作；open 返回 true 只表示请求已处理，应检查实际结果。参数、UI 工厂及业务回调异常沿所属 UI 错误边界报告。能力差异和资源顺序见[窗框规范](../../../docs/specs/SCOOP-UI-WINDOW-CHROME-SPEC.md)与[窗口控制器规范](../../../docs/specs/SCOOP-UI-NATIVE-WINDOWS-SPEC.md)。

托盘包装在独立 [scoop.ui.component.tray](../scoop.ui.component.tray/README.md) Cone，普通 component 不依赖 scoop.tray／DBusMenu。原生组件的组合测试和 Windows 桌面执行入口见 [tests-cones/native-desktop](tests-cones/native-desktop/README.md)，实际平台验收记录见[桌面能力进度](../../../docs/desktop-ui/PROGRESS.md)。

[原生桌面综合 demo](examples/native-desktop/README.md) 将菜单、编辑、窗框、异形窗口、工具／模态窗口、系统文件面板及独立托盘组件组合在一个普通 Cone 中；包含实际能力说明、显式定位降级及无窗口运行流程。

`bubble`、`message`、`marker`、`attachment` 和 `trace` 提供会话展示组合。气泡支持七种变体、逻辑对齐及外伸反应区；Message 以真实正文／页脚高度定位头像。Attachment 借用应用图片，状态、整卡动作及独立动作由应用提供，AttachmentGroup 复用 Carousel 的横向吸附。Trace 组合 Collapsible，保留用户展开选择与嵌套编辑状态；纯状态行没有额外 Tab 目标。可选动作变化保留正文挂载，运行指示在不可见／减少动态／停止后停帧，布局纳入实际焦点环范围。组合实窗测试位于 `tests-cones/conversation`，完整合同见组件规范第 48～50 节。

`messageScroller(source, state)` 以 `TranscriptSource` 的稳定 key 和显式回合锚点按需构造消息。`MessageScrollState` 组合既有滚动／虚拟列表能力，提供初始 End／Start／LastAnchor、新回合顶部定位、可收缩尾部留白、可选自动跟随、阅读保持及按 key 跳转／恢复。指针选择、键盘和滚动立即停止旧跟随；公开阅读快照仅来自 Accepted。`transcript { item(key, scrollAnchor = true) { ... } }` 创建来源，`messageScrollButton` 可放入自定义 controls；真实输入和停帧测试位于 `tests-cones/message-scroller`。

`table { header { row { head(...) } }; body { row(key) { cell(...) } } }` 提供有限表格、跨列、caption 和 footer；`dataTable(model)` 消费应用持有的 DataTableState、TableSource 与类型化 TableColumn。Local 缓存筛选、稳定排序和分页结果，Provided 直接呈现当前页。选择只针对本页可选行，列宽拖动共用一份轨道，固定表头位于 VirtualList 正文之外；行内编辑与菜单保留普通生命周期。完整合同见组件规范第 45 节，原生组合测试位于 `tests-cones/table`。

`calendar(selected.binding(), initialPage)`、`calendarMultiple` 和 `calendarRange` 提供日期单选、多选和完整范围。CalendarOptions 配置多月、完整周、outside days、ISO／locale 周号、月／年下拉、日期规则及自定义 CalendarPageSource；日期运算采用 LocalDate，范围长度按夜数计算。方向键、周边界和翻页复用日期焦点，布局为实际主题焦点环留白。CalendarHandle 可定位／聚焦日期或取消暂定范围。

`datePicker(value.binding(), initialPage) { draft = Some(dateDraft) }` 与 `dateRangePicker` 组合日历弹层和日期输入。DatePickerDraft 保存唯一 TextEditorState；Enter 提交严格 ISO 日期，范围格式为 `start .. end`，解析失败保留原文。commitOnBlur 可选，外部更新保留正在编辑／组合输入的草稿；DatePickerHandle.reset 显式接受当前选中值。presets、显式 parser／referenceDate、生日年份选择和独立 LocalTime 输入是普通组合；`tests-cones/calendar` 覆盖实际键盘、中文文本宿主、范围取消与弹层焦点。

`tooltip(button("Save", save), "Save the current document")` 保留原按钮动作和焦点，提示正文不参与输入；`hoverCard(trigger, content)` 提供可选择文字和可操作正文的预览。两者支持延迟、可选受控 open、Escape 后保持关闭、定位与 Reduced Motion，复用既有 Hover／Portal。Tooltip 默认显示箭头，HoverCard／Popover 可通过 `arrow = Some(PopupArrow())` 开启；箭头跟随实际翻转后的边。定向窗口回归位于 `tests-cones/hover-popup`。

`scoop.ui.component` 是普通 Scoop Cone。当前提供 Label／FieldLabel／SelectableStyledText、Typography／Kbd、Button／ButtonGroup／Badge／Spinner、Checkbox／Switch／Toggle／RadioGroup／ToggleGroup／Slider、Input／Textarea／InputGroup／InputOtp、Card／Alert／EmptyState、Item／ItemGroup／ItemSeparator、Collapsible／Accordion／Tabs／Breadcrumb／Pagination、Avatar／AvatarGroup、Skeleton／Progress、Separator／AspectRatio、ImageView／PathView、Row／Column／Flex／Wrap／Grid／Stack、Frame、Absolute、ScrollArea 和 VirtualList，使用公开的 Component／Element／LazyElement、ParagraphHost／TextEditHost、Theme／ThemePainter、InputArea 和 Surface 动画；UI 框架没有这些控件的特殊节点。

`collapsible(open, content) { trigger("Details"); retention = ContentRetention.KeepMounted }` 以受控 Boolean 展开正文；默认 Dispose 在退出完成后卸载，KeepMounted 保留实例和状态，但关闭时不布局、接收输入或请求动画帧。`accordion(selected) { item(Choice(key, value, label), content) }` 支持可空单值／数组绑定；单选的 allowCollapse 控制是否允许全部关闭，variant 可选 Default／Card。header 普通 Tab、方向键焦点导航与 Space／Enter 激活分开处理；稳定 key 重排保留正文和编辑状态。两者复用 Presence，支持退出中重开、MotionPreference.Reduced 和每项自定义非交互 trigger 内容。`tests-cones/disclosure` 使用真实窗口覆盖绑定、生命周期、IME、键盘导航及关闭停帧。

应用选用一个主题实现，并在实际运行入口绑定稳定 ThemeSource：

```kotlin
import scoop.ui.*
import scoop.ui.component.button
import scoop.ui.style.ThemeMode
import scoop.ui.style.ThemeSource
import scoop.ui.style.macos.macosTheme

class Counter(scope: ComponentScope): Component(scope) {
    private var count: Int by state(0)
    public override fun render(): View =
        button(f"Count: ${count}", { count += 1 })
}

fun main() {
    val source = ThemeSource(macosTheme(ThemeMode.Light))
    context(source) {
        runUiApplication("Counter", { scope -> Counter(scope) })
    }
}
```

控件挂载时保存来源，在 render 中订阅 `source.current.value`。应用在 UI owner 线程更新这一个 State 即可切换风格或明暗；工厂、内部 State 和焦点保留，尺寸及文字按新主题重新布局。可通过控件的 `theme` 参数明确使用另一来源。临时 Context 不会自动成为 View 子树的主题 Provider。

`buttonGroup { button("保存", onSave); input(editor); text(".com") }` 收集带明确类型的条目，支持横向／纵向、嵌套组及分隔线。相邻表面共享接缝，Input 默认占用剩余空间；焦点项最后绘制，Tab 顺序保持声明顺序。自定义条目用 `item({ position -> ... })` 把 `GroupPosition` 明确传给控件的 `group`，`add(View)` 保留已有外观；组不增加动作或选择语义。根 fonts／theme 为内置条目提供默认值，嵌套组独立配置。

`FieldTarget()` 把 `fieldLabel("名称", target)` 与 Input／Textarea／InputOtp／Checkbox／Switch 的 `labelTarget = Some(target)` 关联。标签不占 Tab 停靠点；文本标签请求聚焦，Checkbox／Switch 标签聚焦并执行一次当前选择动作。禁用、隐藏、卸载、跨窗口或模态域外目标不能由标签激活；换绑更新当前关联，按下期间换绑会取消该次点击。普通 `label` 仍是展示文字。

在 macOS 上，已接入默认语义的 Button、Checkbox、Input／Textarea 同时从 FieldTarget 取得标签、说明和错误关系。Field 自动使用 label 的目标；使用普通 title 时通过 `target = Some(accountTarget)` 指定同一目标。独立的 fieldTitle／fieldDescription／fieldError 也提供 target，FieldLabel／FieldText／FieldError 的 Props 和 builder 可用 ComponentSemantics 覆盖名称、说明与标识。

```kotlin
import scoop.ui.View
import scoop.ui.component.*
import scoop.ui.text.TextEditorState

fun accountField(editor: TextEditorState, accountTarget: FieldTarget): View =
    field(input(editor) { labelTarget = Some(accountTarget) }) {
        label("账户", accountTarget)
        description("显示给团队成员的名称")
        required = true
        errors = ["请填写账户名称"]
    }
```

目标由拥有表单的组件保持稳定；Field 不查找或修改不透明 control。多个独立标签按关联登记顺序组成名称，自定义标签内的按钮继续独立可访问；标签因目标未绑定或禁用而停止输入时，子按钮也报告不可用。显式非空关系覆盖对应自动关系，显式 name／description／error（包括空串）优先。正文更新、注释关闭或目标换绑会刷新现有语义节点，空错误不会留下旧消息；关联变化不需要绘制或移动焦点。Field.required 也通过该目标发布实际必填属性，更新与换绑不改正文或焦点；已由控件报告必填时，requiredText 保留显示并退出重复朗读。invalid 等其余状态与其他控件按统一计划继续接入。

`typography("标题", TypographyRole.H1)` 提供 H1–H4、P、Blockquote、InlineCode、Lead／Large／Small／Muted 角色，默认可选择和复制。`typography { text("运行 "); code("scoop build") }` 使用真实行内代码 View，复制结果保留代码文字；span、inline、actions 和 decorations 继续使用公共富文本 API。`typographyList { item("第一项") }` 和 `typographyTable { header { cell("名称") }; row { cell("Scoop") } }` 提供项目符号与共享列宽的静态文字排版，段落间距由父布局控制。

`kbd(chord)` 显示窗口提供的原生快捷键标签，`kbdGroup(chord)` 将各部分分为键帽；两者直接消费与 ShortcutScope 绑定相同的 ShortcutChord。`kbd("Esc")`、自定义 View 和 `kbdGroup { kbd("↑"); kbd("↓") }` 可作为按钮或文字行内内容。Kbd 自身不注册动作或焦点；示例中的实际绑定由应用作用域持有。

Checkbox 可接收 `Binding<CheckState>`（Unchecked／Checked／Indeterminate）或 `State<Boolean>.binding()`，也可用 `checkbox(initialChecked = false)` 保留自己的状态。Button.enabled 是普通 Props，可在组合组件的 render 中从同一 checked State 计算。更换外部 Binding 后交互使用新绑定；initialChecked 只决定自有状态组件首次挂载的值。

`input(editor) { placeholder = "名称" }` 和 `textarea(editor) { rows = 4L }` 接收调用方持有的 `scoop.ui.text.TextEditorState`。通过 editor.text／selection 读取同一模型，外部整体替换使用 replaceAll；不需要在 onEdited 中把字符串写回控件。Input 的 Enter 提交，Textarea 的普通 Enter 换行、⌘Enter 提交，Tab 默认移动焦点。只读仍可聚焦和复制，enabled=false 停用输入。

Input／Textarea／InputOtp 默认提供原生文本菜单。使用 `contextMenu { mode = TextContextMenuMode.StandardNative; selection = TextContextMenuSelection.Preserve }` 可选择标准菜单并保留选区外点击的选择；Disabled 关闭内建入口，PreferSystem 尽量复用可执行的系统模板。相关类型位于 `scoop.ui.text`，可选标签覆盖使用 `scoop.window.TextContextMenuLabels`。InputGroup、ButtonGroup、Command、Combobox 和 SidebarInput 透传当前配置；需要应用自定义菜单时先关闭内建入口，再使用现有菜单包装。编辑动作始终进入同一个 TextEditorState，OTP 的长度与模式限制仍然有效。

macOS 输入控件默认支持文本选区拖出及文本／URL 拖入。使用 `input(editor) { acceptDrops = false }` 关闭接收，使用 `drag = TextDragOptions(enabled = false)` 关闭导出；Textarea、InputOtp 与 Props 同名属性采用同样默认值，InputGroup、Command、Combobox、SidebarInput 等包装透传配置。drag 配置阈值、导出策略、Copy／Move 和完成通知，只读只能 Copy；SelectableStyledText.options.drag 与 Typography 的 `selection = Some(SelectableTextOptions(drag = ...))` 使用同一只读来源。落点、临时 caret、边缘滚动、composition 与 undo 由原 TextEditHost 处理。拖入复用 paste 和通用 policy，OTP 的清洗／裁剪不能伪装成完整 Move；同文档移动一次撤销，SourceRetained 表示目标已接收但源未删除。受保护输入可以导入，默认不拖出原文。具体合同及平台范围见[统一计划](../../../docs/ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)。

SelectableStyledText 的 `options = SelectableTextOptions(contextMenu = textContextMenu { selection = TextContextMenuSelection.Preserve })` 配置只读 Copy／SelectAll 菜单，默认也使用 PreferSystem。Typography 的 `selection = Some(SelectableTextOptions(...))` 使用应用完整的选项；默认 `None` 从当前主题解析选区外观。正文与行内替代文字来自共享只读宿主，辅助点击不触发行内主按钮，键盘菜单沿现有焦点与 Accepted 选区定位。

Input／Textarea 的 TextFieldStyle、InputOtpStyle 及其 builder 提供 `selectionTextColor`／`inactiveSelectionTextColor`，默认与选区背景一起跟随主题。高对比度使用成对的不透明背景与文字；减少透明度把默认选区底色合成到主题背景，OTP 默认槽底色也保持不透明。普通偏好保留原文字颜色。显式选区背景不会自动推导新的前景；需要时同时指定文字覆盖。主题和文字缩放热切换保留模型、方向选区、撤销历史及逻辑焦点。

SelectableStyledText 默认发布共享宿主的正文、选区和只读菜单动作；`onSelectionChanged` 在语义刷新后仍调用一次最新业务回调。选择部分行内替代文字会覆盖整个对象，禁用选择立即撤下对应能力，不提供文本写入。Typography 的两种 selectable 分支都保留 `semantics` 配置，切换分支后旧原生对象退休。标题级别、范围几何及行内范围动作的专门语义继续按计划接入。

密码输入使用 `input(editor) { protectedText = true; projection = textMask() }`；显示密码时只把 projection 换成 textProjection()，继续保留 protectedText=true。Input／Textarea／InputOtp 及组合输入都提供这个独立标记，默认 false。受保护用途或内置遮罩默认禁止 Copy／Cut；Input／Textarea 可用 `copy = TextCopyPolicy.Allow` 明确允许复制原文，该选择不清除保护标记。显隐与用途更新保留模型、选区、撤销和 IME；粘贴及普通编辑继续使用原策略。

Input／Textarea／InputOtp 的 Props 和 builder 提供 `touch = TextTouchOptions()` 与 `touchScroll = TouchScrollOptions()`，分别从 `scoop.ui.text` 和 `scoop.ui` 导入。前者控制直接触摸／笔尖的短按定位、二／三击选择、窗口逻辑距离阈值和一次性长按，后者独立控制文本视口的拖动滚动。`multiTap` 默认 true，第二次短按选词、第三次及以上选逻辑行；`multiTapInterval` 默认 None，跟随系统双击间隔，未知时使用 500 ms，可设置显式非负值。`TextTouchOptions(longPressDelay = None)` 保留短按和多击，`multiTap = false` 只关闭连续计数，`enabled = false` 关闭文本触摸选择；菜单关闭时仍可长按选词，键盘与 accessibility 菜单保持可用。InputGroup、ButtonGroup、Command、Combobox 和 SidebarInput 透传相同配置；只读富文本使用 SelectableTextOptions.touch。可运行场景见[共享文本宿主测试](../scoop.ui/tests-cones/editing/README.md)和[文本控件测试](tests-cones/editing/README.md)。

上述 touch 配置还包含 `selectionHandles = TextSelectionHandleOptions()`，默认显示触摸选区的 caret／双端点把手，并通过已有视口做边缘滚动。可单独关闭 enabled，或设置 radius、hitRadius 与 color；关闭把手保留短按、多击和长按。把手不新增 Tab 焦点或语义节点，文本选择仍由控件的同一模型与 accessibility provider 发布。

两个输入控件在有界父容器中占满宽度，可用 Frame 指定尺寸；Textarea 的 rows 决定建议高度，默认内缩按 body 字号／14 缩放为横向 10、纵向 8。description 放在边框外，invalid 使用当前主题的错误色，业务自行决定验证结果。TextFieldStyle 可覆盖表面、字体、选择及光标颜色；caretBlink 缺省跟随系统，显式 `caretBlink = Some(Duration.ZERO)` 保持光标可见；光标宽度也默认跟随根系统偏好。Textarea 的 Auto／Always 滚动条预留稳定空间，Hidden 不预留；paintCache 独立控制正文像素缓存。`tests-cones/editing` 覆盖实际控件、三套主题、模态 IME、滚动条和虚拟编辑项的保留／卸载。

`inputGroup { input(editor); addon { text("https://") }; addon(InputGroupAlign.InlineEnd) { button("打开", onOpen) } }` 使用一个外框和一个实际编辑宿主。必须且只能提供一个 input／textarea／control；addon 支持 InlineStart／InlineEnd／BlockStart／BlockEnd 和自定义 View，button 默认 Ghost／Xs。点击 addon 空白或文字请求输入焦点，内部按钮独立激活和参与 Tab。相同 key 的 addon 换位置保留实例，根 fonts／theme 为未明确配置的内置内容提供默认值。

Input Group 内置输入使用 `chrome = TextFieldChrome.Bare`，移除内层外框、边框占位和禁用透明度，保留原文本宿主、padding、滚动条和编辑策略。外部组合也可显式使用 Bare；切换 chrome 保留选区、组合输入与撤销历史。自定义 `control(view, focus)` 必须让实际控件绑定该 FocusHandle，并自行选择 Bare；外层不遍历 View 或查找编辑器。`tests-cones/input-group` 覆盖真实窗口中的四向 addon、keyed 重排、IME、焦点换绑和资源收尾。

`field(control) { label("名称", target); description("帮助"); errors = messages }` 组合标签、控件、说明与业务错误。Vertical／Horizontal／Responsive 按实际内容宽度布局，默认响应阈值为 448（随字号缩放）；切换布局和可选内容不会重建控件或断开输入法。required／requiredText 提供必填说明，并通过同一目标发布必填语义；控件的 enabled／invalid 与 Field 的 invalid 由业务明确配置。error 自定义内容优先，errors 忽略空字符串并按首次出现去重，多条显示项目符号。

fieldGroup／fieldContent 收集子项，fieldSet(content) 提供 legend／description；fieldTitle、fieldDescription、fieldLegend、fieldError 和 fieldSeparator 也可单独组合。FieldLabel 现可接受任意 View，用来将整个 choice card 关联到 Checkbox／Switch；点击卡片和直接点击控件均只激活一次。`tests-cones/field` 覆盖真实输入、可选槽和 keyed 重排、窄宽／零尺寸、标签与 choice card、焦点保留及卸载收尾。

Button／Checkbox／Switch／Toggle 默认参与 Tab／Shift+Tab 遍历。`focus = FocusOptions(Some(handle), tabStop = true, order = 0)` 可提供外部 FocusHandle、Tab 资格及作用域内顺序；handle.requestFocus() 也可聚焦退出 Tab 的控件。禁用仍由原有 enabled 控制，布局和绘制顺序不决定 Tab 顺序；更换 handle 不重建区域或重置已有焦点。

Label 的 String 入口接受可选 SpanStyle，只覆盖其中明确的字体、字号、颜色等属性；完整 StyledText 使用公共富文本模型。Button 的标签复用 Label 与主题 action 字体，其基线来自实际文字布局。外部 FontCollection 只借用，默认字体资源由挂载作用域管理。

`card(content)`、`alert(content)`、`emptyState(content)` 接收完整正文 View，配置块中的 title／description 支持 String、StyledText 和 View。其余区域通过 header／action／footer／icon／media 成员组合；仅对应控件实际使用的区域有公开入口。可选区域撤销、动作换行和主题切换保留正文挂载。Card 提供 Default／Sm，Alert 提供 Default／Destructive，EmptyState 提供可选虚线边框及 Default／Icon 媒体。

`item { title("Project"); description("Updated just now") }` 提供 Default／Outline／Muted 外观和 Default／Sm／Xs 尺寸；`item(content)` 可放可选择正文，media／actions／header／footer 组合其他 View。`media(image)` 使用借用图片的 Cover 适配和圆角裁剪；独立动作在窄宽时移到下一行。可选 onClick 为行提供键盘与指针动作，内部按钮和文本选择保有输入；enabled=false 禁用整行。`itemGroup(ItemSize.Sm) { item { title("First") }; itemSeparator() }` 收集条目和普通子项，普通容器中的成员名为 contentItem。

`separator { axis = FlexAxis.Vertical; length = Some(24.0) }` 绘制逻辑尺寸的细线；默认横向填满可用宽度。`aspectRatio(content, 16.0 / 9.0)` 在父约束内计算比例外框并裁剪同一范围，可与 ImageView 的 Cover／Contain 组合。独立用法、明暗及 RTL 可在 [shadcn Gallery](examples/shadcn/README.md) 中操作；逐批交付范围见 [shadcn 实施记录](../../../docs/ui/shadcn/PROGRESS.md)。

`avatar("CX") { image = AvatarImageState.Ready(picture); badge() }` 展示借用图片和状态点；Empty／Loading／Failed 显示当前回退内容。Sm／Default／Lg 为 24／32／40，支持自定义回退和角标。`avatarGroup { avatar("CX"); avatar("AL"); count(4L) }` 提供逻辑方向重叠及显式计数，每个条目可设置稳定 key。

`skeleton(content) { loading = pending }` 保留正文挂载及测量，加载时撤下正文的呈现与输入，切回后继续使用原状态。无内容入口可设置 width／height 与 Circle 形状。`progress(Some(35.0)) { label("上传"); showValue = true }` 用 minimum／maximum 规范化确定值，`progress(None)` 表示未知总量；valueLabel 可替换默认百分比。明确 transition(None) 禁用数值过渡，主题 Reduced 最终生效。三种加载动画共用作用域时间轴，停止、隐藏、离屏与卸载不留下循环帧需求。

shadcn 外观取自 [Base Nova registry](https://ui.shadcn.com/r/styles/base-nova/avatar.json)（2026-11-04），相关 MIT 许可保存在 [shadcn 主题](../scoop.ui.style.shadcn/LICENSE.md)；Spinner 的 Lucide 路径许可见 [LICENSE-LUCIDE](LICENSE-LUCIDE)。

`buttonStyle { ... }`、`checkboxStyle { ... }`、`textFieldStyle { ... }` 和 `interactionStyles { ... }` 构造不可变的局部样式；`value.copy { ... }` 基于已有值修改。surface／marker／scrollbar、states 及 text／placeholder／description 是嵌套配置成员，重复块保留此前字段，完整赋值替换该字段。文字覆盖赋为 None 时恢复继承；transition(None) 禁用动画，把 transition 属性赋为 StyleValue.Inherit 则恢复主题过渡。ButtonStyle／CheckboxStyle 使用公共的尺寸 Options 和 StyleOverride。Inherit 保留本轮主题值，Set(None) 明确清除可空属性。InteractionStyles 按 hovered、pressed、focused 顺序组合，禁用时只应用 disabled。`styled(content, style)` 可以给完整控件添加有序 glow、drop shadow 或 blur；空效果与有效果使用同一个持久装饰节点。

`imageView(image, naturalSize)` 和 `pathView(path, viewBox)` 展示调用方拥有的 Image／Path。配置块提供 Contain／Cover／Fill／None、逻辑对齐和显式 clip；图像还可选择采样及透明度。自然尺寸使用逻辑单位，位图像素和根 DPI 不会重复计入布局。适配器不创建输入、加载器或计时器，替换和卸载不关闭借用资源。它们可以直接作为普通容器或文本行内内容。

内置控件经 `ThemeSource.read(scope)` 消费根的减少动画、高对比度、减少透明度和文字缩放。应用通过 `uiPreferences { ... }`／`UiWindow.preferences` 覆盖系统值，None 恢复跟随；主题来源的基础值保持不变。字号变化重新度量文字与控件，保留编辑模型、挂载及焦点；减少动画生效后，加载动效和过渡停止持续请求帧。局部显式字号、颜色和效果保持应用的选择。

Button／Badge、Toggle 和 Tabs 的默认状态配色在高对比度下保留主题的前景／背景配对；Switch 的 thumb 与轨道也保持可辨识。Slider／Sidebar 的默认焦点环不再衰减已解析的高对比颜色，字号缩小时宽度仍至少为 2 个逻辑单位。减少透明度时，控件派生的半透明底色合成到已知主题表面，原本没有填充的状态仍不填充。显式 surface／thumb／states 配色最后应用，可保留应用指定的透明度。

主题 painter、字体、段落和滤镜归 Element 的 ComponentScope 所有，应用无需为每个控件添加 use。主题和效果没有自己的时钟，控件动画只使用 Surface Animatable；静止时不安排更新或绘制。

Row／Column 按主轴排列，spacing 与交叉轴 AxisAlignment 通过 Props 更新；Stack 按描述顺序叠放。Frame 提供明确的尺寸、padding、两轴对齐和公共装饰，也可以选择 SurfaceKind 使用主题表面。未选表面时无需主题来源；从普通 Frame 切换为主题表面或反向切换都保留内容身份。Frame 透传经实际偏移后的基线，其他容器首期没有基线对齐策略。

`absolute([AbsoluteChild(view, horizontal, vertical, zIndex), ...], width, height, padding)` 在有限容器内定位。AbsoluteAxis 提供 Start／End／Center 的偏移与可选 tight 尺寸，以及 Stretch 两端定位；允许负偏移，无界父轴需要显式容器尺寸。容器不由子项撑开，每项只测量一次，不隐式裁剪。zIndex 控制同层呈现和命中，同值按描述顺序；子项 key、逻辑挂载和 Tab 顺序保留。布局错误在参数／测量边界报告，外观可组合 Frame。

有限宽度的 Label 使用定宽段落排版，无界宽度使用 layoutUnwrapped。行内按钮需要限定宽度时，可用 Frame.width 明确指定；文本行内子项仍只测量一次，不能要求先得到所在行的剩余宽度。

`scrollArea(content) { state = Some(scroll); height = Some(240.0) }` 给普通内容提供垂直滚动；ScrollState 可由组件或业务 Store 持有。axes 可选 Horizontal／Vertical／Both，滚动轴没有有限父约束时须提供明确尺寸。lineStep 默认 40，链式滚动在各轴边界把未用的原始残量交给父容器，`chain = ScrollChain.Contain` 可停止交接。offset 只发布最近 Accepted 的画面位置，wheel、拖动及翻页通过 `scroll.observeIntent` 交付 User 来源；整个内容树保持挂载，滚动不重建其描述。

ScrollArea／VirtualList 默认沿共享 ScrollControl 接入直接触摸和笔尖，可在 Props 或 builder 设置 `touchScroll = TouchScrollOptions(enabled = true, threshold = 8.0, lockAxis = true)`。TouchScrollOptions 和 ScrollChain 均从 `scoop.ui` 导入；后者由 UI 统一供滚轮与触摸滚动使用。短距离 tap 保留原动作，过阈值后取消子项点击候选，嵌套父视口只消费实际剩余位移。VirtualList 在归属移到容器后正常回收离屏子项，接触不会把已经取消的旧行固定在内存中。活动接触保持已显示的滚动条，抬起后按原 idleDelay 隐藏；停留不持续请求空帧。配置热切换、禁用、换绑和卸载取消旧接触，新的键盘、程序或语义定位优先。用法与受控 native 队列验证见 [touch-scroll](tests-cones/touch-scroll/README.md)；释放惯性通过 scrollPhysics.inertia 配置。

ScrollArea／VirtualList／MessageScroller、Input／Textarea／InputOtp 及对应 builder 使用 `scrollPhysics = ScrollPhysicsOptions(...)` 配置共享物理行为。`ScrollBoundary.Platform` 默认在 Darwin 回弹、Windows／Linux 硬边界；Hard 关闭回弹，Elastic 显式开启。resistance、maxOverscroll、shortContent、stiffness 和 dampingRatio 分别控制阻力、最大局部位移、短内容及释放 spring。inertia 接收 ScrollInertiaOptions，独立控制 Touch／Pen 的释放减速及速度范围；默认开启，硬边界仍可惯性滚动，系统 wheel／momentum 不另加模拟。嵌套沿实际参与路径共用一段轨迹，回弹不写入逻辑 offset 或虚拟索引；行回收不退休仍挂载的视口运动。正文、命中、文本 caret 与语义几何共同使用 Accepted 呈现位移；固定滚动条继续表示合法范围。根 reduceMotion 与主题 MotionPreference.Reduced 都会立即收束效果，普通触摸和系统 momentum 继续滚动。InputGroup、ButtonGroup、Command、Combobox 和 SidebarInput 保留输入配置。公开类型从 `scoop.ui` 导入，参数单位及配置示例见 [UI README](../scoop.ui/README.md)。

主题侧的显式减少动画在基础主题中设置：`tokens { motion { preference = MotionPreference.Reduced } }`。ThemeSource.read 会在此基础上合并各根偏好；withPreferences 用于解析一次有效外观，恢复默认偏好时仍返回它保留的基础主题。

macOS 的 ScrollArea 与 VirtualList 使用 `semantics = ComponentSemantics(name = Some("内容"), automationId = Some("content"))` 覆盖容器名称和标识。共享滚动控制发布 ScrollArea 节点，内容后代保留自己的语义；VirtualList 已在容器内发布逻辑 List；可配置 `itemSemantics = Some(ItemSemantics(source, describe))`，让未挂载条目也能通过原生集合查询名称等元数据。描述必须与 source 使用同一快照，查询不会 build、滚动或聚焦；未提供描述时保留真实身份与未知名称。重排保留 key，回收 View 保留逻辑条目，删除使旧引用失效。原生 Realize 仅准备目标行；Reveal 等待真实 Accepted 显露，后续滚动、删除、关闭会取消旧请求。行内有真实聚焦目标时，在描述中显式声明 `focusable = true`，条目 Focus 使用既有 LazyFocus；请求排队不等于实际焦点已建立。每个非零滚动轴提供实际 ScrollBar 范围、位置写入与增减动作；Input／Textarea 的轴沿同一编辑器视口，readOnly 不禁止滚动。视觉 Hidden 或闲置淡出不退休逻辑轴，只有实际呈现的条才发布自己的几何。范围来自 Accepted 快照，重新绑定先撤下旧范围。原生进程内和实窗验证分别见 [组件语义测试](tests-cones/accessibility/README.md)与[滚动场景](tests-cones/scroll-accessibility/README.md)。

`ScrollArea`、`VirtualList`、`Textarea` 和 `MessageScroller` 共用 `scrollbarBehavior { ... }`。默认 FollowSystem 跟随根的自动隐藏偏好，未知时保持常显；Static 使用固定反馈，Custom 提供自动隐藏预设。`scrollbars = ScrollbarVisibility.Auto` 只判断原始正文是否溢出。比如：

```scoop
scrollArea(content) {
    height = Some(240.0)
    scrollbarBehavior {
        preset = ScrollbarPreset.Custom
        placement = ScrollbarPlacement.Gutter
        idleDelay = Duration.milliseconds(800L)
        idleWidth = Some(6.0)
        hoverWidth = Some(12.0)
        dragWidth = Some(14.0)
        minimumHitThickness = Some(18.0)
        trackPress = ScrollbarTrackPress.Jump
    }
}
```

类型位于 `scoop.ui.component`，Duration 位于 `scoop.time`。`autoHide = Some(false)`、`widenOnHover = Some(false)`、`shadeOnHover = Some(false)` 分别关闭隐藏、展宽或深浅变化；时长零表示立即切换。`revealOnScroll`、`revealOnViewportHover`、`revealOnEdgeHover`、`revealOnKeyboard` 控制唤醒，默认除整个视口 hover 外均开启；悬停触发开启时会重新检查当前指针，已经位于对应区域也可唤醒；`repeatDelay = None` 禁用轨道／箭头按住重复。`showTrack = Some(false)` 只取消轨道绘制。重复 receiver 调用保留先前覆盖，`copy` 不修改旧值。隐藏不截获正文点击；减少动画保留隐藏但立即切换，减少透明度使用不透明反馈。`placement` 可选 Overlay／Gutter；ScrollArea、VirtualList、MessageScroller 默认 Overlay，Textarea 默认 Gutter。Gutter 在测量前扣除固定最大条带，Auto／Always 和 idle／hover／hidden 之间保持正文尺寸；Hidden 不预留。Overlay 不缩小测量约束，以 ScrollPadding 为边缘焦点和插入点保留可滚到的空白，显露与翻页使用扣除条带的有效视口。Auto 只依据原始正文溢出，额外留白不会触发另一条。

滚动条使用当前主题，默认覆盖在视口边缘，支持拖 thumb、轨道翻页和主题可选箭头。竖条随 LayoutDirection 位于逻辑 End，双轴条在同一角落让位；方向热切换保留内容和焦点，并取消旧位置的拖条；左侧留白变化通过 LayoutCorrection 尽可能保持正文位置，不生成用户滚动意图。ScrollArea 的 `style { corners(Corners.all(8.0)); border(Some(Border.all(1.0, color))) }` 可配置视口装饰和圆角裁剪，不改变内容尺寸；`focusOutline` 可覆盖主题焦点环，焦点指示遵守 FocusVisibility。ScrollbarVisibility.Auto／Always／Hidden 控制显示；点击条保留内容焦点，视口自身聚焦时可以用 PageUp／PageDown／Home／End；Home／End 只定位启用的滚动轴，其他轴保留当前偏移，水平 RTL 的起终点对应物理右侧／左侧。程序可用 scrollTo／scrollBy、内容局部 Rect 的 reveal，或显式 animateTo；用户滚动会取消旧平滑动画。`tests-cones/scroll` 通过真实窗口验证嵌套残量、双轴、取消、滚动条、主题切换、焦点和裁剪。

ScrollArea 和 VirtualList 均可设置 `paintCache = PaintCachePolicy.Tiles(tileSize = 256, overscan = 1)`，默认 None。策略只缓存 UI drawLayer 声明的有限内容，公共 ParagraphHost 正文自动使用该入口；普通 draw 保持直接执行。绘制缓存与 retention 独立，切换不会替换 ScrollState 或子项实例。含滤镜的记录按 Graphics 的独立裁剪能力正常重放，滤镜也可放在完整内容合成阶段。`tests-cones/paint-cache` 验证四种缓存组合、真实输入、文字更新和停帧。

已放置的内容目标通过 FocusHandle、Tab 或 KeyNavigation 聚焦时会自动显露；嵌套 ScrollArea 每次调整最近一层，等成功帧后继续外层。滚动条与 Portal 按自己的实际场景路径处理。`tests-cones/reveal` 用真实键盘和窗口覆盖这条路径、失败帧、不可显露目标、新位置请求与隐藏时取消。

`virtualList(source, ItemExtent.Fixed(24.0)) { state = Some(scroll); height = Some(240.0) }` 使用 ItemSource 按需创建固定高度项，orientation 可选 Vertical／Horizontal；两轴须有界或提供明确尺寸。overscan 是视口前后额外实现的逻辑长度，默认 0。列表复用同一套滚轮、键盘、主题滚动条与视口 clip，只为可见范围、overscan 和活动交互实现条目。水平 RTL 从右侧排列，默认首屏显示逻辑首项；方向切换保留阅读 key 和已保留的条目实例，图片和文字不镜像。离开范围且无活动交互的项正常卸载，再进入时创建新实例；数据本身可以保存在外部模型。

条目的焦点、按下序列和等待惯性的路由在滚出屏幕后继续有效，交互结束后恢复淘汰；真实删除不受这种保留阻止。固定范围算法、零视口和参数检查在本 Cone 测试；`tests-cones/virtual` 用一亿条的数据源验证实际构建数、Dispose、横纵轴、缩小范围以及跨帧的指针／焦点／惯性行为。`tests-cones/variable` 覆盖可变 extent、按 key 对齐、阅读锚点、FollowEnd 与失败帧。

`retention = ItemRetention.KeepAlive(32L)` 额外保留最近使用的 32 个离屏实例，复用其局部 State 与资源。它不改变 viewport／overscan 和交互固定范围，也不创建绘制缓存；默认 Dispose。容量缩小和真实删除在隐藏窗口中也会处理。`tests-cones/retention` 覆盖缓存淘汰、状态及 shape 更新、嵌套缓存、动画停帧和模态输入边界的保留。

[主题展示示例](examples/themes/README.md)并列展示公共绘图部件，提供三风格、明暗、系统跟随和真实控件／效果切换。

[下载与后台校验示例](examples/download/README.md)展示 UI owner 的异步 HTTP 与真实后台 Thread 如何协作，通过 ApplicationProxy 合并发布进度；覆盖共享业务 State、组件自有状态、取消／替换以及应用退出后的线程回收。

规范见 [基础控件规范](../../../docs/specs/SCOOP-UI-COMPONENT-SPEC.md)和[主题绘图规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md)。普通覆盖／来源／布局测试在本 Cone，真实 Metal／输入／主题切换组合位于 `tests-cones/interaction`，容器与富文本组合位于 `tests-cones/layout`；实施范围与运行结果记录在 [UI 实施记录](../../../docs/ui/PROGRESS.md)。

`tests-cones/absolute` 验证显式子项呈现顺序、真实像素与命中、keyed 更新和 Tab。焦点、组内导航与新增基础容器的本期结果见 [基础交互实施记录](../../../docs/ui/foundations/PROGRESS.md)。

`ItemExtent.Variable(estimate = 24.0)` 按视口逐项测量可变高度内容，允许实际高度为零。尺寸索引只保存访问过的 key，未知高度仍为估计；前方插入或测量变化时保留首个可见 key 和项内位置。`anchor = VirtualListAnchor.FollowEnd` 只在原先已到末端时跟随新增内容，向上阅读时保持阅读位置。

KeepVisible 与 FollowEnd 在惯性和回弹期间仍生效。prepend、已实现项的行高变化或视口改变会先保持同一阅读基点，再收束旧物理运动；`anchor = VirtualListAnchor.None` 保留数值偏移并按新范围收敛。正常续滑不会被旧锚点固定，显式 animateTo 和 handle 的按 key 定位仍优先。MessageScroller 沿同一规则保持 Holding 的阅读位置，同一次用户滚动的惯性到达末端阈值后也可恢复允许的自动跟随；内容或范围变化不会替用户恢复策略。已经在实际末端且没有回合留白时直接跟随，不重复定位或提前取消回弹。组合验证见 [可变尺寸列表](tests-cones/variable/README.md)和[消息列表](tests-cones/message-scroller/README.md)。

把稳定的 `VirtualListHandle` 传给 `handle`，挂载后可调用 `handle.scrollToItem(key, ScrollAlignment.Center)`。列表用实际测量修正位置，`isPending` 在成功帧后清除；更新的普通 ScrollState 定位优先。handle 不另存 offset，ScrollState 继续用于观察位置、滚轮和滚动条。

`handle.reading` 是最近 Accepted 的 `VirtualReadingSnapshot?`，包含排除 overscan 的可见半开区间、首个实际阅读项的 `VirtualReadingAnchor(key, offset)` 及 atEnd；首帧之前和解绑后为 None，读取可建立 State 依赖。保存 anchor 后可调用 `handle.restore(anchor)`，当前源不存在该 key 时返回 false。有效恢复只测量必要项，用户新请求或源替换会取消旧恢复；显式恢复不按旧索引猜另一条数据。阅读偏移沿逻辑排列方向计量，ScrollState 仍使用物理坐标。窗口测试覆盖历史 prepend、图片自然尺寸更新、恢复中断、失败帧和水平 RTL。

`handle.focusItem(key, ItemFocusEdge.First)` 按需实现并定位条目，再聚焦行内首个可用 Tab 目标；Last 选择末个目标。isPending 也包含这个聚焦过程。未知 key、没有可用目标、真实删除、禁用或更新的焦点／滚动请求会结束旧等待；外层缓存的列表可以因待聚焦归属恢复布局。`tests-cones/focus-items` 检查真实按 key 聚焦与实例数量。

`navigation = ItemNavigation.Sequential` 启用逐行 Tab 访问；`ItemNavigation.Roving(direction = LayoutDirection.Ltr, loop = false)` 让列表占一个 Tab 停靠点，并用方向键和 Home／End 导航。轴沿 orientation，入口顺序沿 focus.order；启用时视口自身不再额外占一个 Tab 停靠点，None 默认模式保持普通滚动视口行为。选择／激活由上层组件决定。`tests-cones/navigation-items` 覆盖真实键盘、嵌套列表与焦点作用域、禁用／空项、实例卸载后的返回、RTL、循环和默认动作优先级。


`checkbox(checked)` 的混合态激活后变为 Checked，不做三态循环；Boolean 工厂与三态工厂共享同一个 Checkbox 和 Press，显式 CheckboxProps 使用 CheckState 绑定。全选框可以用普通 Binding 从子项派生 Mixed 并写回所有子项。

`switchControl(checked) { size = SwitchSize.Sm; labelTarget = Some(target) }` 提供两种尺寸，标签／指针／Space 读写同一 Boolean 绑定，Enter 不切换。`switchStyle { track { ... }; thumb { ... }; selectionTransition(None) }` 覆盖轨道、滑块与选择动画；默认扩大点击区不占布局，RTL 反向滑块位置。

`toggle("Bold", pressed) { variant = ToggleVariant.Outline; size = ToggleSize.Sm }` 支持文字、StyledText、自定义 View 和 ActionContentStyle 内容回调。按住时的临时状态与已选中的 pressed 分开，Space／Enter 激活后切换；外部 setter 可拒绝，控件不保存镜像值。通过现有 ButtonStyle 覆盖表面、文字、状态和过渡。

`inputOtp(editor, 6L)` 使用同一 TextEditorState 与 TextEditHost，把 grapheme 放进固定槽。`group { slot(0L); slot(1L) }` 与 `separator()` 显式组合分组，索引须依序覆盖 length；不配置时为单组。默认 ASCII 数字，Alphanumeric 接受 ASCII 字母数字，Custom(TextEditPolicy) 保留自定义策略并收紧长度。onComplete 只在实际接受的编辑达到完整长度时通知，外部 replaceAll 不触发。支持 FieldTarget、只读、禁用、invalid、局部槽样式、RTL 和窄视口；复制不包含分隔图形。

`slider(values.binding(), RangeSpec(0.0, 100.0, Some(5.0), minimumGap = 10.0))` 接受有序的 `Array<Double>`，每个下标保留同一个 thumb 和焦点。支持横纵、RTL、反向、离散步及连续范围；连续模式须显式配置 keyboardStep。点击轨道或拖动更新同一绑定，正常释放或键盘实际修改触发 onValueCommit，Escape／禁用等取消触发 onValueCancel；外部新值不会被旧拖动覆盖。`thumb { focus { handle = Some(handle) } }` 可逐项配置焦点与标签，`sliderStyle` 覆盖轨道、选中范围、thumb 及状态动画。真实窗口组合覆盖在 `tests-cones/slider`。

`tabs(selected.binding()) { item(Choice(ViewKey.Text("account"), 0, "Account"), form) }` 提供 Default／Line、横纵与 RTL、Auto／Manual 激活。方向键导航跳过禁用项，Manual 用 Space／Enter 提交，普通 Tab 进入正文；拒绝 setter 不改变当前面板。默认 Dispose，`retention = ContentRetention.KeepMounted` 保留非活动正文的实例和逻辑更新，停用其布局、输入和动画帧。横纵／主题切换及 keyed 重排保留编辑器状态；`tests-cones/tabs` 覆盖实际键盘、IME、生命周期和停帧。

`breadcrumb { item("Home", navigate); current("Page") }` 提供路径链接、当前项、分隔符、ellipsis 与 custom 内容槽，支持长项单行省略、换行和 RTL。当前项与默认省略号不登记输入，链接复用 Button；同 key 重排不重建正文。`pagination(page.binding(), pageCount)` 以 Long 页码驱动业务视图，自动范围由 `paginationItems` 计算，支持显式 item／ellipsis、compact、本地化文字和内容回调。外部越界页码保持原值，空数据停用导航，拒绝 setter 不产生通知；真实窗口验证在 `tests-cones/navigation`。

`popover(open.binding(), content) { trigger("Open"); title("Details") }` 提供真实可编辑的锚点弹层。mode 选择 NonModal／FocusContained／Modal，focus 独立配置；非模态外部点击继续执行，FocusContained 保留背景 pointer／wheel 并约束焦点，Modal 阻挡背景输入。支持外部 anchor、嵌套内容、业务关闭 veto、拒绝绑定、定位翻转、退出重开和减少动态；`tests-cones/popover` 验证真实键盘、指针、编辑与生命周期。

`dialog(open, content)`、`alertDialog(open, content)` 与 `sheet(open, content)` 提供同一 Portal 模态域中的表单、确认和四边侧板。共同配置保存在 ModalOptions，DSL 展开配置成员；支持原生输入、固定 header／footer、可借用 ScrollState 的滚动正文、嵌套焦点恢复、veto 与拒绝绑定。DialogFooter 在窄宽反向纵排；AlertDialog 默认聚焦取消，action 不替应用关闭；Sheet 的 Start／End 随 RTL 变化。`tests-cones/modal-family` 定向覆盖这些真实窗口行为。

`menu { ... }` 构建显式 key 的 MenuContent，支持动作、勾选、类型化单选、嵌套子菜单、标签、分隔线和快捷键标签。`dropdownMenu(open, content)` 使用普通 trigger 或外部 anchor；`contextMenu(content, menu)` 包装原内容并提供右键、Shift+F10 与直接触摸／笔尖长按；`menubar(open) { menu(key, text, content) }` 提供横向入口导航。三者共用真实焦点、Unicode typeahead、Hover 通路及 Portal 生命周期，支持受控拒绝、关闭 veto、Tab 续行、RTL 和长菜单滚动。`tests-cones/menus` 覆盖原生窗口输入及保留行为。

Command 用 `command(source, query)` 构建带真实编辑器的命令搜索。`source` 可以是 `commandItems { group(key, "Actions") { item(key, "Save", onSave) }; separator(key) }` 的快照，或提供稳定索引的 CommandSource。默认按 cmdk 的模糊评分搜索标题和 keywords；`filter = CommandFilter.Provided` 保留调用方结果，`Custom` 替换评分。方向键只更新 active，焦点留在输入；输入法预编辑不参加搜索，Enter 只执行当前可用候选。列表按需挂载并滚动显露活动项，custom content／icon、disabled、ShortcutChord 标签、空结果、受控 active、样式／字体／RTL 都可组合；Dialog 示例直接复用现有 dialog 和输入 FocusHandle。评分代码的归属见 [LICENSE-CMDK](LICENSE-CMDK)。

Combobox 提供单／多选、Unicode 包含搜索、外部候选、自定义项、分组、可清空输入和带删除／键盘导航的 chips；同一编辑模型可放在入口或弹窗中。`ChoiceSource<T>` 支持大列表的元数据和领域值定位，选择不会隐式改写查询，输入法组合文字不用于筛选或提交候选。

`navigationMenu(Some(open.binding())) { item(key, "Guides", content); link(key, "Home", navigate) }` 组合一级页面链接和普通内容面板，支持横／纵布局、列表或入口锚点、延迟悬浮、跨间隙移动、受控关闭及 RTL。面板中的 `navigationMenuLink`、表单和布局保持原有焦点和按键行为；应用负责实际路由。`tests-cones/navigation-menu` 验证真实指针、键盘、编辑、焦点恢复和动画收尾。

`resizable(sizes) { panel(key, content) { minimum = 80.0; collapsible = true }; panel(other, editor); withHandle = true }` 使用受控 `Array<ResizableSize>` 描述比例与折叠状态，自动在相邻面板间放置键盘／拖动把手。min／max 是逻辑长度，折叠保留恢复比例，取消不覆盖外部新值，窗口缩放只重新布局；正文以稳定 key 保留挂载，零尺寸停止放置。横／纵、RTL、嵌套、主题与局部样式可组合，`tests-cones/resizable` 覆盖真实窗口拖动、焦点和状态生命周期。

Resizable 的 `handle(key) { hitWidth = 32.0 }` 可扩大实际命中带，默认 20，要求有限正值；它围绕分隔线居中并随主题密度缩放，不改变面板尺寸。鼠标与 Touch／Pen 使用相同 Drag、键盘及 Binding 提交／取消规则；[实窗场景](tests-cones/resizable/README.md)分别验证两种来源。Carousel 同样复用公共 Drag；轮播区域内任一活动触点会暂停自动播放，即使关闭 hover／focus 暂停，最后一根手指结束后再等待完整 interval，见[轮播场景](tests-cones/carousel/README.md)。

`drawer(open, content)` 复用 ModalOptions 与 Portal，提供四方向拖动、垂直 Fraction／Length 吸附、受控索引、可聚焦 swipeHandle、三种模态策略和显式 parent 嵌套。DrawerHandle 观察实际展开长度并请求吸附；滚动正文、关闭 veto、取消回弹、尺寸变化及停帧由 `tests-cones/drawer` 的定向实窗覆盖。拖动起点按真实输入位置和 Accepted 命中区判断，支持无 hover 的 Touch／Pen。正文 ScrollPan 使用相同阈值，并在起始边界无滚动余量时让出触点；已开始的正文滚动不在中途转成抽屉拖动，见[抽屉场景](tests-cones/drawer/README.md)。

Carousel 与 Drawer 的 Props 和 builder 均提供 `nativeSwipe = true`，沿公共 SwipeGesture 接受系统交付的离散滑动，可单独关闭。Carousel 水平 LTR 向左下一项、RTL 相反，垂直向上下一项，继续遵守 loop、边界与 selected 绑定。Drawer 向内选择下一更大 snap，向外选择下一更小 snap；最小 snap 再向外时由 swipeToClose 决定是否走原关闭请求，保留 veto 与绑定拒绝。起点沿同一正文滚动边界、把手 only 和嵌套规则准入。直接触摸仍由原 Drag 连续跟手；原生 swipe 接替活动拖动时先取消旧拖动，释放不会重复提交。按钮、键盘和现有命令继续作为等价入口。


`sidebar(open, content) { header(view); footer(view); inset(workspace) }` 提供 Sidebar／Floating／Inset、逻辑侧、Offcanvas／Icon／None、rail 和作用域内 Primary+B。Auto 根据父级宽度切换窄布局 Drawer；主编辑区保持实例，导航跨宿主的业务状态显式提升到外部。分组、菜单、独立动作／badge、子菜单、Skeleton、Input 与 Trigger 均有真实成员 DSL，菜单使用普通 Tab 与 Button。`tests-cones/sidebar` 覆盖折叠、稳定 key、快捷键、关闭拦截、窗口缩放、编辑状态及隐藏停帧。

`ToastState` 与 `toaster(state)` 提供窗口内显式通知。`show` 返回 ToastId，`update` 原位替换内容与时长，`dismiss` 只关闭一次；同一状态只绑定一个活动宿主。六种语义类型、六位置、队列／堆叠、动作、滑动及暂停剩余时间由 `tests-cones/toast` 的真实窗口用例覆盖。异步工作继续由应用 Job 管理，关闭后的迟到更新返回 false。


`questionnaire(state, onSubmit)` 以应用持有的 QuestionnaireState 组合单选、多选与 freeform。QuestionKey／QuestionChoiceKey、类型化结果、显式 Skip、条件题、外部错误和快照恢复按当前定义处理；每份文字草稿只有一个 TextEditorState。Next／Submit 可触发校验，busy 明确阻止重复提交，Primary+Enter 与固定选项快捷键尊重编辑和 IME；部件包装、文案、焦点环和主题均可配置。`tests-cones/questionnaire` 覆盖真实窗口中的导航、编辑、取消和布局生命周期。

`contextMenu` 与 `nativeContextMenu` 的 Props 和 builder 都用 `longPress = LongPressOptions(enabled = true, delay = Duration.milliseconds(500L), threshold = 8.0)` 配置长按；LongPressOptions 从 `scoop.ui` 导入，Duration 从 `scoop.time` 导入。关闭长按保留右键与键盘入口，普通短按继续激活正文，内层文本长按或滚动成功接管后不再打开外层菜单。真实原生 tracking、热更新与文本组合的回归见 [菜单测试](tests-cones/menus/README.md)。

## Linux GNU

支持 Wayland 与 X11，共用公共实现、Skia Ganesh GLES 和 Fontconfig／FreeType。使用已准备的 GNU sysroot，运行 `scoop test sysroot/src/scoop.ui.component --release --sysroot ~/.scoop`；[主题展示](examples/themes/README.md)和[后台下载](examples/download/README.md)分别提供真实窗口组合测试。图形应用通过 `SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 选择显示协议。文本编辑使用 Ctrl 快捷键。依赖、测试范围及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
