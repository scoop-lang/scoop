# UI 无障碍访问、自动化与 AI computer use 设计

日期：2026-11-09；2026-11-16 配合 [UI 交互细节打磨](../interaction-polish/DESIGN.md)复核更新。状态：S0／S2／S5 实施中。本文确定完整目标合同与技术边界；低层 Window 语义资源、AppKit 基础投影以及 UI scope／场景、关系、内容合并／忽略、精确命中及纯文本选区／编辑已接入，正式接口见 [Window](../../specs/SCOOP-WINDOW-ACCESSIBILITY-SPEC.md)和 [UI](../../specs/SCOOP-UI-ACCESSIBILITY-SPEC.md)规范。内置控件、复杂控件关系、文本范围几何／单位导航／保留范围、虚拟集合和其他平台 provider 仍待完成，不能据本文判断完整 accessibility 已可用。调研依据、现有源码入口与平台资料见[调研记录](../ACCESSIBILITY-RESEARCH.md)，实际通过范围见[实施记录](../interaction-polish/PROGRESS.md)。

完整实施任务、依赖和验收统一见 [UI 交互打磨与 accessibility 实施计划](../interaction-polish/PLAN.md)。计划的 S0～S11 覆盖本文全部能力；第 12.2 节的 P0～P5 保留为设计来源分组，实际推进与状态记录使用统一计划。

## 1. 目标与范围

为 Scoop 自绘 UI 建立平台无关的语义模型，并接入 macOS Accessibility、Windows UI Automation 和 Linux AT-SPI。系统辅助技术、自动化工具及能够读取这些接口的 AI agent 共享同一份 UI 事实和动作入口。

本期需要实现：

- 读取控件角色、名称、值、状态、关系、阅读顺序、集合位置和有效几何。
- 按控件语义执行激活、修改值、选择、展开、滚动、文本编辑和焦点操作，收到实际结果与变化通知。
- 正确处理隐藏、离屏、弹层、模态限制及没有实例化的虚拟条目。
- 让第三方 Element 使用与内置控件相同的公共能力，并能通过真实辅助技术完成操作。
- 与普通交互共同处理系统偏好、可见焦点、文字缩放、上下文菜单和拖放替代操作；语义 provider 与键盘、指针、触摸使用同一业务状态。

实施采用已有的 C／Objective-C／C++ 与系统库，不新增 Rust 依赖，不采用 AccessKit，也不开展其 no_std 改造。框架不增加专用编译器 IR、runtime 对象模型或另一套事件循环。

App Intents 留作独立后续工作。Swift framework 互操作、C ABI 包装、编译期元数据和打包由另一条调研处理；本设计不依赖其结果。应用可让 UI 动作与未来的 App Intent 调用同一业务函数，但此处不新增命令注册或 AppEntity 框架。

AI 接入以标准 OS provider 为基础，不默认启动 MCP／HTTP 服务，也不为 agent 保存第二棵语义树。仅接收截图的 agent 仍通过正常画面、鼠标和键盘操作。

## 2. 分层与现有能力的复用

| 层 | 本次职责 | 已有基础 |
| --- | --- | --- |
| `scoop.ui.component` | 默认角色、标签、值、关系、集合信息和动作；Props 提供必要覆盖 | PressControl、FieldTarget、选择控制器、VirtualList、TextEditHost |
| `scoop.ui` | 语义声明、节点身份、树结构、变更提交、虚拟条目接入、动作资格 | ComponentScope、挂载树、UiRoot、SceneContext／SceneCommand、ScrollReveal、焦点与 Portal |
| `scoop.surface` | 复用帧提交、坐标／裁剪状态、输入边界与底层焦点 | FrameCompletion、FrameContext、InputArea／InputScope |
| `scoop.graphics.text` | 复用文字内容、分段、位置与范围几何查询 | TextLayout、TextPosition、UTF-8 文本模型 |
| `scoop.window.accessibility` | 新增低层窗口 provider 接口、公共值类型与原生动作请求 | WindowId、LogicalRect、owner、mailbox、native 生命周期 |
| `scoop.window` 的 native 后端 | 原生对象、属性／pattern 映射、坐标与文本单位转换、通知 | AppKit、Win32 COM、GTK／GIO／GLib |

`scoop.window.accessibility` 是现有 Window Cone 中的包，不新增 Cone。它不依赖 `scoop.ui`、style、控件类型或 Canvas；UI 将语义事实投影到该接口，原生端不检查 Button 等 Scoop 类名。

每个附着的 `UiRoot` 在首次语义登记时为自己的窗口内容视图创建一个 provider 根；没有声明的根不创建空树。窗口、系统菜单和系统对话框继续由原生后端提供其自身语义，UI 根与它们建立正确父子关系。不能同时让原生内容视图与自绘根重复报告同一组内容。

provider 的寿命与 `UiRoot` 附着一致。关闭 `UiWindow` 应退休自绘子树，保留调用方仍持有的原生 Window；重新附着分配新的节点身份。低层 Window 接口同样允许直接使用 Surface 的实际调用方提供语义，不要求实例化 UI 控件库。

系统减少动画、高对比度、减少透明度、滚动条与 caret 等偏好由 Window 的公共平台接口提供，UI／style 消费其解析结果，详见[交互打磨设计第 3 节](../interaction-polish/DESIGN.md#3-分层与平台偏好)。该能力在 provider 尚未接通或当前没有辅助技术客户端时仍生效；不以“检测到屏幕阅读器”作为提供完整键盘和视觉体验的条件。

## 3. 语义节点与身份

### 3.1 三种结构各司其职

挂载树负责 Component／Element 实例，场景负责几何和绘制，语义树负责可理解、可操作的 UI 内容。一个挂载可以产生零个、一个或多个语义节点；文本与分组不需要 InputArea，也可以成为语义节点。

普通布局节点默认透明。语义父子关系来自显式登记和挂载关系，不根据绘图命令、对象反射或屏幕坐标猜测。阅读顺序默认遵循声明／数据顺序；命中顺序沿有效场景的层叠顺序，两者不能混用。需要调整阅读顺序时使用实际节点关系，不以像素坐标排序代替。

虚拟集合通过逻辑子项访问器表达后代，公共模型不要求保存与 count 等长的 child ID 数组。原生接口若在一次明确请求中要求完整子项数组，可以按请求生成轻量代理；这不等于为每项创建 View 或永久保存该数组。

### 3.2 节点数据

| 数据 | 合同 |
| --- | --- |
| `SemanticNodeId` | 类型化的运行期身份，带窗口归属；不使用对象地址或数组下标 |
| 角色 | 创建时确定，如 Button、Text、TextField、List、ListItem、Dialog；原生角色改变时退休旧身份 |
| 名称与说明 | 可访问名称、说明、错误文本、可选 automation identifier；已知空字符串与未知值区分 |
| 状态与值 | enabled、readOnly、protected、checked／mixed、selected、expanded、required、invalid、busy，以及允许公开的实际文本／数值 |
| 关系 | 父子、labelled-by、described-by、controls、active descendant；引用同一活动根内的有效节点 |
| 集合信息 | 数据顺序、条目 key、实际数量，适用时包含层级、行列与选中关系 |
| 几何 | 已提交几何或明确的不可用状态；包括当前可见区域与可命中性 |
| 能力 | 从实际登记的动作、文本与集合查询实现生成，不单独维护一份可能失真的能力布尔表 |

角色和身份必须存在；仅不适用或确实未知的属性可以缺省。有限几何、范围、关系归属和父子无环在相应更新边界检查，未变化的数据不在每次原生 getter 中重新验证。

`automationId` 是应用提供的稳定定位信息，独立于本地化名称；不要求全应用唯一，客户端应结合窗口和容器限定查询。它不能替代运行期节点身份。

### 3.3 身份与退休

普通节点在其语义资源存活期间保持身份；重排不改变身份，真实卸载、角色替换或根关闭使旧身份失效。编号在所属窗口生命期内不复用，根重新附着也不能让旧引用命中新节点。`ViewKey` 只参与协调，不直接作为全局节点 ID。

虚拟条目的逻辑身份是“集合节点 + 条目 key”，与 View 实例分离。原生客户端持有的轻量条目代理可以在 View 回收后继续解析当前数据；已经卸载的具体子按钮、文本编辑器等引用则退休，不能借相同 index 指向另一条目。

没有客户端引用、活动导航目标或正在处理请求的代理可以回收。再次按 key 查询时允许创建新的原生运行期身份；不为永久保持曾访问过的所有 ID 而保存整张列表。仍存活的代理必须保持身份，key 被真正删除时必须报告不可用。

## 4. 公共声明与控件默认语义

### 4.1 登记接口

以下是目标接口形状，公共资源和场景的具体合同见 [UI 无障碍接口规范](../../specs/SCOOP-UI-ACCESSIBILITY-SPEC.md)；其余能力继续按实施批次接入：

| 入口 | 用途 |
| --- | --- |
| `ComponentScope.semantics(role, parent)` → `SemanticsHandle` | 在构造／挂载资源阶段建立语义资源，随 scope 释放；parent 可缺省 |
| `SemanticsHandle.update(properties, actions, relations)` | 在正常更新阶段替换属性、最新动作处理器和同根关系，标记语义变更 |
| `ComponentScope.routeSemanticsChildren(parent)` | 指定本挂载的普通子挂载归入哪个语义节点 |
| `SceneContext.semantics(handle, bounds)` | 记录该节点的实际 transform、clip、层叠顺序与滚动路径 |
| 语义文本／集合接入 | 分别接入现有文本宿主和第 7 节的条目数据源，不要求生成完整子树 |

缺省父节点是最近祖先设置的语义路由目标，最终落到根。一个挂载登记多个节点时，它们默认是有序兄弟；内部嵌套通过显式 parent 表达。`routeSemanticsChildren` 只指定子挂载归属，不悄悄移动本挂载的其他语义节点。Portal 的视觉位置与语义归属可以不同，弹层通过显式路由接入根中相应的活动区域。

`SemanticsHandle` 采用已有 scope 资源管理方式。`render()` 只构造描述，不创建原生对象、登记回调或直接更新 handle；返回的普通 Element 在 mount／update 中应用语义属性。Scene 阶段只登记几何，不修改业务值。

单独关闭 UI handle 时保留仍由其他 scope／handle 拥有的子节点，先将其连接到原父级再退休该身份；角色替换不会误退休仍存活的子控件。真正卸载沿挂载树关闭相应资源。`bindFocus` 借用同一挂载的实际 FocusTarget，查询不获得焦点，目标退休时撤下对应的焦点事实；同步焦点动作的完成仍以实际状态为准。

几何登记作为可记录、可重放的 SceneCommand 参与现有场景缓存。负责绘制的后代 Element 可登记逻辑祖先持有的 handle，使组合控件沿用原 Component 的实际焦点与资源归属；兄弟、后代或跨根资源拒绝，单节点每帧仍只登记一次。它不创建输入目标，也不让每个控件安装 `onFrameComplete` 回调而使整个场景失去缓存能力。

### 4.2 内容合并与隐藏

分别提供“忽略当前节点但保留后代”和“排除整个子树”的表达。前者用于装饰和透明包装，后者用于实际隐藏内容，两者不能共用一个含糊的 hidden 标志。

Button 等控件可以合并自己的非交互标签内容，避免同时朗读“保存按钮”和另一个重复“保存文本”。独立的链接、按钮、输入框不能被名称合并吞掉。一般 Group 保留子节点；框架不自动遍历任意 Custom View 来猜测哪个子节点代表控件名称。

可访问名称按“显式名称 → 有效 labelled-by → 控件明确支持的内容标签”取得。说明和错误分别通过关系或对应属性提供；placeholder 不替代 label，tooltip 不自动成为控件名称。图标按钮必须有明确名称，装饰图标不单独进入树。

### 4.3 内置控件

控件 Props 增加统一的小型语义覆盖值，允许名称、说明、automationId 和必要关系覆盖；角色、动作和核心状态由实际控件行为提供。覆盖值不允许把不可编辑文本宣称为可编辑，或绕过 disabled 状态执行动作。

当前 Label、Button、Checkbox、Input／Textarea 使用 `ComponentSemantics`，Props 与 builder 均提供 semantics；实际接入及包装透传合同见 [Component spec](../../specs/SCOOP-UI-COMPONENT-SPEC.md#11-默认无障碍语义)。Button 的纯标签合并、Checkbox 的真实 Binding 结果、输入框的模型／选区／保护用途沿前述公共语义路径。Field 关系及其余控件仍按统一计划继续，原生方法、实窗和外部辅助技术分别记录验证状态。

| 控件组 | 默认语义与接入 |
| --- | --- |
| Label、Heading、Link | 静态文本／标题级别／链接目标与激活；沿用 StyledText 的替代文字 |
| Button、Checkbox、Switch、Radio | 激活、切换或选择；报告真实 checked／mixed／selected |
| Input、Textarea、密码输入 | 文本宿主、只读状态、选区、编辑能力及受保护值规则 |
| Slider、Spinbox、Progress | 范围、当前值、步进；进度不因此变成可编辑控件 |
| ScrollArea、Textarea 的视口及滚动条 | 真实范围、位置和滚动／显露能力；视觉自动隐藏不移除容器能力，零范围不公布虚假可滚方向 |
| Field、FieldLabel、FieldError | 复用 FieldTarget 建立标签、说明、required、错误关系 |
| List、VirtualList、Tree、Table | 集合及条目关系、位置、选择、展开和虚拟化能力 |
| Tabs、Menu、Combobox、Dialog | 选中页、活动项、弹出状态与模态边界；只采用适用的原生角色 |

表中列出的是映射要求，未存在或未实施的控件不因此成为本批新增控件。shadcn 等风格沿用 component 语义；style 不承担 accessibility 状态或动作逻辑。

## 5. 更新、几何与通知

### 5.1 语义更新独立于绘制

沿现有 `UiHost`／`UiRoot` 更新队列增加语义 dirty 与发布步骤。名称、值、关系、动作变化可以在普通 owner 更新结束后提交，不必请求 drawable 或启动一帧。保留现有 `UiInvalidation.None／Scene／Layout` 的含义；不为了该需求重写整个失效体系。

属性在一次正常 UI 更新完成后成为一致的已发布语义状态。原生 getter 读取这份状态，不观察执行到一半的 render／update。native 侧只保存跨原生查询所需的投影，Scoop 业务状态仍是唯一可写来源。

一次查询使用同一份已发布视图；多次查询之间允许正常更新，不为了客户端遍历锁住整个 UI。动作始终按当前节点和状态重新判定。

一次变更的处理顺序为：

1. 正常更新控件和状态，收集属性、结构及动作变化。
2. 立即收束卸载、隐藏、disabled 和模态限制，阻止旧引用继续执行动作。
3. 更新结束后发布一致的语义属性与结构；只有几何需要变化时才使用正常 Scene／Layout 失效。
4. 帧被 `Accepted` 后提交该帧几何，再发送依赖实际位置或显露结果的通知。
5. 通知发出时，客户端能够读到对应的已发布结果。

### 5.2 几何有效性

`SceneContext.semantics` 的 Rect／Path 重载捕获局部形状、transform、clip 和实际层叠顺序，由根在 `FrameCompletion.Accepted` 时统一发布位置与精确命中。Rect 登记参与场景缓存重放，Path 与现有 Path 输入登记一样使场景不缓存；调用结束后不借用原 Path。`Discarded` 不发布候选坐标；旧场景仍有效且语义对象未改变时可继续使用其几何。

新节点尚未呈现、布局与当前内容不匹配，或旧场景已经失效时，几何明确标为不可用。不能用 `(0, 0, 0, 0)`、缓存前的位置或估算的列表 extent 冒充可点击位置。原生协议要求固定形状返回值时，使用其规定的空值／失败，同时不能据此声称元素已在屏幕上。

对部分可见或非矩形裁剪内容，bounding rectangle 只是外接矩形。可点击点必须通过有效命中区域验证；矩形中心不构成点击保证。精确 hit test 通过 graphics 的 withNativeHitTest 保留已提交场景的不可变原生命中快照，不在 Window 层复制 Path／clip 引擎，也不在原生 getter 中回调 Scoop。候选、Window 节点和已发布投影分别持有所需引用，丢弃、版本失效、替换及关闭释放对应引用。AppKit 从屏幕点转换到当前窗口逻辑坐标，按场景顺序查询；阅读顺序保持独立。ActivationPoint 仍须通过真实输入资格和可点击位置验证，不能仅因已有 hit test 就开放矩形中心。

原生后端负责窗口逻辑坐标到平台要求的屏幕坐标转换，包括缩放、多显示器和坐标轴方向。窗口移动可刷新原生坐标映射，不要求重绘所有语义节点。窗口被其他窗口遮挡与控件被自身滚动容器裁剪分别处理，不能把二者统一映射为 hidden。

滚动回弹的视觉位移沿同一 Accepted 场景变换进入几何；语义滚动范围、当前逻辑位置和虚拟条目索引仍取合法的 ScrollState。不能把超范围视觉位移报告为负滚动百分比，也不能在画面回弹时留下未变换的点击点。滚动条变宽、变浅或闲置隐藏不退休滚动容器；若条本身导出节点，其身份和平台适用的可见状态分别处理，不在每次淡入淡出重建整个子树。

### 5.3 变化通知

提交差量涵盖结构、名称／值／状态、文本、选区、键盘焦点、active descendant 及必要的可见范围变化。按平台发送适用通知，不在每次动画或布局遍历后通知整棵树变化。

同一更新中重复属性变化可合并为最终值；文本变化、节点退休与焦点转换必须保留足以让客户端正确更新的顺序。语义数据修改本身不触发额外轮询。

状态提示和 live region 使用明确的 announcement 入口，提供平台适用的优先级。普通进度值更新、tooltip 和所有文本变化不能自动变成播报，以免覆盖用户正在阅读的内容。

双击／三击、触摸调整选区、菜单编辑和文本 Drop 的通知来自同一份已发布的文档差量与最终选区，不能由输入层和 provider 各发一遍。数据拖放的开始、可用目标变化、完成／失败可按实际控件语义提供简短反馈；每次 pointer move、滚动条透明度采样或回弹采样不触发 announcement。重排完成的反馈使用实际条目及新位置，失败不提前宣布已经移动。

## 6. 动作、焦点与模态

### 6.1 统一业务入口

语义动作包括 Invoke、Toggle、SetValue、Select、Expand／Collapse、文本选择／编辑、实际存在的 ShowContextMenu，以及 Realize、Reveal、Focus。参数使用动作对应的值类型；只有存在真实实现时才向平台公布能力。能力表示控件支持何种操作，enabled 等状态表示当前能否执行；disabled 不必移除其固有的原生 pattern，但动作必须按平台合同拒绝。

Button 的 Invoke 与 Press 共用最新业务激活入口，但不伪造 pointer-down／pointer-up；执行前按当前 enabled、生命周期和输入边界重新判定。选择、切换和设置值沿现有控制器及 Binding 修改状态。受控 Binding 拒绝修改时，native 快照不能自行改成请求值。

动作结果至少区分已完成、参数错误、不支持、目标不可用、业务拒绝及当前无法执行。查询另区分找到、确实不存在和不支持该查询；不能把未实现的属性搜索返回为空结果。

内部等待中的请求不是成功。需要同步完成的平台方法只能在动作达到其定义的完成条件后返回成功。Invoke 的完成表示该 UI 激活已执行，不表示按钮启动的网络请求或其他异步业务已经结束；后者由 UI 状态和通知表达。

动作可能同步删除自己、替换列表数据或关闭窗口。执行方先保存必要身份和结果状态，再调用业务处理器；返回后重新解析需要访问的目标，不继续解引用旧挂载对象。异常按现有根错误路径处理，不能穿过 C／Objective-C／COM ABI。

### 6.2 分离焦点种类

键盘焦点、集合的 active descendant、选择状态以及辅助技术的阅读位置是不同事实。读取名称、枚举子项和取得原生对象都不获得键盘焦点。

`Focus` 经过现有 FocusController、FocusHandle 和必要的滚动显露通路；键盘焦点通知必须反映实际焦点。active descendant 模式允许键盘焦点停留在容器、活动项随导航改变。辅助技术自身的浏览焦点只在平台有明确反馈时记录，不能从普通 getter 推断。

同窗口模态、Portal 与独立模态窗口共用现有输入边界。背景即使仍可绘制，也不能通过语义动作绕过模态限制；原生树按平台合同限制可导航内容。弹层首次可交互仍等待有效场景，退出时立即停止其动作，背景解除限制沿既有模态提交时序完成。

### 6.3 键盘、触摸与拖放的替代操作

只有指针拖动或多指手势才能完成的必要操作不算完整可访问能力。按实际消费者提供等价的普通入口：列表重排可有上移／下移或选择目标位置的命令；Slider／Resizable 使用范围值和步进；缩放内容提供增加、减少和恢复入口。键盘、菜单与语义动作调用同一业务函数，不伪造跨屏坐标拖动，也不新增泛化的“任意 DragTo”自动化协议。

窗口内、同程序跨窗口及跨程序的数据拖放使用[交互打磨设计第 6 节](../interaction-polish/DESIGN.md#6-数据拖放)的真实 offer／effect／完成规则。辅助技术能够通过复制／粘贴或应用的明确移动命令完成适用任务；不能假定外部程序支持 Scoop 的自定义动作。平台有适用的 drag／drop pattern 时只按已经实现的会话能力映射，不把该 pattern 当作所有平台的共同前提。

ShowContextMenu 使用当前语义目标和有效的 caret／选区／控件锚点，沿同一菜单请求通路打开。原生菜单由系统 provider 暴露条目，UI 自绘树只保持发起目标与必要关系，不复制原生菜单子树。菜单关闭按实际焦点会话恢复；目标已卸载、变更文档或被模态屏障阻止时，旧动作不转发给另一编辑器。

触摸探索由平台辅助技术管理；应用处理实际交付的输入，不截取系统保留的辅助手势。必要操作同时提供可触达的普通目标，不依赖 hover、双击速度或精确拖动。放大的触摸命中区域与语义 bounds／可点击点使用实际场景，不能覆盖无关兄弟或穿过裁剪。

### 6.4 视觉与系统偏好

缺省跟随系统，设置变化通过已有 owner 更新。减少动画时停止额外回弹、模拟惯性和装饰性位移，滚动／选择结果仍即时到达真实状态；一个主题的普通 Full 配置不能重新开启根策略已关闭的效果。系统常显滚动条要求、caret 不闪烁或加宽分别保留，不用同一个“无障碍模式”开关代替。

高对比度和减少透明度影响实际配色、边界与背景；焦点、选择、错误及 drop target 不仅用颜色深浅区别。文字缩放影响度量、布局和显露，不能只放大最终像素而保留旧命中矩形。系统字体／缩放不可查询时使用明确的应用配置，不伪造系统值，也不将物理 DPI 与用户文字缩放重复相乘。

这些规则适用于所有用户，先用现有控件完成纯键盘、缩放、对比度和减少动画验证，不等待全部 AX／UIA／AT-SPI 批次结束。

## 7. 隐藏、离屏与虚拟化

### 7.1 三个独立维度

每个节点分别判断：是否属于当前可访问 UI 内容、是否处于有效视口、是否存在具体 View／布局实例。不能用一个 `visible` 布尔值替代这三者。

| 情况 | 查询与原生树 | 动作与几何 |
| --- | --- | --- |
| 真正 hidden，如关闭但缓存的面板 | 排除该子树；只读查询不能将它重新显示 | 旧动作失效，不能利用缓存坐标点击 |
| 普通条目已实例化、完全滚出或被自身容器裁剪 | 保留逻辑可访问性，按平台报告 off-screen／not showing | 可读取已知信息并请求显露；是否允许其他离屏动作按控件与平台合同判断 |
| 部分可见 | 保留，报告真实位置及可见区域 | 命中必须落入有效可见区域 |
| 折叠容器的后代 | 保留容器及 expanded 状态；后代按平台表达为未呈现的逻辑项或暂不枚举 | 通过明确的 Expand／Reveal 请求进入；查询不能自动展开 |
| 虚拟条目没有 View 实例 | 集合仍有逻辑条目，通过 key／index 按需取得轻量代理 | 没有真实布局时无可点击坐标；可请求 Realize／Reveal |
| 条目已删除或被当前过滤条件排除 | 不属于当前集合；旧引用报告不可用 | 不能用旧 index 命中新条目 |

折叠不等于缓存面板被显式隐藏。UIA 允许某些折叠后代仍在逻辑模型中并报告 `IsOffscreen`；平台适配不能强制所有后端采用同一种子树裁剪方式。

opacity 为零、没有绘图、`KeepAlive` 下未被放置都不自动等于语义 hidden。控件必须明确表达其内容是否仍属于可访问 UI；布局阶段只负责真实几何。回收 View 与删除逻辑条目也必须分别处理。

### 7.2 集合与条目语义数据

`VirtualList` 的语义接入增加一个可选的数据级条目描述源，读取与 `ItemSource` 同一份不可变数据快照。它提供以下实际能力：

- 复用现有 count、keyAt、indexOfKey，确定当前条目身份和顺序。
- 按 key 取得不依赖 View 的名称、值、选择／展开状态、automationId 等已知元数据。
- 明确支持哪些属性查找；可以复用真实索引，也可以在一次明确请求中扫描轻量元数据。
- 把条目显露、焦点和已有的集合选择行为交给对应控制器；不复制业务状态。

`ItemSource.build(index)` 是任意构造函数，框架不能在不执行它的前提下反射出名称和控件结构。已有数据格式化函数应同时服务画面与语义描述，避免两套文本生成规则漂移。

条目描述源随 Props／数据源在正常更新中发布，查询持有相应的已发布快照，不读取进行中的 State 更新。名称、选择或展开状态改变时，同一更新同时刷新集合语义；不能等下一次滚动才使辅助技术看到变化。

没有数据级描述时，框架只能提供已知的身份、位置、集合信息及实际支持的实现能力；名称可以未知。此时不宣称支持按名称搜索，不偷偷构造整张列表来补齐属性。需要可发现名称的应用应提供描述源；这是完整虚拟化无障碍体验的调用方要求。

未实例化条目只暴露其真实已知的行级语义。行内任意 Button、输入框、菜单等完整子树在 Realize 后由正常挂载产生，不根据数据字段猜测。条目容器的语义代理与其 View 内的子节点是两个生命周期。

已有 `ItemSource.count` 继续描述当前数据源中的实际条目。服务端未加载的数据与 View 未实例化不同；不得捏造未知 key、名称或完整可查询数量。本期不新增远端分页协议。

### 7.3 查询、实现、显露、聚焦

| 操作 | 允许产生的效果 | 完成条件 |
| --- | --- | --- |
| Lookup／Enumerate／Find | 返回元数据或按需代理；不滚动、不聚焦、不构造整列表 | 查询结果对应一个有效数据快照；不支持的查询明确失败 |
| Realize | 按 key 准备该条目的具体 View，必要时完成所需布局 | 请求的实际语义能力可访问；不保证进入视口或获得焦点 |
| Reveal | 通过实际滚动路径显露条目，必要时先 Realize | 新场景被 Accepted，目标确有有效可见区域；不可显露则报告失败 |
| Focus | 经正常焦点规则选择目标，必要时实现和显露 | 对应键盘焦点实际建立；不能只以请求已排队为完成 |

Realize 增加不抢焦点的按 key 准备入口，复用现有 LazyChildren 和 VirtualList 的实现／测量机制。现有 `focusItem` 不能兼任查询或 Realize；`scrollToItem` 也不能把“接受滚动请求”直接转换为原生同步完成。

显式 Realize 可以使局部挂载和布局发生工作，但不要求公开估算坐标。为满足属性和子树访问已完成实现、尚无已提交几何时，仍报告几何不可用。窗口不可呈现时不把 Reveal 假报为成功。滚动路径的 Reveal 直接复用 ScrollViewport 的 Accepted snapshot 与显露边界，包括 Overlay 滚动条的 ScrollPadding；原生 provider 不另算一份未扣遮挡的可见区，也不把固定 gutter 或视觉自动隐藏误作语义节点退休。

实现期间由正在处理的请求临时保留目标，完成、失败或取消后释放。同步原生调用的返回边界必须保证客户端能使用刚实现的能力；之后回收按平台虚拟化合同重新降为代理，不能在返回前就卸载。

容器自身已有按 key 选择行为时，可以在无 View 的情况下完成该选择。任意行内控件的动作必须先得到真实控件及其 handler，不能让数据描述源冒充所有子控件的业务实现。

### 7.4 数据变化、引用与资源

请求保存 key，使用当前源重新定位；排序或插入不改变目标。删除、过滤排除、根关闭使请求失败；旧数据快照的查询可以完整返回，但后续动作必须重新解析 key。后来的实际滚动／焦点输入按现有控制器规则替换冲突请求，不另建通用事务或调度框架。

普通属性读取和原生对象引用不保活整个 View。真实输入焦点、编辑／composition、正在完成的 Realize／Reveal，以及平台明确反馈的活动导航目标，才按对应需要保留实例；目标改变或请求结束后解除保留。删除优先于保留，不能为了维持引用重新插入已删除条目。

数据拖放只在确实需要当前编辑宿主、落点或目标 handler 时保留相应实例；已经拥有自有数据快照的来源不因此保留整棵 View。会话记录逻辑 key，虚拟项重排不串项，删除立即使相关动作失效。不能为了公布可能的 drop target 而实现整个虚拟列表。

十万条数据、约二十行可见的列表，不应在接入 provider 时构造十万个 View 或常驻十万个语义对象。正常开销随已实现条目和被实际持有的代理增长。客户端主动全量枚举仍有相应真实成本，不能承诺零成本，也不为此引入任意资源计费或访问配额。

平台可能在 View 回收时将某些具体对象移出原生树。通知应表达该平台中的对象实现状态变化，不让客户端把仍在集合中的逻辑 key 误认为业务删除。

## 8. 文本访问与编辑

文本语义复用 `TextEditorState`、`TextEditHost` 和 `TextLayout`，不建立另一份可写字符串、选区或撤销历史。静态文本、只读可选择文本和可编辑文本分别公布能力。

S5 已接入 `SemanticsHandle.updateTextEditor`：普通 update 从同一挂载的 TextEditHost 发布正文、选区和实际焦点，保留调用方的名称／说明／错误与关系。AppKit 提供纯文本范围读取和单选区，原生写入使用 Accessibility 来源及同一外来文本清洗、过滤和历史；单纯选择结束 composition 使用 EditCommand。正文或模型版本尚未重新发布时拒绝旧目标写入，受保护用途不发布正文／长度／选区，即使显式允许复制或显示明文也保持该边界。文本几何、单位导航、retained range 与其他 provider 仍按下述完整合同继续。

只读宿主通过 updateSelectableText 发布含 alternateText 的正文，并复用区间映射处理占位符、替代文字与方向选区；普通 update 和选择通知刷新原 handle。两个宿主的 ShowContextMenu 复用同一菜单会话，可为明确菜单请求取得目标焦点；读取和选择仍不抢焦点。模式禁用不公布菜单动作，等待实际焦点／会话结束或缺少 Accepted 锚点时不假报菜单已经打开；实窗与原生方法的验证范围分别记录。

内部位置沿现有 UTF-8 字节偏移与合法边界合同。AppKit 和 Windows 接口在需要时转换为 UTF-16；AT-SPI 按对应接口转换为 Unicode 字符偏移。转换与同一文本 revision 绑定，不用“一个字符等于一个字节／UTF-16 单元”近似。文字单位移动继续复用现有 grapheme、word、line 与布局能力。

提供正文／范围读取、选区读取与设置、按点定位、范围矩形、适用的值替换和编辑动作。UIA TextPattern 表示文本访问及选择等能力，不应当成任意文本写入接口；写入只通过该控件实际提供的 Value／编辑能力完成。

文本范围持有逻辑位置与所属文档身份，不持有裸 TextLayout 指针。客户端保留的范围通过现有编辑差量更新端点；文档或节点退休后报告不可用。不为范围对象永久保存全文历史。范围如何随插入、删除和端点 affinity 变化，必须在文本 spec 中规定并覆盖组合测试。

正文更新可以先于新布局发布。范围几何必须来自匹配正文的已提交布局，不能将新偏移套入旧布局；没有匹配几何时仍可读取正文，但不能制造矩形。软换行、RTL、emoji、组合序列、行内对象和替代文字使用同一套实际文本规则。

辅助技术编辑进入现有编辑策略、只读检查、selection、composition 与 undo 分组。需要区分编辑来源时，在现有 `TextEditSource` 中增加 accessibility 来源；不绕过应用的 Reject／Transform。原生 text／selection 通知在实际编辑结果发布后产生。

密码控件声明受保护文本属性。原生属性、范围文本、诊断快照和通知均不得包含明文；平台允许的长度、选区和替换能力单独适配。不能仅把画面换成圆点，却仍通过通用 value getter 暴露正文。

受保护用途独立于视觉投影：TextEditOptions 与输入控件的 protectedText 已提供此事实，暂时显示密码明文、切换主题或将复制策略设为 Allow，不自动解除 provider 的保护。文本 provider 接入时消费宿主的同一标记，不从投影或复制权限反推。拖放 preview、系统词典／拼写／Services 等菜单入口也按同一用途决定能否导出内容；缺省禁止受保护文本拖出，正常粘贴／拖入仍遵守输入策略。

多击选择、触摸把手和 Drop 共用 UTF-8 范围、方向及 affinity。按词、逻辑行或视觉行查询必须区分实际单位，软换行不伪装成文档中的换行。同文档文本移动是一个编辑差量和一个 undo 项；范围端点沿该差量更新，不通过删除／重建语义文本节点来绕过范围保持。

原生编辑菜单继续使用既有 EditCommand／Paste 入口；DragDrop 和 Accessibility 来源仅为策略识别真实编辑来源。默认外来文本清洗不能被拖入或辅助技术入口意外绕过，所有入口仍应用相同的只读、受保护状态、字符／长度与 Reject／Transform 合同。Move 因部分接收或来源冲突只完成了复制时，通知必须反映实际结果，不能宣称来源已移动。未接通某个系统文本服务不能通过显示一个空动作冒充支持。

## 9. 原生调用、线程与 C ABI

### 9.1 同步查询的边界

现有 Window 将原生事件排队，在 owner 安全边界执行业务；accessibility 包含必须同步返回的查询和动作，不能把所有调用直接塞进异步事件队列后返回成功。

查询分为两类：已发布的普通属性／结构由 native 投影直接回答；按需虚拟条目和精确文本／场景查询通过受约束的 owner 查询入口回答。该入口只读已提交数据或不可变条目描述源，不执行 build、mount、render、State 写入、滚动、聚焦或窗口操作。

读取数据级描述函数是实际新增的同步查询合同，实施时必须写入 Window／UI spec；它不是允许任意原生回调执行用户业务的通用例外。若发生重入更新、数据不可用或描述函数失败，返回对应不可用／错误，错误报告回到正常 owner 路径。

### 9.2 动作执行与等待

- 原生调用来自非 owner 线程时，只访问原生投影，或把需要 UI 的请求交给现有 owner 队列。同步方法可以等候该次结果；等候期间不持有 provider／UI 锁，owner 不反向等待该调用。
- 原生调用已经在 owner 上时，不得等待自己的队列，也不得为完成一次请求嵌套 pump、render 或 run loop。普通查询直接读取快照；受约束查询只在允许的只读边界执行。
- 原生方法要求同步完成动作时，新增有限的 accessibility owner 派发入口：仅在没有进行中的业务回调、UI 更新、布局、动作或关闭过程时进入现有正常派发保护。它不是 render／layout 重入通道；可能打开原生菜单／对话框的后续操作仍沿现有延迟 native operation 路径。
- 当前边界不允许执行、或者该动作必须等待未来帧时，采用平台允许的异步完成方式；平台只允许同步完成且无法安全等待时，返回其适用的失败／不可用结果，不能伪造成功。

Linux D-Bus 方法可保留调用并在 owner 完成后回复；Windows COM 调用需要核实 apartment、线程和重入行为；AppKit 必须逐项确认同步 getter、动作和 setter 的实际返回合同。无返回值 setter 只能在后续状态与通知中体现真实修改，不提前改写 native 值。

内部请求状态仅服务当前调用的完成、失败与关闭清理，不新增公共 future／RPC 服务。同步 Realize 和跨线程关闭必须在平台原型阶段验证，不能等到所有控件改造后才发现 owner 无法推进。

本轮原生文本菜单和数据拖放同样存在同步查询／动作与 tracking loop。与其共同核查已有 owner 的可进入边界，复用必要的派发保护，不各建一套可重入事件循环。原生菜单条目或 drop callback 触发 UI 修改时仍不能进入正在执行的 render／layout；各协议分别满足自身返回合同，不因共用执行入口而统一返回“已排队”。

### 9.3 数据与生命周期

跨 ABI 只传固定宽度值、明确长度的 UTF-8 数据、类型化 ID 的原生表示和显式拥有的快照。Scoop 闭包及可移动对象通过现有 ForeignCallback／GC handle 规则保活；native 对象不保存裸 managed 地址。Objective-C／C++ 对象与异常不穿过 Scoop ABI。

native 保留语义属性投影和轻量代理，并按需持有当前调用；它不复制业务控制器。文本与数组的借用只在声明的调用边界内有效，跨调用保留时复制或使用已有显式资源所有权。

关闭顺序是停止接受新动作、退休节点和失效引用、完成／取消已接受请求、释放原生登记与 callback。客户端晚于窗口关闭释放对象是合法情况；此类对象只能回答“元素不可用”，不能保活整个 UiRoot 或访问已释放资源。清理继续使用已有 owner 收尾通路，不依赖已经停止业务投递的队列。

## 10. 平台适配

### 10.1 macOS

采用 Objective-C 的 `NSAccessibilityProtocol` 和 `NSAccessibilityElement`，把无独立 NSView 的语义节点接到现有内容视图；不以旧的 key-based informal API 作为主实现。不需要 Swift。

按实际控件角色实现名称、状态、值、关系、子项、动作、focus、hit test 和文本查询。利用 AppKit 的窗口与屏幕坐标转换，避免把 UI 内部逻辑坐标直接当作 AXFrame。

集合区分逻辑 rows、visible rows 及按行列取得条目的查询；按需产生轻量代理，不主动生成全部 View。显露、选中和焦点通过角色适用的 AppKit 动作完成，公共能力映射不要求与 UIA pattern 同名。VoiceOver 导航到未实现条目的真实行为是虚拟化验收的一部分。

### 10.2 Windows

通过现有 HWND 的 `WM_GETOBJECT` 对接 UIA provider，采用 C／C++ COM 实现 Simple、Fragment、FragmentRoot 及适用 pattern。运行期 ID、导航、焦点和屏幕物理坐标由该适配层转换。

按能力实现 Invoke、Toggle、Value、RangeValue、Selection／SelectionItem、ExpandCollapse、Scroll／ScrollItem、Text／TextRange。具有实际行列及标题语义的表格接入适用的 Grid／GridItem、Table／TableItem；普通列表不因此公布虚假的表格能力。虚拟集合另外实现 ItemContainer 与 VirtualizedItem；`Realize` 不等于 `ScrollIntoView`，更不等于 Focus。

placeholder 仅提供其实际支持的能力，不支持的完整属性按 UIA 合同返回错误。`FindItemByProperty` 明确区分不支持的 property 与查无此项，并支持合同要求的顺序枚举。已程序化隐藏的内容退出树；仍属于 UI 模型的离屏内容按 `IsOffscreen` 表达。

### 10.3 Linux

复用当前 GTK 4.18、libadwaita、GIO／GLib 与 owner 的 GMainContext。优先验证以公共 `GtkAccessible` 接入自绘对象，并使用适用的 AccessibleText／AccessibleRange 接口。不能把在线更新版 GTK 文档中的新增 API 直接加入 4.18 基线。

已经核查的 GTK 4.18 源码表明：普通实现 `GtkAccessible` 的 GObject 不自动获得完整 AT-SPI Action，Component 的 GrabFocus 及 EditableText 也存在接入限制。因此“实现 GtkAccessible 后所有动作自然可用”不是本设计的前提。

P0 用按钮、可编辑文本、焦点和虚拟条目验证完整路径，同时验证与 GTK 窗口、菜单、对话框树的衔接。公共 GTK 接口如果不能闭合这些能力，后续正式实现选择通过既有 GIO／GDBus 提供自绘子树的 AT-SPI provider，并验证其树嫁接；不依赖 GTK 私有 API，也不同时维护两套表达同一子树的生产实现。

直接 D-Bus 路线的树嫁接目前未经实测，不能预先标为已解决。如果两条路线都不能在现有基线上满足要求，先记录具体平台阻塞并调整该平台方案，不把缺少 Action／焦点的原型称为 Linux 完整支持。

AT-SPI 映射区分 VISIBLE 与 SHOWING；大集合只在正确提供 active-descendant 等配套行为后使用 MANAGES_DESCENDANTS。`libatspi` 是客户端库，不能作为已经实现 provider 的依据。提供语义接口也不等于取得 Wayland 全局截图或输入注入能力。

## 11. AI agent 的使用合同

能够读取原生 accessibility 的 agent 工具可以取得名称、角色、状态、关系、动作和有效几何，并通过相同原生动作修改 UI。工具若需要输出 snapshot，应由实际客户端适配实现，框架不增加与 OS 数据平行的 agent 状态。

快照可以按“当前窗口／模态上下文、可见节点、集合元数据及按需查询能力”组织，而不是序列化整个大列表。未实例化、离屏和隐藏必须保持第 7 节的区别；没有几何的节点不能被包装成可坐标点击的目标。

一个寻找远处列表条目的调用序列应为：查询集合能力 → 按支持的属性查找或枚举 → 取得 key 对应代理 → 按需 Realize／Reveal → 再查询实际控件与几何 → 执行动作 → 读取结果。每步都可能因数据变化失效，客户端应重新查询，不能依靠旧 index 或缓存坐标继续点击。

对语义 Invoke、文本 SetValue 等操作，框架不要求先伪造一次鼠标点击。仅使用截图和坐标的 agent 则需要先滚动到实际可见位置，使用正常输入，并通过画面或结构化观察确认结果。

验收覆盖两条实际路径：原生语义查询与动作能够完成任务；仅用画面、键盘和鼠标也能完成同一任务。前者不保证任意 AI 产品已经接入 Scoop，后者也不构成模型成功率承诺。

## 12. 实施批次与验收

### 12.1 先行规范与源码范围

每一批实施前，把该批实际采用的合同写入对应 spec，再修改实现与调用方：

| 文档／代码区域 | 需要的变化 |
| --- | --- |
| [UI spec](../../specs/SCOOP-UI-SPEC.md) | 语义声明、身份、树结构、提交、只读查询边界、隐藏与虚拟化；更新现有范围声明 |
| [Component spec](../../specs/SCOOP-UI-COMPONENT-SPEC.md) | 控件默认语义、覆盖值、数据级条目描述、行为与状态对应；菜单和拖放替代入口 |
| [Window spec](../../specs/SCOOP-WINDOW-SPEC.md) | accessibility 包、native 投影、同步派发例外、线程／返回值／关闭合同；复用公共系统偏好与原生菜单／拖放边界 |
| [Surface spec](../../specs/SCOOP-SURFACE-SPEC.md) | 仅补实际需要的新场景／命中／显露接入，不重做输入系统 |
| [文本 spec](../../specs/SCOOP-GRAPHICS-TEXT-SPEC.md)与 UI 文本章节 | 补实际缺少的范围保持及平台文本访问规则；已有布局算法保持复用 |
| [Theme spec](../../specs/SCOOP-UI-THEME-SPEC.md)与原生菜单规范 | 系统偏好的实际消费者、减少动画／对比度；ShowContextMenu、文本目标与原生 provider 归属 |
| `scoop.ui` 的 scope／host／layout／focus／lazy／text | 登记、发布、动作、几何与虚拟条目协调 |
| `scoop.ui.component` 与各目标 native 后端 | 控件接入及 AppKit／UIA／AT-SPI provider |

本次没有需要修改语言语义、runtime 对象模型或 compiler pipeline 的具体问题，因此不安排相应重构。

### 12.2 设计交付分组

下表保留各组的目标与完成要求；与交互打磨的合并拆分、前置和实际执行顺序见[统一实施计划](../interaction-polish/PLAN.md#3-批次与依赖)。

| 批次 | 交付内容 | 通过条件 |
| --- | --- | --- |
| P0：平台边界原型 | 最小原生节点、同步查询／动作／关闭；Linux 路线判断；与交互打磨共同核查偏好和 native tracking | 证明 owner 可安全推进，真实客户端可读取并操作；记录每项返回合同及平台限制 |
| P1：macOS 纵向闭环 | 语义资源、发布、AppKit 根及 Button／Label／Checkbox／Input | VoiceOver 与 AX 客户端完成读取、激活、编辑；禁用、隐藏及关闭正确 |
| P2：交互与文本 | 表单关系、复杂焦点、模态／Portal、文本范围、选区、受保护值、通知；文本菜单和已接入拖放的替代操作 | 真实辅助技术完成表单、菜单和弹层任务，Binding／编辑策略拒绝不假报成功，键盘可完成等价操作 |
| P3：虚拟集合 | 条目描述源、代理、按 key Realize／Reveal、集合选择与数据变化 | 十万行保持局部实例；可找到并操作未实例化条目，排序、删除与焦点正确 |
| P4：其余平台 | Windows UIA 与经 P0 选定的 Linux 实现 | 相同用例经 Narrator／UIA、Orca／AT-SPI 通过；只声明实际完成的平台 |
| P5：覆盖与工具使用 | 现有控件补齐、诊断可读性、实际 agent 客户端接入验证 | 可见节点与集合查询、语义动作及截图输入完成明确任务，无第二套状态 |

P0 不要求提前实现全部控件，但同步动作与 GTK 接入结论必须足以支持后续路线。Windows 后端未完成的相关基础单独列出依赖，不用 macOS 通过代替其他平台验收。

统一计划从 S0 验证原生边界、S2 交付公共语义与 macOS 基础闭环；S5 完成文本／表单／焦点，S7 完成虚拟集合，S10 跟随公共接口完成其余平台，S11 覆盖全部已有控件及工具使用。系统偏好与纯键盘视觉体验可独立先交付，不能因 Linux provider 路线尚未确定而推迟所有平台的这些修复；后者仍保留为核心交付的未完成项。

### 12.3 必要验证

普通身份、树更新、合并、动作资格、数据查询与文本映射使用所属 Cone 测试。需要真实 OS provider、跨进程客户端、原生事件循环或窗口行为的验证使用外部集成测试；不为普通 API 复制正式 CLI fixture。

| 场景 | 核心断言 |
| --- | --- |
| 基本语义 | 本地化名称与 automationId 分离；标签／错误关系正确；装饰不重复，独立交互子项不丢失 |
| 状态与动作 | disabled、readOnly、Binding Reject／Transform、重复设同值、handler 自删或关闭窗口，结果与实际状态一致 |
| 帧与坐标 | Accepted／Discarded、窗口移动、缩放、部分裁剪、RTL 与旧布局失效，getter 不提供虚假点击点 |
| 焦点与模态 | 读属性不抢焦点；active descendant、键盘焦点、选择分离；背景动作不能绕过模态 |
| 虚拟集合 | 可见约二十行时接入不会 build 十万行；元数据查询不 build；只实现被请求项；不支持搜索不返回 NotFound |
| 数据变化 | 按 key 重定位；重排不串项；删除与过滤使旧请求失效；View 回收不等于逻辑删除 |
| 文本 | emoji、组合序列、UTF-16 转换、软换行、范围端点变化、composition、只读与密码文本 |
| 菜单与数据拖放 | 右键／键盘／ShowContextMenu 指向同一文本目标；原生条目不重复；Drop 与多击只通知实际差量，失败不删源，替代操作可完成真实任务 |
| 系统偏好与视觉 | 常显条、减少动画、高对比度、减少透明度、caret、文字缩放热切换；逻辑范围与回弹几何一致，隐藏条不移除 Scroll／Reveal |
| 触摸与键盘 | 不依赖 hover 或精确拖动的实际入口，放大命中不越界；平台辅助触摸探索、普通 Tab 和焦点恢复分别验证 |
| 原生生命周期 | 跨线程查询、同步 Realize、重入拒绝、窗口关闭、迟到 callback 和客户端保留旧对象均安全结束 |
| 空闲行为 | 单纯名称／状态更新不请求绘制；没有变化时无持续轮询、重复整树通知或帧唤醒 |

最终验收必须包含 VoiceOver、Narrator、Orca 中对应已交付平台的实际使用记录，以及 AX／UIA／AT-SPI 客户端对真实业务状态的断言。树结构能被检查器显示，只证明部分接入，不足以证明导航、动作或文本编辑可用。

原始日志放在仓库外；实施记录仅保存范围、结果、失败闭合情况与必要平台限制。本设计的完成不代表上述实施批次已经通过。

## 13. 主要依据

- [仓库调研与源码依据](../ACCESSIBILITY-RESEARCH.md)：当前能力、排除项与平台核查边界。
- Apple：[NSAccessibilityProtocol](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol)、[NSAccessibilityElement](https://developer.apple.com/documentation/appkit/nsaccessibilityelement-swift.class)、[自定义控件](https://developer.apple.com/documentation/appkit/custom-controls)、[逻辑 rows](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol/accessibilityrows())、[visible rows](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol/accessibilityvisiblerows())。
- Microsoft：[UIA provider](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-providersoverview)、[patterns](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-controlpatternsoverview)、[虚拟条目](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-workingwithvirtualizeditems)、[FindItemByProperty](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-iitemcontainerprovider-finditembyproperty)、[IsOffscreen 等属性](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-automation-element-propids)。
- GTK：[Accessible](https://docs.gtk.org/gtk4/iface.Accessible.html)、[AccessibleText](https://docs.gtk.org/gtk4/iface.AccessibleText.html)、[AccessibleRange](https://docs.gtk.org/gtk4/iface.AccessibleRange.html)、[无障碍架构](https://docs.gtk.org/gtk4/section-accessibility.html)。4.18.6 的具体限制见 [Action](https://github.com/GNOME/gtk/blob/4.18.6/gtk/a11y/gtkatspiaction.c)、[Component](https://github.com/GNOME/gtk/blob/4.18.6/gtk/a11y/gtkatspicomponent.c)、[EditableText](https://github.com/GNOME/gtk/blob/4.18.6/gtk/a11y/gtkatspieditabletext.c)。
- AT-SPI：[状态定义](https://docs.gtk.org/atspi2/enum.StateType.html)及[客户端库定位](https://gnome.pages.gitlab.gnome.org/at-spi2-core/libatspi/)。
