# Scoop 自绘 UI 框架规范

日期：2026-11-03。状态：原有自绘框架、公共文本适配、独立基础控件／容器、主题展示及宿主／后台下载组合已完成 macOS aarch64 与 x86_64 Rosetta 验证，详见 [UI 实施记录](../ui/PROGRESS.md)。新增场景与基础交互能力按[本期实施记录](../ui/foundations/PROGRESS.md)分批验收。本文件规定框架公开合同；公共文本能力另见 [文本排版规范](SCOOP-GRAPHICS-TEXT-SPEC.md)，基础控件另见 [控件库规范](SCOOP-UI-COMPONENT-SPEC.md)，取舍、内部组织及实施顺序见 [UI 设计](../ui/DESIGN.md)。现有底层库以 `scoop.surface` 命名，已实现的绘图、窗口呈现及输入合同见 [Surface 规范](SCOOP-SURFACE-SPEC.md)。

## 1. 定位与依赖

`scoop.ui` 是普通 Scoop Cone，提供声明式组合、可观察状态、组件身份、自绘元素协议、测量／布局接入以及资源生命周期。它依赖实际使用的 core／std／math／platform、`scoop.graphics`、`scoop.graphics.text`、`scoop.surface` 和 `scoop.window`。窗口事件循环、呈现后端、动画求值、文本 shaping 和 GC 仍由对应下层负责。

框架不提供 Label、Button、文本样式、点击判定或 Row／Column／Stack 等现成控件和布局策略。它提供实现这些控件所需的公开能力。`scoop.ui.component` 是独立的普通 Cone，使用与第三方控件相同的公开接口构建基础控件；`scoop.ui` 不依赖、转导出或识别该控件库及独立 `scoop.ui.style` 中的类型。公共主题绘图／几何与装饰／效果由[样式库](SCOOP-UI-STYLE-SPEC.md)通过相同公开协议实现，实际滤镜与图层由 [graphics](SCOOP-GRAPHICS-EFFECTS-SPEC.md)负责。只依赖框架的应用可以完成自绘、输入和动画。

首期只实现和验证 macOS：先完成 aarch64 的各批功能，最后统一补验 x86_64；Windows 接入与验收留待后续，不因底层已有 Windows renderer 就声明 UI 已支持该平台。不发布空 renderer。实施批次及定向验证见 [UI 实施记录](../ui/PROGRESS.md)。

静止时停止渲染工作是首期基本要求，见[第 6.1 节](#61-静止时停止渲染工作)。没有待处理更新、绘制失效或需推进的动画时，框架不维持周期性的逻辑遍历、绘制回调或 GPU 提交；该合同同时适用于桌面与未来移动端。

离屏 UI 根、外部 GPU 图像、UI 与 3D 场景的双向合成及视频展示列为后续增强，调研结论见 [UI 设计第 13 节](../ui/DESIGN.md#13-后续增强gpu-互操作3d-与视频)。这些能力尚未实现，不属于本文件的首期合同；进入相应实施阶段时再补充具体 API、线程与资源交付规则。

SkSL runtime effect 同样列为后续 graphics 增强，见[接入设计](../graphics/sksl/DESIGN.md)及 [UI 设计第 14 节](../ui/DESIGN.md#14-后续增强sksl-与组件效果)。组件通过普通 Element 使用效果，Surface Animatable 驱动 uniform；持有 shader 或声明时间参数不产生隐式帧需求，继续遵守第 6.1 节的停帧要求。

弹层／模态、滚动及绘制／实例缓存、虚拟子项、键盘导航、快捷键、路由扩展与公共编辑的后续方案见[基础交互能力设计](../ui/foundations/DESIGN.md)。该方案分批实施；场景与纯绘制协议见第 7.2 节，其他能力进入相应批次时再修订当前合同。

| Cone | 职责 |
| --- | --- |
| `scoop.window` | 原生窗口、事件循环、输入归一、IME 和 native lease |
| `scoop.graphics` | 几何、Canvas、Path、Image、绘图目标、图像滤镜与图层合成 |
| `scoop.graphics.text` | 字体、富文本内容与样式、Paragraph／TextLayout、行内对象排版、文字绘制与查询 |
| `scoop.surface` | SurfaceHost／SurfaceWindow、窗口呈现、帧机会、动画、InputArea 和区域焦点 |
| `scoop.ui` | Component／Element／LazyElement、State／Props／Binding、挂载树、测量与绘制接入、输入／动画的挂载归属；text 子包提供公共段落适配 |
| `scoop.ui.style` | 公共主题绘图／几何、视觉状态及语义参数；通用 Style、外框装饰与内容效果 |
| `scoop.ui.style.macos／material／fluent` | 三套公共协议实现与各自 Light／Dark |
| `scoop.ui.component` | Label、Button、Checkbox、Row／Column／Stack、Frame、Absolute、ScrollArea、VirtualList、控件专用样式及交互外观 |

表中是目标分层；字体与 Paragraph／TextLayout 已迁入 graphics.text，富文本／行内对象、独立自绘框架及公共适配 `scoop.ui.text` 已完成。适配直接依赖 graphics.text，只连接文本排版与正常子项，不提供 Label 或默认样式，也不改变 View 的基础形状。

组件、状态及描述使用普通 class／interface／泛型／属性委托。首期不增加 `@Composable`、result builder、属性宏、关联类型、UI 专用 IR 或 runtime 入口。`scoop.ui` 的类型身份查询复用语言规范 9.6 的 `typeInfo<T>()`，不遍历组件字段，不按名称访问私有属性。

主题来源复用 State／Context，见[主题规范](SCOOP-UI-THEME-SPEC.md)。主题更新作为普通 Props 和布局输入进入原节点，不改变框架描述形状；局部主题通过显式参数传递，临时 Context 不成为挂载树环境。主题绘图不读取 State，也不注册输入。

原有独立自绘控件、Counter、条件组件和可重排列表已经交付；当前按基础交互设计增补虚拟列表和公共文本编辑。完整富文本编辑器、手势竞争、原生控件封装、accessibility 树、并发／可中断渲染和自动 memoization 不进入本期 API。

## 2. 组件与描述

组件的公开协议为：

```kotlin
public interface Component {
    fun render(): View
    fun onMount(scope: ComponentScope): Unit = Unit
    fun onUnmount(): Unit = Unit
}
```

`onMount`／`onUnmount` 的默认行为明确为不执行操作；组件只需实现 `render`。使用方式：

```kotlin
import scoop.ui.*
import scoop.ui.component.button

class Counter: Component {
    private var count: Int by state(0)

    public override fun render(): View = button(text = f"Count: ${count}", onClick = { count += 1 })
}

fun main() {
    runUiApplication(title = "Counter", create = ::Counter)
}
```

组件采用持续存在的 class 实例。`render` 可以执行多次，实例及成员 State 不因重建描述而重新构造。用户不为 Component、Element 或使用的控件逐个调用 `use`／`close`。示例中的 button 来自独立控件库。

无状态组合可以写成普通的返回 View 的函数；这种调用不创建独立的挂载身份，其间的 State 读取归调用者当前的 render。需要独立本地状态和生命周期时使用 Component 及第 5 节的 component 入口。

`View` 是 `scoop.ui` 内定义的封闭描述类型，以不可变 class 保存本次输入、子描述或回调。其形状包括空内容、Component、Element、LazyElement 和 Portal 描述。`empty()` 表示没有内容，测量采用约束下限，宽松约束下为零尺寸，不产生绘制或输入区域。一个描述不拥有 InputArea、TextLayout、动画或 GPU 资源；描述本身没有 `close`。

Component 负责组合描述。完全自绘及自定义子项布局通过普通公开接口 Element 实现，外部库不需要继承内部挂载节点或修改 View 的分支：

```kotlin
public interface Element {
    fun measure(context: MeasureContext, constraints: Constraints): LayoutMetrics
    fun scene(context: SceneContext): Unit
    fun onMount(scope: ComponentScope): Unit = Unit
    fun update(): Unit = Unit
    fun onUnmount(): Unit = Unit
}
```

Element 同样采用持久 class。measure／scene 是必需操作；叶子元素的 measure 不访问子项，容器通过公开 context 测量、放置和绘制其子项。update 在首次 onMount 成功后及每次 Props 或子描述更新后同步执行，负责更新输入开关、处理器及动画目标；隐藏或没有 drawable 时也执行。它不执行绘图，也不在其中重入布局。Element 不自行拥有或关闭子组件，子树继续由框架管理。

同一描述值可被使用多次；每个出现位置分别挂载，不能按描述对象引用相等而合并挂载。描述不可变指保存的属性槽及子序列不可修改，不承诺其所引用的业务模型深度不可变。子序列以不可变 Array<View> 保存，允许复用该数组，不因重建描述而强制复制。

`render` 只读取当前输入／状态并产生描述。它不得写 State、操作窗口、取得需要关闭的原生资源、启动任务或登记订阅。measure／scene／draw 允许维护元素自己的测量和绘制缓存，但不得修改 State 或启动动画。State 写入口在活动 render／measure／scene／draw 期间报告状态错误；其他副作用遵循普通 API 的使用约定，不新增编译器纯度检查。Component、Element 与 LazyElement 的构造器只初始化托管状态和保存输入；资源取得放在挂载回调或元素所属的测量缓存中，归属见第 8 节。

## 3. State、变化通知与依赖

### 3.1 状态对象与订阅

`State<T>` 是可变引用类型，提供普通 `value` 属性，并实现属性委托的 `getValue(thisRef: Any): T`／`setValue(thisRef: Any, value: T): Unit`。两个方法均为普通、非泛型、非挂起的 operator。`thisRef` 不用于推断属性名、查找宿主字段或决定状态身份。`state(initial)` 每次调用创建一个新的 State；它没有 remember／hook 槽位语义。

State 的持有者与 UI 订阅者分别确定。一个 State 可以在任何组件挂载前创建，也可以被多个挂载节点读取；不把创建者或属性委托的 thisRef 当成唯一订阅者。组件自有 State、外部 State 的属性委托，以及共享模型中的可观察属性使用同一读取追踪和通知机制。状态值 T 可以是 struct、class 或其他合法类型；把 State 做成 class 不取消读取值类型 T 时的普通值语义。

状态对象绑定创建线程。值的读写及订阅维护在该线程执行；当前 UI 还要求该线程是窗口 owner。后台工作通过既有 owner 投递机制返回结果，不直接修改 UI 状态。相同 owner 上的多个 UiWindow 可以观察同一状态；不同 owner 不共享这套无锁状态对象。

一次 `render` 使用一个库内、同步的依赖收集作用域：

1. State getter 将当前组件节点记入本次读取集合，同一 State 的重复读取只登记一次。
2. 成功执行后，以本次集合替换该组件的旧订阅；不再读取的 State 不再使它失效。
3. setter 先保存新值，再将当前订阅者标记为 dirty；不在 setter 内递归调用 render。
4. 组件卸载时解除所有订阅，并从待更新集合移除；共享 State 本身仍然可用。

首期每次合法 setter 调用都发出变化通知，包括写回相同值。调度器合并同一轮更新，不为任意 T 引入隐式相等、深比较或接口约束。相等策略和专用集合增量通知以后由实际性能需求决定。

同一同步业务操作中的多次写入分别立即生效，UI 工作在既有安全边界合并；这不提供跨挂起点的事务或状态快照。直接订阅粒度是实际读取的 State；父组件生成新的子描述时仍沿第 4 节更新子项，不承诺只执行直接订阅者的 render。

普通 getter／函数调用中的间接 State 读取，同样归当前活动 render。render 之外的读取只是取值，不产生永久订阅。首期不自动建立 update／measure／scene／draw 阶段的独立依赖图；声明式自绘组件在 render 中读取 State，再通过 Element 的 Props 传入本次绘制数据。Element 自己在输入处理中修改的普通交互字段，通过 scope.requestFrame 显式请求呈现。Surface Animatable 的采样走第 7 节的帧通路，不需要先转换成 State。

普通 class 字段不是可观察属性。对 `modelState.value.name` 赋值不会经过 `modelState.value` 的 setter；`name` 应有自己的可观察属性，或调用方显式赋回状态值。默认 setter 不按引用相等跳过通知。对普通可变集合的原地修改也遵循此规则。

### 3.2 自有状态与共享 Store

Store 是应用定义的普通 class，按业务组织一组 State 或可观察模型。框架不要求 Store 基类、字符串 key 注册表或独立 action／reducer 协议。固定业务字段使用普通属性和类型检查；共享一个 State 直接写成 `var count: Int by store.count`，其中 store.count 的类型为 State<Int>。若 Store 自己使用委托属性，组件也可以在 render 中通过普通 getter 读取，或通过第 4 节的 Binding 连接写入。

| 声明 | State 的持有者 | 卸载后再次挂载 |
| --- | --- | --- |
| `var count: Int by state(0)` | 当前组件实例 | 新组件创建新的 State，使用初始值 |
| `var count: Int by store.count` | Store；组件借用同一 State | Store 仍保留该对象时，继续使用已有值 |

两种声明可以出现在同一个组件中。State 本身是托管对象，无需逐个 use；共享 State 不因任一订阅组件卸载而重置或关闭。应用级 Store 可跨页面或窗口保留，文档级 Store 可随文档结束而释放；资源和任务的实际归属遵守第 8 节。

`by` 表达式沿语言规范 9.2.1 只在该属性初始化时求值一次，随后访问保存的 delegate。Store 引用或用于选择 State 的业务输入后来变化，不自动使委托换绑；变化输入应在 render 中经 Props 读取，或按第 5 节更换挂载身份。`val count: Int by store.count` 只禁止通过这个属性赋值，不冻结共享 State 的后续值。普通业务方法可以集中约束写入，框架不要求额外命令语法。

### 3.3 Context 与挂载时绑定

应用可用语言已有的 [Task-local Context](SCOOP-SPEC.md#83-task-local-context) 提供 Store，通过计算属性或普通辅助函数声明依赖。Component／Element 的协议和构造器语法不变；不在 override 上增加与接口不同的 Context requirement。以下示例同时使用共享与组件自有 State：

```kotlin
import scoop.ui.*
import scoop.ui.component.button

class AppStore {
    val count: State<Int> = state(0)
}

context(store: AppStore)
val currentAppStore: AppStore
get() = store

class SharedCounter: Component {
    private var count: Int by currentAppStore.count
    private var localClicks: Int by state(0)

    public override fun render(): View =
        button(
            text = f"Shared: ${count}; local: ${localClicks}",
            onClick =
                {
                    count += 1
                    localClicks += 1
                }
        )
}

fun main() {
    val store = AppStore()
    context(store) {
        runUiApplication(title = "Shared state", create = ::SharedCounter)
    }
}
```

此处 Context 在实际创建 SharedCounter 时提供 Store，随后组件直接持有 State。Context 按精确静态类型查找；同类型的多种业务角色使用普通 nominal wrapper，缺失按既有 MissingContextException 处理。Context 绑定快照保留 Store 的引用，不复制其中的状态，也不赋予跨线程读写权限。

应用级绑定覆盖 Application.run／runWithAsyncio 或单窗口便利入口；窗口业务回调沿窗口规范恢复运行时的 Context。普通闭包、函数引用、View 描述与 UiHost.attach 不因创建或调用于某个临时 Context scope 而自动捕获它。仅在 scope 中返回一个延迟 View，不保证稍后挂载时仍能取得 Store；延迟入口应显式捕获所需对象并在执行时安装 Context，或使用覆盖实际执行过程的外层绑定。

不同窗口需要不同依赖时，应用在相应窗口的实际业务／挂载入口安装绑定，或显式传递模型；不把 Context 的调用作用域当成挂载子树的环境继承。重新绑定 Context 不通知 State 订阅者，也不替换已保存的 delegate。首期只提供这种挂载时绑定，不新增 UiLocal、Provider 描述、节点环境快照或自动切换整棵子树 Store 的协议。

## 4. Props 与 Binding

### 4.1 变化输入

组件的构造器参数在实例生命周期内只初始化一次，不能用每次变化的构造器捕获来传递更新。需要变化的父级输入使用 `Props<P>`：它是只读引用容器，公开 `value: P`，由框架在 Component 本轮 render 前或 Element.update 前更新。

```kotlin
import scoop.ui.*
import scoop.ui.component.label
import scoop.ui.component.column

class Greeting(private val props: Props<String>): Component {
    public override fun render(): View = label(props.value)
}

class Screen: Component {
    private var name: String by state("Scoop")

    public override fun render(): View =
        column(
            [component(props = name, create = ::Greeting), component(::Counter)],
            spacing = 12.0
        )
}
```

`Props<P>` 由组件节点持有，不允许子组件替换父级输入。它是当前输入的入口，不是自动保存旧版本的快照。需要异步操作使用启动时的参数时，应先复制需要的值到局部 `val`；要在调用时读取最新输入的回调，则在回调体内访问 `props.value`。

父组件产生新的子描述时，框架更新 Props 并执行对应的 render 或 update；初期不根据 P 的相等或引用身份跳过。每个节点在同一逻辑更新轮中至多执行一次；与该轮处理同时登记的新失效留到下一轮。只读输入本身不产生第二套可观察模型。

### 4.2 可写绑定

`Binding<T>` 是带 getter／setter 的普通引用对象，通过 `value: T` 提供读写，用于明确允许控件修改外部状态。可以由显式 State 的 `binding()` 取得，也可使用 `binding(get = { count }, set = { count = it })` 连接委托属性。Binding 不复制状态、不保存另一份当前值；getter／setter 继续执行原 State 的追踪与线程规则。绑定到普通不可观察字段不会自动产生变化通知。不增加 `$property` 或取得隐藏 delegate 字段的语法。

随 Props 传入的 Binding 可以变化。控件每轮 render 读取当前绑定，在交互时使用当前 Props 中的写入路径；不能在构造器中用 `by props.value.checked` 固定第一次输入。成功 render 后沿第 3 节替换依赖，新绑定的后续变化生效，旧绑定不再单独使该控件失效；控件自身的交互状态和挂载身份按通常 Props 规则保留。

### 4.3 控件属性与派生输入

公共访问权限与可观察性分别由普通属性声明和 State 决定。实际持有合法组件实例或普通模型时，读取其公开 getter 内部访问的 State，会订阅当前 render 的挂载节点。View／component／控件便利函数返回的是描述，首期不提供获取挂载实例的 ref 协议，也不允许从 View 直接访问某个实例的 checked。

普通只读输入如 Button.enabled 通过 Props 接收值；用户交互可修改的值如 Checkbox.checked 通过 Binding<T> 接入外部状态；hover／pressed 等内部表现使用私有 State。多个公开状态可以组织为普通模型对象，不要求每个控件另建专用状态类型。组件自有状态与外部 Binding 的具体入口由控件库定义，见[控件库规范](SCOOP-UI-COMPONENT-SPEC.md)。

Checkbox 与 Button 联动时，组合组件在 render 中读取 checked 的 State 来计算 enabled，Checkbox 写入同一个 State。直接订阅者是执行该读取的 render；组合组件重建后通过 Props 更新 Button，不需要另存 enabled State。普通赋值 `button.enabled = checkbox.checked` 只赋值一次；以另一个属性初始化 State 也只取得初始值。框架不把任意赋值或初始化转换成永久属性绑定。

## 5. 创建、身份与复用

组件构造发生在挂载时，公开入口的签名形状为：

```text
fun <C : Component> component(
    create: () -> C, key: ViewKey = ViewKey.Position
): View
fun <P, C : Component> component(
    props: P, create: (Props<P>) -> C, key: ViewKey = ViewKey.Position
): View
fun <E : Element> element(
    create: () -> E, children: Array<View> = [],
    key: ViewKey = ViewKey.Position
): View
fun <P, E : Element> element(
    props: P, create: (Props<P>) -> E,
    children: Array<View> = [], key: ViewKey = ViewKey.Position
): View
fun portal(
    content: View, options: PortalOptions = PortalOptions(),
    key: ViewKey = ViewKey.Position
): View
```

`ViewKey` 是包含 Position、Text(String)、Number(Long) 的值类型。Position 表示采用同一父级子序列中的位置；另外两个分支是显式 key。

`component`／`element` 本身只构造描述，不能立即调用 create。首次挂载才调用工厂，并保存其返回实例；之后匹配到同一节点时更新 Props 及子描述，复用实例。无 Props 的入口统一使用 Unit 输入。Element 的 children 由框架按同样规则挂载，空数组表示叶子。

节点形状包含 Component／Element／LazyElement 类别、`typeInfo<C>().type` 或 `typeInfo<E>().type`，以及 `typeInfo<P>().type`。用完整 TypeRef 相等比较，不比较展示名、函数值引用、描述对象引用、源码行号或分配地址。P 是内部泛型承载的组成部分，输入类型改变时重新挂载，不能把旧的 `Props<P>` 强转为另一类型。

Portal 是独立的描述类别，不创建用户 Component／Element 实例。相同位置／key 的 Portal 复用同一层会话，更新 options 和唯一子描述；子内容仍按普通规则匹配、挂载和释放。层显示位置不改变逻辑 parent，跨根不共享挂载或输入区域。已关闭会话不会因普通 Props 更新自动重新打开；重新呈现需在旧描述退休后再次挂载，或显式选择新的 key。

C／E 必须保留 class 类型。库使用既有的一层 TypeShape 描述拒绝接口、object 或值类型作为具体实例类型，不要求新的编译器诊断。相同静态类型下改变工厂闭包或返回子类，并不改变已挂载实例；工厂只负责初次创建。需要选择不同实现的调用方应保留不同的具体类型，或显式更换 key。

同一个 Component、Element 或 LazyElement 实例只能属于一个挂载位置。工厂必须为新挂载创建新实例，不能返回某个已挂载的单例。框架检查当前活动实例的重复挂载，不为已经退休的对象保留全局历史表；为新挂载返回新实例是 create 的使用合同。共享业务状态应通过普通模型／Props／Binding 表达。

匹配规则如下：

| 情况 | 结果 |
| --- | --- |
| 同一父级、同一显式 key、同一组件形状 | 复用原实例；允许在兄弟间移动 |
| 没有显式 key、同一父级及位置、同一组件形状 | 复用原实例 |
| 组件形状或显式 key 改变 | 卸载旧实例，挂载新实例 |
| 离开父级，或本轮描述中消失 | 卸载；之后出现时创建新实例 |
| 仅尺寸、布局位置或绘制属性改变 | 保留实例与状态 |
| 同一父级出现重复的显式 key | 报参数错误，不能任意复用其中一个 |

key 的有效范围仅为直接父级，不是全局 ID；移动到另一父级不保留挂载。同一兄弟序列中显式 key 必须唯一，不因节点类型不同而允许重复。Position 以完整兄弟序列的索引计算；混用 keyed／unkeyed 节点时，插入前方兄弟仍会改变后者的位置。

列表示例应以稳定业务 ID 构造 `ViewKey.Number(item.id)` 或 `ViewKey.Text(item.id)`。普通 `if`／循环没有隐式源码身份：两个分支在同一位置返回相同组件形状及 key 时保留状态；需要区分时显式使用不同 key。容器的插入、删除或替换可能改变父级身份，不能承诺跨该结构变化保留子状态。

## 6. 重建、布局与呈现

State 变化合并到 owner 上的 UI 工作队列。事件／业务回调退出后，经现有 ApplicationProxy 投递处理；不在活动输入派发、render、layout 或 paint 内递归重建。祖先和后代同时失效时，由祖先处理同轮子描述，并避免重复执行后代 render。没有排队更新时不投递任务，也不在 AboutToWait 遍历树。

UI 逻辑提交与 surface 的呈现提交有不同的时机：

- 逻辑更新执行 render、匹配节点、Props 替换、Element.update、挂载／卸载及订阅更换。隐藏、遮挡、零尺寸或暂时没有 drawable 不能阻止这一过程及资源清理。
- 需要显示的变化向 SurfaceWindow 请求帧。在真实绘制机会中，Surface 先采样动画，再由 UI 遍历挂载树执行测量／布局、场景装配、纯绘图块执行和 InputArea 放置。
- 只有 surface 成功接受提交后，才按 Surface 规范发布命中几何；失败或 Deferred 不发布新几何，不回滚已经生效的业务状态。

首期在 owner 上同步完成一轮逻辑更新，不提供并发、暂停恢复或丢弃后重放 render 的机制。所有更新使用当前可变状态；不承诺 React 式的不可变状态快照。用户 render／构造／挂载回调失败时走第 9 节的关闭路径，不试图回滚用户修改过的任意对象。

新描述中的事件处理器随逻辑更新替换。尚未提交新的画面时，保留最近已提交的几何与节点身份，派发使用节点最新的处理器。新节点在首次成功放置前不能命中；已卸载节点立即退出交互，不能为了等下一帧而继续接收事件。这里承诺几何对应最近提交的帧，不承诺处理器保留旧帧捕获的输入值。

普通元素在每次实际呈现中测量其直接子项，LazyElement 则按第 7.8 节只准备和测量所需项；昂贵的文本／路径计算由实现它的 Element 按实际输入缓存。这样 Surface 动画可以直接参与本帧尺寸和位置计算，不要求 UI 为每个动画另外建立布局失效订阅。逻辑更新仍按需执行，不因动画每帧采样而重新创建描述或重跑所有 Component.render。

所有自绘元素共享所属窗口的 FrameContext 和渲染目标，不为每个元素创建窗口、renderer 或独立 Surface。窗口 resize／DPI 变化在下一帧进入相同的测量／绘制流程，不重建组件实例。普通元素的子序列使用显式 `Array<View>`，按需元素使用 ItemSource；不把普通 lambda 的未使用表达式冒充 result builder。

### 6.1 静止时停止渲染工作

UI 采用按需调度。一个根在逻辑工作队列为空、没有尚待呈现的失效、没有需采样的运行中动画或待处理的动画完成回调，并且没有新的宿主重绘请求时进入静止。最后的必要帧、已排队通知和实际 GPU 使用正常收尾后，只要没有新的工作来源，就必须保持以下行为：

- 不调用 Component.render、Element.update、measure 或 paint，不执行树匹配、布局遍历、文本重排及输入几何重建。第 6 节的“每次实际呈现都遍历”只规定有帧时的处理，不构成持续生成帧的理由。
- 不为检查状态、Props 或动画是否变化而定时扫描，不投递空的 UI 更新任务，不保留持续运行但每次直接返回的帧回调。State 订阅和挂载树可以保留，变化由实际写入或事件通知。
- 撤销所属 Surface 的帧需求，停止原生帧通知的持续唤醒；不获取新的绘制目标、借用 Canvas、录制绘图命令或调用 submit／present，也不反复呈现同一静态画面。
- UI 不以 Poll、固定周期 timer 或不断重新投递的回调维持空转。没有其他应用工作时，宿主阻塞等待事件；真实 I/O 或未来一次任务的 deadline 可以使用既有等待机制，但其唤醒不自动引发 UI 重绘。

首次挂载、State／Props 失效、显式 requestFrame、动画启动／恢复，以及系统要求重画、窗口尺寸／DPI／主题变化，都可以重新唤醒相应工作。多个请求按已有规则合并，处理中产生的新失效不得丢失；工作完成后再次回到静止。第 3 节的 State 同值写入仍属于显式失效，本节不改变 setter 语义；组件或应用主动持续请求帧属于持续工作来源，框架自身不能凭空产生这种请求。

持有 Animatable 不代表动画正在运行。完成、暂停、取消和卸载应及时结束其持续帧需求；最终值及完成回调引起的必要变化完成呈现后停帧。运行中的显式重复动画可以持续请求帧，已完成动画不能因还保存在 scope 中而继续驱动时钟。

需求按根／窗口维护。一个窗口的动画不能使其他无变化窗口执行 render、布局或提交；没有有效 drawable 时保留失效并等待原生恢复，不通过循环 acquire 或逻辑任务自我投递重试。隐藏窗口的真实 State 更新和卸载仍按需处理，时间轴恢复语义沿 Surface 规范，不需要在隐藏期间持续采样。

未来视频帧到期或 3D 场景的显式持续运动属于外部绘制需求，接入时同样必须能够撤销。暂停视频、静止场景以及外部宿主继续转动物体但 UI 纹理未变，都不能使 UI 自行维持无用帧。首期 Surface 的停帧合同见[第 4.1 节](SCOOP-SURFACE-SPEC.md#41-空闲帧需求与原生等待)。

本要求约束 UI／Surface 引起的周期唤醒和渲染工作。已有 GPU 命令的完成、系统合成与显示扫描、应用独立的音频／网络任务不等同于 UI 持续渲染；不以等待 GPU 空闲来阻塞普通帧，也不以整个进程或桌面的 CPU／GPU 读数绝对为零作为合同。

## 7. 自绘、布局、输入与动画

### 7.1 测量及子项布局

`Constraints` 按轴保存有限非负最小值和可选的有限最大值，最大值存在时不得小于最小值。缺少最大值表达无界，不把 Infinity 传给 graphics 或 graphics.text。`constrain(size: Size): Size` 将一个有限非负期望尺寸收敛到约束。Element.measure 返回 LayoutMetrics，其 size 必须满足收到的约束：

公开字段为 minWidth、maxWidth、minHeight、maxHeight，默认最小值为 0.0，最大值为 None；`Constraints.tight(size)` 同时固定两轴。与 graphics 的几何值一样，在测量或 constrain 操作边界检查值的合法性。

```kotlin
public struct LayoutMetrics(
    val size: Size,
    val alphabeticBaseline: Option<Double> = None,
    val ideographicBaseline: Option<Double> = None
)
```

基线是相对元素顶边的有限有符号偏移，文本采用首行基线；None 表示元素没有该基线。外框裁剪不要求基线落在其高度范围内。空内容采用约束下限尺寸和两项 None。Component 原样透传子描述的尺寸与基线；容器可以根据子项偏移给出自己的基线，未定义时返回 None，不隐式扫描后代。框架不扩展成通用 alignment-line 系统。

MeasureContext 提供本帧 time、子项数量、`measureChild(index, constraints): LayoutMetrics` 和 `placeChild(index, origin: Point)`。Element 在自己的 measure 中决定子约束、位置和最终 LayoutMetrics。首期每个直接子项测量一次，随后可以放置零次或一次；没有放置表示此次布局不呈现该子树。每轮测量先清空旧放置，不能把上帧位置遗留给本帧未放置项。结果属于本次测量，不跨回调保存 context、子节点句柄或旧索引。

子项保持描述中的逻辑顺序，框架根据父级放置累计绘制坐标。未放置的子树保留挂载、Props、State、订阅和资源，继续接收逻辑更新；该子树本帧不执行 scene，也不放置 InputArea。祖先未放置时，后代在其内部测量中取得的位置也不会使它们单独呈现。该协议支持行内对象先测量、再根据段落结果选择放置，不要求多轮 intrinsic 协商。

根约束的最小值和最大值均为当前 FrameContext.size，使根填满窗口内容区。Component 将约束原样传给 render 返回的描述，并透明转交绘制；它本身不增加 padding 或对齐策略。子项数量和索引使用 Long，位置为有限 Point，允许显式负偏移。越界、遗漏或重复测量、未测量就放置、重复放置按状态／参数错误报告；测量后不放置是合法结果。

测量允许更新由本 Element 拥有的缓存，例如按宽度替换 TextLayout。创建、替换和失败清理必须由同一个所有者完成；不得把待关闭资源存入临时 View。文字 shaping 与断行由 graphics.text 完成，通用段落缓存及行内子项接入见第 7.5 节；Label 选择自己的输入、外观和裁剪策略。

### 7.2 场景装配与纯绘制

Element.scene 接收 `SceneContext`，提供本帧 time、最终 size、输入区域放置和子项呈现。它不暴露 Canvas。`concat／translate／scale／rotate`、`clipRect／clipPath`、`save／saveLayer／restore` 是影响本场景画面与输入的共同操作，交由 graphics 的同一 Canvas 状态维护，不另建坐标计算实现。原点为元素左上角，DPI 仍只由渲染目标根部施加一次。

`SceneContext.draw(block: (PaintContext) -> Unit)` 提交一个纯绘图块。当前实现同步执行，后续绘制缓存可复用其结果；块只负责像素和自身绘制缓存，不登记 InputArea、呈现子项或修改 State。PaintContext 提供 size、time 和隔离借用的 Canvas；块中增加的 transform／clip 仅影响该块的画面，退出后不影响场景中后续区域或子项。场景几何与绘制块的调用分开，省略一个绘图块不会省略已声明的区域。

场景操作在同一个实际帧内流式提交和消费。它们不持有跨帧借用，Path／滤镜在同步调用时按 graphics 合同使用；不要求元素为装配另外复制原生资源。真正的跨帧绘制记录／图像缓存另按其拥有关系持有资源。当前还不承诺自动跳过未失效绘图块。

`SceneContext.children()` 按描述顺序呈现本次已测量、放置的直接子项，跳过未放置子树。有已放置子项时必须调用一次；没有已放置子项时可不调用或调用一次。重复调用报告状态错误。元素可在调用前后提交背景和前景 draw；子项拥有独立场景作用域，不能把变换泄漏到兄弟。父子关系不隐式裁剪，输入父关系由挂载作用域确定。

`SceneContext.children(order: Array<Long>)` 指定这一遍的子项呈现顺序，order 必须是全部直接子项索引的完整排列，包含本次未放置的子项；未放置项仍跳过。长度、重复或越界错误在任何子项呈现前报告参数错误，失败后仍可提供有效排列。两个重载共用一次呈现计数及 withState 的借用规则。此顺序只影响场景／绘图和重叠命中，不改变逻辑 children、挂载身份、事件父关系或 FocusScope 的 Tab 顺序。

`SceneContext.withState<T>(block: (SceneContext) -> T): T` 继承 size、time 及同一子项呈现计数，隔离变换／clip／图层状态；子作用域内拒绝使用父 context。draw 回调期间同样不能使用外层 SceneContext，即使通过闭包取得引用也会报状态错误。正常或异常退出均恢复 Canvas 保存层级，结束子 context，未配对的内部 save／saveLayer 不会污染调用方。

`PaintContext.withState<T>(block: (PaintContext) -> T): T` 只隔离纯绘图块内的 Canvas；没有区域或子项接口。SceneContext、PaintContext、Canvas、MeasureContext 都只在对应同步借用内有效，不能跨帧、跨线程或跨挂起保存。用户场景／绘图异常按第 9 节关闭，失败帧不发布几何。输入的变换、裁剪和 layer 顺序均使用实际 graphics 状态，不以视觉滤镜的像素扩展扩大区域。

`SceneContext.onFrameComplete(action: (scoop.surface.FrameCompletion) -> Unit)` 为当前实际场景登记一次性通知，随所属挂载节点保存到本帧结束。UiRoot 在 Surface 的完成回调中按登记顺序交付，回调中的 time／size／submission 来自该帧。此时 render／measure／scene／draw 都已退出，可以写 State、请求焦点或后续帧；这些变更按既有更新边界处理。回调执行前节点已经卸载，或根开始关闭时，取消该节点的通知；回调中关闭根也取消剩余通知。异常按普通业务回调失败关闭根，不继续交付。未进入 scene 的子树没有通知，不产生为兑现通知而轮询的帧。

需要新几何的动作只在 Accepted 中执行；Discarded 可丢弃本次位置结果，不能将其发布为可交互位置。FrameCompletion.size 是窗口内容区尺寸；元素若需要自身尺寸，应在 scene 中按值捕获。通知不能用于必须执行的资源释放，也不能延长 SceneContext／PaintContext／Canvas 的借用。API 不保存永久帧订阅；持续动画仍由 Surface 时间轴驱动。

### 7.3 输入与焦点接入

Element 在 onMount 中通过 `scope.createArea(focusable, textInput, handler)` 创建稳定的 Surface InputArea，作用域登记其生命周期。处理器使用 RoutedEventContext／InputEvent；传播和默认行为通过 area.scope 的 handler／captureHandler／defaultAction 设置。框架不将原始事件解释为 Button 的 onClick。

ComponentScope.input 提供该挂载的稳定 InputScope，按需创建并沿逻辑挂载祖先连接；根挂载使用 SurfaceWindow.input。createArea 默认连接本挂载的 input。未使用输入的子树不为此预建全部 scope。作用域退休时立即退出路由，资源仍按既有子先父次序关闭。

容器可以用 `scope.routeChildren(area)` 将自己创建、且直接连接自身 input 的区域作为直接子挂载的输入父节点；缺省为自身 input。该选择属于挂载逻辑，与当前布局、clip 和显示层无关。更新此选择时重新连接已经存在的直接子 scope，后续创建的子 scope 使用同一选择。方法先检查区域归属及其直接父关系，因此不能借此把后代节点变成本挂载子项的共同父节点。

UI 管理的 scope.parent 由挂载和 routeChildren 维护，调用方通过这两个入口改变关系，不在框架外手工重接这些 scope。

SceneContext.place(area, rect或path) 只登记当前几何；children() 只呈现子项，不改变路由。框架不为每个布局节点自动创建 InputArea。显式区域须来自同一窗口，同一区域每帧至多放置一次，绘制顺序可以与逻辑父子顺序不同。

update 在逻辑阶段修改处理器和 enabled 等属性，不等 scene 才应用禁用。保留区域使用最新处理器与最近提交的几何；新区域等待首次成功放置。指针序列、取消、冒泡、局部坐标、hover、逻辑焦点及 IME 路由沿用 Surface，点击／拖动等控件行为由上层实现。

由于行内截断等布局原因不再放置的区域，在该帧成功提交后按 Surface 的区域缺席规则退出命中、取消指针序列并清除焦点；组件本身仍挂载。失败／中止帧保留此前已提交的几何，不能提前使用未提交的可见性。恢复放置不重新创建组件，也不自动恢复已经清除的逻辑焦点。布局可见性不替代逻辑 enabled 或卸载。

移除子树时，框架先使其退出业务交互并注销登记区域，完成必要的取消／离开／失焦收尾，再释放处理器和组件资源。布局尚未提交时不能为新区域强行成功请求焦点。完整文本编辑控件的文档、selection、marked range、剪贴板和撤销行为不进入框架。

#### 7.3.1 FocusHandle、FocusScope 与 Tab

FocusHandle 是 owner 线程上的稳定目标引用，不保存第二份焦点状态。isAttached 表示当前仍登记到活动挂载且区域未关闭；hasFocus／isFocusActive 查询所属 Surface 区域；isPending 表示等待提交的当前请求。requestFocus 返回是否接受请求：可见目标立即聚焦，已登记但等待首次／恢复放置的目标请求一帧并等待成功提交；未登记、已退休或逻辑禁用的目标返回 false。clearFocus 取消该 handle 的待处理请求，并在它是当前焦点时清除焦点。提前关闭 InputArea 使其旧句柄失效；随后可将同一句柄登记到新的活动目标，旧登记的晚到清理不能解除新绑定。

ComponentScope.focusTarget(area, options: FocusOptions = FocusOptions()) 将本挂载直接拥有的可聚焦区域登记为焦点目标，返回随挂载退休的 FocusTarget。同一区域只登记一次，一个 FocusHandle 同时只属于一个活动目标。FocusOptions 包含可选外部 handle、tabStop=true、order=0；没有外部 handle 时目标拥有自己的稳定 handle。FocusTarget.options 在逻辑阶段整体更新，外部 handle 改变时解除旧引用并取消旧请求，区域及其已有焦点不被重建。区域及逻辑祖先的 enabled 是唯一启用状态来源。

每个 ComponentScope 可登记一个 focusScope(options: FocusScopeOptions)，返回 FocusScope。options 包含 loop=false、order=0、defaultFocus 与 fallback。scope 内目标按 order 升序、同值按当前逻辑挂载顺序排列；嵌套 FocusScope 是父序列中按自身 order 排列的一个连续分组，组内独立排序。keyed 子项重排后使用新逻辑顺序，不使用原注册时间或绘制顺序。负 order 只改变排序，tabStop=false 才退出 Tab，仍允许程序化请求。

Tab／Shift+Tab 在内层默认行为未消费后导航。默认跳过禁用、未放置和完全不可见的目标；普通 scope 到边界后让父继续，loop scope 在组内循环，窗口根始终在本窗口循环。没有已登记的当前焦点时，从首／末个可用停靠点开始。真实按下和重复按下可导航，合成按键或带 Command／Control／Alt／Function 的组合不作为普通 Tab。键盘导航不激活 OS 窗口，也不修改控件选择状态。

FocusScope.requestFocus 按有效 defaultFocus、首个可用 Tab 目标、有效 fallback 依次选择；fallback 可是 tabStop=false 的区域代理。子项尚未挂载／放置时可等待下一成功帧后选择。域外或失效的引用不作为目标，无可用目标时请求结束。

UiRoot 在 Surface 的 Accepted 完成边界先处理层的移除、模态恢复与首次聚焦，再兑现当前待处理焦点，最后执行用户登记的 SceneContext.onFrameComplete 动作。请求记录 Surface.focusRevision；UI 接受的待处理请求、已知窗口内被域拒绝的显式请求通过 reserveFocusRequest 推进同一个序号，立即聚焦仍由 InputArea 推进一次。期间另一次显式焦点意图、目标退休、外部 handle 更换或根关闭都会使旧请求失效。未绑定的 handle 没有所属窗口，不能改变其他窗口的请求。Discarded／Deferred 不兑现或轮询；Accepted 后仍不可用的普通目标结束请求。滚动显露与虚拟目标接入由后续滚动批次补充。焦点请求和登记配置变更禁止发生在 render／measure／scene／draw 中。

#### 7.3.2 KeyNavigation 与组内导航

ComponentScope.keyNavigation(options: NavigationOptions) 登记本挂载拥有的无样式导航组，返回随挂载退休的 KeyNavigation；每个挂载至多一个活动组。NavigationOptions 包含不可变 items: Array<NavigationItem>、layout: NavigationLayout = Vertical、direction: graphics.text.TextDirection = Ltr、loop=false、roving=true。NavigationItem 包含稳定 FocusHandle 及 row=0、column=0；Horizontal／Vertical 使用数组顺序，Grid 使用显式行列位置。

options 在逻辑阶段整体更新；重复 handle、同一个 handle 属于两个活动导航组、Grid 重复行列坐标均报告配置错误，并保留旧配置。允许先配置尚未挂载的 handle，但它随后只能绑定到该导航组的逻辑子树。关闭组解除成员关系；替换目标的外部 handle 后，旧 handle 不会继续控制新引用。enabled 和可聚焦性仍来自原区域与逻辑祖先，没有另一份成员禁用状态。

roving=true 时仅一个有效成员参与 Tab：优先当前聚焦且符合 Tab 条件的成员，然后选择最近成功聚焦的有效成员，未有历史则选数组中首个。历史使用 Surface 的聚焦请求记录，包含鼠标及直接 InputArea 请求；当前持有焦点不等于必须有 OS 活动焦点。显式 tabStop=false 始终退出 Tab，但仍可通过箭头聚焦。roving=false 恢复成员各自的普通 Tab 条件，组内导航仍有效。未挂载、禁用或不可见的成员不作为新的导航目标。

组的默认行为仅在当前 focusedArea 就是成员区域时处理方向键和 Home／End，不替代成员内部另一个编辑目标的按键行为。Horizontal 左右按 direction 映射，Vertical 上下不因 RTL 反转；列表 Home／End 到首末有效成员。Grid 的左右只在同一行、上下只在同一列按逻辑坐标查找，越过缺失／禁用项；Home／End 到当前行的逻辑首末。loop 只在当前导航轴循环，不依据像素距离寻找邻居。

仅真实、无修饰键的 key-down（包括 repeat）执行；key-up、synthetic、Command／Control／Alt／Shift／Function 组合不执行。内层默认行为先取得按键；组认领的方向／Home／End 即使到边界也消费，防止祖先误作另一组的移动。组不处理不属于其方向的箭头，也不处理 Enter／Space 或改变选择状态。导航与 Tab 共用 FocusHandle 请求，仍服从成功帧和当前合法域；虚拟成员的 reveal 在滚动批次扩展。

FocusTarget 与 FocusScope 的待聚焦准入使用 InputScope.isInputAllowed：位于当前 InputBoundary 之外的请求直接失败，不能在模态关闭后意外兑现。逻辑 enabled 状态查询与当前输入域资格分开；已有 pending 仍受新请求、目标退休和成功帧检查约束。

### 7.4 Surface 动画接入

`scope.animation(initial)` 提供与 Surface 相同的 Double／Point／Size／Color 重载，也提供带 AnimationConverter 的泛型重载。它调用所属 SurfaceWindow 的 animations.value，并登记所得 `Animatable<T>` 随该挂载卸载；返回值就是 Surface Animatable，不包装另一套动画状态或时钟。

元素在 update 或输入处理中调用 animateTo／snapTo／pause 等现有操作，在 measure／scene／draw 中读取已采样的 value。Surface 的统一时间轴负责请求帧、同帧采样、完成回调及隐藏后的时间推进；UI 不另建 timer 或每组件帧循环。框架保证本次测量和绘制使用同一采样时点。需要动画结束后改变组件结构时，由完成回调写 State，排入下一轮逻辑更新。

退出挂载时退休登记动画，停止其后续业务完成回调，并在资源清理阶段关闭。完成回调的挂载有效性遵守与输入相同的规则。Element 普通字段的变化若未经过 State 或 Animatable，应调用 scope.requestFrame；同一轮请求由 Surface 合并。

### 7.5 公共文本接入

`scoop.ui.text` 是本 Cone 的公共子包，依赖 [scoop.graphics.text](SCOOP-GRAPHICS-TEXT-SPEC.md)，提供 InlineView、inlineChildren 和 ParagraphHost。它是自绘 Element 的复用能力，不是控件库或新的 View 分支；第三方控件与 Label 使用相同入口。

InlineView 是不拥有资源的描述，保存 InlineKey、View、自然或固定 Size 的测量策略，以及对齐选择。默认采用子项 alphabetic baseline；调用方可选择 ideographic baseline，所选基线缺失时使用盒子底边，也可显式给出文本 InlineAlignment。固定尺寸不自动按段落宽度缩小。图片可由普通自绘 Element 包装成 View，沿同一协议接入。

公开构造器为 `InlineView(key, view, size: Size? = None, alignment: InlineViewAlignment = Baseline(TextBaseline.Alphabetic))`。None 尺寸表示自然测量；`InlineViewAlignment.Baseline(kind)` 使用子项相应基线，`Explicit(value)` 直接采用 graphics.text 的 InlineAlignment。两种形式都由文本库参与同一次排版，不在 UI 层另算文字行高。

`inlineChildren(content, inlines): Array<View>` 按 StyledText 中对象的逻辑顺序生成普通 keyed 子描述。它根据 InlineKey 建立对应，不根据文字偏移、视觉顺序或数组下标推断身份；缺失、重复或多余绑定报告参数错误。每个对象用普通 keyed 包装节点保存其出现身份，传入 View 自己的 key 继续作用于包装节点内的子位置。包装只使用普通 Component／Props；Span 不生成挂载节点，同段落的全部对象共享该 Element 的直接父级。

ParagraphHost 是普通 Closeable 辅助对象，由使用它的 Element 在 onMount 中通过 scope.resource 登记。它不拥有子组件，也不自行调用工厂或创建输入区域。创建 Element 描述的 Component 或普通组合函数把 inlineChildren 的结果传入 element.children，并通过 Props 传入同一份内容和绑定；Element 的 measure／scene 分别委托给宿主：

```text
ParagraphHost.measure(context, constraints, fonts, content, settings, inlines): LayoutMetrics
ParagraphHost.scene(context): Unit
```

measure 在零最小值、段落最大宽度及无界高度下测量每个自然尺寸子项；固定尺寸子项使用相应 tight 约束。由 LayoutMetrics 和对齐选择生成完整 InlineMetrics，调用 graphics.text 排版，再按 key 对应的可见 box 放置子项。宽度无界时用 layoutUnwrapped；只有尺寸、基线及约束参与排版，子项不能要求先取得最终所在行的剩余宽度。所有子项仍只测量一次，包括本次最终被截掉的对象。

宿主返回受 Constraints 约束的段落尺寸及相对首行基线，文字原点为元素左上角。scene 通过 draw 使用同一 TextLayout 先画文字，再调用 children 呈现已放置对象；输入父关系取自逻辑挂载作用域，不由场景参数指定。没有可见对象时仍可正常绘制文字。宿主不隐式裁剪外框，调用方需要裁剪时在同一个 SceneContext 作用域内包住文字、子项和输入放置。对象在父段落中不可拆分，其自身超出测量框的绘制遵守普通 Element 的显式 clip 规则。

内容或字体／样式变化重建 Paragraph，宽度和对象 metrics 改变时创建新 TextLayout；实际输入不变时复用缓存。子控件颜色等不改变 metrics 的更新不要求重排；尺寸／基线动画在 Surface 同帧采样后进入测量。关闭或替换文本缓存不卸载行内子项；只有正常的描述匹配、移除和宿主关闭改变挂载。

缓存以不可变 StyledText 的对象身份、FontCollection 的身份及 revision、ParagraphStyle 的字段值识别段落输入；新的 StyledText 描述可以触发重建，不要求 UI 重新解析或比较文本层私有的规范化结果。相同段落的宽度和按逻辑 key 排列的 InlineMetrics 按值比较。只改变约束下限或父级放置不重排，仍按本次 Constraints 返回外框。调用方可复用内容描述；String 便利控件负责在自身输入不变时保留该描述。

`ParagraphHost.layout` 借出最近一次成功测量的 TextLayout，以供自绘控件命中、caret、selection 或位置查询；首次测量前或关闭后访问报告状态错误。调用方不关闭该借用；下次替换或宿主关闭后旧布局资源失效。新 Paragraph／TextLayout 成功取得后才替换缓存；失败时释放新取得的资源并保留原缓存，随后仍遵守框架的回调失败与关闭规则。ParagraphHost 只能在创建它的 owner 线程使用，close 幂等；不关闭传入的字体来源。

所有内容查询、metrics 和 placement 通过文本库的 public API 取得。公共适配不读取 graphics.text 的 internal 缓存，不把无匹配 box 的对象悄悄移除出 children，也不需要基础控件库来实现。

### 7.6 同窗口 portal 与锚点

UiRoot 在普通内容的测量和 scene 完成后处理同窗口层出口。Portal 在原容器中按 empty 的约束下限返回占位尺寸，不测量其内容，也不以弹层尺寸撑开父容器；内容在逻辑阶段正常挂载，获得有效层位置后才在本帧测量一次，并从窗口根的坐标与 clip 状态呈现。普通父容器不重复测量层内内容。未参与本帧正常场景的 Portal 不呈现；层内容的逻辑更新和资源归属仍按普通挂载处理。

LayoutAnchor 是 owner 线程上的稳定引用，通过 ComponentScope.layoutAnchor(handle) 登记到一个挂载，返回随挂载退休的 Closeable。一个 handle 同时只绑定一个活动挂载；重新绑定后旧登记的迟到清理不解除新绑定。SceneContext.anchor(handle, rect) 只允许登记本元素拥有的 anchor，每帧至多一次。它使用 graphics 的同一变换／clip 快照得到窗口包围盒和可见性，不创建 InputArea；旋转使用轴对齐包围盒。LayoutAnchor.isAttached 查询逻辑绑定，不表示已有成功帧或可见几何。

PortalOptions 保存 placement、可选外部 OverlayHandle、anchorVisibility（默认 Close）、可选 modal: ModalOptions、dismiss: OverlayDismiss 和当前 onClose 回调。缺省句柄由挂载会话内部拥有；描述重建不会隐式分配新的活动会话。参数在描述构造／更新边界检查，归属在建立实际绑定时检查；错误保留旧的有效绑定。普通／模态是不同的挂载形状，改变 modal 的有无会退休旧会话并重新挂载；同一模态形状内的 initialFocus、restoreFocus 和关闭策略可正常更新。

OverlayPlacement 有两种形状：Viewport(horizontal=Center, vertical=Center, viewportPadding=8)，在窗口可用矩形中对齐；Anchored(anchor, preferredSide=Bottom, alignment=Start, gap=4, viewportPadding=8, matchAnchorWidth=false)，相对锚点定位。OverlayAlignment 包含 Start／Center／End，OverlaySide 包含 Top／Right／Bottom／Left；对齐使用物理坐标，不隐式读取控件 writing direction。gap／viewportPadding 为有限非负值。

锚点定位先扣除 viewportPadding，求优先侧和反侧的可用范围，用较大主轴范围与视口交叉轴范围给内容一次有界测量。matchAnchorWidth 提供受本次最大宽度限制的 tight 宽度。实际尺寸放得下优先侧时保留该侧，否则选择反侧，二者均不足时选较大侧；交叉轴 shift 到视口内。KeepVisible 对有效但已离开视口的锚点进一步将层位置限制在可用视口内。不存在通过重复测量实现的 flip 协商，也不轮询静止锚点。

SceneContext.overlayLayout 返回当前最近 Portal 的本帧 OverlayLayout，普通内容为 None。OverlayLayout 只包含只读的 bounds、available、可选 anchor 矩形和实际 side；Viewport 的后两项为 None。弹层可用该结果绘制箭头，跟随本帧最终位置，不为取得 side 再触发布局。数据不含 Canvas 借用，输入位置仍只以 Surface 成功提交的几何为准。

层先按所属模态域、再按域内打开顺序排列。普通 Portal 属于最近的逻辑模态祖先，背景域始终先呈现；模态域按打开顺序叠放，逻辑上的子 Portal 在其父层之后完成测量和呈现。后来打开的背景 Tooltip 不能覆盖已打开的模态。普通内容的 anchor 先完成登记，再定位相应层；层内 anchor 可供后续层使用。anchor 必须属于同一 UiRoot，其位置来源不能依赖当前层本身。建立或更新 Portal／anchor 绑定时沿实际依赖关系检查自引用和环，不在每帧重新审计整棵树；尚未绑定的 anchor 在随后绑定时完成同一局部检查。层顺序与已建立依赖不一致、当前没有可用位置时，本帧不使用旧坐标强行呈现。

AnchorVisibility 包含 Close／Hide／KeepVisible。锚点有当前几何但完全不可见时，Close 在成功帧完成后关闭层，Hide 保留内容挂载但不呈现，KeepVisible 使用当前有效包围盒继续定位。没有本帧几何时 Hide 等待恢复，其余策略关闭；锚点解绑／所属挂载退休时所有依赖层关闭，不能继续使用旧绑定。因场景可见性产生的关闭只在 Accepted 后兑现；Discarded 不以未提交结果关闭层。回调均在 render／measure／scene／draw 退出后运行。

OverlayHandle 是会话的稳定引用，提供 isAttached、isOpen、lastCloseReason 和 close()；close 接受显式关闭并停止该会话的交互，子内容在普通 UI 更新边界退休。关闭是幂等的，晚到清理不解除同一句柄的新绑定；同一个外部句柄不能绑定到两个活动挂载。OverlayCloseReason 包含 Explicit、Escape、OutsidePress、AnchorUnavailable、OwnerClosed；每个会话最多通知一次当前 onClose。关闭／配置／绑定变更禁止发生在纯渲染阶段，场景中的 anchor 登记属于正常几何装配。

#### 7.6.1 关闭策略

OverlayDismiss 包含 escape=false 和 outsidePress=OutsidePress.Ignore；OutsidePress 另有 CloseAndConsume、CloseAndContinue。关闭策略在当前窗口／模态域根接入已有默认阶段及 beforePointerPress，不安装第二条路由。Escape 只认领无修饰的真实非重复 key-down，内层默认行为优先；一次只关闭该域最上层允许 Escape 的会话。忽略 repeat，避免按住 Escape 连续关闭多层。

外部按下按 InputScope 的逻辑归属判断，子 Portal 的目标属于父层内部，不按父层矩形判断子菜单是否越界。没有命中的模态域根视作内容外部；控件应为需要接受内部空白点击的面板登记普通区域，遮罩外观本身不创建内部命中。由最上层适用策略处理一次，CloseAndConsume 消费整串按钮，CloseAndContinue 允许已经冻结的原目标继续，不能重新命中或重投。关闭回调产生新模态时仍受新的输入域约束。

模态外部按下始终被输入屏障挡住；可选 Ignore 或 CloseAndConsume，CloseAndContinue 是无效配置。普通 Portal 默认不隔离背景输入；Dialog、AlertDialog、Menu 等组合控件自行选择具体关闭策略。

#### 7.6.2 同窗口模态会话

ModalOptions 包含 initialFocus: FocusHandle? = None、restoreFocus=true。模态 Portal 的一次挂载拥有 InputBoundary、循环 FocusScope 和不可命中的可聚焦根区域；根区域在层呈现时使用窗口视口几何，tabStop=false。输入资格、焦点历史、取消与 IME 收尾均复用 Surface；遮罩、居中或侧边布局仍由内容决定。

挂载时先保存当前有效 FocusHandle 和上一输入域，再安装屏障，之后挂载子内容。首次实际呈现的 Accepted 帧按有效 initialFocus、首个 Tab stop、根区域代理依次选择；期间有更新的显式焦点请求则放弃自动选择。被遮住的外层不抢内层焦点，程序化聚焦不激活 OS 窗口。首帧 Discarded／Deferred 不兑现焦点，也不释放屏障；Hide 或尚未参与场景的模态保持待显示状态，调用方可显式关闭。

关闭立即禁用该模态根及子内容，保存关闭时的焦点意图序号；屏障持续到成功提交了不含该层的帧。节点已卸载时仍保留这条待释放登记；在显示该层的完成回调中关闭，需要等待下一次移除提交。Discarded 不移除登记，窗口／根关闭立即释放全部登记，不等待帧。

成功移除时从内到外释放符合条件的登记，只对原来占据当前域的关闭会话尝试一次恢复。优先仍可聚焦且属于恢复域的原 handle，否则使用恢复域的 FocusScope 或窗口有效目标。上一 UI 模态已关闭时沿保存的会话关系越过它；非当前域先关闭不移动焦点，也不使晚关闭的内层恢复到已退休目标。恢复域与保存的前驱不一致、restoreFocus=false，或关闭期间出现新的显式焦点意图时不执行恢复。滚动目标的 reveal 在滚动批次接入；本批先使用已可见目标和既有待聚焦协议。

同窗口模态不隔离应用的另一个原生窗口。系统 sheet／独立原生窗口模态仍依赖 Window 的 owner／modality 能力。

### 7.7 滚动状态与视口登记

`ScrollState(initialOffset: Point = Point(0, 0))` 是独立于挂载的 owner 线程状态。snapshot 返回 `ScrollSnapshot(offset: Point, viewport: Size, content: Size)`，maximum 为每轴 max(0, content - viewport)；offset／viewport／content／maximum 是对应便利属性，isAttached 和 isAnimating 查询当前绑定。render 读取这些属性建立普通 State 依赖，仅结果改变时通知。滚动容器不必在 render 读取 offset，偏移可直接请求场景帧而不重建内容。

`scrollTo(Point)`、`scrollBy(Point)` 和 `reveal(Rect, horizontal = Nearest, vertical = Nearest)` 更新位置意图。ScrollAlignment 支持 Nearest／Start／Center／End，Rect 使用内容局部坐标；Nearest 对已完整可见或已包含视口的超大目标保持原位置。范围未建立前 offset／extent 为零，构造位置、绝对定位和 reveal 延迟到首个成功布局；此时 scrollBy 在已有位置意图上累加。范围改变时位置收敛；State 脱离视口后保留最后结果供再次挂载。

`ComponentScope.scrollViewport(state)` 返回挂载拥有的 ScrollViewport。同一 State 至多有一个活动绑定，可整体更新登记的 state；先检查新绑定，失败保持旧状态。登记公开 layout(viewport, content) 与 present(SceneContext)：自绘容器在 measure 取得本帧快照，随后在自己的 scene 登记提交。layout 不发布可观察状态，Accepted 才提交范围和偏移，Discarded 保留旧结果。present 只接受本挂载的 SceneContext，每次 layout 结果只提交一次；关闭和绑定替换取消旧帧回调。布局后的新位置意图优先，按新范围收敛并请求必要的下一帧，旧位置不得覆盖它。

`animateTo(Point, AnimationSpec)` 显式平滑滚动，缺省为 300 ms Tween；必须已有绑定，可在首个布局前排队。登记拥有 Surface 同一时间轴的 Animatable<Point>，布局采样、Accepted 发布；isAnimating 包括等待首布局的动画。终点按最新范围收敛，无剩余移动时撤销动画需求。cancelAnimation、直接定位、reveal 与用户滚动在最后发布的 offset 停止旧动画，不在消费一份 wheel 残量时另外推进时间轴。关闭登记立即停止动画并解绑；State 自身不拥有窗口或待关闭资源。

布局容器可向 `ScrollViewport.layout(viewport, content, adjustment)` 提供 `ScrollAdjustment(requestRevision, offset)`，用于可变尺寸内容的阅读锚点或按 key 定位。ScrollState.requestRevision 是仅供比较的当前定位意图序号，读取不建立 State 订阅；scrollTo／scrollBy／reveal、动画定位及取消活动动画推进序号，范围发布不推进。adjustment 只在序号仍匹配且没有活动平滑滚动时应用，按本帧范围收敛；布局仍不写 State，Accepted 后才发布。调整准备期间或布局之后的新请求优先，Discarded 不发布旧修正。`ScrollSnapshot.revealedOffset(rect, horizontal, vertical)` 以相同规则纯计算显露位置，供布局复用对齐和收敛算法，不改变 ScrollState。

上述公开写操作遵守 State 的纯度约束，render／measure／scene／draw 中拒绝调用。几何和位移须有限，尺寸须非负，派生坐标溢出按普通参数错误报告。ScrollAxes 的 Horizontal／Vertical／Both 提供公开 horizontal／vertical 查询，供容器选择约束和交互轴；具体 ScrollArea 布局、滚动条与默认 lineStep 属于独立控件库。按需子项见第 7.8 节，自动焦点显露及绘制／实例缓存按后续滚动批次扩展。

### 7.8 按需子项与 LazyElement

`ItemSource` 提供一个数据快照的 `count: Long`、`keyAt(index): ViewKey`、`indexOfKey(key): Long?` 和 `build(index): View`。count 非负，key 不得为 Position，在同一 source 内唯一；查找函数只读取快照，访问时检查索引与双向映射，不扫描尚未请求的条目。改变数据、索引或条目输入时替换 source 对象；不把可变 source 的原地修改当成通知，也不读取全部内容作摘要。build 只在实际需要的条目节点中执行，遵循 Component.render 的纯度和 State 依赖规则。

`LazyElement` 与 Element 并列，公开 measure(LazyMeasureContext, Constraints)、scene(SceneContext)、onMount(ComponentScope)、update() 和 onUnmount()。`lazyElement(source, create, retention = ItemRetention.Dispose, key)` 及带 props 的重载创建描述，保留具体 class／Props 类型和普通 ViewKey 身份；create 不提前调用。它没有 eager children 数组。普通 Element／MeasureContext 的全部直接子项测量规则保持不变。

LazyMeasureContext 公开 time、itemCount、`measureItem(index, constraints)`、`placeItem(index, origin)` 和 `pinnedItems: Array<Long>`。每个本轮选中的索引至多测量一次，测量后可以放置零次或一次；context 只在当前同步测量中有效，子项准备／测量期间拒绝重入同一个 context。scene 的 children() 按当前已实现项的 source 索引顺序呈现已放置项；children(order) 的索引是这个已实现子序列的位置，不是 source 索引。

measureItem 在框架控制的准备边界按稳定 key 取得条目挂载，再运行其 build／render、onMount 和 update，随后恢复普通纯测量。生命周期可以沿既有规则登记资源和写 State，其产生的其他失效留到下一逻辑更新轮，不递归 flush；build／render 和父子 measure 仍禁止 State 写入。每项使用普通挂载子树保存依赖、Props 与资源，滚动复用已实现项时不重跑其 build。真实 source 替换在逻辑更新阶段通过 indexOfKey 更新已实现项、退休删除项，再更新存活项；即使没有 drawable 也完成删除和输入取消。source.build 返回描述的 shape／key 改变时按普通子树规则替换。

缺省 Dispose 下，离开本次选择范围且无活动交互的条目正常卸载。pinnedItems 包含已有逻辑焦点、待处理的 UI 焦点请求、按钮序列、保留滚动路由、文本输入会话或输入边界所属的条目；容器必须测量并放置这些项，可用同一视口 clip 裁掉其像素。此查询复用 Surface 的实际路由归属，不另存输入状态。滚动路由包含接触 Ended 后等待可能的 momentum 的候选；每窗口至多一个，惯性结束／取消、下一段滚动或原路径失效后释放，不因此请求连续帧。真实删除和 owner 关闭优先于固定。

框架在活动归属改变后请求必要的新布局，使失去固定资格的离屏条目恢复淘汰；同一大子树内焦点从条目 A 移到 B 也算归属改变。条目准备期间显式关闭根会终止当前 UI 布局，仍沿原安全关闭路径释放已经取得的资源，不以未完成的子测量继续装配场景。失败仍按第 9 节关闭，不保留半挂载的输入树。可变 extent 与按 key 定位由控件库通过本协议和 ScrollAdjustment 实现；虚拟焦点显露与绘制缓存按后续批次扩展。

ItemRetention 另提供 KeepAlive(maxOffscreenItems: Long)，上限非负，零等同 Dispose；它只计算未选中且无活动交互的离屏条目。超出时淘汰最久未测量的条目，同一帧内的先后不作承诺。保留同 key 的普通挂载子树，不跨 key 复用实例；形状变化仍替换实际子内容。策略收紧、数据删除及 Props／State 更新在正常逻辑阶段处理，即使没有 drawable 也不延期清理。

缓存条目继续 mounted、订阅和执行逻辑更新，本帧未选择时不测量或呈现场景。框架对 lazy 容器明确未放置的子树关闭继承的帧资格，不按布局 bounds 推测阴影、文字 overhang 或 Portal 的可见性。局部 State 更新和 scope.requestFrame 不单独唤醒绘制；影响已参与节点的更新仍请求帧。每帧沿 Surface 时间轴采样当前动画，条目测量完成后按实际放置结果恢复资格；只测量而未放置不会短暂重新启用帧。嵌套 lazy 子树保留自身未参与状态。每个作用域通过 Surface AnimationGroup 管理这个开关，保留单个动画的显式 isFrameEnabled 设置；有限离屏动画仍能完成，无限重复不产生离屏周期唤醒。输入边界变化等窗口级交互工作继续使用其必要的全局请求。

待聚焦请求完成、取消或被新请求替换时，即使没有产生实际的 Surface 焦点变化，也撤销其固定需求并请求必要的后续帧；不可见目标的失败请求不能使离屏条目一直留在实现集合中。根关闭时直接清理，不再请求帧。

## 8. 挂载与资源生命周期

`ComponentScope` 是框架为每个 Component／Element／LazyElement 分别创建和关闭的引用对象，提供：

- `resource<R : Closeable>(create: () -> R): R`：成功取得后立即登记为本组件拥有的资源。
- `onDispose(action: () -> Unit)`：登记卸载时同步执行的清理动作，例如解除外部订阅或发出任务取消。
- `isMounted`：当前挂载是否仍可接收业务工作。
- `requestFrame()`：请求所属根再次呈现；不立即执行 render、measure 或 paint。
- `createArea(...)`：创建并登记第 7 节的稳定 InputArea。
- `animation(...)`：在所属 Surface 时间轴创建并登记第 7 节的 Animatable。
- `scrollViewport(state)`：登记第 7.7 节的视口范围、位置提交与平滑滚动。

首轮逻辑提交建立节点和订阅后，按父先子后的顺序调用 onMount，后续 render 不再调用。Element 的首次 update 在其 onMount 成功后执行；全轮挂载完成才可能进入测量／绘制，因此元素的资源在首帧可用。onMount 或 update 中的 State 写入排到下一轮，不递归重建。外部订阅和初始资源在此建立；不提供每次 render 自动重跑的 effect API。

卸载先把整棵子树标为不活动并解除依赖／排队工作，然后按子先父后的顺序处理各节点：退休登记输入和动画，调用已成功完成 onMount 的实例的 onUnmount，最后逆登记顺序执行 ComponentScope 清理及元素资源关闭。onUnmount 执行时 scope 已不再允许新增资源或请求帧；除已退休的输入外，其余待清理资源在各自释放前仍属于该实例。没有自定义 onMount 的实例同样完成默认挂载并获得 onUnmount。

原生窗口或 Surface 先进入 Closing 时，仍会向尚未卸载的控件交付失焦／取消通知。UI 在这些通知中继续允许正常的控件状态收尾，但不把新的帧需求送给已经停止呈现的 Surface；根随后按同一关闭路径退休。已经卸载的 ComponentScope 仍拒绝 requestFrame，不改变 SurfaceWindow 自身的关闭后调用规则。

构造器或首轮 render 失败的实例没有完成挂载，不调用 onUnmount；已经建立的节点资源仍须释放。onMount 自身失败时，其 scope 中已登记的资源同样全部清理，不要求失败的回调再自行释放它们。Element 的 update、measure 或 paint 失败时，已完成挂载的实例按通常卸载规则收尾。所有清理分支继续处理剩余资源，错误合并遵守现有 ResourceUseException。

框架管理自己创建的挂载结构及登记资源。控件库和第三方 Element 负责声明自己拥有的 Path、字体或缓存，并通过相同的 ComponentScope 登记。公共 ParagraphHost 管理自己的 Paragraph／TextLayout；其他可变绘图缓存同样可以实现普通 Closeable，在 onMount 时登记一次，由缓存对象替换和关闭旧值，不得每帧追加旧资源直至卸载才释放。测量中创建新缓存值失败时保留或正确关闭原值，成功替换后及时关闭旧值。缓存不拥有行内子组件，截断可见性不触发 scope 清理。

传入的 Image、Path、FontCollection 或外部模型默认借用；需要跨调用保留 native 使用时遵循 graphics／graphics.text 的持有合同，不能擅自关闭调用方拥有的包装对象。作用域不拥有 Surface 的整条时间轴或窗口，只拥有为本挂载登记的对象。关闭动画和区域会从 Surface 的集合中移除，Surface 最终关闭继续作为窗口级收尾。

状态对象与托管引用最终由 GC 回收；卸载、取消输入、关闭 native 资源不能等待 GC。外部继续持有一个卸载后的模型不会使其重新渲染；外部继续持有组件也不延长它的挂载期。

### 8.1 异步任务归属

异步任务属于现有 CoroutineScope／Job，从事件处理或 onMount 启动，不从 render 启动。随组件结束的任务把 Job.cancel 登记到 ComponentScope.onDispose；跨页面或窗口继续的业务任务由应用／文档作用域持有，单个观察组件卸载只解除 State 订阅。State 或 Context 中保存任务引用不改变其结构化归属，也不转移共享 HttpClient 等资源的关闭责任。

ComponentScope 同步发出取消，异步 finally 和实际 I/O 收尾由既有 event loop 完成；宿主按窗口／asyncio 合同持续驱动到收尾结束，不在 UI 线程阻塞等待 Job。Canvas、布局／绘制借用和组件拥有的绘图资源不能跨挂起继续借用。迟到结果在 owner 上确认目标仍有效，或只写仍由业务层持有且已无 UI 订阅的状态。

预期网络失败在业务任务正文内转换为错误状态，取消保留 CancellationException 的传播；未捕获失败继续沿既有任务树与宿主异常路径处理，不增加 UI 错误边界或自动重试。需要显式识别连续任务时，由业务对象保存当前任务身份；旧任务的迟到进度／结果不能覆盖新任务。普通托管状态、任务取消与原生资源释放各自遵守已有合同。

## 9. 宿主、关闭与错误

`UiHost` 包装 SurfaceHost，作为普通 ApplicationHandler 接入 Application.run，也可以接入现有 runWithAsyncio。`UiHost.attach(window, create)` 返回管理根组件的 UiWindow。`runUiApplication(title, create)` 是单窗口便利入口，统一拥有 Application／UiHost／窗口，接受关闭请求并在最后一个窗口关闭后退出；复杂窗口策略使用显式 UiHost。

macOS 的 UiHost.shortcutScope(bindings: Array<surface.ShortcutBinding> = []) 返回其 SurfaceHost 拥有的 HostShortcutScope；采用同一输入路由和生命周期，不复制绑定或匹配算法。每个 UiHost 至多一个活动宿主登记，关闭 UiHost 时停用；局部行为与窗口 ShortcutScope 优先，任何模态输入域都会截断宿主回退。在 Dialog 内有效的保存／确认动作应登记在该域的 InputScope。

底层 scoop.window 已支持同一 Application 的多个独立非模态窗口，见[窗口规范第 4.1 节](SCOOP-WINDOW-SPEC.md#41-多窗口)。一个 UiHost 可对不同 Window 分别 attach，每个窗口拥有独立根、挂载树、布局与区域场景；单窗口便利入口不限制框架只能管理一个窗口。普通关闭只卸载相应窗口的根，显式应用退出或宿主失败继续沿既有整体收尾；同一 owner 上可共享业务 State，不能共享组件实例、InputArea 或跨窗口转移挂载。

原生模态对话框与完整自绘窗框依赖[窗口规范第 4.3 节](SCOOP-WINDOW-SPEC.md#43-模态与自绘窗框的待补能力)的待补接口。窗口归属、窗口模态／应用模态、输入限制、关闭与焦点恢复、系统拖移／缩放属于 window；UI 负责内容、控件和结果交付。当前设计不把置顶或临时屏蔽组件事件当成原生模态，也不因装饰开关已实现就声称已具备完整自绘窗框操作。

资源管理在最外层保留明确的 use 作用域。显式创建 UiHost 的调用方以 `host.use { ... }` 包住事件循环；`runUiApplication` 在内部建立并关闭相同的宿主作用域。所有权由宿主经 SurfaceWindow 的 content 传到 UI 根及各挂载节点，普通组件不再逐个 use。单个窗口关闭时立即收尾它的内容，不等整个应用离开最外层 use；最后的宿主 close 幂等地释放仍然存在的窗口。

UiWindow.close 停止根组件的新工作并请求关闭所附着的 SurfaceWindow，不关闭调用方传入的原生 Window；窗口是否继续存在由调用方决定。根组件的实际资源释放遵循下述 content 的安全关闭时机，不在活动绘制／输入回调中抢先销毁其资源。原生 Closing、Application Exiting、SurfaceHost 失败以及显式 close 都必须走同一幂等资源收尾，不能依赖应用转发某个清理通知。

surface 已提供真实的内容拥有关系：SurfaceContent 具有 paint(FrameContext) 和 Closeable 合同，SurfaceHost.attach 可以接收它。现有 painter 入口继续服务直接绘图，由库适配为无额外持有资源的 content。SurfaceWindow 在当前业务回调和 Canvas 借用结束后、renderer／lease 释放前，关闭其 content 一次。UI 根挂载对象作为 content 被持有，因此异常或原生关闭路径也能释放整个 UI 子树。

SurfaceContent 定义在 scoop.surface，只表达可绘制、需要关闭的内容；它的签名不引用 Component、View 或其他 scoop.ui 类型，不产生低层到 UI 的反向依赖。

SurfaceContent 的 close 必须幂等。通过 owner 线程检查后，attach 接收传入的新 content 的所有权；创建或配置失败也由 surface 关闭 content 并回滚已取得的 renderer／lease，调用方不在返回或抛错后再次自行管理它。错误线程的调用在接收所有权前拒绝。一个 content 不交给多个窗口；UiHost 为每个根创建独立对象。

SurfaceContent 的具体合同见 [Surface 规范第 4／7 节](SCOOP-SURFACE-SPEC.md#4-surface-宿主与帧)，aarch64 实施及原生关闭／异常验证见 [UI 实施记录](../ui/PROGRESS.md#34-surface-内容根的所有权)。UI 根作为其 content 管理完整挂载树，逻辑回调内请求关闭同样延迟到回调返回后释放资源。

参数、重复 key、重复挂载、非法测量／放置或错误线程使用现有参数／状态异常。render、update、measure、paint、输入处理或挂载回调抛错时，停止新 UI 工作，通过宿主关闭其管理的相关根及 surface，按 SurfaceHost 既有异常通路传播。首期不保留一个继续接收输入的半更新树，不增加错误边界或自动重试次数。

### 9.1 异步 I/O、下载进度与跨线程更新

需要异步 I/O 的应用显式组合 UiHost 与 `scoop.window.asyncio` 的 Application.runWithAsyncio，复用附着在窗口 owner 上的 PlatformEventLoop、EventLoopSession 和 currentApplicationScope。普通单窗口便利入口不隐式建立 I/O loop；不在 render 或事件回调内嵌套 EventLoop.run。asyncio、HTTP client 等是使用者／集成示例的实际依赖，不为此加入 UI 核心的公共签名或生产依赖。

默认在该 EventLoop 的活动应用作用域中创建并复用 HttpClient，下载作为子任务执行。同一 client 可由所属 EventLoop 的多个任务并发使用。网络及异步文件等待挂起协程，窗口循环继续处理事件；内核／原生 callback／worker 发布完成，由 asyncio 在 owner 消费并恢复自己的 I/O 等待。此路径的进度回调可直接更新同 owner 的 State；setter 只安排逻辑更新，不同步绘制。

上述恢复保证只覆盖 asyncio 自己的 I/O，任意第三方 suspend 函数仍可能在其他线程恢复。launch 和 Context 本身都不提供线程切换或强制 UI 线程亲和性；计算密集工作也不会因位于 suspend 函数中而自动移出主线程。真实跨线程结果通过 ApplicationProxy.post 或 EventLoopSession.post 交付；需要挂起的后续业务在目标 scope 中启动子任务。

跨线程投递携带稳定的值／只读 class 快照，发送方在交付期间不再修改快照及其引用的数据；整个 State 读改写与通知在 owner 上执行。不在后台先修改 State 再只投递 render，也不让 State setter 隐式变成异步写入。Context 只传递依赖对象；proxy／session 投递执行于接收方的运行 Context，不继承外线程投递时的临时绑定。业务 post 的 false 表示拒绝，true 仍可能在关闭时被丢弃；取消和实际资源清理继续使用下层已有的收尾通路。

下载复用 HttpDownloadOptions.onProgress 及 HttpDownloadProgress 的 bytesWritten、可选 totalBytes 和 resumedBytes。进度快照是普通 class，不要求应用把整个 Store 改成值类型或不可变快照。回调只做轻量状态发布；下载等待该 suspend 回调完成，不能通过等待 UI 绘制来限速回调。总长度未知时保持不定进度，成功完成以 download 返回、目标文件提交及必要清理结束为准，不以字节百分比替代完成结果。

高频普通进度可以在业务交付层按时间合并，只保留最新快照；跨线程实现用同步保护的 latest 值和至多一个待执行投递，入队失败及取消时释放该交付状态。完成、失败、取消及时交付，停止接受该任务的迟到进度；任务被替换后在应用结果时检查业务身份。State 的同轮合并不代替跨线程消息合并，进度交付策略留在业务层。隐藏／无 drawable 不能阻止业务状态、任务结局和清理，不依赖可见帧处理这些工作。

对应下层合同见[窗口集成](SCOOP-WINDOW-SPEC.md#8-asyncio-集成)、[asyncio 宿主接入](SCOOP-ASYNCIO-SPEC.md#113-宿主接入合同)与[HTTP 下载](SCOOP-HTTP-CLIENT-SPEC.md#95-文件下载)；使用示例与取舍见[UI 设计](../ui/DESIGN.md#92-异步下载与-ui-线程)。

具体控制器和显示节流策略归独立下载示例及其业务模型，不加入框架生产依赖或公共接口。组件可在 ComponentScope 中登记普通 Closeable 控制器：关闭时停止接受新发布并取消自己的 Job／后台计算，异步 finally 继续由现有宿主驱动。示例以真实 Thread 执行已下载文件的 SHA-256，线程内创建并关闭文件、缓冲区和 Digest，终态在计算及资源清理之后投递。应用持有活动线程集合，正常终态在 owner 回收句柄并移除任务，事件循环退出后取消并 join 尚未回收的线程；不能依赖可能被拒绝或丢弃的业务 post 完成最终清理。组件卸载不 join 任意后台工作。共享 State、HttpClient、外部字体等借用对象仍由应用所有者关闭；组件卸载不改变其他窗口对共享模型的正常订阅。

## 10. 验收要求

框架的普通库行为归 `scoop.ui` 的 scoop.test。框架测试及自绘示例不依赖 `scoop.ui.component`；控件行为归控件库自身。验收覆盖公开行为，至少包括：

1. 只依赖框架的自绘组件连续更新复用实例和自有 State；两个挂载的自有状态各自独立，卸载后重新出现从初始值开始；同一组件同时委托外部 State，重挂载时保留 Store 中的值。
2. 多组件共享 State、条件读取移除旧依赖、经普通模型 getter 间接读取、只读 val 委托、同轮合并，以及 render／measure／scene／draw 内写入的错误路径；卸载一个订阅者不影响其余订阅或共享值。
3. Component／Element 的 Props 更新不重跑构造器；update 在隐藏时也更新输入；Binding 修改同一原状态、切换绑定后使用新写入路径并退订旧 State；普通 class 内部变更不会被伪装为自动可观察。
4. keyed 列表插入／删除／重排、重复 key、位置身份、组件／Props 类型替换、分支与跨父级移动。
5. 资源取得失败、首轮 render／onMount／update 失败、measure／scene／draw 失败、缓存替换、正常卸载、回调内关闭及清理异常下的剩余资源释放。
6. 隐藏／零尺寸／Deferred 期间仍执行逻辑卸载；成功提交前新区域不命中，失败帧不发布几何，已卸载区域立即退出交互。
7. 外部自绘 Element 的约束、子项测量／放置、背景／子项／前景顺序、变换与 clip、Rect／Path 输入命中和真实窗口关闭。
8. Surface 动画直接驱动测量和绘制、同帧采样、重定目标、隐藏恢复、卸载后取消完成回调，动画不重跑未失效的 Component.render。
9. 普通 Application.run 与现有 runWithAsyncio 的宿主组合、同 owner 的异步进度更新、外线程直接读写 State 的拒绝、proxy 交付、任务取消及关闭时投递拒绝；迟到结果不访问已释放绘图资源。
10. LayoutMetrics 基线、Component 透传、容器偏移；未放置子树保持挂载、继续逻辑更新且不绘制，下一轮不复用旧放置。
11. 不依赖控件库的公共 ParagraphHost 自绘组合：多样式文字、图片和交互子项，一次测量、RTL 对应、截断后退出命中与焦点、恢复后保留 State、失败帧及尺寸动画重排。
12. 同一 Application／UiHost 下两个独立窗口根，共享模型时分别更新，输入与组件身份隔离，普通关闭一个根不卸载另一根；模态与自绘窗框操作在 window 对应能力实现后单独验收。
13. 应用 Context 在延迟组件构造和后续业务入口有效；短暂绑定不被 View／工厂隐式捕获；Context 重绑定不替换已初始化的 State 委托，缺失异常沿正常挂载失败路径清理。
14. 首次画面及已有通知收尾后的稳定区间内，render／update／measure／scene／draw、目标获取、GPU 提交和呈现次数不再增加，UI 不产生周期唤醒或空任务；单次状态变化／合并请求能唤醒必要更新，随后再次停止。
15. 动画自然完成、暂停、取消、零时长和卸载后停帧；完成回调产生的新失效不丢失。一窗动画不带动其他静止根，隐藏／Deferred 不忙重试，恢复后正确呈现再停帧。普通 run 与 runWithAsyncio 均覆盖静止和唤醒。

独立的自绘示例 Cone 通过已编译的公开 UI 接口实现交互和动画，不引入控件库作为间接依赖。随后以独立控件库的 Label／Button／Checkbox／容器实现及 Counter 实例验证同一接口足以构建常用控件；不能通过向框架增加控件类型分支补足缺失能力。

单独的集成示例／测试 Cone 显式依赖 UI、window.asyncio、HTTP client 及实际使用的下层库，以本地 HTTP 服务验证下载进度驱动自绘界面、未知总长度、成功／失败／取消、任务替换及高频进度合并；覆盖窗口隐藏和关闭时的任务／资源收尾。该组合不重复 HTTP 协议、重试或文件 ABI 的全部测试，也不把 HTTP 依赖引入 UI 核心测试。

native 呈现和窗口行为使用现有真实后端，不为测试单独搭一套生产 renderer 工厂。编译／链接合同使用少量必要的独立 Cone 或 fixture，不按每种组件重复普通类型错误与缓存检查。

静止验收用测试内计数及真实后端的调用记录或分析工具，同时检查逻辑／绘制入口、原生帧通知和 GPU 提交，不能仅凭画面不变或平均帧率低判定通过。先完成初始化及有限的在途通知，再用一次性等待观察覆盖多个显示刷新周期的稳定区间；测试自身不以逐帧 timer 唤醒应用。CPU／GPU 活动与能耗观察用于辅助定位，不把固定硬件百分比当成可移植门槛，也不向生产路径加入持续运行的监测循环。
