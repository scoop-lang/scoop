# macOS TCP I/O 优化

2026-11-13，在 `codex/macos-asyncio-performance` 上，针对[首次 Mac 对照](MACOS-TCP-ECHO.md)发现的 dispatch、通知和内核态开销优化 Darwin 后端。最终 64 连接下，单／四 worker 吞吐提高 12.1%／14.8%，总 CPU／echo 降低 22.7%／47.2%。主路径吞吐达到同机 Go 的 61%–84%，CPU 成本仍为 Go 的 1.8–2.9 倍，多 worker 吞吐扩展仍弱。

实现复用 Linux 优化中已有的 owner 本地推进、NativeSafe 批量提交和 ready 分轮边界：

- TCP 字节读写在 owner 的 flush 中先尝试 nonblocking syscall。字节数、EOF 和实际错误直接进入批次结果数组；EAGAIN／EWOULDBLOCK 才创建 dispatch 请求。直接路径不分配 native 请求、不跨线程通知，结果仍经过原 managed 仲裁与 ready 清理。
- TCP 每个方向懒创建并复用 dispatch source，只在有等待请求时启用。source context 保留端点与方向，不保留旧请求或 buffer；完成和取消先暂停 source、解除请求引用，再交付完成。关闭等待全部 cancel handler 退休后才释放 fd，未退休 source 计入 loop 存活工作。
- 通知 pipe 合并为一个未消费提示，在同一 mutex 下完成发布与消费；没有提示时省去 FIONREAD／read。控制动作结束仅在最后一个动作退休时通知排空，真正的数据完成仍通知。

方向占用、部分进度、取消仲裁、GC borrow、NativeSafe 与 public API 合同保持；spec 与设计先行更新。改动集中在 Darwin C 后端、共用 TCP 批次的结果交付及必要测试。

使用同一台 M3 Ultra（24 P＋8 E，512 GiB）、macOS 27.2.1 / ARM64，固定的安装编译器 `9a765e0814b05009ed30b734437dd3fdfa27d4ae+modified`、runtime 和 Go 1.27.3。候选 asyncio 与 TLS／compression／HTTP 依赖闭包放入独立 sysroot，构建 JSON 确认实际选择候选 asyncio；native 部署基线为 `27.0.1`。

沿用[原 TCP echo 套件](../../tests/benchmarks/tcp-echo/README.md)：普通单端口、1 KiB 消息、64 KiB 每连接 buffer、每连接一个在途请求，逐条校验正文与序号。旧版／最终版各有独立 eventLoop、1／4 worker，另测 Go 1／4 worker；1／64 连接，每项预热 1 秒、计时 4 秒、随机交错三次，共 48 个正式样本。客户端固定 8 个 worker，种子 `20261112`，默认 GC，计时期间没有并行构建、测试或采样。下表各列独立取三次中位数。

| 连接 | worker | 最终 Scoop echo/s | Go echo/s | Scoop / Go 吞吐 | Scoop CPU μs/echo | Go CPU μs/echo |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 28,917 | 47,262 | 61.2% | 27.69 | 9.42 |
| 1 | 4 | 29,053 | 47,053 | 61.7% | 26.49 | 9.80 |
| 64 | 1 | 95,972 | 115,476 | 83.1% | 15.12 | 8.56 |
| 64 | 4 | 92,165 | 109,417 | 84.2% | 46.06 | 22.11 |

64 连接单 worker 的旧版／最终版吞吐为 85,613 → 95,972，CPU 为 19.57 → 15.12 μs／echo；四 worker 为 80,296 → 92,165，CPU 为 87.29 → 46.06。四 worker 的内核态 CPU 从 72.86 降到 35.48 μs／echo（−51.3%），用户态从 14.43 降到 10.46（−27.5%）；整个服务器 CPU 从 701.7% 降到 424.5%，100% 表示一个核。Go 四 worker 为 241.1%。单连接单／四 worker 的吞吐分别提高 14.9%／12.1%，CPU 降低 36.8%／37.1%。

| 连接 | worker | 最终 Scoop P50 μs | Go P50 μs | 最终 Scoop P99 μs | Go P99 μs |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 34.303 | 20.991 | 48.639 | 29.439 |
| 1 | 4 | 33.791 | 20.991 | 56.319 | 30.975 |
| 64 | 1 | 712.703 | 475.135 | 909.311 | 1,040.383 |
| 64 | 4 | 688.127 | 581.631 | 933.887 | 901.119 |

四 worker／64 连接的 P50／P99 比旧版下降 12.5%／19.7%。独立 eventLoop／64 连接则只有 84,546 → 86,910 echo/s（+2.8%，两组范围重叠），CPU 为 19.69 → 17.19 μs，P50 从 753.663 升到 786.431 μs（+4.3%）；不把该形态描述为全面加速。Go 在本组 loopback 负载下也没有随 worker 数增加提高吞吐。

64 连接的三次吞吐范围全部保留：旧版单／四 worker 为 85,180–87,094／80,223–81,285；最终版为 95,152–96,556／90,824–93,188；Go 为 114,097–116,927／109,072–110,608 echo/s。未剔除低值。

64 连接单／四 worker 的 RSS 中位数从 13.87／14.15 降到 12.86／13.37 MiB。GC 分配从 1.009／1.063 变为 0.938／1.049 KiB／echo，口径包含启动、连接、预热和收尾；native source 的分配不属于该 GC 指标。本轮收益主要体现为 native 调度、系统调用和唤醒成本降低，未调整 GC 策略。

验证通过：严格 C warnings、Scoop 格式和静态检查；asyncio 190、TLS 23、HTTP 152、HTTP server 155 项 release 测试，共 520 项；提交批次与连续 TCP 进度各两项 moving＋minor GC 定向测试。新增通知用例覆盖合并、重设、最后一个控制动作退休与外线程发布竞争；TCP 用例覆盖连续传输、重复 EOF、反向通信与 worker 交接。进度用例每轮 32 字节、重复 64 轮，压力测试聚焦借用和退休。

正式测量共校验 15,140,169 次 echo（计时窗口内 12,101,575 次），全部正确；另用 64 KiB 消息、257 字节 buffer 完成五个短传输样本，共校验 2,063 次。第一版 48 个样本、六个短对照样本和独立 `sample` 诊断均单独保存，不混入最终表。早期 `--cone-path` 未覆盖系统库的尝试已标记无效，并用明确的独立 `--sysroot` 重建重跑；定向 GC 中过大的逐字节检查曾主动停止，保留相同传输轮次缩小载荷后四项全部通过。

限制：这里只实测 Darwin ARM64 与裸 TCP 性能，TLS／HTTP 是功能验收；没有重跑 Linux、Windows 或 HTTP 吞吐矩阵。macOS 使用 `cpu_affinity=none`，worker 数不限制 libdispatch、GC 或 Go runtime 的其他线程，CPU 指标覆盖整个进程。RSS 峰值是 100 ms 采样最大值。延迟为闭环统计、未校正 coordinated omission，每组仅三次短测，不能据此推断长期尾延迟或跨操作系统的绝对性能。

固定旧版与 Go 二进制位于 `/Users/chenxu/benchmarks/asyncio-macos-20261112`。最终候选、独立 sysroot、源码快照、命令、全部原始输出与汇总位于 `/Users/chenxu/benchmarks/asyncio-macos-opt-20261112`；`comparison-final` 是最终性能数据，`summary-final.json` 与 `REPORT.md` 保存完整汇总，`reproduce.sh` 可重新测量固定二进制。
