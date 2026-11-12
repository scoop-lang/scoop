# macOS Scoop / Go TCP echo 对照

2026-11-12，在同一台 Mac 上使用第 29 轮的公开 API TCP echo 基准比较当前 Scoop 与 Go。Scoop 吞吐为 Go 的 54%–75%，每次往返的整个服务端 CPU 成本为 Go 的 2.3–4.4 倍。64 连接下增加到四 worker 没有提高吞吐，Scoop 的 CPU 成本反而约为单 worker 的 4.3 倍。

环境为 M3 Ultra（24 个性能核、8 个能效核、512 GiB）、macOS 27.2.1 / ARM64、交流电供电。使用 `~/.scoop/bin/scoop` 及安装的系统库与 runtime；版本输出为 `9a765e0814b05009ed30b734437dd3fdfa27d4ae+modified`。Go 为 `go1.27.3 darwin/arm64`。Scoop 使用 release 和默认 GC，没有在计时期间构建或运行其他本任务测试。

沿用[同一套服务器、客户端与测量协议](../../tests/benchmarks/tcp-echo/README.md)：loopback、1 KiB 消息、64 KiB 每连接缓冲区、每连接一个未完成请求、逐条校验内容和序号；1／64 连接，1／4 worker，另测 Scoop 独立 event loop。客户端固定 8 个 worker，每项预热 1 秒、计时 4 秒、随机交错三次，种子 `20261112`。下表各列为三次结果的独立中位数。

| 连接数 | worker 数 | Scoop echo/s | Go echo/s | Scoop / Go 吞吐 | Scoop CPU μs/echo | Go CPU μs/echo | CPU 倍数 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 26,151 | 46,500 | 56.2% | 40.84 | 9.91 | 4.12× |
| 1 | 4 | 25,737 | 47,246 | 54.5% | 42.70 | 9.62 | 4.44× |
| 64 | 1 | 82,969 | 115,038 | 72.1% | 20.20 | 8.63 | 2.34× |
| 64 | 4 | 80,172 | 107,407 | 74.6% | 87.79 | 22.42 | 3.92× |

独立 event loop 在 1／64 连接时分别为 25,696／82,984 echo/s、40.04／20.09 μs CPU/echo，与单 worker 接近。

| 连接数 | worker 数 | Scoop P50 μs | Go P50 μs | Scoop P99 μs | Go P99 μs |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 37.887 | 20.991 | 50.687 | 34.815 |
| 1 | 4 | 38.399 | 20.991 | 59.391 | 29.439 |
| 64 | 1 | 770.047 | 495.615 | 966.655 | 1,015.807 |
| 64 | 4 | 786.431 | 581.631 | 1,163.263 | 942.079 |

64 连接时，Scoop 单／四 worker 的服务端 CPU 中位数为 167.6%／703.4%，Go 为 99.1%／240.8%，其中 100% 表示一个核。Scoop 四 worker 的用户态／内核态成本分别为 14.68／73.30 μs/echo，Go 为 3.01／19.41 μs/echo；内核态成本构成四 worker 差距的大部分。Go 本身也没有在这组 loopback 负载中获得四 worker 吞吐扩展。

64 连接的三次吞吐范围全部保留：Scoop 单 worker 81,914–87,975、四 worker 79,154–80,316；Go 单 worker 91,238–115,226、四 worker 93,160–108,651 echo/s。Go 的短测波动较大，不能把表中数值当作长期性能上限。

Scoop 单／四 worker 的 RSS 中位数为 13.71／14.12 MiB，Go 为 8.52／8.73 MiB。Scoop 分配为 1.006／1.059 KiB/echo；Go 为约 0.0073／0.0077 KiB/echo。该分配口径含启动、连接准备、预热和收尾，不能把 Go 的摊销值解释为稳态每次 echo 的实际分配。Scoop 的累计 GC 收集计时约为 47.7／62.6 ns/echo；这不是暂停分位数，也不与 Go 的 STW 计时直接比较。

独立对 Scoop 单／四 worker 和 Go 四 worker 做了各五秒的 `sample` 调用栈诊断，诊断运行未进入性能统计。Scoop 栈能看到 libdispatch 串行 I/O queue、`scoop_io_notify_locked` 的 pipe write、`scoop_io_loop_poll` 的 poll/read/ioctl 与 mutex 交接，Go 则使用其 netpoll/kqueue 路径。结合内核态 CPU 差距，Darwin 完成通知及等待／唤醒路径值得优先调查。`sample` 包含阻塞线程，不能把 poll、条件变量等栈样本数当作 CPU 百分比，也不能由本次结果断言某一个函数已解释全部差距。

Mac 适配只涉及基准：进程指标使用 psutil 7.2.2；不调用 Linux taskset；每次 accept 后为 Scoop 的 Darwin socket 显式设置 TCP_NODELAY，因为实测 Darwin 不从 listener 继承该选项。Go 原基准已对 accepted socket 设置相同选项。连接配置在预热前完成，echo 热路径和生产 asyncio/runtime 未修改。native metrics archive 使用 `-mmacosx-version-min=27.0.1`，与当前 Scoop 部署基线一致；首次使用宿主 SDK 默认 27.2 的辅助对象被链接器正常拒绝，调整构建参数后完成链接。

正式 30 个样本共校验 9,413,537 次往返（其中计时窗口 7,526,436 次），全部正确。另用 64 KiB 消息、257 字节缓冲区完成五个短传输样本，共校验 1,427 次往返，全部正确。Python ruff、Scoop 格式／静态检查、native 严格 warnings 检查和差异空白检查通过。本轮为性能基准复测，未运行全仓测试或 HTTP 性能矩阵。

macOS 无法按该基准的 Linux 方式硬绑核，结果记录 `cpu_affinity=none`；worker 数不限制 libdispatch、GC 或 Go runtime 的其他线程，性能核／能效核由系统安排。RSS 峰值采用 100 ms 采样最大值，标记为 `sampled_rss`，不作为精确的进程生命周期峰值。延迟为闭环测量，未校正 coordinated omission。本报告只比较当前 Scoop 与 Go 的同机结果。

固定二进制、环境与构建命令、源码副本、逐次 JSON/stdout/stderr、汇总和独立调用栈保存在仓库外 `/Users/chenxu/benchmarks/asyncio-macos-20261112`。复现命令见该目录 `reproduce.sh`；源码内的通用入口与参数见基准 README。
