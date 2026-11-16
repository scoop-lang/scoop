# 13 类 ICE 的结构性问题复核

日期：2026-11-16。源码基线：`01fe6066af05e06aaf9998c20d147f662dcb596c`。本轮只分析与补充文档，没有修改实现或规范。

**结论：部分涉及结构性问题，但不能把 13 个候选编号当成 13 个编译器缺陷。** 它们是上一版按失败领域划出的类别，一类包含许多不同的检查。此次追踪各类的关键数据结构、产生点和消费路径，能直接指出四处数据表示问题，并确认一组跨类别的错误传播问题；其他许多分支属于正常的构造、引用、目标格式或 GC 检查。

这里的“确认结构问题”指代码确实存在重复事实、必需关系分散保存或状态混用，**不表示已经找到合法 Scoop 程序触发这些 ICE**。本轮没有运行编译 fixture，也没有逐一证明原始清单中每个分支的可达性。阅读既有单元测试只作为判断分支用途的证据。

判据采用实现规范 [2.11 的输出完备性要求](../specs/SCOOP-IMPL-SPEC.md#L1540)，同时遵守[由产生结果的 stage 负责正确性、不重放验证的要求](../specs/SCOOP-IMPL-SPEC.md#L1584)。构建中的临时 `Option`、正常的跨表引用和构造器返回 `Result`，本身都不是缺陷。

## 逐类判断

| 候选编号 | 判断 | 实际依据与处理方向 |
| --- | --- | --- |
| `ICE0301` 阶段输入上下文 | **主要是必要的组装边界检查** | `CurrentConeParsedSources::try_new` 检查 Cone、顺序、文本和诊断上下文覆盖，成功后字段私有，消费者获得完整视图；`CurrentConeSources` 同样在构造时检查 world/core。不能据此指控阶段输入结构普遍不完备。见[输入构造](../../compiler/ast/src/parsed_sources/current_cone.rs#L118)、[HIR 输入](../../compiler/hir-lower/src/current_input.rs#L31)。 |
| `ICE0401` 实体身份与来源 | **有明确的局部结构问题** | 同一函数的来源分散在多个集合，最后通过六种来源计数必须为 1 来确认互斥性；签名、文件、owner 另查表。来源应由创建函数的路径明确保存，不能到持久身份投影时再辨认。见[来源拼装](../../compiler/hir-lower/src/persistent_functions.rs#L160)。 |
| `ICE0501` HIR 完备性 | **混合，不能整体判定为结构缺陷** | 正式 `Function.signature`、concrete `return_ty` 等已经必填；exact/local identity 输出也有完整结构。部分“缺 owner/signature”与 `ICE0401` 同根，部分是生成适配器失败或依赖模板消费错误，需要查具体来源。尤其不能仅凭文案中的 `preflight admitted` 判断已有成功前置检查。见[函数结构](../../compiler/hir/src/declarations/functions.rs#L6)、[完整 exact identity 表](../../compiler/hir/src/concrete/exact_types.rs#L46)。 |
| `ICE0601` 求解器状态 | **错误传播存在结构问题，原分类也过宽** | session 已进入变量身份，跨 session 检查合理；但真正的内部错误与正常候选不适用共用 `ConstraintFailure` 通道。`NonConcreteApplication` 还表示 `FunPtr<Int>` 这样的非法应用，不能全归 ICE。见[变量身份](../../compiler/hir-lower/src/call_resolution/constraints.rs#L31)、[实际失败条件](../../compiler/hir-lower/src/call_resolution/solver/materialization.rs#L222)。 |
| `ICE0701` MIR 变换与桥接 | **主要是构造约束；局部 API 可收紧** | materialization/context 与 `Option<OdrGroupId>` 独立传参，会产生缺失或多余 group 的组合；构造器拒绝后只产生字段私有的完整记录。这是局部参数关系可改进，不等于 MIR 阶段整体输出残缺。桥接 consumer 不一致同样先核对组装调用方。见[materialization 构造](../../compiler/mir/src/source_callable_materializations.rs#L26)、[正式调用](../../compiler/mir-lower/src/source_callables.rs#L87)。 |
| `ICE1101` GC/root/statepoint | **主要是必要的变换合同检查** | 根计划在 CFG 完成后计算，写入具体指令后才返回函数；LLVM 优化后的根恢复检查面对的是已经变化的数据。类型、地址空间、leaf offset、relocation 错误触发时当然是严重问题，但检查的存在不能证明设计缺陷。见[完成函数](../../compiler/lir-lower/src/safepoints/mod.rs#L106)、[优化后检查](../../compiler/codegen/src/statepoint/finalization.rs#L1)。 |
| `ICE1201` LIR layout/ABI | **有边界组装风险，尚不足以整体判定** | MIR、依赖 callable、layout、target 分别输入，确有组装不一致的可能；这些关系也正是在 `prepare` 中首次接合。成功后已有 `PreparedLir` 承接数据。外部产物不相容、当前变换漏建类型关系、普通表示限制要分别归类，不能都说成“上游已成功仍缺字段”。见[准备入口](../../compiler/lir-lower/src/lowering/prepared.rs#L35)、[外部 callable 接合](../../compiler/lir-lower/src/external_callables.rs#L11)。 |
| `ICE2201` 驱动与调度 | **有明确的局部结构问题** | prepared source node 与它必需的依赖投影仍保存在独立 map，缓存键和 child request 分别取出并报 missing；可把投影随 source node 保存。调度完成集合、图拓扑、锁使用和线程 panic 则需另看，不能统称状态机设计错误。见[prepared 数据结构](../../compiler/scoop/src/snapshot/prepared/model.rs#L118)。 |
| `ICE2301` LLVM 生成 | **部分是明确的结构问题，许多是正常后端检查** | 最明确的根因实际位于 LIR：`Invoke` 普通指令与 `Br(normal)` 同时描述终结控制流，codegen 才检查末尾位置和目标一致性。其余 LLVM builder 错误、动态 operand/type 检查不自动等于架构缺陷。见[LIR 表示](../../compiler/lir/src/instruction.rs#L406)、[codegen 补查](../../compiler/codegen/src/function.rs#L380)。 |
| `ICE2302` native object | **主要是 native 工具边界和对象改写检查** | 输入字节来自 LLVM 或实际 C 编译器，Rust 内部类型不能保证其 ELF/COFF/Mach-O 形状；atom、relocation、unwind 改写仍需边界检查。计划漏项可以是 Scoop 内部错误，外部工具不相容也可能是普通工具错误，需保留来源。见[C bridge 实际调用](../../compiler/codegen/src/c_bridge_emission.rs#L38)、[ELF 读取](../../compiler/codegen/src/elf_object.rs#L34)。 |
| `ICE2401` 元数据/产物投影 | **混合，主要需要收窄分类和保留原因** | 自产 section 缺关联记录属于内部故障；但序列化、hash、publication 的包装错误还可包含分配、长度、外部输入或 I/O 原因。已有共享投影被实际复用，不能从“seal/project failed”推断全面重复语义验证。见[共享投影](../../compiler/hir/src/production/shared_source.rs#L11)、[编码错误种类](../../compiler/wire/src/cbor/encode.rs#L126)。 |
| `ICE2501` 自产协议/诊断 | **诊断来源表示有明确的局部结构问题** | 一个 `Diagnostic` 同时保存裸 `file: usize` 和 `Option<SourceIdentity>`；未绑定与已绑定使用同一种结构，绑定失败仍能留下 `None`，driver 再补齐/检查。这种状态混用值得改。编码分配失败则不是 ICE。见[诊断结构](../../compiler/source/src/diagnostic.rs#L25)、[绑定过程](../../compiler/source/src/diagnostic.rs#L125)。 |
| `ICE2502` 查询内部关系 | **需要细分：索引完备性风险、错误通道和工作线程故障** | rename 已要求成功语义分析与完整引用，再发现来源缺失/歧义值得查索引生产路径；目前却仍报 `Unavailable`，混同正常查询不可用。stderr 线程 panic、analysis worker 停止不是“查询语义关系缺失”，原映射应移出本类。见[rename 前提](../../compiler/analysis/src/semantic/rename/world.rs#L45)、[anchor 错误](../../compiler/analysis/src/semantic/rename/anchors.rs#L62)。 |

## 四处可直接指出的数据表示问题

### 1. 函数来源由多张表在末端恢复

`persistent_functions::resolve` 分别查询 source、accessor、lexical、initialization、test entry、derived 六条路径，再计算 `kinds != 1`。此外，[source identity builder](../../compiler/hir-lower/src/persistent_functions/source.rs#L20) 还从 `signatures`、`function_files` 和 owner 表找必需数据。

这说明“一个函数恰好有一种身份来源”没有在函数产生时成为明确的数据关系。新增一种合成函数或调整某个 lowering 路径时，要记住同步多张表，错误可能直到最终身份投影才暴露。问题在来源建模，不在 hash 算法或 typed ID 的数量。

最小方向：在已有函数创建路径保存明确的来源枚举及该分支必需的数据。持久 ID 仍可等签名解析完成后生成，不要求未解析声明提前拥有最终 ID；构建中的不完整状态也不必消灭。避免新增一层 identity 验证器来弥补漏记。

这主要解释 `ICE0401`，也会影响 `ICE0501`、MIR 的来源投影和 `ICE2401`，不能按四个编号重复登记四个根因。

### 2. LIR 重复保存 Invoke 的终结控制流

[BasicBlock](../../compiler/lir/src/instruction.rs#L7) 同时具有指令列表和 terminator，但 `Invoke` 在指令列表里，并要求额外 terminator 为相同 normal 目标的 `Br`。源码注释明确说这份 branch 用于 dump 可读性。

因此结构允许两种矛盾：Invoke 后仍有指令；Invoke.normal 与 Br.target 不同。codegen 对应报出初稿 [B0180](BACKEND.md#b0180) 和 [B0182](BACKEND.md#b0182)。这两条 ICE 有同一个清楚的结构根因，并非两种独立算法错误。

最小方向：让 Invoke 成为真正的 LIR terminator，复用现有 `InvokeSite` 保存 normal/unwind 与调用数据；dump 根据该结构显示后继。这样删除重复事实，CFG 变换只更新一处。无需另加全图校验阶段。

发现错误的地方是 codegen，根因却在 LIR。这也说明 ICE 编号表达报告领域，不能替代缺陷归因。

### 3. Prepared source node 的必需投影仍是旁表

`PreparedManifestSourceNode`/`PreparedSingleFileNode` 没有自己的依赖投影；`PreparedBuildGraph` 另存 `source_inputs`。之后 [child request](../../compiler/scoop/src/snapshot/prepared/model/child_request.rs#L265) 和[缓存键构造](../../compiler/scoop/src/snapshot/prepared/model/cache_key.rs#L35) 分别查询该表，并报 `MissingSourceProjection`。

`PreparedGraphNode` 已经区分源码和预编译节点，方向是对的；问题只在源码节点的必需关系未随该分支保存。最小方向是把当前投影移入两个 source 分支，消费者通过已取出的节点直接取得。不要把普通依赖调度、锁协调或外部进程状态也扩展成通用状态证明框架。

现有[单元测试](../../compiler/scoop/src/snapshot/prepared/tests/core.rs#L128) 主动删除投影后检查错误，只证明这个防线存在，不能当成正常构建会丢失投影的复现。

### 4. 诊断的本地索引状态和稳定来源状态混用

`Diagnostic` 与 note 的 `file`、`source` 可以独立设置；[resolve_sources](../../compiler/source/src/diagnostic.rs#L127) 查找失败时仍保持 `None`，不把成功绑定形成不同的输出关系。driver 的 [CurrentConeDiagnosticSet](../../compiler/driver/src/request/report.rs#L47) 再根据 identity 修正 index、检查越界；[协议转换](../../compiler/driver/src/request/report/protocol.rs#L30) 最后仍能发现来源未解析。

这会让一条已经产生的有效源码错误，在跨阶段或格式化时因来源数据不足而再次失败。已有诊断集合封装能挡住部分问题，但未消除同一种诊断结构兼任不同状态的问题。

最小方向：parser 内部可以使用文件内位置；离开生产上下文的诊断应携带明确的稳定来源。无源码位置的 driver 错误使用独立的位置分支，不能用随意的文件索引表达。尽量复用现有 SourceIdentity/位置类型，不引入新的来源授权或证明体系。

## 编号落地前需要处理的错误传播问题

### 求解器把内部失败放进普通候选拒绝通道

[ConstraintFailureKind](../../compiler/hir-lower/src/call_resolution/constraints.rs#L202) 把 `ForeignVariable`/`ForeignTypeParameter` 与类型不匹配、无法推断等放在同一枚举。[probe](../../compiler/hir-lower/src/overload/probe.rs#L179) 把它们全部转为普通候选失败；[重载选择](../../compiler/hir-lower/src/overload.rs#L324) 有适用候选时会选择赢家，没有对内部失败单独终止的分支。

所以如果某个候选发生内部失败，该信息可能被另一候选的成功掩盖，或作为“无适用重载”的普通原因输出。这是能够从控制流确认的传播问题；这里没有声称已经找到合法源码触发 `ForeignVariable`。

求解器已经把 session 和 environment 编入变量身份，保留这些检查即可。需要修改的是失败结果的分流：可预期的不适用继续参加候选聚合；内部不变量失败退出普通候选选择。仅给渲染出来的字符串贴 ICE 编号解决不了这个问题。

### 后端与投影包装过早抹平失败原因

[CodegenError](../../compiler/codegen/src/lib.rs#L57) 只是 `String`，同一类型承载 builder 错误、内部关系缺失，以及 C bridge 临时目录/文件 I/O 失败。`HashError` 也在[转换处](../../compiler/wire/src/digest.rs#L154) 丢弃实际编码错误。这样到 driver 编号时很难可靠区分 E 与 ICE。

最小方向是在实际产生/包装错误的现有类型中保留失败种类和必要 cause。无需建一个通用错误框架，更不能按英文文案关键词或 crate 路径猜编号。

### 查询内部失败与正常不可用共用 Unavailable

rename 的 `World::load` 已检查语法、声明/正文引用、语义诊断和成员关系，之后 anchor 缺失/歧义却仍返回 `RenameError::Unavailable`。应区分正常编辑态不可用和完整结果违反其合同；若合法的 synthetic/依赖目标本来就不可重命名，应明确表达该目标种类，而非把它伪装成丢失来源。

这个区分要结合目标实际来源完成，当前证据不足以断言每一次 anchor 失败都是索引构建 bug。

## 对上一版 ICE 映射的具体修正意见

以下意见优先于上一版按领域归并的宽泛判断；本轮没有批量重排规则号或重写全部原始文案映射。

| 原始条目 | 复核结果 |
| --- | --- |
| [F0104](FRONTEND.md#f0104)、[F0105](FRONTEND.md#f0105) | 不应固定归 ICE0601。未求解的 term、非法 native function-pointer 应用、应用导入错误共用这些变体；要保留具体原因再路由。既有[测试](../../compiler/hir-lower/src/call_resolution/tests.rs#L470) 明确用 `FunPtr<Int>` 期待 `NonConcreteApplication`。该测试构造的是 solver 输入，不是本轮新增的 CLI 复现。 |
| [F0717](FRONTEND.md#f0717)、[F0740](FRONTEND.md#f0740)、[F0741](FRONTEND.md#f0741) | `preflight admitted` 不是充分前提。当前 [prepare](../../compiler/hir-lower/src/expr/named_calls/imported_dependency/defaults/prepare.rs#L27) 调用 [load_default_expression](../../compiler/hir-lower/src/expr/named_calls/imported_dependency/defaults/materialize/definition.rs#L6)，其中才调用可能报错的 materializer。外部不合格模板与自产模板消费缺口必须分开。也不能只看最终 catch-all：前面的分支已经处理多种 StructInit/ClassInit/DirectSuperMethodCall。 |
| [D0015](BUILD.md#d0015) 等 encode 包装、[A2659](ARTIFACTS.md#a2659)、[A2660](ARTIFACTS.md#a2660) | 不能因“自产消息/自产 hash”就全归 ICE。`ProtocolWriteError` 包含 Wire/Frame，`EncodeError` 包含 Allocation/LengthLimit；runtime hash 又抹平原始编码原因。资源、长度、I/O 错误按真实原因报告。 |
| [T0051](TOOLS.md#t0051) | 来自 [JoinHandle::join](../../compiler/lsp/src/cli/client.rs#L150) 的 panic 分支。它是工作线程内部故障，不能作为“查询语义关系缺失”的证据；按现有分区更接近 ICE2201。正常 stderr EOF 并不会触发这条错误。 |
| [T0090](TOOLS.md#t0090) | 来自向 analysis worker [发送工作失败](../../compiler/lsp/src/server.rs#L199)。只证明接收端已退出，应追踪 worker 生命周期/上游 panic；不证明 HIR 查询关系损坏。正常 cancellation 与该失败不是同一件事。 |
| object/ABI/metadata 的共享错误 | 当前自产计划矛盾可用 ICE；首次消费外部输入不合格用 E；外部工具失败保留工具原因。错误在哪个 crate 抛出不能决定归属。 |

## 哪些不应因此重构

- **不能把 builder 中的临时空值当作成功 IR 缺失。** 例如 exact identity builder 用 `Vec<Option<_>>` 支持递归构造，完成后的 `ExactTypeIdentities` 保存 `Vec<ExactTypeRecord>`。这是正常的构造过程与完整输出之分。
- **不能把合法的不适用分支当作缺元数据。** library 没有 executable entry、普通 callable 没有 ODR group、无 managed leaf 的值没有 root，均应按实际分支判断。
- **不能消除必要的 GC/LLVM/object 检查。** GC 的临时 `plans` 表在同一 pass 内计算并写入指令，不是跨阶段留下的可选根计划；LLVM 优化会改变值和调用，native 工具会产生新字节，需要针对新数据的合同检查。
- **不能因存在 shared reader 错误就推断 IR/meta 重复实现语义。** 要证明对同一未变数据重复做了同一检查，才有删减依据；已有序列化、布局计算、外部引用接合本身是正常职责。

## 建议处理顺序

1. **编号接入前先厘清失败通道。** 收窄 ICE0601/ICE2401/ICE2501/ICE2502 的映射，保留根因，避免内部失败参与正常候选淘汰；否则编号可能把实现故障归咎给用户，也可能反过来把普通输入错误称为 ICE。
2. **优先收敛 LIR Invoke 与诊断来源两处结构。** 问题局部、现有消费路径明确，分别消除重复控制流和跨阶段来源状态混用。
3. **再收敛函数来源与 prepared source 投影。** 在真实生产路径保存必需关系，减少末端恢复与重复查找；不按 13 类逐一建设新的验证层。

上述顺序是设计改进建议，不是已复现故障的严重级别排名。后续如进入实现，应按实际改动先更新相关规范，并用真实源码/产物/诊断场景验证；本轮保持实现不变。
