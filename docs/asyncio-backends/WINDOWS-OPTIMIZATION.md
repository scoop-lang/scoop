# Windows TCP I/O 优化

2026-11-13，分支 `codex/windows-asyncio-performance`。沿用 [Windows 初次对照](WINDOWS-TCP-ECHO.md)的机器、公开 API echo 负载和 QPC 客户端，按功能记录优化及其限制。

## W1：owner 批次直接完成 TCP 传输

ConnectEx／AcceptEx 完成 context 更新后，对交付的 stream 设置 FIONBIO。NativeSafe TCP flush 在 owner 上尝试不带 OVERLAPPED 的 recv／send，实际字节数、EOF 和错误直接进入结果数组；未就绪读取回到原 WSARecv 和真实 IOCP packet 退休路径。未就绪写入改为 WSAEventSelect 的 FD_WRITE／FD_CLOSE 和一次性 thread-pool wait，唤醒后由 owner 再次 send；取消只结束等待。直接完成不分配 native 请求、不经收集线程；managed 仲裁、ready、部分进度、buffer 借用、方向占用及取消合同保持。没有启用 skip-completion 模式。

本阶段使用安装的 `~/.scoop` 编译器 `6e9baf029ebead5d5291674830e0a35111d37a52+modified`、原配 runtime 和新构建的 asyncio；尚未混入后续 main 的编译器优化。应用 target 为 `x86_64-pc-windows-msvc-ucrt`，编译器宿主为 `x86_64-pc-windows-gnullvm`。Go 为 1.27.4。

旧版、候选、Go 随机交错三轮，共 48 个样本。1／64 连接、1 KiB 消息、64 KiB 每连接 buffer，每连接一个在途请求，预热 1 秒、计时 4 秒，客户端固定八 worker，种子 `20261113`。默认 GC、系统调度，性能测量与构建、测试分开运行。各列独立取三次中位数，CPU 单位为 μs／echo。

| 连接 | 运行方式 | 旧版 echo/s | W1 echo/s | 吞吐变化 | 旧版 CPU | W1 CPU | Go echo/s | Go CPU |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 独立 eventLoop | 10,427 | 8,540 | −18.1% | 89.91 | 111.12 | — | — |
| 1 | 1 worker | 9,401 | 9,587 | +2.0% | 104.29 | 86.78 | 12,849 | 56.97 |
| 1 | 4 worker | 8,278 | 10,310 | +24.6% | 126.94 | 87.14 | 12,494 | 58.78 |
| 64 | 独立 eventLoop | 29,762 | 42,045 | +41.3% | 41.54 | 23.68 | — | — |
| 64 | 1 worker | 34,919 | 44,887 | +28.5% | 38.80 | 22.25 | 39,755 | 25.05 |
| 64 | 4 worker | 86,912 | 104,870 | +20.7% | 53.66 | 37.62 | 104,888 | 38.17 |

64 连接单／四 worker 的 CPU 降低 42.7%／29.9%，P50 从 1,785.855／729.087 降至 1,392.639／606.207 μs，P99 从 3,440.639／1,023.999 降至 2,064.383／868.351 μs。吞吐约为本轮 Go 的 112.9%／100.0%。GC 分配约 0.883／0.892 KiB／echo，基本不变；四 worker 工作集从 19.15 增至 19.89 MiB，Go 为 12.86 MiB。

单连接仍有明确限制：独立 eventLoop 吞吐 −18.1%、CPU +23.6%，P50 从 91.135 升到 121.855 μs；1 worker 的 P99 从 212.991 升至 229.375 μs，4 worker 为 229.375 → 225.279 μs。不能将高并发的收益推广到所有模式；单 worker／单连接的 +2.0% 也不足以说明稳定吞吐改善。

保留全部低值：64 连接单 worker 旧版／W1 吞吐范围为 33,127–36,010／44,478–44,927；四 worker 为 82,497–88,485／68,659–109,033 echo/s。W1 四 worker 有一个低样本，短测中位数接近 Go 不等于已经证明长期吞吐或尾延迟相当。

严格 C warnings、Scoop 格式和静态检查通过。最终 asyncio **216／216** 项普通 release 测试通过；移动／minor GC 的取消背压写入及连续传输复用各三项，共六项通过。新增 IPv4／IPv6 native 用例覆盖直接批次、重复短读、EOF 后反向写入、待完成 receive 的真实取消退休、可写等待及重复背压恢复；关闭用例实际填满 socket 后再验证两个方向退休。

首次验收发现旧背压用例允许一次 64 MiB send 全部完成，小 SO_SNDBUF 也不限制一次 send 的接受量。改为小缓冲区上的连续分块写入后，又暴露了原 WSASend 取消路径的问题：独立实验在取消 packet、OVERLAPPED.InternalHigh 和 WSAGetOverlappedResult 都返回零时，接收端仍收到额外 65,536 字节。零字节 WSASend 也会在不可写时立即完成，不能用作无轮询的就绪等待。最终改用上述 FD_WRITE 等待，取消不向内核提交用户数据；原借用、已发送前缀和后续 marker 断言全部保留并通过。首轮失败日志和实验均保留，未跳过用例。

最终 48 项性能样本核验 8,138,784 个响应，其中计时窗口 6,644,521 个，全部正确；另用 64 KiB 消息、257 字节 buffer 的五项短传输检查核验 1,313 个响应。短传输用于正确性，不参与性能表。早期候选的另一组 48 样本单独保留，不混入最终数据。

原始数据、固定二进制、构建日志和重放脚本位于 `C:/Users/ChenXu/benchmarks/asyncio-windows-opt-20261113`：`w1-fixed-comparison` 是本阶段最终数据，`w1-fixed-short-transfers` 是短传输检查，`bin/scoop-echo-w1-fixed.exe` 与 `artifacts/w1-fixed/scoop.asyncio.slib` 是固定候选。旧版与 Go 二进制继续使用 `C:/Users/ChenXu/benchmarks/asyncio-windows-20261113/bin`。

本轮仅在 Windows MSVC／UCRT 实机执行。其他 Windows CRT、Linux、macOS 和 HTTP 吞吐不在此表范围内；闭环延迟未校正 coordinated omission。服务器和客户端同处六核／十二线程机器，不能据此推断独立服务器的吞吐上限。
