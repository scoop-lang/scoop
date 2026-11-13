# Profiling / PGO 实施记录

状态：实施中。开始日期：2026-11-13。设计见 [DESIGN.md](DESIGN.md)。

实现分支：`codex/profiling-pgo`。每个完整变更在本地完成针对性验证后提交、推送，再合并最新 `origin/main`；跨平台验证按完整大项安排，不逐文件重复全量测试。原始日志和临时训练产物保存在仓库外，完成一轮后清理不再使用的文件。

## 交付范围

| 大项 | 当前状态 | 验收依据 |
| --- | --- | --- |
| 固定 MIR 采集位置、LLVM InstrProf 与 ODR 计数 | 逻辑位置及 MIR/LIR/LLVM 通道已实现；CLI 与真实训练验收随 profiling 批次接入 | 确定计数、内联映射、多线程、对象选择/裁剪 |
| off/default/full IR template 导出 | 已实现，三平台代表目标验收完成 | 完整模板、语义必需模板保留、实际 consumer 与 Strong 归属 |
| profiling profile 与 runtime 采集 | 待实现 | 控制流/调用计数、运行指标、退出写出、release 无探针 |
| MIR cost 反馈及 `scoop-pgo` | 待实现 | instrument → merge → optimize → run、配置复用及覆盖诊断 |
| 可选 profiling 系统库组件 | 待实现 | 两套 full 库、自动选库、独立下载安装和移除 |
| GC phase/slice 事件 | 待实现 | 时间语义、采集开销和真实进程输出 |
| 使用材料及发行配套 | 待实现 | 规范、帮助、编程指导、安装器和官网组件选项一致 |

## 已完成

- 将评审设计及路线图入口纳入实现分支，包含按目标安装的 profiling 组件、Profiling/full 训练库和配套 Release/full 无插桩库。
- 核对本机 LLVM 22.1.8 的 instrumentation profile：`llvm-profdata` 可以合并，但不提供此类数据的 JSON/YAML show 输出，读取将使用同一 LLVM reader 的适配入口。
- LLVM/Darwin 原型验证：四线程各调用 100,000 次，两个实际执行记录各得到 400,000；重复 weak_odr 对象只计一次，未使用函数和计数记录均被裁剪。只验证 LLVM 接线，尚不代表 Scoop 的 MIR 位置映射完成；原型输入、对象、程序和 profile 已清理。
- Codegen 已提供 LLVM 22.1 原子计数 lowering 和 indexed profile reader。两个定向测试通过：在 Mach-O/ELF/COFF 的 LLVM IR 中确认 64 位原子更新及 ODR 分组；用真实 indexed 数据确认零计数、全零记录保留及无效输入诊断。后续 MIR/LIR 接线见下节，实际构建入口仍待接入。

### 逻辑位置与计数通道

- 新增纯数据 crate `scoop-profile`，保存正文身份、fingerprint、位置、物理记录映射及已解码频次。先在每个实际内联副本内配对调用点和入口，再汇总；零、缺失、部分覆盖、正文变化和计数溢出分别处理，不用其他副本的入口补分母。
- MIR 在首次可选 suspend 内联前固定原正文位置，普通内联保留调用点及 callee 来源；协程状态机将原入口计数保留在启动包装函数，恢复不重复计数。生成 callable 和 release hook 建立自己的计划，计数不增加 GC roots、safepoints 或 MIR cost。字符串融合与 tail forwarding 保持可用。
- LIR 只在 Generate 时发出计数，Codegen 将每个 callable 的实际位置映射为自己的 LLVM 记录。普通关闭模式不建立计划，也不额外扫描函数来建立计数记录。
- 跨 Cone 定向测试发现并修复了导入模板丢弃 CFG 调用点附加信息造成的 fingerprint 差异；普通正文及 private helper 在 provider/consumer 两侧的正文键和位置布局现已一致。
- 本机 workspace 格式化、Clippy 通过（仅既有警告）；共享反馈数据 5 项、MIR 217 项（含 4 项新位置/协程测试）、跨 Cone 匹配 1 项通过。正式对象发射定向测试在本机生成 Mach-O、ELF、COFF，确认计数随 callable 对象保留，独立元数据对象不持有计数。尚未通过实际 CLI 生成训练程序，Linux/Windows 原生训练随完整 profiling 大项验证。

### IR template 导出

- `scoop build` 与 `scoopc build` 已传递 `--export-ir off|default|full`，该选择进入正常编译更新输入并随 HIR 产物保存。compiler protocol 升至 32，cross-cone-interface 升至 95；旧系统库需要按新格式重建。
- `off` 保留语义必需的泛型、默认实参及其支持模板。`full` 保留普通和私有 callable、闭包、Context、异常、协程及普通构造器的完整模板，使用处才降低；普通 Strong 定义仍归 provider。修复了普通优化模板中的局部值未进入闭包捕获身份表的问题。
- 完整模板与当前 MIR 内联能力分开：具有模板的普通 class/secondary 构造器和 suspend callable 在当前内联器中报告 `unsupported-body`；可降低的闭包、Context 与异常正文照常进入成本评估。未导出正文报告 `no-mir-body`。
- macOS AArch64：workspace 格式化和 Clippy 通过（有既有 lint 警告）；HIR 模板消费 102 项、产物生成 121 项、协议 47 项、slib 269 项及缓存键 7 项通过。正式 CLI 的跨 Cone 内联用例通过 4 个 profile/export 组合；新的语义组合用例分别消费 default/full core，在 off/default/full 三档 provider 下均构建、链接和运行成功，验证闭包在 GC 后仍有效、Context 缺失异常、try/catch/finally、协程实际挂起/恢复、构建复用和优化报告。
- 使用本分支正式脚本准备本机 runtime，并实际构建 default/full core。观察到两种 core 产物分别为 12,224,056 与 12,541,705 字节；这是当前构建的体积记录，不是普遍开销承诺。
- Linux GNU x86-64（LLVM 22.1.2）与 Windows MinGW-UCRT x86-64（LLVM 22.1.8）：均使用独立源码目录和正式准备入口构建 runtime、Release/full core；消费 full core 的导出语义用例通过 3 档、15 次进程调用，普通跨 Cone 内联用例通过 4 个组合、16 次进程调用。Linux 另通过 4 项完整 HIR 导出定向单测。尚未用本项单独覆盖 Linux musl、Windows MSVC/MinGW-MSVCRT，不能由上述结果推定所有发行目标已验收。
- Windows 验收发现深层缓存目录中的优化报告发布失败：`tempfile::persist` 直接调用 Windows API，没有处理超长路径。发布前将已有目录解析为扩展路径后闭合；定向回归在原实现真实失败，修复后通过，随后上述 CLI 组合全部通过。本机格式化、`scoop` 定向 Clippy 和 2 项观察缓存测试通过。修复仅影响显式阶段输出复制，不增加普通缓存命中的内容检查。
- 运行期 profiling、反馈优化与独立 profiling 组件仍按表中状态推进。
