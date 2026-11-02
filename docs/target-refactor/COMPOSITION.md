# Target 组合解耦后续重构设计

状态：提案。

日期：2026-11-03。

权威依据：[语言规范](../specs/SCOOP-SPEC.md)、[运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)、[实现规范](../specs/SCOOP-IMPL-SPEC.md)。本文是[上一轮 Target 边界重构](DESIGN.md)的后续设计，不修改其已经完成的历史范围和实施记录。

## 1. 背景与问题

上一轮重构已经建立 Rust／Python 共用的静态 target 目录，分开 host 与 output target，并将过程 ABI、Scoop ABI、EH、machine frame 和 backend 保存为显式合同。但当前实现仍保留一批“现有组合就是维度本身”的类型和分派。它们对现有七个 target 是正确的，却会让新增 OS 或 ISA 同时修改多个无关入口。

| 位置 | 当前表达 | 残留耦合 |
| --- | --- | --- |
| [target layout](../../compiler/target-data/src/layout.rs) | `LayoutKind::{DarwinAarch64, DarwinX86_64, LinuxX86_64, WindowsX86_64}` | data layout、ISA、对象格式、符号规则被一个 OS＋arch 名称绑定 |
| [runtime composition](../../compiler/toolchain/src/runtime.rs) | `ValidatedRuntimeBuildProfile` 分别投影 OS／image／loader／arch／frame／entry／shim，统一由 `bundle.c` 引用所选具名 ops | —（完整 OS＋arch runtime profile 与对应 C 文件已删除） |
| [backend](../../compiler/target-data/src/backend.rs) | 首个 AArch64 backend 常量简称 `LLVM_22_1` | 名称隐藏 Darwin／Mach-O／frame 等实际合同，容易把整份 backend profile 当作通用 AArch64 支持 |
| [codegen target](../../compiler/codegen/src/target.rs) | LLVM backend 初始化只列 AArch64／X86；red-zone 由 x86_64 推导 | ISA、过程 ABI 和 frame policy 没有分开 |
| [LLVM process options](../../compiler/codegen/src/target/machine.rs) | 一次性全局参数同时包含通用 statepoint 约束和 X86 专用选项 | 新 ISA 不能依赖调用顺序获得另一套进程全局配置 |
| codegen deployment | 测试入口按 Mach-O／ELF／COFF 推导 Darwin／Linux／Windows | OpenHarmony 与 FreeBSD 同为 ELF，却不是 Linux deployment |
| [ELF runtime image](../../runtime/src/platform/image/elf.c)／[Linux loader](../../runtime/src/platform/loader/linux_elf.c) | metadata 与 trace 都以显式 ELF image view 解析容器事实，Linux provider 独立枚举模块、解析 self path 与维护 generation | —（ELF parser／Linux loader 边界已完成） |
| [ELF reader](../../compiler/slib/src/link_object/elf/architecture.rs) | `ElfArchitecture` 只有 `X86_64` | ELF 容器入口仍等于当前唯一 machine |
| [object envelope](../../compiler/slib/src/link_object/object_envelope.rs) | `ElfX86_64Rela`、`CoffAmd64` | relocation 容器和 machine 固化在 variant 名称中，并传播到验证、指纹与 callable encoding |
| [final link](../../compiler/toolchain/src/final_link.rs) | target 目录显式绑定当前 Darwin／Linux／Windows profile，不再由 native environment 或对象格式推导 | ELF 编排仍位于 Linux 模块，需继续拆出纯格式 helper |
| Windows 工具链 | MSVC／LLVM-MinGW triple、SDK slice、include／lib 路径和 probe 固定 x86_64 | Windows ARM64 无法复用 Windows 工作流，只能侵入全部工具入口 |
| runtime 源文件 | frame recovery、ManagedEntry 对象语法和 Scoop ABI 返回 shim 已作为独立 source set 选择 | —（原先随 `darwin_aarch64.c` 同组选取的边界已拆开） |
| 准备／发行脚本 | LLVM build list 和个别 recipe 固定 AArch64／X86、`-march=`、x86_64 triple 或架构 include 目录 | 编译器支持的 ISA、host ISA 和资源 target 混为一谈 |

上一轮设计已经要求拆开 `ElfX86_64Rela` 等命名；当前代码说明这一部分只完成了选择入口的收口，还没有完成数据模型和消费者迁移。本文只处理这些剩余边界，不重做已经稳定的 target 目录、ABI planner、linker 或 runtime 架构。

## 2. 目标与非目标

### 2.1 目标

完成后，增加一个 target 应遵循以下流程：

1. 在静态目录登记一个完整、封闭且已实现的 target；
2. 由目录显式绑定 OS、ISA、对象目标、过程 ABI、backend、frame、native environment 和 final-link profile；
3. 复用合同完全相同且已经验收的实现；
4. 只为尚不存在的真实机器、OS、ABI、loader 或工具链差异增加局部实现；
5. 准备该 target 的资源并在目标环境完成编译、链接、运行和 GC／EH 验收。

共享编译流程不应因为一个新 canonical target 再增加名称分支。已经支持某一维度的模块也不应因为出现新组合而复制实现。例如 Linux／FreeBSD／OpenHarmony 的 ARM64 target 在过程 ABI、ELF machine、ISA helper 和 frame 合同相同且实测通过时，应共用同一份实现；三个 OS 仍分别拥有自己的 VM、thread stack、loader、SDK／CRT 和系统库合同。

本轮至少要从架构上覆盖以下未来移植压力：

- Linux ARM64；
- Windows ARM64；
- Linux RV64；
- Linux LoongArch64；
- OpenHarmonyOS（OpenHarmony）x86_64；
- OpenHarmonyOS（OpenHarmony）ARM64；
- FreeBSD AMD64；
- FreeBSD ARM64。

“覆盖”是指重构后的职责边界能容纳这些目标，并能指出届时只需增加哪些真实实现；不是现在把它们登记为 supported target。

本轮同时要求：

- 类型和字段名称表达自身负责的事实，不用 OS＋arch 名称代替多个正交合同；
- 选择结果仍是结构完备的类型化数据，不用 `Option` 或下游 fallback 表示“以后再实现”；
- target 目录是唯一组合根，普通 consumer 不重新列出完整 target；
- 编译器 output target 支持、编译器 host 支持和发行包支持继续是三个独立结果；
- 不改变缓存只判断更新的原则，不增加重放验证或产物来源审计。

### 2.2 非目标

本轮不做以下工作：

- 不直接加入上述八个 target 或其他新 target；
- 不在 enum、TOML、脚本、测试矩阵、LLVM build list 或 runtime source set 中预登记上述目标、OS、ISA、ABI、triple、machine id、SDK 路径或资源；
- 不允许用户任意拼接 OS、ISA、ABI、对象格式或 libc；
- 不建立 target 插件、动态 registry、trait-object 工厂或配置驱动的代码加载；
- 不把全部平台行为抽象为一套通用 capability 框架；
- 不重新设计 ABI、GC、EH、`.slib`、链接器或 sysroot 布局；
- 不为未实现的未来组合增加空枚举项、占位 source set、伪 backend profile 或跳过分支；
- 不承诺旧 profile 编码、旧 `.slib` 或旧 fingerprint 继续兼容。

文中出现未来 target 名称只用于设计压力审查，不构成目录数据、兼容承诺或实现规格。它们的精确 triple、ABI variant、feature baseline、loader 和 SDK 只能在实际移植时依据对应工具链和实机验证确定。

## 3. 拆分原则

### 3.1 只拆错误的等价关系

以下等价关系必须消除：

- OS = ISA；
- OS = 对象格式；
- 对象格式 = deployment／过程 ABI；
- ISA = frame geometry／red-zone policy；
- ISA = 任意 driver 都接受的统一 CPU flag；
- libc／SDK／CRT 环境 = OS；
- ELF = Linux = x86_64；
- COFF = Windows = AMD64；
- AArch64 = Darwin PCS；
- x86_64 = SysV AMD64。

以下合同描述不可随意拆分的真实语义，应继续具名：

- Darwin AArch64、AAPCS64、SysV AMD64、Win64 等过程 ABI；
- 固定帧几何或 Windows virtual unwind 等 frame recovery 合同；
- glibc、musl、Microsoft UCRT、LLVM-MinGW CRT 等 native environment；
- Mach-O／ELF／COFF 各 machine 的 header ABI 与 relocation 语义；
- 一个 backend profile 的 LLVM 版本、triple、CPU／features、LSDA、frame 与对象检查合同；
- 经过实际测量启用的一组 codegen tuning。

解耦不以“枚举名不含平台词”为目标。只要名字确实代表不可分割的 ABI、SDK 或已测 profile，保留平台或架构名称是正确的。

### 3.2 只在一个根节点组合

静态目录继续只登记可完整编译、链接和运行验收的组合。各字段不是公开的自由组合 API，命令行也不能覆盖其中一部分。

```text
SupportedTarget（唯一组合根）
  ├─ identity / aliases / source conditions
  ├─ os
  ├─ architecture
  ├─ object target
  │    ├─ format / machine / header ABI
  │    ├─ relocation profile
  │    └─ symbol normalization
  ├─ language ABI
  │    ├─ procedure ABI / Scoop ABI
  │    └─ exception model
  ├─ backend profile
  │    ├─ LLVM machine / CPU / features / pipeline
  │    ├─ unwind provider / LSDA
  │    └─ machine frame recovery
  ├─ native environment / C producer / Rust producer
  ├─ final-link profile
  └─ measured codegen tuning
```

consumer 只能取得自身需要的投影。它可以对自己实现的有限种类做穷尽分派，但不能从一个投影猜另一个不等价投影。例如 ELF reader 可以分派 machine，Linux VM 不能通过 ELF 被选中，Windows SDK resolver 也不能通过 COFF 被选中。

### 3.3 按拥有者复用，不建立“arch 大模块”

“ARM64 只实现一次”不等于把所有 ARM64 代码放进一个模块。一个 ISA 会在不同职责层拥有少量真实实现：

| 所有者 | 可复用内容 | 不能被它决定的内容 |
| --- | --- | --- |
| LLVM architecture | backend 初始化、LLVM build component 名称 | OS、过程 ABI、frame、SDK |
| Procedure ABI | 参数／返回分类、寄存器分配、stack alignment、red-zone policy | 对象容器、loader、libc |
| Object machine | header machine、flags、relocation、TLS、指令字段 | OS VM、CRT、final-link 默认库 |
| Runtime ISA helper | 浮点环境、纯 ISA intrinsic | ManagedEntry 符号语法、frame geometry |
| Frame recovery | stackmap 寄存器、anchor、root slot、next frame | SDK、对象 section 通用 reader |
| Toolchain machine | 某个 driver family 的 triple、CPU flags、SDK machine slice | 语言 ABI、runtime OS services |

复用发生在这些所有者内部。Linux、FreeBSD 和 OpenHarmony 不应各写一份 AArch64 relocation 或 AAPCS64 planner；同样，它们也不能被迫共用一份名为 `unix_arm64` 的 VM／loader／toolchain 模块。

### 3.4 不为消除 `match` 牺牲类型边界

合理的 `match` 应保留，例如：

- Mach-O／ELF／COFF 的 header、section 和 symbol 结构；
- AArch64／x86_64 等 machine 的 relocation 编码；
- GNU／Clang／MSVC driver 方言；
- DWARF fixed frame 与 Windows virtual unwind；
- Darwin／Linux／Windows 的 VM 和启动行为。

本轮删除的是“列举所有完整 target 的 `match`”以及“用一个维度猜另一个维度”的分支，不追求把真实平台差异藏进通用 trait。

## 4. 目标数据与 backend

### 4.1 移除组合型 `LayoutKind`

`LayoutKind::{DarwinAarch64, ...}` 退役。`targets.toml` 直接登记构造 `TargetLayoutDescription` 所需的事实：

```toml
arch = "aarch64"
llvm_data_layout = "e-m:o-..."
object_format = "MachO64"
object_machine = "Aarch64"
object_header_abi = "MachO64"
relocation_profile = "MachOAarch64"
object_unwind_image = "MachOAarch64CompactUnwindAndEhFrame"
native_symbol_normalization = "MachOExternalUnderscore"
procedure_abi = "DarwinAarch64"
scoop_abi = "ScalarInterfaceAndSmallValueParts"
exception_model = "ItaniumDwarf"
```

上例只说明字段职责，实际字段名和编码在 C0 spec 变更中冻结。它不是允许用户任意拼接的配置格式。

当前全部 target 均为 little-endian、64-bit pointer、16-byte stack alignment，`layout.rs` 可以继续提供这些当前实际共享的常量；不为未支持的 32 位、大端或不同 pointer model 建立新层。构造函数接收显式 architecture、data layout 和 object target，不再通过 OS＋arch 枚举展开。

Rust 构建目录时完成一次性检查：

- 所引用的 architecture、object target、ABI、backend、frame、native environment、final-link 和 tuning 均有实际实现；
- LLVM data layout、backend architecture 和 object machine 相容；
- backend 的 artifact format／unwind／frame 与 target 绑定一致；
- native producer 与 final-link profile 接受所选 OS、machine 和 environment。

数据进入已验证的静态常量后，各 stage 复用结果，不重复读取 TOML 或重放完整一致性检查。

### 4.2 类型化 OS 与显式投影

Rust 侧增加只包含当前已支持 OS 的封闭 `TargetOs`，并直接复用 LIR 的 `TargetArchitecture`。未来 OS 或 ISA 只有随首个真实 target 一起实现时才增加 variant。`TargetDefinition` 不再把 `os`／`arch` 只保存为自由字符串；Python 仍从同一目录得到 source condition 需要的 canonical 小写值。

`env` 继续是 source condition 的用户可见值，不替代 `NativeEnvironment`。source condition、SDK／CRT profile 和 compiler driver profile 即使当前拼写相似，也只在目录生成边界显式映射一次。

以下选择必须直接来自 target 投影，不能继续推导：

| consumer | 读取的投影 | 禁止的推导 |
| --- | --- | --- |
| runtime OS services | `TargetOs` | `NativeEnvironment -> OS`、`ELF -> Linux` |
| object reader／writer | `ObjectTargetProfile` | `ELF -> X86_64`、`COFF -> AMD64` |
| ABI planner | `ProcedureAbi` | `AArch64 -> AAPCS64`、`x86_64 -> SysV` |
| codegen deployment | 显式 deployment profile | `format -> Darwin/Linux/Windows` |
| native toolchain | `TargetOs + NativeEnvironment + ToolchainMachine` | canonical triple 字符串拆解 |
| final link | 显式 final-link profile | `ELF -> Linux link` |
| runtime frame | `MachineFrameRecovery` | `architecture -> frame` |

### 4.3 对象目标不是单独的 format 字段

对象格式与 machine 需要组合成一个由对象层拥有的、封闭且完整的 profile：

```text
ObjectTargetProfile
  ├─ format
  ├─ machine
  ├─ header ABI / flags policy
  ├─ relocation profile
  └─ symbol normalization
```

这是对象文件语义上的真实组合，不是新 target registry。`header ABI` 不能省略：有些 ELF machine 会在 `e_flags` 编码浮点 ABI、扩展或其他链接兼容信息，仅保存 `EM_*` machine 值不足以验证对象兼容性。新增 machine 时随真实实现增加 profile，不提前登记未来 machine 或 flags。

### 4.4 删除 `RuntimeProfile`

`RuntimeProfile::{DarwinAarch64, ...}` 不携带独立合同，只重复 OS、object target、architecture、ABI 和 frame 的组合，应删除 `targets.toml` 的 `runtime` 字段及对应 enum。runtime builder 直接消费这些已验证投影，并生成第 5 节定义的完整 source selection。

注册 target 必须成功形成结构完备的 `ValidatedRuntimeBuildProfile`，否则不能进入目录；不能用缺省 source、`Option` 或“未知则 x86_64”完成构造。

### 4.5 LLVM architecture 与 backend qualification 分开

LLVM architecture 层只拥有：

- 对应 LLVM target 的一次性初始化；
- 项目自建 LLVM 时所需的 component 名称；
- LLVM 接口需要的 architecture 标识。

完整 backend profile 继续拥有 LLVM 版本、triple、CPU／features、object target、unwind／LSDA、machine pipeline、frame 与已测 tuning。它会进入 fingerprint，不能拆成可任意混搭的片段。当前含糊的 `LLVM_22_1` 应改名为能表达实际合同的 profile 名；只有合同逐项相同并通过验证时，不同 OS target 才能引用同一 backend profile。

LLVM component build list 从当前目录实际引用的 LLVM architectures 去重生成。映射表只包含当前已经实现的 architecture，不把 RISC-V、LoongArch 或其他未来 backend 预先塞进构建产物。

### 4.6 进程级 LLVM 选项必须与 target 顺序无关

LLVM command-line options 在进程中只能初始化一次。当前通用 statepoint 约束与 X86 专用选项混在同一全局列表，未来不能通过“第一个编译的 target 决定配置”扩展。

重构规则如下：

- 保留一份所有当前 backend 都需要且已验证的确定性进程全局选项；
- target-specific policy 优先使用 `TargetMachine` 配置、function attributes、pass pipeline 或已维护的局部补丁表达；
- 确实只能由 LLVM 全局选项表达的规则，必须证明它对同一进程内所有已支持 backend 都安全，不能按 target 切换；
- 增加多 target 顺序测试，分别按 A→B 与 B→A 建立 machine，生成结果不得依赖首次初始化顺序。

这项工作是 RV64／LoongArch64 等新 LLVM backend 的前置条件，否则即使 target initializer 已增加，输出仍会隐式继承 X86 policy。

## 5. ABI、frame 与 runtime

### 5.1 ABI planner 只按 `ProcedureAbi` 分派

`ProcedureAbi` 是 ABI planner 的唯一机器规则入口。planner 只处理参数／结果、寄存器耗尽、扩展、aggregate、byval／caller-copy 和 sret，不读取 target OS、对象格式或 native environment。

同一 ISA 可以有多个过程 ABI；不同 OS 也可以引用同一个过程 ABI。共享的 HFA、小 aggregate 或寄存器分配算法可以作为私有函数复用，但不能让 `Aarch64` 本身成为 ABI 选择。

本轮不提前增加未来 ABI variant；只确保实际增加一个 ABI 时改动局限于：

- LIR ABI 数据及编码；
- `codegen/abi-plan` 的分类实现；
- ManagedEntry／native boundary 所需的 ABI 局部实现；
- 对应 C/Scoop ABI 测试与 fixed vectors。

普通 parser、HIR、MIR、对象容器和 OS runtime 不应因此修改。

### 5.2 frame recovery 的机制与寄存器合同分开

重构前的 `BackendMachineFrameV1::WindowsSeh` 内含 AMD64 DWARF register number，名字看似通用 Windows，数据却固定为一种 ISA。C0 先以 `MachineFrameRecoveryV1::WindowsAmd64VirtualUnwind` 准确命名现有实现；继续扩展时使用不能缺字段的和类型，例如：

```text
MachineFrameRecoveryV1
  ├─ Fixed(FixedFrameContract)
  │    ├─ stack / frame DWARF registers
  │    ├─ slot size / alignment
  │    └─ anchor / root / next-frame geometry
  └─ VirtualUnwind(VirtualUnwindContract)
       ├─ register contract
       ├─ return-PC semantics
       └─ unwind context profile
```

实际 enum 和 profile 在 C0 中只编码现有实现。这里的 `VirtualUnwind` 是恢复机制，不等于预留 Windows ARM64；当前合同仍可以准确命名为已实现的 Windows AMD64 profile。

frame 实现拆成两个边界：

- frame decoder／virtual unwind adapter 按 `MachineFrameRecoveryV1` 选择，负责 record、anchor、root slot、return PC 和 next frame；
- ManagedEntry 汇编按 `ProcedureAbi × ObjectTargetProfile` 选择，负责目标寄存器、section、symbol 和对象汇编语法。

`-mno-red-zone` 属于所选 ABI／frame 合同，不属于“所有 x86_64 ISA”的通用 flag，应从 ISA source set 移到相应 ABI／frame build component。

### 5.3 runtime component 按真实职责选择

runtime build profile 由 target 的投影形成以下结构完备的选择：

```text
ValidatedRuntimeBuildProfile
  ├─ os services                 <- TargetOs
  ├─ primitive API adapter       <- OS/runtime API contract
  ├─ object image parser         <- ObjectFormat
  ├─ loaded-image provider       <- TargetOs / loader contract
  ├─ startup / argv adapter      <- TargetOs / native environment
  ├─ trace / symbolizer adapter  <- object format + platform contract
  ├─ ISA helpers                 <- TargetArchitecture
  ├─ frame recovery              <- MachineFrameRecoveryV1
  ├─ ManagedEntry                <- ProcedureAbi + ObjectTargetProfile
  ├─ Scoop ABI shim              <- ProcedureAbi + ScoopAbi + ObjectTargetProfile
  └─ unwind runtime              <- explicit unwind provider
```

箭头表示 selector 的所有者，不是 target 目录新增任意组合轴。当前实现若没有独立的 primitive、startup 或 trace adapter，可先在现有模块内部切出可验证的函数边界；不得为未来 OS 建立空 source set。

必须避免一个泛化的 `unix` fallback：POSIX 名称不保证 thread stack 查询、VM flags、RSS、动态 loader、unwind library、argv 或 TLS 行为完全一致。只有源码实际依赖的 API 合同相同并在两个现有调用方中成立时，才提取 POSIX 公共模块；否则保留各 OS adapter。

### 5.4 ELF image parser 与 loaded-image provider 分开

重构前 `runtime/src/platform/image/elf.c` 同时承担：

- ELF program header／segment 与 Scoop stackmap range 处理；
- 通过 Linux loader API 查找当前已加载 image。

前者属于 ELF image parser，后者属于 OS／loader provider。重构后 Linux provider 继续使用其已验证 API，ELF parser 接收 provider 给出的 image／segment view。FreeBSD 或 OpenHarmony 只有在实际 API 和生命周期验证通过后才能复用 provider；它们可以无条件复用的只是 ELF 结构处理。

当前 metadata image 路径已经完成该边界：`platform/image/elf.c` 只消费 `ScoopElfLoadedImageView`，`platform/loader/linux_elf.c` 独立调用 `dl_iterate_phdr`、取得 linker symbols 并导出 Linux ops。runtime source composition 分别选择 parser 与 provider；没有为尚未实现的 OS 保存 provider 数据。

stack trace 也采用相同边界：`stack_trace/elf.c` 只解析显式 image view 中的 ELF build-id note，`stack_trace/linux.c` 才负责 `dl_iterate_phdr`、`/proc/self/exe` 与 loader generation；libbacktrace 的 ELF/DWARF reader 继续属于对象解析 source set。

### 5.5 ISA helper、ManagedEntry 与 ABI shim 分开

真正只依赖 ISA 的实现放在 `runtime/src/platform/arch/`，例如浮点控制环境。依赖 Scoop 返回约定和对象符号语法的汇编 shim 不属于纯 ISA helper，应移入明确的 ABI／entry source set。

现有 `darwin_aarch64_floating.S` 同时依赖 AArch64 返回寄存器和 Mach-O 符号语法，不能作为通用 AArch64 源，也不能与 frame decoder 隐式绑定。以后出现 AArch64 ELF target 时，只应为尚无实现的对象语法／ABI shim 增加局部汇编，不复制浮点解析的普通 C 实现、AArch64 ISA helper 或 ABI planner。

### 5.6 runtime bundle 只有一个组合入口

重构前的四个 `runtime/src/platform/profiles/<os>_<arch>.c` 只负责把 image、VM 和 frame ops 指针放入 `ScoopPlatformBundle`，会迫使每个新组合复制一个目标文件；这些完整组合文件已经删除。

当前 runtime builder 始终编译同一份 `runtime/src/platform/bundle.c`，并由 `ValidatedRuntimeBuildProfile` 分别选择 metadata image、thread／VM、managed frame 的具名 ops 以及目标 C `long` 宽度。选择 flags、source set 和其他构建参数一起进入现有 runtime 更新记录。它只做静态 C 组合，不增加生成器、运行时插件、函数注册表或动态分发，也不保存任何未来 target 信息。

各具体 image／VM／frame ops 保留具名符号，便于模块级 C 测试直接调用；公共 bundle translation unit 是唯一汇合这些投影的 runtime 源，但其源码本身不知道完整 target 名称。

## 6. 对象格式、relocation 与 unwind

### 6.1 容器、machine 和 header ABI 分层

对象读取维持现有 reader，不增加第二套通用 reader：

```text
ELF container reader
  ├─ ELF header / section / symbol / REL[A] bounds
  └─ selected ObjectTargetProfile
       ├─ machine and header flags
       ├─ relocation semantics
       └─ TLS / call / address instruction rules
```

Mach-O 和 COFF 采用相同职责原则。格式层负责容器边界，machine profile 负责 machine 值、flags、relocation kind、指令字段、TLS 和调用／地址语义。

ELF `e_flags`、COFF machine 和 PE optional-header machine 相关约束必须由 object target 检查，不能只比较格式。未来 RISC-V／LoongArch target 的 ABI 或 extension 信息也不能藏进 canonical triple 或默认值。

### 6.2 relocation envelope 使用格式 variant＋显式 machine

顶层 relocation shape 不再把当前唯一 machine 写进 variant 名：

```rust
enum ObjectRelocationShapeV1 {
    MachO(MachORelocationShapeV1),
    ElfRela(ElfRelaRelocationShapeV1),
    Coff(CoffRelocationShapeV1),
}

struct ElfRelaRelocationShapeV1 {
    machine: ObjectMachineV1,
    kind: u32,
    target_symbol: u32,
    addend: i64,
    storage_width: u8,
}
```

只保留当前 reader 真正支持的 RELA variant；以后只有在实际 target 产物需要 REL 时才增加对应格式 variant。`CoffAmd64` 同样迁移为带 machine 的 COFF shape。Mach-O 在不丢失成对 relocation 语义的前提下补齐一致的 machine 所属表达。

verified shape、form、undefined requirements、callable encoding、pointer／TLS semantics 和 strong relocation closure 同步迁移；不保留 `ElfX86_64Rela`／`CoffAmd64` 兼容 variant。

`storage_width` 只表达对象中受影响的存储范围，不声称所有 relocation 都是简单整数写入。PAGE／LO12、branch、relaxation pair 等指令字段必须由对应 machine 模块验证和解释。

### 6.3 unwind image 与 GC frame 是两个合同

异常展开、GC root frame recovery 和对象 unwind section 会相互关联，但不能合并为一个 `arch` 开关：

- `ExceptionHandling` 决定语言异常模型；
- unwind provider 决定 libunwind／platform unwind API 和链接需求；
- object target 决定 `.eh_frame`、`.pdata`／`.xdata` 等容器语义；
- machine frame recovery 决定 runtime 如何从 statepoint／stackmap 恢复根与调用链。

Windows ARM64 的前置工作因此不是把现有 `WindowsSeh` 重命名为通用 SEH，而是先让 virtual unwind 机制接收显式 machine/register/object unwind contract。RISC-V／LoongArch 的前置工作则是让 DWARF 与 stackmap machine 信息由 backend qualification 产生，不从 ELF 推导。

### 6.4 对象目标核对只做一次

ELF／Mach-O／COFF reader 在入口使用已选 target 核对 format、machine 和 header ABI，形成 validated envelope。下游 relocation 验证消费 envelope 已保存的 machine，不重新从完整 target 或 canonical triple 推导。

对象未变化时，不在 callable、stackmap、linker 等多个消费者重复解析 header 来证明 machine；这些消费者只检查自己负责的引用、语义和 ABI 边界。

## 7. toolchain、final link 与脚本

### 7.1 `NativeEnvironment` 只表示 SDK／CRT／libc 合同

`NativeEnvironment` 表示真实 SDK／CRT／libc／C++ ABI 和链接环境，应保留为封闭枚举。它不再用于选择 runtime OS、ISA、image 或 frame。

toolchain operation 从 target 获得以下显式输入：

```text
NativeToolchainSelection
  ├─ TargetOs
  ├─ ToolchainMachine
  ├─ NativeEnvironment
  ├─ C / C++ / ObjC producer profile
  ├─ SDK / sysroot selection
  └─ FinalLinkProfile
```

`ToolchainMachine` 是 driver／SDK 层对现有 architecture 的映射，不是另一个语言 architecture。Windows toolchain 可以只有当前已实现的 AMD64 machine profile，但公共 workflow 必须把它作为参数使用，不能在 triple、SDK lib path、macro probe、resource compiler 和 linker invocation 中散落 x86_64 常量。未来增加另一 machine 时只增加真实 mapping 与 probe，不复制 Windows 工作流。

未知 architecture 在对应工具入口明确报错，不回退到 x86_64。默认 C driver 只有在 host 与 target machine 相容时才能使用本机名称；交叉目标必须使用实际 cross driver 或显式 Clang target＋sysroot，不能静默调用宿主 `gcc`。

### 7.2 final link 分开 ELF 结构与 OS 环境

`ValidatedFinalLinkProfile::{Darwin,Linux,Windows}` 当前把对象容器编排和 OS 环境放在同一层。重构后保持少量具体 profile，不建立 linker plugin，但拆开两类事实：

- 格式／linker-family 语义：输入对象顺序、response file、shared／executable 输出形态、ELF SONAME、必要的 section／relocation handling；
- 平台环境：CRT start files、dynamic loader、默认 PIE／static policy、系统库、libc／C++ runtime、unwind provider、SDK 搜索与 rpath 约定。

已有 Linux profile 作为实际调用方使用提取出的 ELF helper；FreeBSD／OpenHarmony 不在本轮建立 profile。未来只有在其 linker 语义逐项相同时才复用该 helper，不能把整个 Linux final-link 流程改名为 ELF。

最终选择由 target 直接引用 `FinalLinkProfile`，不能由 ELF 或 `TargetOs` 单独推导。这样同一 OS 的不同 native environment 可以有不同 CRT／driver，而同为 ELF 的不同 OS 不会误入 Linux 默认库和 loader。

当前 target 目录已经为现有七个 target 显式绑定 `FinalLinkProfile::{Darwin,Linux,Windows}`；`ValidatedFinalLinkProfile` 只消费该投影，`NativeEnvironment` 继续负责各 profile 内部的 SDK／CRT／libc 分支。该选择进入 final-tool 发现缓存键，没有为未实现平台增加 profile。

### 7.3 Windows 工作流先参数化 machine

Windows ARM64 要复用现有 MSVC／LLVM-MinGW 流程，当前 Windows 路径必须先完成以下清理：

- target triple 和 compiler／linker target argument 从 machine profile 取得；
- MSVC host/target tool slice 与 Windows SDK lib subdirectory 显式区分；
- predefined macro／object machine probe 不固定 `_M_X64` 或 AMD64；
- import library、resource compiler、PDB／debug 和 final linker 接受 target machine；
- runtime C/assembly flags 来自 ABI／object target，不由 `Windows` 推导 x86_64。

本轮 profile 仍只包含已支持的 Windows machine。参数化后，现有行为和 tool discovery 结果应完全不变。

### 7.4 LLVM 构建与 Python recipe 消费同一事实

`scripts/build_llvm.py` 的 `LLVM_TARGETS_TO_BUILD` 不再手写 AArch64／X86 常量，而从当前 target catalog 实际引用的 LLVM architectures 去重生成；catalog reader 内的 architecture→LLVM component 映射只列当前实现。

`scripts/target_catalog.py` 继续是 `targets.toml` 的薄 reader。公共 helper 可以提供 canonical OS／arch／env 和被多个当前 recipe 实际使用的 CPU flag 拼写；各第三方 recipe 保留上游要求的局部 target mapping，例如某个库的 Configure 名称，不建立“所有工具统一 target”框架。

本轮审计并修正以下模式：

- 固定 `x86_64-unknown-linux-*` 或 `x86_64-*-windows-*`；
- 对某一 ISA 绕过已有 machine helper，直接散落 `-march=`；
- 固定 `/usr/include/x86_64-*` 或 SDK machine 目录；
- 通过 canonical target 字符串后缀猜 libc、OS 或 architecture；
- 把 host architecture 当作所有 distribution target 的 architecture。

这些修改只处理当前已有 target 的实际调用，不建立未来资源记录或通用交叉发行系统。

### 7.5 output target、host 与发行支持保持独立

LLVM 能生成某一 ISA 的对象，不代表 `scoopc` 已能在该 ISA host 上运行，也不代表项目已发布该 target 的 sysroot／native resources。三类状态分别由：

- output target catalog；
- host build／bootstrap 支持；
- resource preparation／distribution manifest；

独立表达。增加 output target 时不应被迫增加 host package；增加 host package 也不能自动宣称该 output target 已完成 ABI、GC 和 linker 验收。

## 8. 未来 target 压力矩阵

本节只验证职责边界，不是待登记的 target 数据。表中的“可复用”都以精确 ABI、LLVM 输出和目标环境验证通过为前提。

| 未来 target | 应复用的现有／公共能力 | 实际移植时才增加的局部实现 | 本轮必须先扫除的障碍 |
| --- | --- | --- | --- |
| Linux ARM64 | Linux OS services、Linux loader、ELF 容器、Linux native environment 与 final-link 平台合同；已有 AArch64 LLVM initializer／ISA helper 中合同相同的部分 | 实际过程 ABI、AArch64 ELF object profile、Linux backend／frame qualification、ELF entry／shim、toolchain machine 和资源 | `LayoutKind`／`RuntimeProfile` 完整组合、AArch64 与 Darwin ABI／Mach-O 绑定、ELF 与 x86_64 绑定 |
| Windows ARM64 | Windows OS services、PE/COFF 容器、Windows SDK／CRT tool discovery、PDB/resource/import-library/final-link 工作流 | 实际过程 ABI、COFF machine／relocation、ARM64 unwind／frame、ManagedEntry／shim、machine SDK slice 与验证资源 | `COFF = AMD64`、`Windows = x86_64`、SEH profile 内嵌 AMD64 register、Windows triple/path/probe 硬编码 |
| Linux RV64 | Linux OS services、Linux loader adapter、ELF 容器、Linux native environment 与 final-link 平台合同 | 实际 psABI profile、LLVM backend qualification、ELF header flags／relocation／TLS／relaxation、frame／stackmap、entry／ISA helper、toolchain machine 和资源 | `ELF = x86_64`、LLVM 全局 X86 policy、architecture build list 固定、object header 不表达 ABI flags |
| Linux LoongArch64 | 与 Linux RV64 相同的 Linux／ELF／toolchain 公共层 | 实际过程 ABI、LLVM backend qualification、LoongArch ELF flags／relocation、frame／stackmap、entry／ISA helper、machine flags 和资源 | 同上；不能用“64 位 ELF”共享 relocation／DWARF register 默认值 |
| OpenHarmonyOS x86_64 | 已验证的 x86_64 ABI／ELF machine／ISA helper／fixed-frame 组件 | OpenHarmony OS services、loader、SDK／CRT／native environment、final link、system-library backends 与资源 | `ELF = Linux`、triple 中出现 `linux` 就选择 Linux、ELF parser 与 Linux loader 合并 |
| OpenHarmonyOS ARM64 | OpenHarmony 的 OS／loader／SDK／final-link 层；经验证的 AArch64 过程 ABI、ELF machine、ISA helper 和 frame | 缺失的 AArch64-OHOS ABI／unwind 差异、toolchain machine、entry／shim 与资源 | OS 与 ISA 必须在目录组合；不能复制一套 `openharmony_arm64` 全栈，也不能借 Linux ARM64 profile 代替 OHOS |
| FreeBSD AMD64 | 已验证的 SysV AMD64／ELF machine／ISA helper／frame 组件及纯 ELF helper | FreeBSD OS services、loader、SDK／CRT／native environment、final link、system-library backends 与资源 | `ELF = Linux`、POSIX = Linux、Linux CRT／loader／系统库混在 ELF 编排 |
| FreeBSD ARM64 | FreeBSD 的 OS／loader／toolchain／final-link 层；经验证的 AArch64 ABI／ELF machine／ISA helper／frame | 缺失的 FreeBSD ARM64 ABI／unwind 差异、machine profile、entry／shim 与资源 | 同时验证“OS 横向复用”和“ISA 纵向复用”，不得产生第二份 FreeBSD OS 或第三份 ARM64 实现 |

矩阵揭示了两类不同工作：

- 新 OS：应主要增加 OS services、loader、native environment、final-link 和系统库平台 backend；不重写已有 ISA／object machine／ABI；
- 新 ISA：应主要增加 LLVM architecture、过程 ABI、object machine、frame、entry／shim 与 toolchain machine；在同一 OS 下不重写 VM、loader 和系统库中与 ISA 无关的部分。

新 OS＋新 ISA 同时出现时，仍分别实现两类能力，再由唯一 target 根绑定；不增加一个完整 `<os>_<arch>` 平行实现。

## 9. crate 与文件级迁移

| 范围 | 主要修改 | 明确不做 |
| --- | --- | --- |
| `compiler/target-data` | 显式 layout／object target 字段、类型化 `TargetOs`、删除 `LayoutKind`／`RuntimeProfile`、backend 命名和目录构建检查 | 不执行工具探测或 ABI 算法 |
| `compiler/lir` | 同步实际 ABI／frame／object identity 编码和 fixed vectors | 不依赖 `scoop-target-data`，不增加 target registry |
| `compiler/codegen/abi-plan` | 只按 `ProcedureAbi` 分类，整理有当前调用的私有公共算法 | 不读取 OS／format／native environment |
| `compiler/codegen` | LLVM architecture 初始化、顺序无关的进程配置、显式 backend／deployment／tuning 投影 | 不按 canonical triple 或 format 选择 OS |
| `compiler/slib` | object profile；格式＋machine relocation shape；machine-local semantics | 不另建通用对象 reader 或重复语言语义验证 |
| `compiler/linker` | 消费 validated object machine／header ABI；保留格式级链接规则 | 不从 format 推导 OS／ABI |
| `compiler/toolchain` | runtime component 组合；显式 OS／environment／machine；Windows machine 参数化；ELF helper 与平台 final-link 分层 | 不把 SDK／driver 命令放入 IR，不建 plugin |
| `runtime` | ELF parser／loader、frame／entry、ABI shim／ISA helper 分组；bundle 单点生成 | 不增加动态 dispatch、`unix` fallback 或 target 插件 |
| `scripts`／fixture runner | LLVM build components 和 machine 参数消费目录投影 | 不把未来 target 加入执行矩阵 |
| 系统库 | 继续按 source condition 选择真实 OS backend | 不因 compiler 能生成对象就宣称全部系统库已支持 |

## 10. 实施顺序

### C0 规范与基线

在代码变更前同步实现规范第 1、2.4～2.8 节和运行时规范 3.2、5.2～5.5 中涉及的职责描述：

- 明确 OS、architecture、object target、过程 ABI、frame、deployment、native environment 和 final-link 的独立所有者；
- 将 runtime profile 改为 component composition；
- 将 relocation envelope 改为格式＋machine，并纳入 header ABI；
- 明确 LLVM 进程级配置不能依赖 target 建立顺序；
- 不改变现有七个 target 的语言、ABI、GC、EH 或最终链接行为。

保存现有 target/backend fingerprint、代表性 ABI plan、对象 relocation、runtime source/flag 列表和 final-link plan 的小型 fixed vectors，作为迁移对照；不保存整份 IR 或大对象快照。

同时把第 8 节八项作为设计审查清单，但不写入 target catalog 或执行矩阵。

### C1 target-data 与显式投影

- 增加只含当前 OS 的类型化 `TargetOs`；
- 将 layout 和 object target 的显式事实移入目录；
- 删除 `LayoutKind` 和 `RuntimeProfile`；
- 修正 backend 常量命名；
- 更新目录生成、fingerprint 和 fixed vectors。

出口是现有七个 target 产生与迁移前相同的实际 layout、object、ABI、backend、native environment 和 source condition 投影。

### C2 codegen architecture 与 backend qualification

- 抽出 LLVM architecture initializer／build component 映射；
- 分离进程通用选项与 X86-specific policy；
- 让 deployment 和 red-zone 等 policy 消费显式 profile；
- 增加同进程多 target 建立顺序测试；
- LLVM build script 从当前目录派生 component set。

出口是当前 AArch64／X86 backend 无论初始化顺序如何都生成相同合同，且没有 object format→OS 推导。

### C3 object envelope 与 unwind identity

- 迁移 ELF／COFF relocation shape；
- 将 machine 和 header ABI 贯通 observed、verified、wire、callable、stackmap 和 closure；
- 拆开 exception model、unwind provider、object unwind image 与 frame recovery；
- 删除旧 variant 及测试 helper。

出口是现有对象和 `.slib` 全部按当前格式直接演进并通过读取、ODR、TLS、EH、stackmap 与 artifact-only link 测试。

### C4 runtime 与 loader composition

- 拆开 ELF parser 与 Linux loaded-image provider；
- 拆开 frame decoder、ManagedEntry、ABI shim 和 ISA helper；
- 将 red-zone 等 flags 移到实际 ABI／frame component；
- 删除手写完整组合 profile，生成唯一 bundle translation unit；
- 让 runtime 从显式 OS、object、arch、ABI、frame 和 unwind 投影选择 source set。

出口是现有七个 target 的 runtime source/flag 集合与迁移前语义等价，GC、EH、stack trace 和 native boundary 定向测试通过。

### C5 toolchain、final link 与脚本

- `NativeEnvironment` 不再承担 OS 选择；
- 参数化 Windows machine；
- 从 Linux final-link 中提取只有 ELF 语义的现有 helper，Linux profile 显式提供 CRT／loader／系统库；
- 清理 Rust／Python 中现存的单架构 fallback；
- 参数、tool probe、sysroot 和发行 recipe 使用显式 target architecture；
- 更新缓存配置输入，但不改变命中顺序或增加输入读取。

出口是所有现有 target 的 C/C++/Objective-C、runtime、native 依赖、最终链接和安装包路径不变，或按当前未发布格式直接演进后闭合。

### C6 回归与八目标压力审查

完成现有七目标回归，并逐项执行第 8 节静态审查。审查方法是为每个未来 target 写“实际加入时需要触及的所有者”清单；如果清单仍要求复制已有 ARM64／AMD64 实现，或在 format／environment consumer 中增加完整 target 名称分支，则回到相应阶段修正。

审查产物只保留职责结论，不登记、构造或探测假的未来 target。

## 11. 验证与完成标准

### 11.1 结构测试

- 目录中的每个现有 target 都能形成完整 LIR/backend/runtime/toolchain/final-link selection；
- architecture、backend architecture、object machine、header ABI 和实际对象 header 一致；
- 没有 consumer 通过 canonical triple 字符串猜 OS、arch、format、ABI 或 libc；
- `NativeEnvironment` 不再被 runtime 或 codegen 当作 OS；
- ABI planner 不读取 OS／format，object reader 不从 format 猜 machine；
- codegen deployment 不从 object format 猜 OS；
- LLVM process global configuration 不依赖 target 建立顺序；
- Windows 公共工具链路径不再散落 AMD64 常量，当前 machine 由显式 profile 提供；
- ELF parser 不包含 Linux-only module enumeration；Linux final-link 平台部分不冒充通用 ELF；
- artifact-only reader/linker 不依赖 codegen、inkwell 或工具探测。

### 11.2 行为测试

现有七个 target 保留以下覆盖：

- C/Scoop ABI 标量、窄整数、HFA、小／大 aggregate、sret、byval／caller-copy 和寄存器耗尽；
- ELF／Mach-O／COFF header、section、symbol、relocation、TLS、COMDAT／weak、EH 和 stackmap；
- moving GC、call／invoke、多层／大 frame、callback、native transition 和异常重抛；
- native C／C++／Objective-C、archive／dynamic provider、artifact-only relink；
- debug/release、独立调试文件、正式 CLI fixture 和适用系统 Cone；
- runtime、native 依赖和发行资源的冷构建、正常命中、真实输入变化与失败不发布。

GC stress 只用于 GC/frame 正确性定向测试。普通功能不因本轮重构增加 stress 组合或重复冷／热 fixture。

### 11.3 backend qualification 门槛

实际增加新 ISA 或新的 backend profile 时，不能只凭 LLVM 接受 triple 或成功输出一个对象就登记 target。至少需要覆盖：

- `call` 与 `invoke`、LSDA／landing pad 和异常重抛；
- `gc.statepoint`／`gc.relocate`，寄存器与 stack root；
- 多层、大 frame、register pressure、return-PC 和 frame walk；
- PIC／PIE、TLS、外部 call、relocation 与 artifact-only link；
- ABI 分类与 C round-trip；
- release 优化下的 moving GC 和 native transition。

qualification 结果绑定 LLVM 版本、triple、CPU／features、pipeline、procedure ABI、frame 和 object target；不能只在 `TargetArchitecture` 上保存一个“支持 stackmap”的布尔值。

### 11.4 完成标准

满足以下条件才算完成：

- 组合型 `LayoutKind` 和 `RuntimeProfile` 已删除；
- generic object envelope 不再出现 `ElfX86_64Rela`／`CoffAmd64` 顶层 variant；
- Windows SEH／toolchain profile 不再把 AMD64 数据藏在看似通用的 Windows 入口；
- runtime source selection 不列举完整 target 或完整 OS＋arch profile；
- ELF 与 Linux、COFF 与 AMD64、ISA 与 procedure ABI／frame 的错误等价关系均已删除；
- OS、ISA、object、ABI、frame、native environment 的选择只发生在各自所有者或唯一 target 组合根；
- 现有七个 target 的 ABI、GC、EH、对象和链接行为通过回归；
- 没有旧数据模型的兼容 wrapper、双 reader 或平行生产路径；
- 第 8 节每个未来 target 的预计改动都局限于其真正新增的所有者，不要求复制已有 OS 或 ISA 全栈。

不要求仓库中完全没有 `darwin_aarch64`、`linux_x86_64` 等名称。真实过程 ABI、frame 实现、目标测试和发行资源可以保留准确的组合名称；完成标准关注的是这些名称不再承担通用维度或 fallback 的职责。

## 12. 调研依据与当前结论

### 12.1 LLVM statepoint／stackmap

用项目现有 `runtime/tests/linux_frames_test.ll` 的真实 `gc.statepoint`／`gc.relocate` 输入对 LLVM 22.1.8 做了探索性探针：`riscv64-unknown-linux-gnu` 与 `loongarch64-unknown-linux-gnu` 都能在 O2／PIC 下生成含 `.llvm_stackmaps` v3 和 `.eh_frame` 的对象，并产生各自 machine relocation 与 DWARF register location。

这只能说明 LLVM 22 已具备继续实测的入口，不能视为 Scoop backend qualification。LLVM 的公开 [StackMaps and PatchPoint Intrinsics](https://llvm.org/docs/StackMaps.html) 文档列出的支持架构与该探索性结果并不完全一致，因此本设计要求以锁定 LLVM/profile 的 call／invoke／EH／大 frame／register pressure／GC 实测为准，不从文档列表或一次 `llc` 成功推导支持状态。

### 12.2 ISA ABI 与对象 flags

- [RISC-V ELF psABI](https://github.com/riscv-non-isa/riscv-elf-psabi-doc) 将调用约定、ELF relocation 和 `e_flags` ABI 信息作为 machine-specific 合同，支持本文将 object header ABI 与通用 ELF reader 分层的设计；
- [LoongArch ELF ABI](https://loongson.github.io/LoongArch-Documentation/LoongArch-ELF-ABI-EN.pdf) 同样定义独立的过程 ABI、ELF flags、relocation 和 DWARF register mapping；
- [Windows ARM64 ABI conventions](https://learn.microsoft.com/en-us/cpp/build/arm64-windows-abi-conventions?view=msvc-170) 说明 Windows ARM64 不能由“通用 AArch64＋现有 AMD64 SEH”拼出，需要显式 procedure ABI 与 unwind/frame 合同。

因此 `architecture` 不能独自决定 ABI、frame 或完整对象兼容性；但相同的已验证 procedure ABI／object machine profile 可以被多个 OS target 引用。

### 12.3 OS 与 ELF 环境

- OpenHarmony 的构建资料展示了按 target machine 组织的独立 Clang/sysroot 环境，例如 [OpenHarmony 三方库移植说明](https://gitee.com/openharmony/build/wikis/gn%E6%9E%84%E5%BB%BA%E4%B8%89%E6%96%B9%E5%BA%93/%E5%9F%BA%E4%BA%8E%E4%B8%89%E6%96%B9%E7%BC%96%E8%AF%91%E6%A1%86%E6%9E%B6%E7%A7%BB%E6%A4%8DOHOS) 中的 OHOS target toolchain/sysroot；triple 中含 `linux` 不能成为选择 Linux runtime 的依据；
- FreeBSD 官方 [Supported Platforms](https://www.freebsd.org/platforms/) 将 AMD64 与 AArch64 列为独立支持平台，但这不意味着其 loader、thread stack、VM、CRT 或系统库可以直接使用 Linux 实现。

结论是 ELF、POSIX API 表面和 64 位指针都不足以构成 OS/runtime 等价性。可跨 OS 复用的是经过边界拆分的 ELF parser、machine semantics、procedure ABI 和 ISA helper；OS services、loader、native environment 与 final link 必须分别验证。
