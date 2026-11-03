# 接收者 DSL builder 设计建议

日期：2026-11-03。状态：API 调研与设计依据；[迁移计划](DSL-MIGRATION.md)已开始分批实施，JSON、URL、Path、纯文本／文本样式、公共 Style／部件 Options 及控件局部样式构造入口已迁移，UI 和其余 API 继续按计划推进。迁移按新风格直接重设计公开入口并更新调用方，不承担历史版本兼容责任。本文按现有模型讨论适用范围、构建协议和生命周期，代码用于解释结构；完整签名、必需参数与实施范围按迁移计划落实到对应规范。范围不包含 CodeBlock。语言已有的接收者函数类型、尾随 lambda 和隐式 this 足以支持基本写法，见 [语言规范 §8.1](../specs/SCOOP-SPEC.md)。

## 1. 建议

将 builder 作为 UI 的常用声明入口。一次 builder 调用同步收集属性和子 View，结束时生成现有的不可变 Props／View；框架继续使用同一 Component、Element、状态观察、key、挂载和测量／场景流程。所需新增代码主要位于控件库的构建入口，不需要改变底层渲染模型。

这一层解决两个现有问题：Row／Column／Stack 目前需要显式组织 Array<View>；Input／Textarea／VirtualList 等入口有较长的命名参数列表。简单的 `label("Hello")` 仍适合直接使用，完整 Props 和 Component 构造入口继续服务自定义组合。

## 2. 最小实现结构

每个控件的 builder 保存可写配置，转换为该控件已有 Props。具有多个子项的容器复用一个普通 ChildrenBuilder 基类，其中保存子项序列、`add(View)` 和标准控件的真实成员方法；Row／Column／Stack 等具体 builder 增加各自布局属性。叶子 builder 只声明自己的配置。已有 View 和第三方组件统一通过 add 加入，应用也可在无歧义的命名／导入下提供接收者扩展。

顶层 `stack { ... }` 返回 View；子项接收者中的 `stack`、`label`、`button` 等成员调用向当前容器追加一次并返回 Unit。两种入口使用同一个私有构建函数，以免在重载调用中递归或重复追加。`add(existingView)` 支持现成 View 和自定义组件；`if`、`when`、`for` 直接使用普通语言控制流。

当前语言规则下，建议标准子项入口使用真实成员：同名顶层工厂与接收者扩展同时可应用时，裸调用会因最具体候选并列而歧义，lambda 的 Unit 结果不能用于强行选择自动追加的版本。真实成员按既有优先级胜过顶层工厂，并且可以从公共 builder 基类继承。这决定构建入口如何组织；如以后需要同名扩展优先于普通函数的 Kotlin 式规则，应先单独设计语言候选层。

```kotlin
fun stack(configure: StackBuilder.() -> Unit): View {
    val builder = StackBuilder()
    configure(builder)
    return component(builder.toProps(), { props -> Stack(props) }, builder.key)
}
```

这里的 `StackProps`、`Stack`、`ViewKey` 和 `component` 均为现有类型／入口。builder 不额外创建一种 Component 形状；原有描述以具体 Props 类型和组件类型确定 shape，相同 key 下仍能复用原实例。

子项序列在转换时保存数组快照；之后修改 builder 不改变已经生成的描述。样式等嵌套 builder 也先生成自己的配置快照。回调和调用方提供的状态引用按现有 Props 契约保存，不进行深拷贝。

具体属性名、类型和必需项按对应 Props 的实际语义设计。普通文本、StyledText、单内容槽和多子项等差别应继续表达清楚；便捷 setter 最终写入同一配置值，避免保存两份互相竞争的状态。先统一 builder 的配置、子项、快照和身份规则，再在各 API 的迁移中确定公开名称。

## 3. 状态、回调和资源

builder 每次 render 都可能执行，持久模型由组件或应用保存。Input／Textarea 必须接收现有 TextEditorState；VirtualList 必须接收 ItemSource 与 ItemExtent。这些必需值可以继续放在前置参数，避免先构建一个没有模型的配置，再补运行时缺失检查。

```kotlin
// nameEditor and accepted are fields of the mounted component.
val owner = this
column {
    spacing = 12.0
    input(owner.nameEditor) {
        placeholder = "Name"
        onSubmit = { model -> owner.saveName(model.text) }
    }
    button("OK", onClick = { owner.accepted.value = true }) {
        enabled = owner.nameEditor.text.byteLength > 0L
    }
}
```

上述示例为建议 API。render 中对 State 的读取仍由现有依赖收集观察；事件回调随后修改应用 State，由框架安排更新。回调若捕获并修改短命的 builder，只会修改那次构建器，不会更新已发布 Props，也不会自动触发 render，应在 API 文档和示例中明确这一点。

ScrollState、TextEditorState、FocusHandle、VirtualListHandle、LayoutAnchor 等保持原有所有权和身份。builder 只引用它们，不因声明一次控件而创建另一份状态。原来已有组件内部默认状态的入口继续沿用原规则。

字体、painter、订阅和原生输入连接仍由挂载作用域管理。builder 执行只生成描述，不负责订阅事件、建立 native session 或逐帧创建资源。UI 线程和 render 阶段的现有读写规则不变。

## 4. UI 中适合同时调整的入口

| 入口 | 当前结构与收益 | 设计边界 |
| --- | --- | --- |
| Row／Column／Stack、Frame | Array<View> 或单一 child 加布局参数；最直接改善嵌套界面阅读 | 多子项保持源码顺序；单内容槽仍接收一个 View，多项用显式容器组合 |
| Label／Button／Checkbox、Input／Textarea | Props 已集中定义配置；复杂控件参数较多 | 复用现有组件与行为；文本编辑模型、Binding 等仍由调用方传入 |
| Absolute | 当前是 Array<AbsoluteChild>，每项组合 View、两轴位置和 zIndex | 将单项定位与其内容写在一起，最后生成同一 AbsoluteChild；不增加新的全局定位机制 |
| Portal 与 modal 配置 | PortalOptions 中已有 placement、dismiss、modal、handle 和 onClose | DSL 组织配置与内容，继续复用现有打开条件、焦点屏障、关闭顺序和恢复 |
| ScrollArea | 内容、滚动参数、条样式和缓存选项可就近配置 | 绘制缓存与子项实例缓存保持独立；ScrollState 更新仍通过现有操作 |
| VirtualList | 已有 ItemSource、ItemExtent、retention、navigation、paintCache 等 | builder 保存惰性 source／项构建回调，不枚举全量数据；实例创建仍由 lazyElement 决定 |
| Style、SurfaceOptions、状态覆盖 | 不可变配置嵌套较深，适合局部配置块 | 保留 Inherit、Set(None)、Set(value) 三种意义；省略不能误变成清除继承值 |
| StyledText、TextSpan 和行内内容 | 已迁移为 styledText／textSpan 及文本样式接收者工厂，成员按顺序收集内容 | 生成不可变富文本描述，保持继承与 InlineKey；UI 行内 View 装配继续按计划推进 |
| Shortcut、焦点／键盘导航配置 | 快捷键绑定、NavigationItem 数组和 options 可集中描述 | 只生成配置值；ShortcutScope、FocusScope 和监听器按挂载生命周期创建与更新 |

event routing 的 handler、capture、bubble 和 defaultAction 已经是普通回调赋值。可以在需要组合输入容器的 API 中提供配置块，但没有必要为事件传播再造一套 DSL 执行器。`requestFocus`、`scrollTo`、文本编辑命令及 Canvas 绘图继续作为显式动作。

建议先覆盖普通容器、现有控件和样式，以主题展示中的实际界面验证可读性；随后覆盖 Absolute、Portal、滚动容器及富文本。导航／快捷键配置在具体组件使用它们时接入，避免把每个小型 options 都扩成独立子系统。

## 5. 其他已有库中的同类机会

| API | 现状 | 建议 |
| --- | --- | --- |
| [HTTP 路由](../../sysroot/src/scoop.http.server/src/router.scoop) | 已有 RouterBuilder、RouteGroup 和同步 group 回调，build 冻结路由 | 高收益：添加 `routing { ... }`，group 改为接收者 lambda；沿用路由检查、中间件顺序和冻结边界 |
| [JSON 构造](../../sysroot/src/scoop.json/src/builders.scoop) | 已迁移为 `jsonObject { ... }`／`jsonArray { ... }`，add 返回 Unit，构造和快照方法仅供库内部使用 | 已完成：保留重复字段拒绝、插入顺序及已有 JsonValue；库测试与实际跨 Cone 示例通过 |
| [HTTP 客户端配置](../../sysroot/src/scoop.http.client/src/config.scoop) | HttpClientConfig 含 TLS、超时、池、重试、代理等多层不可变值 | 高收益：分组配置 builder 生成同一 Config；原有验证仍在配置边界完成 |
| [HTTP 请求](../../sysroot/src/scoop.http.client/src/request.scoop) | 不可变 HttpRequest 组合 URL、fields、body、options | 请求／头／multipart 的装配适合 builder；发送仍显式调用，保留请求体的资源与重放规则 |
| [URL](../../sysroot/src/scoop.url/src/builder.scoop) | `url { ... }`／`base.copy { ... }` 配置独立快照，encoded 组件使用属性，原文编码与追加操作返回 Unit | 保留 raw／encoded、整条 path／segment、空与缺失组件及已有 authority 网络解释 |
| [Graphics Path](../../sysroot/src/scoop.graphics/src/drawing/path.scoop) | `path(fillRule = ...) { ... }` 返回独立 Path，追加／reset 返回 Unit | 临时 builder 在 use 中关闭，返回 Path 的所有权交给调用方；增量构造显式管理同一个 PathBuilder 实现 |
| [CLI schema](../../sysroot/src/scoop.cli/src/cli.scoop) | 已有按类型元数据／注解生成 schema 的主路径 | 优先级低；只为实际手写 CliEntry 集合提供便捷入口，继续利用类型和注解 |

Point、Size、Rect、Duration 等小值构造已经简洁。网络发送、文件读写、资源关闭以及动画播放等动作也有明确执行语义，不需要为了外观一致而改成配置 builder。StringBuilder 和 ByteArrayBuilder 已明确排除，本次不修改它们的 API、实现或调用方式。

## 6. 与 Kotlin 的范围差异

Scoop 还没有 `@DslMarker`。DSL 迁移的 D0a 已加入 receiver function 边界规则，叶子 `label { ... }` 不能隐式借用外层容器的 `button` 或布局属性。普通嵌套写法由库实现，外层隐式访问由编译器名称绑定规则限制。

D0a 的规则是：receiver function 已有自己的 receiver 时，裸成员访问隐式绑定外层 `this` 直接报编译错误，不设警告期。外层组件与外层 builder 一视同仁，receiver function 内定义的普通回调也不能恢复越过该边界的隐式查找。显式 `this@Outer`／`this@outer`、保存的对象引用和普通词法捕获仍可用；没有经过 receiver function 边界的普通闭包不受这条新增限制。完整规则及实现、验收范围见[迁移计划第 1.4 节](DSL-MIGRATION.md#14-禁止隐式使用外层-receiver)和[语言规范 §8.1.2](../specs/SCOOP-SPEC.md#812-lambda-与匿名函数)。

`@DslMarker` 仍列为后续加入的能力，仅用于限制 receiver 查找、减少误用，不改变已经设计好的 API 功能。上述全局禁止规则比按标记限制更强，已经覆盖此处讨论的外层 receiver 误用；后续标记设计应说明与全局规则的关系，不放宽这条规则，也不为相同限制再增加一套检查。加入标记不作为本次迁移前置。

API 的完整构造与组合能力通过显式接收者保持。调用方只需明确外层对象，不改变构建结果、Props／View、状态、回调或资源生命周期。禁止隐式绑定是编译期名称使用规则，不能通过库内异常重载或运行期当前容器栈模拟，也不能在捕获列表中一律拒绝合法的显式外层引用。

不引入额外的 DSL AST、全局 builder context、反射属性写入、代码生成器或第二套 reconciliation。当前实际调用支持的配置与组合即可完成这一层。

## 7. 验证范围

独立的临时普通 Cone 已用实际预编译的 scoop.ui／scoop.ui.component 完成格式化、类型检查和 aarch64 release 构建，验证接收者配置块与嵌套子项声明。探针直接生成现有 LabelProps／ButtonProps／StackProps 与 View；2 项 scoop.test 均通过，覆盖属性快照、源码顺序、循环构建和回调修改应用 State。探针中的命名仅用于验证语言与公共 API 的组合，不发布为生产库；完成后清理临时源码、测试产物与报告。

另一个小型语言探针确认同名普通函数与接收者扩展的歧义诊断；采用共享 builder 基类的真实成员后，格式化、类型检查、release 构建和 1 项嵌套收集测试均通过。该结果用于选择当前语言支持的入口组织方式，没有修改编译器，也不把探针名固化为公开 API。

正式迁移时，验收重点是 DSL 与原入口生成同一类描述、条件／循环与稳定 key 的实例复用、嵌套配置快照、事件回调读取最新 Props，以及懒列表不提前构造离屏项。所有 Cone 改动使用 scoop.test 验证；受影响的 fixture 先评估是否应迁到 scoop.test，仅在确有必要保留时更新。D0a 的编译错误与诊断位置属于必须使用编译器测试的语言规则。未修改 LIR／codegen 时只做本地验证，不因增加 builder 重跑编译器全量或所有系统库测试。
