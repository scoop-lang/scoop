# 用 `@GenerateWith` 生成 codec 和命令行参数解析器

`scoop.codec` 通过 `@GenerateWith` 生成 `ValueEncoder.encode` 和 `ValueDecoder.decode`，取代 HIR 中这两个方法的专用合成。`scoop.cli` 用独立的 `CliArguments<T>` 绑定整组参数，用 `CliValueParser<T>` 解析自定义字段。编译器在每个接口实现自己的 `where` 条件下检查生成的方法体。

两个生成器构造模型时都使用普通具名调用。生成方法所在的作用域必须只能看到目标类型的主构造器。模型需要其他构造入口时，在 `companion object` 上定义具名工厂。需要在解码或绑定过程中调用工厂时，只手写对应方法即可。详见[构造示例](#4-自动构造的约定与-companion-工厂)和 [M47 总体设计](DESIGN.md)。

## 1. 应用如何使用

`@GenerateWith` 标在实际实现接口的 companion 或独立类型上。`scoop.codec` 为显式选择的编码、解码接口生成缺少的方法。`scoop.cli` 为命令行参数类型生成参数表和构造方法。应用可以只选其中之一，也可以让同一个 companion 同时实现两组接口。

```scoop
import scoop.cli.*

public struct Options(
    @CliArgument(position = 0L) val input: String,
    @CliOption(names = ["--port", "-p"]) val port: Int = 8080,
    @CliFlag(names = ["--verbose", "-v"]) val verbose: Boolean = false
) {
    @GenerateWith("scoop.cli", ["CliArguments<Options>"])
    public companion object : CliArguments<Options> {}
}

fun parseOptions(argv: Array<String>): CliResult<Options> =
    CliParser(CliConfig("serve"), Options.Companion).parseCommandLine(argv)
```

这里 `Options` 只声明命令行能力。一个还需要 JSON 的类型可在同一 companion 上另写 `@GenerateWith("scoop.codec", ["ValueEncoder<Options>", "ValueDecoder<Options>"])`，并在继承列表加入相应接口。`Json.encodeToString(value)`、`Json.decodeFromString<Options>(text)` 以及显式传入 codec 的重载沿用现有调用方式。手写方法、继承方法与 interface default 先按普通规则选择，一个方向已有实现时只生成另一个方向。

## 2. 让 `@GenerateWith` 选择条件接口

```scoop
public struct Envelope<T>(val value: T) {
    @GenerateWith("scoop.codec", ["ValueEncoder<Envelope<T>>", "ValueDecoder<Envelope<T>>"])
    public companion object {
        override interface ValueEncoder<Envelope<T>> where T.Companion : ValueEncoder<T> {}
        override interface ValueDecoder<Envelope<T>> where T.Companion : ValueDecoder<T> {}
    }
}
```

两个方法分别使用各自的 `where`。元素类型只有 encoder 时，`Envelope<T>` 可以编码。请求解码的调用处需要独立证明 decoder 条件。条件写在显式接口实现块，注解中只写要选的完整接口。相同规则适用于用户接口和 CLI 接口。

迁移基线（`1681cdf1d`）中的 driver 用无附加条件的 owner 查询可用接口，上例在选择阶段就会报 `E1317`。HIR 的继承检查已记录每个实现对应的接口、成立条件和方法。driver 和正文检查按以下规则使用这些关系。[任务选择](../../compiler/driver/src/generation/plan.rs) · [条件实现关系](../../compiler/hir-lower/src/class/inheritance/interfaces/conditional.rs)

1. 选择配置时枚举 owner 已检查的实现关系，含父接口和沿基类实参代换后的条件。对每个选择器，按注解位置的 import、alias、类型参数和候选关系自身的假设绑定普通类型片段，用规范化后的完整接口身份匹配。这样 `I<T.Companion>` 或带 bound 的 `I<T>` 也能在其合法的块内解析。其他块的假设不得用于证明此候选。普通代码查询“当前能调用哪些接口”继续使用调用处的条件。
2. 用匹配关系和原 requirement 定位缺正文的 `MethodId`。同一完整接口若有多个已检查且互不相交的实现域，选择器覆盖这些域中对应的方法，各方法按自身条件检查。已经手写或继承的接口仍参与配置检查，零缺失方法时无需启动进程。重复配置同一完整接口或两个不同生成器认领同一缺失方法时，沿用现有诊断。同一生成器覆盖同一方法只发一份任务。重叠实现和冲突 default 交给常规继承检查。
3. 语义查询与返回正文的 lowering 使用同一个方法定义环境，含其实际适用条件和基类代换。例如从 `Base<Wrapper<T>>` 继承的条件要在 `Wrapper<T>.Companion` 上证明。每个方法只在定义处检查一次并导出正常 HIR。应用方在满足条件的具体类型上使用该方法。

## 3. `scoop.codec` 生成哪些代码

宿主程序 `scoop-codec-generator` 接收已确定的 requirement 和方法签名，查询目标、原声明及应用后的字段类型、构造参数映射、注解和默认值，返回 `MethodId` 对应的一段 Scoop 方法体。公共协议已有 `describeTypes`、`describeMembers`、`describeAnnotations`、`describeDefaults`、`typeRelation` 和 `lookupExport`。编译器照常解析正文，并在实现者的访问域内检查类型、调用及效果。[同步协议](PROTOCOL.md) · [成员与构造参数查询](MEMBERS-JSON.md)

| 事项 | 生成行为 |
| --- | --- |
| 字段策略 | 分方向选择完整的 `ValueEncoder<F>` 或 `ValueDecoder<F>`。优先取 codec 主构造器唯一匹配的 `val`，多个匹配报歧义。其次是递归字段的 `this`。然后为 core Option、Array、MutableArray、ArrayList、tuple 组合元素策略，Unit 取普通 helper。最后才取已证明满足接口的可见 companion，含条件块中的 `T.Companion`。显式注入的完整容器 codec 优先。 |
| 数据外形 | struct 和合格 final class 生成 keyed record。编码、解码分别检查其可读存储、主构造参数及访问域。普通 enum 保留单 key 外层和 unit／命名／位置 payload 的现有形状，tuple 按固定长度处理。core 标量、Option 和集合本身已有的手写实现继续作为普通库方法调用。 |
| `@Serialize` | 生成器按实际 core 注解的身份解释 `rename`、`ignore`、`required`、`emitNull`，检查方向所需的形状、冲突和重复 wire 名。策略错误定位到对应字段、variant 或注解实参。编译器仍承担普通注解的 target、参数绑定、类型和重复应用检查。仅有合法绑定的 `@Serialize` 且没有请求生成时，不单独触发 codec 策略诊断。 |
| `encode` | 跳过 `ignore` 后，按声明序读取各参与字段，每个字段一次。仅当原声明外层是 core Option、`required` 和 `emitNull` 都为 false，且没有默认值或默认值直接为核心 `None` 时，才省略值为 `None` 的字段。默认表达式为函数调用或 `Some(...)` 时仍写出。决定写入后才取得 child、求值所选字段 codec 并调用。 |
| `decode` | 按构造参数序读取已提供值，显式 `null` 交给完整字段 codec。先完成 record／sequence 的 `end` 检查，enum 先检查所选 payload 再检查外层，随后补默认值并构造。`required` 要求输入中有 key。允许缺失或标为 `ignore` 的字段优先用声明默认值，其次仅对声明处 Option 补 `None`。`T` 后来取 Option 仍遵守原声明的必填规则。 |

生成器按声明身份识别接口、注解和 helper。codec 从 core 查公开声明，CLI 从所选 `CliArguments` 的声明来源取得 CLI Cone，再按其公开路径做 `lookupExport` 并核对声明身份。`describeTypes` 的 `sourcePath` 提供源码名称段，`visibleCompanion` 给出当前方法可访问的伴生类型，class 的 `modality` 和 `baseClass` 用于判断支持的形状。生成正文使用方法作用域内合法的类型名、限定名或局部 import alias，正常转义标识符和 wire 字符串。用重名、别名和跨 Cone 正文验证查询结果。[类型与成员查询](MEMBERS-JSON.md)

切换时为实际依赖自动实现的 companion 和独立 codec 补注解，同时删除 HIR 中 `encode`／`decode` 专用合成、正文队列和 `Serialize` 策略检查。缺少注解的空实现走普通缺失方法诊断。`ValueInspector` 有自己的合成用途，共享的字段查询按其实际调用保留。[现有实现选择](../../compiler/hir-lower/src/class/inheritance/interfaces.rs) · [现有正文入口](../../compiler/hir-lower/src/pipeline/source_bodies.rs) · [字段语义基线](../milestone46/SERIALIZE.md)

## 4. 自动构造的约定与 companion 工厂

`scoop.codec` 生成 `decode`、`scoop.cli` 生成 `bind` 时，对需要构造的 struct 或 final class 遵循同一条构造器约定。**从生成方法的作用域看，目标类型唯一可见的构造器必须是主构造器**。输入项按已有成员映射对应具名形参，生成器用普通的 `类型名(形参名 = 值)` 调用。其他构造需求放在 `companion object` 的具名工厂方法中，由调用方明确选择。

```scoop
public fun defaultBias(id: Int): Int = id + 2

public struct Row(val id: Int, val bias: Int = defaultBias(id)) {
    @GenerateWith("scoop.codec", ["ValueDecoder<Row>"])
    public companion object : ValueDecoder<Row> {
        public fun fromLegacyId(id: Int): Row = Row(id = id, bias = 99)
    }
}
```

原先的次构造器 `constructor(id: Int) : this(id, 99) {}` 会截获省略 `bias` 的调用。把它改为 `fromLegacyId` 后，`Row(id = 5)` 得到 `bias == 7`，`Row.fromLegacyId(5)` 得到 `99`。自动解码只走主构造器。若解码或 CLI 绑定本身也需要业务工厂，用户可以手写对应的 `decode` 或 `bind`，显式调用该工厂，其他缺失方法仍可生成。

**缺失字段中的简单默认值直接补齐，其余通过省略具名实参处理。** 两个生成器共用语言规范 11.13.3 的规则：无前置语句的 Boolean 字面量、直接核心 None 和空数组字面量不参与组合展开，空数组按次创建。生成方法先读完输入并完成容器检查，再补值并根据已保存的字段值执行一次普通构造调用。例如 `suppliedBias: Option<Int>` 表示输入中 `bias` 的存在性，构造部分只需以下源码。

```scoop
return when (suppliedBias) {
    case Some(val bias) -> Row(id = id, bias = bias)
    case None -> Row(id = id)
}
```

- 必填参数始终传值。字段允许缺失且声明有默认值时，简单默认值直接补齐；其余缺失分支省略形参，由 Scoop 按形参声明序使用原来绑定的默认表达式，允许引用前参。`ignore` 且有默认值的参数始终省略。没有声明默认值的可空字段缺失时显式传 `None`。读取可空字段时外层 Option 单独记录存在性，`Some(None)` 将显式空值传给构造器。
- 命名 enum payload 调用已选的 `Choice.Named(...)`，使用同样的具名实参规则。位置 payload 和 tuple 使用原位置顺序。具名 `vararg` 有值时按现有规则传完整数组。默认值的定义处绑定、跨 Cone 可见性和调用处求值均沿用现有规则。[普通调用与默认参数](../specs/SCOOP-SPEC.md#85-调用实参默认参数与-vararg)
- 只有运行时可能缺失且需要构造器提供默认值的参数才需要展开分支。必填、固定忽略、简单默认值和缺失时直接补 `None` 的参数均可预先确定实参形态。若仍有 `k` 个需要省略实参的独立缺省项，生成源码最多有 `2^k` 种调用形态，运行时只执行其中一次；只有简单默认值时只生成一次构造调用。CLI 与 codec 都用多字段模型检查生成代码体积。

生成器通过 `describeMembers` 的 `constructorKind`、`access.accessible` 和形参映射检查这条约定。主构造器不可见或另有可见次构造器时，在 `@GenerateWith` 的对应接口处报告生成错误并关联构造器声明，提示将额外入口移到 companion 工厂。该约定只在实际生成目标对象构造代码时生效。生成 `encode` 或 `entries`、手写 `decode` 或 `bind`、使用其他生成器时，分别遵守对应方法或生成器自己的要求。[成员与访问性查询](MEMBERS-JSON.md)

验收包括主构造器与具名工厂分别得到 `7`、`99`，额外可见构造器的生成诊断，以及跨 Cone 默认值、前参依赖、显式空值和带副作用的默认值。容器 `end` 失败或 CLI 提前返回 help 时，默认值和构造器的执行次数均应为零。[现有默认值用例](../../sysroot/src/scoop.json/tests/consumer/derived-decoding-defaults/main.scoop)

## 5. `scoop.cli` 的独立接口

迁移前，`Cli.schema(typeInfo<T>(), config)` 在运行时扫描字段，`CliParser(schema, codec)` 把 argv 包装成 `CliDecoder` 交给 `ValueDecoder<T>.decode`。迁移后，参数类型通过 `CliArguments<T>` 提供布局和构造逻辑，自定义字段通过 `CliValueParser<T>` 提供转换。token 如何消费、何时返回 help，由 `scoop.cli` 在一处处理。[CLI 入口](../../sysroot/src/scoop.cli/src/cli.scoop) · [argv 处理](../../sysroot/src/scoop.cli/src/tokens.scoop)

```scoop
public interface CliArguments<T> {
    public fun entries(): Array<CliEntry>
    public fun bind(values: CliValues): T
}

public interface CliValueParser<T> {
    public fun shape(): CliValueShape
    public fun parseValue(values: CliFieldValues): T
}
```

- `CliArguments<T>` 是记录级入口。`scoop-cli-generator` 为 `entries()` 生成含源码字段名、role、alias、位置、形状、必填性和 help 的参数表。`bind` 按字段读取已出现的值，调用对应的类型转换，检查必填项，最后按第 4 节构造 `T`。默认支持有真实主构造器映射的 struct 及无 class 基类的 final class。跳过 computed property。含委托属性、自定义存储访问器或其他未映射状态的类型可手写 `CliArguments<T>`。形状已知的错误定位到模型声明。
- `CliValueParser<F>` 用于 `Port` 之类的自定义字段或有约束的泛型字段，方法同时说明一个字段怎样收集及转换。生成器优先选择 parser 主构造器上唯一匹配的显式 `val`，否则使用内置标量、容器组合或满足 `CliValueParser<F>` 的可见 companion。依赖歧义或开放参数缺少条件在定义处报错。应用也可手写整份 `entries` 和 `bind`，覆盖现有显式 schema、只读取部分输入等用法。
- `CliValues` 提供按 `sourceName` 的 `required`／`optional` 只读查询。`CliFieldValues` 保留本字段各次出现顺序、文本或 flag 的布尔值、原始 token 及下标。库向跨 Cone 的生成正文和手写 parser 公开必要的读取与转换方法，统一处理整数宽度、Float／Double 精度、Char 和 Boolean。库在一次 argv 扫描中收集数据，命中 help 即在进入 `bind` 前返回。
- `CliParser<T>(config, arguments: CliArguments<T>)` 在建立时取一次 entries 并建立布局，提供 `parse`、`parseCommandLine`、`help`、`writeHelp`。`parse` 接收纯参数，`parseCommandLine` 跳过程序名，空完整 argv 报错。`CliResult.Parsed` 和 `HelpRequested` 保留。生成器能确定的注解冲突在编译期诊断。手写或取决于用户 `shape()` 的布局在创建时检查一次，后续解析、help 共用该布局。

例如，若第 1 节的 `port` 改为 `val port: Port = Port(8080)`，只需在字段类型上实现文本转换。

```scoop
public struct Port(val value: Int) {
    public companion object : CliValueParser<Port> {
        public override fun shape(): CliValueShape = CliValueShape.Text
        public override fun parseValue(values: CliFieldValues): Port = Port(values.readInt())
    }
}
```

传入 `--port 9000` 时，生成的 `bind` 用 `Port.Companion` 将该字段转为 `Port(9000)`。省略 `--port` 时，`bind` 调用 `Options` 时省略该实参，得到模型声明的 `Port(8080)`。

| 现有用法 | 迁移后的 CLI 行为 |
| --- | --- |
| `@CliOption`、`@CliFlag`、`@CliArgument` | 保留 role、alias、位置与 help 参数。三个注解增加 `required: Boolean = false` 以显式要求出现。默认必填仍由原声明是否为 Option、构造参数是否有默认值决定。`defaultText` 只用于 help。 |
| CLI 字段借用 `@Serialize` | `ignore` 迁为 `@CliIgnore`，被忽略的主构造参数必须有默认值或能按声明补 `None`。`required = true` 迁入该字段的 CLI role。原 `rename` 只改变内部 wire key，argv 名一直由 CLI alias 决定。`@Serialize` 此后只影响 codec。 |
| enum、Option 与重复项 | 纯 unit enum 从源码 variant 拼写直接映射到 variant 值。支持普通标量、Option 以及一层 Array／MutableArray／ArrayList，包括 `Option<Array<T>>`、`Array<Option<T>>`。未出现的重复项保持缺失。已给出的 `"null"` 和空字符串是实际文本。exact Boolean 默认作 flag，显式 option 和可空 Boolean 按文本解析。 |
| 输入错误和手写 reader | argv 和库内字段转换错误抛 CLI 自己的 `CliParseException`，携带以源码字段名生成的路径、原因和可用 token 来源。例如 `port` 的旧 JSON 名为 `wire-port` 时，CLI 路径为 `/port`。动态布局错误继续抛 `IllegalArgumentException`，用户转换／构造／writer 的异常原样传播。手写 caller 改读 `CliValues`。 |

`--name=value`、短 flag 组合、末尾带值短 option、正负 flag、负数、重复项顺序和 `--` 终止符由现有 argv 逻辑继续处理。保留 `--port --help` 先报缺值、`--help --bad` 返回 help、`-- --help` 作为位置值的区别。切换时移除运行时 `automaticEntries` 和通用 `CliDecoder`，删去 `wireKey`、`wireName` 和用于自动推导的反射字段。[现有 CLI 语义基线](../specs/SCOOP-SPEC.md#127-独立命令行参数库)

## 6. 实施与验收顺序

| 顺序 | 交付和完成条件 |
| --- | --- |
| 1. 条件接口 | 先修订语言规范 9.1.8、9.4 与编译器规范的生成阶段。用自定义生成器覆盖含伴生投影的选择、只在块内成立的接口 bound、不同方向互不借条件、继承后代换、已有手写／default、合法与非法调用。未继承、重复选择等诊断指向注解数组元素。 |
| 2. 构造与公共查询 | 将第 4 节的约定写入 11.13 和 12.7 的生成器说明，验证普通具名调用、companion 工厂及不符合约定时的诊断。补齐两个生成器真实使用的查询投影及协议文档，验证同名／alias／private、原声明与应用类型。 |
| 3. codec 切换 | 修订 9.5、11.13，交付宿主 `scoop-codec-generator`。同一改动迁移应用、core 中实际需要生成的声明、库测试、示例与 fixture，并移除两条 HIR 合成路径。通过 JSON 的泛型条件与继承、容器与 tuple、enum、注入优先级、默认值顺序、显式 null 及手写单方向测试。 |
| 4. CLI 切换 | 修订 12.7，交付 `scoop-cli-generator`、独立 CLI 接口和解析库，更新 `greet`、`repeated_options`、跨 Cone 调用和编程材料。测试无 codec 的参数类型、手写 `Port`、help、数值边界、错误位置，以及同一类型上 JSON rename 与 CLI alias 各自生效。静态模型错误和动态 schema 错误在各自的阶段验证。 |
| 5. 工具链交付 | 两个程序均为 Rust 宿主工具，复用 `scoop-generator-protocol` 的同步协议，与 `scoopc` 一同构建并安装。复用现有 `scoop.codec` 映射，增加 `scoop.cli`，沿用 `--generator NAME=PATH`。发布脚本先准备宿主工具再构建目标 sysroot，生成正文随 `.slib` 导出。用真实 `scoop build` 验证冷构建、未变命中、实际使用的生成器更新、消费依赖不重跑上游生成，以及缺少宿主程序的诊断。 |

普通行为覆盖放入各自的 Cone 测试，编译失败／诊断位置、宿主进程及跨产物行为用对应 fixture。回归可从 [条件 codec](../../sysroot/src/scoop.json/tests/consumer/conditional-derived-codecs)、[继承的条件 codec](../../sysroot/src/scoop.json/tests/consumer/conditional-inherited-codecs)、[CLI 基本行为与数值用例](../../sysroot/src/scoop.cli/tests/cli) 开始。规范改动对应[语言规范](../specs/SCOOP-SPEC.md)、[编译器与产物规范](../specs/SCOOP-IMPL-SPEC.md)及[编程材料更新工作流](../scoop-programming/UPDATE-WORKFLOW.md)。

## 7. 本机验收结果

- 五个 Cone 的 566 个测试全部通过，包括 JSON 107 个、CLI 6 个、URL 22 个、HTTP 客户端 254 个和服务端 177 个。额外验证了只生成 `encode` 或 `entries` 时，模型可以保留可见的次构造器。HTTP 客户端的 `post_json` 和服务端的 `json` 示例均通过 `scoop check --release`。
- 133 个相关 fixture 均通过，含修复后复验的 4 个。覆盖条件接口、生成诊断位置、独立宿主进程及下游读取 `.slib`。用真实工具链冷构建时两个生成器各运行一次，立即重建及另一个 Cone 消费预编译产物时调用次数都未增加。更新已使用的 CLI 生成器后，构建重新执行。
- 默认值体积回归使用同一个含 5 个 Boolean、3 个空数组和 2 个 None 默认值的模型：CLI 的 `bind` 正文从 261,905 字节降为 5,899 字节，codec 的 `decode` 从 260,849 字节降为 5,854 字节；构造调用均从 1,024 个降为 1 个。10 个简单默认值与 2 个自定义默认值组合时保留 4 种调用形态。
- 本机 Darwin AArch64 验证通过 5 个 Rust 回归和 13 个相关 fixture，包含 CLI／JSON Cone 的 release 原生测试、生成诊断及产物复用。额外的跨 Cone consumer 从 `.slib` 读取模型默认值，生成 CLI／codec 并运行通过。新增行为覆盖默认值副作用与前参依赖、required／ignore、泛型空数组、独立数组实例、嵌套 Option 和显式 null。
