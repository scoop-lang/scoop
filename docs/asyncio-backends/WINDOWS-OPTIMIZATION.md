# Windows TCP I/O 优化

2026-11-13，分支 `codex/windows-asyncio-performance`。沿用 [Windows 初次对照](WINDOWS-TCP-ECHO.md)的机器、公开 API echo 负载和 QPC 客户端，按功能记录优化及其限制。

最终实现包含 W1、W2、W3，最新同轮对照见 W3。64 连接单／四 worker 吞吐接近 Go；单连接独立 eventLoop 相对 main 的回退仍未解决。

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

## W2：合并 Windows 通知，并在新编译器下重新对照

在 loop 现有 SRW lock 内记录未消费的通知：SetEvent 成功后置位，ResetEvent 成功后清除。重复发布合并为一次系统通知，空 poll 跳过 ResetEvent；清除后的新发布仍会置位，有限批次之后的逻辑完成和 managed ready 工作继续由 rearm 通知。新增用例先核验 131 个完成分三批交付，再核验外线程 32 轮、每轮 65 个完成与 owner 消费通知并发发生，所有 2,211 个记录保持顺序且只交付一次。原生 IOCP 用例通过 owner poll 消费旧提示后再发起新请求，继续验证同步 OVERLAPPED 成功等待真实 packet 的合同。

W1 提交后合入 `origin/main` 的 `e6aa6c170`。其中协程合并和 GC 根表优化涉及编译器与 runtime，已从合并提交 `8b5adc7895ef069aa3eb8aa0dc35e0b4fbc7128c` 重建 release 编译器，后续构建全部使用该编译器。只选择 `x86_64-pc-windows-msvc-ucrt` 应用目标；编译器宿主继续为 `x86_64-pc-windows-gnullvm`，LLVM 为现有 22.1.8，Go 仍为 1.27.4。

工具链范围采用仓库现有的 `lib-base`／`lib-gui` 划分：26 个基础 Cone 已重建，本轮交付基础工具链。完整准备在可选 `scoop.ui.component` 编译超过十分钟后主动停止，18 个 GUI Cone 的产物单独保存在仓库外，不留在本轮 sysroot 中。GUI 可选包未完成重建；本轮测量和上述 TCP／TLS／HTTP 测试均使用基础组件。

同一新编译器分别构建 main 的 Windows 后端、W1 和 W2；三者共用新 core、std、platform、concurrent 及 runtime 源码。对照来源分别保存在仓库外 `main-source`、`w1-new-source` 和当前分支。程序显式选择对应 sysroot、`--runtime-root E:/scoop/runtime`、同一 MSVC/UCRT unwind 和固定 metrics archive；新旧编译器的历史成绩不混合计算 Windows 改动的收益。独立对照目录首次链接缺少 vendor 路径，补齐与新工具链相同的 vendor 后构建成功，原始失败日志保留。

新工具链下的普通 release Cone 测试为 asyncio **217**、concurrent **78**、tls.asyncio **23**、http.asyncio **164**、http.server **167**，合计 **649／649**，无 ignored 或 not run。TCP 背压取消、传输复用及 TLS 短写的 moving／minor GC 专测共 **8／8**。另以普通 CLI 在 Windows 直接构建并运行 `fused-calls/program.scoop`，debug／release × normal／moving／minor 共 **6／6**，覆盖值保留、异常、挂起清理和 Context；没有将原声明中未适用于 Windows 的 fixture 记为通过。C 格式、严格 warnings、Scoop 格式与含测试的静态检查均通过；新编译器的 Rust 格式和 workspace clippy 通过，保留已有风格警告。

最终 W2 程序的五项短传输检查核验 **1,063** 个响应，全部正确。这组使用 64 KiB 消息、257 字节 buffer、两连接，预热 0.25 秒、运行 1 秒；在其他构建期间执行，只验证正文与进度，不参与性能比较。

基础工具链最终含普通、GC stress、启动统计及 stress＋启动统计四种预编译 runtime；默认消费预编译 runtime 的独立运行检查也通过。`toolchain` 为这套基础工具链，版本与范围记录在 `new-toolchain-environment.json`。

### 同一新编译器下的阶段结果

三轮随机交错共 **66** 样本，核验 **11,349,748** 个响应，其中计时窗口 **9,317,769** 个，全部正确。条件与 W1 相同：1／64 连接、1 KiB 消息、64 KiB buffer、客户端八 worker、预热 1 秒、计时 4 秒，系统默认调度和 GC；构建和测试已结束后才测量。以下为三次中位数：

| 连接 | 运行方式 | main echo/s | W1 echo/s | W2 echo/s | W2 相对 main | W2 相对 W1 | Go echo/s |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 独立 eventLoop | 12,400 | 8,328 | 7,511 | −39.4% | −9.8% | — |
| 1 | 1 worker | 8,955 | 11,200 | 10,514 | +17.4% | −6.1% | 13,145 |
| 1 | 4 worker | 8,594 | 10,587 | 10,675 | +24.2% | +0.8% | 12,626 |
| 64 | 独立 eventLoop | 32,853 | 40,498 | 43,891 | +33.6% | +8.4% | — |
| 64 | 1 worker | 36,044 | 39,314 | 45,223 | +25.5% | +15.0% | 41,483 |
| 64 | 4 worker | 79,084 | 109,601 | 109,597 | +38.6% | 0.0% | 102,155 |

| 连接 | 运行方式 | main CPU μs／echo | W1 CPU μs／echo | W2 CPU μs／echo | Go CPU μs／echo |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | 独立 eventLoop | 73.08 | 104.13 | 111.87 | — |
| 1 | 1 worker | 106.87 | 83.71 | 82.11 | 53.78 |
| 1 | 4 worker | 117.27 | 85.23 | 84.16 | 56.70 |
| 64 | 独立 eventLoop | 39.35 | 24.68 | 22.78 | — |
| 64 | 1 worker | 37.05 | 25.41 | 21.85 | 24.13 |
| 64 | 4 worker | 59.86 | 35.78 | 35.99 | 39.09 |

64 连接下，W2 相对 main 的单／四 worker CPU 降低 **41.0%／39.9%**，吞吐为 Go 的 **109.0%／107.3%**。P50 为 **1,376.255／573.439 μs**，Go 为 **1,507.327／622.591 μs**；P99 为 **2,359.295／958.463 μs**，Go 为 **3,932.159／1,163.263 μs**。分配约 **0.820／0.829 KiB／echo**，Go 为 **0.020／0.008**；进程峰值 working set 为 **17.25／21.96 MiB**，Go 为 **12.50／12.85 MiB**。

W2 的新增收益集中于高并发单 owner／单 worker：相对 W1 吞吐 **+8.4%／+15.0%**，CPU **−7.7%／−14.0%**。四 worker 的吞吐和 CPU 基本相当，P99 比 W1 的 868.351 μs 高；不能声称这项合并通知在所有配置都有改善。

单连接限制仍明显。W2 单／四 worker 吞吐为 Go 的 **80.0%／84.5%**，CPU 为 **1.53／1.48 倍**。独立 eventLoop 比 main 低 **39.4%**，CPU 高 **53.1%**，P50 由 **62.975** 升至 **113.663 μs**；这是后续需要处理的实际回退。保留全部波动：W2 单连接独立 loop 为 **7,507–9,451** echo/s；64 连接单／四 worker 为 **44,959–46,309／83,903–111,854**，main 四 worker 为 **71,785–85,154**，W1 四 worker为 **80,300–109,923**。每组只有三次，单机闭环结果不代表独立服务器或持续负载上限。

本阶段最终数据为仓库外 `w2-new-comparison`，短传输为 `w2-new-short-transfers`；固定程序为 `bin/scoop-echo-new-{main,w1,w2}.exe`，候选库与补丁在 `artifacts/w2`。`compare_stages.py` 保存实际交错矩阵，`build-comparison.ps1` 保存构建方式；`rebuild-one.ps1` 记录完整准备尝试，`finish-base.ps1` 补齐基础工具链 runtime。所有路径均相对于上述 `C:/Users/ChenXu/benchmarks/asyncio-windows-opt-20261113`。

## W3：WSARecv 同步完成直接交付

读取直接提交 WSARecv，去掉前置 nonblocking recv。参考当前 Go Windows 后端的模式选择：交付 TCP stream 前检查 provider 的 XP1_IFS_HANDLES，并尝试 FILE_SKIP_COMPLETION_PORT_ON_SUCCESS／FILE_SKIP_SET_EVENT_ON_HANDLE；仅设置成功才使用同步完成结果。OVERLAPPED、返回字节数与 flags 位于稳定请求内存中，同步返回字节数与收集线程写入的 packet count 分开。同步成功直接进入 owner 批次并退休 native 请求，WSA_IO_PENDING 继续等待真实 packet；未启用模式的 socket 连同步成功也等待 packet。写入沿用 W1 的实际 send 前缀与可取消就绪等待，managed ready、取消仲裁和 buffer 借用不变。

本阶段不改编译器或 runtime，继续使用 W2 已重建的 `8b5adc7895ef069aa3eb8aa0dc35e0b4fbc7128c` 编译器、MSVC/UCRT 基础工具链和同一 Go／QPC 客户端。只重建 asyncio 库与消费程序。三轮随机交错对照 main、W2、W3、Go，共 **66** 样本，核验 **11,540,567** 个响应，其中计时窗口 **9,436,695** 个，全部正确；参数与 W2 相同，测量期间没有编译或测试负载。下面是本轮中位数，不与上一轮的 main／Go 数值混算：

| 连接 | 运行方式 | main echo/s | W2 echo/s | W3 echo/s | W3 相对 main | W3 相对 W2 | Go echo/s |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 独立 eventLoop | 13,090 | 8,460 | 9,731 | −25.7% | +15.0% | — |
| 1 | 1 worker | 9,065 | 10,691 | 10,642 | +17.4% | −0.5% | 12,600 |
| 1 | 4 worker | 8,327 | 10,645 | 11,074 | +33.0% | +4.0% | 13,067 |
| 64 | 独立 eventLoop | 30,676 | 42,432 | 42,371 | +38.1% | −0.1% | — |
| 64 | 1 worker | 35,664 | 43,142 | 44,437 | +24.6% | +3.0% | 42,515 |
| 64 | 4 worker | 87,127 | 95,109 | 108,184 | +24.2% | +13.7% | 106,144 |

| 连接 | 运行方式 | main CPU μs／echo | W2 CPU μs／echo | W3 CPU μs／echo | Go CPU μs／echo |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | 独立 eventLoop | 69.83 | 97.89 | 97.55 | — |
| 1 | 1 worker | 110.32 | 80.41 | 79.82 | 51.32 |
| 1 | 4 worker | 125.77 | 80.36 | 85.01 | 52.79 |
| 64 | 独立 eventLoop | 40.22 | 23.47 | 23.50 | — |
| 64 | 1 worker | 36.24 | 22.99 | 22.41 | 23.61 |
| 64 | 4 worker | 53.84 | 41.31 | 36.00 | 37.53 |

64 连接单／四 worker，最终吞吐为 Go 的 **104.5%／101.9%**，CPU 相对 main 降低 **38.2%／33.1%**。W3 的新增收益主要在四 worker：相对 W2 吞吐 **+13.7%**、CPU **−12.8%**；单 worker 提升较小，独立 loop 基本持平。P50 为 **1,376.255／581.631 μs**，Go 为 **1,490.943／598.015 μs**；P99 为 **2,457.599／860.159 μs**，Go 为 **3,506.175／1,294.335 μs**。单 worker P99 高于 W2 的 2,359.295 μs，不能声称所有配置都改善。

单连接独立 loop 的回退仍未解决：相对本轮 main 吞吐 **−25.7%**、CPU **+39.7%**，P50 **61.951 → 106.495 μs**。单／四 worker 的吞吐为 Go 的 **84.5%／84.7%**，CPU 为 **1.56／1.61 倍**，P99 为 **233.471／229.375 μs**，Go 为 **172.031／180.223 μs**；W3 单连接四 worker CPU 比 W2 高 **5.8%**。较早的 12 样本单连接定向检查中，W3 相对 W2 吞吐 +6.1%、相对 main −10.4%，数据单独留在 `w3-owner-check`，不拿其中较小的回退替换完整矩阵结果。

保留全部波动：W3 单连接独立 loop 为 **8,698–9,732** echo/s，main 为 **10,172–13,501**；W3 的 64 连接单／四 worker 为 **41,865–44,961／106,895–108,483**，Go 为 **39,767–43,088／95,352–109,872**，W2 四 worker 为 **86,885–101,580**。每组只有三次，接近 Go 的短测中位数不证明长期吞吐等同。64 连接下 W3 单／四 worker 的分配为 **0.820／0.829 KiB／echo**，Go 为 **0.020／0.008**；峰值工作集为 **17.33／22.03 MiB**，Go 为 **12.53／12.77 MiB**。分配和内存差距仍在。

原生测试为默认模式之外增加真实、从未启用 packet suppression 的 IPv4／IPv6 socket，验证已缓冲同步读取仍等待 packet，覆盖取消、旧取消不影响下一请求、EOF 与重复 EOF；普通连接继续覆盖直接批次、短读、反向传输、重复背压及关闭等待。实机探针确认连接和 accept 的 IPv4／IPv6 stream 均成功启用模式，所有底层检查通过。五项 64 KiB 消息／257 字节 buffer 检查另核验 **1,260** 个响应，全部正确；其间并行进行构建，只计正确性，不计性能。

最终普通 release Cone 验收为 asyncio **217**、tls.asyncio **23**、http.asyncio **164**、http.server **167**，共 **571／571**，无 ignored 或 not run；moving／minor GC 的背压取消、传输复用及 TLS 短写共 **8／8**。验收与测量使用同一份 native 源码；C 格式与严格 warnings 检查通过。早期 W3 测试构建在复查字节数字段时主动中止，该次未计入通过数；最终结果来自修订后的重新编译。W2 的编译器、concurrent 和 fused-calls 验证记录继续保留，W3 未改变对应实现。

本轮完整矩阵在仓库外 `w3-new-comparison`，短传输在 `w3-short-transfers`；固定程序为 `bin/scoop-echo-new-w3.exe`，候选库与补丁在 `artifacts/w3`。`build-w3.ps1` 与 `compare_w3.py` 保存实际构建和交错对照方式；`verify-w3.ps1` 与 `verify-w3-gc.ps1` 保存库与 GC 验收命令，服务端测试程序通过同一命令的 `--no-run` 提前并行构建后复用。所有路径相对于前述工作目录。测量仍仅覆盖 Windows MSVC/UCRT、本机闭环 TCP echo，不外推到其他 CRT、独立服务器或 HTTP 吞吐。
