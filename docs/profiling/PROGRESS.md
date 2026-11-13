# Profiling / PGO 实施记录

状态：实施中。开始日期：2026-11-13。设计见 [DESIGN.md](DESIGN.md)。

实现分支：`codex/profiling-pgo`。每个完整变更在本地完成针对性验证后提交、推送，再合并最新 `origin/main`；跨平台验证按完整大项安排，不逐文件重复全量测试。原始日志和临时训练产物保存在仓库外，完成一轮后清理不再使用的文件。

## 交付范围

| 大项 | 当前状态 | 验收依据 |
| --- | --- | --- |
| 固定 MIR 采集位置、LLVM InstrProf 与 ODR 计数 | LLVM 适配已实现；MIR 位置与生产接线待实现 | 确定计数、内联映射、多线程、对象选择/裁剪 |
| off/default/full IR template 导出 | 本地实现与验收完成；跨平台待验收 | 完整模板、语义必需模板保留、实际 consumer 与 Strong 归属 |
| profiling profile 与 runtime 采集 | 待实现 | 控制流/调用计数、运行指标、退出写出、release 无探针 |
| MIR cost 反馈及 `scoop-pgo` | 待实现 | instrument → merge → optimize → run、配置复用及覆盖诊断 |
| 可选 profiling 系统库组件 | 待实现 | 两套 full 库、自动选库、独立下载安装和移除 |
| GC phase/slice 事件 | 待实现 | 时间语义、采集开销和真实进程输出 |
| 使用材料及发行配套 | 待实现 | 规范、帮助、编程指导、安装器和官网组件选项一致 |

## 已完成

- 将评审设计及路线图入口纳入实现分支，包含按目标安装的 profiling 组件、Profiling/full 训练库和配套 Release/full 无插桩库。
- 核对本机 LLVM 22.1.8 的 instrumentation profile：`llvm-profdata` 可以合并，但不提供此类数据的 JSON/YAML show 输出，读取将使用同一 LLVM reader 的适配入口。
- LLVM/Darwin 原型验证：四线程各调用 100,000 次，两个实际执行记录各得到 400,000；重复 weak_odr 对象只计一次，未使用函数和计数记录均被裁剪。只验证 LLVM 接线，尚不代表 Scoop 的 MIR 位置映射完成；原型输入、对象、程序和 profile 已清理。
- Codegen 已提供 LLVM 22.1 原子计数 lowering 和 indexed profile reader。两个定向测试通过：在 Mach-O/ELF/COFF 的 LLVM IR 中确认 64 位原子更新及 ODR 分组；用真实 indexed 数据确认零计数、全零记录保留及无效输入诊断。已完成本机格式化与定向 Clippy；尚未接入实际 MIR/LIR 和构建入口，不能据此认为 profiling 程序已可构建。

### IR template 导出

- `scoop build` 与 `scoopc build` 已传递 `--export-ir off|default|full`，该选择进入正常编译更新输入并随 HIR 产物保存。compiler protocol 升至 32，cross-cone-interface 升至 95；旧系统库需要按新格式重建。
- `off` 保留语义必需的泛型、默认实参及其支持模板。`full` 保留普通和私有 callable、闭包、Context、异常、协程及普通构造器的完整模板，使用处才降低；普通 Strong 定义仍归 provider。修复了普通优化模板中的局部值未进入闭包捕获身份表的问题。
- 完整模板与当前 MIR 内联能力分开：具有模板的普通 class/secondary 构造器和 suspend callable 在当前内联器中报告 `unsupported-body`；可降低的闭包、Context 与异常正文照常进入成本评估。未导出正文报告 `no-mir-body`。
- macOS AArch64：workspace 格式化和 Clippy 通过（有既有 lint 警告）；HIR 模板消费 102 项、产物生成 121 项、协议 47 项、slib 269 项及缓存键 7 项通过。正式 CLI 的跨 Cone 内联用例通过 4 个 profile/export 组合；新的语义组合用例分别消费 default/full core，在 off/default/full 三档 provider 下均构建、链接和运行成功，验证闭包在 GC 后仍有效、Context 缺失异常、try/catch/finally、协程实际挂起/恢复、构建复用和优化报告。
- 使用本分支正式脚本准备本机 runtime，并实际构建 default/full core。观察到两种 core 产物分别为 12,224,056 与 12,541,705 字节；这是当前构建的体积记录，不是普遍开销承诺。
- 本项尚未做 Linux/Windows 验收；运行期 profiling、反馈优化与独立 profiling 组件仍按表中状态推进。
