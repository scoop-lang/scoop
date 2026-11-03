# 接收者 DSL API 迁移计划

日期：2026-11-03。状态：已开始分批实施。设计依据见 [DSL builder 调研](DSL-BUILDERS.md)，公共命名约定见 [库 API 风格](../specs/SCOOP-API-STYLE.md)。

将 UI 和其他需要组合内容、收集条目或分组配置的公共 API 按接收者 DSL 风格重新设计，并更新规范、实现、调用方、测试和示例。一次构造得到完整描述或配置，执行动作继续由明确的操作入口承担。

本次直接演进 API，不承担任何历史源码、API、ABI 或产物兼容责任。被替换的长参数函数、显式配置回调签名和重复便利入口直接删除，不设置弃用期、兼容别名、转发层或双轨实现。Props、配置值和可复用的增量构造器是否公开，只由新设计中的实际用途决定。依赖产物按正常构建流程重新生成。

## 1 统一设计规则

### 1.1 构造入口与必需数据

- 常用入口采用 `name(requiredArguments..., configure: Builder.() -> Unit)`；有完整默认配置时允许省略 configure。配置块同步执行一次，返回完整结果，不隐式发送请求、创建窗口、启动进程或挂载 UI。
- 有效默认值来自对应领域合同。文本编辑模型、Binding、ItemSource、ItemExtent、单个内容 View，以及没有有效默认值的内容和动作，通过必需参数传入。不得用未初始化字段、`None`、空内容或空回调掩盖必需项，再在结束时补缺失检查。
- 普通标量使用可写配置属性，嵌套配置使用真实成员方法，序列使用 `add` 等明确追加操作。标量后一次设置覆盖前一次；序列按执行顺序追加。嵌套配置块基于当前配置修改，空块不重置先前设置；显式赋入完整配置值表示替换。
- 必需参数加空配置即可完成的简短调用直接由新签名覆盖，例如 `label("Hello")`。这不是另一套旧重载。Point、Size、Rect、Duration、单个 header／条目等简洁值继续使用普通构造或明确转换。
- 各库直接实现自己的构造入口，共享实际已有的值类型和必要 helper。只在存在真实复用时提取基类，不建立跨库 Builder 框架。

### 1.2 快照与生命周期

构造器的可变字段、子项序列和嵌套配置在发布时转换为稳定结果。之后修改被捕获的 builder，不改变已经生成的结果。容器结构按领域需要复制；不可变子值可以共享。状态模型、资源引用和事件回调按明确的借用／所有权合同保存，不进行深复制。

配置回调抛错时整个入口抛错，不发布部分结果。需要关闭的临时资源沿 `use`／`finally` 清理。Path 的作用域构造负责关闭 PathBuilder，返回的 Path 由调用方拥有；UI builder 只生成描述，字体、painter、输入连接和订阅在挂载作用域创建。

路由构建完成后冻结路由及分组；JSON、URL、文本和 UI 发布独立描述。两类行为分别按用途定义，不强制共用一种 builder 状态机。数值、字段、引用与组合合法性在真实构造边界检查，后续消费复用结果。

持久 State、TextEditorState、ScrollState、FocusHandle、VirtualListHandle、LayoutAnchor 和 ThemeSource 由应用或组件持有。事件回调读取最新 Props、修改应用模型；修改已经结束的 builder 不是状态更新。允许保存的 suspend handler／producer 仍显式写 `suspend`，构造期间不执行它们。

### 1.3 UI 子项与名称解析

`scoop.ui.component` 内的 ChildrenBuilder 负责多子项收集，提供 `add(View)` 和标准控件的真实成员方法。Row、Column、Stack builder 继承它并增加自己的布局配置。

- 顶层控件工厂返回 View；ChildrenBuilder 中的同名成员追加恰好一次并返回 Unit。两者调用同一个私有构建 helper，避免递归选中自身或重复追加。
- `if`、`when`、`for` 使用普通语言求值顺序。已有 View 和第三方组件通过 `add` 加入，key 按直接父级内的规则参与匹配。
- Frame、ScrollArea、Portal、Styled 是单内容入口，前置接收一个 View。Absolute 的 `item(content) { ... }` 生成一个完整 AbsoluteChild；它不继承普通 ChildrenBuilder，也不为多个 View 隐式合成布局容器。
- 在 ChildrenBuilder 作用域内需要单独取得 View 时，使用提前构造的值或有别名的顶层工厂，例如 `import scoop.ui.component.label as makeLabel` 后调用 `add(makeLabel("Text"))`。不要写 `add(label("Text"))` 并依赖返回类型选择工厂，成员版 label 返回 Unit。
- 容器、叶子和配置块各自使用自己的 receiver。访问外层组件或其他 builder 时显式写出接收者，按第 1.4 节禁止跨 receiver function 边界的隐式绑定。

上述组织基于[语言规范 §8.1](../specs/SCOOP-SPEC.md#81-函数类型函数值与-closure)。同名顶层工厂与 receiver 扩展不作为标准子项入口组合；receiver 形状与普通首参数互相适配，也不能靠这一区别同时声明两套回调重载。

### 1.4 禁止隐式使用外层 receiver

采用编译错误规则，不设置警告期或可关闭的兼容模式：receiver function 已有自己的 receiver 时，裸成员访问不得再隐式绑定外层 `this`。这同时适用于外层组件实例和外层 DSL builder，不依赖它们是否带 `@DslMarker`。D0a 已在语言规范、HIR 与编辑查询中实施此规则，后续 API 迁移按新规则验收。

- 在 receiver lambda、receiver 匿名函数及具有相同接收者边界的扩展函数中，隐式访问使用该边界内的 receiver。进入嵌套 receiver function 后，前一层 receiver 成为必须显式引用的外层；普通与 suspend 形式一致。
- 对实际选中的接收者检查绑定身份，而不是只比较类型。属性读取、赋值／复合赋值、方法与扩展调用、函数属性 invoke，以及需要隐式外层实例的引用／构造入口都遵守这条规则。诊断指向发生隐式使用的源码位置，并提示显式 receiver。
- `this` 仍指当前 receiver；`this@Outer`、`this@outer`、`owner.member` 和 `owner::method` 等显式访问继续可用。普通词法 `val`、参数、局部函数、顶层声明和 object 名称遵守各自查找规则；不能把普通词法捕获误判成外层 receiver 绑定。
- receiver function 内定义的无 receiver lambda／局部函数继承这条查找边界，不能通过多包一层回调恢复对更外层 `this` 的隐式访问。没有经过 receiver function 边界的普通闭包继续遵守原有捕获规则，本次不禁止所有 lambda 捕获 `this`。
- 外层已经定义好的函数值或局部函数按其定义处的规则检查；调用它或把普通函数值适配成 receiver 函数值时，不追溯重查其函数体，不改变普通函数适配和捕获语义。
- 保留完整 receiver 作用域和捕获来源供显式访问使用。检查放在 HIR 的实际名称／接收者绑定边界，不在最终捕获列表中一律拒绝外层 `this`，也不因探测过某个未选中的候选而报错。原来需要绑定外层 receiver 的调用必须明确诊断，不能静默改绑另一个同名函数。

实现复用现有 [ReceiverBinding 与 ReceiverScope](../../compiler/hir-lower/src/model.rs)、[receiver 查找与显式标签解析](../../compiler/hir-lower/src/expr/receivers.rs)及捕获信息；读取、写入、调用和编辑查询采用同一规则。正常 API 功能通过显式对象引用完整表达，构建结果、状态、回调、资源生命周期和 runtime ABI 不变。

`@DslMarker` 仍列为后续加入的能力，仅用于限制 receiver 查找、减少误用，不改变已经设计好的 API 功能。上述全局规则比按标记限制更强，已经覆盖本次讨论的外层 receiver 误用；后续标记设计应说明与全局规则的关系，不放宽这条禁止规则，也不为相同限制重复增加检查。加入标记不作为本次 API 迁移前置。

### 1.5 配置覆盖

完整 Style 与局部 StyleOverride 各有明确入口。后者保留 `Inherit`、`Set(None)`、`Set(value)`：省略是继承，显式清除不能退回继承。便捷设置方法与显式继承方法只写同一个类型化槽，不保存互相竞争的属性副本。效果数组显式设置时整体替换，并保持顺序。

SpanStyle 的 `lineHeight: Option<Option<Double>>` 同样区分继承、清除和设置。HTTP per-call options 中的 None 按各字段的继承含义处理；禁用某项策略必须使用该策略自己的值，不能把 None 统一解释为禁用。重复配置块、空块和显式清除都纳入测试。

## 2 UI 迁移清单

下表中的入口名称是本计划的设计方向；实施时先把完整签名、属性和默认值写入对应正式规范，再替换代码。每项覆盖该入口的全部现有能力，不另留一条旧 API 承载高级选项。

| 范围与现有实现 | 新入口与构造方式 | 必须覆盖的语义 |
| --- | --- | --- |
| [Row／Column／Stack](../../sysroot/src/scoop.ui.component/src/layout/containers.scoop) | `row { ... }`、`column { ... }`、`stack { ... }`；ChildrenBuilder 收集子项 | 源码顺序、空容器、循环、稳定 key、spacing、alignment；生成同一套组件描述 |
| [Label](../../sysroot/src/scoop.ui.component/src/label.scoop)、[Button](../../sysroot/src/scoop.ui.component/src/button.scoop) | `label(text) { ... }`；`button(text, onClick = ...) { ... }`；String／StyledText 按类型区分 | 字体、段落、行内内容、主题、局部样式、enabled、focus、最新回调；文字和必需动作不使用缺失占位 |
| [Checkbox](../../sysroot/src/scoop.ui.component/src/checkbox.scoop) | `checkbox(binding) { ... }`；自有状态入口以 `initialChecked` 表达初始化值 | 外部 Binding 与组件自有 State 是两种实际状态模型；同 key 重绘不重置自有状态，更换 Binding 立即生效 |
| [Input](../../sysroot/src/scoop.ui.component/src/input.scoop)、[Textarea](../../sysroot/src/scoop.ui.component/src/textarea.scoop) | `input(editor) { ... }`、`textarea(editor) { ... }` | 完整编辑、提交、只读、校验说明、方向、换行、行数、滚动条、选择／光标样式与回调配置 |
| [Frame](../../sysroot/src/scoop.ui.component/src/layout/frame.scoop)、[Styled](../../sysroot/src/scoop.ui.style/src/styled.scoop) | `frame(content) { ... }`、`styled(content) { ... }` | 一个内容槽、尺寸／padding／对齐、主题表面与效果；样式变空也不移除必要包装或重建内容 |
| [Absolute](../../sysroot/src/scoop.ui.component/src/layout/absolute.scoop) | `absolute { item(content) { ... } }`，已有条目可显式 add | 每项 horizontal／vertical／zIndex 就近配置；逻辑子项顺序与绘制排序分别保持正确 |
| [Portal](../../sysroot/src/scoop.ui/src/view/view.scoop)、[弹层配置](../../sysroot/src/scoop.ui/src/overlay/options.scoop) | `portal(content) { ... }`；placement、dismiss、modal 配置块 | 打开条件仍由 View 组合表达；anchor、handle、onClose、焦点屏障、外部点击与焦点恢复 |
| [ScrollArea](../../sysroot/src/scoop.ui.component/src/scroll/area.scoop) | `scrollArea(content) { ... }` | 外部／内部 ScrollState、axes、chain、lineStep、滚动条、尺寸、focus 与 paintCache |
| [VirtualList](../../sysroot/src/scoop.ui.component/src/virtual/list.scoop) | `virtualList(source, itemExtent) { ... }` | 固定／可变 extent、overscan、retention、navigation、handle、anchor、滚动与缓存；构造不枚举 source 或创建离屏项 |
| [公共 Style](../../sysroot/src/scoop.ui.style/src/style.scoop)、[主题部件](../../sysroot/src/scoop.ui.style/src/theme/parts.scoop)、[控件样式](../../sysroot/src/scoop.ui.component/src/styles.scoop) | `style { ... }`、`styleOverride { ... }`；surface／marker／scrollbar／states 等嵌套配置 | 完整值与继承覆盖分开；ButtonStyle、CheckboxStyle、TextFieldStyle、SurfaceOptions、CheckboxOptions、RadioOptions、ScrollbarOptions 全量字段映射 |
| [主题参数](../../sysroot/src/scoop.ui.style/src/theme/values.scoop)与三套主题 | 对 palette、typography、effects、motion 和部件外观作分组配置 | macOS／Material／Fluent 及 Light／Dark 的实际主题生产代码和局部覆盖均迁移；小型几何值直接构造 |
| [StyledText／TextSpan](../../sysroot/src/scoop.graphics.text/src/content.scoop)、[InlineView](../../sysroot/src/scoop.ui/src/text/inline.scoop) | `styledText { text(...); span(...) { ... }; inlineObject(...) }`；UI 层提供行内 View 绑定装配 | 文本／span 数组快照、TextStyle／SpanStyle／ParagraphStyle、继承与顺序、InlineKey、替代文字和实际 View 匹配 |
| [快捷键](../../sysroot/src/scoop.surface/src/platform/darwin/shortcuts/types.scoop)、[导航](../../sysroot/src/scoop.ui/src/focus/navigation_options.scoop) | 快捷键序列、binding 和 navigation 配置块；key／action／handle 为必需参数 | 生成 ShortcutBinding／NavigationOptions／FocusOptions 等配置；Scope、Target、监听器仍在挂载时创建和更新 |

UI 新签名的代表用法如下。代码位于组件方法内，先显式保存组件引用；`nameEditor`、`accepted` 和 `saveName` 属于该持久组件／模型：

```scoop
val owner = this
column {
    spacing = 12.0
    label("Name")
    input(owner.nameEditor) {
        placeholder = "Your name"
        onSubmit = { model -> owner.saveName(model.text) }
    }
    button("Save", onClick = { owner.accepted.value = true }) {
        enabled = owner.nameEditor.text.byteLength > 0L
    }
}
```

Props、具体 Component／Element 类型和 View 仍是同一条构造与挂载路径。新 DSL 与程序化组合必须共享这一模型，不能因入口不同而产生不同 shape、丢失实际类型或把变化输入只捕获进首次工厂。

## 3 其他库的迁移清单

### 3.1 内容与数据构造

| 范围与现有实现 | 新 API 方向 | 调用方与验收重点 |
| --- | --- | --- |
| [JSON builders](../../sysroot/src/scoop.json/src/builders.scoop) | `jsonObject { add(name, value) }`、`jsonArray { add(value) }`；嵌套直接组合这两个值工厂 | JSON tests／examples、HTTP JSON 与展示材料；重复 key 拒绝、插入顺序、JSON null、数字 token、快照；不增加另一套 DOM 或 codec |
| [URL builder](../../sysroot/src/scoop.url/src/builder.scoop) | `url { ... }` 与 `base.copy { ... }`；scheme／port／encoded 组件使用属性，原文编码与整组 authority 替换使用明确操作，替换 `toBuilder/setX/build` 链 | URL tests／examples、HTTP Location／WebSocket、client request／proxy 和 charset 消费方；raw／encoded、整条 path／segment 分名；query 的重复项、无值／空值和 URI／form 编码区别 |
| [Graphics Path](../../sysroot/src/scoop.graphics/src/drawing/path.scoop) | `path(fillRule = ...) { moveTo(...); lineTo(...); ... }` 返回独立 Path；内部用 use 关闭构造器；追加／reset 返回 Unit，增量使用者显式管理同一个 PathBuilder 实现 | graphics、surface、主题 painter 和实际图形示例；正常／异常关闭、快照独立、reset 保留 fill rule、几何与调用方释放 |

这些入口统一一次性装配的调用方式。可复用 native builder 按真实生命周期使用；作用域入口负责单次构造和清理。

### 3.2 HTTP 装配与配置

| 范围与现有实现 | 新 API 方向 | 必须覆盖的语义 |
| --- | --- | --- |
| [字段与消息](../../sysroot/src/scoop.http/src/message/field_operations.scoop) | `httpFields { ... }` 收集有序 HttpField；供 client 请求、server 结果和 multipart 复用 | 支持真实字节和值文本入口，保留重复字段及顺序；add／set／remove 明确区分，Set-Cookie 不自动合并 |
| [ClientConfig／CallOptions](../../sysroot/src/scoop.http.client/src/config.scoop) | `httpClientConfig { ... }`、`httpCallOptions { ... }`，按领域分组 | TLS、pool、TCP、H1／H2 policy／limits／settings、decoding、timeouts、redirects、retries、authentication、proxy、cookies、cache、interceptors、onAttempt、observer 全覆盖 |
| [HttpRequest](../../sysroot/src/scoop.http.client/src/request.scoop) | `httpRequest(url) { method = ...; fields { ... }; body = ...; options { ... } }`；统一替换重复的长参数方法工厂 | URL 前置且完整；method、Host／framing、Content-Length、body metadata 和 per-call 覆盖继续按领域规则检查；构造不发送 |
| [请求体](../../sysroot/src/scoop.http.client/src/body/body.scoop)、[part](../../sysroot/src/scoop.http.client/src/body/part.scoop) | multipart 收集块，把 part 的内容、文件名／媒体类型和字段放在一起；文本／字节等简单 body 工厂直接使用 | 只组装 body plan；不提前打开／消费流，不调用发送 producer；boundary、长度、trailer、压缩及可重放／一次性语义完整 |
| [RouterBuilder／RouteGroup](../../sysroot/src/scoop.http.server/src/router.scoop) | `routing { ... }` 返回 HttpApplication；group 使用 receiver lambda；listen 的同步路由配置入口同步换成新签名 | 路由歧义／重复检查、方法匹配、group 前缀、middleware 进入／退出顺序、fallback 与冻结；保存 suspend handler，构造不运行请求 |
| [ServerOptions](../../sysroot/src/scoop.http.server/src/server_options.scoop)、[HttpResult](../../sysroot/src/scoop.http.server/src/result.scoop) | server／application 分组配置、`httpResult { ... }`，结果共用字段收集入口 | 配置与 listen／takeOwnership／respond 分离；结果保存 body／producer，提交后的响应修改仍受正常状态规则约束 |
| HTTP 附属配置 | 请求／服务端入口就近配置 download／resume、SSE／WebSocket、压缩／解压、multipart 接收和文件响应策略 | 覆盖对应已有 options，继续使用各领域资源、取消、重试与流控合同；单个小选项值直接赋入，不为每项创建独立顶层 DSL |

子配置只复用实际相同的类型。例如 client 默认 timeout 与 request timeout 共享配置实现，但 request 的“继承”语义单独表达；client/server 的 body 与资源模型不同，不强行合并。proxy selector、认证 provider、cookie jar、cache store、TLS identity 等现有对象由调用方提供，配置块不隐式创建一套服务容器。

### 3.3 同类应用配置

| 范围与现有实现 | 新 API 方向 | 边界与调用方 |
| --- | --- | --- |
| [WindowAttributes](../../sysroot/src/scoop.window/src/window.scoop)及 ApplicationOptions | `windowAttributes { ... }`／`applicationOptions { ... }`，构建可独立消费的创建配置；平台设置进入对应 builder | 覆盖 macOS titlebar／activation 和现有目标选项；同步更新 window／surface／UI 示例。createWindow、Application 创建和现有窗口操作保持明确执行时点 |
| [Readline Config](../../sysroot/src/scoop.readline/src/config.scoop) | `readlineConfig { ... }`，按 history、completion、键盘和显示配置组织 | 更新配置消费、examples、普通行为与既有 PTY 定向测试；描述构造不读取输入或创建编辑 session |
| [Command](../../sysroot/src/scoop.platform/src/process/command.scoop) | `command(executable) { ... }`，收集参数、环境、工作目录及 stdio | executable 必需，参数／环境输出快照；默认环境和目录的取值时点写入规范；spawn、PATH 查找与文件／管道所有权保持显式 |

### 3.4 已审查但不扩为 DSL 的入口

- StringBuilder 和 ByteArrayBuilder 完全排除在本次迁移之外，不修改其 API、实现或调用方式，也不增加对应的作用域构造入口。
- CLI 使用 TypeInfo、注解、Cli.schema 和 codec 的主路径。手写 CliEntry 当前主要服务 schema 规则测试，不据此新建另一套 option／flag／argument 语言。schema 配置若有实际调用调整，仍使用这条类型化主路径。
- 普通集合字面量与短值构造、单个 MathContext／日志 provider／文件打开选项等直接表达其值。字段多并不自动成为 DSL 迁移理由；查询结果、事件载荷、TypeInfo 和内部 native 数据也不是配置器。
- 网络发送、文件读写、响应提交、requestFocus、scrollTo、文本编辑命令、Canvas 绘图、动画播放、use、任务启动和普通事件回调保持动作语义，不一律改成 receiver 回调。
- CodeBlock、新控件、ORM 等尚未进入现有生产 API 的设计不在本次实施范围。将来采用本风格时按自己的功能需求设计，不作为本次验收前置。禁止隐式使用外层 receiver 纳入 D0a；`@DslMarker` 按第 1.4 节单独处理后续需求。

## 4 分层与代码组织

1. `scoop.graphics.text` 放纯文本／span／文本样式 builder，只认识 InlineKey 和公共文本数据；不依赖 UI，也不接受 View。
2. `scoop.surface` 放自身 ShortcutBinding 配置；`scoop.ui` 放 Portal、导航、焦点以及行内 View 绑定装配。它们均不反向依赖 `scoop.ui.component`。
3. `scoop.ui.style` 放 Style、覆盖、主题参数和部件配置；三套主题用相同公共配置能力产生各自数据。该层不出现标准控件列表。
4. `scoop.ui.component` 放 ChildrenBuilder、具体控件 builder 及控件样式组合，并用真实成员调用下层 Portal／Styled 等入口。生产实现仅使用下层 public API。
5. HTTP 公共字段收集属于 `scoop.http`，client/server 分别提供自己的配置和 body／result 装配；URL、JSON 和其他库各自定义入口，不新建 DSL Cone。

每种结果只有一条构建实现。配置初始化、Props 转换和实际组件选择按需要拆成小模块，避免把所有控件成员、所有配置和验证集中到一个大文件。公开类型与成员通过正常 import 和继承使用，不用 FQN 回退查找、注册表或反射写入属性。

## 5 实施顺序与交付

每批先修订正式规范，再改实现，随后同步迁移所有受影响的生产调用、测试和示例，格式化并完成该批检查与测试。删除旧签名和迁移调用方在同一批完成；最终清理批次不承担此前留下的旧 API 兼容负担。

工作分支为 `codex/receiver-dsl-migration`。每个可独立验收的变更完成后提交并推送，再合并最新的 `origin/main`，处理冲突并验证实际受影响部分后继续。每轮清理后续不再需要的临时源码、产物和日志；后续构建仍会使用的正常开发 sysroot 与共享缓存继续复用。

| 批次 | 工作与依赖 | 本批完成标准 |
| --- | --- | --- |
| D0 公共约定与签名清单 | 先将第 1 节规则写入 SCOOP-API-STYLE；各 API 批次实现前在所属规范列全第 2／3 节目标入口的属性、默认值、必需参数、返回值、快照／所有权与旧入口替换关系 | 每项现有能力都有新 API 归属；确认真实成员、单内容槽和命名冲突处理；更新相应 spec 的示例与交叉引用 |
| D0a 外层 receiver 显式访问 | 依赖 D0 公共约定；先修订语言 §8.1.2／8.1.3 及相关引用，再在 HIR 与编辑查询实施第 1.4 节的编译错误；迁移现有 receiver 用例 | 隐式外层读取／写入／调用被准确拒绝，显式访问和词法捕获正常；同类型 receiver、嵌套普通回调、泛型与跨 Cone 正反例通过 |
| D1 UI 文本和样式基础 | 依赖 D0a；实现纯文本／span／TextStyle／SpanStyle／ParagraphStyle、Style／StyleOverride、主题部件和主题参数配置；迁移三个主题生产者 | 文本顺序和继承、覆盖三态、数组与嵌套快照测试通过；三个主题仍产生完整绘图数据 |
| D2 常用 UI 声明 | 依赖 D1；实现 ChildrenBuilder、Row／Column／Stack、Label／Button／Checkbox、Frame／Styled、Input／Textarea；迁移主题展示及所有直接调用 | 新风格真实窗口运行；按 key 更新保留实例／状态；最新回调、文本模型和主题切换通过原有行为测试 |
| D3 UI 组合与交互配置 | 依赖 D2；迁移 Absolute、Portal／modal、ScrollArea、VirtualList、行内 View 装配、shortcut／focus／navigation；更新相关 tests-cones 和下载 UI | 定位／弹层／焦点／滚动／编辑组合通过；虚拟列表不提前构建离屏项；关闭、缓存与停帧合同通过 |
| D4 通用内容构造 | 依赖 D0a；按 JSON、URL、Path 分成独立改动，迁移各自一次性构造调用 | 独立快照、重复项／编码、异常和 Path 清理验证通过 |
| D5 HTTP 装配 | 依赖 D4 的 JSON／URL 入口；先字段收集，再 client 配置／request／multipart，再 server routing／options／result 及附属配置；同步迁移下载示例网络层 | client/server 普通测试和 loopback 组合通过；构造没有发送／消费 body；middleware、重试／重放、取消与资源收尾正确 |
| D6 其他应用配置 | 依赖 D0a；按 window、readline、platform.process 独立迁移，对已迁移 UI／HTTP 调用方作必要更新 | 创建配置快照、平台选项、Readline 配置、进程参数／环境与真实执行测试通过 |
| D7 统一收尾 | 依赖 D1～D6；扫除遗漏的调用和说明，更新公共阅读材料及 API 文档，执行受影响范围的组合回归 | 被替换的旧签名和兼容转发为零；新入口覆盖完整；示例可构建运行；简洁记录验证结果与实际平台范围 |

D4、D6 的代码工作不依赖 UI 实现完成，可以在主线批次之间安排。每批必须保持仓库中相应消费者可编译，不交付“新入口已添加、旧调用以后再迁”的中间终态。

正式规范按归属更新：UI、UI-COMPONENT、UI-STYLE、UI-THEME、GRAPHICS-TEXT、SURFACE、HTTP、HTTP-MESSAGE、HTTP-MULTIPART、HTTP-CLIENT、HTTP-SERVER、JSON、URL、WINDOW、READLINE、PLATFORM。语言规范先在 D0a 更新 receiver 查找与捕获边界；Path 更新 graphics 的现有设计与公共文档。相关附属协议规范在对应 options 变更时同步更新。不改三份基础规范中无关的 ABI／阶段合同。

## 6 调用方迁移范围

除各库 `src` 外，每批搜索并更新以下实际消费面：

- 所属 Cone 的 tests、consumer-tests、tests-cones、examples，以及其他系统库的直接调用。测试中专门验证底层对象／算法的代码按其测试职责使用新设计中的底层接口，不为追求 DSL 外观改写被测算法。
- UI 重点包括 `scoop.ui.component/examples/themes`、`examples/download`、`scoop.ui/examples/custom`，以及 style 的 themes／ui 测试 Cone 和 component 的 interaction／editing／virtual 等组合 Cone。
- 三套主题 `scoop.ui.style.macos`、`scoop.ui.style.material`、`scoop.ui.style.fluent` 的源码和测试；surface／graphics／window 示例中的 Path 与创建配置。
- `sysroot/share/scoop/examples/programs`、`topics` 和目录索引中的相关内容，库 README、正式 spec 和当前使用指南。历史实施记录只在其仍充当当前使用说明时更新代码或链接，不重写过去的验证事实。
- compiler／tooling 的内嵌 Scoop 样例和生成模板中对被替换 API 的引用。只修实际调用，不增加编译器专用 builder 机制。
- 受影响的 `tests/fixtures` 先逐项评估：普通库行为迁到所属 Cone 或独立测试 Cone 的 scoop.test；混合用例先拆出普通行为。只有编译诊断、产物／链接／ABI、CLI 或必须由外部进程控制的行为确实无法由 scoop.test 验证时，才保留并更新必要的 fixture。不能仅为通过原有 fixture 直接改写其 API 调用。

D0 清单记录旧声明到新入口的对应关系，实施时用 `rg` 结合实际编译核对遗漏。已有类型名可能仍用于新结果或增量对象，因此完成与否以旧签名、旧语义入口和真实调用为准，不把同名文本出现次数当作验收。

## 7 验证计划

### 7.1 普通行为与组合

| 测试位置 | 核心断言 |
| --- | --- |
| 各 builder 所属 Cone | 空／默认配置，必需参数，构造回调执行一次，条件／循环顺序，嵌套配置、多次配置和显式替换，异常传播，发布后的结构快照 |
| component 普通测试及实际窗口 tests-cones | 自动追加一次、继承成员解析、第三方 add、单内容槽；DSL 与程序化入口使用相同的组件／Props 模型；稳定 key 的增删／重排、State／Binding／编辑模型、替换后的事件回调 |
| style／graphics.text | Inherit／Set(None)／Set(value)，SpanStyle 的嵌套 Option，空配置不覆盖主题，有序效果及文本，行内 key 与 View 对应、字体与 painter 归属 |
| UI 现有专项组合 | Absolute 排序、Portal 关闭／焦点恢复、嵌套滚动残量、VirtualList 的 viewport／overscan／retention／navigation、编辑会话、独立绘制缓存、卸载后资源释放与静止停帧 |
| JSON／URL | JSON 重复键与顺序、数字和 null；URL raw／encoded、空／缺失与重复参数；独立快照 |
| HTTP tests／consumer-tests | 字段顺序与重复项，配置默认／覆盖，路由匹配和 middleware 顺序，冻结；body 未提前消费、可重放与一次性发送、multipart、loopback 请求／响应和取消清理 |
| graphics／window／readline／platform | PathBuilder 成功／异常清理和结果所有权，创建配置快照及平台选项，Readline 行为，Command 参数／环境／stdio 与显式 spawn |

所有 Cone 的 API 改动只用 scoop.test 验证，测试放在所属 Cone 或实际需要的独立测试 Cone，不为库 API 迁移新增或直接修改 fixture。受影响的既有 fixture 必须先按第 6 节评估并迁移普通行为；确有必要保留时才更新 fixture，并记录保留理由。缺少必需参数、错误 receiver 类型等已有语言规则不按控件重复建立负例。

URL 迁移涉及的九组 HTTP fixture 及共享 helper 的一个调用组已按实际 peer 与测试内容评估，迁移边界如下：

| 受影响组 | 处理与保留理由 |
| --- | --- |
| client-basic、client-bearer | 将策略、凭据、选择、缓存／刷新、取消／清理及 HTTP/2 的进程内 loopback 用例迁入 scoop.http.client 的 scoop.test，保留原有线程组合与分支覆盖。fixture 仅保留独立 Python HTTP/1 peer 对认证挑战、100-continue／压缩与 trailers、跨 origin 重定向字节及连接次数的断言；共享实际使用的测试 helper 源码。 |
| client-bearer-lifetime | Python 只启动普通行为程序，没有独立 peer。共享失败、共享清理两个用例迁入同一 Cone；其余重复的取消用例由上述测试覆盖，删除这组 fixture。 |
| client-observer、client-interceptors | 保留 Python peer 对实际请求、签名字节、重试／缓存请求次数、截断响应以及未发布响应／WebSocket 连接关闭的断言。普通 observer 行为继续由所属 Cone 已有测试验证。 |
| client-observer-secure | 保留独立 TLS／HTTP2／代理 peer 对 ALPN、握手失败、隧道与连接复用次数的断言。 |
| client-websocket、client-websocket-secure | 保留独立 WebSocket peer 的握手随机 key、非法响应、帧与压缩字节，以及 TLS／HTTP2 并存、ALPN 和客户端证书互操作验证。 |
| client-websocket-proxy、client-websocket-auth | 保留 CONNECT／SOCKS5／TLS 嵌套协议、认证字段隔离及独立 Digest 校验；这些断言发生在外部 peer 观察到的真实传输上。 |

先完成上述普通行为的测试归位，再更新 URL 调用。保留 fixture 只在这些外部协议合同范围复验，不用它们代替新 URL API 的 scoop.test。

成员／顶层工厂与 alias 的组合放在真实跨 Cone 的 scoop.test 消费测试中。D0a 是编译器语言规则变更，其拒绝编译与诊断位置无法由 scoop.test 断言，因此保留代表性 expected-error fixture，覆盖外层 receiver 的隐式读取、赋值／复合赋值、方法／扩展／函数属性调用和必要的实例绑定；同类型不同 receiver、普通／suspend receiver、嵌套普通回调及泛型／跨 Cone 消费按实际语言规则覆盖。显式 `this@label`、对象别名、普通词法捕获和已有函数值适配为正例。

原 nested、lookup、initializers 三组 receiver fixture 只断言运行结果，已将其嵌套接收者、成员优先级、扩展属性与初始化行为迁入 [receivers 测试 Cone](../../tests/cones/receivers/README.md)，并增加跨 Cone 与函数值适配覆盖。隐式外层使用的编译错误和诊断位置保留在 [implicit-outer fixture](../../tests/fixtures/language/receivers/implicit-outer/program.scoop)。HIR 与编辑查询的定向回归验证候选探测不误报、选中外层成员不能回退改绑及两类入口一致；不修改捕获或 runtime 协议。后续 `@DslMarker` 不重复这些已覆盖的检查。

### 7.2 执行方式

1. 每批 Scoop 变更先对受影响 Cone 执行 `scoop fmt`。Rust／Python 若实际改动，先按 AGENTS.md 执行各自格式化和 lint，再运行相关验证。
2. 内置库变化后，按[准备脚本说明](../../scripts/README.md)显式更新开发 sysroot，设置 `SCOOP_SYSROOT`，复用已有 native 资源和共享 Cone cache。准备与依赖编译使用正确的依赖顺序，测试不逐项复制 sysroot 或重建未变化的库。
3. 对实际改变构造／配置行为的 Cone 执行 `scoop test --release`，覆盖新 API 的领域规则和必要组合。仅机械更新调用方式、运行逻辑未变的消费 Cone，使用 `scoop test --release --no-run` 检查生产与测试源码、类型及链接；不为这些调用迁移扩跑无关行为。测试消费本批更新后的依赖产物，不再为 Cone API 另设 fixture 验证入口。
4. 未修改 LIR／codegen 的批次只做本地验证。UI、主题、Path 和相关图形组合在已支持的 macOS aarch64 release 上通过 scoop.test 分批验证；实际修改 LIR／codegen 时再按变更所影响的目标补充必要验证，不把本机结果宣称为全部平台结果。
5. 只对本批实际影响的行为选择 scoop.test 定向组合，例如主题覆盖、下载请求装配或 HTTP loopback；不因改写构造调用排查无关的窗口激活、系统焦点等环境问题。fixture 先评估迁移，仅在确需保留的编译器／外部进程合同范围运行；不因新增配置块重跑编译器全量、全部系统库或普通功能的 GC stress 矩阵。
6. 用新 API 生成并检查相关公共 API 文档；当前文档、可运行示例和库实现必须一致。原始日志、截图、JSON 报告置于仓库外，仓库只保存批次范围、结果、失败闭合和必要平台限制。

## 8 完成条件

- 第 2／3 节纳入的 API 全部完成新设计，现有能力在新入口中可表达，必需数据与所有权清晰。
- receiver function 中隐式使用外层 `this` 已作为编译错误落实；显式 receiver 与普通词法捕获可用，API 示例和消费者不依赖被禁止的隐式绑定。
- 被替换的旧签名、兼容别名和双轨实现全部删除，全部实际调用方、示例、测试和当前规范同步更新。
- UI 使用同一 Component／Element／Props／View、状态观察和挂载流程；惰性列表、输入、焦点、资源和停帧通过必要组合验证。
- HTTP、JSON、URL、Path 和应用配置保持各自真实领域规则，配置构造与执行动作的时点明确；StringBuilder 和 ByteArrayBuilder 未改动。
- 受影响的预编译 Cone 和消费者可完成构建、链接与运行；验收记录只声明实际完成的测试与平台范围。

## 9 实施记录

| 日期 | 批次 | 完成内容与验证 |
| --- | --- | --- |
| 2026-11-03 | D0 公共约定 | 接收者构造入口、必需参数、快照、覆盖、子项与显式外层引用已写入 API 风格规范；StringBuilder／ByteArrayBuilder 排除，Cone 验证和 fixture 先评估规则已同步。各领域完整签名随对应批次先行落实；此批只改文档。 |
| 2026-11-03 | D0a 外层 receiver | 语言规范、HIR 实际绑定和编辑查询统一禁止隐式外层 receiver；显式捕获与函数值适配保留。三组运行行为 fixture 迁入 receivers 测试 Cone；编译诊断保留必要 fixture。Rust 格式化与 workspace clippy 通过，HIR 全量 2,133 项及后补定向组 13 项通过；本机 aarch64 macOS release 的 7 项 scoop.test、47 个 receiver fixture 通过。合并 origin/main 的 target/runtime 重构后，重建配套工具、基础库与 runtime，Clippy 和同组 13／7／47 项增量复验通过。本任务未改 LIR／codegen。 |
| 2026-11-03 | D4 JSON | jsonObject／jsonArray、Unit add、内部构造与快照已替换公开链式 builder；实现、测试、示例与当前文档同步更新。无受影响 fixture。本机 aarch64 macOS release 的 JSON 整库 105 项 scoop.test 通过；预编译库的 2 项临时消费测试覆盖默认配置与 value_tree 示例，公开 API 文档生成和签名检查通过。 |
| 2026-11-03 | D4 URL 前置测试归位 | 评估九组直接受影响的 HTTP fixture 及一个共享源码调用组。Basic／Bearer 的普通认证行为与 lifetime 独有用例迁入 scoop.http.client，19 个 scoop.test 入口覆盖 79 个场景并保留双线程组合；先验 17 项、再验新增 2 项，全部通过。删除 client-bearer-lifetime；保留的 Basic／Bearer HTTP/1 peer fixture 共 2 项通过。Scoop 格式化与 Python Ruff 检查通过，正式 API 尚未在此批修改。已用当前配套工具完整准备本机 aarch64 macOS 开发 sysroot。 |
| 2026-11-03 | D4 URL | url／Url.copy、可写 encoded 组件与 Unit 原文／追加操作替换公开链式 builder；规范、HTTP／client／charset 调用方、示例和文档同步更新。构造和快照内部化，复制复用已有 authority 解释，raw／encoded 与空／缺失区别保留。本机 aarch64 macOS release 的 scoop.test：URL 整库 22 项、HTTP 整库 303 项、charset 整库 18 项、client 定向 10 项、预编译库实际示例与导入 2 项均通过；已评估保留的外部协议 fixture 7 项通过。Scoop 格式化、公共 API 文档与签名检查通过，开发 sysroot 按依赖顺序更新；本任务未改 LIR／codegen。 |
| 2026-11-03 | D4 Path | path 作用域入口自动关闭临时 builder，追加／reset 改为 Unit；增量 PathBuilder 继续按明确生命周期产生独立快照，reset 保留填充规则。规范、graphics／Surface／style／UI 消费方与示例同步更新，无受影响 fixture。本机 aarch64 macOS release 的 scoop.test：graphics 39 项、Surface 79 项、基础示例 4 项、UI 自绘 1 项、style 11 项、三套主题 13 项和临时消费示例 2 项，共 149 项全部通过。Scoop 格式与公共 API 文档检查通过；按现有脚本更新 Skia 的 PathOps 资源及开发 sysroot，链接失败已闭合。本任务未改 LIR／codegen。 |
| 2026-11-03 | D1 纯文本与文本样式 | styledText／textSpan、TextStyle／SpanStyle／ParagraphStyle 接收者工厂已替换公开长参数构造和 TextSpan companion 工厂；追加成员返回 Unit，重复样式块基于当前值修改，显式赋值替换。规范、文本／UI／控件／主题消费方及示例同步更新，无受影响 fixture。文本整库 51 项（含新增 5 项构造语义）、临时实际文档消费 3 项及已执行的调用组合共 177 项 scoop.test 通过；布局窗口测试因未获系统焦点按用户要求跳过。绘制缓存、主题示例和下载示例的调用通过 scoop test --no-run 编译／链接检查；迁移中的同名帮助函数和隐式外层引用错误已修正。Scoop 格式、公共 API 文档及开发 sysroot 更新完成。公共 Style、主题配置与 UI 行内 View 装配继续按后续批次实施；StringBuilder／ByteArrayBuilder 未改动，本任务未改 LIR／codegen。 |
| 2026-11-03 | D1 公共样式与部件选项 | Style／StyleOverride、Surface／Checkbox／Radio／Scrollbar Options 改为接收者工厂和 copy；构造器内部化，withBackground／withBorder 删除。嵌套 style 基于当前覆盖修改，类型化槽保留继承／清除／设置，数组保持顺序及独立快照。规范、控件调用与示例同步更新，无受影响 fixture。本机 aarch64 macOS release 的样式 16 项（含新增 5 项）、主题几何 4 项、控件样式 5 项，共 25 项 scoop.test 通过；样式 UI／错误收尾、控件布局／交互、主题／下载示例共 6 个消费 Cone 的 scoop test --no-run 编译／链接通过，未运行窗口测试。Scoop 格式、公共 API 文档与开发 sysroot 更新完成。控件样式、主题参数及部件外观继续推进；StringBuilder／ByteArrayBuilder 和 LIR／codegen 未改动。 |
| 2026-11-03 | D1 控件局部样式 | ButtonStyle／CheckboxStyle／TextFieldStyle／InteractionStyles 改为接收者工厂和 copy，公开长参数构造删除；surface／marker／states／文字覆盖／scrollbar 的重复块保留当前值，完整赋值替换，动画槽保留继承／禁用／指定值。SpanStyle.copy 支持跨 Cone 的嵌套修改。规范、控件调用、主题展示与交互消费方同步更新，无受影响 fixture。本机 aarch64 macOS release 的文本构造 5 项、控件样式 9 项（含新增 4 项），共 14 项 scoop.test 通过；主题展示与控件交互两个消费 Cone 的 scoop test --no-run 编译／链接通过。Scoop 格式、公共 API 文档与开发 sysroot 更新完成，未运行窗口测试。后续继续主题参数与部件外观；StringBuilder／ByteArrayBuilder 和 LIR／codegen 未改动。 |
| 2026-11-03 | D1 完整主题配置 | ShapeTheme、主题参数和部件外观统一使用完整必需数据、接收者工厂与 copy；三套预设接受 ShapeThemeBuilder 并迁移其生产代码。嵌套配置保留当前字段、完整赋值替换，配色修改不隐式重算其他描述。ThemePalette 保留定义十二个必需语义色的完整值构造并提供 copy；TextStyle.copy 支持跨 Cone 字体配置。无受影响 fixture。本机 aarch64 macOS release 的文本构造 5 项、主题配置 4 项、三套主题六种模式的几何 4 项，共 13 项 scoop.test 通过；公共样式与三套预设 API 文档和签名检查通过，README 的目录链接已修正。Scoop 格式与开发 sysroot 更新完成。D1 文本和样式基础全部完成，后续进入 UI 声明与组合；StringBuilder／ByteArrayBuilder 和 LIR／codegen 未改动。 |
