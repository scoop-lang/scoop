# Windows 编译性能实施记录

工作分支：`codex/windows-compile-performance`。起点：`f27eda5ee1`。工作范围与验收条件见[任务单](WINDOWS-TASKS.md)。

每个大项先完成本地实现和定向验证，再提交、推送并合并 `origin/main`。Linux/Windows 按大项集中复验，全部完成后分别制作安装包；不逐小改动重复全量测试。

## 当前状态

| 项目 | 状态 |
| --- | --- |
| W0 Windows 基线 | UI／普通应用计时、普通构建文件事件归因与编译器热点符号化完成 |
| W2 检查成功缓存 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W10 编译进度 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W3 Windows 分配器 | Windows UI 对比及普通应用编译／链接／运行验证完成 |
| W1 工具巡检 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W4 生成 C 桥 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W5 原生输入与临时目录 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W6 依赖读取与 Link | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W7 剩余记录 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W8 PM 管道通信 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W9 剩余热点与发行文件量 | 待累计优化复测 |
| W11 三平台安装包 | 待全部实现和验证完成后执行 |

Linux 工作目录为 `~/repos/scoop-windows-performance-20261110`，Windows 为 `E:/scoop-windows-performance-20261110`；均从同一基线建立，未改动已有 checkout。原始报告和日志保存在各自主机的仓库外工作目录。

## 2026-11-10：每次检查根 Cone

提交 `9f5ef42212` 已推送，并合并当时最新 `origin/main`（已是最新）。

删除独立的 `checks-v4` 记录、专用锁、整图输入 key、结果序列化与 `cache_hit` 状态。每次调用直接完成当前根的前端检查；依赖继续按普通构建缓存复用。完成节点不再为检查缓存额外携带 generator 输入副本，检查也不再收集 native 工具状态。同步实现规范、fmt/check 设计、CLI 展示和正式用例。

用例通过实际 child 调用验证每次根检查、源码依赖复用、真实头文件更新、Production/Test 选择、generator 重执行、并发请求、当前 warning 及错误修复。无平台依赖的基础、依赖和测试用途用例开放到 Windows；依赖 POSIX shebang 的 generator 用例保留平台条件。

macOS AArch64 本地结果：

- `cargo fmt --all`、workspace all-targets Clippy 和 Ruff 0.16.10 通过；Clippy 仍报告既有代码在当前 Rust 版本下的警告。
- release 配套 CLI 构建通过；`scoop` CLI 的 40 个单元测试通过。
- fixture runner 规则测试运行 84 项，4 项平台条件跳过，其余通过。
- 12 个检查命令用例通过；另一个缓存/clean 组合用例在更新测试 runtime 后通过，合计 13 个正式用例完成验证。

首次组合用例的链接失败来自已有开发 sysroot 的旧 runtime 索引（10 字段，当前 reader 要求 11 字段）。使用现有准备脚本，仅在本任务的独立 sysroot 更新四种 runtime 配置和头文件，复用已有系统 `.slib`，随后只重跑该失败用例。没有通过修改编译器兼容旧索引或放宽断言处理此环境问题。

本轮结束后清理 fixture 工作目录及不再需要的测试缓存；报告另存仓库外，配套工具、Cargo 缓存和已准备的测试 sysroot 留供后续大项复用。跨平台结果与性能数据尚未取得，不将本地通过写成三平台验收完成。

## 2026-11-10：逐 Cone 编译进度

提交 `0879480068` 已推送，并合并当时最新 `origin/main`（已是最新）。

human 模式在真实 `scoopc` 调用前输出 `Compiling`、`Checking` 或 `Documenting` 及 Cone 坐标；调用结束输出 `Finished` 或 `Failed` 和两位小数的秒数。使用单调时钟计时，每行一次加锁写入 stderr。缓存命中没有编译起止信息，JSON 模式保持原格式，原始诊断与最终命令摘要保留。

macOS AArch64 上 workspace Clippy、Rust 格式化、Ruff 0.16.10 和 release CLI 构建通过。12 个相关正式用例完成验证：新增用例使用两个可并发依赖和一个根，覆盖起止配对、耗时格式、缓存复用、JSON/human 切换、检查、文档生成以及真实编译失败；已有用例覆盖生成器日志、warning、错误诊断和命中后的输出。新增用例的两处既有摘要格式期望修正后，仅重跑该用例通过，没有扩大测试范围。

日志和报告保留在仓库外；本轮 fixture 工作目录及专用缓存清理，跨平台复验留待集中进行。

## 2026-11-10：Windows scoopc 分配器

仅在 Windows `scoopc` 可执行程序启用 `#[global_allocator]`，使用 `mimalloc 0.1.52`，锁定的底层库为 `libmimalloc-sys 0.1.49`（默认 mimalloc v3.3.2）。依赖采用 MIT 许可；已检查上游 C 构建脚本的 MSVC C++17 路径、GNU C 路径及 Windows 系统库链接。不开启 malloc 符号覆盖，不接管动态 LLVM 的 C++ 分配，不影响 Scoop runtime 的 GC。

macOS 上格式化、workspace all-targets Clippy 和 release `scoopc` 构建通过；该平台不编译或启用 mimalloc。Windows 实际构建通过。以进度输出提交 `0879480068` 为默认堆基线，`20b8dc1e52` 为 mimalloc 版本，固定 Rust／LLVM 22.1.8、release、jobs=4、MSVC UCRT 目标和 UI 源码，消费相同预编译系统依赖。前三次默认堆与三次 mimalloc 分组运行后，再按 mimalloc、默认堆的反向顺序各跑一次；不启用 WPR，避免事件跟踪改变计时。

| 指标 | 默认堆 | mimalloc |
| --- | --- | --- |
| 前三次 scoopc 耗时 | 157.24／158.42／158.47 秒 | 141.61／141.89／144.30 秒 |
| 前三次耗时中位数 | 158.42 秒 | 141.89 秒，减少约 10.4% |
| 前三次进程峰值工作集的中位数 | 8,512,135,168 字节 | 8,353,726,464 字节，减少约 1.9% |
| 反向顺序复验 | 158.43 秒 | 142.52 秒 |

内存来自每 50 ms 观察子进程得到的 Windows peak working set；进程数和 CPU 时间属于采样观察，不代替完整 ETW 计数。以上是从源码构建 `scoop.ui` 的维护者场景，不代表普通应用消费预编译 UI 库的耗时。最初遇到旧 graphics `.slib` 与当前源码接口不匹配，以及普通应用遇到旧 runtime 索引；均在任务专用 sysroot 配齐资源，失败场景未混入成功的速度对照。临时测量脚本、原始日志和采样结果保存在仓库外。

## 2026-11-10：删除工具巡检，先判断结果更新

删除 `native-tools`、配套 `scoopc`、C/C++ driver 及最终链接工具的持久巡检记录、专用锁和更新键。调用配置仍经既有机器请求传给 child，但只保存本次编译实际需要的参数；compiler protocol 为 29。构建图先选择静态目标，在 native 对象或 Cone 确需生产时才解析 C 工具，并在该图中复用；编译器定位不查询能力记录或文件指纹。

Cone key、native 对象、runtime 和最终程序的更新判断移除工具文件、版本、PATH／SDK 状态与调试工具输入。相关有效清单配置（含 C++ runtime 需求）、实际源码、头文件与依赖产物仍参与更新。最终程序命中前不准备 C 工具、runtime 或 Link 闭包，所需 runtime 索引位置直接来自已有结果记录。删除只为巡检保留的头文件 depfile 和工具候选枚举，保留实际编译／链接所需的目标、格式与 ABI 检查。

同步实现规范、README 和安装说明：升级发行版后执行 `scoop clean`，并清理显式放在项目 target 外的缓存。退役工具变化自动失效用例；保留真实输入存在的冷／热构建与实际链接器调用观察。

macOS AArch64 验证：

- 格式化、Clippy 和相关 Python 的 Ruff 检查通过；release 配套 CLI 构建成功。
- `scoop` 71 项、协议 47 项、工具链 64 项 Rust 测试通过。工具链用例覆盖真实 native 头文件与 include 候选变化、目标隔离、编译失败不发布、C/C++ 混合与 native 命中不再解析工具。
- 14 个正式 CLI 用例通过，合计 146 次外部进程操作。覆盖纯 Scoop／原生 C／C++、依赖菱形图、源文件更新、实际 native 库更新、最终程序复用、调试产物、两种显式链接入口、检查、逐 Cone 进度和无工具巡检记录。
- fixture runner 规则测试通过（84 项，其中 4 项按平台跳过）。

首次 Rust 验证中，Windows 链接 trace 用例在 macOS 混用了 `/var` 与 `/private/var` 路径；将测试根目录统一为同一路径后复验通过，没有给产品路径增加额外扫描或审计。原始日志和 CLI 报告保存在仓库外，完成后清理本轮 fixture 工作目录及专用缓存。


## 2026-11-10：C 桥批量编译与内存交接

一个 Cone 的逻辑桥实现共享类型声明和布局断言，按稳定的桥身份顺序写入一个 C 翻译单元；静态回调辅助名称带桥身份，生成诊断使用相应身份的 `#line`。复用既有多 unit 成员模型，不改变桥、native ABI、GC 或 `.slib` 成员格式。空桥集合不创建临时目录。

外部 C 编译器只调用一次，对象只读入一次，内存补全边界后直接转移给产物组装；返回时释放临时源码和原始对象。Mach-O 主定义按同 section 的下一个定义确定范围，展开信息按实际 compact-unwind／FDE 记录划分；ELF 与 COFF 启用独立函数和数据 section，并保留原生展开表、重定位及 COMDAT 关联。公共 CIE 前缀归入相邻 FDE 的范围，不把完整共享 section 重复归给每个桥。范围与重定位索引在一次对象处理中复用。

macOS AArch64 验证：格式化、workspace all-targets Clippy、release 配套 CLI 构建通过；19 项 C 布局／桥对象测试、errno 与整数 C 拼写各 1 项、目标数据 11 项、工具链 64 项通过。实际两个桥的用例仅调用一次 C 编译器、产生两份主要文件，检查全部边界不重叠、失败清理及空集合不建目录。8 个正式 CLI 用例全部通过（10 个变体、45 次进程操作），覆盖 packed C ABI、回调与 TLS、回调异常、errno、标量／聚合返回、ObjC 和 GC；两份专门链接快照保持一致。首次 CLI 运行未传开发 LLVM 前缀而未找到 LLD，补齐测试环境后复验通过。

Windows 普通应用另已完成默认堆与 mimalloc 各一次功能和计时观察，消费相同预编译系统库与重新准备的当前 runtime：

| 命令 | 默认堆整条命令 | mimalloc 整条命令 |
| --- | --- | --- |
| 应用冷构建 | 7.758 秒 | 7.406 秒 |
| 无变化构建 | 0.165 秒 | 0.092 秒 |
| 修改源码后构建 | 4.793 秒 | 4.424 秒 |
| 修改源码后 check | 1.110 秒 | 0.971 秒 |
| native C Cone 冷构建 | 5.257 秒 | 5.129 秒 |

这些单次观察验证实际编译、链接和运行；热构建不调用 `scoopc`，其波动不能归因于更换分配器。稳定的分配器收益采用上文多次 UI 编译对照。原始测量、文件事件跟踪及测试报告均保存在仓库外；本轮 CLI 工作目录和专用缓存已清理，三平台对象格式及安装验证随后集中完成。


## 2026-11-10：原生输入与临时目录

原生对象编译复用当前 Cone 的对象工作目录和一个 `unit.d`。GCC、Clang 与 clang-cl 通过 stdin 读取内存中的预处理文本，删除逐源码 `.i/.ii/.mi` 和临时目录；必须从路径读取的 cl 复用 `unit.i/unit.ii`，成功发布前删除中转文件。用户翻译单元仍分别编译，保持宏作用域、静态符号和初始化语义。原生对象进入产物组装的路径原本已经只读一次并转移字节，本次沿用。

保留独立预处理：已有输入更新规则需要 `-dI` 提供实际展开的 include 名称及对应搜索候选，普通编译 depfile 只能给出已包含路径。并发写 stdin、读 stdout/stderr 避免大输入或诊断堵塞；子进程提前失败时保留真实状态和诊断。删除只供测试比较的预处理 SHA，测试直接比较实际预处理结果。

父调度的 staging 按首次实际产物／显式 dump 请求创建，Cone 产物直接放在同一个 staging 根；child 复用传入的临时根，driver 不再无条件创建工作目录。纯检查和无变化构建避免这些空目录，真实对象编译、显式观察及原子发布仍拥有所需空间。

macOS AArch64 验证：格式化、workspace all-targets Clippy、release 五个 CLI 构建通过；进程管道 3 项、`scoop` 71 项、工具链 64 项 Rust 测试通过。8 个正式 CLI 用例通过（9 个变体、94 次进程操作），覆盖 C／C++／ObjC、真实头文件与搜索候选更新、菱形依赖、逐 Cone 进度、检查、阶段观察和最终程序复用。用本机 LLVM 22 的 Microsoft 驱动模式另验证了 C/C++ stdin 参数能生成 COFF；这项参数验证不等同于 Windows 本机验收。报告保留仓库外，测试工作目录与专用缓存已清理。

## 2026-11-10：Windows 普通构建文件事件基线

以默认堆提交 `0879480068` 为基线，使用配套系统库和当前 runtime 完成应用冷构建、热构建、源码修改、检查与 native C Cone 冷构建。WPR 同时采集 CPU 与 FileIO，ETW 丢失事件为 0；按每次 `scoop.exe` 及其子进程归因，排除测量 Python、测试程序和 WPR 自身。创建数来自完成事件区分的新文件／目录，不能将全部 CreateFile 请求视作新建。以下为事件计数，跟踪时的耗时不混入无跟踪性能对照。

| 场景 | 进程数（含 scoop） | 新建文件 | 新建目录 | 写入事件 | rename 成功 | 删除成功 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 应用冷构建 | 13 | 35 | 24 | 612,694 | 19 | 28 |
| 无变化构建 | 1 | 0 | 3 | 13 | 0 | 3 |
| 修改源码后构建 | 5 | 17 | 11 | 625 | 11 | 15 |
| 修改源码后 check | 2 | 2 | 4 | 130 | 1 | 4 |
| native C Cone 冷构建 | 10 | 30 | 23 | 612,094 | 17 | 20 |

应用冷构建的一份 `native-tools-v6` 临时记录为 883,761 字节，产生 610,904 次写入；native 冷构建对应记录为 883,734 字节、610,893 次写入。两份工具记录解释了这两个场景超过 99% 的写入次数，W1 已将其删除。配套编译器记录各有 1,135 次写入，单文件头部记录每次 105 次。无变化构建仍创建并删除 3 个目录，W5 已将相关目录改为按需创建。全部写入均有路径归因，记录中的请求也均有完成事件；失败打开与覆盖事件在外部报告单列，未并入新建数。

原始 ETL 与解析报告留在仓库外。另一次早期 UI 跟踪因旧 graphics 依赖与源码不匹配而失败，仅保留用于定位失败前的文件事件，不能代表成功 UI 编译；UI 稳定耗时采用上文无跟踪、多次成功的对照。优化后的事件对照与剩余 CPU 热点随后集中复测。

## 2026-11-10：共享依赖读取与 Link 职责

Link 按直接依赖引入已有身份图，删除逐 Cone 重新展开全部传递位置的路径。每份图共享自己的声明索引和上游来源；合并菱形图时用仍持有的共享存储识别相同来源，每份来源的记录只访问一次。索引直接复用已有声明表，不再保存完整节点副本。本层 typed 记录查询在已排序的声明表中定位相应种类，不为每一种记录重扫全部依赖候选；完整闭包查询仍保留实际需要的合并结果。

Compile 与 Link 继续使用同一个 section 解码和引用解析实现。Compile 保留语义 key 校验，Link 读取产物声明的身份摘要，不重哈希普通 CBOR／callable 身份或重放源 native 声明的编译工作。Link 保留实际 C ABI 指纹、重复声明、身份种类、引用闭合、循环引用和所选定义冲突检查。LIR 解码共用一次记录还原，Link 不再重放整份桥接、native contract 与 safepoint 生产关系；实际 GC 映射在投影时检查缺项和重复，最终仍按所选物理 callable 检查完整 site 与短 ID 组合。没有增加跨进程凭证或持久验证记录。

macOS AArch64 验证：格式化、workspace all-targets Clippy 和 release 配套 CLI 构建通过；身份 363 项、LIR 380 项、`.slib` 267 项、链接 safepoint 2 项 Rust 测试通过。新增菱形图测试统计共享来源的实际访问次数，覆盖重复引入、局部声明与闭包查询；Link 测试保留未知身份种类／引用报错，并检查缺失 GC 映射返回诊断而非 panic。9 个正式 CLI 用例通过（12 个变体、67 次进程操作、1 个链接快照），覆盖菱形依赖、ODR 选择和冲突、条件接口显式重链接、C ABI、回调异常、聚合返回、ObjC 与 GC。共享索引调整后重跑身份测试和实际菱形构建通过。

首次 `.slib` Rust 验证发现基线的两处固定向量仍期望 HIR interface v92，而现有可选 Context 查询已经将规范和实现升至 v93。同步了版本用例、描述符字节和相应指纹，没有改动产物格式或添加兼容路径。报告保存在仓库外，本轮 fixture 工作目录与专用缓存清理；Linux／Windows 随实现批次集中复验。

## 2026-11-10：剩余记录与无损路径编码

删除 README 解析的独立 JSON 记录与目录，最终文档缓存继续按真实输入更新。单文件依赖头部需要在构建 key 之前取得，保留记录以避免热命中打开源码；改为每个真实源路径复用一份记录和锁，mtime／长度放在记录内，源码更新不再累积新记录。key 直接散列路径字节，去掉中间 JSON 编码。单文件记录和链接 archive 路径索引使用缓冲写入，显式检查 flush 后再原子发布；原本已缓冲的 native／runtime／最终结果记录沿用。

JSON HostPath 保留平台编码标签，将数字数组改成标准 Base64 字符串；Unix 非 UTF-8 字节和 Windows WTF-16LE 未配对代理项均可无损表达。已有 Wire CBOR 仍使用字节串。同步协议版本、PM／driver 数据和 sysroot 准备脚本，不增加旧数组兼容层。工具调用配置继续使用现有跨 crate 协议，未为统一格式新增一套持久记录。

macOS AArch64 验证：workspace 格式化和 all-targets Clippy、Python Ruff 通过；scoop 71 项、协议 47 项、PM 38 项、toolchain 64 项 Rust 测试通过。8 个正式 CLI 用例全部通过（88 次进程操作），覆盖单文件依赖、源码包、workspace build／check／doc、README 更新、native C、最终链接复用和逐 Cone 进度。准备脚本 6 项测试通过，并实际运行 runtime 辅助程序，将其 JSON 返回的两个产物路径交给 Python 解码并确认对应文件存在。

独立 runtime-builder 工程漏用了已删除的工具巡检入口，已改为直接准备工具；补齐其 Base64 锁文件，单独完成格式化、Clippy 和 locked release 构建。原始报告保存在仓库外，fixture 与一次性 runtime 验证目录已清理；Windows 写入计数与跨平台验证随完整实现批次执行。

## 2026-11-10：PM prepare 管道通信

`scoop-pm prepare` 从 stdin 读一个请求至 EOF，完成全部所选根后将完整 JSON 响应写入 stdout 并检查 flush；失败诊断继续走 stderr。driver 使用现有 `scoop_output_with_input` 并发发送请求、读取两条输出管道，先检查退出状态再消费响应。删除 `--request`／`--response` 文件接口及 `pm-requests` 临时目录，不改变正常包源码缓存、解析复用与锁定行为。

冷构建验证发现构建入口提前 canonicalize 尚不存在的缓存目录，过去依赖 PM 写临时文件的副作用才能通过。缓存路径本来已在输入处保证绝对，因此删除这次不必要的访问，让实际写入步骤负责创建目录；没有恢复额外的预创建阶段。

macOS AArch64 验证：workspace 格式化和 all-targets Clippy、配套 release 构建通过；调整后的 PM／scoop 定向 Clippy 通过。PM 19 项真实进程测试及进程管道 3 项测试通过，覆盖真实请求、错误位置、锁定与配置更新、大输入和双向输出、提前失败。7 个正式 CLI 用例通过，覆盖 workspace build／check／doc、源码 registry、local tools、错误请求、逐 Cone 进度和最终链接复用；冷／热 workspace 构建均断言没有 `pm-requests` 目录，失败请求 stdout 为空。首次冷构建失败已按上述原因闭合，定向复验通过。报告保存在仓库外，fixture 工作与缓存目录已清理。
