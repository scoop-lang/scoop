# 接收者 DSL builder 设计建议

日期：2026-11-03。状态：API 调研与设计建议；生产 API 尚未迁移。以当前 UI 基础能力为实现前提，范围不包含 CodeBlock。语言已有的接收者函数类型、尾随 lambda 和隐式 this 足以支持下面的基本写法，见 [语言规范 §8.1](../specs/SCOOP-SPEC.md)。

## 1. 建议

将 builder 作为 UI 的常用声明入口。一次 builder 调用同步收集属性和子 View，结束时生成现有的不可变 Props／View；框架继续使用同一 Component、Element、状态观察、key、挂载和测量／场景流程。所需新增代码主要位于控件库的构建入口，不需要改变底层渲染模型。

```kotlin
stack {
    label {
        title = "Hello"
    }
    button {
        title = "OK"
        enabled = false
    }
}
```

Stack 在当前 API 中表示叠放；顺序排列的表单使用 column／row。这次调整不改变容器的布局含义。

这一层解决两个现有问题：Row／Column／Stack 目前需要显式组织 Array<View>；Input／Textarea／VirtualList 等入口有较长的命名参数列表。简单的 `label("Hello")` 仍适合直接使用，完整 Props 和 Component 构造入口继续服务自定义组合。

## 2. 最小实现结构

每个控件的 builder 保存可写配置，转换为该控件已有 Props。具有多个子项的容器实现一个只负责 `add(View)` 的公共子项接收者；控件库和应用可用普通扩展函数为它增加组件入口。叶子 builder 只声明自己的配置。

顶层 `stack { ... }` 返回 View；子项接收者中的 `stack`、`label`、`button` 等调用向当前容器追加一次并返回 Unit。两种入口使用同一个私有构建函数，以免在重载调用中递归或重复追加。`add(existingView)` 支持现成 View 和自定义组件；`if`、`when`、`for` 直接使用普通语言控制流。

```kotlin
fun stack(configure: StackBuilder.() -> Unit): View {
    val builder = StackBuilder()
    configure(builder)
    return component(builder.toProps(), { props -> Stack(props) }, builder.key)
}
```

这里的 `StackProps`、`Stack`、`ViewKey` 和 `component` 均为现有类型／入口。builder 不额外创建一种 Component 形状；原有描述以具体 Props 类型和组件类型确定 shape，相同 key 下仍能复用原实例。

子项序列在转换时保存数组快照；之后修改 builder 不改变已经生成的描述。样式等嵌套 builder 也先生成自己的配置快照。回调和调用方提供的状态引用按现有 Props 契约保存，不进行深拷贝。

命名建议：用户示例中的 title 可作为纯文本便捷属性；实际 Label／Button 还支持 StyledText，正式设计应为普通文本和富文本保留明确入口。建议统一使用 `text: String` 与 `content: LabelText`，text 的 setter 写入同一个 content，按源码最后一次赋值生效；不同时保存两份互相竞争的正文。不额外保留 title／text 两个同义名称。最终命名在迁移控件 API 时统一确定。

## 3. 状态、回调和资源

builder 每次 render 都可能执行，持久模型由组件或应用保存。Input／Textarea 必须接收现有 TextEditorState；VirtualList 必须接收 ItemSource 与 ItemExtent。这些必需值可以继续放在前置参数，避免先构建一个没有模型的配置，再补运行时缺失检查。

```kotlin
// nameEditor and accepted are fields of the mounted component.
column {
    spacing = 12.0
    input(nameEditor) {
        placeholder = "Name"
        onSubmit = { model -> saveName(model.text) }
    }
    button {
        text = "OK"
        enabled = nameEditor.text.byteLength > 0L
        onClick = { accepted.value = true }
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
| StyledText、TextSpan 和行内内容 | 当前显式拼接文本段数组 | 同步生成现有富文本描述；排版、导航和行内 View 绑定继续走现有文字层 |
| Shortcut、焦点／键盘导航配置 | 快捷键绑定、NavigationItem 数组和 options 可集中描述 | 只生成配置值；ShortcutScope、FocusScope 和监听器按挂载生命周期创建与更新 |

event routing 的 handler、capture、bubble 和 defaultAction 已经是普通回调赋值。可以在需要组合输入容器的 API 中提供配置块，但没有必要为事件传播再造一套 DSL 执行器。`requestFocus`、`scrollTo`、文本编辑命令及 Canvas 绘图继续作为显式动作。

建议先覆盖普通容器、现有控件和样式，以主题展示中的实际界面验证可读性；随后覆盖 Absolute、Portal、滚动容器及富文本。导航／快捷键配置在具体组件使用它们时接入，避免把每个小型 options 都扩成独立子系统。

## 5. 其他已有库中的同类机会

| API | 现状 | 建议 |
| --- | --- | --- |
| [HTTP 路由](../../sysroot/src/scoop.http.server/src/router.scoop) | 已有 RouterBuilder、RouteGroup 和同步 group 回调，build 冻结路由 | 高收益：添加 `routing { ... }`，group 改为接收者 lambda；沿用路由检查、中间件顺序和冻结边界 |
| [JSON 构造](../../sysroot/src/scoop.json/src/builders.scoop) | 已有 JsonObjectBuilder／JsonArrayBuilder 和 build 快照 | 高收益：`jsonObject { ... }`／`jsonArray { ... }` 薄封装，保留重复字段拒绝及已有 JsonValue |
| [HTTP 客户端配置](../../sysroot/src/scoop.http.client/src/config.scoop) | HttpClientConfig 含 TLS、超时、池、重试、代理等多层不可变值 | 高收益：分组配置 builder 生成同一 Config；原有验证仍在配置边界完成 |
| [HTTP 请求](../../sysroot/src/scoop.http.client/src/request.scoop) | 不可变 HttpRequest 组合 URL、fields、body、options | 请求／头／multipart 的装配适合 builder；发送仍显式调用，保留请求体的资源与重放规则 |
| [URL](../../sysroot/src/scoop.url/src/builder.scoop) | 已有 UrlBuilder 的链式 setter 和 build | 小改即可使用接收者入口；raw／encoded 和整条 path／segment 必须继续区分 |
| [Graphics Path](../../sysroot/src/scoop.graphics/src/drawing/path.scoop) | 已有 Closeable PathBuilder 和独立 Path 快照 | 可添加 `path { ... }`，构建器在 use 中关闭，返回 Path 的所有权仍交给调用方 |
| [CLI schema](../../sysroot/src/scoop.cli/src/cli.scoop) | 已有按类型元数据／注解生成 schema 的主路径 | 优先级低；只为实际手写 CliEntry 集合提供便捷入口，继续利用类型和注解 |

Point、Size、Rect、Duration 等小值构造已经简洁。网络发送、文件读写、资源关闭以及动画播放等动作也有明确执行语义，不需要为了外观一致而改成配置 builder。

## 6. 与 Kotlin 的范围差异

当前 Scoop 的隐式接收者从内向外查找，语言没有 `@DslMarker`。因此，在叶子 `label { ... }` 内仍可能找到外层容器的 `button` 或布局属性，意外把内容添加到外层。普通嵌套写法可以在库层实现，但禁止这种外层隐式访问需要真实的语言作用域规则。

第一轮使用现有接收者语义，示例清楚展示容器／叶子的区别，必要时通过显式对象访问外层。若要提供 Kotlin 式的 DSL 作用域隔离，应另行修改语言规范与编译器，并用真实嵌套 builder 的正反例验证。它不是实现当前基本语法的前置条件，也不通过库内异常重载或运行期当前容器栈模拟。

不引入额外的 DSL AST、全局 builder context、反射属性写入、代码生成器或第二套 reconciliation。当前实际调用支持的配置与组合即可完成这一层。

## 7. 验证范围

独立的临时普通 Cone 已用实际预编译的 scoop.ui／scoop.ui.component 完成格式化、类型检查和 aarch64 release 构建，包含用户原样的 `stack { label { title = ... }; button { ... } }`。探针直接生成现有 LabelProps／ButtonProps／StackProps 与 View；2 项 scoop.test 均通过，覆盖属性快照、源码顺序、循环构建和回调修改应用 State。该探针用于确认语言与公共 API 的组合，不发布为生产库；完成后清理临时源码、测试产物与报告。

正式迁移时，验收重点是 DSL 与原入口生成同一类描述、条件／循环与稳定 key 的实例复用、嵌套配置快照、事件回调读取最新 Props，以及懒列表不提前构造离屏项。沿用相关 Cone 的 scoop.test 和现有原生组合用例；不因增加 builder 重跑编译器全量或所有系统库测试。
