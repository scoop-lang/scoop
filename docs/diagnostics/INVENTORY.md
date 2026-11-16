# Scoop 诊断清单初稿

盘点日期：2026-11-16。源码基线：`01fe6066af05e06aaf9998c20d147f662dcb596c`。本次只建立清单，未改变 spec、编译器实现、诊断文案或编号。

**本次静态清单收录 5,960 条错误相关文案模板、14 条 warning 模板、19 条结构化 note 模板；另列 22 条内嵌说明/帮助片段与 4 条文档警告原因。** 错误相关文案包括主消息、具体原因、外层包装及内部格式/合同错误，**不是 5,960 个相互独立的语言错误规则，也不是建议分配这么多个错误码**。

**第二轮已完成规则归并与编号提案：见 [编号方案](NUMBERING.md)、[规则目录](RULES.md) 和 [完整对照表](RULE-MAPPING.md)。下文保留第一轮的原始文案计数；第二轮排除了误收项，并将附加原因、包装及内部错误单独归类。**

## 计数口径

- 范围为 `compiler/` 中的 Rust 实现，包括 scoopc、scoop、链接器、产物 reader、包管理和编辑器工具；配套工具单列。覆盖源码中的不同平台分支，不限于当前 macOS 可运行的分支。
- 按“严重级别 + 文案模板”去重；格式化参数名、位置及格式说明的差别不另算一项。例如 `unknown type {}` 与 `unknown type {name}` 合并，并保留两处实现位置。字面量中的普通花括号保留。
- 保留不同措辞和不同包装。例如“参数类型不符”和“无适用重载，原因是参数类型不符”可能共用将来的规则编号；本次不凭文案擅自合并语义。
- 排除测试中的模拟消息、断言/panic/expect 文案、正常进度与 dump 输出、类型/签名的普通显示文本和只用作参数的名称标签。parser 的 136 个静态 expected-token/expected-name 描述见 [语法预期参数](SYNTAX-EXPECTATIONS.md)，不把每个 token 名都当成新规则。
- 盘点覆盖诊断构造器、前端诊断辅助函数、错误类型 Display、常见错误构造/传播入口，并对照 fixture 诊断预期。它是可供分类的静态初稿；没有逐个证明内部错误分支都能从公开 CLI 到达。
- 第三方库、LLVM/系统链接器、操作系统 I/O、外部 generator/子进程可以透传任意文本。这里记录 Scoop 自己的包装与已写出的文案，不把这些动态文本枚举成固定错误种类。runtime trap、应用异常、sysroot 库和仓库外 generator 自身的诊断不在本次范围。

第一轮计数只表示文案基数。第二轮按规则与诊断角色整理的编号数见 [编号方案](NUMBERING.md)；编号数不是这些模板的简单去重结果，也不等于编译时同时输出的诊断条数。

## 分类总表

| 清单 | 错误相关模板数 | 完整明细 |
| --- | --- | --- |
| 前端错误文案 | 1524 | [FRONTEND.md](FRONTEND.md) |
| MIR LIR 下沉与代码生成错误文案 | 880 | [BACKEND.md](BACKEND.md) |
| IR 实体产物与链接错误文案 | 2682 | [ARTIFACTS.md](ARTIFACTS.md) |
| 构建驱动清单协议与工具链错误文案 | 648 | [BUILD.md](BUILD.md) |
| 包管理编辑器与 API 工具错误文案 | 226 | [TOOLS.md](TOOLS.md) |


各表的数字已跨实现位置去重；一个模板在多个模块出现时，只按首个收录来源归入一个表，行中保留所有已收录位置。

| 其他信息 | 数量 | 性质 |
| --- | ---: | --- |
| warning | 14 | 有严重级别的主诊断；当前编译协议统一使用 `SCOOPC_COMPILER_WARNING` |
| note | 19 | 附属于主诊断，其中 18 条来自编译前端/driver，1 条来自包管理 |
| 内嵌说明/帮助片段 | 22 | 解释重载、入口资格、转换建议等；不具有独立严重级别 |
| 文档引用 warning 的原因 | 4 | 代入同一个 `documentation reference …` 模板，不额外计入 14 条 warning |
| 独立 info/hint/help/note 主诊断级别 | 0 | 源码和编译协议的 severity 枚举只有 Error、Warning |
| 已有诊断/能力标识 | 96 | 见 [已有标识](EXISTING-CODES.md)，不与文案数相加 |
| 错误/失败相关枚举 | 816 个类型、4,901 个变体 | 见 [错误类型](ERROR-TYPES.md)，与文案是不同维度，不相加 |

严重级别依据：[source/src/diagnostic.rs:8](../../compiler/source/src/diagnostic.rs#L8)、[protocol/src/diagnostic.rs:14](../../compiler/protocol/src/diagnostic.rs#L14)。warning 代码统一转换：[driver/src/request/report/protocol.rs:91](../../compiler/driver/src/request/report/protocol.rs#L91)。

## Warning 完整清单

| 序号 | 含义 | 英文模板 | 实现位置 |
| --- | --- | --- | --- |
| <a id="w01"></a>1 | 文档中的符号引用语法无效 | `` invalid documentation symbol `{id}` `` | [doc/src/markdown.rs:71](../../compiler/doc/src/markdown.rs#L71) |
| <a id="w02"></a>2 | 文档别名为空白 | ` documentation alias must not be blank ` | [doc/src/project/annotations.rs:36](../../compiler/doc/src/project/annotations.rs#L36) |
| <a id="w03"></a>3 | 文档注释没有附着到声明 | ` documentation comment is not attached to a declaration ` | [doc/src/syntax.rs:22](../../compiler/doc/src/syntax.rs#L22) |
| <a id="w04"></a>4 | @param/@property 应指定单个参数名 | `` `@{}` requires one parameter name `` | [doc/src/tags.rs:42](../../compiler/doc/src/tags.rs#L42) |
| <a id="w05"></a>5 | @since 缺少版本文字 | `` `@since` requires version text `` | [doc/src/tags.rs:78](../../compiler/doc/src/tags.rs#L78) |
| <a id="w06"></a>6 | 未知文档标签 | `` unknown documentation tag `@{name}` `` | [doc/src/tags.rs:85](../../compiler/doc/src/tags.rs#L85) |
| <a id="w07"></a>7 | 文档标签缺少名称 | `` `@{}` requires a name `` | [doc/src/tags.rs:116](../../compiler/doc/src/tags.rs#L116) |
| <a id="w08"></a>8 | 文档标签的名称语法无效 | `` invalid name for documentation tag `@{}` `` | [doc/src/tags.rs:124](../../compiler/doc/src/tags.rs#L124) |
| <a id="w09"></a>9 | 单文件模式忽略 package 声明 | ` package declaration is ignored in single-file mode; using the default root package ` | [driver/src/request/preflight/packages.rs:69](../../compiler/driver/src/request/preflight/packages.rs#L69) |
| <a id="w10"></a>10 | 重复文档标签 | `` duplicate documentation tag `{}` `` | [hir-lower/src/documentation/binding.rs:155](../../compiler/hir-lower/src/documentation/binding.rs#L155) |
| <a id="w11"></a>11 | 文档符号引用无法绑定，四种原因见下表 | `` documentation reference `{name}` {reason} `` | [hir-lower/src/documentation/binding.rs:221](../../compiler/hir-lower/src/documentation/binding.rs#L221) |
| <a id="w12"></a>12 | 文档标签没有适用的目标 | `` documentation tag has no applicable target `{label}` `` | [hir-lower/src/documentation/binding/tags.rs:115](../../compiler/hir-lower/src/documentation/binding/tags.rs#L115) |
| <a id="w13"></a>13 | 同一目标有重复说明，保留第一份 | `` duplicate documentation for `{label}`; the first direct or tagged description is retained `` | [hir-lower/src/documentation/binding/tags.rs:122](../../compiler/hir-lower/src/documentation/binding/tags.rs#L122) |
| <a id="w14"></a>14 | 绑定名称遮蔽当前位置可用的枚举变体 | `` binding `{}` shadows an enum variant available here; rename the binding or qualify the variant `` | [hir-lower/src/patterns/warnings.rs:69](../../compiler/hir-lower/src/patterns/warnings.rs#L69) |


文档引用模板中的 `reason` 有以下四种值。它们是同一 warning 的细分原因，可在确定编号粒度时一起讨论：

| 文档引用失败原因 | 实现位置 |
| --- | --- |
| <a id="r01"></a>` names a package, not a declaration ` | [hir-lower/src/documentation/symbols.rs:25](../../compiler/hir-lower/src/documentation/symbols.rs#L25) |
| <a id="r02"></a>` cannot be resolved ` | [hir-lower/src/documentation/symbols.rs:35](../../compiler/hir-lower/src/documentation/symbols.rs#L35)；[hir-lower/src/documentation/symbols.rs:135](../../compiler/hir-lower/src/documentation/symbols.rs#L135) |
| <a id="r03"></a>` has an ambiguous owner ` | [hir-lower/src/documentation/symbols.rs:46](../../compiler/hir-lower/src/documentation/symbols.rs#L46) |
| <a id="r04"></a>` is ambiguous ` | [hir-lower/src/documentation/symbols.rs:151](../../compiler/hir-lower/src/documentation/symbols.rs#L151) |


## 结构化 Note 完整清单

这些是 related location 的文案，附加位置不能替代主诊断位置。import 建议还携带结构化 `import_path`。同一条 note 被多个主诊断使用时，本表只列一次。

| 序号 | 含义 | 英文模板 | 实现位置 |
| --- | --- | --- | --- |
| <a id="n01"></a>1 | 前一个生成器接口绑定的位置 | ` previous interface binding is here ` | [driver/src/generation/plan.rs:108](../../compiler/driver/src/generation/plan.rs#L108) |
| <a id="n02"></a>2 | 前一个生成器方法绑定的位置 | ` previous method binding is here ` | [driver/src/generation/plan.rs:126](../../compiler/driver/src/generation/plan.rs#L126) |
| <a id="n03"></a>3 | 冲突的另一份条件接口实现 | ` the other implementation is here ` | [hir-lower/src/conditional_interfaces/overlap.rs:49](../../compiler/hir-lower/src/conditional_interfaces/overlap.rs#L49) |
| <a id="n04"></a>4 | 该声明的签名未能解析 | ` this declaration's signature could not be resolved ` | [hir-lower/src/declaration_surface.rs:150](../../compiler/hir-lower/src/declaration_surface.rs#L150) |
| <a id="n05"></a>5 | 请求生成实现的位置 | ` implementation requested here ` | [driver/src/generation/external/diagnostics.rs:30](../../compiler/driver/src/generation/external/diagnostics.rs#L30)；[driver/src/generation/external/fragments.rs:89](../../compiler/driver/src/generation/external/fragments.rs#L89)；[hir-lower/src/generation.rs:111](../../compiler/hir-lower/src/generation.rs#L111) |
| <a id="n06"></a>6 | 候选声明的位置 | ` candidate declared here ` | [hir-lower/src/imports/lookup/diagnostics.rs:37](../../compiler/hir-lower/src/imports/lookup/diagnostics.rs#L37)；[hir-lower/src/imports/lookup/diagnostics.rs:98](../../compiler/hir-lower/src/imports/lookup/diagnostics.rs#L98)；[hir-lower/src/imports/lookup/values/diagnostics.rs:56](../../compiler/hir-lower/src/imports/lookup/values/diagnostics.rs#L56) |
| <a id="n07"></a>7 | 相关 import 未能解析 | ` this import could not be resolved ` | [hir-lower/src/imports/lookup/unavailable.rs:192](../../compiler/hir-lower/src/imports/lookup/unavailable.rs#L192) |
| <a id="n08"></a>8 | 相关声明没有完整语义输入 | ` this declaration has no complete semantic input ` | [hir-lower/src/imports/lookup/unavailable.rs:194](../../compiler/hir-lower/src/imports/lookup/unavailable.rs#L194) |
| <a id="n09"></a>9 | 相关声明超出本文件的访问域 | ` declaration is outside this file's permitted access domain ` | [hir-lower/src/imports/resolve.rs:128](../../compiler/hir-lower/src/imports/resolve.rs#L128) |
| <a id="n10"></a>10 | 可通过该 import 使用声明 | ` available with import {rendered} ` | [hir-lower/src/imports/suggestions.rs:82](../../compiler/hir-lower/src/imports/suggestions.rs#L82) |
| <a id="n11"></a>11 | 另一个入口候选的位置 | ` another {kind} entry candidate is declared here ` | [hir-lower/src/output_kind.rs:73](../../compiler/hir-lower/src/output_kind.rs#L73) |
| <a id="n12"></a>12 | 入口声明不合格的原因 | `` `{}` is not eligible because {} `` | [hir-lower/src/output_kind.rs:286](../../compiler/hir-lower/src/output_kind.rs#L286) |
| <a id="n13"></a>13 | 入口函数签名不合格的原因 | `` `fun {}({}): {}` is not eligible because {} `` | [hir-lower/src/output_kind.rs:296](../../compiler/hir-lower/src/output_kind.rs#L296) |
| <a id="n14"></a>14 | 另一份测试生命周期方法 | ` the other lifecycle method is declared here ` | [hir-lower/src/test_annotations.rs:158](../../compiler/hir-lower/src/test_annotations.rs#L158) |
| <a id="n15"></a>15 | 前一个负接口约束 | ` previous negative bound is here ` | [hir-lower/src/types/constraints/negative.rs:43](../../compiler/hir-lower/src/types/constraints/negative.rs#L43) |
| <a id="n16"></a>16 | 发生冲突的上界约束 | ` conflicting upper bound is here ` | [hir-lower/src/types/constraints/negative.rs:93](../../compiler/hir-lower/src/types/constraints/negative.rs#L93) |
| <a id="n17"></a>17 | 前一个成员 requirement | ` previous requirement is here ` | [hir-lower/src/types/constraints/requirements.rs:44](../../compiler/hir-lower/src/types/constraints/requirements.rs#L44) |
| <a id="n18"></a>18 | 第一条 package 声明 | `` first `package` header is here `` | [parser/src/parser/file.rs:46](../../compiler/parser/src/parser/file.rs#L46) |
| <a id="n19"></a>19 | 包管理错误中的相关声明 | ` related declaration ` | [pm/src/error.rs:71](../../compiler/pm/src/error.rs#L71) |


## 正文内的说明和帮助

以下是独立拼接的说明片段；主错误句子自带的 `use …`、`rename …` 等修复建议保留在相应主模板中，不再次拆开计数。三条 intrinsic 文案包含字面量 `help:`，但不是独立的 Help 诊断。

| 序号 | 正文中的说明/帮助片段 | 实现位置 |
| --- | --- | --- |
| <a id="p01"></a>1 | ` this intrinsic is evaluated at compile time and has no runtime callable entry\nhelp: import and call it directly with a constant path ` | [hir-lower/src/expr/callable_references/intrinsics.rs:35](../../compiler/hir-lower/src/expr/callable_references/intrinsics.rs#L35) |
| <a id="p02"></a>2 | ` this intrinsic is evaluated at compile time and has no runtime callable entry\nhelp: import and call it directly with a constant name ` | [hir-lower/src/expr/callable_references/intrinsics.rs:38](../../compiler/hir-lower/src/expr/callable_references/intrinsics.rs#L38) |
| <a id="p03"></a>3 | ` this intrinsic requires the source location of a direct call; indirect calls do not provide it\nhelp: call it directly where the location is needed, or pass a SourceLocation value explicitly ` | [hir-lower/src/expr/callable_references/intrinsics.rs:41](../../compiler/hir-lower/src/expr/callable_references/intrinsics.rs#L41) |
| <a id="p04"></a>4 | ` matches the expected function type ` | [hir-lower/src/expr/callable_references/resolution/diagnostics.rs:83](../../compiler/hir-lower/src/expr/callable_references/resolution/diagnostics.rs#L83) |
| <a id="p05"></a>5 | ` forms a complete non-generic function type ` | [hir-lower/src/expr/callable_references/resolution/diagnostics.rs:85](../../compiler/hir-lower/src/expr/callable_references/resolution/diagnostics.rs#L85) |
| <a id="p06"></a>6 | ` it is generic ` | [hir-lower/src/output_kind.rs:163](../../compiler/hir-lower/src/output_kind.rs#L163) |
| <a id="p07"></a>7 | ` it is suspend ` | [hir-lower/src/output_kind.rs:166](../../compiler/hir-lower/src/output_kind.rs#L166) |
| <a id="p08"></a>8 | `` its parameters must be two ordinary `Ptr<Unit>` values `` | [hir-lower/src/output_kind.rs:190](../../compiler/hir-lower/src/output_kind.rs#L190) |
| <a id="p09"></a>9 | `` its parameters must be empty or one `Array<String>` `` | [hir-lower/src/output_kind.rs:192](../../compiler/hir-lower/src/output_kind.rs#L192) |
| <a id="p10"></a>10 | ` it has a vararg parameter ` | [hir-lower/src/output_kind.rs:204](../../compiler/hir-lower/src/output_kind.rs#L204) |
| <a id="p11"></a>11 | `` it must return `Unit` `` | [hir-lower/src/output_kind.rs:214](../../compiler/hir-lower/src/output_kind.rs#L214) |
| <a id="p12"></a>12 | `` it must return `Unit` or `Int` `` | [hir-lower/src/output_kind.rs:216](../../compiler/hir-lower/src/output_kind.rs#L216) |
| <a id="p13"></a>13 | ` it has a receiver ` | [hir-lower/src/output_kind.rs:224](../../compiler/hir-lower/src/output_kind.rs#L224) |
| <a id="p14"></a>14 | ` it must be internal or public ` | [hir-lower/src/output_kind.rs:230](../../compiler/hir-lower/src/output_kind.rs#L230) |
| <a id="p15"></a>15 | ` it has no ordinary Scoop body ` | [hir-lower/src/output_kind.rs:234](../../compiler/hir-lower/src/output_kind.rs#L234) |
| <a id="p16"></a>16 | `   - {signature} — {reason} ` | [hir-lower/src/overload/diagnostics.rs:120](../../compiler/hir-lower/src/overload/diagnostics.rs#L120) |
| <a id="p17"></a>17 | `   - {} — tied after pairwise declaration forwarding{} ` | [hir-lower/src/overload/diagnostics.rs:165](../../compiler/hir-lower/src/overload/diagnostics.rs#L165) |
| <a id="p18"></a>18 | ` {parameter} (expected {}): {reason} ` | [hir-lower/src/overload/diagnostics.rs:223](../../compiler/hir-lower/src/overload/diagnostics.rs#L223) |
| <a id="p19"></a>19 | ` {reason}; fixed type arguments: {} ` | [hir-lower/src/overload/diagnostics.rs:273](../../compiler/hir-lower/src/overload/diagnostics.rs#L273) |
| <a id="p20"></a>20 | `` argument for `{}`{literal} = {} `` | [hir-lower/src/overload/diagnostics/integers.rs:52](../../compiler/hir-lower/src/overload/diagnostics/integers.rs#L52) |
| <a id="p21"></a>21 | `` ; primitive integer operands require one exact type; convert this operand explicitly with `{}()` `` | [hir-lower/src/overload/diagnostics/integers.rs:262](../../compiler/hir-lower/src/overload/diagnostics/integers.rs#L262) |
| <a id="p22"></a>22 | ` ; guarded arms do not contribute to exhaustiveness ` | [hir-lower/src/patterns/exhaustiveness.rs:46](../../compiler/hir-lower/src/patterns/exhaustiveness.rs#L46)；[hir-lower/src/stmt/when/sealed.rs:117](../../compiler/hir-lower/src/stmt/when/sealed.rs#L117) |


## 分类编号时需要明确的边界

1. **优先按语言规则编号。** 名称查找、可见性、类型匹配、调用匹配、泛型约束、控制流、模式、FFI/NoGC、构建输入、产物格式和内部编译器错误应分开处理；crate 只是当前实现归属。
2. **主诊断与原因分开。** 重载/构造器失败有顶层消息、候选签名和具体拒绝原因；不要机械地为每一个格式化片段分配同级错误码。
3. **源码错误与内部错误分开。** 例如 codegen 的无效 LIR 操作数、缺失 LLVM 函数、产物 reader 的索引/ABI 错误，不应与用户源码类型错误混成一个分类。
4. **已有代码大多粒度很粗。** `SCOOPC_PARSE_ERROR`、`SCOOPC_HIR_ERROR`、`SCOOPC_BUILD_FAILED` 与 `SCOOPC_COMPILER_WARNING` 代表阶段或兜底分类；已有 96 个标识不能代表只有 96 种诊断。
5. **Error 枚举变体是补充线索。** 有些 Display 只输出 `{self:?}`，实际差别在变体；也有许多变体只是包装下游错误。需要结合调用入口和格式化方式决定是否有独立编号。

## 核对范围

使用 Rust 语法树提取，排除测试模块与测试支持文件，再人工核对前端辅助消息、warning/note 和正文提示。对照了 `tests/fixtures` 中 3,054 条带 severity 的诊断记录；其中 2,981 条可直接与模板整句匹配，其余涉及路径/上下文拼接、fixture 转义或第三方透传，未将它们机械地各算作新模板。

这不是完整的测试覆盖率统计，也没有重跑 fixture 或编译器测试。清单的分类与总数已做一致性核对；第二轮提供了全部文案行的处置及共用校验器的边界规则；采用提案时仍需按具体发出位置接入 typed error，不能按英文字符串批量替换。原始扫描数据与一次性盘点程序在仓库外，仓库仅新增本组清单。
