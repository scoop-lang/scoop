# Windows Scoop / Go TCP echo 对照

2026-11-13，使用已安装的当前工具链，在 Windows 上复用 Linux／macOS 的公开 API TCP echo 基准。单连接时 Scoop 吞吐为 Go 的约 64%，64 连接时为 78%–81%；整个服务器的 CPU／echo 为 Go 的 1.48–2.58 倍。64 连接从一到四 worker 的吞吐扩展为 Scoop 2.61 倍、Go 2.53 倍，本机多 worker 路径能够提高吞吐。

环境为 Intel Xeon W-2133（6 核／12 线程）、64 GiB 内存、Windows 11 Pro 10.0.26300、系统平衡电源方案。Scoop 来自 `C:/Users/ChenXu/.scoop/bin/sco.exe`，版本 `6e9baf029ebead5d5291674830e0a35111d37a52+modified`，编译器 host 为 `x86_64-pc-windows-gnullvm`，实际应用目标为默认的 `x86_64-pc-windows-msvc-ucrt`，release／LLVM 22.1.8／普通 GC。构建记录确认 core、std、platform、concurrent、asyncio 和 runtime 均来自安装目录的预编译产物。Go 为 `go1.27.4 windows/amd64`，使用默认优化和 GC。

沿用[同一套服务器、客户端与测量协议](../../tests/benchmarks/tcp-echo/README.md)：loopback、1 KiB 消息、每连接复用 64 KiB buffer、每连接一个未完成请求，逐条核对正文和序号。1／64 连接、1／4 worker，另测独立 eventLoop。客户端固定 8 个 worker；每项预热 1 秒、计时 4 秒、三个独立进程样本，按种子 `20261112` 随机交错，共 30 项。构建与计时分开，没有在计时期间并行构建或执行其他本任务测试。

本轮只补齐基准的 Windows 测量：`psutil 7.2.2` 提供进程 CPU／working set／线程和 context switch 快照；C helper 在连接准备阶段通过本进程 PSS handle 快照找到 SOCKET 并设置 TCP_NODELAY。helper 使用与应用一致的 MSVC／UCRT 目标和 Windows SDK 头文件，既有 Scoop／Go 服务端工作负载保持原样。

客户端增加 Windows QueryPerformanceCounter 时钟，本机频率为 10 MHz，同时计量吞吐窗口和逐请求延迟。第一轮使用 Go `time.Now` 的 monotonic 部分，实际基于 Windows interrupt ticks，本机约 0.5 ms 的更新时间使单连接 P50 变成零；这 30 项仅保留为历史数据，不进入下表。修正后五项时钟检查全部通过，再用同一对固定服务端重跑完整矩阵。所有下表数值来自 QPC 最终轮，每列独立取三个样本的中位数，子项中位数不保证相加等于总项中位数。

| 连接 | worker | Scoop echo/s | Go echo/s | Scoop / Go 吞吐 | Scoop CPU μs/echo | Go CPU μs/echo | CPU 倍数 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 8,647 | 13,465 | 64.2% | 113.01 | 53.93 | 2.10× |
| 1 | 4 | 8,588 | 13,444 | 63.9% | 119.17 | 46.20 | 2.58× |
| 64 | 1 | 32,997 | 42,129 | 78.3% | 39.49 | 23.64 | 1.67× |
| 64 | 4 | 86,125 | 106,658 | 80.7% | 53.76 | 36.33 | 1.48× |

单连接吞吐差约 36%；64 连接单／四 worker 分别差 21.7%／19.3%，对应 Go 吞吐为 Scoop 的 1.28／1.24 倍。单连接增加 worker 没有收益，四 worker 主要用于多连接扩展。独立 eventLoop 在 1／64 连接时为 10,579／30,923 echo/s，CPU 为 87.51／40.53 μs/echo；单连接比单 worker AsyncRuntime 快约 22.3%，64 连接则慢约 6.3%。

| 连接 | worker | Scoop P50 μs | Go P50 μs | Scoop P99 μs | Go P99 μs |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 112.639 | 69.631 | 237.567 | 167.935 |
| 1 | 4 | 114.687 | 69.631 | 223.231 | 165.887 |
| 64 | 1 | 1,802.239 | 1,490.943 | 3,768.319 | 3,637.247 |
| 64 | 4 | 729.087 | 589.823 | 1,040.383 | 1,130.495 |

单连接 P50 高约 62%–65%，64 连接 P50 高约 21%–24%。四 worker／64 连接的 P99 在这次短测中低约 8%，不能据此推断长期尾延迟优势。延迟为每连接一个在途请求的闭环统计，没有做 coordinated-omission 修正，分位数使用直方图桶上界。

全部三次吞吐范围保留，没有剔除低值：

| 连接 | worker | Scoop echo/s 范围 | Go echo/s 范围 |
| ---: | ---: | ---: | ---: |
| 1 | 1 | 8,082–8,918 | 13,110–13,547 |
| 1 | 4 | 8,424–8,901 | 13,152–13,874 |
| 64 | 1 | 32,136–34,903 | 41,828–43,519 |
| 64 | 4 | 85,423–87,982 | 104,781–108,796 |

64 连接时，Scoop 单／四 worker 整个服务器 CPU 为 131.1%／469.3%，Go 为 100.3%／397.1%，100% 表示一个逻辑 CPU 的时间。Windows 此组使用 `cpu_affinity=none`，worker 数只限定对应执行器的并行度，IOCP 收集、GC 和 Go runtime 辅助线程仍由系统调度，不能当成相同 CPU 总预算的实验。Go 四 worker 的客户端另使用约 577.2% CPU，同机客户端与服务端共享六个物理核，吞吐不代表独立服务器的上限。

| 64 连接配置 | 用户态 CPU μs/echo | 内核态 CPU μs/echo | working set 中位数 MiB | 线程峰值 | 托管分配 KiB/echo，含预热 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scoop 1 worker | 10.18 | 30.22 | 13.53 | 7 | 0.884 |
| Go 1 worker | 2.99 | 20.91 | 12.51 | 6 | 0.020 |
| Scoop 4 workers | 14.49 | 39.33 | 19.20 | 13 | 0.893 |
| Go 4 workers | 5.38 | 31.24 | 12.76 | 8 | 0.008 |

单／四 worker 的用户态 CPU 仍分别约为 Go 的 3.41／2.69 倍，内核态也更高；这是吞吐差距之外的成本差距。本轮未做独立调用栈采样，不能把全部差值归因到某个函数或某次 IOCP 交接。分配计数含启动、连接准备、预热和收尾，Go 的小摊销值不能直接视为每次稳态 echo 的实际分配；native malloc 不在 Scoop 的托管计数中。Windows 的内存读数是 working set，峰值取进程启动以来的 PeakWorkingSetSize，不等同于 macOS 的 100 ms 采样峰值。

验证通过：Python ruff、Go gofmt／vet、C11 严格 warnings、Scoop release 构建；最终 30 项正式样本共完成 4,252,494 个计时内往返，含预热共校验 5,239,967 个响应，全部正确。另以 64 KiB 消息／257 字节 buffer 完成五项短读短写检查，共 310 个响应，全部正确。没有改动生产编译器或库，未将这些检查扩展为全仓功能验收。

相较[macOS 最终记录](MACOS-OPTIMIZATION.md)中 64 连接的 83.1%／84.2%，本机 Windows 的 78.3%／80.7% 仍有更大相对差距；但处理器、Go 补丁版本、操作系统调度和网络后端不同，不能据此比较跨系统绝对性能。这里实测的是裸 TCP，不能代替 HTTP／TLS 性能测量；桌面背景工作和电源调度未隔离，三个短样本不提供长期分布保证。CPU 采用与既有套件一致的进程快照差分，观测窗口与客户端窗口存在约 1% 的边界差异。

固定二进制、源码快照、环境、构建记录、原始日志与汇总位于仓库外 `C:/Users/ChenXu/benchmarks/asyncio-windows-20261113`。`results-qpc/samples.json` 为正式结果，`short-transfers-qpc` 为最终短传输验证，`summary.json` 保存中位数与范围，`reproduce.ps1` 使用固定二进制和已保存的测量脚本生成一轮新结果。旧 `results` 使用粗粒度时钟，明确不参与最终结论。
