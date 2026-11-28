# Scoop 诊断分类与编号方案

状态：**编号已接入，本机全量验收与 Linux、Windows 平台验收全部通过**。原始清点基于 2026-11-16 工作区、源码基线 `01fe6066af05e06aaf9998c20d147f662dcb596c`；实现分支基于结构性修复合并后的 `ad9ae0d8a802f990ce3d0c11474d983bc182f25a`。范围沿用[初稿](INVENTORY.md)：编译流水线及 scoop、包管理、链接器、文档、LSP/API 配套工具。

当前注册 **421 个编号：348 个 E、16 个 W、14 个 N、30 个 H、13 个 ICE**。其中 E 的 01–18 类为语言及编译相关规则 269 项，20–25 类为项目、依赖、构建、产物及工具规则 79 项。

这些是**主规则/附属信息编号的注册数量**，也不是把 5,960 条文案逐句去重后得到的“5,960 种错误”。有些合同有许多不同的失败原因，但只需要一个主诊断编号。

- [完整规则目录](RULES.md)：每个编号、中文含义、触发条件、合并边界和代表源码位置。
- [完整归并对照](RULE-MAPPING.md)：覆盖初稿全部 5,960 条错误相关文案，以及 warning/note/内嵌片段；保留原行号与英文文案。
- [旧标识归并方向](CODE-MIGRATION.md)：96 个既有标识逐一处理。
- [13 类 ICE 的结构性复核](ICE-ANALYSIS.md)：区分数据表示问题、必要检查和原映射过宽的情况。13 类不等于 13 个缺陷；ICE 边界以复核意见为准，实际边界已经在编号接入中分流。

## 编号格式

采用 **前缀 + 两位分类号 + 两位规则序号**。例如 `E0501` 中 `05` 是类型与转换，`01` 是类型不匹配；完整码在各工具中相同，不额外加 `SCOOPC_` 或 stage 名。

| 前缀 | 本版数量 | 意义 | 输出角色 |
| --- | ---: | --- | --- |
| E | 348 | 源码、输入、配置或外部操作违反明确规则 | 主错误；也可作为候选/聚合错误的具体原因 |
| W | 16 | 当前实现已有的警告规则 | 主警告；被 deny 时仍保留 W 编号 |
| N | 14 | 相关位置、候选背景、解释 | 附属于主诊断 |
| H | 30 | 可采取的修复动作 | 附属于主诊断；不代表自动修改 |
| ICE | 13 | 编译器内部不变量/阶段合同失败 | 内部错误，和用户输入错误分开 |

前缀表达**诊断类别**，不是新增五个 severity。当前 [source](../../compiler/source/src/diagnostic.rs#L8) 和 [protocol](../../compiler/protocol/src/diagnostic.rs#L14) 的严重级别只有 Error/Warning；N/H 是附属角色，ICE 的严重性仍是 error。目前独立 info/hint 主诊断仍是 **0**。

同一语义规则换措辞、国际化、移动 crate 或在导入依赖中触发，都不改编号。只有触发条件或修复含义不同才分码。项目尚未发布，可按实际规则直接调整目录；不增加旧码 alias、转发兼容层或双轨输出。

## 按什么合并

本轮做了两种不同处理。

1. **同义规则归并。** 触发谓词相同，只有声明种类、所属阶段、local/imported、expected/found 的呈现不同，就使用同一编号。
2. **主诊断与原因分层。** 一个固定合同可以有多个不同失败原因。例如无适用重载、FFI 签名不合格、产物格式无效。这些原因完整保留，但不机械地各分一个同级主码。这不意味着不同原因是同义词。

| 原文案/规则 | 候选归属 | 保留的差异 |
| --- | --- | --- |
| expression has type … expected …；function argument …；initializer …；lambda result … | [E0501](RULES.md#e0501) | expected、found、关系种类、参数/返回/初始化位置 |
| struct/class/interface/enum/typealias takes N type arguments；is not generic | [E0601](RULES.md#e0601) | 目标、实际/所需数量、宿主应用信息 |
| unknown variable/function/type/annotation | [E0401](RULES.md#e0401) | 名称域和查找位置 |
| unknown parameter；missing required；duplicate argument | [E0702](RULES.md#e0702)、[E0708](RULES.md#e0708)、[E0703](RULES.md#e0703) | 三条实际参数映射规则，不合成“调用参数错误” |
| no applicable function/constructor/callable reference | [E0711](RULES.md#e0711) | 查找层、候选签名、每个候选的具体失败原因 |
| overload ambiguous 与 name lookup ambiguous | [E0712](RULES.md#e0712)、[E0404](RULES.md#e0404) | 一个是适用性/排序，另一个是名称绑定 |
| nominal invariance detail | [E0501](RULES.md#e0501) + [N0501](RULES.md#n0501) + [H0501](RULES.md#h0501) | 主错误仍是类型关系失败，附注指出首个不同实参 |
| non-exhaustive when 的 missing pattern / subtype family / else | [E0904](RULES.md#e0904) | 缺失 witness 与 subject/subjectless 形态 |
| parser expected token/name/list separator | [E0201](RULES.md#e0201) | [136 个 expected 参数](SYNTAX-EXPECTATIONS.md)作为数据，不各编一个码 |
| WireError 的 unknown tag / wrong length / duplicate field | [E2401](RULES.md#e2401) 或 [E2502](RULES.md#e2502) 的原因 | 检查对象决定外层主码；tag、长度、字段、offset 保留 |

覆盖返回类型、property override、context 精确顺序等是**继承合同**，与普通表达式赋值的 subtype/assignability 检查不同，仍使用 [E1013](RULES.md#e1013)。编译器内建 core 声明的规定签名归 [E1403](RULES.md#e1403)，不会仅因出现 `expected` 就套类型不匹配。

## 主错误、附加原因和内部错误

### 主诊断

主诊断表示这一次失败的直接规则。源码 token/类型/访问/调用错误优先定位到用户能修改的位置；工具 I/O/target/包输入错误给出实际输入和操作。普通失败不能因为发生在 codegen、reader 或 native 工具阶段就变成 ICE。

阶段计数与包装，如 `current Cone HIR lowering failed with N diagnostics`、`typealias X: {reason}`、`cannot execute source Cone: {source}`，继续携带原来的诊断，不新增一个“总失败”编号。

### 附加原因

“原因”是**角色**，不再开一套 R 错误码：如果原因本身就是目录中的规则，就复用它的 E 编号。只在解释位置、候选状态、guard、invariance 等本身不是另一条错误时使用 N；可执行的修改建议使用 H。

诊断按下列层级呈现：

```text
error[E0711]: no applicable candidate for `f`
  note[N0002]: fun f(x: Int): Unit
    cause[E0501]: expected Int, found Long
    help[H0501]: convert this operand explicitly with `toInt()`
```

应沿用现有错误选择行为：[overload diagnostics](../../compiler/hir-lower/src/overload/diagnostics.rs#L31) 对单个效果不允许的候选直接报效果错误；如果所有候选共享同一个与上下文无关的实参根错误，则报根错误；依赖已失败声明时抑制重复报错。只有确实是候选集合失败时才报“无适用候选”。[constructor resolution](../../compiler/hir-lower/src/constructor_resolution.rs#L100) 也区分单候选失败与多候选原因链。编号接入不能改变这套选择逻辑。

### 内部错误

ICE 指当前编译器在已满足前提的输入上破坏了自身合同，例如：

- `ICE0601`：候选引用另一求解 session 的变量、产生不完整具体类型。
- `ICE0501`：通过语义检查的 HIR 缺少必要 receiver、signature 或 body 关系。
- `ICE1201`：已降低的数据缺布局、ABI 或描述符。
- `ICE2301`：LLVM operand/type/definition 与合法 LIR 不一致。
- `ICE1101`：root plan、live set、statepoint 或 release machine body 不符合 GC 合同。
- `ICE2401`：本次编译器自产产物的元数据投影/组装结构不一致。

ICE 的细节仍保留 operation、typed ID、原错误变体与发生位置。它不能伪装成“参数不适用”而仅作为普通重载候选拒绝；也不能将“请修改源代码以绕过缺失 LLVM 函数”作为通常的 H 建议。

### 产物校验的边界

同一个校验器可能用于外部 `.slib` reader 和当前 producer。仅凭 `invalid`、`missing` 或 crate 名不能判定错误归属。

| 实际边界 | 编号方向 |
| --- | --- |
| 首次消费外部 `.slib` 时发现其接口/模板/布局无效，即使是延迟解码 | E24，原字段校验为原因 |
| 已被接受且未变化的数据，经过内部选择/变换后丢失关系 | 对应 ICE |
| 当前编译器刚生成的对象、IR 或元数据无法满足自身合同 | 对应 ICE |
| 外部工具返回不合法 frame/result | E25 |
| 自己构造的请求、诊断或响应不满足协议 | `ICE2501` |
| 真正的文件读写、进程失败、allocation 或有限索引表示不足 | E22，不因为返回它的是 IR crate 就归 ICE |

完整对照中 **2,449 条**标为“按边界”，是共享数据合同/包装的上下文路由，不为这些文案另分编号。映射表给出应选的 E/ICE 合同。已有下游源码诊断、I/O、资源或 capability 错误优先透传，不被外层边界码覆盖。

这里的边界使用既有 caller、输入类型和 error chain。**不新增来源认证、产物证明、通用资源预算、缓存命中内容复核或重复语义重放**。既有低层检查的合理性和是否应保留是另一项实现审查，本次编号不为其增加新的规范地位。

## 错误分类目录

分类按失败问题组织，不按当前 crate 排列。具体每一项都在 [规则目录](RULES.md)。

| 分类号 | 分类 | 本版 E 范围 | 项数 |
| --- | --- | --- | ---: |
| 01 | 词法与字面量 | E0101–E0110 | 10 |
| 02 | 语法结构 | E0201–E0208 | 8 |
| 03 | 声明与修饰符 | E0301–E0311 | 11 |
| 04 | 名称、导入与可见性 | E0401–E0411 | 11 |
| 05 | 类型与转换 | E0501–E0511 | 11 |
| 06 | 泛型、约束与推断 | E0601–E0624 | 24 |
| 07 | 调用、重载与函数引用 | E0701–E0732 | 32 |
| 08 | 控制流与异常 | E0801–E0814 | 14 |
| 09 | 模式与穷尽性 | E0901–E0916 | 16 |
| 10 | 类、继承与初始化 | E1001–E1025 | 25 |
| 11 | 效果、上下文与 release | E1101–E1118 | 18 |
| 12 | FFI、原子与原始存储 | E1201–E1223 | 23 |
| 13 | 注解与自动派生 | E1301–E1318 | 18 |
| 14 | core 与编译器内建声明 | E1401–E1406 | 6 |
| 15 | 属性、常量与值语义 | E1501–E1526 | 26 |
| 16 | 编译期输入 | E1601–E1607 | 7 |
| 17 | 入口与测试声明 | E1701–E1708 | 8 |
| 18 | 文档 | E1801–E1801 | 1 |
| 20 | 命令、项目与源码输入 | E2001–E2020 | 20 |
| 21 | 依赖与包管理 | E2101–E2121 | 21 |
| 22 | 进程、I/O 与构建协调 | E2201–E2206 | 6 |
| 23 | 目标、工具链与链接 | E2301–E2308 | 8 |
| 24 | 编译产物与跨 Cone 合同 | E2401–E2412 | 12 |
| 25 | 工具协议与查询 | E2501–E2512 | 12 |
| 合计 | | | 348 |

## 警告、说明与建议

**16 个 W**：文档 13 项、单文件 package 被忽略 1 项、模式绑定遮蔽 enum variant 1 项、未读取的局部写入 1 项。文档引用的一个原模板根据实际 reason 拆为“引用 package”“未解析”“歧义”三个规则；重复 reference tag 与重复目标说明合并，标签名称格式文案合并。原清点的 14 条 warning 模板之外，接入时发现 README 本地资源缺失实际也是 warning，初稿误归入 E1801；补记 W1813，保留既有严重性，不新增 warning 行为。

文档/编译 warning 被 deny 时，仍保留 W 编号和原位置，输出/退出状态表达其导致失败；不将所有 warning 改成一个 `documentation warnings are denied` 错误码，也不擅自将现在的 useless type check/cast 等 error 降成 warning。

**14 个 N** 合并了之前的 19 条 related-note 与部分正文说明。前一声明、另一约束/绑定/生命周期方法等共用 `N0001`，通过关系参数说明位置作用。入口的十种不合格原因统一挂在 `E1702` 下，用 `N1701` 表达。

**30 个 H** 同时覆盖独立拼接的 help 和主文案中已有的修复建议。包括 import、显式类型、数值转换、lambda 包装、变体构造、明确 receiver、模式写法、release/捕获快照、派生实现、依赖 resolve/fetch 以及工具配置。没有新增“unused variable”“unreachable statement”“deprecated”等尚不存在的告警功能。

## 对初稿的校正

| 原项 | 本轮处置 | 依据 |
| --- | --- | --- |
| F0186、F0188：decode variant 失败 | 排除出编译诊断 | [生成运行时 decode failure](../../compiler/hir-lower/src/class/inheritance/decoding/variants.rs#L29) |
| F0211：inspection metadata 不符 | 排除出编译诊断 | [构造 IllegalStateException 并生成 throw](../../compiler/hir-lower/src/class/inheritance/inspection/expressions.rs#L140) |
| F0141：struct secondary this delegation | 仅为上下文标签 | [传入委托检查的标签](../../compiler/hir-lower/src/class/construction/structs.rs#L63) |
| D0350：测试报告中的失败位置打印 | 排除出编译诊断 | [运行结果展示](../../compiler/scoop/src/cli/presentation/test_report/human.rs#L84) |
| F0019–F0021：local top-level val/var/initializer 文案 | 限定在 raw storage 路径 | [唯一调用入口](../../compiler/hir-lower/src/globals/raw.rs#L56)，不能据此认定普通顶层 val/var 不可用 |
| F0596：rounding produces infinity | 数值范围主错误的原因 | [parse_decimal 的调用](../../compiler/hir-lower/src/expr/floating.rs#L32) |
| F1278、T0155 | 附注/建议 | 不变性的具体类型差异、offline 解析范围说明 |
| T0008、T0068、D0213 | 取消/中断状态 | 不给取消行为编一个源程序错误码 |

初稿 5,960 行保留以便复核；“排除”不删除证据。本轮的 5,960 行处置相加仍为 5,960：2,091 条可作主/因、29 条明确原因、1,301 条内部合同、2,449 条按边界、其余 90 条为包装/附注/建议/策略/状态/排版/排除项。

## 接入原则与本轮验证

本分支先在 [编译器规范](../specs/SCOOP-IMPL-SPEC.md) §2.11 明确 code、附注/建议及内部错误呈现合同，再接入实现；语言语义、[runtime 契约](../specs/SCOOP-RUNTIME-SPEC.md) 和错误选择行为均不因编号自行改变。

接入应从触发规则的 typed error/diagnostic 构造点携带编号，通过 driver、CLI、LSP 和 generator 协议传播；不要在输出末端按英文文案猜码。每条已有子诊断与 related source/import_path 应保留。第三方工具的任意 stderr、generator 自定义消息和应用运行时异常不纳入 Scoop 固定规则数量；显示时保留其来源。

[816 个错误类型、4,901 个枚举变体](ERROR-TYPES.md)是错误合同和具体原因的证据，和主码不一一对应，数量不相加。`Display` 只显示 `{self:?}` 的类型，也应在所属合同下保留变体/字段，不按每个变体机械分号。

核对完成：原 5,960 行无缺失、无重复处置；59 条 warning/note/内嵌片段/文档原因均有归属；421 个编号唯一；96 个旧标识都有处置方向。源码触发条件和关键调用边界已用于归并，未通过执行构造所有非法产物来证明每个静态分支都能由公开 CLI 到达。共享校验器的编号以表中边界规则为准。

初版方案的文档链接、锚点、表格和计数已做静态校验。编号接入使用 `scoop-source::DiagnosticCode` 作为唯一注册表；compiler/package/generator 协议分别升级为 32/8/2，Scoop CLI 和 LSP query JSON 使用 schema 2。诊断缓存命名空间同步更新，普通缓存命中行为不变。验收结果如下，原始日志与临时工作区均保存在仓库外。

| 宿主与范围 | 结果 |
| --- | --- |
| macOS ARM64：完整 Rust workspace（含 doc-tests） | 6,483 通过，0 失败，0 忽略，111 个测试套件 |
| macOS ARM64：完整 fixture | 4,347 项中 4,319 通过；28 项仅适用于 Linux/Windows；4,551 variants、13,501 次进程调用 |
| Linux x86-64：平台专属 Rust 测试 | 50 通过，0 失败，0 忽略 |
| Linux x86-64：平台专属 fixture | GNU 13/13、musl 16/16 通过；合计 34 variants、121 次进程调用 |
| Windows x86-64：平台专属 Rust 测试 | 42 项主用例通过；另 2 个子进程入口由父用例实际调用 |
| Windows x86-64：平台专属 fixture | MSVC/UCRT、MinGW/MSVCRT、MinGW/UCRT 各 9/9 通过；合计 30 variants、216 次进程调用 |

fixture runner 公共规则在本机 80 项通过、Windows 专属 4 项通过；VS Code 24 项测试和 lint 通过。Rust fmt/clippy 与 Python Ruff 通过；本机 clippy 保留 57 条既有风格警告，本轮新增警告已清理。远端仅运行平台专属用例，未重复运行平台无关集合。

Linux 复跑修正了既有 C++ 诊断的阶段/文案期望，以及与当前缓存 spec 冲突的工具更新断言；后者同时验证工具更新继续命中、实际源码变化后重编译并重新探测工具。

Windows 验证修复了长缓存路径下编译记录与 native 构建记录的原子发布，分别补充发布、复用和实际输入变化后的更新回归测试；native 工具探针按既有 COFF 合同接受 MinGW 产生的 `.pdata$…`／`.xdata$…` 非空展开节。
