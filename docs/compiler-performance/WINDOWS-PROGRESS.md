# Windows 编译性能实施记录

工作分支：`codex/windows-compile-performance`。起点：`f27eda5ee1`。工作范围与验收条件见[任务单](WINDOWS-TASKS.md)。

每个大项先完成本地实现和定向验证，再提交、推送并合并 `origin/main`。Linux/Windows 按大项集中复验，全部完成后分别制作安装包；不逐小改动重复全量测试。

## 当前状态

| 项目 | 状态 |
| --- | --- |
| W0 Windows 基线 | 已取得 UI 编译的进程时间及内存对照，文件事件归因与普通应用基线进行中 |
| W2 检查成功缓存 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W10 编译进度 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W3 Windows 分配器 | Windows UI 默认堆／mimalloc 对比完成，普通应用验证进行中 |
| W1 工具巡检 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W4～W9 | 待实施 |
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
