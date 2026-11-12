# Scoop 库 API 风格

本文规定 core、std 和各官方库的公共 API 约定。语言语义与各领域合同仍见 [语言规范](SCOOP-SPEC.md)、[标准基础库规范](SCOOP-STDLIB-SPEC.md)、[平台库规范](SCOOP-PLATFORM-SPEC.md)及对应库规范。本约定用于设计和评审，不新增编译器命名检查或产物验证阶段。

日常调用词汇以 Kotlin 为主，类型与能力边界遵守 Scoop 自身规则。名称应让调用者判断操作、返回形状和必要的所有权责任。

| 项目 | 约定与例子 |
| --- | --- |
| 函数、属性、参数 | lowerCamelCase：currentSourceLocation、bufferSize、isClosed |
| 类型、普通 enum variant | UpperCamelCase，缩写按单词处理：Json、Cli、Tls、Uuid、Ip、Os、Io；ByteOrder.BigEndian、Month.January、AnnotationTarget.Function |
| 常量与配置实例 | 常量保留 Duration.ZERO、Uuid.NIL、Int.MAX_VALUE；具名配置实例使用 Base64.Default／UrlSafe |
| 外部标准名称 | JwsAlgorithm 的 HS256／RS256／PS256／ES256／EdDSA 和 Architecture.X86_64 保留公认拼写；其他普通 variant 不因此改成全大写 |
| 工厂 | 普通值构造用构造器；输入含义需要区分时用 fromNumber、fromIsoDayNumber、fromByteArray；资源动作用 open、spawn、generateRsa 等具体动词 |
| 参数 | 参数名属于源码 API；同义 override 与同步／异步入口保持名称和次序，主要操作回调放最后。流传输用 buffer／count，复制用 destination／bufferSize，数组区间用 offset／count |
| 属性与函数 | 对象已有的廉价状态用属性；外部查询、IO、资源分配、生命周期动作使用函数。File.size() 查询 OS，FileMetadata.size 是已取得的值；异步 socket 的地址属性保存创建时快照 |

命名参数由静态声明决定。领域术语可以不同：Map.get(key) 使用键的含义，IndexGet.get(index) 表达通用下标；通过对应接口调用时使用该接口的参数名。索引能力使用 IndexGet／IndexSet／IndexGet2／IndexSet2，集合接口使用 Set<T>，不靠类型参数数量消歧。

| 返回值与操作 | 约定 |
| --- | --- |
| 失败 | 普通解析或转换入口抛具体异常；OrNone 只把约定的正常失败变成 None；谓词返回 Boolean；需要失败载荷时使用 Result<T, E>。无关异常照常传播 |
| 数组与字节 | 高层只读输入 List<Byte>，独立快照输出 Array<Byte>，可写目标 MutableArray<Byte>；FFI 指针继续按实际 native 元素类型表达 |
| 转换与适配 | toX 创建与源后续结构修改无关的结果；toMutableX 明确可写；asX 表示适配或 view。容器快照是浅复制，引用元素仍引用原对象 |
| 集合求值 | 普通集合转换默认急切；Sequence 惰性。Iterable.withIndex() 是明确的惰性例外，需快照时调用 toList()；有序容器 copyOfRange 返回独立快照 |
| 修改与构建 | sort／sorted、shuffle／shuffled 区分就地与新结果；集合用 add，文本和字节 builder 用可链式 append，IO 用 read／write |
| 整数 | 普通转换对应 Byte／Short／Int／Long 和无符号类型名；二进制字段使用 read/writeInt8／Int16／Int32／Int64 及 UInt8／UInt16／UInt32／UInt64；索引、计数沿用 Long |
| Codec | ValueEncoder／ValueDecoder／ValueInspector 表达处理 T 的独立对象；Json 的 encodeToString／decodeFromString、encodeToValue／decodeFromValue 明示边界表示 |
| 资源 | 可关闭资源实现 Closeable；dispose 默认桥接 close。暴露关闭状态时使用 isClosed。一般清理能力仍为 Disposable；use 同时保存正文与清理异常 |
| 所有权 | 资源回调形式 withX { ... } 表示作用域内借用；takeX 取走，takeOwnership 接管。具体入口说明移交时点与失败责任；finish／shutdownGracefully 与本地 close 各表达实际动作 |

缺失值的命名使用 Scoop 的 `None`：可选结果的配对入口使用 `OrNone`，例如 `toIntOrNone`、`parseOrNone`、`getOrNone`；过滤可选值使用 `mapNotNone`、`filterNotNone`，只去掉外层 None 并提取 Some 的载荷，保留载荷中的 None。Map.get、Iterator.next、find 等已有明确缺失语义的查询无需统一追加后缀。JSON 格式的 null 与 FFI 的空指针按各自领域术语表达，不属于 Option API 的命名。

withPadding 是不可变配置派生，withIndex 是惰性迭代变换，二者均不属于资源回调形式。计时单位保留 `5.seconds()` 等受检函数；它们可能溢出，不能把它们当作免费读取的字段。Duration.floorSeconds 是内部规范化表示的整秒下界，inWholeSeconds 和 toComponents() 的整秒向零截断，分量余数与时长同号。

文件系统操作以 Path 扩展为入口，例如 `path.readText()`、`source.copyTo(destination)`、`path.metadata()`、`link.createSymbolicLink(target)`。File.open(path, options) 创建句柄；createTempFile／createTempDirectory 是顶层工厂。跨库 native 支持入口放在相应 native package，普通配置类型不承诺 native 位布局。

从 Rust、Swift 或 Kotlin 移植时，以下差异是 Scoop 的明确选择。

| 主题 | Scoop 的使用规则与差异 |
| --- | --- |
| 字符串 | String.charLength 计 Unicode 标量且为 O(n)，byteLength 计 UTF-8 字节且为 O(1)，getCharAt 定位为 O(n)。文本 `e` + U+0301 + `😀` 的计数分别是 Scoop charLength 3、Rust str.len() 7 字节、Swift String.count 2 字素簇、Kotlin String.length 4 个 UTF-16 code unit。顺序处理字符优先迭代；上述计数都不直接代表终端列宽 |
| 迭代终止 | Iterator.next(): Option<T>，无需 Kotlin 风格的 hasNext()；返回 None 表示结束 |
| 嵌套可选值 | T? 是 Option<T>，Some(None) 与 None 不同；mapNotNone 只去掉一层。Map.getOrPut 只把外层 None 视为缺少键，已有 Some(None) 不触发缺省值生成 |
| Result | Result<T, E> 使用 Success／Failure，E 无异常上界；mapError、flatMap 与 getOrThrow 各按明确的载荷／异常约束使用 |
| 数组与能力 | Array 只读、MutableArray 可写；Any 没有通用 equals／hashCode／toString 回退。普通 struct 可派生 ==，但不自动获得 Equals<T> conformance；assertEquals 按实际类型选择名义 Equals<T> 或公开 equals(T): Boolean 的成员要求入口，Option、struct、enum、tuple 可使用其派生相等能力。数组内容比较由实际元素规则决定 |
| 任务执行 | scope 有 executor 时 launch／async 排队，无 executor 的低层 scope 才允许同步启动；普通 binding 取调用处快照，executor 取 scope 默认值或显式参数。blocking 接受普通托管闭包；withExecutor 建立结构化子作用域。CancellableContinuation.tryResume 返回 Boolean，一步选择并提交恢复，不使用 Kotlin 的 tryResume／completeResume 两阶段协议 |
| CLI | parse(arguments) 不含程序名，parseCommandLine(argv) 包含程序名；CliResult.Parsed／HelpRequested 表达正常控制流程 |

每个公共入口的领域文档应交代返回形状、求值方式、复制或借用、正常失败以及有实际影响的复杂度。普通 enum 源名字、反射／debug 名字与默认派生编码名字一起变化；显式协议格式独立规定。API 迁移必须同时修改规范、实现与实际消费者，验收使用真实编译诊断和运行结果。

## 接收者构造 DSL

需要组合内容、收集条目或分组配置的入口使用普通接收者函数类型：`name(requiredArguments..., configure: Builder.() -> Unit)`。没有额外配置也能形成完整结果时，configure 默认空块。构造块同步执行一次，结束时生成完整描述或配置；发送请求、创建窗口、启动进程和挂载 UI 等操作由明确动作入口执行。持久增量对象、简单值构造和普通事件回调按实际用途设计，不统一改成配置块。

必需模型、Binding、ItemSource、单内容 View、没有有效默认值的内容或动作放在前置参数。默认值必须来自领域合同，不能在 builder 中存放未初始化字段、None 或空回调，再以结束检查代替必需参数。必需事件动作前置后，最后参数留给 configure；其余可选回调作为配置属性。

普通配置使用可写属性，嵌套配置使用真实成员方法，序列使用明确的追加操作。标量后写覆盖先写，序列按执行顺序追加。重复的嵌套配置块在当前配置上修改，空块不重置先前设置；显式赋入完整配置值表示替换。领域中的继承、清除和设置必须分别表达，不能用同一个 None 同时表示继承与清除。

发布的结果与 builder 后续结构修改独立；按领域需要复制可变序列和嵌套配置，不可变子值可以共享。状态模型、回调和资源引用按实际所有权保存。构造失败向调用方传播异常，不发布部分结果；需要关闭的临时资源在作用域退出时清理。修改已经结束的 builder 不代表更新应用状态。

UI 多子项 builder 通过 ChildrenBuilder 的真实成员收集标准子项。同名顶层工厂返回 View，成员入口追加一次并返回 Unit；两者共享同一私有构建实现。已有 View 和第三方组件使用 add。单内容入口显式接收一个 View，不隐式合并多个子项。需要在收集块中取得独立 View 时使用提前构造的值或顶层工厂 import alias，不依赖返回类型选择同名入口。

配置代码只隐式使用当前 receiver；访问外层组件或 builder 时使用 `this@label` 或不可变对象引用。普通词法捕获继续按语言规则使用；receiver 查找边界由语言规范 8.1.2 统一规定，不依赖 `@DslMarker`。

大型 UI render 优先按真实表单、工具栏或列表行提取普通 View 函数，前置参数接收显示快照、稳定模型／Binding 与动作，builder 只配置当前对象和收集子项。显示值来自当前 render；事件按执行时的模型或 Props 操作，不用旧显示快照计算下一状态。普通函数不增加挂载层，保留显式 key、子项次序及编辑模型。小型 render 可先准备局部 val 与动作，不为每个叶子创建额外组件或 ViewModel。

默认文字由 ComponentScope.defaultFonts 解析，示例仅在确有覆盖需求时传字体。使用现有 style／focus 等嵌套成员配置，减少多余的 Some(builder { ... }) 包装；省略、清除与显式赋值仍保留领域区别。容器布局参数优先写在子项之前。内容槽使用组件已提供的具名显示输入，例如 ActionContentStyle 或 SidebarPresentation，不注入整个外层 owner，也不放宽 D0a。

配置回调通常是普通函数类型；真正允许挂起的任务块在 API 签名中明确使用 `suspend` 函数类型。调用处的 `{ ... }` 按期望类型确定挂起性，无需重复关键字。已定型的普通函数值不自动变成挂起函数值；需要适配时显式包装调用。普通/挂起回调重载遵守语言规范 8.6 的对称性与逐位置优先规则，不能依赖返回类型或声明顺序消歧。

每项迁移先在所属领域规范写全入口签名、必需参数、可配置字段、默认值、返回值、快照／所有权及原入口的替换关系，再同步修改实现与全部实际消费者。被替换的旧签名直接删除，不设置历史兼容别名、转发层或双轨实现；仍有独立用途的结果值、Props 和持久增量对象按新设计的实际需要公开。各库直接实现自己的 builder，只共享实际相同的值与必要 helper，不建立跨库构造框架。
