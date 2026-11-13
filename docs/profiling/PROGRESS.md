# Profiling / PGO 实施记录

状态：实施中。开始日期：2026-11-13。设计见 [DESIGN.md](DESIGN.md)。

实现分支：`codex/profiling-pgo`。每个完整变更在本地完成针对性验证后提交、推送，再合并最新 `origin/main`；跨平台验证按完整大项安排，不逐文件重复全量测试。原始日志和临时训练产物保存在仓库外，完成一轮后清理不再使用的文件。

## 交付范围

| 大项 | 当前状态 | 验收依据 |
| --- | --- | --- |
| 固定 MIR 采集位置、LLVM InstrProf 与 ODR 计数 | 待实现 | 确定计数、内联映射、多线程、对象选择/裁剪 |
| off/default/full IR template 导出 | 待实现 | 完整模板、语义必需模板保留、实际 consumer 与 Strong 归属 |
| profiling profile 与 runtime 采集 | 待实现 | 控制流/调用计数、运行指标、退出写出、release 无探针 |
| MIR cost 反馈及 `scoop-pgo` | 待实现 | instrument → merge → optimize → run、配置复用及覆盖诊断 |
| 可选 profiling 系统库组件 | 待实现 | 两套 full 库、自动选库、独立下载安装和移除 |
| GC phase/slice 事件 | 待实现 | 时间语义、采集开销和真实进程输出 |
| 使用材料及发行配套 | 待实现 | 规范、帮助、编程指导、安装器和官网组件选项一致 |

## 已完成

- 将评审设计及路线图入口纳入实现分支，包含按目标安装的 profiling 组件、Profiling/full 训练库和配套 Release/full 无插桩库。
- 核对本机 LLVM 22.1.8 的 instrumentation profile：`llvm-profdata` 可以合并，但不提供此类数据的 JSON/YAML show 输出，读取将使用同一 LLVM reader 的适配入口。

尚无编译器功能或跨平台验收完成记录；后续每项仅记录实际完成范围、结果和未闭合限制。
