# 紧凑 GC 元数据设计

状态：实施中。调研基线为 2026 年 11 月的 runtime contract 33、metadata ABI 10；编译器和 runtime 已接通 contract 34／metadata ABI 11，Darwin AArch64 的 GC 与混合 profile 产物专项通过，UI 体积及其他目标正在验收。

本项将 LLVM 的通用 stackmap 转换为 Scoop 的紧凑机器根表，并通过已有 callable 登记引用该表，取消独立的逐 safepoint 运行时登记。完整类型化身份继续保存在编译产物中，程序链接负责所选定义的身份与 ODR 关系；runtime 按实际 PC 和根位置扫描。第一版采用 4 字节定长根位置、函数内布局共享和轻量 PC 索引。

本项服务可执行文件体积、启动成本和 GC 元数据常驻内存。保持函数正文、实际 safepoint、根集合、moving GC、异常与 native 边界不变。UI 类型描述的生成与使用另行处理；不包含 native dead strip、LTO、Skia 裁剪或通用压缩。

## 1 调研依据和收益口径

样本使用本机现有 release CLI、预编译 sysroot、LLVM 22.1 和 `aarch64-apple-darwin`，调试信息为 none。Counter 来自 `sysroot/src/scoop.ui/examples/custom`，控件示例来自 `sysroot/src/scoop.ui.component/examples/themes`。下表单位为 MiB，数量项除外。

| 项目 | Counter | 控件示例 |
| --- | ---: | ---: |
| 原可执行文件 | 82.87 | 118.05 |
| 原 LLVM stackmap | 27.60 | 41.91 |
| 原 168 字节 safepoint 登记 | 9.07 | 14.60 |
| 实际 safepoint 数 | 56,602 | 91,156 |
| 实际 root pair 数 | 1,044,079 | 1,571,344 |
| 按第 3 节格式编码的机器根表 | 4.63 | 6.83 |
| 按第 3 节数据替换推算的完整文件 | 50.45 | 67.75 |
| 推算节约 | 32.42 | 50.30 |

机器根表已经离线编码，逐项还原了每个 site、指令偏移、帧大小和有序根位置。文件大小推算计入取消 168 字节登记及其 8 字节表指针、为每个 callable 增加一个 8 字节指针；未计入 image 固定头缩减、最终重定位、段对齐、签名和新 runtime 代码的变化。这些数值是设计阶段的数据规模依据；真实链接与运行的结果单独记录于第 11 节，不将推算冒充 executable 实测。

此前的通用紧凑编码实验还完整还原了原始记录头与 live-out，并由现有 C stackmap parser 和独立 C decoder 交叉核对。另一个实验在保留全部 site／owner／ODR 身份的情况下，将控件示例的登记表从 14.60 MiB 编为 6.48 MiB；它说明重复字段的成本，本设计直接按实际消费者划分运行时字段，不发布这一中间格式。

控件示例的原 runtime parser 为根集合分配 78,774 个数组。其记录和根数组有效载荷合计 85.84 MiB，分配器实际提供的块容量为 92.17 MiB；这不包含文件映射及其余登记索引，也不等于整个进程 RSS。第 5 节的 PC 索引为每个 site 24 字节，当前样本约 2.09 MiB，根位置直接引用映像中的紧凑表。

类型描述优化会改变实际保留的函数和 site 数，不能把两个项目的体积收益相加。正式验收应在同一份届时源码、系统库与工具链上构建配对基线。

## 2 字段用途和身份边界

现行 [runtime 规范 2.8](../specs/SCOOP-RUNTIME-SPEC.md#28-多-image-登记与启动-abi)要求每对根是相同的 8 字节 SP/FP Indirect 栈槽；[产物归一化](../../compiler/slib/src/link_object/stackmap_normalization.rs)也检查 base 与 derived 相等。LLVM 的两条 12 字节位置记录可用一个栈槽位置表示。

独立 safepoint 登记目前主要用于启动时将 raw record 与 site／owner 对应。[实际扫栈](../../runtime/src/gc_binding/stack_roots.c)通过精确 PC 找到帧大小和根位置，不查询完整 site 或 ODR 身份。新表示保留各字段的实际用途：

| 信息 | 编译及产物 | 程序链接 | 最终 runtime |
| --- | --- | --- | --- |
| `PersistentCallableBodyId` | 保留原类型化身份 | 选择完整物理实现 | 沿用 callable 登记 |
| `PersistentSafepointSiteId` 与完整 key | 保留 owner、role、ordinal | 检查所选 site 的身份与映射 | 不逐 site 保存完整 key／ID |
| `SafepointId` | 保留原非零 64 位映射 | 检查不同完整 site 的碰撞 | 保留在 PC 记录中用于诊断 |
| site role 与逻辑 live set | 按现有 LIR／RS4GC 合同处理 | 复用产物结果 | 不复制角色表或逻辑根计划 |
| callable 与 site 的 ODR 关系 | 保留实际定义归属 | 随所选 callable 闭合 | site 不另登记 ODR member |
| 机器 PC 偏移、帧大小、根位置 | 从实际机器发射取得 | 保持正文与附属数据同选 | 扫栈直接消费 |
| LLVM header、deopt／live-out 编码 | 在机器对象边界解析和检查 | 无独立运行时用途 | 不保存通用 LLVM 记录 |

全局实体继续使用现有 typed ID，不改变 ID 派生、源码身份或名称解析。GC 表是 callable 的对象附属数据，使用现有 `ObjectDefinitionAtomId` 表达，不增加新的语言实体或身份体系。表内偏移只是局部格式索引。

## 3 最终静态数据格式

### 3.1 Callable 登记

在 `ScoopCallableRegistrationDescriptorV1` 末尾增加：

```c
const ScoopCallableGcMapV1 *gc_map;
```

原字段保持其意义，`gc_map` 位于 offset 144，记录从 144 增至 152 字节，alignment 为 8。

该指针由最终物理 site 集决定：有 site 时指向本 callable 的非空 GC 表，无 site 时为 null。Managed effect 本身不决定是否有表；省去入口 poll 的有界叶函数也可能没有 site。含零根 site 的函数仍有表，不能把零根等同于没有 safepoint。

这一状态在编译器中使用完整的空／非空分支表达；非空分支必须同时具有表 atom 和实际 site 数据。最终产物不能用缺失指针表示尚未生成的表。

### 3.2 函数 GC 表

所有当前目标使用 64 位指针和 little-endian 字段。固定头为 32 字节，alignment 为 8：

```c
typedef struct ScoopCallableGcMapV1 {
    ScoopDescriptorPrefixV1 prefix;
    uint64_t stack_size;
    uint32_t site_count;
    uint32_t layout_word_count;
} ScoopCallableGcMapV1;

typedef struct ScoopGcSiteV1 {
    uint64_t safepoint_id;
    uint32_t instruction_offset;
    uint32_t layout_word_offset;
} ScoopGcSiteV1;
```

| 头字段 | offset | 说明 |
| --- | ---: | --- |
| `prefix.magic` | 0 | `0x53434f4f5047434d` |
| `prefix.abi_version` | 8 | 本次统一 metadata ABI |
| `prefix.struct_size` | 12 | 固定头大小 32 |
| `stack_size` | 16 | LLVM 发射的实际帧大小 |
| `site_count` | 24 | 非零，随后 16 字节 site 记录的数量 |
| `layout_word_count` | 28 | 布局区的 32 位 word 总数 |

固定头之后依次为 `ScoopGcSiteV1[site_count]` 和 `uint32_t[layout_word_count]`，末尾补零至 8 字节边界。完整大小为：

```text
round_up_8(32 + 16 * site_count + 4 * layout_word_count)
```

函数入口地址从引用该表的 callable 登记取得，表中不重复保存地址、owner ID 或指向 owner 的反向指针。

site 按 `instruction_offset` 严格递增保存；排序不改变 `SafepointId` 或完整 site identity。实际查找地址为 `callable.entry + instruction_offset`，使用原始精确 return PC，不减指令宽度或使用邻近 PC。每个保留 site 恰有一条记录，`safepoint_id` 非零。

### 3.3 根布局

布局区是若干有序根序列的连续编码。每个布局先存一个 `u32 root_count`，随后存 `root_count` 个根位置 word。`layout_word_offset` 从布局区起点以 word 计数，必须指向某个布局的 count word。运行时的 root index 和相应诊断字段使用同宽的 `u32`，检查通过后才能索引；实际生产的根数仍受 LLVM 机器记录本身的可表示范围约束。

同一函数内完全相同的有序根序列共用布局，按排序后 site 的首次使用顺序放置。保留根的原始顺序和数量，不合并序列内重复的槽。零根布局为单个值为 0 的 word。第一版不跨函数共享布局。

每个位置 word 保存原有 signed i32 字节偏移和 SP/FP 选择。当前机器根槽为 8 字节对齐，偏移的低三位为零，因此编码为：

```text
encoded = bit_pattern_u32(byte_offset) | base
base: SP = 0, FP = 1
bits 1 and 2: zero

base = encoded & 1
byte_offset = interpret_i32(encoded & 0xfffffff8)
```

例如 `SP + 32` 编为 `0x00000020`，`FP - 16` 编为 `0xfffffff1`。使用明确的二进制位重解释，不依赖 C 对负数右移的实现行为。SP/FP tag 由当前 frame profile 映射到实际寄存器；它不是固定的 DWARF register number。

该编码覆盖原 i32 中全部 8 字节对齐偏移。生产边界明确检查机器根的对齐和可表示范围；异常机器输出是后端合同错误，不截断、不降低根精度。数量和大小只受现有机器格式与所选字段的可表示范围约束，乘加使用 checked arithmetic，不引入程序资源配额。

### 3.4 Image 和物理存储

删除 `ScoopSafepointRegistrationDescriptorV1` 及其独立符号、运行时登记种类和 image 中的 `safepoints/safepoint_count`。Image 从 208 缩为 192 字节，保留 storage、immortal、initialization、type、callable 五组表；callable 表位于原 safepoint 表的位置。

GC 表放在已有的不可变 metadata 数据区，与所属 callable 的代码、EH、登记、私有常量和 Context cells 保持同一个物理选择组。表内只含整数，唯一新增指针是 callable 登记到 GC 表的普通对象重定位。函数地址继续由 callable 的既有 entry 重定位绑定。

最终对象及 executable／native-app 不再携带原始 LLVM stackmap 段。Mach-O 的专用段重命名、ELF 的 stackmap 起止符号、PE 的 `.smap` 保留规则和相应 runtime section 读取路径随之移除。普通 metadata 的只读段与 RELRO 规则继续适用。

## 4 编译产物和程序链接

### 4.1 对象生产

根计划、优化、RS4GC 和机器代码生成保持原有顺序。转换消费机器发射后的真实栈位置，不从优化前 IR 推测偏移：

1. 发射临时机器对象，取得实际 LLVM stackmap、函数符号及其重定位。代码生成停止发射逐 site C 登记，最终物理根数保存在已有生产结果中。
2. 在现有机器对象边界检查实际 site 与最终 LIR／RS4GC 结果的对应关系，包含完整 owner、role、物理根数及目标 frame 合同。复用现有解析与归一化结果。
3. 按第 3 节生成本 callable 的完整表，物化表 atom 和 callable 引用，移除原始 stackmap。重新计算实际改变的 section、symbol、relocation 和 atom 范围。
4. 对新写入的数据格式与引用做必要检查，完成现有对象内容和产物指纹，再发布 `.slib`。代码和未变化的 LIR 不重新执行语义或根分析。

临时 LLVM 对象不是可发布产物。复用现有对象生产输入／输出类型区分原始机器输入和完成 GC 表物化的最终对象；写入最终产物的分支必须持有完整表或明确的零 site 状态，不通过后续 reader 补齐。

具体实现复用 [对象生产编排](../../compiler/driver/src/object_production/layout/verification.rs)、现有 stackmap reader 和目标对象写入辅助代码。格式读写与数据结构放在现有 IR／meta 边界，driver 负责编排；codegen 不依赖上游 stage 或 `scoop-slib` 的实现来反向查询语义。不另建一套语言／GC 语义 verifier。

### 4.2 产物身份和 Link 投影

LIR foundation 现有 `safepoint_sites` 与 resolved safepoint mappings 继续保存完整身份，并沿现有 codegen 清理路径只保留实际发射的 site。删除逐 site 的物理 Registration definition／symbol 不等于删除语义 site。

从这些已保存数据向 `ProgramLinkProduction` 投影按 `PersistentCallableBodyId` 分组的 `(PersistentSafepointSiteId, SafepointId)`。只扩展实际 Link 消费视图，不复制另一张持久身份表，也不读取源码、展开 HIR 或恢复机器调用图。

ODR 选择和可达性处理完成后，Link 只检查所选 callable 的 site：不同完整 site 不能共用非零 64 位 ID，同一 site 不能有不同 owner；重复候选先随 body 选择归并。未选实现的 site 不进入最终程序检查，也不要求 debug／release 候选具有相同 site 集。

runtime 不再为这项已经完成的身份关系建立 `site_ids` 索引或重算 ID。64 位 ID 保留诊断作用，实际 GC 查找始终以加载后的 PC 为键。

### 4.3 ODR 和更新判断

代码、GC 表及 callable 登记始终来自同一个所选物理 member。不同优化实现可以有不同机器 site／根布局；不比较它们的正文、完整栈映射或压缩结果来决定 ODR 相容性。独立 helper 继续按原规则选择。

GC 表转换在生产端完成一次，存入现有对象和缓存。最终链接直接消费完成的对象，不新增逐次对象转换或原始 stackmap 扫描。Link 输入复用已保存的定义和引用关系。

布局与 ABI 变化进入现有 backend／runtime 配置及 Code、Artifact、Link 更新键。RuntimeImage 描述实际五张登记表和 callable 引用；GC 表字节由其所属对象的现有内容指纹覆盖，不新增运行时指纹、独立收据或跨阶段证明。缓存仍先判断更新，未变化且结果存在时直接复用。

## 5 运行时索引和扫描

### 5.1 启动

沿用现有 image、callable、类型与存储登记顺序。在 managed 初始化之前，对已经选定并归并的 callable 处理 `gc_map`：

- null 表示该 body 没有物理 site；非空表检查 prefix、完整地址范围及计数运算。
- 每个不同根布局只解析一次，确认 count、位置编码和目标 frame 的静态约束；site 引用必须落在真实布局起点。处理本函数后释放仅用于读取的布局边界索引。
- 根据实际 entry 加指令偏移计算 return PC，检查加法溢出和已加载代码范围，再建立 PC 索引。最后按 PC 排序并拒绝重复地址。
- 不再解析 LLVM v3、分配 base/derived pair、按完整 site ID 排序，或将 raw record 与另一份登记表 join。

平台 loader 继续从当前承载 runtime 的已加载模块取得 metadata 范围与代码区间。代码区间从原有解析结果独立返回，不再依赖存在 stackmap section；不重新打开 executable、查询另一套 VM 权限或引入模块身份认证。native-app 仍只登记自身模块，host 范围和 Cone 装卸语义保持。

### 5.2 常驻结构

runtime 私有 PC 索引只保存查找与解码所需的三个 machine words：

```c
typedef struct ScoopGcPcEntry {
    uintptr_t return_pc;
    const ScoopCallableGcMapV1 *map;
    const ScoopGcSiteV1 *site;
} ScoopGcPcEntry;
```

当前目标上每条为 24 字节。一次分配索引，填充后排序；根列表和帧大小直接引用不可变映像。取消原 160 字节 record 数组、逐 site root arrays、site 登记地址索引、site ID 索引和 raw／joined 两份记录数组。其余登记类型的索引按原职责保留。

`scoop_stackmap_lookup` 的职责保持为精确 PC 二分查找。frame ops 消费紧凑条目的视图，`resolve_root(frame, index)` 可通过 count 与第 `index` 个 4 字节位置直接索引，保持 O(1) 访问，无须引入顺序解压接口。

这些表在 runtime 生命周期内不可变，执行 GC 时不重新归并、分配或重建根索引。SafepointId 从 site 记录取得，仍能用于 frame／root 错误诊断。

### 5.3 动态根合同

每次扫描仍使用线程实际发布的 return PC、SP、FP 和栈边界。位置解码之后，目标 frame 实现检查地址运算、真实帧范围和槽位对齐，并向现有 visitor 交付可改写的真实槽位。

普通 call／poll 的位置、`gc.relocate` 使用和根生命周期不变。invoke、NativeSafe／NativeBorrowed transition 的显式 root frame 及 normal／unwind 重读规则保持；其零 gc-live 机器 site 仍进入 PC 表。callback、协程、冻结 managed segment 和多线程 STW 的扫描边界不变。

全局根、native/compiler root frames、handle、pin、异常对象和堆 scan program 不属于本次元数据压缩范围。

## 6 目标帧合同

| Frame profile | SP／FP DWARF number | 根位置的解释 |
| --- | --- | --- |
| Darwin AArch64 fixed frame | 31／29 | 保留实际 SP／FP；stack size 为 16 的倍数，沿用原帧记录和槽范围 |
| SysV AMD64 fixed frame | 7／6 | 保留 `N % 16 == 8`、`FP = SP + N - 8` 与原 return-PC 恢复 |
| Windows AMD64 virtual unwind | 7／6 | 保留实际 RSP／RBP，向外扫描使用现有 Windows unwind |

格式中的 base tag 选择寄存器，不能用固定帧几何把 Windows 的 FP 偏移统一改成 SP 偏移。frame pointer、red zone、tail call 限制及 ABI shim 保持。

Mach-O、ELF 和 COFF 的工作是存放普通 metadata、绑定 callable 引用并保留原 EH／unwind；根编码由实际 frame profile 解释，不从对象格式或 OS 名称推导。

## 7 必要检查和失败处理

| 边界 | 承担的检查 |
| --- | --- |
| 最终 LIR／RS4GC | 实际 site 的 owner、role、逻辑根与 relocation 使用，沿用现有检查 |
| 原始机器对象 | LLVM 格式、机器 site 完整性、物理根数、相同 base/derived、受支持栈位置 |
| 紧凑对象写入 | 新表的范围、布局起点、位置编码及真实 section／symbol／relocation 关系 |
| 外部 `.slib` 读取 | 普通格式、typed 引用及 ABI 相容性，不重跑语言语义或代码生成 |
| 程序链接 | 所选 site 的完整身份与 64 位映射、body／表同选、实际符号引用闭合 |
| 加载与启动 | 记录格式、加载地址、表范围、目标 frame、精确 PC 唯一性 |
| 实际 GC | 当前线程／帧地址范围、根槽定位与移动后的真实回写 |

同一不可变输入已经完成的检查直接复用；共享布局不按 site 重复完整验证。最终表的完整性由生产结果和 body 同选保证，runtime 不保存第二份预期 site 清单再次证明所有 site 存在。

错误使用现有编译／链接诊断或 runtime metadata／frame fatal 路径。错误 ABI、坏偏移、截断表、重复 PC、非法位置与实际越界都明确失败；不回退到保守扫描，也不截断根数。测试可以定向构造坏输入，不将这种破坏恢复加入缓存命中路径。

## 8 规范和产物迁移

本项不改变 Scoop 源码行为。实施前同步修订：

| 文档 | 修改范围 |
| --- | --- |
| [Runtime 规范](../specs/SCOOP-RUNTIME-SPEC.md) 2.8 | metadata 布局、五张表、callable GC 引用、身份与 PC 检查职责 |
| Runtime 规范 3.2、3.3 | 紧凑机器根表、位置编码、frame 访问与不可变索引 |
| [实现规范](../specs/SCOOP-IMPL-SPEC.md) 2.5、2.6 | 原始机器对象到最终 GC 表的生产边界、对象 atom 和产物字段 |
| 实现规范 2.8、2.14、2.19 | Link site 投影、最终 image、正文与 GC 表同选、更新判断 |

当前实现采用 runtime contract **33 → 34**、metadata ABI **10 → 11**，同步更新公共 C 头、LLVM 布局、启动代码和目标合同。LIR `identity-foundation`、`cone-production`、`strong-production` 分别升至 **15、18、28**；容器、manifest 和 metadata 外层格式保持原版本。

compiler、runtime、系统库及含机器正文的 `.slib` 需要配套重建。按未发布阶段约定直接演进，最终运行时只有上述一套格式；LLVM v3 reader 仅保留在真实生产边界与其必要测试中。清理退役字段、symbol／definition role、原始段加载及只服务旧 join 的检查和测试，不保留空兼容槽或双轨启动。

普通程序、native-app、debug／release 和 GC stress 构建统一使用新格式，无额外压缩开关。root entry、native-app entry、对象布局、用户调用 ABI 与 GC 算法保持。

## 9 实施与验收

### 9.1 实施顺序

1. 修订上述规范，落实 C 布局、typed GC atom 与最终 Link site 视图，明确版本迁移。
2. 接通真实 LLVM 对象读取、紧凑表生产、callable 引用及五表 image 输出；完成 Mach-O／ELF／COFF 的普通对象处理和所选 site 碰撞检查。
3. 将 runtime 启动和 frame ops 切换到紧凑表，删除旧登记／raw join 及专用段通道，同步重建 sysroot，形成源码和 artifact-only 两个入口的运行闭环。
4. 在最终实现上完成下述正确性、空间、运行和构建成本对照，再更新路线图与启用优化清单的交付状态。

### 9.2 正确性

测试围绕真实机器根和产物合同组织，不为每个普通库复制同一组覆盖：

- 格式测试覆盖 SP／FP、正负偏移及边界、零根、共享布局、布局内重复槽、非布局起点、截断／溢出、坏 tag、重复 PC 与 ABI 不匹配。
- 代表性真实对象测试对照临时 LLVM 记录和紧凑表的精确 PC、帧大小、有序根位置与物理数量，确认最终对象无原始 stackmap 和退役登记；这项交叉核对放在测试，不在消费者中重放。
- 产物专测覆盖 Strong／ODR、debug／release 混合选择、优化删除 site、同一泛型不同实现、不可达 callable、完整身份／短 ID 冲突，以及共享表与函数同选。原始 live-out 等非 GC 字段的读取仍由机器对象测试覆盖。
- 定向 moving GC 验证普通 call／poll、接口和聚合引用、EH normal／unwind、native callback、协程及多线程 managed segment。检查真实搬迁和回写结果，普通运行与 GC stress 分开。
- 正式 CLI 覆盖源码构建、消费预编译库、artifact-only 链接、eager／root 启动及 executable／native-app；涉及 ABI 和产物的专测覆盖当前支持的 frame／容器组合。

当前离线体积数据只有 Darwin AArch64。跨目标验收按完整 target 执行，包含 Darwin 两种架构、Linux GNU／musl AMD64，以及现有 Windows AMD64 native environments；某环境未验证时明确保留限制，不把另一平台结果作为通过依据。

### 9.3 空间和性能

固定同一组源码与编译优化，记录实际 callable／site／root 数、紧凑表和登记字节、完整文件、`.slib` 大小、启动索引分配以及进程 RSS。机器码与 EH 的差异按真实重定位区分，不能用少生成函数或漏根取得本项收益。

默认启动和显式 `--enable-startup-stats` 分开测量。沿用既有测量设施；site 数与 stackmap 数从同一紧凑表取得，统计关闭时继续排除计时代码。记录默认启动中位数、GC 扫栈／暂停和正常程序吞吐，避免用文件缩小代替运行成本验收。

构建成本分别测库冷生产、消费预编译库、最终链接和正常缓存命中。命中时源码、工具及原产物保持存在，以实际调用和输出核对复用；不通过隐藏或破坏输入验收。新格式转换的成本应发生在必要的生产过程中，不能转移到每次链接或运行。

采用的依据是完整正确性与实际空间收益，并如实记录运行和构建回退。第 1 节的约 50 MiB 节约是当前大 UI 样本的测量方向，不作为任意程序必须达到的比例门槛。

## 10 后续编码选择

第一版定长位置支持直接读取和现有按根索引访问。差分／变长编码的既有离线实验将控件示例的栈映射编为 3.19 MiB，但需要顺序访问或一次性解码共享布局；在定长版本完成真实 GC 扫描测量后，再判断额外数 MiB 是否值得相应成本。

跨函数字典、整段压缩、位图和减少 safepoint／根生命周期不进入本次交付。后者改变的是代码生成和活跃性问题，应有独立的具体需求与设计。

## 11 调研记录

2026 年 11 月 5 日的原始解析、编码脚本、C 交叉核对和布局推算保存在仓库外的本机临时目录 `/var/folders/6k/02kz7z0s7nxc1m0gg3xj99c40000gn/T/scoop-gc-metadata-zz59jyea/`，主要结果为 `summary.json`、`c-parser-results.json` 和 `design-layout-results.json`。临时文件不承担设计有效性的前提；可复核的字段、布局、样本位置和计算口径已在本文保留。

本文件只保存设计与必要结果概要，完整日志、逐次 JSON、可执行文件和压缩数据不进入仓库。正式实现后以相同口径追加简短的验收结果，替换未实现状态。

### 11.1 首轮实现验证

Darwin AArch64 使用 LLVM 22.1.8、release Scoop 产物与无调试信息。Hello 可执行文件从 **1,464,072** 字节降至 **918,008** 字节，减少 **37.3%**；GC 表从 **229,776** 字节降至 **62,288** 字节。独立解析新旧 executable 后，471 个含表函数、1,965 个 site 和 3,549 对根完全对应，逐 site 的精确 PC 偏移、帧大小、有序根位置全部一致。最终文件不含原始 LLVM stackmap，程序正常输出 `Hello`。

本地验证包括全工作区格式化及 clippy、identity 361 项、LIR 379 项、slib 267 项、codegen 396 项，以及 runtime 元数据／映像／扫栈 C 专项。正式 GC fixtures 为 **71 通过、2 项仅适用 Linux**，共 150 个变体；ODR selection 与 coroutine mixed profiles 的 4 个变体通过，覆盖源码、预编译产物、artifact-only 链接和缓存复用。实际泛型机器对象测试也通过。

codegen 中 Windows WinEH 精确 PC 项用 Scoop 配套的 LLVM 22.1.8 复验通过；本机 Homebrew LLVM 缺少实现规范要求的 WinEH statepoint 标签修正。该差异未通过修改预期 PC 或放宽 runtime 查找规则规避。后续目标及大 UI 样本的结果如下。

### 11.2 实际空间与运行成本

2026-11-05 的配对样本保持原 Scoop 源码、release 优化、目标、native 资源与无调试信息设置。独立解析原 executable 的 LLVM v3 表和新 executable 的紧凑表，按 `SafepointId` 比较精确 PC 偏移、帧大小和有序根位置；三个样本均无缺失、增加或不同记录。保留的类型、callable、初始化与其他登记计数也一致。以下是完整文件实测，单位 MiB：

| 样本 | 原文件 | 新文件 | 减少 | 缩小比例 | 原／新 GC 表 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hello | 1.40 | 0.88 | 0.52 | 37.3% | 0.219／0.059 |
| Counter | 82.87 | 49.81 | 33.06 | 39.9% | 27.60／4.63 |
| 控件示例 | 118.05 | 66.70 | 51.35 | 43.5% | 41.91／6.83 |

Counter 保持 **56,602** 个 site、**1,044,079** 对根，控件示例保持 **91,156** 个 site、**1,571,344** 对根。含非空 GC 表的函数分别为 6,447 和 9,972；包含无 site 函数的 callable 总数分别为 6,580 和 10,258。新表没有原始 LLVM stackmap，也没有独立逐 site 登记。

默认构建分别启动 5 次，在启动后 3 秒读取进程 RSS，随后由测量脚本终止其自建进程。显式 startup-stats 构建另测 5 次；旧版本从原对象重新链接配套的统计 runtime，新版本使用正常 `--enable-startup-stats`。下表均为中位数，startup 只包含 runtime 初始化阶段，不代表完整窗口呈现时间。

| 指标 | Counter 原／新 | 控件示例原／新 |
| --- | ---: | ---: |
| 默认进程 RSS，MiB | 204.78／97.47 | 279.69／115.00 |
| runtime startup，ms | 76.12／15.06 | 113.19／23.45 |
| 其中 GC 表初始化，ms | 40.90／4.18 | 58.70／6.08 |
| 新版常驻 PC 索引，MiB | 1.30 | 2.09 |

RSS 包含映像、UI 与其他进程分配，不等同于单独 GC 索引；PC 索引按实际 site 数乘 24 字节计算。根位置直接引用文件映射，原来的逐 record 解码根数组不再分配。两组 UI 的默认启动无 stderr，统计构建的新旧登记计数逐项一致；这些启动观测不代替完整 UI 交互测试。

配套 `.slib` 的 `scoop.core` 从 13.58 降至 11.74 MiB，`scoop.ui` 从 92.45 降至 66.22 MiB，`scoop.ui.component` 从 90.51 降至 63.56 MiB。产物中仍保留完整 typed site 与 owner，缩减来自物理表及其退役生产记录。

另以 `tests/benchmarks/old-graph-large.scoop` 交替运行 9 对普通构建和 9 对显式 GC 统计构建。该样本的 1,994 个 site、3,576 对根也逐项一致，输出均为 `199680`；两版均分配 9,021,576 字节，完成 4 次 minor、2 次 full，访问 414 个根槽。普通进程耗时中位数为 13.06／12.39 ms；GC 总暂停为 1.243／1.233 ms，根扫描为 6／5 µs。在这一小样本上未观察到明显 GC 或吞吐回退，不将微秒差异解释为稳定的扫栈加速。

### 11.3 构建成本

两版均使用 release Rust CLI；每轮为真实输入建立新的 root cache，复用配套预编译系统库。库采用 `tests/benchmarks/inlining/members/provider`，consumer 采用上述 old-graph 程序；artifact-only 链接消费刚发布的 root／依赖 `.slib` 与 runtime 索引并实际运行。3 对测量的中位数如下，单位秒：

| 阶段 | 原版 | 新版 |
| --- | ---: | ---: |
| 小型库冷生产 | 0.516 | 0.456 |
| 消费预编译库并构建程序 | 1.452 | 1.333 |
| artifact-only 链接 | 0.901 | 0.831 |
| 原请求缓存命中 | 0.0083 | 0.0086 |

缓存命中的子编译调用为空，root 指纹保持，源码、工具和原产物始终存在。该测量覆盖小型实际 consumer，不代表完整 UI 冷构建或系统库重建时间；未以 debug Rust CLI 的构建耗时与 release CLI 作比较。
