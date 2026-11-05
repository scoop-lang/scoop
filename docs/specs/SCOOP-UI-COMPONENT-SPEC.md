# Scoop 基础控件库规范

日期：2026-11-04。状态：原有 Label／Button／Checkbox、Row／Column／Stack／Frame、完整主题展示及后台下载／宿主组合已完成 macOS aarch64 与 x86_64 Rosetta 验证，实际范围见 [UI 实施记录](../ui/PROGRESS.md)。新增场景协议迁移及基础交互按[本期实施记录](../ui/foundations/PROGRESS.md)分批验收。框架合同见 [UI 规范](SCOOP-UI-SPEC.md)，公共文本合同见 [文本排版规范](SCOOP-GRAPHICS-TEXT-SPEC.md)，主题绘图／几何见[主题规范](SCOOP-UI-THEME-SPEC.md)，分层与实施顺序见 [UI 设计](../ui/DESIGN.md)。

控件基础的布局、方向与公共交互演进按[控件基础实施记录](../ui/component-foundations/PROGRESS.md)逐批交付；下文描述已实施批次的现行合同。

shadcn 控件扩展的批次与平台验收见 [shadcn 实施记录](../ui/shadcn/PROGRESS.md)，操作控件、内容面板、头像与加载状态的合同分别见第 11～13 节。

## 1. 定位与依赖

生产 Scoop 源码不限定 Darwin，也不包含 OS native 代码或平台判断；键盘、文本服务、字体和呈现均依赖基础库的公共合同。当前只验证 macOS，其他后端补齐基础能力后以同一份上层源码重新构建。

`scoop.ui.component` 是独立的普通 Scoop Cone，提供基于 `scoop.ui` 的自绘基础控件。它依赖框架、公共 `scoop.ui.style` 及实际使用的 `scoop.graphics`、`scoop.graphics.text`、`scoop.surface`、`scoop.window` 和 core／std／math／platform；使用下层类型的源码须声明相应直接依赖，不能借由框架的间接依赖访问它们。

`scoop.ui` 的生产代码、测试和自绘示例不依赖本库。本库只调用框架及下层库的 public 接口，不使用 UI 的 internal 节点、特殊控件注册表或按控件类型分支的入口。任何应用自己的控件库都可以使用相同的 Component／Element、测量／绘制 context、输入区域和动画能力。

框架决定挂载身份、树的更新和生命周期；[公共样式库](SCOOP-UI-STYLE-SPEC.md)提供装饰与效果应用，本库决定控件外观、文字输入与裁剪策略、点击语义、布局算法及默认样式。公共文本模型、样式继承、shaping 和行内对象排版由 graphics.text 完成，段落与子组件的测量／缓存适配使用 scoop.ui.text；路径绘制、帧呈现和动画求值继续由 graphics／surface 完成。第一期控件全部自绘，不嵌入原生 Label／Button。

首期与 UI 框架相同，先在 macOS aarch64 完成实现与验证，最后统一补验 x86_64；Windows 接入后续单独推进。普通控件测试使用本 Cone 的 scoop.test，实施记录见 [UI 实施记录](../ui/PROGRESS.md)。

Absolute／ScrollArea／VirtualList、弹层、组内导航及 Input／Textarea 所需能力的扩充设计见[基础交互能力设计](../ui/foundations/DESIGN.md)。各项按[本期实施记录](../ui/foundations/PROGRESS.md)分批验收，当前公开合同以本文件及 UI 规范为准；Code Block 不在本次扩充设计范围内。

## 2. 控件与使用入口

第一批控件包括：

| 控件 | 职责 |
| --- | --- |
| Label | 纯文本或 StyledText、字体／样式输入及行内 View 绑定，复用公共段落测量与绘制 |
| Button | 标签、hover／pressed／focus／disabled 外观、指针与键盘激活 |
| Checkbox | CheckState 三态或 Boolean 绑定、自有布尔状态、指针与 Space 切换及 disabled／invalid 外观 |
| Input／Textarea | 共享 TextEditorState 的单行／多行输入、主题表面、占位文本、验证说明与滚动条 |
| Row／Column | 子项的单轴排列、spacing 与交叉轴对齐 |
| Stack | 子项叠放和对齐 |
| Absolute | 有限容器内的显式轴定位、拉伸及同层 zIndex |
| ScrollArea | 单轴／双轴滚动、残量交接和主题滚动条 |
| VirtualList | 按需子项、固定／可变 extent、阅读锚点和按 key 定位 |
| Frame | 单个子项的 padding、固定尺寸、对齐、背景和显式 clip |

这些控件是普通的持久 Component class，构造器接收各自的 Props；render 通过公开的 element 和 component 入口组织绘制元素及子控件。输入区域和布局策略可放在本库私有的 Element class 中，文本缓存复用公共 ParagraphHost；这些 class 仍只实现公开协议，不获得框架内部访问权。

常用入口提供返回 View 的普通函数，保留具体组件类型并使用延迟工厂。最小调用形式为：

```kotlin
import scoop.ui.*
import scoop.ui.component.button
import scoop.ui.component.column
import scoop.ui.component.label

class Counter: Component {
    private var count: Int by state(0)

    public override fun render(): View =
        column {
            this.label(f"Count: ${this @Counter.count}")
            this.button("Increment", { this @Counter.count += 1 })
            this.spacing = 12.0
        }
}
```

label／button／checkbox／row／column／stack／frame 都属于 `scoop.ui.component`。它们不是编译器语法或框架内建节点，也不在 `scoop.ui` 中提供别名。工厂函数必须将变化的文字、回调、样式、Binding 和子描述放入 Props；不得把变化参数只捕获在首次构造闭包中。

显式 `component(props = ..., create = ::Label)` 与便利函数沿用相同的类型／key 匹配。改变 key 或组件类型重建实例；普通输入更新保留状态、区域和可复用缓存。子数组使用普通 Array<View>，函数体中的未使用表达式不会自动成为子项。

### 2.1 接收者构造入口

Label／Button／Checkbox 的文字或显式内容、动作和外部 Binding 是必需参数；其余输入使用同步执行一次的接收者配置。入口返回普通 View，不挂载组件、运行事件回调或创建字体、painter、区域等资源。配置结果通过现有 LabelProps／ButtonProps／CheckboxProps 及自有勾选组件的 Props 发布，组件与 key 匹配方式保持不变，不保留原来的长参数函数重载。

```scoop
label(text: String, configure: PlainLabelBuilder.() -> Unit = {}): View
label(content: StyledText, configure: StyledLabelBuilder.() -> Unit = {}): View
button(text: String, onClick: () -> Unit, configure: ButtonBuilder.() -> Unit = {}): View
button(text: StyledText, onClick: () -> Unit, configure: StyledButtonBuilder.() -> Unit = {}): View
button(content: View, onClick: () -> Unit, configure: ContentButtonBuilder.() -> Unit = {}): View
checkbox(checked: Binding<Boolean>, configure: CheckboxBuilder.() -> Unit = {}): View
checkbox(checked: Binding<CheckState>, configure: CheckboxBuilder.() -> Unit = {}): View
checkbox(initialChecked: Boolean = false, configure: CheckboxBuilder.() -> Unit = {}): View
```

| Builder | 全部可写属性与默认值 |
| --- | --- |
| LabelBuilder（两种 Label 的共享基类） | fonts: FontCollection? = None；settings: ParagraphStyle? = None；theme: ThemeSource? = None；key: ViewKey = Position |
| PlainLabelBuilder : LabelBuilder | 继承上述字段；style: SpanStyle? = None |
| StyledLabelBuilder : LabelBuilder | 继承上述字段；inlines: Array<InlineView> = [] |
| ContentButtonBuilder（显式 View 及文字入口的共有配置） | enabled: Boolean = true；invalid: Boolean = false；variant: ButtonVariant = Default；size: ButtonSize = Default；group: GroupPosition? = None（第 16 节）；style: ButtonStyle = buttonStyle()；theme: ThemeSource? = None；key: ViewKey = Position；focus: FocusOptions = FocusOptions() |
| ButtonBuilder : ContentButtonBuilder | 继承上述字段；fonts: FontCollection? = None |
| StyledButtonBuilder : ButtonBuilder | 继承上述字段；settings: ParagraphStyle? = None；inlines: Array<InlineView> = [] |
| CheckboxBuilder | enabled: Boolean = true；invalid: Boolean = false；style: CheckboxStyle = checkboxStyle()；theme: ThemeSource? = None；key: ViewKey = Position；focus: FocusOptions = FocusOptions()；labelTarget: FieldTarget? = None（第 15 节） |

LabelBuilder／StyledButtonBuilder 的 settings 成员接受 ParagraphStyleBuilder；PlainLabelBuilder 的 style 接受 SpanStyleBuilder；ButtonBuilder／CheckboxBuilder 的 style 接受各自样式 Builder。真实嵌套成员返回 Unit，基于当前值继续修改，空块保留配置；完整赋值替换。PlainLabelBuilder.style 的 None 表示没有局部覆盖，第一次 style 块从全部继承的 spanStyle() 开始。String Button 的标签段落继续默认居中，StyledText Button 使用其 settings；不为某种文字模式暴露没有意义的输入。

Builder 构造器内部化，公共基类只服务上述真实复用。工厂和容器子项成员共用构造实现；未使用的 View 不自动成为子项。发布后修改捕获的 builder 不改变 Props，数组和不可变子描述可共享；Binding、FontCollection 和普通事件函数按原有引用／借用规则保存。回调抛错不发布部分 View，构造不读取或修改绑定值。onClick 始终进入当前 ButtonProps，不捕获到首次组件工厂里；自有勾选状态仍只在原有持久组件首次挂载时初始化。

Input／Textarea 同样以共享编辑模型为必需参数：`input(state: TextEditorState, configure: InputBuilder.() -> Unit = {}): View` 与 `textarea(state: TextEditorState, configure: TextareaBuilder.() -> Unit = {}): View`。InputBuilder 的可写属性是 InputProps 除 state 外的全部输入，另有 key: ViewKey = Position；属性名、类型、默认值与第 9 节编辑合同及 InputProps 一致。TextareaBuilder 继承这些共同输入，增加 rows: Long = 4、wrap: Boolean = true、tab: TextTabBehavior = Navigate、submit: TextSubmitKeys = PrimaryEnter、scrollbars: ScrollbarVisibility = Auto。两者 style 成员接受 TextFieldStyleBuilder，重复配置保留当前局部样式。onEdited／onSubmit 是普通 `(TextEditorState) -> Unit` 通知函数，默认不执行动作；构造不编辑模型或调用它们，回调经 Props 更新。

多子项容器使用 `row(configure: LinearBuilder.() -> Unit = {})`、`column(configure: LinearBuilder.() -> Unit = {})`、`stack(configure: StackBuilder.() -> Unit = {})`，均返回 View。LinearBuilder 提供 spacing: Double = 0、alignment: CrossAlignment = Start、justify: MainAlignment = Start、padding: Insets = Insets()、logicalPadding: LogicalInsets? = None、clip=false；StackBuilder 提供 alignment: Alignment = Alignment()。它们继承 ChildrenBuilder 的 key: ViewKey = Position、有序 add(View)／addAll(Array<View>) 和控件真实成员；同名成员追加一次并返回 Unit，使用顶层工厂相同的构造实现。构造结束时子项转为独立 Array，空容器有效；循环、条件与追加按源码执行顺序组合，捕获的 builder 后续追加不影响已发布 Props。

`frame(content: View, configure: FrameBuilder.() -> Unit = {}): View` 固定接收一个完整内容，FrameBuilder 提供 width／height: Double? = None、padding: Insets? = None、logicalPadding: LogicalInsets? = None、alignment: Alignment = Alignment()、style: StyleOverride = styleOverride()、surface: SurfaceKind? = None、theme: ThemeSource? = None、key: ViewKey = Position。style 成员接受 StyleOverrideBuilder 并基于当前值配置。Frame 不继承 ChildrenBuilder；尺寸校验与包装行为仍由原有 FrameProps 和 Frame 完成。

Absolute、ScrollArea、VirtualList 使用以下入口，仍构造原有 Props 和具体组件：

```scoop
absolute(configure: AbsoluteBuilder.() -> Unit = {}): View
absoluteChild(content: View, configure: AbsoluteChildBuilder.() -> Unit = {}): AbsoluteChild
scrollArea(content: View, configure: ScrollAreaBuilder.() -> Unit = {}): View
virtualList(source: ItemSource, itemExtent: ItemExtent, configure: VirtualListBuilder.() -> Unit = {}): View
```

AbsoluteBuilder 的 width／height 为 None、padding 为 Insets()、logicalPadding 为 None、key 为 Position；add(AbsoluteChild)／addAll(Array<AbsoluteChild>) 按顺序收集，item(content) 配置 horizontal／vertical（均 Start()）和 zIndex=0 后追加一次。它不继承 ChildrenBuilder。AbsoluteChild 保留单条完整几何值构造；absoluteChild 与 item 共享配置实现。逻辑子项顺序不因 zIndex 绘制排序而改变。

ScrollAreaBuilder 的属性与 ScrollAreaProps 除 content 外的全部字段一致，VirtualListBuilder 与 VirtualListProps 除 source／itemExtent 外的全部字段一致，另有 key=Position。默认值沿用本规范的滚动与虚拟列表合同。两者 scrollbar／focus 嵌套块基于当前选项配置；required source、itemExtent 不放入可缺失槽。发布时仅保存状态、资源和 source 引用，构造不读取 source.count、枚举 source、创建条目、滚动或获取焦点。ChildrenBuilder 的 absolute／scrollArea／virtualList／portal／styled 成员均调用对应顶层入口并追加一次，返回 Unit。

### 2.2 公共属性与状态归属

只读输入如 Button.enabled 作为普通值经 Props 更新；用户交互会修改的公开值如 Checkbox.checked 可以接收 Binding<T>；hover／pressed 等内部表现由持久组件的私有 State 保存。绑定读写同一份外部状态，不在控件内部复制一份值并靠回调保持同步。调用者可以提供自己的 State、Context 中 Store 持有的 State，或普通模型 getter／业务方法组成的 Binding。

控件每轮 render 读取当前 Props 中的 Binding，交互写入当前路径。更换 Binding 时按框架规则替换依赖，保留同一控件的挂载和内部交互状态；不能把变化绑定只存进首次构造的 delegate。公开计算属性的依赖来自其实际 State 读取；没有 State 的普通属性不会自动可观察。

便利函数返回 View 描述，调用者不通过它访问挂载实例或私有成员。跨控件联动通常由共同组合组件或业务模型持有状态，消费方在 render 中读取并计算输入。复杂控件可以公开普通状态模型，但不要求为每个布尔属性建立额外模型、全局 Store 或实例引用协议。

## 3. Label

Label 接受 String 加可选 SpanStyle 的便利输入，也接受公共 StyledText、字体来源、ParagraphStyle 和 `Array<InlineView>`。便利输入转换为相同的纯文本描述，不建立另一套排版路径。默认字体、字号和颜色来自当前 Theme 的公共 typography／语义参数；框架与控件库都不建立全局可变默认主题。调用方可以传入自己的字体及样式，其资源默认借用。

显式组件接收 `Props<LabelProps>`。LabelProps 保存纯文本及可选 SpanStyle 覆盖，或完整 StyledText，另有可选 FontCollection、ParagraphStyle、主题来源和行内绑定；label 的 String／StyledText 重载构造相同的输入。Plain 以当前主题 TextStyle 为基础，仅覆盖明确的 SpanStyle 属性；完整 StyledText 保留调用者指定的内容模型。文字与实际已解析样式不变时保留规范化内容。字体为 None 时，挂载的 Label 自己创建一次系统 fallback 字体集合并登记到 scope；外部字体只借用，切换来源不关闭调用方资源。

LabelProps.baseStyle 可显式提供本轮完整 TextStyle，供 Button 等组合控件传入从当前主题解析的操作标签样式；Plain 的局部 SpanStyle 再覆盖它。显式 baseStyle 或完整 StyledText 已包含所需文字默认值时，不要求仅为排版建立环境主题来源。Button 对未明确指定子标签主题／基础样式的 Label 传入本轮 action 字体与表面文字颜色，保留标签自身 SpanStyle。

Label.render 使用 scoop.ui.text.inlineChildren 把行内对象绑定成普通 keyed children；自己的 Element 在 onMount 中通过 ComponentScope 登记 ParagraphHost，在 measure／scene 中委托公共适配。字体混排、SpanStyle 继承和内容索引由 graphics.text 负责，Label 不复制这些算法。

父约束给出有限最大宽度时按该宽度布局，无界时使用 layoutUnwrapped。最终 LayoutMetrics 的 size 遵守 Constraints，基线来自同一 TextLayout；绘制、对象放置及查询也使用这个结果，不另行估计文字宽度。换行、对齐、maxLines、ellipsis 和 Unicode 行为采用 graphics.text 合同。默认不裁剪超出外框的绘制，需要 clip 时显式包住整个文字与子项绘制作用域。

ParagraphHost 按内容、字体来源／revision、样式、段落设置、宽度和行内 metrics 管理缓存。框架每帧可以再次调用 measure，输入未变化时复用已有结果；行内图片尺寸、按钮文字或布局动画改变实际 metrics 时重新排版，纯位置、整体透明度或不改变尺寸的子项颜色变化复用布局。

行内子组件在插字、换行及省略号隐藏后保留 key、实例和状态；截断项不放置、不绘制，在该帧成功提交后退出输入和焦点。改变 key、移除对象或卸载 Label 才执行正常卸载。ImageView／PathView 可以使用与 Button 相同的行内协议；Label 和内容适配均不接管外部 Image／Path 的所有权。

Label 本身不创建 InputArea，其行内控件可以拥有并放置自己的区域。文字选择、编辑文档、marked range、剪贴板和撤销不属于首期 Label；公共 graphics.text 的 hitTest／caret／selection 及替代文字导出仍可由其他控件使用。

Label 的 settings 为 None 时通过 ParagraphHost 继承有效布局方向；明确提供完整 ParagraphStyle 时使用其文本方向。settings 配置块从默认 ParagraphStyle 或已有显式值复制，提供完整段落设置；Plain 与 Styled 两种文字模式使用同一规则。

### 3.1 SelectableStyledText

SelectableStyledTextProps／SelectableStyledText／selectableStyledText(content) 是公共 SelectableTextHost 的普通组件适配。receiver builder 提供 fonts、settings、inlines、options、focus、actions、decorations、onSelectionChanged 和 key；StyledText 保留全部样式及行内对象，通过 inlineChildren 构造相同的 keyed 子描述。它自己不复制文本选择、命中或剪贴板算法，也不持有额外编辑模型。

未指定 fonts 时挂载创建一次 fallback FontCollection；外部字体只借用。Element 的 update／measure／scene 委托同一宿主，焦点与范围动作遵守 UI 规范第 7.5.4 节。Label 继续保留原来的纯显示行为。

## 4. Button

Button 的输入包括内容 View、onClick、enabled 及样式。组件在 onMount 创建稳定 InputArea 并登记公共 Press，render 读取其 InteractionState 传给绘制 Element；文字入口组合 Label。scene 使用实际布局矩形或明确的 Path 放置同一个区域。

ButtonProps、CheckboxProps 及便捷构造函数提供 focus: FocusOptions。所属 ComponentScope 将稳定 InputArea 登记为 FocusTarget，默认加入 Tab 顺序；外部可传 FocusHandle、tabStop、order 和 visibility。update 同步最新 focus 配置与原有 enabled，外部 handle 变更不重建区域，不复制 Surface 焦点状态。InteractionVisual.focusVisible 消费公共焦点指示；普通指针聚焦不画键盘焦点环，实际焦点与按键接收仍有效。Input／Textarea 沿 TextEditHost 的文本指示策略，局部 InteractionStyles.focused 也只按可见焦点应用。

首次 Component.render 使用未交互外观；onMount 在子 Element 挂载前创建 area／focus／Press，并通过普通 State 发布这组挂载资源。后续 render 读取同一个 Press.interaction；Element.update／scene 借用已就绪资源，不能在首次 render 解引用尚未挂载的 Press。

显式组件接收 `Props<ButtonProps>`，其中 content 是 ButtonContent.Label(LabelProps) 或 ButtonContent.Custom(View) 或 ButtonContent.Resolved((ActionContentStyle) -> View)，另有 onClick、enabled、ButtonStyle 和可选显式主题来源；button 提供 String、StyledText 和显式 View 入口。StyledText 允许行内绑定，Custom 内容由普通布局、图标和文字组合，所有入口共享同一个 Button／Press，不克隆 View 或合并隐藏属性。ButtonStyle 以 StyleOverride／SpanStyle 及可选尺寸／动画输入覆盖明确属性，未指定部分继续使用本轮主题；不再用一组固定默认背景颜色截断主题切换。默认点击区域为整个布局矩形，可根据主题几何与标签组合决定轮廓。

ButtonProps／ContentButtonBuilder 与 CheckboxProps／CheckboxBuilder 增加 invalid=false，受控及自有 Checkbox 入口使用相同字段。它只表示校验结果，仍可点击／聚焦；不会暗改 enabled 或 checked。Button、Checkbox、Input／Textarea 将 invalid 传入公共 InteractionVisual，统一使用主题 validation。选中的 Checkbox 保留 primary 边框，验证环仍显示；显式 marker.validation 或局部 Style 可覆盖。

ButtonStyle 包含 SurfaceKind、SurfaceOptions、可选文字 SpanStyle、InteractionStyles 和动画覆盖。InteractionStyles 的局部覆盖在 enabled 时按 hovered、pressed、focused 的顺序组合；disabled 时只在基本样式上组合 disabled。组合使用 StyleOverride.then，仍继承本轮主题。SurfaceOptions 内的建议／最小尺寸、padding 和边框统一参与布局，实际尺寸取内容需求及主题建议／最小尺寸后再受父约束限制。

Button 查询公共 SurfaceMetrics，根据标签和内缩确定布局，使用 SurfaceGeometry 的内容区域放置标签，并在 ThemePainter 的 Background／Foreground 两阶段间绘制内容。几何与绘制来自同一主题，不在控件中复制表面、焦点、阴影或滤镜实现。公共 Style 可继续装饰完整控件子树，动画采样由控件交给主题绘图。需要将子项输入纳入按钮冒泡路径时，在 onMount 中调用 scope.routeChildren(area) 建立逻辑关系；scene 放置自己的区域并调用 children() 呈现子项。控件外观变化不改变区域身份；单纯换色不重新创建组件或区域。

Button 的 LayoutMetrics 将标签基线加上实际垂直放置偏移，使整个按钮可以和周围文字按基线对齐。没有对应标签基线时返回 None，不根据字号估算另一条基线。

Button 的 measure 使用本帧 Surface 动画采样查询 metrics，并保存由最终 bounds 得到的 SurfaceGeometry 和同次视觉输入，scene 直接消费这份结果；不在 scene 另算一份可能不同的内容内缩。Checkbox 同样在 measure 保存本帧标记几何，scene 放置输入并在 draw 中绘制。采样不读取另一套时钟，普通几何快照无需关闭，输入仍在成功提交后发布。

Button／Checkbox 复用 UI 的公共 Press；行为通过独立登记参与 Surface 的普通传播、默认动作和事件完成通知。父级 preventDefault 可取消本次默认行为而不截断冒泡；Enter／Leave、FocusChanged 和 Cancelled 通过不可取消的 Notification 维护表现与清理。Surface 在默认阶段执行指针聚焦，控件不提前在传播阶段聚焦。onEventFinished 在释放的默认行为被取消时仍清除按下／按键状态，不产生点击。

指针激活采用以下规则：

- 主按钮在有效区域内按下后进入按下序列；拖出暂时取消按下外观，移回可以恢复。
- 在同一有效区域内释放主按钮时触发一次 onClick；区域外释放不触发。
- Cancel、禁用、卸载或窗口失焦结束按下状态，不能合成会触发点击的普通 Up。
- 普通重绘或 Props 更新不产生点击，也不重启已有按下序列。

获得活动键盘焦点时，无额外修饰键的 Enter 在真实按下时激活、Space 在真实释放时激活；自动重复和 synthetic 不产生额外激活。失焦、禁用或卸载取消键盘激活。指针和键盘激活状态须避免同一次交互重复调用回调。

指针进入／离开按 Surface 最近提交的命中路径维护，包含祖先 clip 和遮挡；不能只用本次测量矩形判断释放是否仍有效。键盘释放须匹配先前的物理键，synthetic 释放仅取消。一次激活序列只由指针或键盘其中一方持有；另一输入源不能在原序列中额外激活。控件不消费滚轮或无关按键。

enabled 和最新回调在 Element.update 的逻辑阶段生效。禁用不等待下一帧，隐藏时也立即退出交互；区域几何继续对应最近成功提交的画面。触发时调用当前 Props 中的 onClick，不延长旧闭包的生命周期。

动画是样式实现的可选行为。实现需要颜色、位置或大小过渡时，通过 scope.animation 取得 Surface Animatable；只有实际目标变化才调用 animateTo，普通 Props 刷新不能不断重启动画。measure／scene 读取本帧已采样值，动画完成不自行触发 onClick。

首批控制 hover／press 的 0～1 采样值，交给主题的 InteractionVisual；Checkbox 另有选择进度。主题默认动画参数来自 tokens.motion，StyleValue.Set(None) 可显式关闭过渡。局部状态覆盖是本轮明确的外观值；需要动画化这些自定义值时由应用传入动画后的 Props。禁用立即结束正在进行的交互过渡，完成、暂停、取消、隐藏和卸载继续遵守 Surface 停帧合同。

## 5. Checkbox

Checkbox 标记使用 Theme.checkboxGeometry 和 ThemePainter.drawCheckbox，向主题传入 Off／On／Mixed 及交互视觉数据，布局与点击可组合标签／留白，不维护另一套标记绘图。选中与混合态的选择进度均为 1，未选中为 0；invalid 时选中与混合态保留 primary 边框。

`CheckState` 为 Unchecked／Checked／Indeterminate，分别映射主题 CheckMark.Off／On／Mixed。CheckboxProps.checked 统一为 Binding<CheckState>；Boolean 便利入口通过 getter／setter 映射同一绑定，true 为 Checked、false 为 Unchecked，写回只有 Checked 对应 true，不持有镜像 State。两种受控入口共用 Checkbox、InputArea 和 Press，互换绑定类型保留组件。最小入口为：

```text
fun checkbox(
    checked: Binding<CheckState>,
    configure: CheckboxBuilder.() -> Unit = {}
): View
fun checkbox(
    checked: Binding<Boolean>,
    configure: CheckboxBuilder.() -> Unit = {}
): View
fun checkbox(
    initialChecked: Boolean = false,
    configure: CheckboxBuilder.() -> Unit = {}
): View
```

显式 Checkbox 接收 `Props<CheckboxProps>`，保存 Binding、enabled、CheckboxStyle 和可选显式主题来源。便利函数另可接收该样式；其尺寸、padding、标记颜色、Style 和动画输入都作为明确覆盖，未指定时使用主题 CheckboxMetrics／视觉外观。自有状态重载由独立的内部组件持有初始 State，并复用同一个 Checkbox。

CheckboxStyle 使用 CheckboxOptions 和同一 InteractionStyles，并可明确覆盖外部 padding；默认 padding 来自主题 small spacing。建议／最小标记尺寸与 padding 决定测量，实际可用空间中的标记 bounds 交给主题计算。同一个 InputArea 包含显式留白，不因选择、glow、阴影或主题 painter 替换而重新创建。

外部绑定入口以当前 Binding 的 value 为显示和交互的状态来源。有效激活时读取当前值，按 Unchecked→Checked、Checked→Unchecked、Indeterminate→Checked 写入一次，随后由底层 State 通知读取者；绑定的业务 setter 可以按自身规则接受或拒绝修改，控件不先保存另一个乐观值。外部修改 checked 只更新显示，不合成用户激活。

自有状态入口由持久的内部组件在首次构造时创建 State，initialChecked 只决定首次值；普通父级重建或再次传入初始值不重置它。该组件把自己的 State 作为 Binding 交给相同的 Checkbox 实现，不复制绘制和输入逻辑。两种入口采用不同组件形状，切换外部绑定／自有状态模式时按框架第 5 节重新挂载；同一外部绑定入口更换 Binding 对象则沿通常 Props 更新保留实例。

Checkbox 通过公开 Element 和 InputArea 自绘框、勾选标记及焦点外观。指针切换遵守 Button 的主按钮按下／区域内释放与取消规则；使用 acceptsEnter=false 的公共 Press，获得焦点后 Space 的一次完整按下／释放切换一次，自动重复不重复切换。禁用、失焦、卸载及区域取消结束活动序列，不改变 checked；Enter 不作为本控件的切换入口。enabled 和当前处理器在 Element.update 阶段生效，隐藏或没有 drawable 时同样处理。

例如，同一个状态驱动勾选与另一个控件的 enabled：

```kotlin
import scoop.ui.*
import scoop.ui.component.button
import scoop.ui.component.checkbox
import scoop.ui.component.column

class Form: Component {
    private val checked: State<Boolean> = state(false)

    public override fun render(): View =
        column {
            this.checkbox(this @Form.checked.binding())
            this.button("Clear selection", { this @Form.checked.value = false }) {
                this.enabled = this @Form.checked.value
            }
        }
}
```

Checkbox 在自身 render 中读取绑定，Form 在 render 中读取 checked.value，两者分别订阅同一 State。用户切换后，Form 重建描述并通过 Props 更新 Button.enabled；点击 Button 又更新同一状态。enabled 是派生输入，不另存一份 State，也不把普通赋值当作持续绑定。Checkbox 卸载时仅解除自己的订阅，Form 或 Store 持有的状态继续有效。

## 6. 容器与 Frame

Row／Column／Stack／Frame 全部通过普通 Element 的 MeasureContext 测量和放置子项，通过 SceneContext 装配场景和纯绘图块。框架只执行协议，不识别这些 class，也不内置它们的算法。

Row／Column 使用共同的 LinearProps（items、spacing、alignment、justify、padding、clip），items 为 Array<FlexItem>，children 为由这些内容生成的只读子序列。Flex 使用 FlexProps(axis: FlexAxis, layout: LinearProps)，FlexAxis 为 Horizontal／Vertical，复用同一测量算法。Stack 使用 StackProps（children、Alignment）；其 AxisAlignment 仍为 Start／Center／End，Alignment 为横／纵两轴的组合。所有输入随 Props 更新，不在首次构造时捕获变化子项。

FlexItem(content, options = FlexItemOptions()) 关联一份内容与不可变分配选项；flexItem(content) { ... } 和 LinearBuilder.item(content) { ... } 使用同一个 FlexItemBuilder。普通 add(View) 及控件成员使用默认选项，追加顺序保持。选项为 basis=FlexBasis.Content（或 Fixed(length)）、grow=0、shrink=1、min=0、max=None、align=None；权重及长度有限非负，max 不小于 min。basis 先按 min／max 收敛；None 对齐继承容器，不通过识别某个子控件类型取得分配参数。flex(configure: FlexBuilder.() -> Unit = {}) 的 builder 继承 LinearBuilder 的内容／配置成员，另有 axis=Horizontal。ChildrenBuilder 同时提供 flex 成员。

测量先扣除 padding 和 spacing。Content basis 在父交叉轴限制下取得基础主轴尺寸；Fixed basis 直接参与分配，避免自然尺寸探测。主轴有界时，正剩余按 grow 分配、负剩余按 shrink × 原 basis 分配，达到 min／max 的项冻结，再分配未冻结项；每轮至少冻结一项或完成分配。无界主轴使用内容／固定 basis，不假设无限剩余；Content 未被 min／max 改变时直接保留自然测量，避免嵌套布局反复施加没有分配需求的 tight 约束。有分配需求时按最终紧主轴约束重新测量，每项尺寸和文字换行真实响应可用空间，最后只放置一次。最小尺寸总和大于可用空间时保留 min 并产生明确 overflow，clip=true 才裁剪。空间不足不自动卸载子项；尺寸与坐标溢出按普通参数错误报告。

CrossAlignment 支持 Start／Center／End／Stretch／Baseline；Stretch 按最终交叉轴尺寸作必要的紧约束测量。Baseline 用水平布局的 alphabeticBaseline，缺少时用该项底边，垂直布局不接受 Baseline。MainAlignment 支持 Start／Center／End／SpaceBetween／SpaceAround／SpaceEvenly。spacing 和 padding 均为有限非负值。容器按父约束确定尺寸；有界 grow 项可以填满可用主轴，其余按实际内容收敛。主轴留白由 justify 分配，分配型间距不减小显式 spacing。

Stack／Frame 的 Alignment.horizontal 的 Start／End 为逻辑起终点；Center 不变。两者按有效方向计算实际原点，不改变子内容自身。Frame 另有 logicalPadding，非 None 时解析为物理 Insets 后替代 padding；无覆盖时仍消费主题默认边距。

Stack 对每个子项使用零最小值和同样的可用最大约束，取子项包围尺寸并按父约束收敛，再依对齐放置。绘制顺序与描述顺序一致，输入重叠选择继续由 Surface 的放置顺序决定。

Frame 先扣除 padding 得到子约束，再测量唯一子项，加回 padding 并按父约束确定自身尺寸。固定尺寸在父约束允许范围内生效；对子项的放置由 padding 和对齐决定。超出部分只有显式 clip 才裁剪，不因父子关系自动裁剪。装饰使用公共 Style／StylePainter，背景在子项之前绘制，显式 clip 使用同一个 Canvas 的几何状态；padding 是布局输入，效果不隐式增加它。

FrameProps 保存 content、可选 width／height、可选 padding、Alignment、StyleOverride、可选 SurfaceKind 和显式主题来源。未选表面时它是普通布局与装饰容器：未指定 padding 为零，覆盖基于空 Style 解析，不要求环境主题。选择 SurfaceKind 后，Frame 消费当前主题的 SurfaceMetrics／SurfaceGeometry／ThemePainter：未覆盖的内缩、最小／建议尺寸、背景、圆角、边框、阴影随主题变化；明确 padding 替换主题 padding，边框内缩仍来自最终几何。Frame 不创建输入区域，状态外观由需要交互的组合控件传入或自行绘制。

这两种外观配置更新同一个 Frame 及 Element 的 Props，不增删包装层。主题表面在 Background 与 Foreground 之间通过 StylePainter 绘制内容；普通装饰使用相同 StylePainter 的完整顺序。解除主题表面时清空当前 ThemePainterCache 并释放其资源，后续可再取得 painter；不会在每次切换时向作用域追加所有历史资源。

各容器消费 measureChild 返回的 LayoutMetrics；Frame 把子项基线加上实际 y 偏移后返回。Row／Column／Flex 透传首个有相应基线的直接子项及其实际 y 偏移；水平 Baseline 组使用同一对齐位置。Stack 仍返回 None，不扫描后代。容器无截断策略时仍测量并放置所有子项，不能把父约束溢出当成隐式卸载。

Flex／Row／Column 的位置按本次 MeasureContext.layoutDirection 解析；水平逻辑起点在 RTL 时位于右侧，垂直排列仅翻转交叉轴逻辑起点。声明顺序、子 key 和 Tab 顺序不变，内容自身不镜像。logicalPadding 非 None 时先按方向解析为 Insets，再替代 physical padding；两个字段是不同值类型，赋入 None 恢复 physical padding。

### 6.1 Absolute

AbsoluteProps 包含 children: Array<AbsoluteChild>、可选 width／height、padding: Insets（默认零）及 logicalPadding: LogicalInsets?（默认 None）。AbsoluteChild 保存 View、horizontal／vertical: AbsoluteAxis（默认 Start(0)）和 zIndex: Int = 0；absolute 工厂与 Absolute 组件消费同一 Props。子 View 的 key 及逻辑顺序直接保留，不为 zIndex 重排逻辑子树。

AbsoluteAxis 的 Start(offset, size?)、End(offset, size?)、Center(offset, size?) 从起边、终边或中心放置；Stretch(start, end) 由两端确定 tight 尺寸。offset／start／end 必须有限，允许负值；size 若提供必须有限非负。Start／End 没有 size 时用 max(0, extent - offset) 作最大测量范围，Center 使用内容轴 extent；指定 size 使用 tight 约束，可溢出容器。Stretch 的 extent - start - end 为 tight 尺寸，负值或非有限结果报参数错误。

容器先用显式 width／height 或该轴的有限最大父约束确定自身尺寸，再受完整父约束收敛；无界轴必须提供显式尺寸。子项不反向撑大 Absolute。扣除 padding 得到非负内容范围，每个直接子项测量一次；测量后用实际子项尺寸计算 End／Center 位置，再加 padding 原点。水平 AbsoluteAxis 的 Start／End 及 Stretch 的起终边按当前布局方向解析；纵向不变。logicalPadding 提供时先解析并替代 padding。Absolute 不创建输入区域，不隐式 clip，也不依赖环境主题；需要裁剪和装饰时组合 Frame。尺寸及坐标计算溢出按普通参数错误报告。

同层子项按 zIndex 升序、同值按描述顺序呈现，使用 SceneContext.children(order)；排序在 Props 逻辑更新时完成，绘制与输入使用同一呈现顺序。Tab 与路由继续使用原逻辑挂载顺序。Absolute 不突破祖先裁剪，不代替 portal 或模态屏障。

### 6.2 ScrollArea

ScrollAreaProps／scrollArea 接受一个 content View、可选外部 ScrollState、ScrollAxes（默认 Vertical）、可选 width／height、lineStep（默认 40 个逻辑单位）、ScrollChain（Chain／Contain，默认 Chain）、ScrollbarVisibility（Auto／Always／Hidden，默认 Auto）、ScrollbarOptions、enabled、FocusOptions、StyleOverride（默认空覆盖）、可选 focusOutline: OutlineStyle 和 ThemeSource。没有外部 State 时组件保留自己的实例；替换外部 State 更新同一视口登记，不更换内容实例。

每个滚动轴须有有限的父最大约束或显式尺寸，最终尺寸服从父最小／最大限制。非滚动轴有界时填满，无界时从一次子项测量取得尺寸。内容沿滚动轴采用无界约束，沿已确定的交叉轴采用 tight 约束；每次 measure 回调测量一次内容，按最终测量结果以负 offset 平移呈现。视口对像素与命中使用同一个 clip，整个显式内容树保持挂载。普通滚动只请求位置帧，不把 offset 作为内容 Props 重建；绘制缓存和虚拟化另行接入。

ScrollArea 的 style 配置视口背景、边框、圆角、阴影和内容效果；默认透明、无边框、无圆角。装饰不改变内容约束或增加内边距，边框覆盖视口边缘。圆角裁剪同时作用于正文、滚动条和命中；clipContent 不能取消视口裁剪。focusOutline 覆盖主题焦点环，环在视口裁剪之外绘制。enabled=false 停用输入，不额外降低整棵正文的透明度。

ScrollArea 与 VirtualList 通过 ScrollViewport.present 的内容回调登记实际滚动路径，复用 UI 的焦点显露。行内目标的 FocusHandle、Tab 和组内键盘导航可显露已放置的离屏内容，嵌套视口按从内到外的成功帧顺序处理；滚动条和 Portal 不因逻辑祖先关系而被错误当作内容滚动。

两者另提供 paintCache，使用 UI 的 PaintCachePolicy.None／Tiles，默认 None，经同一 ScrollViewport 传给内容场景。它只优化采用 drawLayer 的有限图层；公共 ParagraphHost 的正文已经使用稳定 PaintKey、真实绘制边界和布局修订，普通 draw 仍直接执行。切换此策略不更换 ScrollState、内容实例、输入作用域或虚拟化策略。VirtualList.retention 与 paintCache 独立，Dispose／KeepAlive 均可搭配 None／Tiles。

Wheel 在视口 InputArea 的默认阶段使用 Surface.consumeScroll(lineStep, apply)，将内容位移取反为 offset，返回实际消费位移；子项到边界后只交接剩余原始单位。Contain 在本层消费后 preventDefault。新 wheel（包括零位移手势开始）先取消旧平滑滚动；系统 phase／momentum 沿用 Surface 路由。控件不另建惯性时钟。

滚动条覆盖视口边缘，不改变内容测量；双轴条在角落各让出另一条的厚度。竖条位于继承 LayoutDirection 的 End（LTR 右、RTL 左），横条位于底部，并在对应侧让出竖条。方向热切换保留内容实例、焦点和物理 offset，不反转正文或隐式请求新的滚动位置；新方向的几何 Accepted 后取消尚未结束的旧条拖动。外观、最小 thumb 长度、轨道和可选箭头位置全部使用 Theme 的 ScrollbarGeometry／ThemePainter。拖动将实际 thumb travel 映射到完整范围，travel 为零时保持位置；点击轨道移动一页，可选箭头移动一个 lineStep。Auto 在零范围时隐藏，Always 画禁用外观，Hidden 不呈现。条的输入几何只在 Accepted 更新；提交后用 InputArea.pointerPosition 刷新子部件 hover，避免容器移动后沿用旧局部坐标。

滚动条 thumb 使用公共 Drag 的零阈值接管，坐标与 grab 比例来自已 Accepted 的 ScrollbarGeometry；track 和箭头仍是普通单次默认动作。条的按下消费默认聚焦，保留已聚焦子编辑器；终结通知和独立事件完成登记清理拖动，preventDefault 拦截释放也不会遗留按下状态。视口默认参与 Tab，焦点环遵守 FocusOptions.visibility；Automatic 的指针焦点不显示，真实键盘和程序聚焦显示，Always／Never 分别固定显示策略，指示变化单独请求重绘。ScrollArea 和 VirtualList 共用此合同。PageUp／PageDown／Home／End 只在事件 target 就是该视口且无修饰键时执行，包含真实 repeat，拒绝 synthetic；子项的文本或列表导航不会被祖先抢走。只有水平轴时，RTL 的 Home／End 对应物理最大／零偏移，PageDown／PageUp 对应向左／向右一页；有垂直轴时继续按自上而下解析。wheel、滚动条、reveal 和 ScrollState.offset 始终使用物理坐标。enabled=false 通过视口作用域停用其子树交互，程序定位仍可更新 State。

### 6.3 VirtualList

VirtualListProps／virtualList 接受 ItemSource、ItemExtent、可选 ScrollState、Orientation（默认 Vertical）、以逻辑长度计的非负 overscan（默认 0），以及 ScrollArea 相同的 width／height、lineStep、chain、scrollbars、scrollbar、enabled、focus 和 theme。另可提供 VirtualListHandle 与 VirtualListAnchor（KeepVisible／FollowEnd，默认 KeepVisible）。单主轴列表的两轴视口均须有界或有显式尺寸，不为测量交叉轴的 intrinsic 大小创建全部条目。

retention 使用 UI 的 ItemRetention，缺省 Dispose，KeepAlive(maxOffscreenItems) 保留有限的离屏原实例；可见／overscan 选择与活动交互项不计入上限。它直接传给 lazyElement，不在控件中维护另一份挂载或回收池。绘制缓存独立，保留实例并不自动缓存像素。

ItemExtent.Fixed(value) 要求有限正值，按 count × value 计算总长，沿主轴和交叉轴使用 tight 约束。ItemExtent.Variable(estimate) 同样要求有限正估计值，沿主轴使用无界约束，交叉轴 tight；实际主轴尺寸可为零，每步仍推进索引。以稀疏的已测量尺寸和前缀和索引定位范围，不为首屏遍历全部 source 或分配 count 个槽。总长与坐标必须有限；尚未测量的尺寸是估计，因此 ScrollState.content／maximum 会随新测量变化。

列表根据 offset／viewport／overscan 准备覆盖范围，每项每帧至多测量一次；视口主轴长度为零时不因 overscan 新建项。extent 配置、方向或交叉轴约束改变会清除旧尺寸索引；source 替换按已知稳定 key 重定位既有尺寸，作为新输入的初始估计，实际进入范围时重新测量。条目自身 State 或主题改变时，本帧所需项沿正常布局重新测量；未实现项没有 View 构建、Component、InputArea 或挂载资源。

布局以本帧 snapshot.offset 为内容原点，在 Double 中计算条目前缀减去原点后的放置位置，并通过 ScrollViewport.present 的 contentOrigin 呈现。完整前缀、范围和阅读锚点仍使用逻辑内容坐标；可见条目的绘制、输入与锚点几何保持在视口附近，一亿项等超长列表不会因图形后端的巨大平移相消丢失局部位置。

水平列表按继承的 LayoutDirection 排列，RTL 的第一项位于逻辑起点即物理右侧，条目绘图不镜像；不足一屏的内容也在逻辑起点对齐。ScrollState.offset 仍是从物理左侧计量的偏移，阅读锚点则使用逻辑排列方向。默认零位置首次呈现从逻辑首项开始；非零构造位置或显式 scrollTo 等请求按物理位置处理。方向热切换按稳定 key 保持阅读位置，重新测量依赖方向的条目，不重新创建同一 key。scrollToItem 的 Start／End 和横向 PageUp／PageDown／Home／End 按逻辑起终点解析，wheel 与滚动条继续使用物理位移。

KeepVisible 记录最近 Accepted 帧中首个可见 key 及项内偏移。前方插入／删除或尺寸变化时，使用当前尺寸索引保持该 key 的阅读位置；key 被删除则使用原索引附近的存活项。项缩小时收敛项内偏移。FollowEnd 仅在最近成功位置已处于末端时保持新末端，否则沿 KeepVisible；初始定位及较新的 scrollTo／wheel／动画意图优先，不能被旧阅读锚点覆盖。Discarded 不替换阅读锚点。

VirtualListHandle 是对一个挂载列表的稳定线程内引用，提供 isAttached、isPending 和 scrollToItem(key, alignment = Nearest)。key 必须显式，未挂载返回 false；true 表示请求已接收，key 在下一布局中解析，不存在时结束请求。请求先使 ScrollState 停止旧动画并登记新的定位意图，再按估计前缀定位并测量目标，以实际尺寸修正；一次布局不重复测量同一项，需要补足范围时请求后续帧，Accepted 后结束。更新的列表请求或普通 ScrollState 定位、handle／state 替换及卸载使旧请求失效。一个 handle 不能同时绑定两个活动列表。

handle 另提供 `focusItem(key, edge = ItemFocusEdge.First)`，通过 UI 的 LazyFocus 按需实现并定位 key，再聚焦行内首／末个可用 Tab 目标。isPending 包括当前滚动定位或聚焦请求。它先停止旧动画与定位，后续具体目标的显露继续复用同一 ScrollState；新 focusItem、普通 FocusHandle 请求、显式滚动、handle／state 替换、禁用及卸载使旧请求失效。未知 key 或没有可用目标结束请求，真实删除在隐藏状态下也取消；已有有效区域焦点不因取消而主动清除。此接口不要求提前取得未挂载项的 FocusHandle。

`VirtualListHandle.reading: VirtualReadingSnapshot?` 返回最近 Accepted 的可观察值，首帧之前及解绑后为 None。`VirtualReadingSnapshot(visible: VirtualRange, anchor: VirtualReadingAnchor?, atEnd: Boolean)` 的 visible 为排除 overscan 的逻辑索引半开区间 `VirtualRange(start, end)`；anchor 为首个实际阅读项的 `VirtualReadingAnchor(key, offset)`，offset 从该项的逻辑起点计量。空数据、零视口或没有实际可见长度时 anchor 为 None，visible 为空。快照不持有 MountedNode、排版或 Canvas；较新请求尚未呈现时仍保留最后画面位置，Discarded 不更新。

`handle.restore(anchor): Boolean` 按稳定 key 和项内偏移恢复阅读位置。key 须显式，offset 须有限且非负；未挂载或当前数据源不存在该 key 时返回 false，不用旧索引替代，也不发起定位。有效请求登记 Programmatic 意图，以 extent 估计定位、按需测量目标及视口、按实际尺寸收敛项内偏移；Accepted 后结束。更新的请求、handle／state 替换、源对象替换或卸载取消旧恢复。已有 KeepVisible 在删除锚点后的相邻项回退只服务自动保持位置，不用于显式 restore。

ScrollArea／VirtualList 的 wheel、滚动条按下／拖动和键盘翻页使用 User 来源，即使到达边界也可观察；普通公开定位和焦点自动显露使用 Programmatic。布局保持锚点、prepend、图片等内容重新测量产生的位移只在 Accepted 后通知 LayoutCorrection，沿用原请求序号。观察由调用方通过 ScrollState 登记并随 scope 关闭，不把聊天、未读或会话跟随策略加入控件。

navigation 使用 UI 的 ItemNavigation，默认 None。Sequential 按数据顺序访问行内各个有效 Tab 目标，跨过行尾才按需准备下一行；`Roving(direction = None, loop = false)` 让列表只占一个条目 Tab 停靠点，方向键和 Home／End 切换实际行目标。导航轴来自 orientation，条目入口顺序来自 focus.order。两种模式均关闭视口自身独立的 Tab 停靠点，保留其显式 FocusHandle 和鼠标／程序聚焦用途；空列表没有虚拟条目停靠点。focus.tabStop 在 None 模式下仍只控制视口自身。键盘导航不选择或激活数据，具体菜单／表格语义由组合控件实现。

列表复用 ScrollArea 的视口登记、滚轮残量、键盘默认动作、主题滚动条和统一 clip，只通过 UI 的公开 LazyElement 协议取得子项。已经实现且具有活动交互的条目额外固定，仍测量、放置并由视口 clip 裁剪；交互结束后在必要的下一布局恢复淘汰。其他离开实现范围的项按 retention 保留或正常卸载，同 key 的存活项在重排／Props 更新中保留原实例。source 替换的删除立即退休，不等待绘制机会。已放置目标复用第 6.2 节的焦点显露；绘制缓存只影响图层像素，不改变这些规则。

### 6.4 Wrap

Wrap 使用 `WrapProps(axis: FlexAxis, layout: LinearProps, lineSpacing=0)`，复用 FlexItem 和 LinearProps；`wrap { ... }` 的 WrapBuilder 继承 LinearBuilder，另有 axis=Horizontal、lineSpacing=0。ChildrenBuilder 提供 wrap 成员。lineSpacing 有限非负。

按内容／固定 basis 和 min／max 得到各项基础尺寸，按声明顺序生成行；加入下一项及 spacing 后超过有限主轴空间时开始新行，单个超大项仍形成一行。每行复用 Flex 的 grow／shrink 分配；无界主轴只有一行。按最终主轴约束重测，按各行实际 cross size／baseline 作 stretch 和对齐，再一次放置。主轴无 grow 时按最长行收敛，grow 可填满有限空间；多行的 cross size 加 lineSpacing 后受父约束约束，保留明确 overflow，clip=true 才裁剪。水平行支持 Baseline，垂直排列拒绝 Baseline。两种轴均按有效布局方向解析物理 x，不镜像内容。

### 6.5 Grid

GridProps 保存 columns／rows: Array<GridTrack>、items: Array<GridItem>、columnGap／rowGap=0、padding=Insets()、logicalPadding=None、clip=false。GridTrack 为 Fixed(length)、Auto、Fraction(weight)；每轴至少一条轨道，固定长度有限非负，fraction 权重有限且大于零。GridItem 包含 content、column／row、columnSpan／rowSpan=1、horizontal／vertical=CrossAlignment.Stretch；位置和 span 必须在轨道范围内，单元格对齐接受 Start／Center／End／Stretch。重叠单元格按声明顺序绘制，不改变挂载和 Tab 顺序。

`grid { ... }` 的 GridBuilder 默认 columns／rows 各为 [Auto]，另提供上述几何字段及 key；add(GridItem) 追加完整配置，item(content) { ... } 与 gridItem(content) { ... } 共享 GridItemBuilder，配置默认坐标为 (0,0)。ChildrenBuilder 提供 grid 成员。描述构造验证轨道、坐标、跨度和间距，不挂载或遍历外部数据。

有限轴先确定 Fixed，按实际供给的子项取得 Auto 需求，再由 Fraction 按权重分配扣除 gap 和 Fixed／Auto 后的非负余量。有限 Fraction 不因文本自然宽度而扩张；跨越 Fraction 的内容按分配后的 span 排版，不反过来撑大该跨度内的 Auto。没有 Fraction 的跨轨需求扣除已知尺寸后在 Auto 中等分；无界轴的 Fraction 作为按权重增长的内容轨道参与。单轨需求先处理，再按跨度递增、同跨度按声明顺序处理；固定轨道不增长。不可满足的 Auto／Fixed 总尺寸保留为显式 overflow。

先确定最终列宽，再用每项实际跨度宽度重新测量高度；获得 Auto 行需求、分配行 Fraction 后施加最终单元格约束并放置。已知有限的行跨度用于初次测量需要有界视口的内容；Auto 轴没有可供虚拟列表查询的全数据自然尺寸，须由应用提供有限轨道。逻辑列起点与单元格横向对齐按 LayoutDirection 解析；行从上到下。

表头和虚拟行共享的列宽是普通不可变 Array<Double>，各自转为 Fixed 轨道消费，不建立全局列登记或扫描／挂载虚拟数据。Grid 不承担排序、筛选、分页、拖列或二维虚拟化。所有布局仅调用 UI 的公开协议，独立 intrinsic 查询不在本轮范围。

### 6.6 范围、相邻尺寸与吸附计算

`RangeSpec(minimum, maximum, step=None, minimumGap=0, reversed=false)` 提供有限区间内的纯值计算。minimum／maximum 有限且有可表示的非负跨度，step 若指定必须有限为正；离散模式用整数步号计算，不持续累加浮点值。步数须能由 Double 精确表示整数；非整步 maximum 不增加一个特殊末端 step。连续模式不量化，退化区间返回唯一值。

`clamp(value)` 返回可用值；`valueAt(position)`／`positionOf(value)` 在归一化 0～1 位置与数值之间转换，reversed 显式反转。`validate(values)` 检查当前有序 thumb、步对齐和 minimumGap 是否可满足；连续间距同时接受恰等于相邻值加减 minimumGap 的 Double 边界，避免边界运算与反向减法的舍入差异拒绝 move 的结果，正间距仍要求值严格递增。`move(values,index,value)` 只替换活动下标，限制在邻居及配置边界内，不排序、不换身份，不改变其他 thumb。配置更新前由调用方验证现有值，非法配置／当前值报告参数错误。

`resizePair(first,second,delta,firstMin=0,firstMax=None,secondMin=0,secondMax=None)` 在两侧 min／max 内限制同一个位移，返回保留总长度的 SplitSizes；现有尺寸和位移必须可表示且满足配置。不会由两侧各自计算导致总量漂移。

`snapTarget(position,velocity,anchors,projectionSeconds=0.15)` 从真实布局提供的有限锚点中，按释放速度投影后选择最近项，返回可选的 SnapTarget(index,position)；空锚点返回 None，同距离按速度方向选择、零速取较小位置。anchors 可保留调用方声明顺序，结果下标对应原数组。结果目标始终是实际有限锚点；函数不持有 Window、不启动动画、不管理条目模型。调用方在布局变化后重新解析锚点，用已有 Animatable／spring 推进，新拖动从当前值接管并取消旧目标。

## 7. 样式与资源归属

ButtonStyle／CheckboxStyle 是本库的局部输入对象，使用公共 StyleOverride、SpanStyle 和明确的尺寸／动画覆盖。公共默认外观、视觉状态数据、表面／Checkbox／Radio／Scrollbar 的绘图和几何合同均属于 scoop.ui.style，三风格实现不依赖具体控件。本库把当前状态转换为主题视觉输入，消费同一几何和绘图，不按 macOS／Material／Fluent 分支。段落内 SpanStyle 的局部继承属于 graphics.text 的内容模型，复用其公共合同。

局部样式通过以下接收者入口构造，configure 同步执行一次；每个结果提供 `copy(configure: 对应Builder.() -> Unit = {})`，以当前值初始化配置并发布独立描述。值和 builder 的构造器内部化，不保留公开长参数构造。

```scoop
buttonStyle(configure: ButtonStyleBuilder.() -> Unit = {}): ButtonStyle
checkboxStyle(configure: CheckboxStyleBuilder.() -> Unit = {}): CheckboxStyle
textFieldStyle(configure: TextFieldStyleBuilder.() -> Unit = {}): TextFieldStyle
interactionStyles(configure: InteractionStylesBuilder.() -> Unit = {}): InteractionStyles
```

| Builder | 全部可写属性与默认值 |
| --- | --- |
| InteractionStylesBuilder | hovered／pressed／focused／disabled: StyleOverride，分别默认 styleOverride() |
| ButtonStyleBuilder | kind: SurfaceKind = Action；surface: SurfaceOptions = surfaceOptions()；text: SpanStyle? = None；states: InteractionStyles = interactionStyles()；transition: StyleValue<AnimationSpec?> = Inherit |
| CheckboxStyleBuilder | marker: CheckboxOptions = checkboxOptions()；padding: Insets? = None；states: InteractionStyles = interactionStyles()；transition／selectionTransition: StyleValue<AnimationSpec?> = Inherit |
| TextFieldStyleBuilder | surface: SurfaceOptions = surfaceOptions()；text／placeholder／description: SpanStyle? = None；states: InteractionStyles = interactionStyles()；selectionColor／inactiveSelectionColor／caretColor／markedColor: Color? = None；caretWidth: Double = 1.0；scrollbar: ScrollbarOptions = scrollbarOptions() |

嵌套配置使用真实同名成员：InteractionStylesBuilder 的四种状态接收 StyleOverrideBuilder；surface／marker／scrollbar 接收对应部件 OptionsBuilder；states 接收 InteractionStylesBuilder；text／placeholder／description 接收 SpanStyleBuilder。这些成员均返回 Unit、基于当前值修改，重复块累积，空块保留已有字段。可选文字覆盖尚为 None 时，文字配置块从全部继承的 spanStyle() 开始；直接给属性赋入完整值表示替换，赋入 None 移除该文字覆盖。

transition／selectionTransition 还提供同名 Unit 设置方法，将同一类型化槽写为 Set(value)。Set(None) 禁用过渡，Inherit 使用当前主题；恢复继承时显式把属性赋为 StyleValue.Inherit。InteractionStyles 的状态组合顺序保持第 4 节合同，构造期间不组合某一帧的状态，也不读取或冻结当前主题。

发布后修改被捕获的 builder 不改变结果；不可变子描述可以共享。回调异常不发布部分结果，不创建字体、painter、输入区域、动画或订阅；持久资源与状态仍在挂载作用域管理。控件现有布局、颜色与文本有效性检查继续在实际消费边界执行。

公共 style 的 `Insets(left, top, right, bottom)` 表达有限非负内边距，`Insets.all(value)` 构造四边相等的值，控件不再复制同名类型。布局操作检查边距、尺寸及求和结果，颜色和绘图参数继续由 graphics 检查。首批交互过渡复用 Surface Animatable<Double> 生成 0～1 采样，主题根据它解析颜色或标记外观；目标相同的普通更新不重新启动动画，显式清除 transition 时直接吸附到当前目标。

此前 ComponentDefaults 的固定视觉默认值方案由 ThemeSource／Theme 取代。应用在实际运行入口绑定稳定 ThemeSource；控件挂载保存来源，在 render 读取当前 Theme，其余局部属性继续继承主题。显式来源可经 Props 提供，组合控件向子标签转交；不存在自动挂载树 Provider。切换风格、明暗或效果保持组件实例、State、区域和焦点，只更新 Props、必要布局及绘图资源。

控件或其 Element 通过 ComponentScope.resource 登记自己创建的资源，或登记拥有可替换资源的缓存对象；输入区域和动画通过作用域的对应入口取得。卸载沿框架的子先父顺序清理，应用不为 Label／Button／Checkbox 逐个 use。借用的 Image／Path／字体、Binding 与外部模型不得由控件擅自关闭。

Element 构造、update、measure、paint 或回调失败沿框架与 Surface 的统一关闭路径传播。缓存创建及替换失败不得遗留新取得的资源；清理错误不能阻断其余对象收尾。控件库不另建窗口状态机、异常恢复树或动画时钟。

## 8. 验收要求

普通行为测试放在本 Cone 的 scoop.test，至少覆盖：

1. Label 使用实际 graphics.text／graphics 及公共 ParagraphHost 完成字体混排、继承、换行、尺寸与绘制；输入变化替换缓存，位置／透明度变化及重复测量复用缓存。
2. Button 指针按下、拖出／返回、区域外释放、取消、禁用、失焦，以及 Enter／Space 自动重复和激活取消。
3. Props 改变后使用最新文字、enabled 和回调；普通更新保留状态，key 改变后创建新实例。
4. Row／Column／Stack／Frame 的有限／无界约束、子项顺序、对齐、padding、显式 clip 及嵌套组合。
5. Counter 连续交互、两个独立实例、条件移除、keyed 列表重排，以及隐藏期间禁用和资源卸载。
6. 实际 Surface 动画驱动外观或布局，目标更新不重建实例，卸载后停止业务完成回调。
7. 自建／借用资源、缓存替换失败、回调内关闭、清理异常和真实窗口关闭。
8. 同一 Label 中的不同字号文字、行内图片和 Button：基线经 Button／Frame 正确传递，RTL 位置、自然／固定尺寸、窗口换行、截断及恢复后保持组件状态。
9. 行内按钮被截断后取消输入／清除焦点，失败帧保留已提交几何；按钮内容、图片尺寸与实际布局动画变化触发重排，缓存替换不关闭子组件。
10. Checkbox 的指针／Space 切换、自动重复抑制、禁用／失焦／取消；自有状态的初始值、重建保留和重新挂载重置。
11. 外部 Binding 与 Checkbox 自绘同步、Form 的 enabled 派生输入、反向更新同一状态、卸载不重置共享值；更换 Binding 后读写新目标并移除旧依赖，切换状态归属模式按组件形状重新挂载。

12. 控件消费公共 Theme／ThemePainter 的几何与绘图，三风格与明暗切换时未覆盖属性继续跟随主题；组合公共 Style 的外框装饰、内容效果与状态外观；样式切换保持内部 State、基线和点击区域，参数动画完成／暂停／取消后下层帧需求停止。

控件库必须能作为独立 Cone 使用已编译的公开框架接口构建和测试；独立自绘示例继续只依赖框架。类型、可见性及链接规则使用现有编译器与 Cone 合同，不为本库引入专用编译规则或重复每个控件的通用负例。

## 9. Input／Textarea

InputProps／Input／input 和 TextareaProps／Textarea／textarea 是普通 Component 入口，必须显式接收 TextEditorState。它是唯一可写正文与选区；控件不保存 String binding 副本，onEdited 交付已生效的模型，外部 replaceAll 不回放用户编辑通知。placeholder、enabled、readOnly、invalid、description、fonts、style、theme、focus、direction、alignment、paintCache、caretBlink、policy、projection、copy、labelTarget、group 和当前回调均经 Props 更新；标签关联遵守第 15 节，连接位置遵守第 16 节。direction 为 TextDirection?，None 继承挂载方向，显式值独立于布局方向。默认 caretBlink 为 500ms，零表示保持可见，paintCache 缺省 None。卸载保留外部模型和借用字体。

InputProps／TextareaProps 和对应 builder 的 policy 默认为 textEditPolicy()，直接交给同一 TextEditHost；输入、粘贴、删除及组合最终提交在发布前按 UI 规范第 7.5.1 节处理。onEdited 只观察最终结果，不用作过滤或光标修复；控件不复制策略或历史实现。

两者增加 `chrome: TextFieldChrome = Framed`，可选 Bare。Framed 保持完整表面；Bare 是实际组合使用的内容模式：沿相同 TextFieldInk／TextEditHost 保留正文、占位、滚动条、输入与描述，去掉外框绘制、边框内缩、圆角、状态环及控件禁用合成层，padding 和文字样式仍有效。主题建议／最小尺寸扣除原边框占用，避免组合后重复增加边框。改变 chrome 不重建宿主或文本连接；调用方负责外层状态外观。Textarea 未显式指定 padding 时使用按 body 字号／14 缩放的横向 10、纵向 8，Input 保持主题内缩。

projection 默认为 textProjection()，copy 默认为 TextCopyPolicy.Default；Props／builder 直接传给宿主。内置遮罩默认禁用 Copy／Cut，明确 Allow 时复制原文；切换显隐保留模型、历史与 IME 连接，控件不创建另一份显示文档。

Input 固定单行，Tab 导航，无修饰 Enter 调用 onSubmit。Textarea 默认四行、wrap=true、tab=Navigate、submit=PrimaryEnter；rows 为正整数，可明确选择 Insert Tab 或其他 TextSubmitKeys。编辑、Unicode 导航、组合、剪贴板、只读和候选框均委托同一 TextEditHost；不在组件内再映射原生按键或排版文字。

组件 render 读取 model.inputState，并观察实际已挂载宿主的 interaction。内部只保存一个可观察的宿主挂载引用；Element 创建时登记，退休时解除对应实例，不复制 focus／hover 或另建文本订阅器。Host、字体、段落度量缓存及主题 painter 由 ComponentScope 管理；未传 fonts 时创建一次系统 fallback 字体集合，更换外部来源不关闭借用资源。

TextFieldStyle 包含 SurfaceOptions、正文／placeholder／description 的 SpanStyle、InteractionStyles、可选选择／非活动选择／caret／marked 颜色、caretWidth 和 ScrollbarOptions。表面使用 Theme 的 Plain 几何和 Background／Foreground 绘制阶段，padding、边框内缩及显式内容效果沿公共 Style 处理。默认选择取强调色的半透明值，caret／marked 取强调色。invalid 仅改变外观，正文合法性由业务决定：默认错误边框和状态环来自 tokens.validation，即使未聚焦也显示；description 使用 palette.destructive.background。明确的局部表面与 validation 覆盖仍优先，focus 只覆盖正常焦点外观，禁用说明使用 disabledText。没有内置验证器或提交阻断。正文区域（背景、文字、边框和状态环）整体应用 SurfaceGeometry.opacity，description 在此层之外。placeholder 始终使用 muted.foreground，再随正文区域合成；局部 placeholder 覆盖仍优先。

有界宽度占满父约束，无界宽度使用显式 SurfaceOptions.suggestedSize.width，缺省 240，再满足主题 minimumSize。Input 一行、Textarea rows 行的建议高度使用同一字体与正文样式下真实空段落的行高，合并 padding、边框和主题建议／最小高度，再按父约束收敛；不以字符数估算布局。Host 每帧只按最终内视口测量一次，基线来自该布局并加实际内容偏移。description 作为普通 Label 在边框外下方按同宽约束测量，不作为输入正文或滚动内容。

Textarea 的 scrollbars 默认为 Auto，可选 Always／Hidden。非 Hidden 固定预留主题垂直条宽；wrap=false 时另预留水平条高，避免文字溢出改变滚动条后再反复测量。Auto 只在实际范围非零时显示，Always 显示禁用的零范围外观，Hidden 不预留。复用已有 ScrollBarControl 与宿主本帧 scrollSnapshot，只有宿主持有 ScrollViewport。条的滚动和输入沿既有残量、捕获及成功几何规则；Input 只水平显露，不绘制滚动条。

外框留白使用不参与 Tab 的区域，在正文之前放置；点击留白把焦点交给同一宿主，不建立第二个文本连接。正文区域仍执行普通定位和选择。主题／明暗、描述、只读或模型替换更新保留控件挂载；编辑模型替换按 TextEditHost 的旧组合／连接收尾规则生效。模态、虚拟项固定、外层显露和绘制缓存只沿框架既有能力组合。

验收覆盖实际 Input／Textarea 的外部更新、模型／字体更换、说明与主题状态、正常和无效表面、只读／禁用、留白聚焦、Tab／提交、长文滚动条，以及模态 IME 取消和虚拟编辑项保留。原生窗口行为使用 scoop.test，与纯编辑事务和主题像素用例分层验证。

## 10. Image 与 Path 展示

`imageView(image, naturalSize, configure)` 和 `pathView(path, viewBox, configure)` 返回普通 Element 描述，对应 ImageViewProps／ImageView 与 PathViewProps／PathView；ChildrenBuilder 提供同名的追加成员。构造只保存输入，不访问文件、网络、挂载或取得资源所有权。

Image 的 naturalSize 是明确的正有限逻辑尺寸，像素密度由像素尺寸与该值的关系给出。Path 的 viewBox 是有正有限尺寸的图形坐标矩形，可额外指定 naturalSize，缺省使用 viewBox.size。测量以自然宽高比在父最大约束内收缩，再服从父最小／紧约束；不会按根 DPI 再乘一次尺寸。

两种 builder 均提供 fit: ContentFit=Contain、alignment: Alignment=Center／Center、clip=true 和 key；Image 另有 sampling=Linear、opacity=1，Path 另有 naturalSize=None 和 paint=Paint()。fitContent(naturalSize, bounds, fit, alignment, direction) 公开同一纯几何算法：Contain 等比包含，Cover 等比覆盖，Fill 使用完整目标矩形，None 保留自然尺寸。零宽／高目标返回空目标；对齐的水平 Start／End 依 LayoutDirection 解析，图片和路径本身不镜像。clip=true 时只裁到布局框，false 允许明确的内容溢出。

Path 按 viewBox 原点和平移／缩放映射到目标矩形，stroke 与路径一起变换。Image 使用现有 graphics 绘图与采样合同。两者借用输入资源，更新或卸载不关闭调用方的 Image／Path；持有者必须在使用期间保持它们打开，关闭资源的错误沿普通绘图失败路径处理。替换内容通过 Props，绘制读取本轮输入，不跨更新保留旧资源缓存。展示本身不创建 InputArea、焦点、计时器或加载状态；加载／失败／重试属于应用或后续控件组合。


## 11. 操作变体、Badge 与 Spinner

ButtonProps／ContentButtonBuilder 增加 variant: ButtonVariant = Default、size: ButtonSize = Default。ButtonVariant 为 Default／Outline／Secondary／Ghost／Destructive／Link；ButtonSize 为 Xs／Sm／Default／Lg／IconXs／IconSm／Icon／IconLg。ButtonStyle.kind 继续选择主题基础表面的尺寸与外观，variant／size 在其上解析，随后依次应用显式 surface／text 及适用的 states；明确的局部覆盖优先。状态解析只消费公共 token、mode 和几何，不判断主题实现类型。

Default 保留所选主题部件；Outline 使用背景与边框、hover 使用 muted（Dark 为 input 的 0.3／0.5 alpha）；Secondary 使用 secondary，Button hover 以 OKLCH 混入 5% foreground；Ghost 无底色、hover 使用 muted（Dark alpha 0.5）；Destructive 使用 destructive 的 0.1／0.2 alpha 底色及同色文字，Dark 为 0.2／0.3；Link 无底色与可见边框，使用 primary 文字，hover 时给文字入口加下划线。Destructive 的普通焦点边框 alpha 0.4，环 Light／Dark alpha 0.2／0.4，宽度与外扩沿主题 validation。invalid 优先于普通焦点，显式局部外框／状态环覆盖仍有效。无整体禁用衰减（disabledOpacity 截取后为 1）的主题，新增非 Default 变体使用 disabledText，并将底色与 muted 等量混合；采用整体衰减的主题保留未衰减颜色，避免重复衰减。颜色过渡沿现有 hover／press 采样并使用主题的预乘 alpha 插值；Outline／Secondary／Ghost 的 pressed 在 hover 背景上混入 8% background.foreground，Destructive 的 pressed alpha 为 Light 0.3／Dark 0.4，Link 保持透明。按下与悬停使用不同终点，松开后返回当前悬停或普通状态。Secondary 的 OKLCH 色彩混合只服务此处 hover 预设，不引入 CSS 解析器。

以 Nova 32 高表面为基准，Xs／Sm／Default／Lg 建议高 24／28／32／36，对应文字字号 12／12.8／14／14，图标尺寸 12／14／16／16，内容间距 4／4／6／6；其他主题按基础建议高和操作字号等比例派生。Icon 四尺寸取对应边长，图标为 12／16／16／16，无文字专用水平留白；正常父约束、内容最小需求及显式局部尺寸仍优先，不能裁掉放大后的字体。默认保留主题圆角，Xs／Sm 使用 medium 圆角；局部 corners 可覆盖。禁用清除交互过渡，但不清除 invalid。

新增 `button(content: (ActionContentStyle) -> View, onClick, configure: ContentButtonBuilder.() -> Unit = {})` 及同名 ChildrenBuilder 成员。ActionContentStyle 是完整不可变值，包含 text: TextStyle、iconSize: Double、spacing: Double、interaction: InteractionVisual、underline: Boolean；回调只在组件 render 中构造内容，接收当前主题、变体、尺寸及局部文字覆盖的结果。它不注入子树 Context，不克隆 View。既有任意 View 与 StyledText 保留自身样式；需要自动适配的图标／Spinner 使用此入口显式消费尺寸和颜色。

LabelProps／LabelBuilder 增加 underline: Color? = None。它是纯绘制装饰，使用 ParagraphHost 的实际文字选择区域在每行底部绘制 1 单位线条，既不改正文／测量也不添加输入。Button 的 Link 文字入口在 hover 时提供该装饰；显式 Label 下划线优先。

### 11.1 Badge

`badge(text: String, configure: BadgeBuilder.() -> Unit = {})`、StyledText 对应 StyledBadgeBuilder、View 与 `(ActionContentStyle) -> View` 对应 ContentBadgeBuilder；ChildrenBuilder 有同名真实成员。BadgeProps／Badge 是普通组件，content 复用 ButtonContent，onClick: (() -> Unit)? = None、variant=Default、enabled=true、invalid=false、style: ButtonStyle=buttonStyle()、theme=None、focus=FocusOptions()。builder 另有 key；文字 builder 有 fonts，Styled 另有 settings／inlines。

六种变体与 Button 同名；Badge 为 20 建议高、12／16 文字、12 图标、4 间距、8 水平与 1 垂直内缩、胶囊圆角，按主题操作字号缩放并受真实内容和父约束限制。Secondary 的动作 hover 使用 secondary 背景 alpha 0.8；Outline 明暗均使用 border 边框，Outline／Ghost 的动作 hover 文字使用 muted.foreground；Destructive 动作 hover 使用 destructive alpha 0.2／0.3（Light／Dark）。pressed 反馈和局部覆盖顺序与 Button 相同。

无 onClick 时为被动展示，不建立 Press 或 Tab 目标，不拦截内容中的独立动作；自身不产生 hover／pressed 状态。有动作时复用同一 Press 和焦点规则，读取最新回调，一次激活只执行一次。onClick 有无变化按普通子树替换规则建立／撤销行为；disabled／卸载取消未完成序列。Link 只提供样式，不自动打开 URL。

### 11.2 Spinner

`spinner(configure: SpinnerBuilder.() -> Unit = {})` 与 ChildrenBuilder.spinner 返回普通 SpinnerProps／Spinner 组件。字段为 size: Double? = None、color: Color? = None、running: Boolean = true、period: Duration = Duration.seconds(1L)、theme: ThemeSource? = None；builder 另有 key。size 为有限非负逻辑尺寸，None 使用操作字号的 16／14；color=None 使用当前操作文字颜色。尺寸为零合法且不请求动画帧，period 必须大于 ZERO。

图形采用实际 Lucide loader-circle 路径，保存来源与许可证；路径由挂载作用域持有，不逐帧分配。rotation 使用 scope.animation、线性 Tween 与 Repeat.Restart，running／period／减少动态变化才调整动画，普通刷新不重启。停止或 Reduced 显示静态图形且不请求帧。Spinner 在成功帧后用自身图形范围与 Canvas.localClipBounds 的保守交集设置该 Animatable.isFrameEnabled，实际零尺寸或透明图形也不请求帧；滚回视口沿同一时间轴恢复，不扫描离屏子树或建立定时器。虚拟保留、窗口隐藏和卸载继续沿已有动画组与 Surface 参与规则处理；局部开关不越过所属组的资格。它没有计时线程、后台任务或全局状态；父控件通过 ActionContentStyle 显式传入大小与颜色。

## 12. 内容面板、分隔线与比例布局

### 12.1 类型化内容与公共组合

Card、Alert、EmptyState 的必需 body 是 View。标题／说明使用 `PanelContent.Label(LabelProps)／Custom(View)`，可分别提供纯文字、StyledText 或完整 View；普通文字接受根的字体与当前角色样式，StyledText 和显式 baseStyle 保留自身样式，Custom 不被检查或修改。三个 builder 的 title／description 成员各有 String、StyledText、View 重载，直接赋入 None 撤销该可选区；fonts、theme、style、key 的约定与现有控件一致。PanelBuilder 仅复用这些实际消费者及第 17 节 Item 的字段和成员，不建立插槽注册表或挂载树上下文。

无内容的可选区域不产生节点、间距或边框。各区域有稳定的局部 key；移除 header／description／action 等相邻区域，或窄宽换行，不重新挂载仍存在的 body 与其状态。header、action、footer、media 等 View 都是调用方内容，既不克隆，也不偷偷增加输入行为；root 的显式 theme 只用于自身表面与便利文字，任意子 View 保留自己的主题选择。构造和配置只发布描述，字体、Path 和 painter 的所有权仍沿第 7 节。

Card 头部及 Alert 的正文／动作布局在有限宽度下优先并排：动作使用自然宽度，正文取得余量；余量不足当前正文字号的 8 倍时，动作移到下一行的逻辑末端。无界宽度按两者自然尺寸排列。两种布局均使用同一子树、方向和最终测量，不以窗口宽度猜测 CSS breakpoint，也不裁掉长动作。

### 12.2 Card

`card(content: View, configure: CardBuilder.() -> Unit = {})` 创建 CardProps／Card。除公共 title／description、fonts／theme／style 外，字段为 header／action／footer: View? = None、size: CardSize = Default、headerSeparator: Boolean = false；CardSize 为 Default／Sm。builder 的 header／action／footer(View) 写入对应字段；同名 ChildrenBuilder.card 追加一次。

Nova 下 Default／Sm 的区域间距和内缩分别为 16／12，随 body 字号相对 14 缩放。header 中自定义内容、标题和说明按 4 间距排列，action 位于逻辑末端；标题 Default 为 16／22、Sm 为 14／19.25，使用操作字重，说明为 body 与 muted.foreground。body 独立左右内缩；footer 存在时使用同样内缩、顶边分隔线和 muted 背景 alpha 0.5，根不再重复加尾部内缩。headerSeparator 仅在实际 header 存在时添加底边与对应留白。

表面使用 card 配色、large 圆角、foreground alpha 0.1 的 1 单位边线及内容裁剪。style 最后覆盖。header、body、footer 的可组合边界由明确的 View 槽表达，不为每一层无行为的容器另建持久组件。

### 12.3 Alert

`alert(content: View, configure: AlertBuilder.() -> Unit = {})` 创建 AlertProps／Alert。额外字段为 variant: AlertVariant = Default、icon／action: View? = None；AlertVariant 为 Default／Destructive。icon／action(View) 为真实 builder 成员。

表面为 card 背景、border 边框和 medium 圆角，水平／垂直内缩为 10／8；图标在逻辑起端，与正文间隔 8、向下偏移 2，正文标题、说明和 body 之间间距 2。标题使用操作字重；Default 的说明使用 muted.foreground，Destructive 的标题使用 destructive.background、说明使用其 alpha 0.9。尺寸随 body 字号缩放，动作窄宽行为见 12.1。任意内容保持自身文字或图标颜色，便利文字随 variant 更新。Alert 是静态内容，不创建焦点、计时器、关闭动作或弹层。

### 12.4 EmptyState

`emptyState(content: View, configure: EmptyStateBuilder.() -> Unit = {})` 创建 EmptyStateProps／EmptyState，保留 UI.empty() 的原含义。额外字段为 header／media: View? = None、mediaVariant: EmptyMediaVariant = Default、outlined: Boolean = false；EmptyMediaVariant 为 Default／Icon。header／media(View) 写入对应字段。

根使用 large 圆角、24 内缩和 16 区域间距；有限父宽下占满可用宽度，无界时使用自然宽度。header 中 media、自定义 header、title、description 按 8 间距排列，media 后另留 8，标题为 14 的操作文字，说明为 14／22.75 的 muted 文字。header 和 body 分别限制为 384 建议宽并在可用宽度中居中；父宽更小时正常收缩换行，不强制最小 384。Icon media 使用 32 方形、medium 圆角和 muted 背景，Default 保留内容自然尺寸。这些尺寸按 body 字号相对 14 缩放。

outlined 提供 1 单位、3 开／3 间隔的圆角虚线，默认关闭；显式 style.border 覆盖该预设，可改实线或清除边框，style.background 可设置背景。虚线由已有 Path／PathMeasure 生成有限段并按最终尺寸／圆角缓存，只在几何变化时重建，卸载关闭；不引入绘制后端或周期任务。面板自身不参与输入，body 中按钮、文本选择或表单保持原行为。

### 12.5 Separator

`separator(configure: SeparatorBuilder.() -> Unit = {})` 创建 SeparatorProps／Separator，字段为 axis: FlexAxis = Horizontal、length: Double? = None、thickness: Double = 1、color: Color? = None、theme: ThemeSource? = None；builder 另有 key。length／thickness 必须有限非负，零表示空线。默认颜色使用当前 border。

length=None 使用父约束该轴的有限上限；无界时使用该轴下限，不虚构自然长度。父约束决定最终外框，线条保持指定 thickness 并在交叉轴居中；父交叉尺寸不足时收窄到可用值。只绘制逻辑坐标中的矩形，由 Canvas 应用一次 DPI，不创建 InputArea。

### 12.6 AspectRatio

`aspectRatio(content: View, ratio: Double, configure: AspectRatioBuilder.() -> Unit = {})` 创建 AspectRatioProps／AspectRatio；ratio 必须有限且严格大于零，builder 字段为 clip: Boolean = true、key。ratio 为宽／高。

有有限最大宽时优先用该宽推算高度，再受有限最大高限制；只有有限最大高时反向推算宽度。两轴无界时用内容自然尺寸求能包含它的比例外框。随后在不越过最大值的前提下满足可兼容的最小值；比例与父约束不可同时满足时，父约束优先，不报告比例配置错误。计算出的实际尺寸必须有限；不可表示的几何正常报参数错误。

实际子项最后按外框紧约束测量并放在原点；空尺寸合法。clip=true 裁剪同一外框，clip=false 允许内容自身的显式溢出。图片继续使用 ImageView 的 Contain／Cover，AspectRatio 不复制图像适配、读取像素或预乘 DPI。

## 13. 头像与加载状态

### 13.1 Avatar 与头像组

`avatar(fallback, configure: AvatarBuilder.() -> Unit = {})` 的 fallback 接受 String、StyledText 或 View，按 PanelContent 保存。AvatarProps 的其余输入为 image: AvatarImageState = Empty、size: AvatarSize = Default、badge: AvatarBadge = None、fonts／theme、style；builder 另有 key。AvatarSize 为 Sm／Default／Lg，对应 Nova 24／32／40，按 body 字号相对 14 缩放；回退文字分别为 12／14／14，使用 muted 前景，普通文字居中。

AvatarImageState 为 Empty／Loading／Failed／Ready(Image)。前三种状态都显示当前 fallback，Ready 使用同一 ImageView 的 Cover；资源由调用方装载并拥有，状态和图片替换不关闭借用 Image，不创建下载任务、重试或定时器。已关闭的 Ready 图片沿 ImageView 的普通资源错误处理，不偷偷改写业务状态。默认背景仅在 fallback 时使用 muted，外框为圆形裁剪和 1 单位 border；默认边框在 Light／Dark 下分别用 Darken／Lighten 混合，显式 style.border 覆盖或清除它，其他局部装饰也继续有效。父约束决定最终外框，圆角使用实际短边的一半。

AvatarBadge 为 None／Dot(Color?)／Content(View, Color?)。builder 的 badge(color: Color? = None) 和 badge(content: View, color: Color? = None) 写入对应值。标记位于逻辑末端底部，Sm／Default／Lg 的边长为 8／10／12，使用 primary 背景和 2 单位 background 外环；Sm 仅显示圆点，Content 的 View 不呈现。它没有额外输入行为，调用方内容也不被克隆或增加动作。

`avatarGroup(size: AvatarSize = Default, configure: AvatarGroupBuilder.() -> Unit)` 收集 avatar 的三个 fallback 重载和 count(Long)／count(View) 成员；size 是条目配置的初始尺寸，显式条目 size 可覆盖。count 的数值必须非负，默认文字为 `+N`，使用组 size 与 muted 颜色。组不推断缺失人数、不截断数据。每项使用稳定 key，后项绘制在前项之上；默认重叠 8，方向只改变逻辑排列。overlap 可配置为有限非负逻辑长度，每个接缝最多重叠相邻较小项的宽度，位置不倒退。组在每项外绘制 2 单位 background 环，布局保留最外侧环的 2 单位空间；空组尺寸为零。组和条目均不增加选择或 roving 导航语义。

### 13.2 Skeleton

`skeleton(configure: SkeletonBuilder.() -> Unit = {})` 和 `skeleton(content: View, configure: SkeletonBuilder.() -> Unit = {})` 创建同一个 SkeletonProps／Skeleton。字段为 content、loading: Boolean = true、animated: Boolean = true、shape: SkeletonShape = Rectangle、width／height: Double?、period: Duration = 2 秒、color: Color?、theme；builder 另有 key。无内容入口以 empty() 为正文并默认 height=16；显式内容入口默认 height=None。width 默认 None，两入口可显式覆盖；所有明确尺寸必须有限非负，period 必须严格为正。

width=None 使用有限父宽，否则使用内容自然宽度；height=None 使用实际内容高度。内容在两种 loading 状态下均按相同约束测量；loading=true 时不放置或呈现内容，只画占位，内容不能命中或保持活动焦点。loading=false 时放置原内容并停止占位动画，保留该子树及业务状态，不在切换时重建正文。Rectangle 使用 small 圆角和整个外框；Circle 在外框中居中绘制短边为直径的圆。默认颜色在 muted 背景中混入 12% background.foreground，使明暗主题的脉冲都具有可见对比度；显式 color 保留原值。

默认脉冲周期为 2 秒，透明度从 1 到 0.5 再回到 1，两个半周期使用 cubic-bezier(0.4, 0, 0.6, 1)。animated=false、loading=false、Reduced、透明、零尺寸或不可见时不留下循环帧需求；静态占位使用完整透明度。

### 13.3 Progress

`progress(value: Double?, configure: ProgressBuilder.() -> Unit = {})` 创建 ProgressProps／Progress；另有 Double 便利重载。None 表示未知进度，Some 表示确定值；min／max 默认为 0／100，与 [Base UI Progress](https://base-ui.com/react/components/progress) 的值模型一致。minimum／maximum 必须有限、差值有限且 maximum>minimum；确定值必须有限，显示位置通过现有 RangeSpec 钳制到 0～1，不反写调用方的值。控件不创建任务、轮询或完成回调。

builder 提供 minimum／maximum、label／valueLabel: PanelContent?、showValue: Boolean = false、thickness: Double?、indicatorColor: Color?、style: StyleOverride、transition: StyleValue<AnimationSpec?> = Inherit、period: Duration = 2 秒、fonts／theme／key。label／valueLabel 有 String、StyledText、View 成员；valueLabel 优先于 showValue。showValue 生成取最近整数的百分比，未知进度显示省略号；调用方可用自定义文字承担本地化和业务格式。

可选 label／value 在轨道上方，分别使用 action 与 muted body 文字，位于逻辑起端／末端；窄宽时允许换行，不叠字，区域间距为 12。轨道填满有限宽度，无界时自然宽度为零，默认高 4、muted 背景、圆角为半高，指示使用 primary。尺寸按 body 字号缩放，明确 thickness 必须有限非负。style 仅覆盖轨道装饰；指示始终裁剪到同一轨道几何，RTL 从右侧增长，不镜像图像或文字。

确定值首次挂载直接显示当前值，后续更新默认使用主题 selection 动画；transition 可显式替换或禁用，并最终服从 Reduced。未知进度使用占轨道 30% 的移动段，以 period 沿逻辑方向穿过轨道；Reduced 下在轨道中央静态显示该段。确定／未知切换停止前一种动画，最新 Props 决定当前目标，不创建另一份业务进度状态。停止、隐藏、离屏和卸载的帧资格遵守 13.4。

### 13.4 可见动画与资源

Spinner、Skeleton 与 Progress 复用 scope-owned Animatable 的循环和可见性处理。成功帧按图形范围与当前 Canvas clip 的保守交集设置既有 isFrameEnabled；未知 Progress 使用整个轨道作为可见范围，避免移动段暂时出界后永远停住。窗口隐藏、未呈现子树和卸载继续由已有 Surface／ComponentScope 处理。失败帧不发布新的可见资格，离屏暂停不改写原时间轴；恢复后沿原时钟求值。控件不增加全局动画管理器、独立 timer 或新的调度入口。

## 14. Typography 与快捷键提示

### 14.1 文字角色与段落

`typography(text: String, role: TypographyRole = P, configure)`、StyledText 重载及 `typography(role: TypographyRole = P, configure: TypographyTextBuilder.() -> Unit)` 创建 TypographyProps／Typography。角色为 H1／H2／H3／H4／P／Blockquote／InlineCode／Lead／Large／Small／Muted。当前设计采用 [Typography 的明确角色](https://v3.shadcn.com/docs/components/typography)，不引入 Typeset、Markdown 解析或代码块。

默认尺寸按主题 body 字号相对 14 缩放，使用其字体族和当前语义前景：H1 为 36／40、800，H2 为 30／36、600，H3 为 24／32、600，H4 为 20／28、600；标题字距为字号的 -0.025。P 为 16／28、400；Blockquote 为 16／24、斜体；InlineCode 为 14／20、600、monospace；Lead 为 20／28、muted 前景；Large 为 18／28、600；Small 为 14／14、500；Muted 为 14／20、muted 前景。没有浏览器视口断点，H1 的更大字号由显式文字覆盖表达。段落外间距由父布局决定，不推断相邻节点或 CSS margin collapse。

H2 带底边 1 和底部留白 8；Blockquote 带逻辑起端边 2 和留白 24；InlineCode 使用 muted 背景、small 圆角、水平 4.8／垂直 3.2 留白。尺寸随正文密度缩放，H2 的细线保持 1；所有装饰可用 style 覆盖。InlineCode 先取自然宽，再在父约束内换行／收缩，不占满段落为行内对象提供的最大宽度；基线包含实际垂直留白。普通文字使用角色基础样式；StyledText 保留原基础样式与片段，显式 textStyle: TextStyle 覆盖最终基础样式，不重写嵌套 span。

builder 公共字段为 fonts／theme、textStyle、style、settings、selectable: Boolean = true、selection: SelectableTextOptions、focus、actions／decorations、onSelectionChanged、inlines、key；settings／textStyle／style／focus 提供对应接收者配置。选择模式直接使用 SelectableStyledText，非选择模式使用 Label。文字更新、方向、主题与局部样式更新沿现有文本宿主保留合法选区；没有额外编辑文档或剪贴板实现。选择以各个实际段落为边界，不建立跨段落文档模型。

TypographyTextBuilder 另有 text、add(TextSpan)、span、code(String, InlineKey)、inline(InlineKey, View, alternateText, configure)。code 创建带真实大小、基线和圆角背景的非选择 InlineCode 子 View，作为原段落内的一个行内对象参与排版／选择，复制时使用原代码文字；不会给子 View 再创建选择宿主。明确 key 保持行内对象身份，默认 code key 按声明位置生成；重复 key 沿 StyledText 规则报错。其他行内 View 也通过现有 InlineView 测量、命中和替代文字合同处理。range action 的位置始终是 StyledText 的 UTF-8 字节位置，行内对象占 U+FFFC 的三个字节。

### 14.2 列表与文字表格

`typographyList(configure: TypographyListBuilder.() -> Unit)` 收集 `item(String／StyledText／View, key = Position)`。普通项使用 P，可选择和换行，圆点使用同一正文基线；内容起端缩进 24、项间距 8，随密度缩放。View 可组合嵌套列表或其他内容，容器不检查其内部结构。builder 提供 fonts／theme／key；显式条目 key 保留挂载身份。

`typographyTable(configure: TypographyTableBuilder.() -> Unit)` 收集 header／row 接收者，行 builder 用 cell(String／StyledText／View) 按顺序添加单元格。header 至多一行且必须先声明，row 可指定稳定 key。所有行列数必须相同且非零；空表合法，返回零自然尺寸。columns: Array<GridTrack>? 默认 None，由实际列数生成等分 Fraction(1)；明确 columns 必须覆盖所有列，直接复用 Grid 的轨道规则。有界默认列共享可用宽度，无界则从各列真实内容取得自然尺寸；所有行共享同一 Grid，长文字换行决定实际行高。

表格普通文字为 16／24，表头为 700；单元格水平留白 16、垂直 8、边线 1，接缝只画一次，偶数正文行使用 muted 背景。列顺序、起端对齐和单边线随 RTL 转换，不镜像文字。builder 提供 fonts／theme／key；单元格的 StyledText 和 View 保留明确样式或行为。本入口只负责静态文字排版；P7 的 Table／DataTable 另行提供完整表格部件和交互，不把这里的文字组合计为 P7 已完成。

### 14.3 Kbd 与 KbdGroup

`kbd(content, configure: KbdBuilder.() -> Unit = {})` 接受 String、View、ShortcutChord 或 ShortcutLabel，创建 KbdProps／Kbd。builder 提供 fonts／theme、textStyle: TextStyle?、style、key。默认使用 muted 颜色对、small 圆角，最小宽与高度 20、水平留白 4，文字 12／16、500；数值按 body 字号相对 14 缩放。宽度来自内容自然测量并满足父约束，不把普通 Label 的有限段落宽当作键帽的自然宽。

ShortcutChord 在挂载后通过 ComponentScope.nativeWindow.shortcutLabel 取得当前平台／键盘布局的 parts 与 separator，单键帽按原顺序连接。`kbdGroup(chord／label, configure: KbdBuilder.() -> Unit = {})` 为各 part 分别创建键帽，并在 separator 非空时绘制原分隔文字；顺序始终按平台标签保持 LTR，外层布局可为 RTL。显式 ShortcutLabel 使用调用方已有结果。后续 Props 或主题更新重新查询当前窗口标签；没有键盘布局轮询或自动绑定注册。

`kbdGroup(configure: KbdGroupBuilder.() -> Unit)` 则收集普通 ChildrenBuilder 子项，以中心交叉对齐和默认 4 的 spacing 排列，支持组合快捷键、说明文字或自定义键图标；spacing 必须有限非负。Kbd 自身只绘制，不创建 Press、焦点或快捷键注册。实际动作与提示由调用方复用同一个 ShortcutChord；嵌入 Button、文字行内对象或其他区域不改变其事件归属。资源沿普通 Label／Frame 与作用域管理，借用字体不关闭。

## 15. FieldLabel 与控件关联

`FieldTarget()` 是标签使用的稳定窄句柄；`isAttached` 表示存在尚未卸载的控件关联，`isEnabled` 表示该关联的当前 enabled 输入。它不暴露组件实例或任意调用入口，也不替代 FocusHandle。一个句柄同时只能关联一个已挂载控件；重复绑定报告普通参数错误。InputProps／TextareaProps／CheckboxProps／SwitchProps 及对应 builder 提供 `labelTarget: FieldTarget? = None`。控件作用域拥有关联，更新 Props 时更新目标及动作，替换或卸载解除旧关联；多个标签可以使用同一个目标。

`fieldLabel(text: String, target: FieldTarget, configure: FieldLabelBuilder.() -> Unit = {})` 及 StyledText／View 重载创建 FieldLabelProps／FieldLabel，ChildrenBuilder 有同名成员。Props 的 content 为 FieldLabelContent.Text(LabelText) 或 Custom(View)；builder 提供 fonts／theme、textStyle: SpanStyle?、settings: ParagraphStyle?、style: StyleOverride、key。文字使用现有 Label 排版，不可选择，不增加 Tab 停靠点。普通文字以当前 body 字体族、14／14、500 和 foreground 为基础，尺寸按 body 字号相对 14 缩放；StyledText 保留明确基础样式和片段，textStyle 覆盖基础样式。Custom 保留完整 View 及自身样式，标签只提供原有指针关联，可包裹含控件的 choice card；子控件消费的点击不再次触发标签动作。无关联或目标禁用时整体透明度为 0.5，局部装饰仍由 style 表达；原有 label 保持纯文本用途。

标签的指针操作复用非聚焦区域上的 Press，正常取消拖出、取消序列和被子动作消费的点击。激活时检查标签与控件属于同一窗口、双方仍在当前输入域、控件 enabled 且实际区域可聚焦；未挂载、禁用、隐藏、未放置、域外或旧关联均不触发动作。Input／Textarea 只按 Pointer 来源请求焦点，保留文本、选区及只读语义；Checkbox／Switch 请求焦点后执行与自身相同的一次选择动作，读取最新 Binding。FocusOptions 被替换后使用当前 FocusTarget.handle；不合成键盘／指针事件。标签目标或实际关联在按下期间改变时取消该次点击，不把旧按压转交新控件。作用域收尾不关闭借用目标或向已退役控件发起请求。

## 16. ButtonGroup

`buttonGroup(configure: ButtonGroupBuilder.() -> Unit)` 创建 ButtonGroupProps／ButtonGroup，ChildrenBuilder 提供同名成员。根字段为 entries: Array<ButtonGroupEntry>、axis: FlexAxis = Horizontal、fonts／theme: None、style: StyleOverride = styleOverride()；builder 另有 key。ButtonGroupEntry 保存明确的 ButtonGroupContent、key: ViewKey = Position 和 layout: FlexItemOptions = FlexItemOptions()。ButtonGroupContent 区分 Button(ButtonProps)、Input(InputProps)、Text(PanelContent, FontCollection?, StyleOverride)、Separator(Color?)、Custom((GroupPosition) -> View)、Plain(View)、Nested(View)，不检查或克隆不透明 View。空组自然尺寸为零。

builder 的 button 提供现有四种内容重载及相同配置，input 接收 TextEditorState／InputBuilder，text 接受 String／StyledText／View 和 ButtonGroupTextBuilder（fonts、style、key）。separator(color: Color? = None, key = Position) 创建内部接缝；item(render: (GroupPosition) -> View, key = Position, configure: FlexItemBuilder.() -> Unit = {}) 接入自定义条目；add(View, key = Position, configure: FlexItemBuilder.() -> Unit = {}) 保留调用方已完成的外观；group(configure: ButtonGroupBuilder.() -> Unit) 加入嵌套组。Button／Input 的 key 来自自己的 builder，其他条目使用明确的外层 key；不读取 View 的内部 key。根 fonts／theme 为未明确指定的内置条目提供默认来源，嵌套组拥有自己的配置。

GroupPosition 是含 axis、first、last 的普通值。ButtonProps、InputProps／TextareaProps 及对应 builder 增加 group: GroupPosition? = None，自定义条目可把当前位置明确传给实际控件。组内没有选择状态、roving 导航或额外 Press。没有嵌套组时，按条目位置连接相邻表面；含嵌套组时各直接条目保持独立边角并使用 8 的间隔，按 body 字号相对 14 缩放。Plain 始终保留原外观，Custom 自行使用位置值；多个窗口和主题不共享可变位置上下文。

连接表面去掉内部圆角和后项逻辑起端边框，垂直组去掉后项上边框；第一项起端保持原圆角，末项末端默认使用主题 large 圆角，明确的局部圆角覆盖保留外侧值。测量、内容内缩、边框内轮廓、绘制和焦点环使用同一连接结果，RTL 只解析逻辑起止侧。父约束继续优先，零尺寸合法。文本条目使用 muted 背景、border、14／20 的 medium 文字和水平内缩 10；内部分隔线使用 input 颜色、主轴厚度 1、交叉轴两端各留白 1。尺寸随正文密度缩放，细线保持 1。

排列复用现有 Flex 测量、收缩及交叉轴 Stretch。纵向先取条目自然宽度，再受父宽约束统一拉伸；普通有限宽上限不把组撑满，明确的最小宽仍生效。builder 的内置 Input 默认 grow=1，其余条目默认 grow=0。本组中含实际焦点的条目最后呈现，保证接缝上的焦点环可见；这只改变绘制顺序，不改变布局、Tab 顺序或子组件动作。组默认不裁剪溢出的环，显式根 style 仍可裁剪。移除、增添、重排和方向／主题更新沿普通 key 匹配保留仍存在的控件及文本模型。

## 17. Item

### 17.1 内容、尺寸与排列

`item(configure: ItemBuilder.() -> Unit = {})` 与 `item(content: View, configure: ItemBuilder.() -> Unit = {})` 创建同一个 ItemProps／Item。content: View? 默认 None；title／description: PanelContent?、media／actions／header／footer: View? 也均默认 None。标题／说明已经足以组成条目，不为未提供的正文创建空 View 或间距。ItemBuilder 继承 PanelBuilder，提供 media／actions／header／footer 的同名 View 成员；title／description 沿用 String、StyledText、View 重载，正文由所选工厂重载提供。ChildrenBuilder 使用 `contentItem` 的两个重载，避免与现有 LinearBuilder.item 的 Flex 配置产生歧义；顶层工厂仍为 item。

其余字段为 variant: ItemVariant = Default（另有 Outline／Muted）、size: ItemSize = Default（另有 Sm／Xs）、mediaVariant: ItemMediaVariant = Default（另有 Icon／Image）、onClick: (() -> Unit)? = None、enabled: Boolean = true、focus: FocusOptions = FocusOptions()、focusOutline: OutlineStyle? = None、states: InteractionStyles = interactionStyles()、transition: StyleValue<AnimationSpec?> = Inherit、fonts／theme: None、style: StyleOverride = styleOverride()；builder 另有 key。style／states／focus 提供既有接收者配置，transition 明确 Set(None) 关闭过渡。

默认外框为 large 圆角、1 单位透明边框与透明背景；Outline 使用 border 颜色，Muted 使用 muted 背景的 0.5 透明度。Default／Sm 水平内缩 12、垂直 10、区域间距 10；Xs 为 10／8／8。尺寸按主题 body 字号相对 14 缩放，细边保持 1。条目填满有限父宽，无界时取内容自然宽；父约束和零尺寸沿普通布局合同处理。

header 和 footer 独占全宽，中心区域按逻辑方向排列 media、content 和 actions。content 依次包含实际存在的 title、description 与正文，Default／Sm 间距 4，Xs 为 0。标题普通文字为 body 字体族的 14／19.25、500，默认一行省略；说明为 muted 前景的 14／21、400，Xs 为 12／18，默认两行省略。普通文字方向随挂载方向；明确的 LabelProps.settings、StyledText 基础样式和自定义 View 保留自身配置，正文可直接使用 Typography／SelectableStyledText 提供选择与复制。

动作区域先取自然宽度；留给内容不足当前 body 字号的 8 倍时，动作移到下一行的逻辑末端。并排时正文和动作垂直居中；media 在有说明时顶端对齐并下移 2，其他情况居中。Default 媒体保持自然尺寸，可放 Avatar 或任意 View；Icon 使用 16 方框，Image 使用 Default／Sm／Xs 对应的 40／32／24 方框与 small 圆角裁剪，内容按实际方框紧约束测量。`media(image: Image)` 便利成员创建借用该图片的 Cover ImageView 并选择 Image 变体，不读取文件或下载资源；`media(View)` 不检查图片或修改其适配方式。

可选区域使用稳定的局部 key；标题／说明、媒体、动作、头尾的增删及窄宽换行，保留仍存在的正文与内部控件。Group 与 Item 都不推断列表选择、导航目的地或数据源，不自行打开 URL。

### 17.2 行动作与输入归属

无 onClick 时只展示，不建立行 Press 或 Tab 目标，正文和动作里的控件仍保有自己的输入行为。有动作时建立一个可聚焦区域，使用同一 Press 的 default 阶段与 routeChildren；行空白或非交互文字可点击，Space／Enter 可激活。子按钮消费激活，选择文字与 Drag 接管指针序列时取消行点击，不在整行上覆盖透明按钮，也不合成子输入事件。事件读取最新回调和 FocusOptions。

动作行的 hover／press 默认过渡为 100ms，hover 使用 muted 背景，pressed 在其上混入 8% background.foreground；局部 style／states 覆盖默认表面，focusOutline 可覆盖当前主题焦点环，Reduced 最终关闭过渡。enabled=false 禁用本行及其子树，并采用主题 disabledOpacity；展示模式也可禁用内部控件。禁用、移除及卸载取消未完成的行输入。onClick 有无变化按普通子树替换规则建立／撤销行为；同一动作模式内的 Props 更新不重新挂载行内容。借用字体、图片、FocusHandle 和业务模型不随行关闭。

### 17.3 ItemGroup 与 ItemSeparator

`itemGroup(size: ItemSize = Default, configure: ItemGroupBuilder.() -> Unit)` 收集普通 ChildrenBuilder 子项，并提供 `item(configure)`／`item(content, configure)` 成员。size 决定组间距并作为这些条目的初始尺寸，明确条目 size 可覆盖；Default／Sm／Xs 间距为 16／10／8，按 body 字号缩放。ItemGroupProps 保存 children、size、theme 和 style；builder 的 theme／style／key 只配置容器，条目字体与主题继续显式配置或取 ambient。组填满有限宽度并按声明顺序排列，不增加焦点、选择或输入行为；key 沿普通子项匹配。

`itemSeparator(configure: ItemSeparatorBuilder.() -> Unit = {})` 创建 ItemSeparatorProps／ItemSeparator；Props 保存 color: Color? = None、theme: ThemeSource? = None，builder 另有 key。内容为水平 Separator，颜色默认 border，上下各留白 8 并随 body 字号缩放。ChildrenBuilder 提供同名成员，ItemGroup 可混排分隔线和自定义内容。实现复用原 Separator 的有限／无界测量与绘制，不另建分隔线或列表框架。


## 18. Switch 与 Toggle

### 18.1 Switch

`switchControl(checked: Binding<Boolean>, configure: SwitchBuilder.() -> Unit = {}): View` 以及同名 ChildrenBuilder 成员构造 Switch。SwitchProps 接收 checked、enabled=true、invalid=false、size=SwitchSize.Default、style=switchStyle()、theme=None、focus=FocusOptions()、labelTarget=None；builder 提供相同选项及 key，并提供 style／focus 接收者配置。SwitchSize 为 Default／Sm。控件读取并写入当前 Binding；有效指针、Space 或关联 FieldLabel 激活时读取最新值并取反一次。业务 setter 可以拒绝写入，不保存镜像或乐观值；外部更新不合成激活。Enter 不切换。取消、重复键、失焦、禁用和卸载遵守 Checkbox 的 Press 合同；替换绑定、FocusHandle、FieldTarget 或主题保留挂载和输入区域。

SwitchStyle 的 track: SurfaceOptions、thumb: StyleOverride、states: InteractionStyles、hitInsets: Insets? 默认未覆盖；transition／selectionTransition 为 StyleValue<AnimationSpec?>.Inherit。switchStyle 及 copy 使用 SwitchStyleBuilder，track／thumb／states 有接收者配置，两个 transition 也可显式写 None。hitInsets 未指定时为左右 12、上下 8；必须有限且非负，不参与布局。主题 body 字号／14 作为默认尺寸密度；显式 hitInsets 不再缩放。

默认轨道为 32×18.4，Sm 为 24×14，边框 1，圆角为完整胶囊，thumb 为 16／12。有限父约束优先，零尺寸合法；轨道使用最终布局 bounds。滑块尺寸不超过轨道内框可用尺寸，垂直居中；Unchecked 在逻辑起点，Checked 在逻辑终点，RTL 反向。命中范围在轨道 bounds 外扩 hitInsets，沿正常场景裁剪；实际零宽／零高不增加命中范围。

未选中背景为 palette.input，Dark 乘 0.8 alpha；选中为 primary.background。Light thumb 为 background.background，Dark 未选中为 background.foreground、选中为 primary.foreground。轨道和 thumb 颜色以及 thumb 位移使用同一个 0…1 选择动画采样；首次挂载直接显示当前值，后续使用主题 selection（shadcn 为 150ms），局部可覆盖或禁用。hover／press 使用已有交互动画；disabled 停止过渡，Reduced 最终关闭所有过渡，普通重建不重启动画。没有独立时钟或定时线程。

track 使用公共 SurfaceKind.Plain 的几何和 painter；focus、invalid、disabled opacity 沿主题统一合同。局部 track／thumb Style 最后覆盖预设，states 应用于轨道。默认轨道透明边框，focus-visible 使用 ring 边框、invalid 使用 validation.borderColor，invalid 优先。禁用透明度包住轨道与 thumb 整体，避免重复衰减。动画不活动或卸载后没有额外帧需求。

### 18.2 Toggle

`toggle(content, pressed: Binding<Boolean>, configure): View` 提供 String、StyledText、View、(ActionContentStyle) -> View 四种内容入口及 ChildrenBuilder 成员；对应 ToggleBuilder、StyledToggleBuilder、ContentToggleBuilder。ToggleProps 使用同一 ButtonContent，并接收 pressed、enabled=true、invalid=false、variant=ToggleVariant.Default、size=ToggleSize.Default、style=buttonStyle { kind=Plain }、theme=None、focus=FocusOptions()。builder 另有 key；String／StyledText 借用 fonts，StyledText 可指定 settings／inlines。style 和 focus 的接收者配置沿 Button 合同，构造只发布描述，不创建资源或调用 setter。

ToggleVariant 为 Default／Outline，ToggleSize 为 Sm／Default／Lg。pressed 是业务选择值；鼠标或 Space／Enter 的短暂按下仍由 Press.interaction.pressed 表达，两者独立。每次有效激活读取最新 Binding 并取反一次，setter 可拒绝；外部更新、更换绑定和普通重建不生成第二份状态。取消、重复键、禁用、失焦、卸载和子动作消费遵守 Button 的输入规则，复用同一 ActionInk／内容解析路径。

默认透明背景、无边框；Outline 为透明背景和 1 单位 input 边框。hover 或选中时背景 muted.background、文字 background.foreground；瞬态 pressed 在 muted 背景上混入 8% background.foreground，松开后恢复对应选择／悬停状态；未选中文字也为 background.foreground。height／minimum width 为 28／32／36，paddingX=10、paddingY=0，gap=4，默认字号 14、Sm 为 12.8，行高 20；图标 16、Sm 为 14。尺寸按主题 action 字号／14 缩放，圆角使用主题 large，Sm 使用 min(medium,12)。内容自然宽度和父约束优先；explicit style／文字设置仍可覆盖。focus-visible、invalid 和 disabled 沿统一主题合同；选择不伪造短暂按下，禁用透明度覆盖文字与图标。ToggleProps／builder 可接收 group: GroupPosition? = None，沿 ButtonGroup 的圆角、单接缝及焦点绘制合同；分组选择策略属于 ToggleGroup。

## 19. Input OTP

`inputOtp(state: TextEditorState, length: Long, configure: InputOtpBuilder.() -> Unit = {}): View` 和 ChildrenBuilder 的同名成员创建一个 InputOtpProps／InputOtp。length 必须为正；Props 包含 state、length、parts: Array<InputOtpPart>=[]、pattern=Digits、enabled=true、readOnly=false、invalid=false、fonts／theme／labelTarget=None、focus=FocusOptions()、style=inputOtpStyle()、caretBlink=500ms、onEdited／onComplete 两个默认空回调。所有编辑、选区、composition 和撤销历史只有一份 TextEditorState；整体只有一个 Tab 目标和 TextEditHost，FieldTarget 沿现有输入标签关联。

InputOtpPart 为 Group(InputOtpGroup)／Separator(View)。InputOtpGroup 保存非空的 Array<InputOtpSlot>，slot 包含 index 和 invalid=false；全部 group 的索引必须按正文逻辑顺序恰好覆盖 0..<length。parts 为空时使用一个默认组。显式描述必须以组开始和结束，不能出现连续 separator；相邻组可直接相连。builder 的 `group { slot(index, invalid=false) }` 和 `separator(content: View)` 构建描述，`separator()` 使用 16 单位的减号图形。分隔内容是正常 View，保留其 key 与挂载；槽和组是同一编辑表面的几何，不各自挂载输入控件。方向改变按逻辑起止侧翻转布局，组和分隔符不进入正文、粘贴结果或复制文本。

InputOtpPattern 为 Digits／Alphanumeric／Custom(TextEditPolicy)。前两者只接受 ASCII 数字／ASCII 数字与字母，粘贴先移除新增片段中的 ASCII 空白和连字符；新增片段中的其他非法字符使本次提交拒绝，不顺带清理外部写入的旧正文。Custom 保留调用方的 paste／transform／accepts，长度上限取其明确 maxLength 与 length 的较小值；未设上限时使用 length。统一通过 TextEditPolicy 处理过滤、grapheme 计数和截断，一次粘贴只产生一个撤销项。preedit 不提前过滤或截断，最终提交使用当前策略；外部模型更新不触发完成通知。

onEdited 沿 TextEditHost 的正文通知；onComplete 只在宿主接受的操作改变已提交正文、组合已经结束且新正文恰为 length 个 grapheme 时调用。满长度下真正替换内容可再次完成；纯选择、主题／方向／分组变化、重复 render、取消或拒绝后恢复原基线不重复通知。回调读取最新 Props，允许调用方替换正文或移除控件，不自动提交表单或移走焦点。

InputOtpStyle 配置 cellSize: Size?、gap=8、surface: StyleOverride、text: SpanStyle?、focusOutline／validation: OutlineStyle?、selectionColor／inactiveSelectionColor／caretColor／markedColor: Color?、caretWidth=1。提供 inputOtpStyle、copy 和对应 builder；surface 只覆盖槽的装饰，text 覆盖主题正文。默认槽为 32×32，按主题正文大小／14 缩放；组内接缝只画一条 input 边框，组首尾使用 medium 圆角。Dark 底色为 input 的 0.3 倍 alpha；Light 透明。活动槽按统一焦点规则显示 ring，invalid 优先，局部装饰继续优先。禁用透明度作用于整个组合，readonly 仍允许选择、复制和导航。

槽位、间隙、鼠标命中、方向导航、选区、marked 装饰与原生 caret 共享 UI 规范第 7.5.3.1 节的 TextEditLayout。空槽中的可见 caret 居中，已有字符处使用真实插入边缘；只在 caret 确实可见时闪烁。临时超长 preedit 或外部超长正文按同一槽几何向逻辑末尾扩展，延续末组边框并可滚动显露；不改写模型来凑固定槽数。测量为外侧焦点环预留空间；有限窄宽和零尺寸沿同一视口裁剪，主题／分组／方向变化保留输入宿主与选区；卸载释放排版、取消闪烁及输入连接，字体由调用方持有。


## 20. Radio Group 与 Toggle Group

### 20.1 选择项及分组协议

`Choice<T: Equals<T>>(key: ViewKey, value: T, label: String, enabled: Boolean = true)` 是两类分组共用的不可变领域元数据。key 必须显式；同组 key 和 value 均唯一，新的 Props 快照接入时检查，报告 IllegalArgumentException；普通内部交互重绘不重复校验。选择值使用 Equals<T>，不转换为字符串、反射类型或挂载 key。外部绑定值不在当前条目中时保留；空条目合法且不产生 Tab 停靠点。

组按条目 key 保留内部 FocusHandle，重排不重新挂载项；可通过项 focus 提供外部 handle，替换遵循 FocusTarget 合同。组只有一个 roving Tab 停靠点，默认循环；方向及 Home／End 复用 KeyNavigation，跳过 disabled／不可达项，水平轴继承 RTL。组的 enabled=false 禁用整个输入子树；条目可独立禁用，组 invalid 与项 invalid 合并。绑定 setter 可拒绝写入；渲染不调用 setter，不保存选择镜像，激活使用最新绑定。

### 20.2 Radio Group

`radioGroup(selected: Binding<T?>, configure: RadioGroupBuilder<T>.() -> Unit = {}): View` 及 ChildrenBuilder 成员使用 RadioGroupProps／RadioGroup。builder 的 `item(choice: Choice<T>, configure: RadioItemBuilder.() -> Unit = {})` 收集 RadioGroupEntry<T>，`add(entry)` 接受完整值。组属性为 entries、selected、axis=Vertical、spacing=8、loop=true、enabled=true、invalid=false、fonts=None、theme=None、style=styleOverride()，builder 另有 key。项属性为 choice、focus=FocusOptions()、labelTarget=None、invalid=false、marker=radioOptions()、content=None。

默认项使用 16 单位圆形 indicator（尺寸由主题 RadioMetrics 决定），与 14／20 的标签相隔 8；选择圆点 8、边框 1、focus-visible 和 invalid 使用主题 ring，颜色／选择动画复用主题及 Surface 时间轴。禁用透明度只对整项叠加一次，包含 indicator 与文字。默认纵向项占用可用宽度，横向按内容布局；spacing 有限非负，父约束和 RTL 优先。所有主题使用既有 radioMetrics／radioGeometry／drawRadio 路径。

项的自定义 `content: ((RadioItemContent) -> View)?` 收到 indicator: View、selected、enabled、invalid、textStyle: TextStyle，可组合说明与 choice card。indicator 是普通非交互绘制描述，整项拥有一套 Press／FocusTarget，子动作消费仍能取消整项动作；不检查或改写不透明 View。Pointer／Space／FieldLabel 激活选择该项，已选项不清空、不重复写入；Enter 不选择。方向／Home／End 接受组内焦点请求后选择目标；Tab 优先已选有效项，当前仍持有焦点的有效项优先，未知或禁用选择回退到历史／首项。直接请求焦点不改变选择。FieldTarget 关联使用与整项相同的最新选择动作。

### 20.3 Toggle Group

`toggleGroup(selected: Binding<T?>, configure: ToggleGroupBuilder<T>.() -> Unit = {})` 与 `toggleGroup(selected: Binding<Array<T>>, configure)` 及 ChildrenBuilder 对应成员提供单／多选。完整 ToggleGroupProps 使用 `ToggleGroupSelection<T>.Single(binding)`／`.Multiple(binding)`。builder 以 `item(choice, configure: ToggleGroupItemBuilder.() -> Unit = {})` 或 add(ToggleGroupEntry<T>) 收集项；项默认使用 choice.label，可提供 ButtonContent（包括 Resolved 自定义图标／文字），以及 focus、invalid、可选 ButtonStyle。

组属性为 entries、selected、axis=Horizontal、spacing=8、loop=true、enabled=true、invalid=false、variant=Default、size=Default、fonts=None、theme=None、style=styleOverride()，builder 另有 key。每项复用 Toggle／ActionInk。单选再次激活清空，多选按当前数组追加／删除本值，保留未知值及其顺序；重复本值删除全部。方向键只移动焦点，Space／Enter／指针更改选择，Tab 遵循当前／历史焦点，不因外部选择变化合成焦点请求。

spacing=0 时沿轴连接边框和圆角，Outline 相邻处只画一条边，首末保留外角，焦点项最后绘制；默认左右 padding=8，独立或有间距项保持 Toggle 的 padding=10，显式项样式优先。横向按内容宽度，纵向按最宽内容宽度并拉齐各项；样式、尺寸、禁用透明度和主题更新沿 Toggle 合同，不新建绘图或按压状态机。


## 21. Slider

`slider(values: Binding<Array<Double>>, range: RangeSpec, configure: SliderBuilder.() -> Unit = {}): View` 及 ChildrenBuilder 成员构造 SliderProps／Slider。values 使用稳定 thumb 下标；支持零个、单个、范围及多个 thumb，重建不交换其实例或焦点。组属性为 orientation=Horizontal、enabled=true、invalid=false、keyboardStep=None、thumbs=[]、labelTarget=None、style=sliderStyle()、theme=None、onValueCommit={}、onValueCancel={}，builder 另有 key。thumb(configure: SliderThumbBuilder.() -> Unit) 收集 SliderThumbOptions(focus=FocusOptions(), labelTarget=None)，addThumb 接受完整选项。未指定 thumbs 时每个值使用默认选项；指定时数量须与 values 一致。根 labelTarget 关联首个 thumb，显式项 labelTarget 优先；标签只请求焦点，不改值。

输入快照沿 RangeSpec 检查有限性、范围、离散步、排序和 minimumGap；非法输入报告 IllegalArgumentException，禁止排序或静默改写外部值。连续范围必须提供有限正 keyboardStep，离散范围使用 RangeSpec.step。相同不可变 values 数组和等值范围复用校验结果；内部指针／键盘计算复用已检查的界限，不重复全量验证。临时 Binding 或 RangeSpec 对象可以在普通配置中重建；业务更新以当前数值及范围字段变化识别，不以适配闭包的对象身份判断。

每个 thumb 有独立 FocusTarget 和正常 Tab 停靠点。仅真实、无修饰的 key-down／repeat 执行动作：水平 Left／Right、垂直 Down／Up 分别减少／增加，水平 RTL 和 reversed 各反转一次；Home／End 到当前 thumb 允许的最小／最大值，PageUp／PageDown 增减十个步长。其他轴箭头、Space／Enter 及 key-up 保留给外层。只移动当前 thumb，保持邻居和身份，达到边界仍消费已认领的键。每次实际被 Binding 采纳的键盘变化调用 onValueCommit 一次，拒绝或无变化不通知。

指针输入复用 threshold=0 的轴向 Drag，覆盖轨道和 thumb 扩展命中区。轨道按下选取最近可移动 thumb，同距优先当前聚焦项，再按下标；点击 thumb 保留抓取位置，点击轨道立即移动最近 thumb。移动使用最近 Accepted 的轨道／thumb 几何和该次样本，不从待提交布局推算；横向与纵向的端点将 thumb 保留在控件尺寸内。只有正常 release 提交一次实际最终变化；onValueCommit 接收当前采纳的完整数组，外部同步更新不合成通知。

取消恢复本次按下前的数组，但仅在当前值仍等于最后一次自身写入结果且范围未被业务替换时尝试恢复，setter 仍可拒绝。外部值／范围、thumb 数量、方向、orientation 变化使旧拖动结束；外部新值与范围保留。禁用、失焦、几何丢失、Escape、被阻止的释放和卸载均有取消出口。`onValueCancel(values, reason: DragCancelReason)` 报告收尾后的实际数组及原因；取消不再调用 commit。所有动作使用最新 Props，回调可以移除控件；输入资源随挂载收尾，没有常驻时钟或第二份受控值。

SliderStyle 的 length／thickness／thumbSize／hitInsets 可覆盖默认 160／4／12／8（按主题 body 字号／14 缩放）；长度有限非负。有限主轴填充可用空间，无界主轴使用 length；交叉轴取 thumbSize，父约束优先，包括零尺寸。纵向最小值在下端，reversed 翻转；单 thumb 的 range 从数值最小端延伸到 thumb，多 thumb 在首末之间填充。track／range 使用普通 StyleOverride，默认 muted.background／primary.background 与全圆角；thumb 使用普通 StyleOverride 并映射为现有 SurfaceOptions，默认白底、ring 色 1 单位边框。hover／pressed／focus-visible 使用 3 单位主题 ring，invalid 使用统一 validation。主题及样式更新走现有 geometry／StylePainter／ThemePainter；disabled 透明度对整控件覆盖一次。动画只平滑交互颜色／光环，值位置立即响应绑定；reduced motion 遵循主题，静止后停止请求帧。

## 22. Input Group

`inputGroup(configure: InputGroupBuilder.() -> Unit): View` 及 ChildrenBuilder 成员收集一个编辑控件和附加区域。InputGroupProps 必须有 `control: InputGroupControl`，其变体为 Input(InputProps)、Textarea(TextareaProps)、Custom(View, FocusHandle)；其余属性为 addons=[]、enabled=true、invalid=false、fonts=None、theme=None、surface=surfaceOptions()。builder 另有 key；input(state, configure)、textarea(state, configure) 或 control(view, focus) 恰好调用一次，重复／缺失立即报告 IllegalArgumentException。这只是正常的配置校验，不检查不透明 View 或反射类型。

`addon(align=InlineStart, configure: InputGroupAddonBuilder.() -> Unit)` 收集 InputGroupAddon(content: View, align, key=Position, padding=None, style=styleOverride())，也可 addAddon 完整描述。InputGroupAlign 为 InlineStart／InlineEnd／BlockStart／BlockEnd。addon builder 提供 add(View)、text(String)、button 的文字／View／解析后内容入口，spacing 默认继承 8（按字号缩放），可配置 key、padding、style；button 默认 Ghost／Xs，其余变体和尺寸使用现有 ButtonBuilder。text 使用 Muted 文字角色，图标、Kbd、Spinner、菜单等通过普通 View 组合；内置文字与按钮继承根 fonts／theme 的缺省值，显式配置仍优先。

正文始终位于逻辑子序列首位，附加区域按声明顺序挂载、按 align 放置；RTL 只交换横向逻辑起止。添加、移除、改位置或按 key 重排 addon 不改变正文父级与描述类型；保留文本模型、选区、撤销历史、IME 连接和实际焦点。相同描述类型下替换配置回调不更换实例。Input 与 Textarea 是不同控件类型，显式切换遵守通常卸载／挂载规则。

内置控件显式使用 Bare 模式；根只消费其 enabled／invalid／focus 等已知 Props，既不遍历后代，也不创建编辑器或转写正文。根 enabled 与内置 enabled 共同控制输入，invalid 合并为外框状态。根使用原 Theme Plain 表面，内缩默认 0，只增加一份边框；内置控件的 padding 负责文字留白。禁用透明度对整组应用一次，附加按钮仍通过原 Press 接收有效状态和取消。内置 description 保留在编辑槽内；需要组外帮助文字时由 Field 或普通兄弟视图组合。

每个组有稳定的内部 FocusHandle，只有内置控件未提供外部 handle 时使用；撤销外部 handle 回到原内部 handle。外框的 focus-visible 只跟随编辑控件，附加按钮显示自己的焦点环。点击未被后代默认动作处理的附加内容／空白，以 Pointer 来源请求同一控件焦点，不改选区或合成提交；按钮、链接等自己的动作优先。自定义控件必须由调用方实际连接所给 FocusHandle，并自行采用无外框外观及 FieldTarget 关联；组不为其补挂另一个输入区域或 IME 连接。禁用、未挂载、零尺寸、隐藏或域外的目标不能通过附加区域获取焦点。

有限宽度填满父约束，无界宽度采用 surface.suggestedSize.width 或 240。顶部／底部区域按可用宽度测量，起始／末尾区域占用各自自然宽度，正文取得剩余宽度；高内容按实际尺寸扩展整组。Inline 默认水平内缩 8、垂直 0，Block 默认水平 10、垂直 8，均按字号缩放，可逐区覆盖。窄宽和紧高度下各区域受剩余空间约束，正文可以为零尺寸；整个组零尺寸时不发布子输入区域。边框、文字基线、裁剪与命中使用同一次测量结果，失败帧不替换成功几何。

验收覆盖全部四个位置、RTL、窄宽／零尺寸、单行与多行、外部 handle／模型更新、正文与 keyed addon 的实例保留、真实中文 preedit／commit、复制／撤销、密码显隐、输入策略、只读／禁用／invalid、附加按钮动作与 Tab 顺序、标签、静止停帧及卸载收尾。Gallery 的文件输入示例接应用选择动作，不在本库增加文件对话框或文件解析。

## 23. Field 与表单分组

`field(control: View, configure: FieldBuilder.() -> Unit = {}): View` 和 ChildrenBuilder 成员构造 FieldProps／Field。control 必需且始终保留在同一直接父级的固定槽中；其余输入为 heading: FieldHeading?、description: PanelContent?、errors: Array<String>=[]、error: PanelContent?、orientation=Vertical、controlPlacement=End、descriptionPlacement=AfterControl、required=false、requiredText="Required"、invalid=false、breakpoint／controlWidth／spacing／contentSpacing=None、fonts／theme=None、padding=Insets()、style=styleOverride()；builder 另有 key。FieldHeading 为 Label(FieldLabelContent, FieldTarget) 或 Title(PanelContent)，label／title 成员选择一个标题，支持 String／StyledText／View；description／error 同样接受这些内容。required 只附加可本地化说明，不执行校验或创建提交规则。

Field 不读取不透明 control 的属性或后代，不拥有编辑模型、选择绑定、控件工厂或校验状态。invalid 只决定自动标题与必填说明的错误色；error 自定义内容优先于 errors，错误数组忽略空字符串并按首次出现顺序去重，一条直接显示、多条显示带缩进的项目符号，空数组不产生错误槽。控件自身的 enabled／invalid、Field 的状态和业务错误由调用方明确传入；没有自动状态同步、通用 Form engine、类型反射或额外类型身份。FieldLabel 沿第 15 节关联目标并反映其 enabled；普通 title 没有激活动作。自定义标题／说明／错误 View 保留调用方样式与交互。

FieldOrientation 为 Vertical／Horizontal／Responsive。Responsive 仅在本次有限可用内容宽度达到 breakpoint 时横排；默认阈值为 448，按主题 body 字号／14 缩放，无界宽度按纵排。FieldControlPlacement 为 Start／End，按逻辑方向决定控制槽在标题前／后；FieldDescriptionPlacement 为 AfterControl／WithHeading。纵排按标题与 control 的指定顺序排列说明及错误，错误总在最后；WithHeading 将说明紧随标题。横排把标题和 control 分列，说明依配置归列；End 的错误在 control 列，Start 的错误在标题列，适用于 Checkbox／Switch 与说明组合。RTL 只交换逻辑横向位置，不改变挂载、键盘顺序或输入方向的显式设置。

默认主间距为 8、同列内容间距为 2，均按字号缩放。有限宽度填满父约束；横排先以 controlWidth 或扣除间距后可用宽度的 60% 为最大宽度测量 control，紧凑控件保持自然宽度，标题列取得实际余量；无界横排按内容自然宽度。controlWidth、breakpoint、间距及 padding 必须有限非负。正文每轮只测量一次，布局切换和标题／说明／错误增删不重新挂载它，也不丢失焦点、选区、preedit 或撤销历史。窄宽和有限高度遵守父约束，内容超出最终布局时裁剪；正常布局保留控件自身向外绘制的焦点环。零尺寸只测量、不放置子节点，Accepted 帧按既有规则撤销输入资格，恢复时沿同一实例继续。外框装饰复用 Frame／StyleOverride，Field 自身没有 Press 或焦点停靠点。

`fieldTitle`、`fieldDescription`、`fieldLegend` 的 String／StyledText／View 入口构造 FieldTextProps／FieldText；普通字段文本使用 body 字号和 20／14 行高，Title／LegendLabel 字重 500，Description 为 muted.foreground，Error 为 destructive.background。Legend 默认字号 16、行高 24／16、字重 500，可选 FieldLegendVariant.Label；尺寸仍按字号缩放。FieldTextBuilder 配置 fonts／theme、textStyle: SpanStyle?、style 和 key，FieldLegendBuilder 另有 variant。自动标题与说明和这些独立入口使用同一解析函数；自定义 View 不被重写。

`fieldError(errors: Array<String>, configure: FieldErrorBuilder.() -> Unit = {})` 及自定义内容重载使用 FieldErrorProps／FieldError，遵循上述错误数组和优先级规则；builder 支持 content、fonts／theme、textStyle、style、key。多条错误间距为 4、项目符号列与正文缩进共 16，按字号缩放；错误与普通说明不增加 Tab 停靠点，富交互内容由调用方显式提供。没有内容时尺寸为零。

`fieldGroup { ... }` 与 `fieldContent { ... }` 使用同一 FieldStackProps／FieldStack，FieldStackKind.Group／Content 的默认间距分别为 20／2，按字号缩放。FieldStackBuilder 继承 ChildrenBuilder，提供 spacing、theme、padding、style、key；子项是普通 View，嵌套组或选择组的间距由调用方显式配置，不检查后代类型。`fieldSet(content: View) { legend(...); description(...) }` 使用 FieldSetProps／FieldSet，FieldSetBuilder 提供 legend: PanelContent?、variant=Legend、description、spacing、fonts／theme、padding、style、key，默认间距 16。legend／说明增删保留固定 content 槽；字段组和集合不建立另一套焦点域或禁用状态。

`fieldSeparator()` 及 String／StyledText／View 内容重载提供分组间的水平线，可配置 fonts／theme、color、style、key。两段 Separator 均分内容以外的宽度，内容两侧间距 8；没有内容时为一条完整线。最小建议高度 20，较高内容自然增高，窄宽和零尺寸遵守约束；颜色默认 border，内容默认 muted.foreground。它不增加输入区域或焦点。

验收包括纵排／横排／实际宽度切换、RTL、窄宽／零尺寸、单／多／重复／空错误、自定义错误与 required 说明、标签／目标换绑、choice card 内外点击仅激活一次、禁用与只读、标题／说明／错误更新保留实例和 IME、keyed 分组重排、嵌套 FieldSet／FieldGroup、静止停帧与卸载收尾。Gallery 提供带校验的设置表单组合 InputGroup、OTP、Checkbox／Radio／Switch 和 Slider；DatePicker 在 P7 接入同一表单。真实窗口行为验收不等于 OS accessibility 认证。


## 24. Collapsible 与 Accordion

### 24.1 触发器与受控 Collapsible

`collapsible(open: Binding<Boolean>, content: View, configure: CollapsibleBuilder.() -> Unit = {})` 使用 CollapsibleProps／Collapsible。Props 为 open、content、trigger: TriggerSpec?=None、enabled=true、retention=ContentRetention.Dispose、spacing=8、transition: StyleValue<AnimationSpec?>=Inherit、onOpenChange: (Boolean) -> Unit={}、fonts／theme、style=styleOverride()；builder 同名并有 key。spacing 须有限非负。没有 trigger 时可用外部按钮或程序写入 binding。构造不读取 binding、不挂载内容、不调用回调。

TriggerSpec 包含非交互的 ButtonContent、enabled=true、indicator=true、variant=Ghost、style: ButtonStyle?=None、focus=FocusOptions() 和 fonts=None。TriggerBuilder 配置这些字段，提供 style／focus 配置块。CollapsibleBuilder.trigger 的 String／StyledText／View 重载构造此描述；String／StyledText 使用原有 Label，View 直接组合。实际触发器只有一份 Button／Press／焦点登记，不能把另一个 Button 作为原始 trigger 内容。箭头位于逻辑 End，以同一 Presence 进度旋转，不建立第二个循环动画。调用方可以关闭 indicator 或使用自定义内容。

触发时读取最新 Props 和 binding；禁用不写入，允许时取反当前值，写入后只按 setter 实际采纳的变化调用最新 onOpenChange。外部写入和换绑不回放用户通知，不另存一份业务 open。Space／Enter、指针取消、焦点指示及禁用沿用 Button。根 enabled=false 停用触发器和正文输入，但不更改受控 open 或隐式收起。增删 trigger 保留正文槽。

### 24.2 折叠布局、保留与动画

ContentRetention 为 Dispose／KeepMounted。Dispose 在初始关闭时不创建正文，开始退出时保留正文到 Presence.Absent 后卸载；KeepMounted 即使初始关闭也挂载正文，关闭完成后以 MeasureContext.skipChild 保留但不测量、不放置、不接收输入或 Tab，但继续接收 Props／State 更新。两种模式开始关闭就通过内容作用域撤销输入、指针候选、焦点和 IME，并关闭其帧资格；退出绘图继续由外层 Presence 驱动。不能提前 deactivate 正文或用反射检查其类型。关闭过程中重开复用正文并从当前进度返回；输入在新内容场景 Accepted 且有非零呈现区域后恢复，不恢复关闭前的焦点。最终卸载后重开则创建新实例。

默认过渡为 200ms cubic-bezier(0.4, 0, 0.2, 1)，由 ThemeMotion 解析；显式 StyleValue.Set(None) 或 Reduced 立即达到终值，热切换 Reduced 也收束当前过渡。首次挂载直接呈现受控初值。动画只改变正文可见高度和 trigger／正文间距，不缩放文字；正文以可用宽度及无界高度每轮最多测量一次，取自然高度乘进度，最后服从父约束。正文自身更新或宽度变化重新测量；零尺寸不放置内容，恢复沿原挂载。过渡和约束溢出时绘制／命中共用裁剪，完全展开且未溢出时保留子控件向外的焦点环。关闭完成、不可见和卸载均不保留循环帧请求。保留挂载与像素缓存是两件事。

style 仅装饰根，内容装饰可组合 Frame。默认有限宽度填满，未约束宽度从触发器和正文自然宽度取得；RTL 使用同一子树和逻辑排列，不反转模型。外层显示过渡与正文动画使用各自现有 scope AnimationGroup，不引入新调度器或按帧读取 binding。

### 24.3 Accordion

AccordionSelection<T: Equals<T>> 为 Single(Binding<T?>, allowCollapse=true)／Multiple(Binding<Array<T>>)。`accordion(selected: Binding<T?>) { ... }` 与 `accordion(selected: Binding<Array<T>>) { ... }` 构造同一 AccordionProps<T>／Accordion<T>；Single builder 另有 allowCollapse=true。Props 包含 entries、selected、enabled=true、loop=true、variant=AccordionVariant.Default、retention=Dispose、transition=Inherit、fonts／theme、style 和 onOpenChange: (T, Boolean) -> Unit={}。

AccordionEntry<T> 包含 Choice<T>、必需 content、可选 TriggerSpec、可选 retention 和 style。builder 的 `item(choice, content) { trigger(...); ... }` 收集它们，item 的 trigger 默认使用 Choice.label 作为 heading，采用 Link 触发外观、逻辑 End 箭头和 hover 下划线。每项需显式且唯一的 key，value 也唯一；沿用 Choice 校验。根与项 enabled 共同决定交互。keyed 重排、标题／内容 Props 变化、绑定替换和主题／方向变化按普通描述匹配保留实例，不把 ViewKey 当作类型身份。

Single 展开一项时写入它的 value；allowCollapse=false 只禁止用户关闭当前项，不替调用方选择初值，不改写外部 None、缺失或禁用值。Multiple 只切换目标值，保留其余值和顺序，选中不重复追加，取消移除该值的全部出现。缺失值不呈现面板但不清洗外部数组；disabled 项仍按外部值呈现，只停用交互。每项以 Boolean binding 适配同一 Collapsible 实现，不复制动画和内容保留状态机。onOpenChange 只报告用户实际改变的目标项；Single 隐式关闭的旧项不另发一次用户事件。

所有可用 heading 默认参与普通 Tab；同一根用 KeyNavigation 的非 roving Vertical 规则处理 Up／Down／Home／End，跳过禁用或不可用项，loop 控制首尾环绕，只移动焦点，不改变展开值。Space／Enter 仍由各 heading 的唯一 Press 激活；正文编辑器的按键不被祖先截获。Default 各项间使用 border 分隔，最后一项不画底边；Card 每项使用 card 配色、完整圆角边框与 12 的间距。触发器纵向留白和正文尾部留白默认 10，按 body 字号／14 缩放。

验收覆盖受控 setter 拒绝、换绑和最新回调；默认卸载／KeepMounted 的工厂次数、局部状态、Props 更新、重排与嵌套；关闭途中停止输入／IME／Spinner、重开与 Reduced 热切换、零尺寸、隐藏／恢复、静止和卸载；Single／Multiple、缺失值、禁止全关、禁用跳过、键盘只导航与激活分离；真实 Gallery 组合输入、按钮和加载内容。

## 25. Tabs

`tabs(selected: Binding<T?>, configure: TabsBuilder<T>.() -> Unit = {})` 构造 TabsProps<T>／Tabs<T>，T 遵守 Equals<T>。Props 包含 entries、selected、activation=TabActivation.Auto、axis=FlexAxis.Horizontal、variant=TabsVariant.Default、retention=ContentRetention.Dispose、enabled=true、loop=true、spacing=8、fonts／theme、style=styleOverride()、listStyle=styleOverride() 与 onValueChange: (T?) -> Unit={}；builder 同名并有 key。spacing 有限非负。

TabEntry<T> 含 Choice<T>、必需 content、trigger: ButtonContent?=None、focus=FocusOptions()、retention: ContentRetention?=None、triggerStyle: ButtonStyle?=None 和 panelStyle=styleOverride()。`TabsBuilder.item(choice, content) { ... }` 收集 TabItemBuilder；trigger 的 String／StyledText／View／ActionContentStyle 回调重载设置非交互原始内容。缺省使用 Choice.label；内置 Label 未配置 fonts 时继承根字体，自定义 View 自行配置。一个 header 只有一份 Press／FocusTarget。显式 key 与 value 必须分别唯一，沿用 Choice 校验，不以 key 替代普通描述类型匹配。构造不读取 binding、不创建面板或触发动作。

Auto 通过同一个 roving KeyNavigation 在沿 axis 的方向键、Home／End 导航被接受后选择目标；Horizontal 遵守当前 RTL，Vertical 使用 Up／Down，loop 控制首尾。Manual 只导航，Space／Enter 或指针激活才选择。普通 Tab 以当前可用选中项为入口，在组内已有焦点时保留该目标；组只有一个 Tab 停靠点，随后进入当前面板的普通可聚焦内容。disabled 与不可用 header 跳过，正文编辑器不被祖先导航截获。程序聚焦或外部 selected 更新不模拟导航，也不把焦点移入面板。

选择动作按 key 读取最新 Props，禁用或值相同不写入。写入候选 Some(value) 后，只有实际值与旧值不同才调用最新 onValueChange，并传入 setter 实际采纳的可空值；拒绝 setter 保留原选择和面板，但不强制退回已移动的 header 焦点。None、缺失或 disabled 的外部值均不清洗、不自动选择第一项；缺失值不显示面板，已选 disabled 项仍显示对应面板并停用该项输入。根 enabled=false 同时停用 header 与面板输入，不改写选择。

面板默认 Dispose，未选中不挂载，切换时立即卸载旧面板；KeepMounted 从初始状态保留全部面板实例，未选中时跳过布局、输入／IME、Tab 和动画帧请求，但继续接收逻辑更新。复用 §24 的内容门控，不另建生命周期协议。切换没有高度过渡；重新显示只在 Accepted 非零几何后恢复输入，不恢复旧面板焦点。每项可覆盖 retention；改为 Dispose 立即卸载未选中的保留内容。同父级稳定 key 的重排、标题／内容 Props、主题、横纵及 Default／Line 切换保留实例与编辑状态，类型变化仍按普通规则重新挂载。

Horizontal 为 header 在上、面板在下，Vertical 为 header 在逻辑 Start、面板在 End；固定同一逻辑父级和两个槽，方向／轴变化只更新几何。列表按内容取宽，面板使用剩余可用宽度，spacing 分开两槽。Default 列表为 muted 背景、large 圆角、3 的 padding；header 默认高 26、横向 padding 6、14／20 medium 文字、medium 圆角；选中采用 background 色，Dark 使用 input 边框及 input／30 底色。Line 列表透明、项间距 4，选中条使用 foreground、厚 2，Horizontal 在下边、Vertical 在逻辑 End。尺寸按 action 字号／14 缩放。未选中文字为淡化 foreground（Dark 使用 muted.foreground），hover／选中恢复 foreground；pressed 具有可区分的反馈。hover／press 和焦点环复用 ActionInk／ThemePainter，选中条使用 ThemeMotion.selection，Reduced 热切换立即收束。局部 triggerStyle、panelStyle、listStyle、根 style 各作用于对应部分，不重叠登记输入。

验收包含 Auto／Manual、横纵／RTL、Tab 入口、禁用跳过、拒绝 setter／换绑／实际值通知、None／未知／禁用外部值；Dispose／KeepMounted 的工厂次数、状态、关闭 Props 更新、IME 与停帧；keyed 重排、轴／变体／主题切换保留编辑；零尺寸与恢复、Reduced、卸载清理；真实 Gallery 中的表单与加载内容及 Default／Line 焦点外观。

## 26. Breadcrumb

`breadcrumb(configure: BreadcrumbBuilder.() -> Unit = {})` 与 ChildrenBuilder 成员构造 BreadcrumbProps／Breadcrumb。Props 为 entries=[]、separator: View?=None、spacing=6、lineSpacing=6、enabled=true、fonts／theme、style=styleOverride()。spacing／lineSpacing 有限非负，按 action 字号／14 缩放；根使用普通 Wrap，RTL 交换逻辑起止。separator 未配置时为 14 的 ChevronEnd，可配置任意非交互 View；首项不显示分隔符，分隔符与后项同组换行。

BreadcrumbEntry 含 `content: BreadcrumbContent`、key=Position、enabled=true、focus=FocusOptions()、maxWidth: Double?=None、style: ButtonStyle?=None。BreadcrumbContent 为 Link(ButtonContent, onNavigate)、Current(ButtonContent)、Ellipsis、Custom(View)。builder 的 item(String／StyledText／View／ActionContentStyle 回调, onNavigate)、current(String／StyledText／View)、ellipsis()、custom(View) 收集条目，配置块设置其余字段；add 接收完整条目。maxWidth 有限非负，限制整项（含其前置分隔符）的逻辑宽度；内置 Label 默认单行省略，未指定字体时继承根 fonts。自定义正文自行遵守约束。

Link 使用同一个普通 Button／Press／FocusTarget，普通 Tab、Space／Enter 与指针激活只调用最新 onNavigate；根与条目 enabled 共同控制。Current、Ellipsis、默认分隔符不登记输入、无跳转；Custom 保留调用方自己的行为，可放置 Dropdown trigger 等已有控件，根 enabled 不推断或改写不透明 Custom 的内部状态。没有内置 URL 打开或路由。相同父级、key 和描述类型按普通规则复用，重排及首项改变不改变正文槽；类型变化仍重新挂载。

内置文字为 action 字号、20／14 行高、normal weight；链接默认 muted.foreground，hover／pressed 恢复 foreground，按下有轻微背景反馈，焦点环不影响布局；Current 为 foreground。Ellipsis 为 20 的容器和 16 的 MoreHorizontal 图标。所有预设尺寸按字号缩放，局部 ButtonStyle 可覆盖链接外观。验收覆盖普通／自定义分隔符、长项省略与换行、RTL、当前项无动作、最新回调、禁用、键盘与 keyed 重排；Dropdown 组合在菜单批次交付后接入同一 Custom 槽。

## 27. Pagination

`pagination(page: Binding<Long>, pageCount: Long, configure: PaginationBuilder.() -> Unit = {})` 与 ChildrenBuilder 成员构造 PaginationProps／Pagination。页码从 1 开始；pageCount 可以为 0，pageCount／siblings=1／boundaries=1 必须非负。`paginationItems(page, pageCount, siblings=1, boundaries=1): Array<PaginationItem>` 是实际控件和调用方共用的纯范围计算，PaginationItem 为 Page(Long)／Ellipsis。显示首尾 boundaries 项及当前页两侧 siblings 项，重叠去重；只缺一页时直接显示该页，多页缺口显示省略号。未知外部 page 只在计算窗口时夹到有效范围，不写回 binding；空数据没有页码。算术覆盖 Long 上界，不遍历被省略的页。

其余 Props 为 items: Array<PaginationItem>?=None、enabled=true、showPrevious=true、showNext=true、compact=false、previousText="Previous"、nextText="Next"、pageLabel: (Long)->String、pageContent: ((Long, ActionContentStyle)->View)?=None、ellipsis: View?=None、fonts／theme、buttonStyle=buttonStyle()、style=styleOverride()、previousFocus／nextFocus=FocusOptions()、pageFocus: (Long)->FocusOptions、onValueChange: (Long)->Unit。None items 使用自动范围；Some 使用显式顺序，Page 必须在 1..pageCount 内且不重复。builder 另有 key，item(page)／ellipsis() 追加显式条目，ellipsis(View) 设置省略号正文；add 接收完整 PaginationItem。构造和 DSL 不读取 binding、不调用业务回调或内容工厂。

数字按钮用 Ghost／Icon，实际有效选中页用 Outline；Previous／Next 用 Ghost／Default、逻辑方向的 Chevron 和可本地化文字，compact 只隐藏文字。缺省省略号是 32 的非交互容器及 16 的 MoreHorizontal 图标。列表间距 2，使用普通 Row／Button，有限宽度居中，无界宽度取内容宽度；父约束优先，不创建第二套键盘导航或命中。逐页 key 来自页码，边缘按钮有独立稳定 key；页码窗口变化保留仍存在的按钮实例和焦点。根 style 和 buttonStyle 分别作用于整体及按钮，预设尺寸按 action 字号缩放，RTL 镜像布局和方向图标。

动作读取最新 Props／binding。Previous 在 page<=1 时不可用，page>pageCount 时指向最后一页；Next 在 page>=pageCount 时不可用，page<1 时指向第一页；空数据、根 disabled 时两者不可用，计算避免溢出。数字或边缘动作只提交有效且不同的候选；setter 拒绝时不伪造选择，实际改变后通知 setter 采纳的 Long。外部值与 pageCount 更新不主动修正页码、不合成通知，控件不取数据。普通 Tab、Space／Enter 和指针走原 Button，禁用／卸载／隐藏取消在原层收尾。

验收覆盖空数据、单页、首尾、缺口合并、Long 上界、显式条目错误、拒绝 setter／换绑／实际值通知、动态 pageCount、局部状态与焦点保留、RTL／窄宽／compact、明暗主题及真实应用页码驱动的数据视图。

## 28. Popover

`popover(open: Binding<Boolean>, content: View, configure: PopoverBuilder.() -> Unit = {})` 与 ChildrenBuilder 同名成员构造 PopoverProps／Popover。完整内容、受控 open 为必需参数。Props 包含 trigger: PopupTriggerSpec?=None、anchor: LayoutAnchor?=None、title／description: PanelContent?=None、width=288、side=Bottom、alignment=Center、gap=4、viewportPadding=8、matchAnchorWidth=false、anchorVisibility=Close、mode=NonModal、focus=OverlayFocusOptions()、handle=None、enabled=true、dismiss: OverlayDismiss?=None、transition: StyleValue<AnimationSpec?>=Inherit、onOpenChange、onCloseRequest、onClosed、fonts／theme、style。必须提供 trigger 或外部 anchor；width／gap／viewportPadding 有限且非负。builder 增加 key，并继承 PanelBuilder 的 title／description／fonts／theme／style 配置。

PopupTriggerSpec 保存 ButtonContent、enabled=true、variant=Outline、size=Default、style=buttonStyle()、focus=FocusOptions()、fonts=None。trigger(text／StyledText／View) 块配置上述按钮选项，View 是单个按钮的非交互正文。内置 trigger 沿用 Button 的 Press、Tab 与焦点合同，以最新 Props 切换绑定；setter 真实采纳不同值后才通知 onOpenChange。外部更新不合成通知。anchor=None 时用 trigger 的真实场景矩形；外部 anchor 使用公共 LayoutAnchor，触发器可省略，关闭不需要私有节点或自存窗口坐标。根布局固定 trigger／Portal 两槽，开闭不重建触发器。

内容以 Portal 呈现，默认非模态、明确初始／恢复焦点；初始 None 选第一个 Tab stop。NonModal／FocusContained 默认 Escape 与外部 CloseAndContinue，Modal 默认 CloseAndConsume，两者默认 includeAnchor=true，避免触发器按下先关再开；完整 dismiss 可覆盖；无效 Modal＋Continue 沿 Portal 报错。关闭请求先经过业务 veto，再提交绑定；拒绝 setter 不伪造关闭。enabled=false 禁用触发器和内容输入，不改变 open。Presence 默认 100ms，按真实定位边做 8 的短位移、0.95 到 1 缩放与淡入淡出；Reduced Motion 即时结束。退出立即停用内容输入，中途重开保留内容与进度；最终移除释放内容。焦点保存、恢复、嵌套关闭及三种模式沿 UI §7.6，不另建键盘或焦点处理器。

默认内容宽 288、padding 与主体 spacing=10、corner=8、popover 配色、foreground 10% 的 1 边框及 floating 阴影；Header 的 title／description 间距 2，title 为 action，description 为 muted body。尺寸按字号缩放并服从父级／视口约束；局部 style 覆盖面板。所有文字槽和自定义正文可组合输入、表单与按钮。验收包含可编辑内容、普通／仅焦点约束／模态、Tab 与 Escape、外部继续／消费、veto／拒绝绑定、最新回调、退出重开／资源释放、锚点翻转与裁剪、RTL、明暗 System／Neutral、Reduced Motion 与窄窗口。
