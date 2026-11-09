# Windows 编译性能任务单

日期：2026-11-10。调研基线：`f27eda5ee1`。状态：**核心实现与跨平台功能验证完成，安装包验收中**。详见[实施记录](WINDOWS-PROGRESS.md)。

目标是减少日常构建和改代码后检查的实际开销，优先处理工具与缓存记录、小文件创建和宿主编译器的堆分配。Windows 的 `RtlFreeHeap` 热点及小文件创建显著慢于 macOS/Linux 来自用户的实际采样与观察；代码路径核查与 Windows 本机对比均已完成，实测结果见实施记录。下面的文件数量关系来自调研基线，不是某次 `scoop.ui` 构建的实测总数。

## 已确定的行为

1. 发行版携带配套的系统 `.slib`；额外源码依赖使用项目构建缓存，不增加全局 `.slib` 构建缓存。PM 的共享源码下载缓存属于另一用途。
2. 更新发行版后由用户执行 `scoop clean`。普通构建不承担工具链文件、版本、SDK 状态的持续巡检，也不维护为这些状态服务的额外缓存失效链。显式放在 `target` 外的项目缓存按其位置清理。
3. 更新判断使用实际源码输入、`Cone.toml` 的相关有效配置和被依赖的 `.slib` 等已有因素。头文件搜索路径、native 编译参数已经由清单配置覆盖；它们只影响实际包含相应原生源码的 Cone。保留实际包含的头文件等正常源码依赖，不把整个 SDK 目录树变成额外输入。
4. 缓存判断先于源码内容读取、预处理、输入复制、工具探测和链接准备。命中直接使用已有结果；不读回产物复核内容或生产过程。需要执行编译/链接时才准备工具路径与调用参数，同次调用内复用已经取得的数据。
5. `scoop check` 每次检查当前根 Cone，执行 Parser/HIR 和必要的 generator，返回本次诊断；删除整次检查成功结果的缓存。根不进入 MIR/LIR/codegen/link，依赖仍正常复用预编译库或普通构建缓存。
6. 同一事实在明确边界检查后复用结果。保留实际格式、类型、引用、ODR、ABI 和 GC 契约检查，不为证明前一步已完成而重复扫描，也不引入验证凭证或平行 reader。

实施涉及 pipeline 职责或命令行为时，先修订对应 spec，再改实现和测试。现有 spec 中的工具巡检和检查成功缓存条款属于待修改内容，不能据此继续保留这些机制。

## 工作顺序

| 编号 | 优先级 | 工作 | 完成时应看到的变化 |
| --- | --- | --- | --- |
| W0 | P0 | 建立 Windows CPU、进程和文件事件基线 | 知道每个阶段的创建次数、写入次数、子进程数和耗时 |
| W1 | P0 | 简化更新判断，删除工具巡检记录 | 热构建不读写大型工具记录，不扫描工具/SDK 状态 |
| W2 | P0 | 删除 `scoop check` 成功结果缓存 | 每次检查当前代码，无 `checks-v4` 的记录、锁及整次 key 工作 |
| W3 | P0 | Windows 宿主编译器换 malloc | Rust 前端使用成熟的替代分配器，记录实际耗时和 RSS 变化 |
| W4 | P0 | 减少生成 C 桥的文件与编译进程 | 多个桥接实现批量编译；对象修补在内存交接 |
| W5 | P0 | 减少原生源码中转文件和临时目录 | 消除可避免的逐源码目录、预处理落盘和重复中转 |
| W6 | P1 | 减少依赖读取与 Link 的重复验证 | 同一依赖图、section 和已验证事实不反复解码、扫描 |
| W7 | P1 | 清理剩余记录，缓冲必要写入 | 只保存有实际复用价值的数据，消除大量 token 级文件写入 |
| W8 | P1 | PM prepare 改用管道传递请求/响应 | 每次 manifest 命令不再创建两份 IPC 文件及临时目录 |
| W9 | P2 | 复测剩余前端热点与发行准备文件量 | 根据剩余热点安排后续工作，区分日常编译与发行准备 |
| W10 | P1 | 显示每个 Cone 的编译进度与耗时 | 常规终端显示实际 `scoopc` 调用的开始、成功/失败和耗时 |
| W11 | 交付 | 三平台验证与安装包 | macOS、Linux、Windows 各产出一套可安装、可编译运行的发行包 |

W0 为各项保留改动前对照，不是重新讨论是否更换分配器的前置决策。W1/W2 删除记录后，再确定 W7 的剩余范围。W4 先完成内存修补交接，再完成桥接批量编译；W5 先消除目录放大，再处理实际工具支持的输入方式。

## 小文件生成点核查

这里的“文件数”指一次执行中创建的不同文件路径；同一路径重复写入另行计数。临时文件会被清理，不能只统计命令结束后剩余的文件。

**桥接单元的含义：** `GeneratedBridgeUnit` 表示编译器生成的一份具体 C 桥接实现，例如需要参数/返回值适配的外部函数包装、原生全局变量的读取/写入/取地址包装，或者 C 回调入口。全局变量的这三种操作分别属于不同单元。它按桥接实现区分，不按调用次数计数，也不等于 Cone、Scoop 源文件或普通函数。调研基线让每个桥接单元对应一个 C 翻译单元；逻辑身份可以独立于物理文件分组。

下表记录调研基线，以 `B` 表示当前 Cone 生成的桥接单元数，`N` 表示需要编译的原生源码数，`R` 表示需要编译的 runtime 源文件数。

| 步骤 | 当前文件与进程模式 | 影响与优化判断 |
| --- | --- | --- |
| 生成 C 桥 | 每个桥接单元写一个 `.c`、生成一个 `.o`：约 `2B` 个文件、`B` 次 C 编译；对象边界修补还会读出、写回，再被 driver 读一次 | 会随桥接规模线性放大，是重点。合并可共同编译的生成代码；对象第一次读入后在内存修补并直接交接。`B=100` 时这条路径产生约 200 个文件和 100 次编译进程 |
| 用户 C/C++/ObjC 源码 | 每个源码新建临时目录，写 `.i/.ii/.mi`，另有 `.o`；约 `N` 个逐源码目录、`2N` 个主要文件，预处理和编译各一次 | 会随原生源码数放大。复用 Cone 的 scratch 目录；验证通过 stdin 输入预处理文本或合并必要步骤。预处理的 `unit.d` 共用路径，不能误算成 `N` 个独立 depfile |
| PM prepare | 每次调用创建临时目录，写 `request.json`、由 PM 写 `response.json` 后读回 | 即使后续构建命中也有固定开销。改用 stdin/stdout 可以直接消除这两份文件；数量按命令增长，不能称为逐函数文件爆炸 |
| 普通编译缓存和 scratch | 每个 Cone/结果有记录、锁、产物发布临时文件；父调度、child、driver、generated-C 还分别管理临时目录 | 按 Cone/更新次数增长。删除退役记录和锁，空目录延迟创建，梳理可共用的临时目录生命周期；保留必要的原子发布与并发协调 |
| 最终链接 | 当前按最多 64 MiB 或 8192 个成员分批写 archive，Windows 使用 `.lib`；有少量索引、锁与 startup 的 `.c/.o` | 已经从内存对象批量生成 archive，没有逐成员解包落盘。文件数按批次增长；startup 是每次冷链接的少量固定成本 |
| LLVM 普通对象与 `.slib` 发布 | LLVM 对象直接写内存；成员在内存组装、压缩，整个 `.slib` 通过一个临时文件发布 | 当前没有每函数 `.o` 落盘，也没有发布时预填充全部 `objects-v1`。容器里的大量成员不能算作操作系统小文件 |
| runtime 源码构建 | `R` 个对象；复用预处理/depfile 路径，通常每源码有预处理和编译两个进程 | 适用于显式源码构建和开发 sysroot 准备。发行版普通应用消费预编译 runtime，不应重复进入这条路径 |
| Windows 发行准备 | MinGW 头文件逐个复制，目标配置头与 `.a/.o` 也需要输出；第三方源码解包及供应者构建另有大量文件 | 头文件数量大时确实重要，但属于发行准备。公共 MinGW 头目录当前已经跨两个 CRT 目标共享，不能重复立项“首次共享” |

主要证据：

- C 桥：[源码分组](../../compiler/codegen/src/c_bridge.rs)、[逐单元编译](../../compiler/codegen/src/c_bridge_emission.rs)、[对象边界修补](../../compiler/codegen/src/generated_c_atom_boundaries.rs)、[driver 对象读取](../../compiler/driver/src/object_production/members/codegen.rs)。
- 原生源码：[预处理](../../compiler/toolchain/src/native/prepare.rs)、[逐源码中转](../../compiler/toolchain/src/native/command.rs)、[对象输出](../../compiler/toolchain/src/native/objects.rs)。
- PM 与临时目录：[prepare](../../compiler/scoop/src/cli/package_manager/prepare.rs)、[staging](../../compiler/scoop/src/snapshot/staging.rs)、[child](../../compiler/scoop/src/child.rs)、[driver preflight](../../compiler/driver/src/request/preflight.rs)。
- 已有内存与批量路径：[LLVM 发射](../../compiler/codegen/src/emission/prepare.rs)、[`.slib` 发布](../../compiler/slib/src/publish/cross_cone/atomic.rs)、[链接归档](../../compiler/linker/src/link/archive.rs)、[归档格式](../../compiler/linker/src/native_archive.rs)。
- 发行准备：[runtime 编译](../../compiler/scoop/src/runtime_build/compile.rs)、[Windows 打包](../../scripts/package_windows.py)。

## 实施项与验收

### W0：Windows 基线与文件事件归因

- [x] 使用 release 工具，在同一 Windows 主机保存改动前 CPU 采样；覆盖 `scoop`、`scoopc`、`scoop-pm` 及 C 编译器/链接器子进程。
- [x] 用 ProcMon 或 WPR/ETW 记录文件事件，按路径和阶段汇总新文件/目录创建、打开、截断、写入、rename、删除及子进程启动次数。Windows 的 `CreateFile` 也可能只是打开已有文件，不能全部计为新建。
- [x] 分别运行预编译系统库的应用冷构建、无变化热构建、修改一个 Cone 后构建、改源码后的 `scoop check`。另测有实际 native 源码/桥接需求的应用。
- [x] 将从源码构建 `scoop.ui`、准备 sysroot、打包发行版列为维护者场景，独立统计，不与普通 GUI 应用消费预编译库混在一起。

验收：得到可归因的阶段耗时、文件/目录创建数、写入调用数、进程数和内存占用。优先级可据实际占比调整，但不以“发现 RtlFreeHeap 是否为热点”推迟 W3。

### W1：删除工具与 SDK 巡检及其持久记录

- [x] 删除 `native_record` 中为监控工具文件、版本、SDK/环境状态服务的记录与失效计算，包括命中前读整份 JSON、逐输入 `metadata/canonicalize` 和重复读取记录。
- [x] 删除配套 `scoopc` 的工具文件状态/能力持久缓存及其额外输入指纹。正常执行时仍定位并调用配套工具，沿现有协议处理实际执行错误。
- [x] 从 Cone、native 对象及最终链接的更新判断中删除这些冗余工具输入。核查 `resolve_request_cached` 在构建图/缓存判断之前的调用，真正需要生产时再准备调用配置。
- [x] 保留 `Cone.toml` 的有效 native 配置、实际源码/头文件与 `.slib` 依赖更新；区分正常编译期输入记录与被退役的工具状态记录。
- [x] 同步发行版升级后清理项目缓存的说明，删除旧的工具变化自动失效承诺、专用记录和对应测试；不为旧记录保留迁移层。

原 `native_record` 与配套编译器持久记录已退役；实际调用配置见 [native_invocation](../../compiler/toolchain/src/native_invocation.rs)，真实输入更新数据见 [build_record/inputs](../../compiler/toolchain/src/build_record/inputs.rs)。其余入口：[编译器解析](../../compiler/scoop/src/compiler.rs)、[build 入口](../../compiler/scoop/src/cli/build.rs)、[Cone key](../../compiler/scoop/src/snapshot/prepared/model/cache_key.rs)、[native key](../../compiler/toolchain/src/native/objects/inputs.rs)。

验收：真实输入存在、结果可用的热构建不打开工具巡检 JSON、不扫描工具/SDK、不启动编译或链接；源码、相关清单配置、实际头文件或依赖更新仍使受影响结果重建。失败不发布成功结果。

### W2：删除 `scoop check` 整次成功缓存

- [x] 删除 `checks-v4` 命中分支、成功记录读写、整次检查 key/输入收集及专用锁。
- [x] 每次调用执行根 Cone 的 Parser/HIR 和必要 generator，输出当前诊断；清理只为检查缓存存在的 `cache_hit` 状态和展示。
- [x] 继续正常复用依赖 `.slib` 和源码依赖的普通构建缓存，保留根不发射机器码、不准备 runtime、不最终链接的职责。
- [x] 改写检查缓存相关 spec/设计/测试。用真实源码修改、错误修复、warning 更新及依赖复用验证命令，不把“连续两次未改代码的检查命中”作为性能目标。

入口：[check 编排](../../compiler/scoop/src/check.rs)、`check/cache.rs`（已删除）、[源码选择](../../compiler/scoop/src/check/plan.rs)、[检查调用](../../compiler/scoop/src/check/compile.rs)。

验收：每次检查产生本次前端结果；没有独立检查成功记录、锁和旧诊断回放。根不产生 `.slib`、对象或 executable，依赖仅在实际更新时构建。

本地实现、Linux/Windows 集中复验已完成，范围与结果见实施记录。

### W3：Windows 宿主编译器更换 malloc

- [x] 在 Windows `scoopc` 的 Rust 分配路径接入成熟替代分配器 `mimalloc`；必要时将同样存在分配热点的宿主 CLI 一并接入，不自行实现分配器。
- [x] 实施前核对维护状态、许可证、Windows MSVC/GNU 构建支持和当前依赖版本。2026-11-10 查询 [mimalloc crate](https://crates.io/crates/mimalloc) 得到版本 `0.1.52`、更新日期 2026-05-22；依赖与本地验证情况见实施记录，Windows 实际编译和测量已完成。
- [x] 以相同源码、优化配置和 jobs 对比默认堆与替代分配器，记录 scoopc 与命令整体耗时、RSS，并以 CPU 采样定位前端、产物组装和分配热点；不把多线程样本数当作分阶段墙钟时间。
- [x] 区分 Rust 全局分配器和动态 LLVM/C++ 库的分配。`#[global_allocator]` 不会自动替换后者；采样确认还有 LLVM 堆热点时再确定对应构建改动，保持跨库分配与释放匹配。

验收：Windows 实际编译器使用替代分配器并完成有代表性的编译、链接和运行检查，报告收益及内存变化。范围是宿主工具的内存分配，不改 Scoop 程序的 GC/对象语义。

### W4：生成 C 桥的内存交接与批量编译

- [x] 外部编译器生成对象后只读入一次，在内存修补精确 atom 边界并直接交给 production，删除“读对象 → 修补写回 → driver 再读对象”的路径。
- [x] 没有桥接源码时不创建 generated-C 临时目录。
- [x] 在相同 target/编译参数下，将可组合的多个生成桥接实现放入少量 C 翻译单元，减少 `.c/.o` 文件和编译器进程。逻辑桥接身份继续独立，不按物理文件重新定义身份。
- [x] 沿现有可表达多 unit 的对象成员模型调整消费方，避免新增平行分组框架；处理生成代码中的重复声明、辅助符号及调试源码定位。
- [x] 保持桥接签名、atom 边界、ODR 选择、按需裁剪、native ABI、回调和 GC 契约。用实际桥接需求覆盖 Windows MSVC/MinGW，必要的对象格式验证覆盖 Mach-O/ELF/COFF。

实现及 Mach-O／ELF／COFF 真对象验证已完成；Windows MSVC／MinGW 三目标均完成 Debug／Release 实际编译、链接和运行。

验收：相同桥接集合的临时文件数和 C 编译进程数实际减少；对象修补不再写回磁盘。结果正确且未用桥接代码仍可裁剪。不能仅用“序列化写入变快”代替“文件创建数减少”的验收。

### W5：原生源码中转与 scratch 生命周期

- [x] 复用当前 Cone 的工作目录，删除每个源码独立创建目录的开销；父调度、child、driver 的空目录按实际需要延迟创建。
- [x] 对实际支持的工具，通过 stdin 传递已在内存中的预处理文本，消除 `.i/.ii/.mi` 的中转。对必须使用路径的工具保留最少的必要文件，在现有工具分支中处理差异。
- [x] 核查是否可在一次正常编译中同时取得对象与实际头文件依赖，减少独立预处理进程；落实前核对宏处理、语言选项、源码映射和诊断行为。
- [x] 原生对象首次读入后复用字节，减少重复读写；保留正常增量更新及失败清理。用户 C/C++/ObjC 翻译单元不能直接拼接，避免改变宏作用域、静态符号和初始化行为。

本地实现与定向验证完成。保留独立预处理以取得实际展开的 include 名称及搜索候选；普通编译 depfile 不能完整替代这些更新输入。GCC／Clang／clang-cl 使用 stdin，必须使用路径的 cl 复用工作目录内的中转路径。Windows 实机行为和文件计数已完成集中复验，结果见实施记录。

验收：有多个真实 native 源文件的 Cone 不再按源码数量创建可避免的目录/中转文件；源码、实际包含的头文件和清单配置变化后正确重建。热命中不重新预处理或复制输入。

### W6：减少依赖重复解码与身份图扫描

- [x] 核查 `read_program_link_closure` 的传递依赖展开：已有依赖图本身包含上游信息时，不再把每份传递图反复导入并遍历相同节点。
- [x] 同一消费进程按实际需要解码 section 一次，复用已有身份图、布局与摘要；缩减原 `validate_foundation_identity_graph_with_authorities` 重复工作的实际部分。
- [x] 梳理 `scoopc` 与 Link 的责任：Link 只检查本次选择、实际引用和最终组合所必需的事实，不为前端已经完成的事实重建完整语义验证。进程间不为复用验证结果新增凭证/收据机制。
- [x] 保留独立消费 `.slib` 的正常格式、引用、目标/ABI 与所选定义冲突检查，用多个共享上游的实际依赖图验证结果与扫描次数。

本地实现及定向验证完成。Link 保留实际 ABI 指纹、身份种类／引用、定义归属和最终组合检查；不重哈希普通身份 key 或重放整份 LIR 生产关系。现有按需文件入口已经共享解压后的 section，本次沿用，不另设 reader 缓存。Linux／Windows 集中复验已完成。

入口：[program link reader](../../compiler/slib/src/program_link/read.rs)、[Compile 解码](../../compiler/slib/src/compile_decode.rs)、[Link 机器记录解码](../../compiler/slib/src/link_decode/layout/machine.rs)、[外部身份图导入](../../compiler/identity/src/validation/external.rs)。

验收：共享依赖增多时不重复完整扫描同一图，实际链接语义保持。当前 Link 读取部分 HIR/MIR 身份和 LIR 机器记录，不能描述成“重新运行全部前端”；按具体重复点验收。

### W7：剩余记录的必要性、格式和缓冲

原调研列出的六处直接 JSON 文件写入，按用途分别处理：

| 记录 | 处理 |
| --- | --- |
| 原 `toolchain/src/native_record.rs` | W1 已删除工具/SDK 巡检记录 |
| 原 `scoop/src/compiler/record.rs` | W1 已删除配套工具状态记录 |
| `scoop/src/check/cache.rs` | 随 W2 删除整个检查成功结果缓存 |
| `linker/src/link/archive.rs` | 已生成 archive 的小型路径索引有复用用途；精简必要字段并缓冲写入 |
| `scoop/src/cache/single_file.rs` | 依赖头部在构建 key 之前使用，保留以避免热命中打开源码；改为每个真实源路径一份记录和锁，更新时替换，不随每次源码变化增加文件 |
| `scoop/src/document/overview.rs` | 删除 README 解析旁路记录，保留最终文档结果的正常缓存；文档命令独立计量 |

- [x] 对确有必要保留的记录使用 `BufWriter` 或一次 `to_vec` 后 `write_all`，发布前检查 flush/写入错误；沿用现有原子发布机制。
- [x] 清理 Windows 路径的 JSON 数字数组膨胀。现有 CBOR 路径继续使用字节串；JSON 路径保留编码标签并改用标准 Base64，同步发送方、接收方和准备脚本，不用 `to_string_lossy`。
- [x] 复查工具调用参数和 PM 消息等剩余路径载体，删除单文件头部 key 中不必要的 JSON 套编码；跨 crate 的 native 配置和 PM 数据继续使用现有协议，不把“清理记录”扩大成所有 JSON 接口必须改格式。
- [x] 不改造已经使用 `BufWriter` 的 native/runtime/最终结果记录，也不把现有 CBOR 编译记录重复列为 JSON 热点。

证据：[路径载体](../../compiler/protocol/src/path.rs) 在 Windows 保存 WTF-16LE 字节，JSON 派生为整数数组，而现有 wire 编码已经支持 bytes。本轮独立 `Write` 计数实验中，262,144 字节的此类数组生成 786,433 字节 JSON；直接写入调用 `Write` 524,289 次，8 KiB 缓冲后为 97 次。这说明小写入放大成立，不代表这些调用的 Windows 实测时长。原调研的 794 KB 记录和具体微秒数据未在本轮重采样。

验收：退役记录确实消失；保留记录的写调用按缓冲块增长，路径保持无损。缓冲只能降低写入次数，不能代替 W4/W5/W8 对文件创建数量的优化。

### W8：PM prepare 使用 stdin/stdout

- [x] 在既有 `scoop-pm prepare` 一次请求/响应流程中，使用 stdin 传请求、stdout 返回成功数据，stderr 保留诊断。
- [x] 删除 `pm-requests` 下的 request/response 临时文件与目录，同步 PM、调用方和包管理规范；不新增传输框架或旧协议兼容层。
- [x] 使用现有子进程管理处理读写、退出码、中断和错误，避免大请求/响应导致管道死锁；失败不能被当作部分成功。

验收：manifest 构建热路径没有 prepare IPC 文件创建，依赖解析/锁定语义及错误诊断保持。此任务不删除 PM 的正常包源码缓存和解析复用。

### W9：剩余前端热点与发行准备

- [x] 完成上述简化后再采样 Parser/HIR/MIR/LIR 的分配、复制和遍历，按残余热点选择局部优化；单 Cone 前端并行化另行评估，不作为本任务必须完成的前置改造。
- [x] 文档准确区分并行层次：单 Cone 前端当前串行；`scoopc --jobs` 主要作用于 LLVM 工作，上层 `scoop` 还可以并发构建多个 Cone。
- [x] 分别统计 sysroot/runtime 准备、MinGW 头文件复制、第三方源码获取/解包和发行打包的文件量。复用已准备输入，检查是否仍有逐 Cone/逐目标重复复制；已有公共头共享继续沿用。
- [x] API 文档页面、测试日志、显式 IR dump 按各自命令计数，不把这些必要输出归入普通 build 的小文件开销。

验收：剩余问题有实际阶段、输入和测量依据；不把发行准备时的大量文件误判为每次普通应用编译都会产生，也不引入缓存数据库或通用资源计费框架。

### W10：逐 Cone 编译进度

- [x] 在实际调用 `scoopc` 时显示类似 Cargo 的 `Compiling <Cone>`，结束时显示 `Finished` 或 `Failed` 及耗时。
- [x] 常规终端信息写 stderr；结构化 JSON 模式沿其记录合同输出。并发编译时每条信息完整成行，缓存复用不伪装成一次编译。
- [x] 覆盖多个 Cone、编译失败及耗时展示；保留原始诊断和进程退出语义。三平台相关 CLI 验证已完成。

### W11：提交、跨平台验证与发行交付

- [x] 在独立分支实施，每个大项先完成本地验证，再提交并推送，随后获取并合并 `origin/main`，再继续下一项。
- [x] Linux 使用 `nuc12` 的新工作目录，Windows 使用 E: 盘的新工作目录；复用已准备的依赖，按大项集中验证，不逐小改动重复全量测试。
- [ ] 每轮清理后续不再需要的临时文件，保留后续仍需使用的构建缓存、测试资源和基线。
- [ ] 全部改动完成后，在 macOS、Linux、Windows 分别打包，并用安装后的工具验证编译、链接和运行；修复实际遇到的打包脚本问题，记录各平台产物位置。

## 规范与验证安排

| 工作 | 实施前需同步的规范/设计 |
| --- | --- |
| W1/W7 | [实现规范](../specs/SCOOP-IMPL-SPEC.md)的工具发现、缓存更新、编译/链接职责及记录条款；单文件头部记录和文档记录若退役，同步其对应条款 |
| W2 | 实现规范 §2.7 的检查成功记录；[fmt/check 设计](../fmt-check/DESIGN.md)的缓存、状态与验收；路线图中的既有缓存描述 |
| W4/W5/W6 | 实现规范的生成 C 对象分组、产物生产、按需读取与 Link 边界；保持[语言规范](../specs/SCOOP-SPEC.md)与[运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)中的 FFI/GC/ABI 契约一致 |
| W8 | [包管理规范](../specs/SCOOP-PACKAGE-SPEC.md)的 prepare 传输合同及实现规范 §2.7 的调用方职责 |
| W3/W9 | 记录实际实现与测量；若仅改变宿主分配器和内部临时文件，无需修改语言或 runtime ABI |

- 代码变更先格式化、lint，再运行与改动相称的 Rust、正式 CLI 和必要 ABI/GC 定向测试。Python 改动使用仓库指定 Ruff 版本；GC stress 只用于相关契约验证。
- 缓存验收保持真实源码、工具和原产物存在，以正常调用、输出及实际输入变化判断复用；不增加删除/隐藏输入、破坏缓存后的恢复测试。
- A/B 保持同一主机、源码、工具构建模式、目标、profile 和 jobs；使用多次交替运行的中位数，并列记录文件/进程计数和 RSS。Windows 覆盖实际使用的 MSVC/MinGW 环境，跨平台合同变更补足对应验证。
- 原始 trace、采样、逐次 JSON 报告留在仓库外；本目录只补充完成项、结果概要、失败闭合和实际限制。不得将静态审计、合成写入计数或用户历史观察写成优化后的 Windows 实测收益。
