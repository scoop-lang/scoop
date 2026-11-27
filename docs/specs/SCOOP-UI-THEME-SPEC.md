# Scoop 公共主题绘图与几何规范

日期：2026-11-03。状态：公共主题协议、四类部件几何／绘图、三套风格及 Light／Dark、基础控件消费和完整展示已在 macOS aarch64 交付并完成 x86_64 补验，验证范围见 [UI 实施记录](../ui/PROGRESS.md)。x86_64 使用 Apple Silicon 上的 Rosetta，Windows 后续。本文扩展 [公共样式规范](SCOOP-UI-STYLE-SPEC.md)，保留其内容效果、资源、稳定身份与停帧合同。

2026-11-04：完整语义配色、显式减少动态与第四套 shadcn 预设已完成 macOS aarch64 定向验证，见 [shadcn 实施记录](../ui/shadcn/PROGRESS.md)。这一扩展的 x86_64 验证留在控件目录收尾。

## 1. 本轮调整与分层

三套主题同时用于 macOS 和 Linux GNU，不按显示协议分叉。Linux 使用同一几何、Light／Dark 色板和效果实现，在 Wayland／X11 的 GLES 目标上呈现；主题指定的字体不存在时，使用 graphics.text 的系统字体回退。操作系统未提供明暗偏好时保留未知，由应用已有的主题策略处理。

主题以公共绘图部件及其几何为核心。`scoop.ui.style` 定义 Theme／ThemePainter 协议、语义颜色／字体／尺寸／动画参数、类型化视觉状态、尺寸输入、几何结果，以及通用 Style／装饰适配。`scoop.ui.component` 负责持久状态、布局组合、输入与业务，不定义公共主题合同，也不依赖某个主题实现。

本轮明确替换此前三项设计：

1. 不再把公共主题及默认颜色／尺寸都放在 ComponentDefaults 或控件专用样式中；它们由 style 的公共 Theme 提供。
2. Button／Checkbox 不再直接用自己的一套背景、圆角、焦点和标记绘图。它们消费主题的公开几何与绘图；ButtonStyle／CheckboxStyle 只表达明确的局部覆盖和控件特有输入。
3. 控件不在构造时永久保存当时的 Theme 或解析后的外观。保存稳定 ThemeSource，在 render 中读取其当前 Theme，并把本轮主题及视觉输入通过 Props 交给持久 Element。

五组实现为普通独立 Cone：`scoop.ui.style.macos`、`scoop.ui.style.material`、`scoop.ui.style.fluent`、`scoop.ui.style.shadcn`、`scoop.ui.style.liquidglass`。它们依赖 style 和实际使用的 graphics／graphics.text／surface，不依赖 component；style 不反向引用主题库。应用选择并依赖所需主题。`scoop.ui` 仍不依赖 style 或 component。

五套主题的生产源码不使用 OS 条件。macos／fluent／material／shadcn／liquidglass 是自绘风格名称；字体可有偏好列表，但以 graphics.text 的通用 family fallback 收尾，缺少某个系统专有字体不要求上层选择平台。

实现库可复用公共的几何、StylePainter 和普通绘图辅助代码。共同算法按实际调用复用，差异由各自的主题参数或绘图实现表达；控件中不得按主题类型或名称分支。自绘主题不替换 window 的原生装饰配置，也不声称与操作系统某个版本逐像素一致。

## 2. 主题对象、绘图对象与普通数据

Theme 是不可变的主题描述／协议实现，提供语义参数和各部件的 metrics／geometry 方法；不拥有需要应用逐次关闭的 Path／滤镜，不读取业务 State。ThemeSource 和 UI 消费适配与这些纯几何／绘图入口分开。

几何方法只接收有限的逻辑 bounds、尺寸选项和类型化视觉数据，不需要 Canvas，不访问 Application、InputArea、挂载节点或 Context。每类部件返回自己的结果类型，不用名称字典或通用无类型字段。绘制严格消费该结果中的尺寸、轮廓和区域，不能重新估算另一份布局。

首批采用 `Theme.tokens`、`surfaceMetrics／checkboxMetrics／radioMetrics／scrollbarMetrics` 与对应 geometry 方法；各部件的 Options 表达显式尺寸和局部覆盖。几何结果同时保存本次解析的普通 Style／颜色快照，绘图使用这份结果，不重新解析另一套尺寸。InteractionVisual 的可选 hover／press 采样值和选择进度以 0～1 表达，省略时使用对应布尔视觉状态；有限输入收敛到该范围。改变采样值时重新计算普通几何描述，原生轮廓缓存仍只按真实轮廓变化替换。

五套预设共同使用 `ShapeTheme` 的圆角表面、勾选线段、圆环／圆点和滚动条几何算法，各库显式构造自己的参数。Theme 协议仍允许普通第三方实现独立的几何与 ThemePainter；控件不依赖 ShapeTheme。复用只覆盖这四类已有部件，不引入主题插件、节点注册或自动控件生成。

原生缓存由 `ThemePainter : Closeable` 持有，通过 `Theme.createPainter()` 创建；其 drawSurface／drawCheckbox／drawRadio／drawScrollbar 只接收普通 Canvas、对应几何和视觉数据。这样离屏程序可以长期复用同一 painter，自绘 Element 则通过 ComponentScope 登记拥有可替换 painter 的 `ThemePainterCache`。cache 在主题实际变化时先构建新 painter，再替换并关闭旧值；创建失败保留旧值用于清理或下一次绘制。应用不为每个部件／每帧使用 use。

ThemePainter 和缓存限创建线程使用，close 幂等。ThemePainterCache.clear 释放当前 painter 并回到可复用的空状态，供同一个 Element 从主题表面切换为普通装饰；close 则结束整个缓存的有效期。绘图同步借用 Canvas，退出后不能保存该画布；正常及异常路径恢复嵌套状态。Theme／几何只是普通描述，不替代实际资源所有者。普通离屏用法为先查询 geometry，再把同一 geometry 交给对应 painter 的 draw 方法；不要求运行 UI。

UI 整改 P5 的场景缓存与像素缓存遵守 UI 规范第 7.2 节。尺寸和放置从完整布局结果取得；颜色、透明度及其他动态采样仍属于当前 scene。普通主题绘图继续使用 draw，不为缓存而引入不需要的合成隔离。有限绘图记录的版本由实际内容拥有者维护，必须覆盖其字体、资源和动态采样；不能只凭 Theme 或 Props 的引用身份宣称输出未变。借用 Path／滤镜、原始绘图和帧通知所在节点继续执行，稳定的子场景可在显式边界内复用。

## 3. 通用参数与覆盖

Theme 提供以下公共语义参数，使没有标准绘图部件的第三方控件也能选用相同外观：

- 颜色：ThemePalette 的 background、card、popover、primary、secondary、muted、accent、destructive 都是完整的 `ColorPair(background: Color, foreground: Color)`。primary 表达强调操作；accent 表达候选／悬浮表面。border、input、ring、disabledText、shadow 是独立 Color。chart 是 `ChartPalette(one, two, three, four, five: Color)`；sidebar 是 `SidebarPalette(background, primary, accent: ColorPair, border, ring: Color)`。没有可空颜色或运行时补全。旧 canvas／text 迁到 background，surface 迁到 card，elevated 迁到 popover，accent／onAccent 迁到 primary，secondaryText 迁到 muted.foreground，focus 迁到 ring，error 迁到 destructive.background；旧字段不保留别名。
- 字体：正文、辅助文字和操作标签，直接使用 graphics.text 的 TextStyle；局部字体／字号／颜色覆盖复用 SpanStyle，段落设置继续使用 ParagraphStyle。
- 尺寸：基本间距、各部件建议／最小尺寸、内容内缩、边框宽度、圆角、焦点外扩和 Scrollbar 的实际度量。
- 效果：表面外框阴影和必要的 Style 内容效果参数。
- 动画：进入／离开状态和选择变化的 Surface AnimationSpec。主题只提供参数，绘图方法只消费本帧已采样的数值。

`ThemeMode.Light` 与 `ThemeMode.Dark` 只表达明暗；风格由所选实现独立决定。五套主题同时提供两种模式，不把 Dark 当成另一套风格，也不因切换模式改变组件类型。需要跟随系统的应用在现有 Window ThemeChanged 通知中更新 ThemeSource，主题库不轮询系统、不自行建立窗口监听或时钟。

Style 是完整的已确定描述；新增 `StyleOverride` 表达局部覆盖。未指定属性继续使用本轮主题，背景／边框等可空属性须区分“继承”和“显式清除”，采用类型化 `StyleValue<T>` 的 Inherit／Set，而不是用一个 None 混淆两者。数组效果显式覆盖时整体替换，顺序不变；需要组合时由调用者明确给出顺序。

控件局部样式和主题默认值在本轮几何计算前解析。圆角、边框内缩、padding 或建议尺寸的覆盖必须进入同一份最终几何；不能只改变画出的尺寸，却让布局或拖动继续用主题默认值。纯阴影／glow／blur 仍不改变布局尺寸、基线或命中区域。

### 3.1 部件选项的接收者配置

部件选项通过 `surfaceOptions(configure: SurfaceOptionsBuilder.() -> Unit = {})`、`checkboxOptions(configure: CheckboxOptionsBuilder.() -> Unit = {})`、`radioOptions(configure: RadioOptionsBuilder.() -> Unit = {})`、`scrollbarOptions(configure: ScrollbarOptionsBuilder.() -> Unit = {})` 构造，分别返回对应不可变 Options。每类 Options 提供 `copy(configure: 对应Builder.() -> Unit = {})`，基于当前完整覆盖值派生新值；值与 builder 的构造器内部化。

| Builder | 全部可写属性与默认值 |
| --- | --- |
| SurfaceOptionsBuilder | suggestedSize: Size? = None；minimumSize: Size? = None；padding: Insets? = None；style: StyleOverride = styleOverride()；focus: OutlineStyle? = None；validation: OutlineStyle? = None |
| CheckboxOptionsBuilder | size: Double? = None；minimumSize: Double? = None；markInset: Double? = None；style: StyleOverride = styleOverride()；markColor: Color? = None；focus: OutlineStyle? = None；validation: OutlineStyle? = None |
| RadioOptionsBuilder | size: Double? = None；minimumSize: Double? = None；dotInset: Double? = None；background: StyleValue<Paint?> = Inherit；border: StyleValue<BorderEdge?> = Inherit；dotColor: Color? = None；focus: OutlineStyle? = None；validation: OutlineStyle? = None |
| ScrollbarOptionsBuilder | thickness: Double? = None；minimumThumbLength: Double? = None；trackInset: Double? = None；thumbInset: Double? = None；buttonLength: StyleValue<Double?> = Inherit |

普通可选字段的 None 继续表示使用当前主题；Radio 的 background／border 和 Scrollbar 的 buttonLength 保留 Inherit、Set(None)、Set(value)。这三个覆盖字段各提供同名 Unit 设置方法，把同一槽写为 Set(value)；显式赋入 Inherit 恢复继承。

SurfaceOptionsBuilder／CheckboxOptionsBuilder 提供真实成员 `style(configure: StyleOverrideBuilder.() -> Unit): Unit`。它基于当前 style 修改，重复块累积、空块不重置；给 style 属性赋入完整值表示替换。嵌套和外层配置都同步执行一次，结束后修改被捕获的 builder 不改变结果。回调异常不发布部分结果；选项构造不解析某个主题、不创建 native 资源，尺寸与颜色仍在实际 geometry／metrics 边界检查。

### 3.2 完整主题与部件外观的接收者配置

五套预设的入口为 `macosTheme(mode: ThemeMode = ThemeMode.Light, palette: ThemePalette? = None, configure: ShapeThemeBuilder.() -> Unit = {}): ShapeTheme`，materialTheme／fluentTheme／shadcnTheme／liquidGlassTheme 使用相同签名。palette 为 None 时使用风格预设配色；提供 palette 时，文字、表面、标记、滚动条、焦点／验证与效果颜色从其语义角色生成，保留该风格的字体、尺寸、轮廓、阴影几何与动效。Fluent 的预设状态使用官方 token；外部配色的 hover／pressed 从所给语义色派生，不遗留固定品牌蓝。先产生完整数据，再同步执行 configure 一次。返回的 ShapeTheme 继续实现 Theme；第三方 Theme 实现不需要采用这些 builder。

独立主题使用 `shapeTheme(tokens: ThemeTokens, surfaces: SurfaceParts, checkbox: CheckboxPart, radio: RadioPart, scrollbar: ScrollbarPart, configure: ShapeThemeBuilder.() -> Unit = {}): ShapeTheme`。五个必需参数都是完整描述，不使用未初始化字段、None 或虚构部件补足必需数据。

下列入口也接收尾随 `configure: 对应Builder.() -> Unit = {}` 并返回对应不可变值。表中的必需参数位于 configure 之前；其余可写属性从列出的领域默认值开始。

| 工厂 | 必需参数 | 其他可写属性与默认值 |
| --- | --- | --- |
| themeTypography | body／caption／action: TextStyle | input: TextStyle 初始取 body；可独立配置，表示可编辑正文 |
| themeEffects | 无 | raised／floating: Array<BoxShadow> = []；emphasis: Array<ContentEffect> = [] |
| themeMotion | 无 | hover／press／selection: AnimationSpec? = None；preference: MotionPreference = Full |
| themeTokens | palette: ThemePalette；typography: ThemeTypography；spacing: ThemeSpacing；corners: ThemeCorners；validation: OutlineStyle | mode: ThemeMode = Light；motion: ThemeMotion = themeMotion()；effects: ThemeEffects = themeEffects()；disabledOpacity: Double = 1.0；textField: TextFieldTreatment = Standard |
| surfacePart | metrics: SurfaceMetrics；fill／border／foreground: StateColors；focus: OutlineStyle | shadows: Array<BoxShadow> = []；hoverShadows: Array<BoxShadow>? = None |
| surfaceParts | plain／action／outlinedAction／elevated: SurfacePart | 无 |
| markerAppearance | empty／selected／border／selectedBorder／mark: StateColors；focus: OutlineStyle | shadows: Array<BoxShadow> = []；stateLayer: MarkerStateLayer? = None |
| checkboxPart | metrics: CheckboxMetrics；appearance: MarkerAppearance | mixedAppearance: MarkerAppearance? = None；mixedShape: MixedMarkShape = Bar |
| radioPart | metrics: RadioMetrics；appearance: MarkerAppearance | 无 |
| scrollbarPart | metrics: ScrollbarMetrics；track／thumb／button／arrow: StateColors | arrowWidth: Double = 1.5 |

每类 Builder 的全部可写属性就是对应值的字段，类型与表中参数／属性一致；ShapeThemeBuilder 的五个属性对应 shapeTheme 的必需参数。对应值均提供 `copy(configure: 对应Builder.() -> Unit = {})`，值和 builder 的构造器内部化。工厂和 copy 共用配置及快照实现。

ThemePalette 的上述十五个字段均为必需数据，没有可适用于任意主题的缺省配色。它保留一次给齐这些字段的完整值构造，并提供 `copy(configure: ThemePaletteBuilder.() -> Unit = {}): ThemePalette`；builder 的同名可写字段从现有配色初始化。ColorPair、ChartPalette、SidebarPalette、StateColors、ThemeSpacing、ThemeCorners、OutlineStyle 和各 Metrics 使用普通值构造。

`shadcnTheme(mode: ThemeMode = Light, palette: ThemePalette? = None, configure: ShapeThemeBuilder.() -> Unit = {}): ShapeTheme` 遵循相同的完整描述及配置规则。palette=None 使用 Base Nova／Neutral，其颜色从上游 OKLCH 转为 sRGB，并保留 alpha；预设源码记录源 token。显式 palette 是完整语义配色，所有部件及文字从它生成，保留 shadcn 的尺寸、布局和动效；configure 最后覆盖。默认正文／操作文字为 14／20 逻辑单位、操作 weight 500；基准圆角 10，small／medium／large 为 6／8／10；常规表面边框 1、键盘焦点环 3 且 ring alpha 0.5。操作表面建议高 32、水平 padding 10；输入使用独立 input 边框；Checkbox 标记 16、圆角 4。所有长度均为逻辑单位，DPI 沿现有目标接入转换。其余 Button 变体在控件规范中定义，不增加按控件名查询的主题接口。

macOS 上 `scoop.ui.style.macos.macosSystemPalette(mode: ThemeMode = Light): ThemePalette` 在主线程读取 AppKit 的系统语义色快照，按指定 Aqua／Dark Aqua 外观解析为 sRGB 并保留 alpha。背景、文字、控件、强调、选择、分隔线、焦点、禁用和图表颜色取自对应 NSColor；shadow 使用系统阴影色及主题所需透明度。返回普通不可变配色，不持有原生资源、监听器或全局主题状态；读取失败报告普通状态错误。该平台适配入口只在 macOS 提供，原有可跨平台使用的 macosTheme 预设不变。应用可将快照独立传给任意风格工厂或其他配色消费者，并在系统外观变化或窗口重新获得焦点时重新读取和发布 ThemeSource；不逐帧调用取色。

ThemeMotion 增加 `preference: MotionPreference = Full`（Full／Reduced）。`resolve(spec: AnimationSpec?): AnimationSpec?` 在 Reduced 时返回 None，其余返回原参数；控件在解析局部 transition 后调用，切换 Reduced 时结束当前过渡。该配置是应用显式偏好，不表示已接入 OS 设置。

嵌套成员均为真实成员并返回 Unit：

| Builder | 嵌套成员及 receiver |
| --- | --- |
| ShapeThemeBuilder | tokens: ThemeTokensBuilder；surfaces: SurfacePartsBuilder；checkbox: CheckboxPartBuilder；radio: RadioPartBuilder；scrollbar: ScrollbarPartBuilder |
| ThemeTokensBuilder | palette: ThemePaletteBuilder；typography: ThemeTypographyBuilder；motion: ThemeMotionBuilder；effects: ThemeEffectsBuilder |
| ThemeTypographyBuilder | body／caption／action／input: TextStyleBuilder |
| SurfacePartsBuilder | plain／action／outlinedAction／elevated: SurfacePartBuilder |
| CheckboxPartBuilder／RadioPartBuilder | appearance: MarkerAppearanceBuilder |
| CheckboxPartBuilder | mixedAppearance: MarkerAppearanceBuilder（从当前覆盖或 appearance 派生） |

所有嵌套配置基于当前字段调用其 copy，重复块累积，空块保留已有设置；直接给属性赋完整值表示替换。模式参数只负责选择预设起点；之后编辑 palette、typography 或某个部件，就只改变该显式字段，不隐式重新计算其他已经生成的文字颜色或部件外观，也不重放回调。需要统一改变多个描述时，由同一配置块明确设置它们。

构造仅发布普通描述，不读取 Context／State、不创建 painter／Path／字体资源或动画；回调异常沿普通调用传播，原值不变。Array 元素槽与子描述不可变，可安全共享；发布后修改被捕获的 builder 不改变结果。颜色、长度与视觉采样继续在原有实际消费边界检查，不增加另一套布局或绘图验证流程。

### 3.3 物理与逻辑几何

Insets(left, top, right, bottom) 始终表示逻辑长度单位中的物理四边。LogicalInsets(start, top, end, bottom) 是独立值类型；resolve(LayoutDirection) 先把 start／end 映射到左右，再返回已检查的 Insets。两者的 all(value) 生成四边相等的值，长度及总和有限非负。主题和通用几何消费已解析的物理 Insets；graphics 不依赖 UI 方向。

Surface／Checkbox 的 metrics 与 geometry 方法接收最后一个 direction: LayoutDirection = Ltr 参数。组件测量和 scene 传入同一有效方向，先解析 Style 的逻辑边框和圆角。SurfaceMetrics／CheckboxMetrics 保存 borderInsets: Insets 与 corners: CornerRadii；不保留原先单一 borderInset／cornerRadius 字段。contentInsets 逐边相加，geometry 使用 graphics.RoundedOutline.normalized／inset 计算相同的外框和内框。Geometry.style 保存本次已经解析为物理值的 Style，ThemePainter 不再推测方向。

公共 RoundedOutline 统一归 graphics，包含 bounds 和 radii。背景、边框、内容裁剪和焦点环复用这份几何；OutlineRing 的 border 使用一个真正均匀的 BorderEdge，焦点外框由 expanded(outset) 得到。Radio 的环宽和 Scrollbar 的统一半径仍按其实际形状语义表示；RadioOptions.border 使用可清除的 BorderEdge 覆盖，不为圆环引入四条独立边。

## 4. 公共绘图部件

各部件的 InteractionVisual 使用明确的 enabled／hovered／pressed／focusVisible／invalid 数据，部件另带自身选择或滚动输入。focusVisible 由统一焦点合同提供，不用逻辑 focused 推测；disabled 时不绘制焦点环。它们都是值描述，不是响应式 State；不存在组件回调、输入注册、任务或业务模型引用。

`mixThemeColor` 在预乘 alpha 的 sRGB 通道数值上插值，alpha 同步线性插值，再还原为普通 Color；两端均透明时返回透明色，进度端点保留原值。StateColors 依次混合 normal／hovered 和 pressed，禁用直接使用 disabled。透明色中不可见的 RGB 不能在过渡中造成先变暗再变亮的闪色。

SurfacePart.hoverShadows 为 None 时沿用 shadows；Some([]) 显式清除悬停阴影。提供覆盖时，启用表面按 hoverProgress × (1 − pressProgress) 在两组投影之间淡入淡出，未提供采样时使用布尔状态；禁用或按下的稳定状态回到 shadows。最终阴影保存到 SurfaceGeometry.style，普通 StyleOverride.boxShadows 仍可覆盖。高对比度或减少透明度移除两组默认投影。

MarkerAppearance.stateLayer 使用 `MarkerStateLayer(size: Double, empty: StateColors, selected: StateColors)` 描述标记背后的圆形交互底层，None 表示无此效果。size 为有限非负直径，实际直径不小于标记外框；颜色沿选择进度混合 empty／selected 各自解析的交互色，禁用时不绘制。invalid 时保留解析 alpha 并使用验证颜色。CheckboxGeometry／RadioGeometry 的 `stateLayer: MarkerStateLayerGeometry? = None` 保存最终 `circle: Circle` 和 `color: Color`，ThemePainter 先画状态层再画标记，不从 visual 重算尺寸或颜色。配置状态层时，普通焦点环围绕该圆形轮廓；Checkbox Options 显式覆盖 corners 时，焦点改用已经解析方向的标记轮廓，以保留局部形状覆盖。仅覆盖 focus 的颜色、宽度或外扩不会改变主题默认轮廓。验证环始终围绕标记本身。metrics.focusOutset 包含状态层及最终焦点所需的外扩，标记本体不扩张；控件使用该度量预留外围布局空间，主题本身不创建布局节点或输入区域。高对比度／减少透明度隐藏此装饰但保留轮廓尺寸和焦点；全程复用已有动画采样，不创建任务或时钟。

`contrastThemeColors(pair: ColorPair, canvas: Color, minimum: Double = 7.0): ColorPair` 复用主题偏好的颜色配对算法，供实际底色不同于根背景的菜单、弹出层等默认装饰使用。颜色通道须有限，canvas 必须不透明，minimum 为有限的 1～21，非法参数抛出 IllegalArgumentException。计算按标准 sRGB 将颜色通道与源 alpha 截取到 0～1，先把 pair.background 合成到 canvas，再将前景向黑或白调整到指定相对亮度对比度；只有背景本身无法容纳该对比度时才调整背景。返回完整、不透明且满足 minimum 的配对；已经满足要求的颜色保留。它不读取系统偏好或改写主题，应用显式覆盖继续由调用方决定。

`OutlineStyle(color, width, outset, borderColor: Color? = None)` 描述一个环及可选的外框颜色替换；None 表示保留部件自身边框，width=0 可省略环。它替代仅限焦点命名的旧描述，普通焦点仍通过部件及 Options 的 focus 配置。ThemeTokens.validation 是完整必需的 OutlineStyle，Surface／Checkbox／Radio Options 的 validation 可局部覆盖。五套主题显式提供验证外观；不根据具体主题名称分支。

invalid 与 focusVisible 独立，前者是应用传入的展示状态，不改变焦点、输入资格或业务值。invalid 时选择 validation；否则只有 enabled 且 focusVisible 才选择 focus。禁用不显示普通焦点环；invalid 边框与环和控件一起合成，不单独再次降低 alpha。状态边框只替换颜色，保留同一份边宽，随后应用显式局部 Style 覆盖。SurfaceGeometry／CheckboxGeometry／RadioGeometry 的 ring: OutlineRing? 保存最终唯一的环，painter 直接绘制这个结果，不再自行重判焦点；invalid 与焦点不叠画两层环。metrics.focusOutset 取配置中最终焦点轮廓、验证环及标记状态层所需外扩的最大值，状态切换不改变内容布局。

shadcn 的焦点环替换边框为 ring token；验证边框为 destructive，Dark 使用 0.5 alpha。验证环宽及外扩为 3，Light／Dark 的 destructive alpha 分别为 0.2／0.4；其他三个主题保留原输入错误颜色和 2 宽、2 外扩、0.72 alpha 的基线。

ThemeTokens.disabledOpacity 控制禁用控件的整体合成透明度，有限值按 0～1 截取，默认 1.0 保留部件的 StateColors.disabled 配色。SurfaceGeometry／CheckboxGeometry／RadioGeometry 的 opacity: Double = 1.0 保存本帧解析值；启用时始终为 1。Surface 的 All 和 Checkbox／Radio painter 将整个绘制作为一层；使用 Surface Background／Foreground 的控件在自己的内容两侧包同一个合成层，两阶段不能各自重复应用 opacity。Scrollbar painter 将禁用透明度与本帧 visual.opacity 相乘。shadcn 为 0.5，并提供未衰减的禁用颜色；其他三个主题保持 1.0。透明度不改变测量或输入资格。

shadcn Plain 表面的禁用背景使用 input 颜色的 0.5／0.8 倍 alpha（Light／Dark），随后与内容一起应用 disabledOpacity。

shadcn Action 表面的 normal 使用 primary 背景，hover／pressed 分别使用其 0.8／0.65 倍 alpha，提供可区分的悬停与按下反馈；普通输入使用的 Plain 表面不因此增加动作配色。

### 4.1 外框／表面

SurfaceMetrics 给出建议／最小尺寸、内容 padding、边框内缩及焦点外扩。SurfaceKind 的 Plain／Action／OutlinedAction／Elevated 分别表达普通输入表面、填充操作面、描边操作面和提升表面，不引用具体控件类型。描边操作必须有独立部件，避免与输入框共享胶囊圆角、强调文字或状态底色；五套预设均显式提供这四种表面。SurfaceGeometry 至少返回外边界、圆角外轮廓、边框内区域、内容区域和可选焦点轮廓；轮廓为 Rect／圆角等普通描述，原生 Path 留在 painter。

Surface 的绘制支持 Background／Foreground／All 阶段。Background 绘制外框阴影和背景；Foreground 绘制边框及焦点外框；All 依次完成两者，供独立展示使用。控件在两阶段间绘制内容，完整子树的 Style 效果仍由通用装饰适配负责。这样背景、内容、边框的顺序与既有 Style 合同一致，不要求主题方法理解挂载树。

SurfaceGeometry.contentStyle 保留同次解析的内容效果、圆角和 clip；需要内容的调用者在 Background／Foreground 之间通过自己的 StylePainter 应用它。All 没有内容子树，不把内容滤镜偷换为外框滤镜。Checkbox／Radio 标记绘制可把自身标记作为 Style 的内容，外框装饰与标记效果仍遵守相同顺序。

Button 根据主题建议尺寸、自己的标签 metrics 和内容内缩决定布局，然后用最终 bounds 计算几何；Frame 可组合相同外框能力。基线从实际子项加放置偏移得到，不由主题猜测字体基线。控件可把标签或留白纳入命中范围，焦点光晕不会自动扩大输入。

#### 4.1.1 玻璃表面用途

`SurfaceRole` 与外观种类 SurfaceKind 独立，取值为 Content／Control／Input／Navigation／Toolbar／Popover／Dialog／Notification。`SurfaceVisual(kind=Plain, interaction=InteractionVisual(), role=Content)` 传递实际用途；同一种圆角或颜色不意味着相同用途。

`ThemeTokens.glassSurfaces: GlassSurfaces` 默认为全部关闭。`glassSurfaces(configure: GlassSurfacesBuilder.() -> Unit = {})`、copy 和 `ThemeTokensBuilder.glassSurfaces { ... }` 遵守不可变描述与嵌套快照规则。GlassSurfaces 的 content、control、input、navigation、toolbar、popover、dialog、notification 均为 `GlassMaterial? = None`，`get(role)` 返回对应材质；None 明确表示该用途不采用玻璃，不自动扩散到其他用途。

主题按用途选择材质，然后应用 SurfaceOptions.style.glass 的继承、替换或显式清除。有效材质存在时，默认填充作为透光的语义染色；显式 background 覆盖仍由调用方决定。显式清除材质会恢复原默认填充。几何、内容排版及输入范围不因选择材质改变。减少透明度或高对比度偏好将主题策略解析为全部关闭；恢复偏好从基础主题恢复选择。应用显式 Style.glass 仍属于局部自定义。

默认玻璃表面根据当前状态的实际前景计算可读性底色，不使用固定低透明度替代普通填充。`glassSurfaceFill(pair: ColorPair, canvas: Color, minimum: Double = 4.5): Color` 返回供材质之后绘制的语义染色；canvas 必须不透明，输入颜色通道有限，minimum 为有限 1～21。先将原背景合成到 canvas，以原前景在该背景上的对比度与 minimum 的较小值为目标，保持文字色与状态区分。利用下层 sRGB 通道在 0～1 内的范围，求足以覆盖任意底图的染色 alpha；不采样屏幕、不依赖上一帧。原配对已低于 minimum 时不改写调用方文字色，但不能因玻璃而进一步降低其对比度。像素量化保留必要余量。

公共 Surface 使用当前 StateColors 前景；输入表面还覆盖默认 placeholder 等次要文字。Card、弹层、通知、导航与工具栏按实际正文／次要文字选择较强的底色，操作变体按其实际文字颜色选择。Hover／pressed／disabled／focus／invalid 继续使用原有状态采样、边界及语义；带内容的整体淡化组使用 saveBackdropLayer，避免禁用时隔断玻璃与下层。禁用的整体透明度仍由主题表达，高对比度则沿 §6.1 去掉该透明度并满足原有对比度要求。

高对比度／减少透明度解析后不会重新引入默认玻璃；偏好热更新保持控件与焦点身份。减少动画立即收束已运行的形变和状态过渡，文字缩放走真实布局。直接 Style.glass、GlassEffectContainer 的显式材质、自定义内容颜色与显式 background 继续由应用配对；容器的实体回退、键控内容语义和命中边界遵守组件规范 §6.8。

### 4.2 Checkbox 标记

CheckboxVisual 包含 `CheckMark.Off／On／Mixed` 与交互视觉状态；可选采样参数控制标记显现等外观，主题不决定何时切换。CheckboxMetrics 给出建议／最小标记尺寸、内缩与焦点外扩。CheckboxGeometry 明确标记外框、内部区域、勾选／混合标记使用区域和焦点轮廓。

绘制仅根据输入显示空框、勾选或混合标记及其状态外观，不读取 Checkbox.checked、不调用 onChange。CheckboxPart.mixedAppearance 是半选状态的可选独立外观，None 继承 appearance；CheckboxPartBuilder.mixedAppearance { ... } 从当前覆盖或 appearance 派生，完整赋值 None 恢复继承。MixedMarkShape.Bar 使用原来的圆头横线，Square 使用 markBounds 内的方形；二者直接保存为 CheckboxGeometry.mixed 的最终轮廓。焦点留白取普通／半选外观与验证环的最大外扩，切换三态不改变布局；高对比度／减少透明度同时解析两种外观。菜单、树节点、表格和第三方自绘可直接使用同一能力。Checkbox 控件的 Boolean／CheckState Binding 与三态切换遵守 [控件规范第 5 节](SCOOP-UI-COMPONENT-SPEC.md#5-checkbox)，主题只消费对应标记。

### 4.3 Radio 标记

RadioVisual 包含 selected 与交互状态及可选采样参数。RadioMetrics 给出建议／最小尺寸、环宽、圆点内缩与焦点外扩；RadioGeometry 返回外圆、内圆／圆点区域和焦点轮廓。圆用中心、半径或等价的普通矩形描述，实际绘制和控件命中可复用同一结果。

主题不建立分组、互斥状态或键盘导航。首批交付可独立调用的 Radio 绘图，完整 Radio 分组不成为当前基础控件的前置。

### 4.4 Scrollbar

Scrollbar 接收 Orientation、有限 position／visibleFraction 及部件视觉状态。position 和 visibleFraction 按 0～1 的归一比例表达，几何计算将合法有限输入收敛到范围内。hoveredPart／pressedPart 使用 Track／Thumb／Decrement／Increment 等类型化枚举，整体 enabled 独立表达。ScrollbarVisual.showTrack: Boolean = true 控制轨道绘制，不改变 thumb／按钮几何或调用方的轨道命中；opacity 仍作用于整条。调用方可用更大的 bounds 保留固定命中条带，再以 ScrollbarOptions.thickness 提供本帧绘制厚度，主题沿交叉轴居中绘制。

ScrollbarMetrics 包含建议厚度、最小滑块长度、轨道／滑块内缩及可选端部按钮长度。ScrollbarGeometry 返回轨道、滑块、可选两端按钮的具体矩形／轮廓、滑块起始位置和可移动距离。先扣除端部按钮，再按 visibleFraction 计算并限制滑块长度；滑块行程是轨道可用主轴长度减去最终滑块长度。极短 bounds、全可见内容和零行程仍返回非负有限几何，不产生除零。

滑块位置使用最终行程和 position。主题厚度、最小长度或端部按钮改变后，绘制及调用者的拖动换算都使用这份结果。横／纵两种方向适用相同规则。主题不处理滚轮、拖动、键盘、自动重复或内容滚动，也不启动自动隐藏 timer；透明度等动画值由调用方采样后传入。

## 5. 风格与明暗

五套主题完整覆盖上述四类部件，差异必须同时包含颜色之外的可验证内容：

| 风格 | 方向 |
| --- | --- |
| Liquid Glass | 系统 UI 字体、14／20 文字、36 高胶囊操作面、22 浮动圆角，清晰前景与下层光学分离；用途策略见 §5.1 |
| shadcn | Base Nova／Neutral，14／20 文字、32 高操作表面、10 基准圆角、3 宽焦点环；完整 Light／Dark、图表与侧栏语义配色 |
| macOS | macOS Sequoia（15）的 Aqua／Dark Aqua；13／16 系统风格文字、22 高常规操作面、5 圆角与细边框、14 大小标记、贴边的 3 宽焦点外框及无端部按钮的中性滚动条 |
| Material | Material Design 3／Material Web；Roboto 14／20 文字、40 高胶囊操作面与 24 水平留白、18 复选框／20 单选框、40 圆形交互状态层，按状态提升的阴影和中性滚动条 |
| Fluent | Fluent 2 Web 的品牌与中性色状态、Segoe UI 字体回退、14／20 正文与操作文字、12／16 辅助文字；32 高操作面、4 圆角、16 大小标记、清晰的 2 宽焦点边框、分层浮动阴影及中性细滚动条 |

这些是自绘预设，具体数字、颜色、轮廓与状态组合在各实现库中明确声明并测试。Light／Dark 调整整组语义颜色、边框／阴影强度和可读内容，而不只替换画布背景。共享字体描述使用系统存在的 family 回退列表；测试使用已有固定字体资源，不能依赖本机某一种商业字体一定安装。

macOS 预设固定以 Sequoia 的窗口正文控件为基准，参考 Apple 的 [AppKit 外观说明](https://developer.apple.com/videos/play/wwdc2020/10104/)、[按钮](https://developer.apple.com/design/human-interface-guidelines/buttons)和[选择控件](https://developer.apple.com/design/human-interface-guidelines/toggles)。不采用 Liquid Glass 的透镜、折射、背景采样或玻璃高光，也不随运行系统升级改变轮廓。Light／Dark 默认窗口底色为 #ececec／#323232，内容底色为 #ffffff／#1e1e1e，强调色为 #007aff／#0a84ff；窗口、内容与普通控件底色分别承担不同角色。正文与操作文字采用 13 大小、16 行高及普通字重，辅助文字采用 11／14。

macOS 普通按钮、默认操作按钮及 Checkbox／Radio 在 hover 时保留原有外观，按下立即显示压下色；不将普通按钮染为强调色。禁用状态不保留强调底色，使用中性控件底色与禁用文字，不再额外衰减整个控件。Checkbox 的 On／Mixed 均为强调色底，分别绘制对比色勾／横线；Radio 为强调色实心圆与对比色小圆点。3 宽焦点环直接贴合控件外缘，保留配色中焦点色的透明度，禁用时不绘制。选择标记使用短暂的 100ms ease-in-out 过渡；hover／press 不插值。按钮使用轻微投影，浮动面使用柔和分层阴影，默认不使用辉光。滚动条使用 15 宽条带、内缩的圆头中性滑块且不带端部按钮；显示／隐藏及条带预留继续由组件的 ScrollbarBehavior 决定。外部 palette 仍适用于全部部件，不改变上述尺寸、轮廓与动效。

Fluent 预设以微软 [Fluent UI tokens](https://github.com/microsoft/fluentui/tree/master/packages/tokens/src) 和 [Fluent UI React components](https://github.com/microsoft/fluentui/tree/master/packages/react-components) 为配色及四类公共部件的对照。Action 的 Light／Dark 默认品牌背景分别为 #0f6cbd／#115ea3，标签均为白色；Checkbox／Radio 使用独立的 compound brand 状态色，不能在 Dark 中复用操作面的深蓝。普通和操作面不附加投影或辉光；浮动面使用环境与方向两层阴影，CSS blur 转换为本库的 Gaussian sigma。Checkbox 的 On 为品牌底色勾选，Mixed 为透明底色、品牌边框与方形；Radio 保持透明底色、品牌边框与实心品牌圆点。禁用颜色取独立中性 token，不由任意混色推导。滚动条默认不带端部按钮，hover／pressed 滑块均保持中性色；显式 buttonLength 覆盖仍由公共几何处理。

Fluent 的 hover／press 使用 durationFaster（100ms）与 curveEasyEase（cubic-bezier(0.33, 0, 0.67, 1)）；Checkbox／Radio 的选择标记即时更新，与对应 Web 控件一致。减少动态仍通过公共 ThemeMotion.resolve 关闭过渡。

Material 预设以 Google [Material Web 的 M3 tokens](https://github.com/material-components/material-web/tree/main/tokens/versions/v0_192) 及 [Checkbox](https://github.com/material-components/material-web/blob/main/checkbox/internal/_checkbox.scss)／[Radio](https://github.com/material-components/material-web/blob/main/radio/internal/_radio.scss) 为基准。Light／Dark 的 surface 为 #fef7ff／#141218，primary 为 #6750a4／#d0bcff，on-primary 为 #ffffff／#381e72；surface-container、secondary-container、outline／outline-variant 和 error／on-error 分别映射到对应语义角色，避免用近似紫色或背景色代替。正文使用 Roboto 回退的 14／20、0.25 字距，辅助文字为 12／16、0.4 字距，操作标签为 14／20、500 字重和 0.1 字距。

Material Action 为无常驻阴影的 filled button，hover 混入 8% on-primary，pressed 混入 12% on-primary；仅 hover 使用一级提升的两层投影，默认不使用 glow。OutlinedAction 使用透明 outlined 表面、outline 边框与 primary 标签；hover／pressed 以 8%／12% primary 表达状态。填充／描边操作面均为 40 高、64 最小宽、24 水平内容留白和胶囊轮廓。禁用填充操作面以 12% on-surface 填充，描边操作面保持透明并使用 12% on-surface 边框；文字为独立 disabledText（预设 38% on-surface），不再施加整体透明度。浮动面使用 surface-container 和二级提升投影。操作焦点为 secondary 色的 3 宽外环，与轮廓间隔 2。Plain 输入表面为 56 建议高、固定 4 圆角、透明底色和 on-surface 正文，普通边框为 outline，hover 为 on-surface；焦点为贴边的 2 宽 primary 描边，验证为贴边 error 描边，不使用操作按钮的外围环。多行高度增加时圆角保持 4；单行输入和 Textarea 共用该输入表面。

Material 的输入控件同时遵循 [Text fields 规格](https://m3.material.io/components/text-fields/specs)和[标签规则](https://m3.material.io/components/text-fields/guidelines)。`ThemeTokens.textField` 是视觉处理枚举 `TextFieldTreatment.Standard／Outlined`，Material 选择 Outlined，其余预设选择 Standard；控件不判断主题类型。Standard 使用普通外置标签，Outlined 使用位于框内或上边框缺口内的浮动标签。Material 的 typography.input 为 16／24、0.5 字距，浮动标签及辅助文字使用 12／16；输入正文与 placeholder 不再借用 14／20 的普通正文角色。输入文字缩放与其他 typography 角色一并解析。Outlined 的错误边框保持不透明，普通为 1 宽、聚焦为 2 宽，高对比度不低于 2；应用明确的 validation／border 覆盖仍优先。浮动标签、缺口布局及可访问名称由控件消费这些普通描述实现，不向 Theme 添加编辑状态或原生资源。

Material Checkbox 使用 18 大小、2 圆角／边宽和 2 宽勾／横线；Radio 使用 20 大小、2 宽圆环和 10 直径圆点。未选中内部透明，普通边框取 on-surface-variant，hover／pressed 取 on-surface；选择后保持 primary 标记，Checkbox 的勾／半选线使用 on-primary。禁用标记使用 38% on-surface，选中 Checkbox 的标记使用 surface，边框不与半透明填充重复合成。两种标记共享 40 大小圆形状态层：未选中 hover 为 8% on-surface、pressed 为 12% primary，选中 hover 为 8% primary、pressed 为 12% on-surface；焦点轮廓跟随该圆形区域。hover／press 使用 100ms，选择变化使用 200ms，均采用 M3 standard easing（0.2, 0, 0, 1）。滚动条保留桌面的 14 宽条带和 32 最小滑块，以 on-surface-variant 中性色表达状态；不冒充 Material Web 未提供的滚动条组件。所有颜色仍从所给 palette 派生，外部配色不改变以上几何与动画。

`scoop.ui.component/examples/shadcn` 的右上角 Style 下拉框只选择 shadcn、macOS、Material、Fluent。配色来源是独立的 Preset／System 设置，提供系统取色适配时默认 System，否则固定 Preset。System 适用于全部四种风格，通过相同的 palette 参数传入当前风格工厂；只在 System 被选中时读取系统颜色。切换风格保留配色来源，切换配色保留风格；二者通过同一个 ThemeSource 发布完整主题，并保留明暗、减少动态、布局方向、页面、编辑内容与业务状态。

### 5.1 Liquid Glass

`scoop.ui.style.liquidglass.liquidGlassTheme(mode: ThemeMode = Light, palette: ThemePalette? = None, configure: ShapeThemeBuilder.() -> Unit = {}): ShapeTheme` 是独立完整预设，复用公共 ShapeTheme、StylePainter 和玻璃绘制能力。主题描述不持有图形资源、不读取业务状态，不通过其他预设构造不完整的外观再补齐。显式 palette 决定所有语义颜色；configure 最后执行一次。基础主题、外部配色与根交互偏好分别沿第 3.2、6.1 节解析。

默认字体使用系统 UI 字体偏好并回退 Inter／sans-serif，正文和操作字号为 14、相对行高 20／14，caption 为 12／16。普通操作建议高 36、最小交互尺寸 28、水平内缩 14，圆角为 18；输入继续沿控件的尺寸与布局规则。主题提供完整的普通／强调／描边／浮动表面、勾选、单选和滚动条，所有状态色从当前语义配色派生；焦点轮廓与错误边界持续清晰。

明暗配色分别提供中性内容底色、足够对比的正文和次要文字、强调与破坏性操作、图表和侧栏颜色。玻璃策略通过既有 GlassSurfaces 表达，默认操作、导航、工具栏、弹层、对话框和通知可使用材质，正文 Content 和输入 Input 保留稳定底色。局部 StyleOverride 的明确背景或 glass 清除仍优先；高对比度和减少透明度恢复实体表面，减少动态停止过渡而不改变内容与语义。独立主题的材质参数不要求应用逐帧构造、关闭资源或读取背景像素。

默认材质按用途尺度共享三组不可变描述：Control 的 blurSigma／refraction／edgeWidth 为 4／2／5，Navigation 与 Toolbar 为 8／3／9，Popover／Dialog／Notification 为 12／5／16，单位均为局部逻辑像素。小操作的折射集中于窄边缘，较大浮动层平滑较宽的下层区域；正文与输入不隐式继承相邻用途的材质。应用可直接复用或 copy `theme.tokens.glassSurfaces.get(role)`，也可配置某个用途；不需要为取得预设光学参数引入第二套主题或 painter。明暗 tint 与高光相应调整，可读性染色仍由实际前景和语义配色决定，不能用强制不透明的染色掩盖外部低对比配色的问题。

## 6. 整体切换与局部主题

`ThemeSource` 是应用持有的稳定普通 class，包含 `State<Theme>` 当前值。应用将来源通过语言 Context 绑定在真实 Application.run／runWithAsyncio 及挂载入口；控件挂载时取得来源，在每次 render 中读取当前值，使实际消费者订阅。一次来源更新合并逻辑工作和必要帧，不遍历未订阅的任意业务对象。

Component 的挂载工厂在该入口构造实例，因此可以在构造阶段保存 ThemeSource 引用。首轮 render 早于 onMount，render 所需的普通来源引用不能等到 onMount 才初始化；onMount 继续用于登记控制器、订阅和 native 资源。保存来源不等于保存当时的 Theme，每次 render 仍调用 source.read(scope)，同时读取基础主题与本根偏好。

控件只保存来源和持久交互状态。Theme、已解析局部覆盖及几何经 Props／同帧 measure／paint 更新；切换风格或明暗可以按新的 metrics 重新布局，但保留组件实例、State、Binding、InputArea 和焦点。没有结构变化时不能通过更换 Element 类型或增删装饰包装实现主题切换。painter 资源替换不等于组件卸载。

局部主题首期通过明确的 ThemeSource／Theme 参数和 Props 传入，组合控件显式转交需要继承的子项。不把临时 Context 当成 View 捕获或自动挂载树继承；需要子树 Provider 时另行设计。没有环境来源且未显式提供主题的控件，使用语言现有的缺失 Context 诊断，不偷偷建立全局可变默认主题。

输入使用 Surface 已成功提交的几何。控件在 paint 放置主题给出的轮廓／区域，必要的拖动数据随该帧成功发布；失败帧不能提前替换已发布的输入几何。逻辑 enabled 和最新处理器仍在 update 生效，绘制变化不推迟禁用。完整 ScrollView 的拖动／滚动行为不属于此绘图批次。

### 6.1 根交互偏好的主题消费

`ThemePreferences(reduceMotion=false, highContrast=false, reduceTransparency=false, textScaleFactor=1.0)` 是纯值输入，文字缩放须为有限正数。`Theme.withPreferences(preferences): Theme` 返回采用这些偏好的不可变主题，必须同时更新实际使用的 tokens、部件及绘图数据；第三方主题也实现这一入口，不能只替换 tokens 而继续绘制旧的部件颜色。重复应用相同偏好不叠加字号；ShapeTheme 从保留的基础描述解析，恢复默认偏好得到原基础主题。对解析结果显式 copy 会建立新的基础描述，应用通常修改 ThemeSource.current 中的基础主题。

`Theme.preferences: ThemePreferences` 公开当前主题已经应用的偏好，基础主题为默认值，包装主题转交所包装结果。控件用它解析自身产生的默认选区和底色，不能在主题解析后重新引入被关闭的透明外观；局部明确的颜色与样式仍优先。它是主题值的一部分，不重新查询系统或订阅窗口。

`ThemeSource.resolve(preferences)` 读取当前 State 并复用最近一次相同基础主题／偏好的结果；不拥有 painter 或窗口资源。`ThemeSource.read(scope)` 从 ComponentScope.interactionPreferences 取得根已合并的设置，未知布尔值使用 false、未知文字缩放使用 1.0，再调用 resolve。内置控件和自绘示例通过 read 消费；current.value 保留未经根解析的基础值，供应用修改或纯数据检查。同一来源可供不同根使用，根的偏好不写回共享 State。

内置 ShapeTheme 的具体规则如下：

- reduceMotion=true 将有效 ThemeMotion 设为 Reduced，主题原本的 Reduced 继续有效；普通皮肤的 Full 不能恢复根已关闭的动画。现有过渡、位移和不定进度动效通过同一有效 ThemeMotion 停止或立即到达目标，不增加时钟。恢复后按各控件已有的启动规则运行。
- textScaleFactor 作用于 body／caption／action 的字体大小，保留字体族、字重及相对行高。控件用新文字度量重新布局和命中，不缩放整个 Canvas，也不改变窗口 DPI；控件明确的局部字号继续由应用决定。
- highContrast=true 使用主题当前明暗模式作为不透明背景，将默认语义配色、表面与标记的实际颜色调整为高对比。稳定交互状态下，启用文字／选中标记与背景的对比度至少 7:1，禁用文字至少 4.5:1，控件边界／焦点至少 3:1；对比度使用 sRGB 相对亮度。取消会降低辨识度的整体禁用透明度；基础主题用 disabledOpacity 表达的禁用文字和选中标记先将该衰减合成到实际底色，再满足禁用对比度，避免取消透明度后与启用状态完全同色。焦点环宽度至少 2 个逻辑单位；外环不再用固定颜色替换各状态已解析的边框。颜色仍保持可用的原色；不足时朝黑或白调整。这是内置主题的对比度策略，不冒充 OS 强制色调色板。
- reduceTransparency=true 将默认填充颜色合成到对应的不透明背景，并去掉主题默认投影、辉光与模糊装饰；highContrast 同时采用这一处理。抗锯齿、图像内容与应用显式指定的绘图效果不被重写。局部颜色／样式覆盖仍归应用所有，应在相应界面单独验收对比度。

上述变化沿根的既有显示依赖更新；实际排版、绘图与输入区域使用同一轮结果，保留挂载、编辑模型、IME 与焦点。测试同时检查四套主题的明暗配色和真实控件热切换，尤其验证停止动画后不继续请求帧。

滚动边界与直接接触惯性使用 UI 规范 §7.7.2／§7.7.3 的 ScrollPhysicsOptions。各主题解析后的 MotionPreference.Reduced 必须由共享滚动控制器和文本宿主转交局部 reduceMotion；根热更新同时约束自绘 ScrollViewport。颜色、透明度、字体和滚动条静态外观继续沿既有 ThemeSource 解析，不在主题实现复制滚动物理。

## 7. 效果、缓存与能耗

保留 [Style 规范第 3～4 节](SCOOP-UI-STYLE-SPEC.md#3-绘制布局与输入)的完整合同：外框阴影与内容 alpha 效果不同，有序 blur／drop shadow／glow、显式 clip、扩展输出和一次 DPI；背景滤镜通过 StylePainter 的 backdrop 参数借用。主题使用同一 graphics／Style 能力，不能复制滤镜实现或要求先完成 SkSL。

ThemePainter 缓存考虑本轮主题参数、几何、效果和本帧采样值，正确替换并关闭旧资源。首批不缓存子树像素，文字／图像变化与子项动画不会因主题对象未变而失效。Painter 不建立任何任务或时钟；动画参数由控件交给 Surface Animatable，完成、暂停、取消、隐藏或卸载后恢复等待。

静止验收同时观察 Component.render、Element.update／measure／paint，以及底层帧需求和提交；不能用截图不变代替停止工作。主题切换及真实参数变化可以唤醒一次必要更新，之后停止。原生窗口关闭和异常退出沿 ComponentScope／SurfaceContent 的既有所有权路径清理所有 painter、Path 和滤镜。

## 8. 实施与验收

实施顺序为 graphics 基础滤镜及通用嵌套 PaintContext → 公共 Style／装饰 → 主题协议与几何／绘图基础 → 三套主题及离屏展示 → 基础控件消费主题 → 完整展示示例和真实 UI 组合。上述基础批次及原有布局、生命周期、异步进度目标均已完成其 macOS 验收。本轮第 6.1 节新增偏好的实施与验证范围见 [UI 交互打磨记录](../ui/interaction-polish/PROGRESS.md)，不沿用历史批次的完成状态。每批格式、检查与定向测试通过后提交、推送、merge origin/main，正常缓存保留。

主题展示示例并列显示四类公共部件和已实现控件的普通、hovered、pressed、disabled、selected／mixed 和 focused 状态；支持四套风格、Light／Dark、系统跟随及窗口内高对比度／减少透明度开关。公共部件区域直接调用 Canvas 绘图，真实控件区域同时验证交互和主题切换，菜单、Select 和 Card／Alert 弹出面板展示实际表面的配色。

展示的默认 1120×840 逻辑尺寸窗口完整显示状态表格、真实控件、偏好／效果开关和弹出预览入口。通过调整示例的排列与留白满足这一要求，不缩放整棵 UI 来掩盖主题尺寸差异；更小窗口继续遵守容器的显式 clip，首期不因此增加 ScrollView。

所属 Cone 的 scoop.test 验证以下事实：

1. 几何无 Canvas／Application 可计算，建议／最小尺寸和内缩符合各部件合同；短轨道、最小滑块和端部按钮行程正确。
2. 真实离屏像素与同一几何一致，三套主题在尺寸、圆角、边框、阴影／焦点及明暗上有实际差异；DPI 只变换一次。
3. 控件仅使用公共主题接口，局部覆盖未指定的属性随主题变化；标记绘图可独立用于第三方内容。
4. 切换主题／效果保留实例、State、焦点和业务绑定，必要布局更新且输入几何按成功帧发布。
5. Surface 参数动画及最终停帧、缓存替换、绘图失败、卸载和原生关闭的资源释放。
6. 必要的真实 Metal／UI 组合及展示例子的交互切换。只记录实际运行的架构／后端；不新增普通 fixture 或执行全量测试。
