# Scoop 诊断规则与编号

状态：**编号已接入；实际发出点以类型化 `DiagnosticCode` 为准**。原始文案证据基于 2026-11-16 清点；实际实现及验收基线见方案。编号语义、边界和计数口径见 [方案](NUMBERING.md)；逐条原文归属见 [完整对照](RULE-MAPPING.md)。

每个编号对应一个主规则或一个可参数化的诊断合同。合同下不同的字段、参数、候选、阶段和失败原因仍保留；它们并不因为共用主码而被宣称为同义。证据列只列代表项，全部原始行在对照表中。

## 错误（348 项）

### 01 · 词法与字面量

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0101"></a>`E0101` | 无法识别的字符 | 当前位置不能形成任何 token。 | [F1470](FRONTEND.md#f1470)；[parser/src/lexer.rs:324](../../compiler/parser/src/lexer.rs#L324) |
| <a id="e0102"></a>`E0102` | 未闭合的词法结构 | 字符串、原始字符串、插值、块注释、转义标识符或转义序列未结束；结构种类为参数。 | [F1469](FRONTEND.md#f1469)；[F1471](FRONTEND.md#f1471)；[F1476](FRONTEND.md#f1476)；[parser/src/lexer.rs:108](../../compiler/parser/src/lexer.rs#L108) |
| <a id="e0103"></a>`E0103` | 无效的转义标识符 | 转义标识符为空，或包含换行/控制字符。 | [F1475](FRONTEND.md#f1475)；[F1477](FRONTEND.md#f1477)；[parser/src/lexer/identifiers.rs:18](../../compiler/parser/src/lexer/identifiers.rs#L18) |
| <a id="e0104"></a>`E0104` | 数字分隔符位置无效 | 分隔符必须位于该进制的两位有效数字之间。 | [F1473](FRONTEND.md#f1473)；[F1478](FRONTEND.md#f1478)；[parser/src/lexer/floating.rs:83](../../compiler/parser/src/lexer/floating.rs#L83) |
| <a id="e0105"></a>`E0105` | 数字组成不完整或进制不符 | 进制前缀、浮点小数/指数缺少数字，或包含该进制不允许的数字。 | [F1474](FRONTEND.md#f1474)；[F1479](FRONTEND.md#f1479)；[F1480](FRONTEND.md#f1480)；[parser/src/lexer/floating.rs:95](../../compiler/parser/src/lexer/floating.rs#L95) |
| <a id="e0106"></a>`E0106` | 数值字面量后缀无效 | 整数/浮点后缀不在语言允许的集合内。 | [F1472](FRONTEND.md#f1472)；[F1481](FRONTEND.md#f1481)；[parser/src/lexer/floating.rs:57](../../compiler/parser/src/lexer/floating.rs#L57) |
| <a id="e0107"></a>`E0107` | 数值字面量超出表示范围 | 含词法 u64 幅值上限、显式后缀范围及期望数值类型的表示范围；浮点舍入到无穷为附加原因。 | [F0595](FRONTEND.md#f0595)；[F0596](FRONTEND.md#f0596)；[F0803](FRONTEND.md#f0803)；[hir-lower/src/expr/floating.rs:39](../../compiler/hir-lower/src/expr/floating.rs#L39) |
| <a id="e0108"></a>`E0108` | 字符字面量不是一个 Unicode 标量 | 空、多字符及非标量字符不能作为字符字面量。 | [F1486](FRONTEND.md#f1486)；[parser/src/lexer/literals.rs:43](../../compiler/parser/src/lexer/literals.rs#L43) |
| <a id="e0109"></a>`E0109` | 无效转义 | 不支持的转义、Unicode 转义格式或标量值无效。 | [F1489](FRONTEND.md#f1489)；[F1490](FRONTEND.md#f1490)；[F1491](FRONTEND.md#f1491)；[parser/src/lexer/literals.rs:126](../../compiler/parser/src/lexer/literals.rs#L126) |
| <a id="e0110"></a>`E0110` | 插值缺少花括号 | 插值表达式必须采用规定的带花括号形式。 | [F1484](FRONTEND.md#f1484)；[parser/src/lexer/interpolation.rs:81](../../compiler/parser/src/lexer/interpolation.rs#L81) |

### 02 · 语法结构

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0201"></a>`E0201` | 缺少预期的语法项 | 缺少 token、名称、分隔符或合法声明/表达式；expected/found 和语法上下文为参数，136 项 expected 列表不各建编号。 | [F1366](FRONTEND.md#f1366)；[F1371](FRONTEND.md#f1371)；[F1385](FRONTEND.md#f1385)；[parser/src/decl/enums/members.rs:72](../../compiler/parser/src/decl/enums/members.rs#L72) |
| <a id="e0202"></a>`E0202` | 重复 package 声明 | 同一源文件只能有一个 package header。 | [F1492](FRONTEND.md#f1492)；[parser/src/parser/file.rs:41](../../compiler/parser/src/parser/file.rs#L41) |
| <a id="e0203"></a>`E0203` | 文件头顺序不合法 | package 在 import 和声明之前，文件级 import 在声明之前。 | [F1493](FRONTEND.md#f1493)；[F1494](FRONTEND.md#f1494)；[parser/src/parser/file.rs:52](../../compiler/parser/src/parser/file.rs#L52) |
| <a id="e0204"></a>`E0204` | 语法列表不允许为空 | 当前要求非空的 tuple、context、requires、copy update 列表为空。 | [F0462](FRONTEND.md#f0462)；[F1359](FRONTEND.md#f1359)；[F1376](FRONTEND.md#f1376)；[hir-lower/src/expr/aggregates.rs:16](../../compiler/hir-lower/src/expr/aggregates.rs#L16) |
| <a id="e0205"></a>`E0205` | copy update 字段形式不合法 | 不允许 rest、嵌套路径、语句值和尾逗号；原因保留为具体语法约束。 | [F1451](FRONTEND.md#f1451)；[F1452](FRONTEND.md#f1452)；[F1453](FRONTEND.md#f1453)；[parser/src/expr/places.rs:27](../../compiler/parser/src/expr/places.rs#L27) |
| <a id="e0206"></a>`E0206` | 数组切片语法不受支持 | 现有 parser 明确拒绝切片；去掉文案中的里程碑信息后仍保留实际限制，不据此扩大实现。 | [F1455](FRONTEND.md#f1455)；[parser/src/expr/places.rs:100](../../compiler/parser/src/expr/places.rs#L100) |
| <a id="e0207"></a>`E0207` | 类型占位符使用位置不合法 | `_` 仅用于紧接调用的类型实参列表。 | [F1464](FRONTEND.md#f1464)；[F1521](FRONTEND.md#f1521)；[parser/src/expr/primary.rs:350](../../compiler/parser/src/expr/primary.rs#L350) |
| <a id="e0208"></a>`E0208` | suspend 缺少函数类型 | suspend 类型前缀必须修饰函数类型。 | [F1520](FRONTEND.md#f1520)；[parser/src/ty.rs:62](../../compiler/parser/src/ty.rs#L62) |

### 03 · 声明与修饰符

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0301"></a>`E0301` | 重复声明 | 同一声明域出现同名非重载实体、参数、字段、变体或类型参数；实体种类为参数。 | [F0008](FRONTEND.md#f0008)；[F0009](FRONTEND.md#f0009)；[F0010](FRONTEND.md#f0010)；[hir-lower/src/aliases/declarations.rs:32](../../compiler/hir-lower/src/aliases/declarations.rs#L32) |
| <a id="e0302"></a>`E0302` | 重复可调用签名 | 同一重载域出现相同函数/构造器/扩展属性签名。 | [F0142](FRONTEND.md#f0142)；[F0143](FRONTEND.md#f0143)；[F0358](FRONTEND.md#f0358)；[hir-lower/src/class/construction/validation.rs:25](../../compiler/hir-lower/src/class/construction/validation.rs#L25) |
| <a id="e0303"></a>`E0303` | 重复修饰符 | 相同修饰符重复，或同一位置出现多个可见性修饰符。 | [F1356](FRONTEND.md#f1356)；[F1358](FRONTEND.md#f1358)；[F1367](FRONTEND.md#f1367)；[parser/src/decl/compatibility.rs:35](../../compiler/parser/src/decl/compatibility.rs#L35) |
| <a id="e0304"></a>`E0304` | 互斥修饰符组合 | modality 等修饰符组合不允许；不与重复修饰符合并。 | [F1431](FRONTEND.md#f1431)；[F1434](FRONTEND.md#f1434)；[parser/src/decl/nominals/prefix.rs:169](../../compiler/parser/src/decl/nominals/prefix.rs#L169) |
| <a id="e0305"></a>`E0305` | 修饰符不适用于该声明 | 声明类别/位置不允许该修饰符；包括顶层 modality、非函数 suspend、非类 inner、受限 protected。 | [F0227](FRONTEND.md#f0227)；[F0232](FRONTEND.md#f0232)；[F0839](FRONTEND.md#f0839)；[hir-lower/src/class/inheritance/overrides/access.rs:59](../../compiler/hir-lower/src/class/inheritance/overrides/access.rs#L59) |
| <a id="e0306"></a>`E0306` | 不支持 lateinit 声明 | 当前语言要求使用 Option 属性及允许的省略初始化写法。 | [F1427](FRONTEND.md#f1427)；[parser/src/decl/nominals/prefix.rs:115](../../compiler/parser/src/decl/nominals/prefix.rs#L115) |
| <a id="e0307"></a>`E0307` | 扩展声明位置不合法 | 扩展函数/属性仅允许顶层声明。 | [F1116](FRONTEND.md#f1116)；[F1372](FRONTEND.md#f1372)；[hir-lower/src/properties/declarations.rs:111](../../compiler/hir-lower/src/properties/declarations.rs#L111) |
| <a id="e0308"></a>`E0308` | 声明缺少必须的函数体 | 具体函数、局部函数或私有接口成员必须有函数体。 | [F0249](FRONTEND.md#f0249)；[F0252](FRONTEND.md#f0252)；[F1205](FRONTEND.md#f1205)；[hir-lower/src/class/methods.rs:157](../../compiler/hir-lower/src/class/methods.rs#L157) |
| <a id="e0309"></a>`E0309` | 抽象声明不允许函数体 | 抽象函数/属性不得同时提供实现或初始化。 | [F0253](FRONTEND.md#f0253)；[F1125](FRONTEND.md#f1125)；[F1373](FRONTEND.md#f1373)；[hir-lower/src/class/methods.rs:192](../../compiler/hir-lower/src/class/methods.rs#L192) |
| <a id="e0310"></a>`E0310` | 声明类别不允许类型参数 | 如 annotation、secondary constructor 等不能声明类型形参。 | [F1352](FRONTEND.md#f1352)；[F1388](FRONTEND.md#f1388)；[F1402](FRONTEND.md#f1402)；[parser/src/decl/annotations/declaration.rs:40](../../compiler/parser/src/decl/annotations/declaration.rs#L40) |
| <a id="e0311"></a>`E0311` | 成员或声明形态不适用于所属类型 | enum/interface/struct 的 init/constructor、匿名 object 的显式构造等不允许；附加原因保留所属类型及被拒绝成员种类。 | [F0376](FRONTEND.md#f0376)；[F1364](FRONTEND.md#f1364)；[F1365](FRONTEND.md#f1365)；[hir-lower/src/declarations/objects.rs:202](../../compiler/hir-lower/src/declarations/objects.rs#L202) |

### 04 · 名称、导入与可见性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0401"></a>`E0401` | 名称未解析 | 在对应名称域找不到变量、函数、类型、注解或绑定；不把访问域错误和歧义并入本规则。 | [F0461](FRONTEND.md#f0461)；[F0488](FRONTEND.md#f0488)；[F0601](FRONTEND.md#f0601)；[hir-lower/src/expr.rs:439](../../compiler/hir-lower/src/expr.rs#L439) |
| <a id="e0402"></a>`E0402` | 成员不存在 | 接收者/类型/包不存在所需字段、属性、方法、嵌套类型或 enum 变体。 | [F0240](FRONTEND.md#f0240)；[F0241](FRONTEND.md#f0241)；[F0490](FRONTEND.md#f0490)；[hir-lower/src/class/initialization/fields.rs:159](../../compiler/hir-lower/src/class/initialization/fields.rs#L159) |
| <a id="e0403"></a>`E0403` | 声明不可访问 | 引用位置超出声明或 accessor 的访问域；本地/依赖、构造器/方法/属性使用同一规则。 | [F0003](FRONTEND.md#f0003)；[F0135](FRONTEND.md#f0135)；[F0194](FRONTEND.md#f0194)；[hir-lower/src/aliases/applications.rs:22](../../compiler/hir-lower/src/aliases/applications.rs#L22) |
| <a id="e0404"></a>`E0404` | 名称查找有歧义 | 同一有效查找层内出现多个名称绑定；与可调用候选适用性歧义分开。 | [F0519](FRONTEND.md#f0519)；[F0617](FRONTEND.md#f0617)；[F0618](FRONTEND.md#f0618)；[hir-lower/src/expr/callable_references/unbound/qualifier.rs:36](../../compiler/hir-lower/src/expr/callable_references/unbound/qualifier.rs#L36) |
| <a id="e0405"></a>`E0405` | 名称种类不适用于当前位置 | 把类型/包/注解当值，把值当类型限定符，或把函数声明当现成函数值。 | [F0458](FRONTEND.md#f0458)；[F0616](FRONTEND.md#f0616)；[F0676](FRONTEND.md#f0676)；[hir-lower/src/expr.rs:326](../../compiler/hir-lower/src/expr.rs#L326) |
| <a id="e0406"></a>`E0406` | 导入目标不合法或不存在 | 精确 import 必须指向可导入绑定；目标不可用时保留查找原因。 | [F1007](FRONTEND.md#f1007)；[F1008](FRONTEND.md#f1008)；[hir-lower/src/imports/selector.rs:36](../../compiler/hir-lower/src/imports/selector.rs#L36) |
| <a id="e0407"></a>`E0407` | 公开导入规则不满足 | 公开导入只在文件头且目标必须是直接依赖绑定；core 不允许重导出。 | [F0980](FRONTEND.md#f0980)；[F0994](FRONTEND.md#f0994)；[F0997](FRONTEND.md#f0997)；[hir-lower/src/imports/blocks.rs:18](../../compiler/hir-lower/src/imports/blocks.rs#L18) |
| <a id="e0408"></a>`E0408` | 星号导入形式不合法 | 星号 selector 不能带 alias，且必须以星号结束。 | [F1466](FRONTEND.md#f1466)；[F1467](FRONTEND.md#f1467)；[parser/src/header.rs:87](../../compiler/parser/src/header.rs#L87) |
| <a id="e0409"></a>`E0409` | 公开绑定冲突 | 重导出目标与已有公开绑定重名。 | [F1000](FRONTEND.md#f1000)；[F1072](FRONTEND.md#f1072)；[hir-lower/src/imports/reexports.rs:320](../../compiler/hir-lower/src/imports/reexports.rs#L320) |
| <a id="e0410"></a>`E0410` | 签名泄露不可访问或局部类型 | 可见声明的类型不能超出其访问域，局部/匿名类型不能通过推断结果逃逸。 | [F1013](FRONTEND.md#f1013)；[F1014](FRONTEND.md#f1014)；[F1349](FRONTEND.md#f1349)；[hir-lower/src/local_types/exposure.rs:37](../../compiler/hir-lower/src/local_types/exposure.rs#L37) |
| <a id="e0411"></a>`E0411` | protected 接收者不合法 | 访问 protected 成员还必须满足接收者限制。 | [F0244](FRONTEND.md#f0244)；[hir-lower/src/class/lookup.rs:122](../../compiler/hir-lower/src/class/lookup.rs#L122) |

### 05 · 类型与转换

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0501"></a>`E0501` | 类型不匹配 | 赋值/实参/返回/初始化/元素/receiver 的实际类型不满足期望关系；expected、found 和关系（相同/可赋值/子类型）是参数。 | [F0098](FRONTEND.md#f0098)；[F0106](FRONTEND.md#f0106)；[F0108](FRONTEND.md#f0108)；[hir-lower/src/call_resolution/diagnostics.rs:201](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L201) |
| <a id="e0502"></a>`E0502` | 表达式不是可调用类型 | 要求函数类型/可调用值却得到其他类型。 | [F0099](FRONTEND.md#f0099)；[F0522](FRONTEND.md#f0522)；[hir-lower/src/call_resolution/diagnostics.rs:219](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L219) |
| <a id="e0503"></a>`E0503` | 函数类型的参数形状不符 | lambda/匿名函数/期望函数类型的参数数量或 receiver 形状不符；普通与 suspend 另列。 | [F0101](FRONTEND.md#f0101)；[F0475](FRONTEND.md#f0475)；[F0479](FRONTEND.md#f0479)；[hir-lower/src/call_resolution/diagnostics.rs:225](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L225) |
| <a id="e0504"></a>`E0504` | 普通与 suspend 函数类型不相容 | 两种函数类型不能直接作为同一种函数值使用。 | [F0100](FRONTEND.md#f0100)；[F0474](FRONTEND.md#f0474)；[hir-lower/src/call_resolution/diagnostics.rs:222](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L222) |
| <a id="e0505"></a>`E0505` | Option 操作数类型不符 | `?:`、`?.`、`!!` 要求 Option 操作数；操作符为参数。 | [F0583](FRONTEND.md#f0583)；[F0587](FRONTEND.md#f0587)；[F0588](FRONTEND.md#f0588)；[hir-lower/src/expr/elvis.rs:19](../../compiler/hir-lower/src/expr/elvis.rs#L19) |
| <a id="e0506"></a>`E0506` | 不可能成功的类型检查 | is 检查的两个类型不可能相交；当前是 error，不自动改为 warning。 | [F0805](FRONTEND.md#f0805)；[hir-lower/src/expr/type_checks.rs:59](../../compiler/hir-lower/src/expr/type_checks.rs#L59) |
| <a id="e0507"></a>`E0507` | 不可能成功的类型转换 | 强制转换的源/目标类型不可能相容。 | [F0806](FRONTEND.md#f0806)；[hir-lower/src/expr/type_checks.rs:117](../../compiler/hir-lower/src/expr/type_checks.rs#L117) |
| <a id="e0508"></a>`E0508` | 值类型不能使用引用相等 | 引用相等操作必须用于允许的引用类型。 | [F0793](FRONTEND.md#f0793)；[hir-lower/src/expr/operators.rs:346](../../compiler/hir-lower/src/expr/operators.rs#L346) |
| <a id="e0509"></a>`E0509` | 值布局无限递归 | 按值递归路径未经过引用边界；普通值布局与 C 布局路径共用此规则。 | [F0393](FRONTEND.md#f0393)；[F0830](FRONTEND.md#f0830)；[F1281](FRONTEND.md#f1281)；[hir-lower/src/derived.rs:145](../../compiler/hir-lower/src/derived.rs#L145) |
| <a id="e0510"></a>`E0510` | 类型不可直接构造 | interface、object、裸 enum、annotation、没有源码构造器的 intrinsic 等不能直接调用构造；原因列明形态。 | [F0459](FRONTEND.md#f0459)；[F0460](FRONTEND.md#f0460)；[F0493](FRONTEND.md#f0493)；[hir-lower/src/expr.rs:423](../../compiler/hir-lower/src/expr.rs#L423) |
| <a id="e0511"></a>`E0511` | 布局查询要求值类型 | memory layout 查询不能用于引用类型。 | [F0606](FRONTEND.md#f0606)；[hir-lower/src/expr/generic_calls/intrinsics.rs:159](../../compiler/hir-lower/src/expr/generic_calls/intrinsics.rs#L159) |

### 06 · 泛型、约束与推断

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0601"></a>`E0601` | 类型实参数量不符 | 包含非泛型声明收到类型实参、缺少完整宿主/外部类型应用；不按名义类型种类分码。 | [F0001](FRONTEND.md#f0001)；[F0004](FRONTEND.md#f0004)；[F0005](FRONTEND.md#f0005)；[hir-lower/src/aliases.rs:126](../../compiler/hir-lower/src/aliases.rs#L126) |
| <a id="e0602"></a>`E0602` | 不满足 kind bound | 实际类型实参不符合 value/ref/GC-free 等声明 kind 要求；GC-free 操作的效果限制另列。 | [F0091](FRONTEND.md#f0091)；[F0925](FRONTEND.md#f0925)；[F1279](FRONTEND.md#f1279)；[hir-lower/src/call_resolution/diagnostics.rs:109](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L109) |
| <a id="e0603"></a>`E0603` | 不满足名义上界 | 实际类型实参不满足 class/interface 上界。 | [F0093](FRONTEND.md#f0093)；[F0094](FRONTEND.md#f0094)；[F0749](FRONTEND.md#f0749)；[hir-lower/src/call_resolution/diagnostics.rs:140](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L140) |
| <a id="e0604"></a>`E0604` | 违反负接口约束 | 实参实现了被负约束排除的接口，或 companion 负约束不成立。 | [F0092](FRONTEND.md#f0092)；[F0109](FRONTEND.md#f0109)；[hir-lower/src/call_resolution/diagnostics.rs:122](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L122) |
| <a id="e0605"></a>`E0605` | 类型推断约束冲突 | 同一推断变量得到不相容解；不和没有信息/多解合并。 | [F0095](FRONTEND.md#f0095)；[hir-lower/src/call_resolution/diagnostics.rs:160](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L160) |
| <a id="e0606"></a>`E0606` | 类型推断存在多个解 | 约束不能唯一确定类型。 | [F0096](FRONTEND.md#f0096)；[hir-lower/src/call_resolution/diagnostics.rs:176](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L176) |
| <a id="e0607"></a>`E0607` | 缺少足够的类型推断信息 | 空数组、lambda 参数、泛型函数引用、Ptr/enum 应用缺少可确定类型的信息。 | [F0076](FRONTEND.md#f0076)；[F0097](FRONTEND.md#f0097)；[F0464](FRONTEND.md#f0464)；[hir-lower/src/call_resolution/applicability/references.rs:56](../../compiler/hir-lower/src/call_resolution/applicability/references.rs#L56) |
| <a id="e0608"></a>`E0608` | 类型别名循环 | 展开 typealias 遇到循环依赖。 | [F0002](FRONTEND.md#f0002)；[hir-lower/src/aliases.rs:158](../../compiler/hir-lower/src/aliases.rs#L158) |
| <a id="e0609"></a>`E0609` | 声明类型推断循环 | 声明签名互相依赖，无法完成推断；需要明确类型注解。 | [F0352](FRONTEND.md#f0352)；[F0355](FRONTEND.md#f0355)；[hir-lower/src/declaration_inference.rs:219](../../compiler/hir-lower/src/declaration_inference.rs#L219) |
| <a id="e0610"></a>`E0610` | 递归调用改变完整类型实参映射 | 当前实现拒绝递归 callable 链改变完整类型实参映射；不把它描述为编译器已经证明该程序无限递归。 | [F0261](FRONTEND.md#f0261)；[F0418](FRONTEND.md#f0418)；[hir-lower/src/concretize/recursion.rs:47](../../compiler/hir-lower/src/concretize/recursion.rs#L47) |
| <a id="e0611"></a>`E0611` | 约束形式不适用于声明 | 负约束/companion 条件/requires 只在规定的泛型函数等位置允许。 | [F1253](FRONTEND.md#f1253)；[F1255](FRONTEND.md#f1255)；[F1265](FRONTEND.md#f1265)；[hir-lower/src/types/constraints/bounds.rs:19](../../compiler/hir-lower/src/types/constraints/bounds.rs#L19) |
| <a id="e0612"></a>`E0612` | 约束主体不合法 | where 参数不存在，或 requirement 越过本声明的类型/虚拟参数范围。 | [F1252](FRONTEND.md#f1252)；[F1268](FRONTEND.md#f1268)；[F1271](FRONTEND.md#f1271)；[hir-lower/src/types/constraints.rs:65](../../compiler/hir-lower/src/types/constraints.rs#L65) |
| <a id="e0613"></a>`E0613` | 上界类型种类错误 | 正上界必须是 class/interface；负上界必须是 interface；不能将引用单例作为 value 约束。 | [F1254](FRONTEND.md#f1254)；[F1258](FRONTEND.md#f1258)；[F1262](FRONTEND.md#f1262)；[hir-lower/src/types/constraints/bounds.rs:26](../../compiler/hir-lower/src/types/constraints/bounds.rs#L26) |
| <a id="e0614"></a>`E0614` | 泛型约束相互冲突 | kind 与名义上界互斥、多 class 上界、正负约束冲突。 | [F1256](FRONTEND.md#f1256)；[F1259](FRONTEND.md#f1259)；[F1260](FRONTEND.md#f1260)；[hir-lower/src/types/constraints/bounds.rs:49](../../compiler/hir-lower/src/types/constraints/bounds.rs#L49) |
| <a id="e0615"></a>`E0615` | 重复约束 | 重复 kind/接口上界/负约束/member requirement。 | [F1257](FRONTEND.md#f1257)；[F1261](FRONTEND.md#f1261)；[F1263](FRONTEND.md#f1263)；[hir-lower/src/types/constraints/bounds.rs:57](../../compiler/hir-lower/src/types/constraints/bounds.rs#L57) |
| <a id="e0616"></a>`E0616` | 名义泛型不允许变型投影 | 声明处 in/out 和使用处 in/out/* 投影都违反不变性规则；普通类型不相容仍用 type-mismatch。 | [F1374](FRONTEND.md#f1374)；[F1522](FRONTEND.md#f1522)；[F1523](FRONTEND.md#f1523)；[parser/src/decl/generics.rs:53](../../compiler/parser/src/decl/generics.rs#L53) |
| <a id="e0617"></a>`E0617` | 缺少可用的 companion 条件 | 静态类型没有 public companion，或泛型环境没有要求该 companion。 | [F1247](FRONTEND.md#f1247)；[F1248](FRONTEND.md#f1248)；[hir-lower/src/types/companions.rs:25](../../compiler/hir-lower/src/types/companions.rs#L25) |
| <a id="e0618"></a>`E0618` | 依赖类型投影形式不合法 | 依赖类型只支持 Companion 投影。 | [F1250](FRONTEND.md#f1250)；[F1375](FRONTEND.md#f1375)；[hir-lower/src/types/companions.rs:106](../../compiler/hir-lower/src/types/companions.rs#L106) |
| <a id="e0619"></a>`E0619` | 静态声明不能使用外层类型参数 | companion/静态 nested 作用域不能隐式使用宿主类型参数。 | [F1298](FRONTEND.md#f1298)；[F1299](FRONTEND.md#f1299)；[hir-lower/src/types/resolution.rs:84](../../compiler/hir-lower/src/types/resolution.rs#L84) |
| <a id="e0620"></a>`E0620` | member requirement 递归 | 验证 requirement 时形成递归谓词。 | [F0770](FRONTEND.md#f0770)；[hir-lower/src/expr/named_calls/requirements.rs:18](../../compiler/hir-lower/src/expr/named_calls/requirements.rs#L18) |
| <a id="e0621"></a>`E0621` | 精确成员 requirement 不满足 | 类型没有要求的精确 public 成员/构造器、效果或返回类型；保留每个候选拒绝原因为原因。 | [F0771](FRONTEND.md#f0771)；[F0772](FRONTEND.md#f0772)；[F0774](FRONTEND.md#f0774)；[hir-lower/src/expr/named_calls/requirements.rs:36](../../compiler/hir-lower/src/expr/named_calls/requirements.rs#L36) |
| <a id="e0622"></a>`E0622` | 成员 requirement 有歧义 | 存在多个满足 requirement 的成员或 invocation。 | [F0773](FRONTEND.md#f0773)；[F0777](FRONTEND.md#f0777)；[hir-lower/src/expr/named_calls/requirements.rs:102](../../compiler/hir-lower/src/expr/named_calls/requirements.rs#L102) |
| <a id="e0623"></a>`E0623` | member requirement 声明形态不合法 | 构造结果必须是所构造类型参数、构造不能 suspend、调用必须写出结果类型。 | [F1269](FRONTEND.md#f1269)；[F1270](FRONTEND.md#f1270)；[F1378](FRONTEND.md#f1378)；[hir-lower/src/types/constraints/requirements.rs:132](../../compiler/hir-lower/src/types/constraints/requirements.rs#L132) |
| <a id="e0624"></a>`E0624` | 扩展属性类型参数不可从 receiver 推断 | 扩展属性的每个类型形参必须仅凭 receiver 确定。 | [F0885](FRONTEND.md#f0885)；[hir-lower/src/globals/extensions.rs:179](../../compiler/hir-lower/src/globals/extensions.rs#L179) |

### 07 · 调用、重载与函数引用

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0701"></a>`E0701` | 调用实参数量不符 | 普通调用、函数值、注解、内建调用共用参数数量规则。 | [F0078](FRONTEND.md#f0078)；[F0525](FRONTEND.md#f0525)；[F0658](FRONTEND.md#f0658)；[hir-lower/src/call_resolution/arguments.rs:127](../../compiler/hir-lower/src/call_resolution/arguments.rs#L127) |
| <a id="e0702"></a>`E0702` | 未知命名实参 | 命名实参没有对应形参。 | [F0079](FRONTEND.md#f0079)；[F0564](FRONTEND.md#f0564)；[F0660](FRONTEND.md#f0660)；[hir-lower/src/call_resolution/arguments.rs:129](../../compiler/hir-lower/src/call_resolution/arguments.rs#L129) |
| <a id="e0703"></a>`E0703` | 形参被重复传入 | 同一形参获得多个实参。 | [F0080](FRONTEND.md#f0080)；[F0565](FRONTEND.md#f0565)；[F1317](FRONTEND.md#f1317)；[hir-lower/src/call_resolution/arguments.rs:131](../../compiler/hir-lower/src/call_resolution/arguments.rs#L131) |
| <a id="e0704"></a>`E0704` | 实参顺序不满足映射规则 | 位置实参不能接在打乱声明顺序的命名实参后；注解参数使用更严格的“位置参数在前”规则。文案必须保留当前调用形态的条件。 | [F0081](FRONTEND.md#f0081)；[F1351](FRONTEND.md#f1351)；[hir-lower/src/call_resolution/arguments.rs:134](../../compiler/hir-lower/src/call_resolution/arguments.rs#L134) |
| <a id="e0705"></a>`E0705` | 向仅命名形参传位置实参 | 调用违反 named-only 参数规则。 | [F0082](FRONTEND.md#f0082)；[hir-lower/src/call_resolution/arguments.rs:137](../../compiler/hir-lower/src/call_resolution/arguments.rs#L137) |
| <a id="e0706"></a>`E0706` | 向仅位置形参传命名实参 | 包括没有参数名合同的函数值。 | [F0083](FRONTEND.md#f0083)；[F0523](FRONTEND.md#f0523)；[hir-lower/src/call_resolution/arguments.rs:140](../../compiler/hir-lower/src/call_resolution/arguments.rs#L140) |
| <a id="e0707"></a>`E0707` | 非 vararg 位置使用 spread | spread 只能传入允许展开的变长参数。 | [F0084](FRONTEND.md#f0084)；[F0524](FRONTEND.md#f0524)；[F0566](FRONTEND.md#f0566)；[hir-lower/src/call_resolution/arguments.rs:143](../../compiler/hir-lower/src/call_resolution/arguments.rs#L143) |
| <a id="e0708"></a>`E0708` | 缺少必填实参 | 必需形参无实参且无默认值。 | [F0085](FRONTEND.md#f0085)；[F0567](FRONTEND.md#f0567)；[F1318](FRONTEND.md#f1318)；[hir-lower/src/call_resolution/arguments.rs:146](../../compiler/hir-lower/src/call_resolution/arguments.rs#L146) |
| <a id="e0709"></a>`E0709` | 混合互斥的 vararg 输入形式 | 同一变长参数使用不允许混合的输入形式。 | [F0086](FRONTEND.md#f0086)；[hir-lower/src/call_resolution/arguments.rs:149](../../compiler/hir-lower/src/call_resolution/arguments.rs#L149) |
| <a id="e0710"></a>`E0710` | 尾随 lambda 不能映射到 vararg | 保留为参数映射规则。 | [F0087](FRONTEND.md#f0087)；[hir-lower/src/call_resolution/arguments.rs:152](../../compiler/hir-lower/src/call_resolution/arguments.rs#L152) |
| <a id="e0711"></a>`E0711` | 没有适用可调用候选 | 重载、构造器、函数引用均无适用候选；候选拒绝原因不能另算主错误。 | [F0276](FRONTEND.md#f0276)；[F0505](FRONTEND.md#f0505)；[F0513](FRONTEND.md#f0513)；[hir-lower/src/constructor_resolution.rs:125](../../compiler/hir-lower/src/constructor_resolution.rs#L125) |
| <a id="e0712"></a>`E0712` | 可调用候选无法唯一选择 | 多个候选都适用，排序/声明转发后仍并列。 | [F0107](FRONTEND.md#f0107)；[F0277](FRONTEND.md#f0277)；[F0503](FRONTEND.md#f0503)；[hir-lower/src/call_resolution/named.rs:292](../../compiler/hir-lower/src/call_resolution/named.rs#L292) |
| <a id="e0713"></a>`E0713` | 该声明形式不能成为函数引用 | 成员 requirement、现成函数值、不可引用 intrinsic、annotationKey、super 等不支持该函数引用形式。 | [F0485](FRONTEND.md#f0485)；[F0487](FRONTEND.md#f0487)；[F0501](FRONTEND.md#f0501)；[hir-lower/src/expr/callable_references.rs:52](../../compiler/hir-lower/src/expr/callable_references.rs#L52) |
| <a id="e0714"></a>`E0714` | 不能经接口值使用泛型成员 | 接口 receiver 不提供此泛型成员的调用/引用能力。 | [F0489](FRONTEND.md#f0489)；[F0664](FRONTEND.md#f0664)；[hir-lower/src/expr/callable_references/bound.rs:49](../../compiler/hir-lower/src/expr/callable_references/bound.rs#L49) |
| <a id="e0715"></a>`E0715` | 构造器缺少外部实例或捕获值 | inner/local 构造器需要声明处外部实例/捕获输入；requirements 不得隐式提供这些输入。 | [F0160](FRONTEND.md#f0160)；[F0279](FRONTEND.md#f0279)；[F0498](FRONTEND.md#f0498)；[hir-lower/src/class/enclosing/receivers.rs:150](../../compiler/hir-lower/src/class/enclosing/receivers.rs#L150) |
| <a id="e0716"></a>`E0716` | 缺少所需接收者 | this、extension/member 调用等没有有效 receiver。 | [F0625](FRONTEND.md#f0625)；[F0654](FRONTEND.md#f0654)；[F0655](FRONTEND.md#f0655)；[hir-lower/src/expr/imported_properties.rs:135](../../compiler/hir-lower/src/expr/imported_properties.rs#L135) |
| <a id="e0717"></a>`E0717` | 调用目标未声明 infix 角色 | 函数值不提供 infix invoke 等指定角色。 | [F0520](FRONTEND.md#f0520)；[hir-lower/src/expr/callables.rs:25](../../compiler/hir-lower/src/expr/callables.rs#L25) |
| <a id="e0718"></a>`E0718` | 同一调用有多个尾随 lambda | 一个调用只接收一个尾随 lambda。 | [F1465](FRONTEND.md#f1465)；[parser/src/expr/trailing.rs:106](../../compiler/parser/src/expr/trailing.rs#L106) |
| <a id="e0719"></a>`E0719` | 一个参数列表含多个 vararg | 单列表至多一个变长形参。 | [F1357](FRONTEND.md#f1357)；[parser/src/decl/compatibility.rs:41](../../compiler/parser/src/decl/compatibility.rs#L41) |
| <a id="e0720"></a>`E0720` | 默认表达式访问域不合法 | 默认值不能读 delegate 存储或超出其允许的访问域。 | [F0385](FRONTEND.md#f0385)；[F0386](FRONTEND.md#f0386)；[hir-lower/src/defaults/access.rs:96](../../compiler/hir-lower/src/defaults/access.rs#L96) |
| <a id="e0721"></a>`E0721` | 继承的默认参数冲突 | 多个继承来源为同一参数提供不相容默认值。 | [F0387](FRONTEND.md#f0387)；[hir-lower/src/defaults/inherited.rs:141](../../compiler/hir-lower/src/defaults/inherited.rs#L141) |
| <a id="e0722"></a>`E0722` | 默认参数展开循环 | 默认表达式递归展开形成环。 | [F0391](FRONTEND.md#f0391)；[hir-lower/src/defaults/preparation/resolve.rs:51](../../compiler/hir-lower/src/defaults/preparation/resolve.rs#L51) |
| <a id="e0723"></a>`E0723` | override 不能新增默认参数 | 覆盖方法不能重新引入默认值。 | [F0392](FRONTEND.md#f0392)；[hir-lower/src/defaults/registration.rs:74](../../compiler/hir-lower/src/defaults/registration.rs#L74) |
| <a id="e0724"></a>`E0724` | 该字段形式不允许默认值 | enum 字段默认值只允许 constructor-style 形式。 | [F1169](FRONTEND.md#f1169)；[F1363](FRONTEND.md#f1363)；[hir-lower/src/signatures.rs:356](../../compiler/hir-lower/src/signatures.rs#L356) |
| <a id="e0725"></a>`E0725` | operator/infix 声明缺少 receiver | 规定角色必须是实例成员或允许的扩展。 | [F0111](FRONTEND.md#f0111)；[F0112](FRONTEND.md#f0112)；[hir-lower/src/callable_modifiers.rs:56](../../compiler/hir-lower/src/callable_modifiers.rs#L56) |
| <a id="e0726"></a>`E0726` | operator/infix 参数数量不符 | 角色规定的参数数量不匹配。 | [F0113](FRONTEND.md#f0113)；[F0119](FRONTEND.md#f0119)；[F0123](FRONTEND.md#f0123)；[hir-lower/src/callable_modifiers.rs:90](../../compiler/hir-lower/src/callable_modifiers.rs#L90) |
| <a id="e0727"></a>`E0727` | operator/infix 不允许 vararg | 角色要求 required/non-vararg 参数，不能使用默认值或变长参数；目标角色/参数为原因。 | [F0110](FRONTEND.md#f0110)；[F0114](FRONTEND.md#f0114)；[F0124](FRONTEND.md#f0124)；[hir-lower/src/callable_modifiers.rs:29](../../compiler/hir-lower/src/callable_modifiers.rs#L29) |
| <a id="e0728"></a>`E0728` | operator 角色名称无效 | component 索引必须为正，或名称不是支持的角色。 | [F0115](FRONTEND.md#f0115)；[F0116](FRONTEND.md#f0116)；[F0117](FRONTEND.md#f0117)；[hir-lower/src/callable_modifiers.rs:128](../../compiler/hir-lower/src/callable_modifiers.rs#L128) |
| <a id="e0729"></a>`E0729` | operator 声明形状不符 | 特定角色的普通/suspend、泛型、返回类型、实例成员约束；具体不符项作为原因保留。 | [F0118](FRONTEND.md#f0118)；[F0120](FRONTEND.md#f0120)；[F0121](FRONTEND.md#f0121)；[hir-lower/src/callable_modifiers.rs:162](../../compiler/hir-lower/src/callable_modifiers.rs#L162) |
| <a id="e0730"></a>`E0730` | 复合赋值的两条路径都适用 | assign 形式与普通 operator 回写形式同时适用。 | [F1184](FRONTEND.md#f1184)；[hir-lower/src/stmt/compound_assignments.rs:125](../../compiler/hir-lower/src/stmt/compound_assignments.rs#L125) |
| <a id="e0731"></a>`E0731` | 复合赋值没有适用运算符 | 保留 assign/fallback 两条候选链的拒绝原因。 | [F1187](FRONTEND.md#f1187)；[hir-lower/src/stmt/compound_assignments.rs:170](../../compiler/hir-lower/src/stmt/compound_assignments.rs#L170) |
| <a id="e0732"></a>`E0732` | 变体构造形式不合法 | unit variant 不使用调用括号，payload variant 必须提供其构造实参；变体本身并非不可构造。 | [F0555](FRONTEND.md#f0555)；[F0556](FRONTEND.md#f0556)；[F0572](FRONTEND.md#f0572)；[hir-lower/src/expr/constructors/aliases.rs:46](../../compiler/hir-lower/src/expr/constructors/aliases.rs#L46) |

### 08 · 控制流与异常

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0801"></a>`E0801` | 返回值路径不完整 | 非 Unit callable 存在可正常结束却不返回值的路径。 | [F0473](FRONTEND.md#f0473)；[F1182](FRONTEND.md#f1182)；[F1203](FRONTEND.md#f1203)；[hir-lower/src/expr/callable_literals/anonymous.rs:109](../../compiler/hir-lower/src/expr/callable_literals/anonymous.rs#L109) |
| <a id="e0802"></a>`E0802` | 匿名函数混用有值/无值 return | 匿名函数结果推断遇到互斥的 return 形态。 | [F0472](FRONTEND.md#f0472)；[hir-lower/src/expr/callable_literals/anonymous.rs:88](../../compiler/hir-lower/src/expr/callable_literals/anonymous.rs#L88) |
| <a id="e0803"></a>`E0803` | return 缺少返回值 | 当前函数需要值，当前 return 未携带值。 | [F1195](FRONTEND.md#f1195)；[hir-lower/src/stmt/jumps.rs:80](../../compiler/hir-lower/src/stmt/jumps.rs#L80) |
| <a id="e0804"></a>`E0804` | return 不允许用于当前上下文 | 初始化、构造、默认值、表达式函数体或无函数体上下文不允许显式 return。 | [F1194](FRONTEND.md#f1194)；[F1197](FRONTEND.md#f1197)；[F1198](FRONTEND.md#f1198)；[hir-lower/src/stmt/jumps.rs:8](../../compiler/hir-lower/src/stmt/jumps.rs#L8) |
| <a id="e0805"></a>`E0805` | return 目标不合法 | 非局部 return 或 lambda 裸 return 不符合当前 callable 的返回规则。 | [F1200](FRONTEND.md#f1200)；[F1201](FRONTEND.md#f1201)；[hir-lower/src/stmt/jumps.rs:163](../../compiler/hir-lower/src/stmt/jumps.rs#L163) |
| <a id="e0806"></a>`E0806` | 循环跳转没有合法目标 | break/continue 不在当前 callable 的循环内，或 label 不存在。 | [F1207](FRONTEND.md#f1207)；[F1208](FRONTEND.md#f1208)；[hir-lower/src/stmt/loops.rs:113](../../compiler/hir-lower/src/stmt/loops.rs#L113) |
| <a id="e0807"></a>`E0807` | 值 if 缺少 else | 作为值的 if 必须具备 else 分支。 | [F1181](FRONTEND.md#f1181)；[hir-lower/src/stmt/blocks.rs:245](../../compiler/hir-lower/src/stmt/blocks.rs#L245) |
| <a id="e0808"></a>`E0808` | 异常类型不满足 Throwable | throw 值或 catch 类型必须是 Throwable 子类型。 | [F1188](FRONTEND.md#f1188)；[F1189](FRONTEND.md#f1189)；[hir-lower/src/stmt/exceptions.rs:27](../../compiler/hir-lower/src/stmt/exceptions.rs#L27) |
| <a id="e0809"></a>`E0809` | catch 已被前面的分支覆盖 | 前面的 catch 已覆盖当前异常类型。 | [F1190](FRONTEND.md#f1190)；[hir-lower/src/stmt/exceptions/catches.rs:28](../../compiler/hir-lower/src/stmt/exceptions/catches.rs#L28) |
| <a id="e0810"></a>`E0810` | 读取尚未初始化的值 | 初始化中的 field 或 do-while continue 路径中的局部尚未完成赋值。 | [F0242](FRONTEND.md#f0242)；[F1191](FRONTEND.md#f1191)；[hir-lower/src/class/initialization/fields.rs:335](../../compiler/hir-lower/src/class/initialization/fields.rs#L335) |
| <a id="e0811"></a>`E0811` | iterator 协议不满足 | iterator 返回类型必须实现唯一的 Iterator<T> 应用；缺失与多应用作为原因。 | [F1192](FRONTEND.md#f1192)；[F1193](FRONTEND.md#f1193)；[hir-lower/src/stmt/iteration/head.rs:48](../../compiler/hir-lower/src/stmt/iteration/head.rs#L48) |
| <a id="e0812"></a>`E0812` | smart cast 在所有路径上不成立 | 可变局部的类型细化不能覆盖全部入口路径。 | [F1212](FRONTEND.md#f1212)；[hir-lower/src/stmt/smart_flow/expressions.rs:106](../../compiler/hir-lower/src/stmt/smart_flow/expressions.rs#L106) |
| <a id="e0813"></a>`E0813` | 表达式不能作为该语句 | 当前 statement 位置要求函数调用。 | [F1213](FRONTEND.md#f1213)；[hir-lower/src/stmt/statements.rs:83](../../compiler/hir-lower/src/stmt/statements.rs#L83) |
| <a id="e0814"></a>`E0814` | catch 缺少类型注解 | catch 参数必须显式声明异常类型。 | [F1508](FRONTEND.md#f1508)；[parser/src/stmt/control_flow/exceptions.rs:127](../../compiler/parser/src/stmt/control_flow/exceptions.rs#L127) |

### 09 · 模式与穷尽性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e0901"></a>`E0901` | 裸名称不能作为当前匹配模式 | 需明确 val 绑定或 enum 变体，不凭裸名称推断绑定。 | [F1028](FRONTEND.md#f1028)；[hir-lower/src/patterns.rs:116](../../compiler/hir-lower/src/patterns.rs#L116) |
| <a id="e0902"></a>`E0902` | 不可失败的绑定位置使用可失败模式 | refutable pattern 只允许 when。 | [F1029](FRONTEND.md#f1029)；[hir-lower/src/patterns.rs:130](../../compiler/hir-lower/src/patterns.rs#L130) |
| <a id="e0903"></a>`E0903` | 模式 rest 位置或次数不合法 | 字段 rest 必须最后且最多一次，class component pattern 不允许 rest。 | [F1033](FRONTEND.md#f1033)；[F1499](FRONTEND.md#f1499)；[F1500](FRONTEND.md#f1500)；[hir-lower/src/patterns/binding/aggregates.rs:75](../../compiler/hir-lower/src/patterns/binding/aggregates.rs#L75) |
| <a id="e0904"></a>`E0904` | when 未穷尽 | 缺少具体 pattern、sealed 子类型族，或 subjectless value when 缺少 else；guard 不参与覆盖证明是附注。 | [F1034](FRONTEND.md#f1034)；[F1216](FRONTEND.md#f1216)；[F1221](FRONTEND.md#f1221)；[hir-lower/src/patterns/exhaustiveness.rs:50](../../compiler/hir-lower/src/patterns/exhaustiveness.rs#L50) |
| <a id="e0905"></a>`E0905` | 该类型不能以此形式解构 | intrinsic/pointer 不可解构，匿名字段模式只适用于 struct。 | [F1035](FRONTEND.md#f1035)；[F1036](FRONTEND.md#f1036)；[F1037](FRONTEND.md#f1037)；[hir-lower/src/patterns/structure.rs:25](../../compiler/hir-lower/src/patterns/structure.rs#L25) |
| <a id="e0906"></a>`E0906` | 模式字段数量不符 | 位置模式字段数与目标形状不一致；裸 payload variant 同样需要字段模式。 | [F1038](FRONTEND.md#f1038)；[F1044](FRONTEND.md#f1044)；[hir-lower/src/patterns/structure.rs:151](../../compiler/hir-lower/src/patterns/structure.rs#L151) |
| <a id="e0907"></a>`E0907` | 字段模式遗漏字段 | 未完整列举字段时必须明确用 rest 忽略其余字段。 | [F1039](FRONTEND.md#f1039)；[hir-lower/src/patterns/structure.rs:210](../../compiler/hir-lower/src/patterns/structure.rs#L210) |
| <a id="e0908"></a>`E0908` | 模式字段名称形式不合法 | 字段模式不接受 `_` 名称；忽略值时不能写 val _。 | [F1040](FRONTEND.md#f1040)；[F1498](FRONTEND.md#f1498)；[hir-lower/src/patterns/structure.rs:220](../../compiler/hir-lower/src/patterns/structure.rs#L220) |
| <a id="e0909"></a>`E0909` | 模式重复列举字段 | 同一字段在模式内出现多次。 | [F1041](FRONTEND.md#f1041)；[hir-lower/src/patterns/structure.rs:227](../../compiler/hir-lower/src/patterns/structure.rs#L227) |
| <a id="e0910"></a>`E0910` | 模式形式与变体字段形式不符 | named 与 positional pattern 必须与字段声明形式一致。 | [F1045](FRONTEND.md#f1045)；[F1046](FRONTEND.md#f1046)；[hir-lower/src/patterns/variants.rs:52](../../compiler/hir-lower/src/patterns/variants.rs#L52) |
| <a id="e0911"></a>`E0911` | case subject 类型不允许 | 当前 case 支持的 subject 类型集合不包含该类型。 | [F1218](FRONTEND.md#f1218)；[hir-lower/src/stmt/when/primary.rs:24](../../compiler/hir-lower/src/stmt/when/primary.rs#L24) |
| <a id="e0912"></a>`E0912` | 模式或简写条件缺少 subject | case、is、in、guard 需要 when subject。 | [F1219](FRONTEND.md#f1219)；[F1220](FRONTEND.md#f1220)；[F1509](FRONTEND.md#f1509)；[hir-lower/src/stmt/when/primary.rs:78](../../compiler/hir-lower/src/stmt/when/primary.rs#L78) |
| <a id="e0913"></a>`E0913` | 多条件分支不能同时带 guard | 有 guard 的 when arm 不能同时列多个条件。 | [F1510](FRONTEND.md#f1510)；[parser/src/stmt/when/arms.rs:43](../../compiler/parser/src/stmt/when/arms.rs#L43) |
| <a id="e0914"></a>`E0914` | case arm 只能包含一个模式 | 多个模式应分成独立 arm。 | [F1512](FRONTEND.md#f1512)；[parser/src/stmt/when/conditions.rs:26](../../compiler/parser/src/stmt/when/conditions.rs#L26) |
| <a id="e0915"></a>`E0915` | 匹配模式缺少 case | 模式 arm 需要 case 前缀。 | [F1513](FRONTEND.md#f1513)；[parser/src/stmt/when/conditions.rs:36](../../compiler/parser/src/stmt/when/conditions.rs#L36) |
| <a id="e0916"></a>`E0916` | when subject 绑定形态不合法 | subject 声明必须是非委托 val。 | [F1516](FRONTEND.md#f1516)；[F1517](FRONTEND.md#f1517)；[parser/src/stmt/when/subject.rs:13](../../compiler/parser/src/stmt/when/subject.rs#L13) |

### 10 · 类、继承与初始化

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1001"></a>`E1001` | 构造器委托路径不合法 | secondary 的 this/super、primary 存在性、struct this-only 等由构造器种类决定；具体失败条件是原因。 | [F0133](FRONTEND.md#f0133)；[F0136](FRONTEND.md#f0136)；[F0137](FRONTEND.md#f0137)；[hir-lower/src/class/construction.rs:134](../../compiler/hir-lower/src/class/construction.rs#L134) |
| <a id="e1002"></a>`E1002` | 构造器委托成环 | this 委托链不能形成循环。 | [F0144](FRONTEND.md#f0144)；[hir-lower/src/class/construction/validation.rs:121](../../compiler/hir-lower/src/class/construction/validation.rs#L121) |
| <a id="e1003"></a>`E1003` | 不能显式继承根类型 | 该根 intrinsic type 不能作为源码显式列出的父类型。 | [F0149](FRONTEND.md#f0149)；[hir-lower/src/class/declarations.rs:230](../../compiler/hir-lower/src/class/declarations.rs#L230) |
| <a id="e1004"></a>`E1004` | 父类型种类不合法 | 单 class base、interface-only 位置、value type 不能有 class base 等父类型类别限制。 | [F0150](FRONTEND.md#f0150)；[F0151](FRONTEND.md#f0151)；[F0154](FRONTEND.md#f0154)；[hir-lower/src/class/declarations.rs:246](../../compiler/hir-lower/src/class/declarations.rs#L246) |
| <a id="e1005"></a>`E1005` | 不能继承 final 类型 | 基类不允许派生。 | [F0152](FRONTEND.md#f0152)；[hir-lower/src/class/declarations.rs:263](../../compiler/hir-lower/src/class/declarations.rs#L263) |
| <a id="e1006"></a>`E1006` | 接口继承项不接受构造实参 | interface 继承不是 constructor 调用。 | [F0156](FRONTEND.md#f0156)；[F0153](FRONTEND.md#f0153)；[hir-lower/src/class/declarations.rs:318](../../compiler/hir-lower/src/class/declarations.rs#L318) |
| <a id="e1007"></a>`E1007` | 接口委托声明位置不合法 | 该 owner 类型不允许 interface delegation。 | [F0155](FRONTEND.md#f0155)；[hir-lower/src/class/declarations.rs:312](../../compiler/hir-lower/src/class/declarations.rs#L312) |
| <a id="e1008"></a>`E1008` | 具体类型缺少抽象成员实现 | 类/接口方法或属性义务未履行。 | [F0161](FRONTEND.md#f0161)；[F0164](FRONTEND.md#f0164)；[F0165](FRONTEND.md#f0165)；[hir-lower/src/class/generation.rs:48](../../compiler/hir-lower/src/class/generation.rs#L48) |
| <a id="e1009"></a>`E1009` | 继承图成环 | class/interface 继承关系不能循环。 | [F0162](FRONTEND.md#f0162)；[F0163](FRONTEND.md#f0163)；[hir-lower/src/class/hierarchy.rs:17](../../compiler/hir-lower/src/class/hierarchy.rs#L17) |
| <a id="e1010"></a>`E1010` | 继承实现冲突 | 多个 default/delegate 实现竞争同一成员，需要显式 override；保留各来源位置。 | [F0190](FRONTEND.md#f0190)；[F0215](FRONTEND.md#f0215)；[F0217](FRONTEND.md#f0217)；[hir-lower/src/class/inheritance/delegation/selection.rs:63](../../compiler/hir-lower/src/class/inheritance/delegation/selection.rs#L63) |
| <a id="e1011"></a>`E1011` | override/重列接口条件不覆盖原合同 | 条件应与继承条件等价或覆盖相应域。 | [F0218](FRONTEND.md#f0218)；[F0219](FRONTEND.md#f0219)；[F0231](FRONTEND.md#f0231)；[hir-lower/src/class/inheritance/interfaces/inherited.rs:74](../../compiler/hir-lower/src/class/inheritance/interfaces/inherited.rs#L74) |
| <a id="e1012"></a>`E1012` | override 没有匹配目标 | 该成员未覆盖目标接口/父类的成员。 | [F0221](FRONTEND.md#f0221)；[F0226](FRONTEND.md#f0226)；[F0234](FRONTEND.md#f0234)；[hir-lower/src/class/inheritance/overrides.rs:58](../../compiler/hir-lower/src/class/inheritance/overrides.rs#L58) |
| <a id="e1013"></a>`E1013` | override 签名合同不符 | context 数量顺序精确类型、vararg、修饰符、精确返回/属性类型、val/var 合同必须与被覆盖者匹配。 | [F0169](FRONTEND.md#f0169)；[F0170](FRONTEND.md#f0170)；[F0204](FRONTEND.md#f0204)；[hir-lower/src/class/inheritance/context.rs:29](../../compiler/hir-lower/src/class/inheritance/context.rs#L29) |
| <a id="e1014"></a>`E1014` | 不能覆盖 final 成员 | 被覆盖的方法或属性是 final。 | [F0223](FRONTEND.md#f0223)；[F0224](FRONTEND.md#f0224)；[F0237](FRONTEND.md#f0237)；[hir-lower/src/class/inheritance/overrides.rs:349](../../compiler/hir-lower/src/class/inheritance/overrides.rs#L349) |
| <a id="e1015"></a>`E1015` | 覆盖成员缺少 override | 实际覆盖时必须显式标记 override。 | [F0225](FRONTEND.md#f0225)；[F0238](FRONTEND.md#f0238)；[F1395](FRONTEND.md#f1395)；[hir-lower/src/class/inheritance/overrides.rs:430](../../compiler/hir-lower/src/class/inheritance/overrides.rs#L430) |
| <a id="e1016"></a>`E1016` | 继承成员访问域合同不符 | override 不得缩小访问域，abstract 成员应覆盖继承域，public interface 成员必须显式 public。 | [F0203](FRONTEND.md#f0203)；[F0228](FRONTEND.md#f0228)；[F0229](FRONTEND.md#f0229)；[hir-lower/src/class/inheritance/imported.rs:201](../../compiler/hir-lower/src/class/inheritance/imported.rs#L201) |
| <a id="e1017"></a>`E1017` | 泛型方法声明/覆盖限制 | 泛型类成员的 final、generic interface method、generic override 有既定限制。 | [F0246](FRONTEND.md#f0246)；[F0247](FRONTEND.md#f0247)；[F0248](FRONTEND.md#f0248)；[hir-lower/src/class/methods.rs:36](../../compiler/hir-lower/src/class/methods.rs#L36) |
| <a id="e1018"></a>`E1018` | abstract/open 成员所属类型不允许 | abstract 成员要求 abstract class，open 成员要求允许继承的 class；object 不允许。 | [F0255](FRONTEND.md#f0255)；[F0250](FRONTEND.md#f0250)；[F0251](FRONTEND.md#f0251)；[hir-lower/src/class/methods.rs:231](../../compiler/hir-lower/src/class/methods.rs#L231) |
| <a id="e1019"></a>`E1019` | sealed 派生/构造访问范围不符 | sealed 子类型及构造器必须满足其规定范围。 | [F0256](FRONTEND.md#f0256)；[F0257](FRONTEND.md#f0257)；[hir-lower/src/class/sealed.rs:64](../../compiler/hir-lower/src/class/sealed.rs#L64) |
| <a id="e1020"></a>`E1020` | inner class 所在位置不合法 | inner 需要合法的实例宿主。 | [F0360](FRONTEND.md#f0360)；[hir-lower/src/declarations/class.rs:18](../../compiler/hir-lower/src/declarations/class.rs#L18) |
| <a id="e1021"></a>`E1021` | 重复或冲突的 companion | 同一宿主至多一个 companion，且不得与静态命名空间冲突。 | [F0378](FRONTEND.md#f0378)；[F0379](FRONTEND.md#f0379)；[hir-lower/src/declarations/objects/declaration.rs:55](../../compiler/hir-lower/src/declarations/objects/declaration.rs#L55) |
| <a id="e1022"></a>`E1022` | 条件接口实现重叠 | 两个实现条件可能同时成立且目标冲突。 | [F0268](FRONTEND.md#f0268)；[F0269](FRONTEND.md#f0269)；[hir-lower/src/conditional_interfaces/overlap.rs:23](../../compiler/hir-lower/src/conditional_interfaces/overlap.rs#L23) |
| <a id="e1023"></a>`E1023` | 条件接口证明成环 | 当前条件接口关系验证无法终止于有效证明。 | [F0272](FRONTEND.md#f0272)；[hir-lower/src/conditional_interfaces/recursion.rs:43](../../compiler/hir-lower/src/conditional_interfaces/recursion.rs#L43) |
| <a id="e1024"></a>`E1024` | 条件接口实现的语法/成员限制不满足 | 需要非空 positive where，不接受 context/requires、storage/delegate 等。 | [F1397](FRONTEND.md#f1397)；[F1398](FRONTEND.md#f1398)；[F1399](FRONTEND.md#f1399)；[parser/src/decl/nominals/interface_implementations.rs:38](../../compiler/parser/src/decl/nominals/interface_implementations.rs#L38) |
| <a id="e1025"></a>`E1025` | super 目标或形式不合法 | 必须直接引用规定的 base/direct superinterface 的具体成员；abstract 方法/accessor 不能被 super 调用。 | [F0640](FRONTEND.md#f0640)；[F0670](FRONTEND.md#f0670)；[F0671](FRONTEND.md#f0671)；[hir-lower/src/expr/imported_properties/members.rs:229](../../compiler/hir-lower/src/expr/imported_properties/members.rs#L229) |

### 11 · 效果、上下文与 release

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1101"></a>`E1101` | 操作要求 unsafe 上下文 | unsafe 函数、地址/FFI 等操作或 InteriorMutable 暴露要求显式不安全上下文。 | [F0424](FRONTEND.md#f0424)；[F1016](FRONTEND.md#f1016)；[F1017](FRONTEND.md#f1017)；[hir-lower/src/effects/type_properties.rs:250](../../compiler/hir-lower/src/effects/type_properties.rs#L250) |
| <a id="e1102"></a>`E1102` | unsafe 函数不能存入普通托管函数值 | 当前函数类型不携带 safety 身份，不能抹去 unsafe 调用限制。 | [F0077](FRONTEND.md#f0077)；[hir-lower/src/call_resolution/applicability/references.rs:57](../../compiler/hir-lower/src/call_resolution/applicability/references.rs#L57) |
| <a id="e1103"></a>`E1103` | 当前位置不能调用 suspend 函数 | 普通 callable/初始化等上下文不允许挂起。 | [F0526](FRONTEND.md#f0526)；[F1015](FRONTEND.md#f1015)；[hir-lower/src/expr/callables/values.rs:98](../../compiler/hir-lower/src/expr/callables/values.rs#L98) |
| <a id="e1104"></a>`E1104` | NoGC 中的调用可能分配/转换/抛出 | 直接托管调用、function value、NativeSafe pointer、constructor/requirement 等不满足 NoGC 调用合同。 | [F0410](FRONTEND.md#f0410)；[F0411](FRONTEND.md#f0411)；[F0412](FRONTEND.md#f0412)；[hir-lower/src/effects/calls.rs:92](../../compiler/hir-lower/src/effects/calls.rs#L92) |
| <a id="e1105"></a>`E1105` | NoGC 中的操作要求 GC 或可抛出 | 分配、托管读写、异常、装箱、类型检查、数组、字符串等按实际效果被禁止；操作名/效果为原因。 | [F0428](FRONTEND.md#f0428)；[F0429](FRONTEND.md#f0429)；[F0430](FRONTEND.md#f0430)；[hir-lower/src/effects/violations.rs:26](../../compiler/hir-lower/src/effects/violations.rs#L26) |
| <a id="e1106"></a>`E1106` | NoGC 中的值类型不是 GC-free | 签名、局部、receiver、具体类型实参或表示中包含非 GC-free 值。 | [F0414](FRONTEND.md#f0414)；[F0264](FRONTEND.md#f0264)；[F0399](FRONTEND.md#f0399)；[hir-lower/src/effects/constructors.rs:91](../../compiler/hir-lower/src/effects/constructors.rs#L91) |
| <a id="e1107"></a>`E1107` | 泛型约束不能证明 GC-free | ref kind 等约束与 GC-free 要求冲突。 | [F0419](FRONTEND.md#f0419)；[hir-lower/src/effects/no_gc_generics.rs:97](../../compiler/hir-lower/src/effects/no_gc_generics.rs#L97) |
| <a id="e1108"></a>`E1108` | safe 签名暴露 InteriorMutable | safe 接口不能通过参数/返回/receiver 暴露受限制的内部可变值。 | [F0406](FRONTEND.md#f0406)；[F0407](FRONTEND.md#f0407)；[F0408](FRONTEND.md#f0408)；[hir-lower/src/effects.rs:196](../../compiler/hir-lower/src/effects.rs#L196) |
| <a id="e1109"></a>`E1109` | release 所属类型不合法 | 只能用于普通 final class，Throwable 子类型和其他 owner 不允许。 | [F1139](FRONTEND.md#f1139)；[F1140](FRONTEND.md#f1140)；[F1141](FRONTEND.md#f1141)；[hir-lower/src/release.rs:39](../../compiler/hir-lower/src/release.rs#L39) |
| <a id="e1110"></a>`E1110` | 同一类重复 release | 每个类最多一个 release block。 | [F1138](FRONTEND.md#f1138)；[hir-lower/src/release.rs:31](../../compiler/hir-lower/src/release.rs#L31) |
| <a id="e1111"></a>`E1111` | release 读取非本类普通 backing field | release 只能读取自己的 backing field。 | [F1142](FRONTEND.md#f1142)；[hir-lower/src/release/fields.rs:26](../../compiler/hir-lower/src/release/fields.rs#L26) |
| <a id="e1112"></a>`E1112` | release 不能写 owner field 或取其地址 | 必须先复制到局部值再操作。 | [F0602](FRONTEND.md#f0602)；[F1143](FRONTEND.md#f1143)；[hir-lower/src/expr/generic_calls/intrinsics.rs:43](../../compiler/hir-lower/src/expr/generic_calls/intrinsics.rs#L43) |
| <a id="e1113"></a>`E1113` | release 中不允许该操作 | release 不能逃逸 this、调用不合格目标等；操作种类保留在原因。 | [F0388](FRONTEND.md#f0388)；[F1144](FRONTEND.md#f1144)；[hir-lower/src/defaults/instantiate/expressions.rs:14](../../compiler/hir-lower/src/defaults/instantiate/expressions.rs#L14) |
| <a id="e1114"></a>`E1114` | release 泛型值/调用合同不成立 | 要求 ReleaseValue 或允许的直接 NoTransition，ref 约束不能证明成立。 | [F0421](FRONTEND.md#f0421)；[F0422](FRONTEND.md#f0422)；[F0423](FRONTEND.md#f0423)；[hir-lower/src/effects/release/hooks.rs:43](../../compiler/hir-lower/src/effects/release/hooks.rs#L43) |
| <a id="e1115"></a>`E1115` | context key/binding 类型不合法 | 必须为非 null 托管引用类型。 | [F0574](FRONTEND.md#f0574)；[F0579](FRONTEND.md#f0579)；[hir-lower/src/expr/context.rs:30](../../compiler/hir-lower/src/expr/context.rs#L30) |
| <a id="e1116"></a>`E1116` | 重复 context key | 包括泛型替换后产生的重复 key。 | [F0259](FRONTEND.md#f0259)；[F0575](FRONTEND.md#f0575)；[hir-lower/src/concretize/context.rs:46](../../compiler/hir-lower/src/concretize/context.rs#L46) |
| <a id="e1117"></a>`E1117` | context 声明/绑定形状不合法 | prefix 位置/次数、局部声明种类、默认值或 scope 实参数量不符合规定。 | [F1360](FRONTEND.md#f1360)；[F1361](FRONTEND.md#f1361)；[F1421](FRONTEND.md#f1421)；[parser/src/decl/context.rs:61](../../compiler/parser/src/decl/context.rs#L61) |
| <a id="e1118"></a>`E1118` | 效果注解组合不合法 | Safe/Unsafe 互斥，NoGC 与 suspend 不兼容，contextual 声明及 requirement 的效果有限制。 | [F0044](FRONTEND.md#f0044)；[F0059](FRONTEND.md#f0059)；[F0577](FRONTEND.md#f0577)；[hir-lower/src/annotations/constructors.rs:34](../../compiler/hir-lower/src/annotations/constructors.rs#L34) |

### 12 · FFI、原子与原始存储

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1201"></a>`E1201` | 类型不能用于选定 FFI ABI | 参数/返回/field/指针/回调类型不能按 C 或 Scoop native ABI 表示；Unit 位置、managed ref、tuple、Option、零尺寸 pointee 等为具体原因。 | [F0425](FRONTEND.md#f0425)；[F0808](FRONTEND.md#f0808)；[F0809](FRONTEND.md#f0809)；[hir-lower/src/effects/type_validation.rs:90](../../compiler/hir-lower/src/effects/type_validation.rs#L90) |
| <a id="e1202"></a>`E1202` | extern 符号冲突 | 两个源声明要求不相容的同名 native symbol。 | [F0812](FRONTEND.md#f0812)；[F0813](FRONTEND.md#f0813)；[F0817](FRONTEND.md#f0817)；[hir-lower/src/ffi.rs:182](../../compiler/hir-lower/src/ffi.rs#L182) |
| <a id="e1203"></a>`E1203` | Extern 声明形态不合法 | 要求顶层、非泛型、非 suspend、无 receiver/body，extern global 无 initializer 且仅 C data ABI。 | [F0015](FRONTEND.md#f0015)；[F0016](FRONTEND.md#f0016)；[F0060](FRONTEND.md#f0060)；[hir-lower/src/annotations.rs:98](../../compiler/hir-lower/src/annotations.rs#L98) |
| <a id="e1204"></a>`E1204` | 不支持的 native ABI/调用约定 | ABI、calling convention 名称不在支持集合。 | [F0054](FRONTEND.md#f0054)；[F0057](FRONTEND.md#f0057)；[hir-lower/src/annotations/externs.rs:112](../../compiler/hir-lower/src/annotations/externs.rs#L112) |
| <a id="e1205"></a>`E1205` | 调用约定注解使用条件不符 | 声明位置、NoGC、extern/suspend 条件不满足。 | [F0068](FRONTEND.md#f0068)；[F0069](FRONTEND.md#f0069)；[F0070](FRONTEND.md#f0070)；[hir-lower/src/annotations/functions.rs:205](../../compiler/hir-lower/src/annotations/functions.rs#L205) |
| <a id="e1206"></a>`E1206` | native 库或符号名称不合法 | library 必须是逻辑库名，symbol 必须是可移植 C 标识符。 | [F0052](FRONTEND.md#f0052)；[F0053](FRONTEND.md#f0053)；[hir-lower/src/annotations/externs.rs:93](../../compiler/hir-lower/src/annotations/externs.rs#L93) |
| <a id="e1207"></a>`E1207` | CLayout 对齐值无效 | CLayout 的 alignment/pack 数值必须来自允许集合。 | [F0043](FRONTEND.md#f0043)；[hir-lower/src/annotations/arguments.rs:120](../../compiler/hir-lower/src/annotations/arguments.rs#L120) |
| <a id="e1208"></a>`E1208` | raw/extern 存储方式不合法 | Global/ThreadLocal 互斥，raw local 必须 mutable，extern var/val 的 storage 注解规则不同。仅限 allocate_raw_global 路径，不禁止普通顶层 val/var。 | [F0013](FRONTEND.md#f0013)；[F0017](FRONTEND.md#f0017)；[F0018](FRONTEND.md#f0018)；[hir-lower/src/annotations.rs:80](../../compiler/hir-lower/src/annotations.rs#L80) |
| <a id="e1209"></a>`E1209` | raw/extern 属性形态不合法 | 不接受 delegate、Option 省略初始化或 accessor。 | [F0889](FRONTEND.md#f0889)；[F0890](FRONTEND.md#f0890)；[F0891](FRONTEND.md#f0891)；[hir-lower/src/globals/raw.rs:17](../../compiler/hir-lower/src/globals/raw.rs#L17) |
| <a id="e1210"></a>`E1210` | raw 类型没有合法全零初始映像 | 存储初始化要求该类型可表达全零值。 | [F0892](FRONTEND.md#f0892)；[hir-lower/src/globals/raw.rs:68](../../compiler/hir-lower/src/globals/raw.rs#L68) |
| <a id="e1211"></a>`E1211` | raw 存储类型不是具体 GC-free 值 | 原始全局存储不接受引用或未具体化类型。 | [F0894](FRONTEND.md#f0894)；[hir-lower/src/globals/raw.rs:166](../../compiler/hir-lower/src/globals/raw.rs#L166) |
| <a id="e1212"></a>`E1212` | raw 初始化不是合格常量 | 必须是 GC-free 编译期常量，不可调用普通函数或读取其他全局。 | [F0021](FRONTEND.md#f0021)；[F0895](FRONTEND.md#f0895)；[hir-lower/src/annotations.rs:138](../../compiler/hir-lower/src/annotations.rs#L138) |
| <a id="e1213"></a>`E1213` | Ptr/addressOf 要求 GC-free 值 | pointee/取址目标包含托管状态。 | [F0416](FRONTEND.md#f0416)；[F0417](FRONTEND.md#f0417)；[F0563](FRONTEND.md#f0563)；[hir-lower/src/effects/gc_free_pointees/validation.rs:59](../../compiler/hir-lower/src/effects/gc_free_pointees/validation.rs#L59) |
| <a id="e1214"></a>`E1214` | addressOf 实参不可取址 | 实参必须是可取址的 local/parameter/global/value this。 | [F0603](FRONTEND.md#f0603)；[F0746](FRONTEND.md#f0746)；[hir-lower/src/expr/generic_calls/intrinsics.rs:86](../../compiler/hir-lower/src/expr/generic_calls/intrinsics.rs#L86) |
| <a id="e1215"></a>`E1215` | 非空 Ptr 的原始地址为零 | Ptr raw address 不允许零；nullability 使用 Option 表达。 | [F0569](FRONTEND.md#f0569)；[hir-lower/src/expr/constructors/ffi.rs:143](../../compiler/hir-lower/src/expr/constructors/ffi.rs#L143) |
| <a id="e1216"></a>`E1216` | native 函数指针类型参数不合法 | FunPtr/GCLeafFunPtr/ForeignCallback 需要规定的普通、具体函数类型。 | [F0529](FRONTEND.md#f0529)；[F0532](FRONTEND.md#f0532)；[F0533](FRONTEND.md#f0533)；[hir-lower/src/expr/callbacks.rs:96](../../compiler/hir-lower/src/expr/callbacks.rs#L96) |
| <a id="e1217"></a>`E1217` | native 函数地址目标不合格 | 只能选择满足精确签名、顶层非泛型非 suspend、有源码 body 且 NoGC 的目标；每个拒绝原因不另分主码。 | [F0486](FRONTEND.md#f0486)；[F0502](FRONTEND.md#f0502)；[F0504](FRONTEND.md#f0504)；[hir-lower/src/expr/callable_references.rs:89](../../compiler/hir-lower/src/expr/callable_references.rs#L89) |
| <a id="e1218"></a>`E1218` | 回调 mode 不是允许的常量 | 只能为 Reusable 或 OneShot。 | [F0531](FRONTEND.md#f0531)；[hir-lower/src/expr/callbacks/registration.rs:55](../../compiler/hir-lower/src/expr/callbacks/registration.rs#L55) |
| <a id="e1219"></a>`E1219` | 回调 context 参数位置或类型无效 | contextIndex 必须为有效常量索引，对应参数必须精确 Ptr<Unit>。 | [F0534](FRONTEND.md#f0534)；[F0535](FRONTEND.md#f0535)；[F0536](FRONTEND.md#f0536)；[hir-lower/src/expr/callbacks/signature.rs:33](../../compiler/hir-lower/src/expr/callbacks/signature.rs#L33) |
| <a id="e1220"></a>`E1220` | captureErrno 合同不满足 | 仅 C extern function，可用返回形状必须为 (R, Int)。 | [F0833](FRONTEND.md#f0833)；[F0834](FRONTEND.md#f0834)；[F0835](FRONTEND.md#f0835)；[hir-lower/src/ffi/errno.rs:39](../../compiler/hir-lower/src/ffi/errno.rs#L39) |
| <a id="e1221"></a>`E1221` | 原子内存序不是编译期常量 | 内存序必须是常量 MemoryOrder variant。 | [F0470](FRONTEND.md#f0470)；[hir-lower/src/expr/atomics.rs:133](../../compiler/hir-lower/src/expr/atomics.rs#L133) |
| <a id="e1222"></a>`E1222` | 原子操作不允许该内存序 | load/store 可用内存序与 CAS success/failure 组合按操作规则判定。 | [F0467](FRONTEND.md#f0467)；[F0468](FRONTEND.md#f0468)；[F0469](FRONTEND.md#f0469)；[hir-lower/src/expr/atomics.rs:26](../../compiler/hir-lower/src/expr/atomics.rs#L26) |
| <a id="e1223"></a>`E1223` | 带内存序的原子方法不能直接取引用 | 需 lambda 固定 order 后再形成函数值。 | [F0466](FRONTEND.md#f0466)；[hir-lower/src/expr/atomics.rs:3](../../compiler/hir-lower/src/expr/atomics.rs#L3) |

### 13 · 注解与自动派生

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1301"></a>`E1301` | 注解目标或 use-site 不合法 | 注解不适用于该声明/类型位置，use-site 名称或落点无效。 | [F0014](FRONTEND.md#f0014)；[F0022](FRONTEND.md#f0022)；[F0023](FRONTEND.md#f0023)；[hir-lower/src/annotations.rs:89](../../compiler/hir-lower/src/annotations.rs#L89) |
| <a id="e1302"></a>`E1302` | 非 repeatable 注解被重复使用 | 包括内建 marker 与普通用户注解。 | [F0012](FRONTEND.md#f0012)；[F0045](FRONTEND.md#f0045)；[F1325](FRONTEND.md#f1325)；[hir-lower/src/annotations.rs:69](../../compiler/hir-lower/src/annotations.rs#L69) |
| <a id="e1303"></a>`E1303` | 注解组合冲突 | Intrinsic 与不允许共存的其他注解/Extern 冲突，Serialize ignore 与其他选项冲突。 | [F0024](FRONTEND.md#f0024)；[F0073](FRONTEND.md#f0073)；[F1312](FRONTEND.md#f1312)；[hir-lower/src/annotations.rs:207](../../compiler/hir-lower/src/annotations.rs#L207) |
| <a id="e1304"></a>`E1304` | 注解值不是允许的常量形式 | 数组/嵌套注解/静态 enum variant/const val 及标量字面量形式有要求。 | [F0042](FRONTEND.md#f0042)；[F1311](FRONTEND.md#f1311)；[F1319](FRONTEND.md#f1319)；[hir-lower/src/annotations/arguments.rs:113](../../compiler/hir-lower/src/annotations/arguments.rs#L113) |
| <a id="e1305"></a>`E1305` | 注解参数类型不合法 | 只允许标量、integral enum、annotation 或这些类型的一维 core Array。 | [F1339](FRONTEND.md#f1339)；[F1340](FRONTEND.md#f1340)；[hir-lower/src/user_annotations/types.rs:88](../../compiler/hir-lower/src/user_annotations/types.rs#L88) |
| <a id="e1306"></a>`E1306` | 注解参数、默认值或类型注解循环 | 参数/default 依赖或类型注解展开成环。 | [F1329](FRONTEND.md#f1329)；[F1338](FRONTEND.md#f1338)；[hir-lower/src/user_annotations/schemas.rs:63](../../compiler/hir-lower/src/user_annotations/schemas.rs#L63) |
| <a id="e1307"></a>`E1307` | 注解声明形状不合法 | annotation class 不允许 supertypes/body。 | [F1353](FRONTEND.md#f1353)；[parser/src/decl/annotations/declaration.rs:62](../../compiler/parser/src/decl/annotations/declaration.rs#L62) |
| <a id="e1308"></a>`E1308` | Target 注解的目标集合不合法 | 需要 AnnotationTarget 数组，元素必须已知且不重复。 | [F1326](FRONTEND.md#f1326)；[F1327](FRONTEND.md#f1327)；[F1328](FRONTEND.md#f1328)；[hir-lower/src/user_annotations/rules/declarations.rs:22](../../compiler/hir-lower/src/user_annotations/rules/declarations.rs#L22) |
| <a id="e1309"></a>`E1309` | Serialize 选项不适用于该目标 | enum variant 只能 rename，emitNull 需要 Option field。 | [F1333](FRONTEND.md#f1333)；[F1334](FRONTEND.md#f1334)；[hir-lower/src/user_annotations/serialization.rs:44](../../compiler/hir-lower/src/user_annotations/serialization.rs#L44) |
| <a id="e1310"></a>`E1310` | 类型不满足自动派生的形状要求 | encode/decode/inspect 的允许目标、final/no-base/primary constructor、intrinsic 显式实现要求；派生种类与不符条件作为原因。 | [F0172](FRONTEND.md#f0172)；[F0173](FRONTEND.md#f0173)；[F0174](FRONTEND.md#f0174)；[hir-lower/src/class/inheritance/decoding.rs:124](../../compiler/hir-lower/src/class/inheritance/decoding.rs#L124) |
| <a id="e1311"></a>`E1311` | 字段无法按默认规则自动派生 | 非 primary storage、custom accessor、delegate 等需要明确 Serialize(ignore) 或显式实现；保留 encode/decode/inspect 的差异原因。 | [F0175](FRONTEND.md#f0175)；[F0176](FRONTEND.md#f0176)；[F0177](FRONTEND.md#f0177)；[hir-lower/src/class/inheritance/decoding/classes.rs:87](../../compiler/hir-lower/src/class/inheritance/decoding/classes.rs#L87) |
| <a id="e1312"></a>`E1312` | 自动派生找不到字段 codec | 字段缺少合格 primary val dependency、companion 或显式实现。 | [F0168](FRONTEND.md#f0168)；[hir-lower/src/class/inheritance/coding/codecs.rs:136](../../compiler/hir-lower/src/class/inheritance/coding/codecs.rs#L136) |
| <a id="e1313"></a>`E1313` | 自动派生有多个字段 codec 候选 | 多个 primary val dependency 同时适用。 | [F0166](FRONTEND.md#f0166)；[hir-lower/src/class/inheritance/coding/codecs.rs:41](../../compiler/hir-lower/src/class/inheritance/coding/codecs.rs#L41) |
| <a id="e1314"></a>`E1314` | 派生序列化名称重复 | 字段/变体的最终 wire name 重复。 | [F0181](FRONTEND.md#f0181)；[F0187](FRONTEND.md#f0187)；[F0200](FRONTEND.md#f0200)；[hir-lower/src/class/inheritance/decoding/records.rs:27](../../compiler/hir-lower/src/class/inheritance/decoding/records.rs#L27) |
| <a id="e1315"></a>`E1315` | 省略解码参数却没有默认值 | 自动 decode 的可省略 constructor parameter 必须有声明默认值。 | [F0182](FRONTEND.md#f0182)；[hir-lower/src/class/inheritance/decoding/records.rs:32](../../compiler/hir-lower/src/class/inheritance/decoding/records.rs#L32) |
| <a id="e1316"></a>`E1316` | 字段无法派生相等比较 | 字段没有可用 equals 或存在歧义；外层 derive equals 是组合主诊断，字段候选为原因。 | [F0794](FRONTEND.md#f0794)；[F0796](FRONTEND.md#f0796)；[F0797](FRONTEND.md#f0797)；[hir-lower/src/expr/operators/equality.rs:181](../../compiler/hir-lower/src/expr/operators/equality.rs#L181) |
| <a id="e1317"></a>`E1317` | GenerateWith 配置不合法 | 名称/参数/接口应用/目标类型不符合生成器选择规则。 | [D0020](BUILD.md#d0020)；[D0021](BUILD.md#d0021)；[D0022](BUILD.md#d0022)；[driver/src/generation/configuration.rs:64](../../compiler/driver/src/generation/configuration.rs#L64) |
| <a id="e1318"></a>`E1318` | 生成器重复绑定同一目标 | 重复 interface application 或多个 generator 选择同一待实现 method。 | [D0041](BUILD.md#d0041)；[D0042](BUILD.md#d0042)；[driver/src/generation/plan.rs:106](../../compiler/driver/src/generation/plan.rs#L106) |

### 14 · core 与编译器内建声明

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1401"></a>`E1401` | 未知 intrinsic 标识 | 内建类型/函数 intrinsic 名不在编译器注册表。 | [F0027](FRONTEND.md#f0027)；[F0055](FRONTEND.md#f0055)；[hir-lower/src/annotations.rs:264](../../compiler/hir-lower/src/annotations.rs#L264) |
| <a id="e1402"></a>`E1402` | intrinsic 声明位置不合法 | Intrinsic 只允许在规定的 core 位置、目标及非嵌套声明使用。 | [F0029](FRONTEND.md#f0029)；[F0056](FRONTEND.md#f0056)；[F0361](FRONTEND.md#f0361)；[hir-lower/src/annotations.rs:283](../../compiler/hir-lower/src/annotations.rs#L283) |
| <a id="e1403"></a>`E1403` | core/intrinsic 声明合同不符 | 源码 core 的内建声明必须满足固定形状、签名、效果和表示；具体协议/成员和失败条件作为附加原因，不能仅因含 core 就定为 ICE。 | [F0028](FRONTEND.md#f0028)；[F0071](FRONTEND.md#f0071)；[F0072](FRONTEND.md#f0072)；[hir-lower/src/annotations.rs:271](../../compiler/hir-lower/src/annotations.rs#L271) |
| <a id="e1404"></a>`E1404` | 缺少所需 core 协议声明 | 导入的 core 中缺少或不能选择语言糖/派生所需协议；坏产物解码原因转产物码，当前生成器违约转 ICE。 | [F0286](FRONTEND.md#f0286)；[F0292](FRONTEND.md#f0292)；[F0296](FRONTEND.md#f0296)；[hir-lower/src/core_contract/common.rs:17](../../compiler/hir-lower/src/core_contract/common.rs#L17) |
| <a id="e1405"></a>`E1405` | annotationKey 参数不是声明查询 | 必须显式指定一个 annotation declaration，不能以普通 annotated type use 代替。 | [F0763](FRONTEND.md#f0763)；[F0765](FRONTEND.md#f0765)；[F0766](FRONTEND.md#f0766)；[hir-lower/src/expr/named_calls/nominals/annotation_keys.rs:50](../../compiler/hir-lower/src/expr/named_calls/nominals/annotation_keys.rs#L50) |
| <a id="e1406"></a>`E1406` | intrinsic 语义身份重复绑定 | 同一内建函数/类型标识由多个声明或 provider 重复定义。 | [F0312](FRONTEND.md#f0312)；[F0316](FRONTEND.md#f0316)；[hir-lower/src/core_contract/intrinsics.rs:16](../../compiler/hir-lower/src/core_contract/intrinsics.rs#L16) |

### 15 · 属性、常量与值语义

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1501"></a>`E1501` | 不能写入不可变值 | val、构造参数、backing field 和值类型 field 都不允许写入。 | [F0635](FRONTEND.md#f0635)；[F1172](FRONTEND.md#f1172)；[F1173](FRONTEND.md#f1173)；[hir-lower/src/expr/imported_properties/extension/generic.rs:97](../../compiler/hir-lower/src/expr/imported_properties/extension/generic.rs#L97) |
| <a id="e1502"></a>`E1502` | 不能捕获可变局部变量 | 要求 val 快照或显式引用状态。 | [F0537](FRONTEND.md#f0537)；[hir-lower/src/expr/captures.rs:152](../../compiler/hir-lower/src/expr/captures.rs#L152) |
| <a id="e1503"></a>`E1503` | 该局部类型不能捕获运行时值 | 只有允许的 local class instance 可捕获。 | [F1012](FRONTEND.md#f1012)；[hir-lower/src/local_types/captures.rs:79](../../compiler/hir-lower/src/local_types/captures.rs#L79) |
| <a id="e1504"></a>`E1504` | copy update 目标不是精确 struct 值 | 包括 Option safe-navigation 不能直接用于 copy update。 | [F0580](FRONTEND.md#f0580)；[F1458](FRONTEND.md#f1458)；[hir-lower/src/expr/copy_updates.rs:53](../../compiler/hir-lower/src/expr/copy_updates.rs#L53) |
| <a id="e1505"></a>`E1505` | copy update 重复字段 | 同一字段被多次指定。 | [F0581](FRONTEND.md#f0581)；[hir-lower/src/expr/copy_updates.rs:69](../../compiler/hir-lower/src/expr/copy_updates.rs#L69) |
| <a id="e1506"></a>`E1506` | 写入目标不可赋值 | update、赋值或复合赋值回写要求 writable place，禁止通过 safe navigation 赋值。 | [F1185](FRONTEND.md#f1185)；[F1209](FRONTEND.md#f1209)；[F1211](FRONTEND.md#f1211)；[hir-lower/src/stmt/compound_assignments.rs:141](../../compiler/hir-lower/src/stmt/compound_assignments.rs#L141) |
| <a id="e1507"></a>`E1507` | 值类型属性形态不合法 | value type 属性必须 immutable/computed val，不能满足 mutable interface property。 | [F0214](FRONTEND.md#f0214)；[F1117](FRONTEND.md#f1117)；[F1118](FRONTEND.md#f1118)；[hir-lower/src/class/inheritance/interfaces.rs:109](../../compiler/hir-lower/src/class/inheritance/interfaces.rs#L109) |
| <a id="e1508"></a>`E1508` | 属性缺少可推断类型的来源 | 省略属性类型时必须有能确定类型的 initializer/accessor 等。 | [F0354](FRONTEND.md#f0354)；[hir-lower/src/declaration_inference/properties.rs:36](../../compiler/hir-lower/src/declaration_inference/properties.rs#L36) |
| <a id="e1509"></a>`E1509` | 无初始化值的属性不符合 Option 简写 | 只有规定的 mutable Option property 可以省略初始化。 | [F0896](FRONTEND.md#f0896)；[hir-lower/src/globals/storage.rs:24](../../compiler/hir-lower/src/globals/storage.rs#L24) |
| <a id="e1510"></a>`E1510` | computed 属性缺少 accessor body | val 需要 getter，var 还需要 setter。 | [F1120](FRONTEND.md#f1120)；[F1122](FRONTEND.md#f1122)；[F1131](FRONTEND.md#f1131)；[hir-lower/src/properties/declarations/accessors.rs:132](../../compiler/hir-lower/src/properties/declarations/accessors.rs#L132) |
| <a id="e1511"></a>`E1511` | 不可变属性声明了 setter | val 不允许 setter。 | [F1121](FRONTEND.md#f1121)；[hir-lower/src/properties/declarations/accessors.rs:154](../../compiler/hir-lower/src/properties/declarations/accessors.rs#L154) |
| <a id="e1512"></a>`E1512` | accessor 可见性不合法 | setter 不得比 property 更可见，getter 不能单独设置可见性。 | [F1113](FRONTEND.md#f1113)；[F1445](FRONTEND.md#f1445)；[hir-lower/src/properties/access.rs:55](../../compiler/hir-lower/src/properties/access.rs#L55) |
| <a id="e1513"></a>`E1513` | 重复 accessor | 同一 property 有多个 getter 或 setter。 | [F1444](FRONTEND.md#f1444)；[F1446](FRONTEND.md#f1446)；[parser/src/decl/properties.rs:255](../../compiler/parser/src/decl/properties.rs#L255) |
| <a id="e1514"></a>`E1514` | field 使用位置不合法 | field 只能在直接 accessor body 中使用。 | [F1112](FRONTEND.md#f1112)；[hir-lower/src/properties.rs:391](../../compiler/hir-lower/src/properties.rs#L391) |
| <a id="e1515"></a>`E1515` | 该属性位置不允许存储或委托 | extension 必须 computed/delegated；interface/contextual property 不允许 backing storage 等。 | [F0884](FRONTEND.md#f0884)；[F1130](FRONTEND.md#f1130)；[F1443](FRONTEND.md#f1443)；[hir-lower/src/globals/extensions.rs:26](../../compiler/hir-lower/src/globals/extensions.rs#L26) |
| <a id="e1516"></a>`E1516` | 实例属性不能是 const | const 不允许普通 class instance。 | [F1127](FRONTEND.md#f1127)；[hir-lower/src/properties/declarations/classes.rs:268](../../compiler/hir-lower/src/properties/declarations/classes.rs#L268) |
| <a id="e1517"></a>`E1517` | const 依赖成环 | 常量求值依赖关系不能循环。 | [F0848](FRONTEND.md#f0848)；[hir-lower/src/globals/consts.rs:167](../../compiler/hir-lower/src/globals/consts.rs#L167) |
| <a id="e1518"></a>`E1518` | 表达式不是允许的编译期常量 | 只允许现有规范内的 literal/const 引用/精确 core primitive/input intrinsic；不能把普通调用标成常量。 | [F0608](FRONTEND.md#f0608)；[F0852](FRONTEND.md#f0852)；[F0858](FRONTEND.md#f0858)；[hir-lower/src/expr/imported_constants.rs:35](../../compiler/hir-lower/src/expr/imported_constants.rs#L35) |
| <a id="e1519"></a>`E1519` | 常量求值除零 | 整数常量除法/求余的除数为零。 | [F0857](FRONTEND.md#f0857)；[hir-lower/src/globals/consts/binary.rs:148](../../compiler/hir-lower/src/globals/consts/binary.rs#L148) |
| <a id="e1520"></a>`E1520` | tuple 常量元素数量不符 | 常量 tuple 与声明 tuple 类型的长度不符。 | [F0853](FRONTEND.md#f0853)；[hir-lower/src/globals/consts/aggregate.rs:49](../../compiler/hir-lower/src/globals/consts/aggregate.rs#L49) |
| <a id="e1521"></a>`E1521` | integral enum 值声明不合法 | 要求显式 integer constant、不能带 payload；显式值需要整数父类型。 | [F0860](FRONTEND.md#f0860)；[F1165](FRONTEND.md#f1165)；[F1166](FRONTEND.md#f1166)；[hir-lower/src/globals/consts/enumerations.rs:46](../../compiler/hir-lower/src/globals/consts/enumerations.rs#L46) |
| <a id="e1522"></a>`E1522` | integral enum 值重复 | 两个 variant 具有相同整数值。 | [F0862](FRONTEND.md#f0862)；[hir-lower/src/globals/consts/enumerations.rs:62](../../compiler/hir-lower/src/globals/consts/enumerations.rs#L62) |
| <a id="e1523"></a>`E1523` | enum 不允许声明指定自有运算符 | 当前 enum 的保留运算符合同不允许该声明。 | [F0374](FRONTEND.md#f0374)；[hir-lower/src/declarations/enumeration/integral.rs:80](../../compiler/hir-lower/src/declarations/enumeration/integral.rs#L80) |
| <a id="e1524"></a>`E1524` | 运行时初始化依赖循环 | top-level/object 等 initialization unit 依赖形成环。 | [F0888](FRONTEND.md#f0888)；[hir-lower/src/globals/initialization.rs:333](../../compiler/hir-lower/src/globals/initialization.rs#L333) |
| <a id="e1525"></a>`E1525` | 缺少合格的属性委托 operator | delegate 类型不能提供所需 getValue/setValue/provideDelegate 角色。 | [F1132](FRONTEND.md#f1132)；[hir-lower/src/properties/delegates.rs:270](../../compiler/hir-lower/src/properties/delegates.rs#L270) |
| <a id="e1526"></a>`E1526` | 局部委托属性不是简单名称 | 局部 delegated property 不允许解构等绑定形态。 | [F1507](FRONTEND.md#f1507)；[parser/src/stmt.rs:291](../../compiler/parser/src/stmt.rs#L291) |

### 16 · 编译期输入

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1601"></a>`E1601` | 编译期输入参数不是要求的常量 | env/includeStr/includeBytes/includeDir 路径、名称及过滤数组必须为对应的编译期 String 常量。 | [F0541](FRONTEND.md#f0541)；[F0542](FRONTEND.md#f0542)；[F0543](FRONTEND.md#f0543)；[hir-lower/src/expr/compiler_inputs.rs:54](../../compiler/hir-lower/src/expr/compiler_inputs.rs#L54) |
| <a id="e1602"></a>`E1602` | includeStr 输入不是 UTF-8 | 被包含文件不能解码为字符串。 | [F0546](FRONTEND.md#f0546)；[hir-lower/src/expr/compiler_inputs.rs:175](../../compiler/hir-lower/src/expr/compiler_inputs.rs#L175) |
| <a id="e1603"></a>`E1603` | includeDir pattern 无效 | include/exclude glob 无法按当前规则解析。 | [F0549](FRONTEND.md#f0549)；[F0550](FRONTEND.md#f0550)；[D0528](BUILD.md#d0528)；[hir-lower/src/expr/compiler_inputs/directory.rs:54](../../compiler/hir-lower/src/expr/compiler_inputs/directory.rs#L54) |
| <a id="e1604"></a>`E1604` | 当前入口未提供编译期输入能力 | 调用 env/include* 的入口没有输入 host；不要把它归为源程序类型错误。 | [F0544](FRONTEND.md#f0544)；[F0545](FRONTEND.md#f0545)；[F0548](FRONTEND.md#f0548)；[hir-lower/src/expr/compiler_inputs.rs:86](../../compiler/hir-lower/src/expr/compiler_inputs.rs#L86) |
| <a id="e1605"></a>`E1605` | 编译期输入路径不合法 | include 路径必须非空、无 NUL；文件/目录种类、相对位置、循环及路径编码也须满足现有输入规则，不新增路径来源审计。 | [D0518](BUILD.md#d0518)；[D0519](BUILD.md#d0519)；[D0520](BUILD.md#d0520)；[toolchain/src/compiler_inputs/directories.rs:94](../../compiler/toolchain/src/compiler_inputs/directories.rs#L94) |
| <a id="e1606"></a>`E1606` | 编译期环境变量不是 Unicode | env 的主机输入不能转换为 Unicode。 | [D0517](BUILD.md#d0517)；[toolchain/src/compiler_inputs.rs:88](../../compiler/toolchain/src/compiler_inputs.rs#L88) |
| <a id="e1607"></a>`E1607` | 编译期环境变量名称不合法 | env 的名称必须以 SCOOP_ 开头，且不能包含等号或 NUL。 | [A0008](ARTIFACTS.md#a0008)；[A0009](ARTIFACTS.md#a0009)；[hir/src/compiler_inputs.rs:10](../../compiler/hir/src/compiler_inputs.rs#L10) |

### 17 · 入口与测试声明

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1701"></a>`E1701` | 存在多个合法入口 | 指定产物种类只能有一个合格入口。 | [F1018](FRONTEND.md#f1018)；[hir-lower/src/output_kind.rs:67](../../compiler/hir-lower/src/output_kind.rs#L67) |
| <a id="e1702"></a>`E1702` | 没有合法入口 | 没有名字/签名/效果满足当前产物种类的入口；不合格候选的具体原因附在此主诊断下。 | [F1022](FRONTEND.md#f1022)；[F1023](FRONTEND.md#f1023)；[hir-lower/src/output_kind.rs:268](../../compiler/hir-lower/src/output_kind.rs#L268) |
| <a id="e1703"></a>`E1703` | 测试注解出现在非测试源码 | Test/BeforeTest/AfterTest/Ignore 等只在测试 source 中可用。 | [F1224](FRONTEND.md#f1224)；[hir-lower/src/test_annotations.rs:26](../../compiler/hir-lower/src/test_annotations.rs#L26) |
| <a id="e1704"></a>`E1704` | 测试容器不符合规定形状 | 必须为顶层非泛型具体 class。 | [F1225](FRONTEND.md#f1225)；[hir-lower/src/test_annotations.rs:65](../../compiler/hir-lower/src/test_annotations.rs#L65) |
| <a id="e1705"></a>`E1705` | 测试注解组合不合法 | Ignore 需同方法 Test；Test/BeforeTest/AfterTest 互斥。 | [F1226](FRONTEND.md#f1226)；[F1227](FRONTEND.md#f1227)；[hir-lower/src/test_annotations.rs:100](../../compiler/hir-lower/src/test_annotations.rs#L100) |
| <a id="e1706"></a>`E1706` | 测试方法不在直接 Test class 内 | 测试/生命周期方法必须直接属于 Test class。 | [F1228](FRONTEND.md#f1228)；[hir-lower/src/test_annotations.rs:115](../../compiler/hir-lower/src/test_annotations.rs#L115) |
| <a id="e1707"></a>`E1707` | 测试/生命周期方法签名不合格 | 普通 safe 同步实例方法，无 value/type/context 参数，Unit 返回，有 Scoop body；九条具体拒绝理由只作原因。 | [F1229](FRONTEND.md#f1229)；[F1231](FRONTEND.md#f1231)；[F1232](FRONTEND.md#f1232)；[hir-lower/src/test_annotations.rs:121](../../compiler/hir-lower/src/test_annotations.rs#L121) |
| <a id="e1708"></a>`E1708` | 重复测试生命周期方法 | 同一容器同种 BeforeTest/AfterTest 至多一个。 | [F1230](FRONTEND.md#f1230)；[hir-lower/src/test_annotations.rs:151](../../compiler/hir-lower/src/test_annotations.rs#L151) |

### 18 · 文档

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e1801"></a>`E1801` | 文档资源或站点配置无效 | 显式 README 缺失、资源 URL 非法或发布的 API 结构/schema/target/coordinate 不符。实际 I/O 用 E2201，输出路径用 E2004，当前自产站点关系不一致用 ICE2501。 | [D0369](BUILD.md#d0369)；[D0370](BUILD.md#d0370)；[T0040](TOOLS.md#t0040)；[scoop/src/document/overview.rs:43](../../compiler/scoop/src/document/overview.rs#L43) |

### 20 · 命令、项目与源码输入

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e2001"></a>`E2001` | 命令参数或操作组合不合法 | CLI mode/operand/selection 与当前操作不兼容；具体选项和值作原因。 | [D0048](BUILD.md#d0048)；[D0049](BUILD.md#d0049)；[D0050](BUILD.md#d0050)；[driver/src/request.rs:242](../../compiler/driver/src/request.rs#L242) |
| <a id="e2002"></a>`E2002` | 输入路径或文件种类不符合入口要求 | Cone 目录/Cone.toml/.scoop/regular file 要求不满足；普通读取失败另列 I/O。 | [D0118](BUILD.md#d0118)；[D0119](BUILD.md#d0119)；[D0120](BUILD.md#d0120)；[manifest/src/discovery/error.rs:87](../../compiler/manifest/src/discovery/error.rs#L87) |
| <a id="e2003"></a>`E2003` | 源码或清单不是 UTF-8 | 输入源码/Cone.toml 解码失败。 | [D0125](BUILD.md#d0125)；[D0182](BUILD.md#d0182)；[manifest/src/discovery/error.rs:113](../../compiler/manifest/src/discovery/error.rs#L113) |
| <a id="e2004"></a>`E2004` | 没有选择到源码 | 当前编译/分析 Cone 必须有源码。 | [F0395](FRONTEND.md#f0395)；[F1108](FRONTEND.md#f1108)；[D0126](BUILD.md#d0126)；[hir-lower/src/editing.rs:104](../../compiler/hir-lower/src/editing.rs#L104) |
| <a id="e2005"></a>`E2005` | 重复选择同一源码 | 多个 source selections 重叠或出现相同源码身份；若 driver 自己重复构造已规范化输入，转内部输入合同。 | [F1501](FRONTEND.md#f1501)；[D0124](BUILD.md#d0124)；[parser/src/source_input.rs:152](../../compiler/parser/src/source_input.rs#L152) |
| <a id="e2006"></a>`E2006` | 源码发现遇到目录循环 | 源码目录/符号链接遍历成环，属于必要终止检查。 | [D0121](BUILD.md#d0121)；[D0122](BUILD.md#d0122)；[manifest/src/discovery/error.rs:97](../../compiler/manifest/src/discovery/error.rs#L97) |
| <a id="e2007"></a>`E2007` | 源码 package 不属于当前 Cone group | package 必须等于 group 或其子 package。 | [D0089](BUILD.md#d0089)；[D0090](BUILD.md#d0090)；[D0348](BUILD.md#d0348)；[driver/src/request/preflight/packages.rs:26](../../compiler/driver/src/request/preflight/packages.rs#L26) |
| <a id="e2008"></a>`E2008` | 清单结构或字段值无效 | Cone.toml 解析、schema、Cone kind、未知字段或字段类型错误；嵌套原因保留原字段路径。 | [D0164](BUILD.md#d0164)；[D0169](BUILD.md#d0169)；[D0170](BUILD.md#d0170)；[manifest/src/package/parse.rs:24](../../compiler/manifest/src/package/parse.rs#L24) |
| <a id="e2009"></a>`E2009` | 清单配置不适用于此 Cone/workspace | 虚拟 workspace、bundle、Windows executable 或不允许的 native/publish 配置组合。 | [D0165](BUILD.md#d0165)；[D0166](BUILD.md#d0166)；[D0167](BUILD.md#d0167)；[manifest/src/package/parse.rs:35](../../compiler/manifest/src/package/parse.rs#L35) |
| <a id="e2010"></a>`E2010` | 坐标或依赖 key 格式无效 | 要求规范 group:name 或完整 group:name:version。 | [D0163](BUILD.md#d0163)；[D0191](BUILD.md#d0191)；[D0192](BUILD.md#d0192)；[manifest/src/package/format.rs:11](../../compiler/manifest/src/package/format.rs#L11) |
| <a id="e2011"></a>`E2011` | 保留坐标或 core kind 不合法 | 当前项目不允许普通 manifest 声明保留身份，core 必须 library。 | [D0195](BUILD.md#d0195)；[D0196](BUILD.md#d0196)；[D0359](BUILD.md#d0359)；[manifest/src/semantic.rs:370](../../compiler/manifest/src/semantic.rs#L370) |
| <a id="e2012"></a>`E2012` | 清单路径格式无效 | 需相对路径、非空、无 NUL 或不得越过声明根；作为路径规则，不扩展审计。 | [D0143](BUILD.md#d0143)；[D0144](BUILD.md#d0144)；[D0150](BUILD.md#d0150)；[manifest/src/native/parse.rs:95](../../compiler/manifest/src/native/parse.rs#L95) |
| <a id="e2013"></a>`E2013` | target predicate 无效 | target 选择条件名称、类型或取值不在当前集合。 | [D0184](BUILD.md#d0184)；[D0185](BUILD.md#d0185)；[D0186](BUILD.md#d0186)；[manifest/src/selection/parse.rs:78](../../compiler/manifest/src/selection/parse.rs#L78) |
| <a id="e2014"></a>`E2014` | 依赖来源声明不合法 | 必须选一个来源、限定 Git selector、artifact 精确版本、workspace 继承形式等；这些子约束保留为原因。 | [D0151](BUILD.md#d0151)；[D0152](BUILD.md#d0152)；[D0153](BUILD.md#d0153)；[manifest/src/package/dependency.rs:49](../../compiler/manifest/src/package/dependency.rs#L49) |
| <a id="e2015"></a>`E2015` | 版本需求语法无效 | 版本范围/交集/精确版本与 build metadata 使用不满足当前解析规则。 | [D0174](BUILD.md#d0174)；[D0175](BUILD.md#d0175)；[D0176](BUILD.md#d0176)；[manifest/src/requirement.rs:46](../../compiler/manifest/src/requirement.rs#L46) |
| <a id="e2016"></a>`E2016` | registry/archive/Git 地址无效 | 要求当前源类型允许的 URL 或 locator 形式。 | [D0172](BUILD.md#d0172)；[D0173](BUILD.md#d0173)；[T0172](TOOLS.md#t0172)；[manifest/src/package/registry.rs:20](../../compiler/manifest/src/package/registry.rs#L20) |
| <a id="e2017"></a>`E2017` | native 输入或选项无效 | include/library/flags/macro/源文件扩展及目标语言开关不符合 native 配置规则。 | [D0134](BUILD.md#d0134)；[D0135](BUILD.md#d0135)；[D0136](BUILD.md#d0136)；[manifest/src/native/include.rs:69](../../compiler/manifest/src/native/include.rs#L69) |
| <a id="e2018"></a>`E2018` | workspace 成员选择无效 | 成员缺失/重复、嵌套 workspace、重复坐标或 default selection 不成立。 | [T0152](TOOLS.md#t0152)；[T0210](TOOLS.md#t0210)；[T0211](TOOLS.md#t0211)；[pm/src/resolve.rs:107](../../compiler/pm/src/resolve.rs#L107) |
| <a id="e2019"></a>`E2019` | workspace 继承无法解析 | 非成员请求继承，或父 workspace 缺少要求字段/依赖。 | [T0221](TOOLS.md#t0221)；[T0222](TOOLS.md#t0222)；[T0223](TOOLS.md#t0223)；[pm/src/workspace/expand.rs:95](../../compiler/pm/src/workspace/expand.rs#L95) |
| <a id="e2020"></a>`E2020` | 输出路径不合法或与输入冲突 | 输出缺文件名/parent、类型不对、别名冲突或拒绝覆盖既有目标。 | [D0064](BUILD.md#d0064)；[D0065](BUILD.md#d0065)；[D0067](BUILD.md#d0067)；[driver/src/request/output.rs:87](../../compiler/driver/src/request/output.rs#L87) |

### 21 · 依赖与包管理

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e2101"></a>`E2101` | 找不到依赖产物/包输入 | 显式依赖、sysroot artifact、已锁包记录未找到。 | [D0083](BUILD.md#d0083)；[D0303](BUILD.md#d0303)；[D0378](BUILD.md#d0378)；[driver/src/request/preflight/dependencies.rs:263](../../compiler/driver/src/request/preflight/dependencies.rs#L263) |
| <a id="e2102"></a>`E2102` | 依赖实际坐标与要求不符 | source/prebuilt/registry 选中的坐标不符合请求。 | [D0283](BUILD.md#d0283)；[D0284](BUILD.md#d0284)；[D0293](BUILD.md#d0293)；[scoop/src/artifact/error.rs:131](../../compiler/scoop/src/artifact/error.rs#L131) |
| <a id="e2103"></a>`E2103` | 依赖不是可消费的库 | 依赖必须是 manifest library，不能是 executable/native-app/single-file 节点。 | [D0076](BUILD.md#d0076)；[D0285](BUILD.md#d0285)；[D0286](BUILD.md#d0286)；[driver/src/request/preflight/dependencies.rs:214](../../compiler/driver/src/request/preflight/dependencies.rs#L214) |
| <a id="e2104"></a>`E2104` | 依赖闭包包含冲突版本 | 同一 package 只能选一个精确版本。 | [D0078](BUILD.md#d0078)；[D0296](BUILD.md#d0296)；[D0383](BUILD.md#d0383)；[driver/src/request/preflight/dependencies.rs:235](../../compiler/driver/src/request/preflight/dependencies.rs#L235) |
| <a id="e2105"></a>`E2105` | 依赖图存在环 | 含直接自依赖、回到当前 Cone 的边和间接环。 | [D0075](BUILD.md#d0075)；[D0081](BUILD.md#d0081)；[D0082](BUILD.md#d0082)；[driver/src/request/preflight/dependencies.rs:206](../../compiler/driver/src/request/preflight/dependencies.rs#L206) |
| <a id="e2106"></a>`E2106` | 依赖有冲突的候选或来源 | 同一身份的 source/artifact/locator/fingerprint 不唯一。 | [D0077](BUILD.md#d0077)；[D0280](BUILD.md#d0280)；[D0301](BUILD.md#d0301)；[driver/src/request/preflight/dependencies.rs:223](../../compiler/driver/src/request/preflight/dependencies.rs#L223) |
| <a id="e2107"></a>`E2107` | 依赖集合与声明/构建图不一致 | direct/support 集合缺失、多余或边关系不同；编译器自己构造的图违约使用 ICE，不归咎源码。 | [D0080](BUILD.md#d0080)；[D0084](BUILD.md#d0084)；[D0088](BUILD.md#d0088)；[driver/src/request/preflight/dependencies.rs:243](../../compiler/driver/src/request/preflight/dependencies.rs#L243) |
| <a id="e2108"></a>`E2108` | 预编译依赖与当前语义版本不相容 | 当前消费的产物依赖语义 fingerprint 不匹配；不据此新增缓存命中重读或恢复检查。 | [D0279](BUILD.md#d0279)；[D0289](BUILD.md#d0289)；[D0295](BUILD.md#d0295)；[scoop/src/artifact/completion.rs:150](../../compiler/scoop/src/artifact/completion.rs#L150) |
| <a id="e2109"></a>`E2109` | 测试依赖边使用不合法 | core 不能是 test-only；测试依赖必须是当前 Cone 的直接依赖。 | [F0344](FRONTEND.md#f0344)；[F0345](FRONTEND.md#f0345)；[hir-lower/src/current_input.rs:189](../../compiler/hir-lower/src/current_input.rs#L189) |
| <a id="e2110"></a>`E2110` | 没有满足要求的依赖解 | 版本、来源、锁定/yank 等条件无法同时满足。 | [T0160](TOOLS.md#t0160)；[T0164](TOOLS.md#t0164)；[T0165](TOOLS.md#t0165)；[pm/src/resolver/report.rs:19](../../compiler/pm/src/resolver/report.rs#L19) |
| <a id="e2111"></a>`E2111` | 锁定结果不能满足当前请求 | locked/frozen 缺相应 resolution/selector，或 manifest 已变化；需重新 resolve。 | [T0095](TOOLS.md#t0095)；[T0103](TOOLS.md#t0103)；[T0105](TOOLS.md#t0105)；[pm/src/catalog.rs:151](../../compiler/pm/src/catalog.rs#L151) |
| <a id="e2112"></a>`E2112` | 所需包数据未缓存 | offline/metadata 阶段缺源码、索引、Git object/submodule 或 archive。 | [T0096](TOOLS.md#t0096)；[T0106](TOOLS.md#t0106)；[T0108](TOOLS.md#t0108)；[pm/src/catalog.rs:222](../../compiler/pm/src/catalog.rs#L222) |
| <a id="e2113"></a>`E2113` | Cone.lock 内容无效 | format、坐标、边、源 locator、Git commit、URL/摘要等现有格式规则失败，具体项作为原因。 | [T0126](TOOLS.md#t0126)；[T0129](TOOLS.md#t0129)；[T0130](TOOLS.md#t0130)；[pm/src/lockfile.rs:35](../../compiler/pm/src/lockfile.rs#L35) |
| <a id="e2114"></a>`E2114` | 选中的包不在当前图中 | deps why/update 等目标未出现在所选 locked/resolved graph。 | [T0117](TOOLS.md#t0117)；[T0157](TOOLS.md#t0157)；[pm/src/cli/deps/why.rs:80](../../compiler/pm/src/cli/deps/why.rs#L80) |
| <a id="e2115"></a>`E2115` | Git selector 无法确定提交 | precise 不是完整 commit，或 remote 无默认 HEAD。 | [T0104](TOOLS.md#t0104)；[T0179](TOOLS.md#t0179)；[pm/src/catalog/remote.rs:49](../../compiler/pm/src/catalog/remote.rs#L49) |
| <a id="e2116"></a>`E2116` | 没有可用 registry 配置 | 依赖未给显式 registry 且无默认 registry。 | [T0111](TOOLS.md#t0111)；[pm/src/catalog/requirements.rs:110](../../compiler/pm/src/catalog/requirements.rs#L110) |
| <a id="e2117"></a>`E2117` | 包下载失败 | HTTP 下载返回错误；保留 HTTP/系统原因。 | [T0184](TOOLS.md#t0184)；[pm/src/sources/registry/archive.rs:90](../../compiler/pm/src/sources/registry/archive.rs#L90) |
| <a id="e2118"></a>`E2118` | 下载摘要与包元数据不符 | 既有 SHA-256 内容校验失败；不引入额外认证或信任机制。 | [T0185](TOOLS.md#t0185)；[pm/src/sources/registry/archive.rs:100](../../compiler/pm/src/sources/registry/archive.rs#L100) |
| <a id="e2119"></a>`E2119` | 包归档结构无效 | 路径越界、重复/空路径或不支持的 archive entry type。 | [T0181](TOOLS.md#t0181)；[T0182](TOOLS.md#t0182)；[T0183](TOOLS.md#t0183)；[pm/src/sources/registry/archive.rs:29](../../compiler/pm/src/sources/registry/archive.rs#L29) |
| <a id="e2120"></a>`E2120` | registry 元数据格式或一致性错误 | 版本/坐标/摘要/依赖表及 immutable version 内容不匹配。 | [T0189](TOOLS.md#t0189)；[T0190](TOOLS.md#t0190)；[T0192](TOOLS.md#t0192)；[pm/src/sources/registry/fetch.rs:181](../../compiler/pm/src/sources/registry/fetch.rs#L181) |
| <a id="e2121"></a>`E2121` | 发布包的 manifest 未展开或不自足 | 不允许未展开 workspace 配置、缺版本或缺固定来源；保留已有规则。 | [T0200](TOOLS.md#t0200)；[T0201](TOOLS.md#t0201)；[T0202](TOOLS.md#t0202)；[pm/src/sources/registry/manifest.rs:18](../../compiler/pm/src/sources/registry/manifest.rs#L18) |

### 22 · 进程、I/O 与构建协调

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e2201"></a>`E2201` | 文件或目录 I/O 失败 | 实际 read/write/create/inspect/canonicalize/rename/lock 等系统操作失败；role/path/operation 和 OS 原因附加，不按目录或阶段分码。 | [D0014](BUILD.md#d0014)；[D0016](BUILD.md#d0016)；[D0046](BUILD.md#d0046)；[driver/src/child_protocol.rs:243](../../compiler/driver/src/child_protocol.rs#L243) |
| <a id="e2202"></a>`E2202` | 外部进程/工作线程不能启动 | compiler、generator、Git、native 工具等启动失败。 | [D0032](BUILD.md#d0032)；[D0314](BUILD.md#d0314)；[D0319](BUILD.md#d0319)；[driver/src/generation/external/session.rs:34](../../compiler/driver/src/generation/external/session.rs#L34) |
| <a id="e2203"></a>`E2203` | 外部进程执行失败 | 外部工具非零退出/信号退出；若已返回具体编译诊断，应保留它们而非再报同级错误。 | [D0039](BUILD.md#d0039)；[D0328](BUILD.md#d0328)；[D0338](BUILD.md#d0338)；[driver/src/generation/external/session.rs:93](../../compiler/driver/src/generation/external/session.rs#L93) |
| <a id="e2204"></a>`E2204` | 子进程通信或等待失败 | 管道、frame 读写及 wait 失败；失败内容保留系统原因。 | [D0009](BUILD.md#d0009)；[D0011](BUILD.md#d0011)；[D0320](BUILD.md#d0320)；[driver/src/child_protocol.rs:25](../../compiler/driver/src/child_protocol.rs#L25) |
| <a id="e2205"></a>`E2205` | 主机资源或表示容量不足 | 真实 allocation/索引/固定表示位宽不足；不引入 CPU/内存预算或计费。 | [F0622](FRONTEND.md#f0622)；[F0623](FRONTEND.md#f0623)；[F1081](FRONTEND.md#f1081)；[hir-lower/src/expr/imported_origins.rs:172](../../compiler/hir-lower/src/expr/imported_origins.rs#L172) |
| <a id="e2206"></a>`E2206` | 准备期间输入发生变化 | 当前快照无法稳定发布，请重试；沿用现有并发更新检查。 | [T0094](TOOLS.md#t0094)；[T0150](TOOLS.md#t0150)；[T0158](TOOLS.md#t0158)；[pm/src/cache/inputs.rs:35](../../compiler/pm/src/cache/inputs.rs#L35) |

### 23 · 目标、工具链与链接

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e2301"></a>`E2301` | 不支持或不认识目标配置 | target triple、机器、object format 或 native feature 组合不支持。 | [T0037](TOOLS.md#t0037)；[T0038](TOOLS.md#t0038)；[D0530](BUILD.md#d0530)；[api/src/main.rs:96](../../compiler/api/src/main.rs#L96) |
| <a id="e2302"></a>`E2302` | 找不到所需工具或 native 组件 | paired compiler/C/C++/SDK/runtime/native 库等资源缺失；具体组件为参数。 | [D0101](BUILD.md#d0101)；[D0103](BUILD.md#d0103)；[D0318](BUILD.md#d0318)；[driver/src/trusted_core.rs:55](../../compiler/driver/src/trusted_core.rs#L55) |
| <a id="e2303"></a>`E2303` | 工具链不符合选定目标合同 | C bridge probe、SDK/compiler identity、target/profile 等配置或返回不符合要求；不按每个 Mach-O 字段分主码。 | [D0463](BUILD.md#d0463)；[D0464](BUILD.md#d0464)；[D0465](BUILD.md#d0465)；[toolchain/src/c_bridge.rs:29](../../compiler/toolchain/src/c_bridge.rs#L29) |
| <a id="e2304"></a>`E2304` | 最终链接失败 | 系统 linker 返回失败，保留命令及 stderr；Scoop 先验输入合同错误用更具体的产物/native 码。 | [A1882](ARTIFACTS.md#a1882)；[A1888](ARTIFACTS.md#a1888)；[A1892](ARTIFACTS.md#a1892)；[linker/src/link/darwin.rs:46](../../compiler/linker/src/link/darwin.rs#L46) |
| <a id="e2305"></a>`E2305` | native 链接输入无效 | 用户提供 native library/object/archive 的类型、目标、可解析性或符号输入合同不符。 | [A1839](ARTIFACTS.md#a1839)；[A1840](ARTIFACTS.md#a1840)；[A1841](ARTIFACTS.md#a1841)；[linker/src/dynamic/exports.rs:59](../../compiler/linker/src/dynamic/exports.rs#L59) |
| <a id="e2306"></a>`E2306` | native 符号/namespace/ODR 冲突 | 输入闭包的定义/符号合同相互冲突。 | [A1846](ARTIFACTS.md#a1846)；[A1853](ARTIFACTS.md#a1853)；[A1911](ARTIFACTS.md#a1911)；[linker/src/dynamic/graph.rs:121](../../compiler/linker/src/dynamic/graph.rs#L121) |
| <a id="e2307"></a>`E2307` | runtime 配置或 ABI 不相容 | 选中 runtime/exception model/目标版本不满足编译链接要求。 | [D0602](BUILD.md#d0602)；[D0603](BUILD.md#d0603)；[D0604](BUILD.md#d0604)；[toolchain/src/runtime.rs:51](../../compiler/toolchain/src/runtime.rs#L51) |
| <a id="e2308"></a>`E2308` | native 符号未解析 | 所选库/namespace 没有提供所需符号或 symbol version。 | [A1919](ARTIFACTS.md#a1919)；[A2003](ARTIFACTS.md#a2003)；[A2004](ARTIFACTS.md#a2004)；[linker/src/namespace/elf/graph.rs:257](../../compiler/linker/src/namespace/elf/graph.rs#L257) |

### 24 · 编译产物与跨 Cone 合同

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e2401"></a>`E2401` | 产物容器或记录格式无效 | 归档、envelope、section、wire/CBOR、tag/length/order 等不符合格式；字段级细节保留为原因。 | [A2362](ARTIFACTS.md#a2362)；[A2363](ARTIFACTS.md#a2363)；[A2364](ARTIFACTS.md#a2364)；[slib/src/archive.rs:56](../../compiler/slib/src/archive.rs#L56) |
| <a id="e2402"></a>`E2402` | 产物 schema/协议版本不支持 | 已有产物版本与当前 reader 不兼容。 | [A2584](ARTIFACTS.md#a2584)；[A2590](ARTIFACTS.md#a2590)；[A2617](ARTIFACTS.md#a2617)；[slib/src/manifest/decode.rs:281](../../compiler/slib/src/manifest/decode.rs#L281) |
| <a id="e2403"></a>`E2403` | 产物缺少必需成员或节 | 消费操作所需 member/section/profile inventory 不完整。 | [A1976](ARTIFACTS.md#a1976)；[A2369](ARTIFACTS.md#a2369)；[A2611](ARTIFACTS.md#a2611)；[linker/src/program.rs:143](../../compiler/linker/src/program.rs#L143) |
| <a id="e2404"></a>`E2404` | 产物 target/profile/ABI 不匹配 | 目标机器、完整 target、profile 或固定 ABI 版本不符合本次消费要求。 | [D0287](BUILD.md#d0287)；[D0288](BUILD.md#d0288)；[A2589](ARTIFACTS.md#a2589)；[scoop/src/artifact/error.rs:163](../../compiler/scoop/src/artifact/error.rs#L163) |
| <a id="e2405"></a>`E2405` | 产物实体身份或引用关系无效 | typed entity/owner/source/decl 引用缺失、冲突、循环或不符合编码规则。 | [F0613](FRONTEND.md#f0613)；[F0620](FRONTEND.md#f0620)；[F0621](FRONTEND.md#f0621)；[hir-lower/src/expr/imported_constants.rs:154](../../compiler/hir-lower/src/expr/imported_constants.rs#L154) |
| <a id="e2406"></a>`E2406` | 产物声明接口合同无效 | 外部 HIR 声明/签名/类型应用/属性/条件接口等接口结构与引用不一致。 | [T0007](TOOLS.md#t0007)；[F0309](FRONTEND.md#f0309)；[F0134](FRONTEND.md#f0134)；[analysis/src/semantic/dependencies.rs:185](../../compiler/analysis/src/semantic/dependencies.rs#L185) |
| <a id="e2407"></a>`E2407` | 产物可执行模板或 MIR 合同无效 | 导入 default/generic/init/closure/template/控制流体不满足 reader 的既有格式与引用规则。 | [F0178](FRONTEND.md#f0178)；[F0179](FRONTEND.md#f0179)；[F0180](FRONTEND.md#f0180)；[hir-lower/src/class/inheritance/decoding/constructors.rs:139](../../compiler/hir-lower/src/class/inheritance/decoding/constructors.rs#L139) |
| <a id="e2408"></a>`E2408` | 产物布局、调用 ABI 或描述符无效 | 外部 LIR/layout/dispatch/scan/ABI 记录相互不一致。 | [A1992](ARTIFACTS.md#a1992)；[A1993](ARTIFACTS.md#a1993)；[A1994](ARTIFACTS.md#a1994)；[linker/src/program/native/contracts.rs:187](../../compiler/linker/src/program/native/contracts.rs#L187) |
| <a id="e2409"></a>`E2409` | 产物 native object 或 runtime 记录无效 | 归档内 object/atom/relocation/GC registration/runtime metadata 不满足消费合同。 | [A1975](ARTIFACTS.md#a1975)；[A1978](ARTIFACTS.md#a1978)；[A1981](ARTIFACTS.md#a1981)；[linker/src/program.rs:126](../../compiler/linker/src/program.rs#L126) |
| <a id="e2410"></a>`E2410` | 产物压缩或解压失败 | 压缩流/codec 无效；OS I/O 和 allocation 仍使用对应操作错误。 | [A2372](ARTIFACTS.md#a2372)；[A2374](ARTIFACTS.md#a2374)；[A2375](ARTIFACTS.md#a2375)；[slib/src/compression.rs:11](../../compiler/slib/src/compression.rs#L11) |
| <a id="e2411"></a>`E2411` | 产物不提供当前操作所需能力 | source-only/layout/dispatch/generic/native/Strong profile 能力不足；不是“尚未实现”的通用占位码。 | [F0262](FRONTEND.md#f0262)；[F0394](FRONTEND.md#f0394)；[F0897](FRONTEND.md#f0897)；[hir-lower/src/concretize/runtime_exceptions.rs:251](../../compiler/hir-lower/src/concretize/runtime_exceptions.rs#L251) |
| <a id="e2412"></a>`E2412` | 跨产物定义或链接合同冲突 | ODR、exact identity 对应不一致定义，或不同生产者给出矛盾合同。 | [A1977](ARTIFACTS.md#a1977)；[A1979](ARTIFACTS.md#a1979)；[A1980](ARTIFACTS.md#a1980)；[linker/src/program.rs:159](../../compiler/linker/src/program.rs#L159) |

### 25 · 工具协议与查询

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="e2501"></a>`E2501` | 协议版本或配套工具不相容 | scoopc/package/LSP 协议版本或协商能力不匹配。 | [D0010](BUILD.md#d0010)；[D0230](BUILD.md#d0230)；[D0265](BUILD.md#d0265)；[driver/src/child_protocol.rs:235](../../compiler/driver/src/child_protocol.rs#L235) |
| <a id="e2502"></a>`E2502` | 外部工具协议内容无效 | frame、magic、schema、字段/关联/路径等违反该工具协议；内部编码自己生成的非法值用 ICE。 | [D0012](BUILD.md#d0012)；[D0223](BUILD.md#d0223)；[D0225](BUILD.md#d0225)；[driver/src/child_protocol.rs:238](../../compiler/driver/src/child_protocol.rs#L238) |
| <a id="e2503"></a>`E2503` | 查询请求或编辑范围无效 | URI、position、range、query kind 或 edit overlap 不符合查询 API 合同；来自编译器诊断的内部越界另判 ICE。 | [T0004](TOOLS.md#t0004)；[T0005](TOOLS.md#t0005)；[T0063](TOOLS.md#t0063)；[analysis/src/host/edits.rs:24](../../compiler/analysis/src/host/edits.rs#L24) |
| <a id="e2504"></a>`E2504` | 文档/分析版本或会话状态不符 | 文档 open/change/version、code action revision 或 LSP lifecycle 不满足当前状态。 | [T0001](TOOLS.md#t0001)；[T0003](TOOLS.md#t0003)；[T0060](TOOLS.md#t0060)；[analysis/src/host/edits.rs:20](../../compiler/analysis/src/host/edits.rs#L20) |
| <a id="e2505"></a>`E2505` | 查询所需语义/源码输入不可用 | IDE/API 请求不能得到完整结果；不是被查询源程序的新类型错误。 | [F1145](FRONTEND.md#f1145)；[F1147](FRONTEND.md#f1147)；[F1149](FRONTEND.md#f1149)；[hir-lower/src/semantic_queries.rs:268](../../compiler/hir-lower/src/semantic_queries.rs#L268) |
| <a id="e2506"></a>`E2506` | 重命名目标名称不合法 | 新名称不是有效源码标识符。 | [T0010](TOOLS.md#t0010)；[analysis/src/semantic/rename.rs:50](../../compiler/analysis/src/semantic/rename.rs#L50) |
| <a id="e2507"></a>`E2507` | 重命名会改变绑定或不可完整编辑 | 编辑会改变引用目标，或必要声明/隐式 operator use 没有可编辑标识符。 | [T0011](TOOLS.md#t0011)；[T0014](TOOLS.md#t0014)；[T0015](TOOLS.md#t0015)；[analysis/src/semantic/rename.rs:52](../../compiler/analysis/src/semantic/rename.rs#L52) |
| <a id="e2508"></a>`E2508` | 测试进程报告用例失败 | 配套 test 命令的执行结果，不是编译错误。 | [D0351](BUILD.md#d0351)；[scoop/src/cli/test_run.rs:98](../../compiler/scoop/src/cli/test_run.rs#L98) |
| <a id="e2509"></a>`E2509` | 生成器程序未注册 | GenerateWith 要求的 generator 没有主机程序映射。 | [D0025](BUILD.md#d0025)；[driver/src/generation/external.rs:51](../../compiler/driver/src/generation/external.rs#L51) |
| <a id="e2510"></a>`E2510` | 外部生成器报告失败 | 保留 generator 返回的原始诊断；失败包装只在没有具体诊断时作主错误。 | [D0031](BUILD.md#d0031)；[D0034](BUILD.md#d0034)；[driver/src/generation/external/session.rs:25](../../compiler/driver/src/generation/external/session.rs#L25) |
| <a id="e2511"></a>`E2511` | 外部生成器结果违反协议 | 位置/provider、batch/method、result/id/输出顺序不合法。 | [F0836](FRONTEND.md#f0836)；[F0837](FRONTEND.md#f0837)；[D0026](BUILD.md#d0026)；[hir-lower/src/generation.rs:67](../../compiler/hir-lower/src/generation.rs#L67) |
| <a id="e2512"></a>`E2512` | 生成器查询引用不合法 | export path 或 identity reference 不符合查询接口输入规则。 | [D0110](BUILD.md#d0110)；[D0111](BUILD.md#d0111)；[D0112](BUILD.md#d0112)；[generator-protocol/src/queries/exports.rs:15](../../compiler/generator-protocol/src/queries/exports.rs#L15) |

## 警告（15 项）

### 09 · 模式与穷尽性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="w0901"></a>`W0901` | 模式绑定遮蔽可用 enum 变体 | 当前绑定名与当前位置可用 variant 重名。 | [W14](INVENTORY.md#w14)；[hir-lower/src/patterns/warnings.rs:69](../../compiler/hir-lower/src/patterns/warnings.rs#L69) |

### 18 · 文档

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="w1801"></a>`W1801` | 文档引用符号语法无效 | Markdown 文档符号标识无法解析为有效路径。 | [W01](INVENTORY.md#w01)；[doc/src/markdown.rs:71](../../compiler/doc/src/markdown.rs#L71) |
| <a id="w1802"></a>`W1802` | 文档 alias 为空 | alias 不能仅包含空白。 | [W02](INVENTORY.md#w02)；[doc/src/project/annotations.rs:36](../../compiler/doc/src/project/annotations.rs#L36) |
| <a id="w1803"></a>`W1803` | 文档注释没有声明归属 | 该注释没有附着到声明。 | [W03](INVENTORY.md#w03)；[doc/src/syntax.rs:22](../../compiler/doc/src/syntax.rs#L22) |
| <a id="w1804"></a>`W1804` | 文档标签名称格式不合法 | 标签 subject 不符合所需名称语法；param/property 只接受单个名称，其他标签可接受限定名。 | [W04](INVENTORY.md#w04)；[W08](INVENTORY.md#w08)；[doc/src/tags.rs:42](../../compiler/doc/src/tags.rs#L42) |
| <a id="w1805"></a>`W1805` | since 缺少版本文本 | @since 的正文不能为空。 | [W05](INVENTORY.md#w05)；[doc/src/tags.rs:78](../../compiler/doc/src/tags.rs#L78) |
| <a id="w1806"></a>`W1806` | 未知文档标签 | 标签名称不在已支持集合。 | [W06](INVENTORY.md#w06)；[doc/src/tags.rs:85](../../compiler/doc/src/tags.rs#L85) |
| <a id="w1807"></a>`W1807` | 文档标签缺少 subject | 要求名称的标签未给出 subject。 | [W07](INVENTORY.md#w07)；[doc/src/tags.rs:116](../../compiler/doc/src/tags.rs#L116) |
| <a id="w1808"></a>`W1808` | 同一文档目标重复说明 | 相同 reference tag 或同一 (category,target) 的正文重复，保留首份。 | [W10](INVENTORY.md#w10)；[W13](INVENTORY.md#w13)；[hir-lower/src/documentation/binding.rs:155](../../compiler/hir-lower/src/documentation/binding.rs#L155) |
| <a id="w1809"></a>`W1809` | 文档符号引用到了 package | 文档链接要求 declaration，实际名称为 package。 | [W11](INVENTORY.md#w11)；[R01](INVENTORY.md#r01)；[hir-lower/src/documentation/binding.rs:221](../../compiler/hir-lower/src/documentation/binding.rs#L221) |
| <a id="w1810"></a>`W1810` | 文档符号未解析 | 文档引用的名称无法绑定。 | [W11](INVENTORY.md#w11)；[R02](INVENTORY.md#r02)；[hir-lower/src/documentation/binding.rs:221](../../compiler/hir-lower/src/documentation/binding.rs#L221) |
| <a id="w1811"></a>`W1811` | 文档符号引用有歧义 | owner 或最终 declaration 不能唯一确定。 | [W11](INVENTORY.md#w11)；[R03](INVENTORY.md#r03)；[R04](INVENTORY.md#r04)；[hir-lower/src/documentation/binding.rs:221](../../compiler/hir-lower/src/documentation/binding.rs#L221) |
| <a id="w1812"></a>`W1812` | 文档标签没有适用目标 | param/property/accessor/throws 等不能落到要求的实际声明。 | [W12](INVENTORY.md#w12)；[hir-lower/src/documentation/binding/tags.rs:115](../../compiler/hir-lower/src/documentation/binding/tags.rs#L115) |
| <a id="w1813"></a>`W1813` | README 本地资源缺失 | README 链接或图片指向的本地资源不存在，保留链接并报告 warning。 | [scoop/src/document/overview.rs](../../compiler/scoop/src/document/overview.rs) |

### 20 · 命令、项目与源码输入

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="w2001"></a>`W2001` | 单文件模式忽略 package | 单文件使用默认 root package；不是错误。 | [W09](INVENTORY.md#w09)；[driver/src/request/preflight/packages.rs:69](../../compiler/driver/src/request/preflight/packages.rs#L69) |

## 附加说明（14 项）

### 00 · 通用附注与建议

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n0001"></a>`N0001` | 相关或冲突声明的位置 | previous/other/related declaration、约束、绑定、package header 等共用一个位置关系，relation 为参数。 | [N01](INVENTORY.md#n01)；[N02](INVENTORY.md#n02)；[N03](INVENTORY.md#n03)；[driver/src/generation/plan.rs:108](../../compiler/driver/src/generation/plan.rs#L108) |
| <a id="n0002"></a>`N0002` | 候选声明的位置 | 函数/类型/入口候选的相关声明位置。 | [N06](INVENTORY.md#n06)；[N11](INVENTORY.md#n11)；[hir-lower/src/imports/lookup/diagnostics.rs:37](../../compiler/hir-lower/src/imports/lookup/diagnostics.rs#L37) |
| <a id="n0003"></a>`N0003` | 实现生成请求的位置 | 显示当前生成/派生实现由哪个源位置请求。 | [N05](INVENTORY.md#n05)；[hir-lower/src/generation.rs:111](../../compiler/hir-lower/src/generation.rs#L111) |
| <a id="n0004"></a>`N0004` | 上游声明或导入未能完成 | 与已有根错误关联；不另发语义主错误。 | [N04](INVENTORY.md#n04)；[N07](INVENTORY.md#n07)；[N08](INVENTORY.md#n08)；[hir-lower/src/declaration_surface.rs:150](../../compiler/hir-lower/src/declaration_surface.rs#L150) |

### 04 · 名称、导入与可见性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n0401"></a>`N0401` | 声明访问域的说明 | 指出声明在哪个访问域外，主错误仍用 access。 | [N09](INVENTORY.md#n09)；[hir-lower/src/imports/resolve.rs:128](../../compiler/hir-lower/src/imports/resolve.rs#L128) |

### 05 · 类型与转换

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n0501"></a>`N0501` | 名义泛型不变性的具体差异 | 显示第一个不一致类型实参与 source/target application。 | [F1278](FRONTEND.md#f1278)；[hir-lower/src/types/invariance.rs:45](../../compiler/hir-lower/src/types/invariance.rs#L45) |

### 07 · 调用、重载与函数引用

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n0701"></a>`N0701` | 候选函数引用的适用依据 | 解释候选匹配期望函数类型或形成完整非泛型函数类型。 | [P04](INVENTORY.md#p04)；[P05](INVENTORY.md#p05)；[hir-lower/src/expr/callable_references/resolution/diagnostics.rs:83](../../compiler/hir-lower/src/expr/callable_references/resolution/diagnostics.rs#L83) |
| <a id="n0702"></a>`N0702` | 候选应用与已固定实参 | 签名、参数位置、fixed type arguments 是上下文；真正失败若已有 E 规则则复用 E，不能以本 N 替换原因。 | [P16](INVENTORY.md#p16)；[P18](INVENTORY.md#p18)；[P19](INVENTORY.md#p19)；[hir-lower/src/overload/diagnostics.rs:120](../../compiler/hir-lower/src/overload/diagnostics.rs#L120) |
| <a id="n0703"></a>`N0703` | 候选排序后仍并列 | 声明转发/比较后没有唯一更优候选。 | [P17](INVENTORY.md#p17)；[hir-lower/src/overload/diagnostics.rs:165](../../compiler/hir-lower/src/overload/diagnostics.rs#L165) |
| <a id="n0704"></a>`N0704` | 整数实参与具体化类型的说明 | 展示字面量实参及确定的 primitive integer 类型。 | [P20](INVENTORY.md#p20)；[hir-lower/src/overload/diagnostics/integers.rs:52](../../compiler/hir-lower/src/overload/diagnostics/integers.rs#L52) |
| <a id="n0705"></a>`N0705` | intrinsic 不可间接调用的原因 | 编译期求值或 direct source location 要求导致没有普通运行时 callable entry。 | [P01](INVENTORY.md#p01)；[P02](INVENTORY.md#p02)；[P03](INVENTORY.md#p03)；[hir-lower/src/expr/callable_references/intrinsics.rs:35](../../compiler/hir-lower/src/expr/callable_references/intrinsics.rs#L35) |

### 09 · 模式与穷尽性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n0901"></a>`N0901` | guard 不参与穷尽性证明 | 解释为什么存在 guarded arm 仍未穷尽。 | [P22](INVENTORY.md#p22)；[hir-lower/src/patterns/exhaustiveness.rs:46](../../compiler/hir-lower/src/patterns/exhaustiveness.rs#L46) |

### 17 · 入口与测试声明

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n1701"></a>`N1701` | 入口候选不合格的原因 | generic/suspend/参数/返回/receiver/visibility/body 等都是入口资格原因。 | [N12](INVENTORY.md#n12)；[N13](INVENTORY.md#n13)；[P06](INVENTORY.md#p06)；[hir-lower/src/output_kind.rs:286](../../compiler/hir-lower/src/output_kind.rs#L286) |

### 21 · 依赖与包管理

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="n2101"></a>`N2101` | 离线解析范围说明 | 说明本次只使用本地 metadata/Git objects，未检查远端版本。 | [T0155](TOOLS.md#t0155)；[pm/src/resolve.rs:165](../../compiler/pm/src/resolve.rs#L165) |

## 修复建议（30 项）

### 01 · 词法与字面量

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0101"></a>`H0101` | 补上插值花括号 | 将非法简写改成语言规定的插值形式。 | [F1484](FRONTEND.md#f1484)；[parser/src/lexer/interpolation.rs:81](../../compiler/parser/src/lexer/interpolation.rs#L81) |

### 04 · 名称、导入与可见性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0401"></a>`H0401` | 导入可用声明 | 给出可导入的完整路径，保留原结构化 import_path；不凭文案反向解析编辑。 | [N10](INVENTORY.md#n10)；[hir-lower/src/imports/suggestions.rs:82](../../compiler/hir-lower/src/imports/suggestions.rs#L82) |
| <a id="h0402"></a>`H0402` | 限定名称或明确接收者 | 裸 variant 使用 E.V/期望类型；外部 receiver 使用 this@label 或显式对象。 | [F0601](FRONTEND.md#f0601)；[F0790](FRONTEND.md#f0790)；[F0801](FRONTEND.md#f0801)；[hir-lower/src/expr/generic_calls.rs:129](../../compiler/hir-lower/src/expr/generic_calls.rs#L129) |

### 05 · 类型与转换

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0501"></a>`H0501` | 显式转换或构造目标类型 | primitive integer 使用明确转换；不变 nominal application 直接构造目标或转换。 | [F1278](FRONTEND.md#f1278)；[P21](INVENTORY.md#p21)；[hir-lower/src/types/invariance.rs:45](../../compiler/hir-lower/src/types/invariance.rs#L45) |
| <a id="h0502"></a>`H0502` | 补充显式类型 | 补充返回/属性/参数/enum 期望类型或所需类型实参；原文没有建议的场景不主动新增自动修复。 | [F0352](FRONTEND.md#f0352)；[F0355](FRONTEND.md#f0355)；[F0562](FRONTEND.md#f0562)；[hir-lower/src/declaration_inference.rs:219](../../compiler/hir-lower/src/declaration_inference.rs#L219) |

### 06 · 泛型、约束与推断

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0601"></a>`H0601` | 用有界泛型或明确接口代替星号投影 | 保留当前推荐的 bounded generic callable、exact interface 或 wrapper 选择。 | [F1524](FRONTEND.md#f1524)；[parser/src/ty.rs:262](../../compiler/parser/src/ty.rs#L262) |

### 07 · 调用、重载与函数引用

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0701"></a>`H0701` | 直接调用 intrinsic | 使用常量 path/name 直接调用，或在需要的位置调用/显式传 SourceLocation。 | [P01](INVENTORY.md#p01)；[P02](INVENTORY.md#p02)；[P03](INVENTORY.md#p03)；[hir-lower/src/expr/callable_references/intrinsics.rs:35](../../compiler/hir-lower/src/expr/callable_references/intrinsics.rs#L35) |
| <a id="h0702"></a>`H0702` | 用 lambda 固定调用上下文 | 固定原子 order 或包装 requirement 调用。 | [F0466](FRONTEND.md#f0466)；[F0485](FRONTEND.md#f0485)；[hir-lower/src/expr/atomics.rs:3](../../compiler/hir-lower/src/expr/atomics.rs#L3) |
| <a id="h0703"></a>`H0703` | 使用正确的函数值/函数引用形式 | 已有函数值直接使用；函数声明需要 :: 引用。 | [F0487](FRONTEND.md#f0487)；[F0787](FRONTEND.md#f0787)；[hir-lower/src/expr/callable_references.rs:105](../../compiler/hir-lower/src/expr/callable_references.rs#L105) |
| <a id="h0704"></a>`H0704` | 使用正确的变体构造写法 | unit 去掉调用括号；payload 使用带参数构造。 | [F0088](FRONTEND.md#f0088)；[F0555](FRONTEND.md#f0555)；[F0556](FRONTEND.md#f0556)；[hir-lower/src/call_resolution/candidates/imported.rs:35](../../compiler/hir-lower/src/call_resolution/candidates/imported.rs#L35) |
| <a id="h0705"></a>`H0705` | 调整实参或调用分组 | vararg lambda 放入括号；多次尾随调用先分组。 | [F0087](FRONTEND.md#f0087)；[F1465](FRONTEND.md#f1465)；[hir-lower/src/call_resolution/arguments.rs:152](../../compiler/hir-lower/src/call_resolution/arguments.rs#L152) |

### 08 · 控制流与异常

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0801"></a>`H0801` | 使用本地 return label | lambda 的返回使用 return@label 指向当前 callable。 | [F1201](FRONTEND.md#f1201)；[hir-lower/src/stmt/jumps.rs:169](../../compiler/hir-lower/src/stmt/jumps.rs#L169) |

### 09 · 模式与穷尽性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h0901"></a>`H0901` | 明确绑定或忽略模式 | 需要绑定时使用 val；忽略使用 _，漏字段用 ..，payload 用对应 named/positional 模式。 | [F1027](FRONTEND.md#f1027)；[F1028](FRONTEND.md#f1028)；[F1039](FRONTEND.md#f1039)；[hir-lower/src/patterns.rs:109](../../compiler/hir-lower/src/patterns.rs#L109) |
| <a id="h0902"></a>`H0902` | 重写 when 条件/模式分支 | 无 subject 时合并 Boolean 条件；模式 alternatives 分成不同 case arm。 | [F1509](FRONTEND.md#f1509)；[F1512](FRONTEND.md#f1512)；[parser/src/stmt/when/arms.rs:37](../../compiler/parser/src/stmt/when/arms.rs#L37) |
| <a id="h0903"></a>`H0903` | 重命名绑定或限定变体 | 消除绑定与 enum variant 的名称歧义。 | [W14](INVENTORY.md#w14)；[hir-lower/src/patterns/warnings.rs:69](../../compiler/hir-lower/src/patterns/warnings.rs#L69) |

### 10 · 类、继承与初始化

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h1001"></a>`H1001` | 明确覆盖继承冲突 | 提供覆盖冲突来源和条件范围的 override。 | [F0190](FRONTEND.md#f0190)；[F0215](FRONTEND.md#f0215)；[F0217](FRONTEND.md#f0217)；[hir-lower/src/class/inheritance/delegation/selection.rs:63](../../compiler/hir-lower/src/class/inheritance/delegation/selection.rs#L63) |
| <a id="h1002"></a>`H1002` | 将无条件接口放入接口列表 | 没有条件时使用 type/companion 的无条件接口列表。 | [F1398](FRONTEND.md#f1398)；[parser/src/decl/nominals/interface_implementations.rs:44](../../compiler/parser/src/decl/nominals/interface_implementations.rs#L44) |

### 12 · FFI、原子与原始存储

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h1201"></a>`H1201` | 使用现有 leaf 调用合同 | C extern 采用 GCLeaf；NoGC 中的函数指针采用符合合同的 GCLeafFunPtr。 | [F0066](FRONTEND.md#f0066)；[F0450](FRONTEND.md#f0450)；[hir-lower/src/annotations/functions.rs:180](../../compiler/hir-lower/src/annotations/functions.rs#L180) |
| <a id="h1202"></a>`H1202` | 选择允许的存储声明 | 仅针对 raw-storage 路径，使用允许的局部绑定/Extern val；不暗示普通顶层 val 被禁止。 | [F0019](FRONTEND.md#f0019)；[hir-lower/src/annotations.rs:124](../../compiler/hir-lower/src/annotations.rs#L124) |

### 13 · 注解与自动派生

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h1301"></a>`H1301` | 提供派生依赖或显式实现 | 补 codec dependency/companion、显式 encode/decode/inspect，或在允许处 Serialize(ignore)。 | [F0168](FRONTEND.md#f0168)；[F0172](FRONTEND.md#f0172)；[F0173](FRONTEND.md#f0173)；[hir-lower/src/class/inheritance/coding/codecs.rs:136](../../compiler/hir-lower/src/class/inheritance/coding/codecs.rs#L136) |

### 15 · 属性、常量与值语义

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h1501"></a>`H1501` | 先复制字段或可变值 | 捕获 val snapshot/显式引用状态，release 字段先复制到局部再修改/取址。 | [F0537](FRONTEND.md#f0537)；[F0602](FRONTEND.md#f0602)；[F1143](FRONTEND.md#f1143)；[hir-lower/src/expr/captures.rs:152](../../compiler/hir-lower/src/expr/captures.rs#L152) |
| <a id="h1502"></a>`H1502` | 用 Option 表达未初始化状态 | 按现有说明使用 Option 属性初始化或省略初始化简写；copy update 先解包。 | [F1427](FRONTEND.md#f1427)；[F1458](FRONTEND.md#f1458)；[parser/src/decl/nominals/prefix.rs:115](../../compiler/parser/src/decl/nominals/prefix.rs#L115) |

### 18 · 文档

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h1801"></a>`H1801` | 生成或更新 API 文档 | scoop doc 或 scoop-install docs，底层缺失/格式错误仍保留原编号。 | [T0042](TOOLS.md#t0042)；[api/src/query.rs:76](../../compiler/api/src/query.rs#L76) |

### 20 · 命令、项目与源码输入

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h2001"></a>`H2001` | 补充或改用匹配的命令参数 | 按当前操作选择 scoop build/check、compile、member、output/name/target 等。 | [D0059](BUILD.md#d0059)；[D0068](BUILD.md#d0068)；[D0303](BUILD.md#d0303)；[driver/src/request.rs:306](../../compiler/driver/src/request.rs#L306) |

### 21 · 依赖与包管理

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h2101"></a>`H2101` | 更新依赖解析/锁定结果 | 按现有错误建议运行 pm resolve，必要时去掉 locked/frozen/offline 约束。 | [T0095](TOOLS.md#t0095)；[T0103](TOOLS.md#t0103)；[T0105](TOOLS.md#t0105)；[pm/src/catalog.rs:151](../../compiler/pm/src/catalog.rs#L151) |
| <a id="h2102"></a>`H2102` | 获取缺失包数据 | 按现有建议运行 pm fetch；需要网络时去掉 offline。 | [T0096](TOOLS.md#t0096)；[T0106](TOOLS.md#t0106)；[T0108](TOOLS.md#t0108)；[pm/src/catalog.rs:222](../../compiler/pm/src/catalog.rs#L222) |
| <a id="h2103"></a>`H2103` | 配置依赖 registry | 声明 registry.default 或显式 registry URL。 | [T0111](TOOLS.md#t0111)；[pm/src/catalog/requirements.rs:110](../../compiler/pm/src/catalog/requirements.rs#L110) |

### 23 · 目标、工具链与链接

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="h2301"></a>`H2301` | 配置或安装所需工具/资源 | 设置工具 locator/PATH/LLVM prefix/sysroot 或执行已有准备脚本；不是新安装流程。 | [D0540](BUILD.md#d0540)；[D0547](BUILD.md#d0547)；[D0553](BUILD.md#d0553)；[toolchain/src/debug.rs:98](../../compiler/toolchain/src/debug.rs#L98) |
| <a id="h2302"></a>`H2302` | 重建不相容的依赖/runtime | 仅针对实际消费失败的上游 artifact 或 runtime。 | [A2028](ARTIFACTS.md#a2028)；[T0097](TOOLS.md#t0097)；[linker/src/runtime/index.rs:40](../../compiler/linker/src/runtime/index.rs#L40) |
| <a id="h2303"></a>`H2303` | 移除不允许的 native exports | native-app DLL 只导出规范要求的入口。 | [A1893](ARTIFACTS.md#a1893)；[linker/src/link/pe.rs:84](../../compiler/linker/src/link/pe.rs#L84) |

## 内部错误（13 项）

### 03 · 声明与修饰符

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice0301"></a>`ICE0301` | 阶段输入混用了不同上下文 | AST/source/Cone/world 输入不一致，当前 core 误导入自身协议。 | [F0343](FRONTEND.md#f0343)；[F0346](FRONTEND.md#f0346)；[F0348](FRONTEND.md#f0348)；[hir-lower/src/current_input.rs:187](../../compiler/hir-lower/src/current_input.rs#L187) |

### 04 · 名称、导入与可见性

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice0401"></a>`ICE0401` | 编译器实体身份或源码定位不一致 | 当前声明的 owner/origin/source/span/typed ID 推导违反内部合同。 | [F0267](FRONTEND.md#f0267)；[F0381](FRONTEND.md#f0381)；[F0983](FRONTEND.md#f0983)；[hir-lower/src/conditional_interfaces/identity.rs:56](../../compiler/hir-lower/src/conditional_interfaces/identity.rs#L56) |

### 05 · 类型与转换

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice0501"></a>`ICE0501` | HIR 完备性或已解析语义关系被破坏 | 通过前端检查后的 signature/type/body/receiver 缺失或关系不一致。 | [F0491](FRONTEND.md#f0491)；[F0518](FRONTEND.md#f0518)；[F0538](FRONTEND.md#f0538)；[hir-lower/src/expr/callable_references/constructors/adapter.rs:102](../../compiler/hir-lower/src/expr/callable_references/constructors/adapter.rs#L102) |

### 06 · 泛型、约束与推断

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice0601"></a>`ICE0601` | 类型求解器状态不一致 | session/type parameter/term/application 违反求解器内部合同。 | [F0102](FRONTEND.md#f0102)；[F0103](FRONTEND.md#f0103)；[F0104](FRONTEND.md#f0104)；[hir-lower/src/call_resolution/diagnostics.rs:229](../../compiler/hir-lower/src/call_resolution/diagnostics.rs#L229) |

### 07 · 调用、重载与函数引用

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice0701"></a>`ICE0701` | MIR 变换或调用桥接不一致 | 合法 HIR 降低后，MIR callable/type/control-flow/adapter 合同不能建立。 | [B0867](BACKEND.md#b0867)；[B0869](BACKEND.md#b0869)；[B0870](BACKEND.md#b0870)；[mir-lower/src/cross_cone_bridge.rs:128](../../compiler/mir-lower/src/cross_cone_bridge.rs#L128) |

### 11 · 效果、上下文与 release

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice1101"></a>`ICE1101` | GC/root/statepoint 合同不一致 | 已生成 scan/root plan/live set/storage/relocation/release machine body 与 GC ABI 不相容。 | [B0070](BACKEND.md#b0070)；[B0071](BACKEND.md#b0071)；[B0072](BACKEND.md#b0072)；[codegen/src/callable_runtime_scans.rs:72](../../compiler/codegen/src/callable_runtime_scans.rs#L72) |

### 12 · FFI、原子与原始存储

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice1201"></a>`ICE1201` | LIR 布局或调用 ABI 变换不一致 | 合法 MIR 降低后，layout/callable ABI/descriptor/dispatch 合同不能建立。 | [B0794](BACKEND.md#b0794)；[B0795](BACKEND.md#b0795)；[B0797](BACKEND.md#b0797)；[lir-lower/src/abi/prepare/fields.rs:55](../../compiler/lir-lower/src/abi/prepare/fields.rs#L55) |

### 22 · 进程、I/O 与构建协调

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice2201"></a>`ICE2201` | 驱动、调度或缓存操作状态不一致 | 内部构建图、完成顺序、lock 使用、配套请求计划等违反已有状态合同；不新增缓存检查。 | [D0008](BUILD.md#d0008)；[D0017](BUILD.md#d0017)；[D0018](BUILD.md#d0018)；[driver/src/artifact_production/layout.rs:119](../../compiler/driver/src/artifact_production/layout.rs#L119) |

### 23 · 目标、工具链与链接

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice2301"></a>`ICE2301` | LLVM 生成不变量失败 | 合法 LIR 与 LLVM value/type/function/builder 预期不一致；缺 definition、错误 operand 和 IR 验证失败为原因。 | [B0003](BACKEND.md#b0003)；[B0013](BACKEND.md#b0013)；[B0014](BACKEND.md#b0014)；[codegen/src/abi.rs:284](../../compiler/codegen/src/abi.rs#L284) |
| <a id="ice2302"></a>`ICE2302` | 编译器自产 object 的结构不一致 | object 分区、atom 边界、relocation/stackmap/unwind 改写违反既有产出合同。 | [B0004](BACKEND.md#b0004)；[B0005](BACKEND.md#b0005)；[B0006](BACKEND.md#b0006)；[codegen/src/atom_boundaries.rs:47](../../compiler/codegen/src/atom_boundaries.rs#L47) |

### 24 · 编译产物与跨 Cone 合同

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice2401"></a>`ICE2401` | 编译器自产元数据或产物投影不一致 | 当前 producer/frozen IR 的序列化、section 拼装、publishing 结构合同失败；实际 I/O 失败仍用 E。 | [F0258](FRONTEND.md#f0258)；[F0260](FRONTEND.md#f0260)；[F0349](FRONTEND.md#f0349)；[hir-lower/src/concretize.rs:74](../../compiler/hir-lower/src/concretize.rs#L74) |

### 25 · 工具协议与查询

| 规则编号 | 规则 | 触发条件与归并边界 | 代表证据 |
| --- | --- | --- | --- |
| <a id="ice2501"></a>`ICE2501` | 自产协议消息或诊断位置无效 | 自己构造的响应、diagnostic source/span、encode/projection 不满足协议；外部接收值无效用 E25。 | [D0013](BUILD.md#d0013)；[D0015](BUILD.md#d0015)；[D0044](BUILD.md#d0044)；[driver/src/child_protocol.rs:240](../../compiler/driver/src/child_protocol.rs#L240) |
| <a id="ice2502"></a>`ICE2502` | 查询实现内部关系缺失 | 已有完整语义世界中的 source origin/projection 等不一致；不完整编辑输入本身不是 ICE。 | [T0012](TOOLS.md#t0012)；[T0013](TOOLS.md#t0013)；[T0051](TOOLS.md#t0051)；[analysis/src/semantic/rename/anchors.rs:69](../../compiler/analysis/src/semantic/rename/anchors.rs#L69) |
