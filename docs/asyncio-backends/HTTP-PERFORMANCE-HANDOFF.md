# HTTP 性能任务交接（B47，2026-11-12）

任务已恢复推进。长线目标尚未完成：裸 TCP 已达到用户认可的 Go 80%～90% 水平，但完整 HTTP 静态服务仍有数倍差距。当前工作分支是 `perf/http-asyncio-optimizations`。用户最新要求是每轮先提交，再合并远端 `main`、推送并清理临时文件；每轮重新评估最大瓶颈，不能沿同一方向持续打磨微小收益。详细历次记录在 [PROGRESS.md](PROGRESS.md)，不要把早期记录中的“下一步”当成当前指令。

B45 实现及验收记录已提交为 **`48cf2bdd0` — Reuse HTTP/1 connection timers and response output buffers**。B46 的事件桥接候选和 B47 的操作生命周期候选均因完整服务收益有限而撤回；正式性能底座仍是 B45。两轮的固定 ELF、SDK、补丁、测试和原始测量保存在仓库外，不能把其候选 SDK 当作已采纳底座。Git 中两项未跟踪的用户打包脚本仍须保留。

## B47 最新评估：成组定位后撤回操作生命周期候选

先在仓库外分别移除请求 scope／deadline 启停／收尾，以及协议操作的方向保护／取消监听／idle，再测两组合并。它们只估算成本上界，缺少必要语义，不能交付。64 连接、1 KiB、默认 GC、独立绑核，正反顺序各一次、预热 2 秒＋测量 6 秒：

| 路径 | 1 worker CPU μs/请求 | 4 workers CPU μs/请求 |
| --- | ---: | ---: |
| 同次重建的 B45 原版 | 26.68 | 30.69 |
| 移除请求管理 | 24.58 | 26.90 |
| 移除协议操作管理 | 22.16 | 26.42 |
| 两组同时移除 | 18.66 | 22.17 |
| Go net/http | 7.19 | 8.69 |

协议操作组的上界约 **4.5／4.3 μs**，请求组约 **2.1／3.8 μs**；请求组单 worker 两次吞吐 36,725／45,701，波动不能省略。第二组对照仅关闭读写 idle，每请求 CPU 只减少约 **1.6／1.5 μs**，没有证据继续单独细调计时。

因此完整实现过同步开始／结束操作管理：准备、方向占用与 idle 安装合在同一状态边界，实际 I/O 留在调用方；撤销中间挂起包装，合并 H1 计时状态，并只在连接开始关闭时推进关闭。取消、超时、短写和关闭等待均保留。`http.asyncio` **164/164**、`http.server` **158/158** release 测试通过，已有准备失败测试还扩展了 idle 组合。

**完整服务前后交替复测仅得到单／四 worker 吞吐 +7.7%／+3.5%，CPU −7.2%／−5.4%，仍不能解释数倍差距。候选的源码、规范和测试已全部撤回，不再继续细调这一实现。** 不用早先较慢的原版样本夸大收益；候选同轮原版每请求 CPU 为 25.25／29.89 μs，候选为 23.44／28.27 μs。两组管理全部移除后的诊断路径仍需 Go 约 2.6 倍 CPU，也不能把生命周期认作完整差距的唯一原因。

详见 [B47 记录](PROGRESS.md#第四十七轮成组评估生命周期并撤回低收益操作合并)。全部 40 个样本、27,417,209 个测量请求无 wrk 请求错误；候选性能只测了完整 H1 小响应，没有扩展到大响应或协议程序。原始目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round47`，`candidate.patch`、`source/candidate/` 和 `candidate/sysroot` 是撤回候选；`source/control|request|operation|both/` 与对应二进制是成组诊断。

## B46 归因与事件桥接结论

同机重测 B45／Go，64 连接、1 KiB 正文、默认 GC、独立绑核，正反顺序各一次、预热 2 秒＋测量 6 秒：

| worker | 完整 B45 req/s／CPU μs | B45 协议连接 req/s／CPU μs | Go net/http req/s／CPU μs |
| --- | --- | --- | --- |
| 1 | 39,415／25.39 | 67,804／14.77 | 139,217／7.17 |
| 4 | 120,316／30.67 | 216,154／16.75 | 429,331／8.80 |

完整服务／协议程序各取得 6,734／6,194 个独立用户态 CPU 样本。按最近 HTTP 边界互斥归类，完整服务的请求执行与连接管理约 16.3%，但该组最大独立函数归属仅约 3.3%；解码与 native 桥接约 11.8%，HTTP/1 操作生命周期与 idle 约 10.6%，路由及 middleware 约 9.9%，响应编码约 7.2%。GC 栈约 11.4%，只作样本解释，不启动 GC 支线。原生栈与内联限制、实际分组脚本均保留；这些粗分组不能与 inclusive 子树相加，也不是可全部消除的成本。

因此试过按批次接管 native 事件、直接读取字段视图、保留已有不可变数组快照，并只给有界无分配 getter／批次取出使用 GCLeaf。空 Trailers、分段输入、消息顺序与关闭后的已返回数据仍保留。独立 1,000 GET 减零 GET 的计数确认普通 native 调用由 20 次降到 2 次，另有 9 次 GCLeaf；RECV／SEND 均保持一次。**完整服务前后交替实测只有单／四 worker 吞吐 +2.6%／+3.7%，CPU −2.9%／−6.4%；单 worker P50 还回退 6.3%。候选已撤回，不再细调桥接。** 本轮没有采纳新的 HTTP 性能实现。

详细数值、样本波动、验证范围见 [B46 记录](PROGRESS.md#第四十六轮重新归因并撤回低收益事件桥接候选)。原始目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round46`，其中 `candidate.patch` 和 `source/` 是撤回候选；`profiles/` 是 B45 正式底座采样；`summary.json` 汇总 20 个样本。后续继续使用下文 B45 固定产物。

## 用户明确的优先级

- 优先主路径、架构和数倍差距。完成 TCP 主线后，当前只推进 HTTP 协议／连接／请求执行层。
- 先解决静态内容，JSON 和其他序列化暂时不动。当前关注吞吐、每请求 CPU、内存、P50／P80，不把 P99 当主要目标。
- 用户已叫停 GC 支线。保留默认 GC 进行公平测量；GC 计数可以解释样本，不能因此又转向阈值、nursery 或 GC worker 调参。
- 收益只有几个百分点的候选应及时收束、换方向，不反复打磨局部分支。不能用取消作用域、超时、清理或协议语义换取表面吞吐。
- 每轮在该分支先提交，再 fetch 并 merge 远端 `main`，随后推送。先完整实现一项，再集中格式化、检查和必要测试，不逐个小修改跑大回归。
- 源码／spec／测试遵守仓库 [AGENTS.md](../../AGENTS.md)。输出中文，代码标识符和注释英文。
- 原安装目录 `~/.scoop` 和历史固定二进制保持。新候选、cache、日志与原始 JSON 都在仓库外；完成后清本轮不再需要的 cache／target，不动依赖底座或仓库原有大 target。
- 重建编译器时必须使用从源码构建并带项目私有补丁的 LLVM；参考 [scripts/README.md](../../scripts/README.md) 和打包流程。不能替换为发行版 LLVM。下载使用 `https_proxy=http://192.168.2.106:10809`。
- `scripts/package_linux.py` 和 `scripts/tests/test_package_linux.py` 是用户已有的未跟踪文件，不属于本任务，不修改、不删除、不提交。

## 当前最重要的结论

**不要因怀疑 C 代码质量就用 Scoop 重写 llhttp。主要时间花在已有的 Scoop HTTP 上层。** B44 的相同 46 字节 GET、parser 复用实验：llhttp 无用户回调下界约 **0.056 μs/请求**；正式 C 回调、字段创建和事件取出／释放合计约 **0.194 μs/请求**。这些成本不能解释完整服务约 30 μs/请求。

B44 网络分层均使用 1 KiB 静态正文、64 连接、TCP_NODELAY、默认 GC，服务与客户端独立绑核。每组正反顺序各一次，预热 2 秒、测量 6 秒，以下为中位数：

| 路径 | 1 worker req/s | CPU μs/请求 | 4 workers req/s | CPU μs/请求 |
| --- | ---: | ---: | ---: | ---: |
| AsyncTcpStream＋分隔符＋预编码响应 | 210,622 | 4.75 | 601,813 | 6.35 |
| 同上，使用 llhttp＋正式 C 事件 | 201,295 | 4.96 | 592,145 | 6.47 |
| 正式 Http1Decoder＋预编码响应 | 150,577 | 6.64 | 448,363 | 8.44 |
| 正式 decoder＋encoder | 121,545 | 8.23 | 337,245 | 10.53 |
| AsyncHttp1Connection | 68,547 | 14.59 | 217,573 | 16.80 |
| 完整 HttpServer | 33,786 | 29.60 | 103,391 | 35.21 |
| Go net/http | 139,895 | 7.15 | 433,921 | 8.73 |

单 worker 相邻路径增量约 **0.21／1.68／1.59／6.36／15.01 μs**。这是不同完整路径的差分，含功能／表示差异，不能当作可直接相加的 profiler self 百分比。薄路径没有完整流式 body、取消、超时和请求生命周期，不是生产替代，也不能凭其吞吐宣称已超过 Go。

确有 C／Scoop 接口设计冗余：无正文 GET 仍产生 Request、空 Trailers、End 三个事件；输入字段先复制到 C，再逐项取回临时 Array，消息构造器还保存快照。独立探针测得接收／事件访问 **16 次普通 native 调用/请求**，加上协议／完整服务分别 **3／4 次 switched 查询**；RECV、SEND 各一次。C 本体很快不代表细碎 FFI 免费。若之后重做桥接，应批量返回结果、明确字节所有权、只构造一次消息，保留分段输入后事件有效的合同。不能简单删除复制，也不能把每个 llhttp 回调换成 Scoop 回调。

HTTP 响应编码和传输编排本来就在 Scoop。B44 没有发现能解释数倍差距的 C 解析算法或 TCP 发送次数问题；该调查不是完整 C 正确性审计。

诊断源码及构建方式：[tests/benchmarks/http-layers/README.md](../../tests/benchmarks/http-layers/README.md)。B44 提交为 `f36645af0f9310e8a1794f328b983b5e8809a091`。

## B45 已交付的改动

这轮在 B42 正式底座上只重建 `scoop.http.asyncio` 和 `scoop.http.server`，没有重建编译器、runtime 或调整 GC。

1. H1 请求头等待直接属于连接 Job，连接复用计时状态和更早的未到期通知。通知触发时核对当前等待的实际期限；头部完成后不计 handler 时间，更早期限会替换通知，连接结束撤销通知。头部超时本来就要求结束连接，因而移除每条头部独立的子作用域；**每个请求的独立 coroutineScope、RequestDeadline、清理和公平 yield 仍保留**。
2. 增加完整响应的不可变 `Array<Byte>` 重载，直接按块复制到连接的输出 buffer。静态响应不再每请求建立中间 NativeByteBuffer。最多 64 KiB 的输出块、短写、flush、取消和关闭语义沿用共同过程。
3. `Http1ResponseProgress.writtenBytes` 在发送成功或失败返回后报告实际已发送的正文前缀，排除响应头，flush 失败保留已发正文。server 完成记录继续报告真实进度。公开 native-buffer 重载共用同一发送实现。

主要代码入口：

- [http1_head_deadline.scoop](../../sysroot/src/scoop.http.server/src/http1_head_deadline.scoop)、[server_http1.scoop](../../sysroot/src/scoop.http.server/src/server_http1.scoop)。
- [http1_buffered_response.scoop](../../sysroot/src/scoop.http.asyncio/src/http1_buffered_response.scoop)、[http1_connection.scoop](../../sysroot/src/scoop.http.asyncio/src/http1_connection.scoop)。
- [http1_io.scoop](../../sysroot/src/scoop.http.server/src/http1_io.scoop)、[response_execution.scoop](../../sysroot/src/scoop.http.server/src/response_execution.scoop)。
- 先更新了 [HTTP spec](../specs/SCOOP-HTTP-SPEC.md) 和 [server spec](../specs/SCOOP-HTTP-SERVER-SPEC.md)。

完整小响应在本轮前后交替、每配置两次测量中：

| worker | req/s 前 → 后 | CPU μs/请求 | P50 μs | P80 μs | RSS MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 33,872 → 39,580 | 29.50 → 25.29 | 2,020 → 1,704.5 | 2,142.5 → 1,856 | 43.87 → 43.41 |
| 2 | 61,894 → 73,008 | 31.54 → 26.68 | 872 → 739 | 1,298.5 → 1,117 | 46.88 → 45.68 |
| 4 | 103,774 → 120,995 | 35.21 → 30.28 | 498 → 426 | 899.5 → 770.5 | 48.91 → 47.72 |

单／四 worker 小响应吞吐提高约 **16.9%／16.6%**，每请求 CPU 下降 **14.3%／14.0%**。与 B44 的 Go 固定程序结果相比，剩余 CPU 仍约 **3.5 倍**；这是跨轮参考，B45 没有重测 Go。不能说 HTTP 主问题已经解决。完整大小响应矩阵、共享 native-buffer 路径对照和计数见 [B45 记录](PROGRESS.md#第四十五轮复用连接头部计时与响应输出缓冲)。

完整大小响应的 1／2／4 worker 矩阵中，吞吐提高 **16.6%～22.3%**，CPU 降低 **14.0%～17.8%**。两项改动一起测量，没有分别归因。共享 native-buffer helper 的直接协议程序有小幅回退：单 worker 吞吐 −2.3%、CPU +2.2%；四 worker 吞吐 −2.0%、CPU +0.5%。这些样本均保留，没有单独证明原因，不宣称协议层整体加速。

普通 release 测试 `http.asyncio` **164/164**、`http.server` **158/158** 通过，合计 **322 项**。新增 5 项覆盖跨请求复用旧计时通知、活动 handler 不被旧通知取消、更早 keep-alive 期限及连接隔离，以及数组响应的多块／短写、部分失败、flush 超时进度、HEAD 定界、源不变和复用。已有 H2、TLS、WebSocket、SSE、请求取消及清理组合也随上述 Cone 通过。没有运行全仓 CLI／Rust 回归或其他平台性能验收。

32 个性能样本共 **15,691,836 个测量请求**无 wrk 请求错误；独立核验 **3,072 个大小交替响应**及探针实验的 **2,000 个响应**，正文逐字节正确。探针确认每 1,000 GET 中间 native buffer 的 resize／free 净计数各 1,000→0，timer 登记差分减少 1,000 次，RECV／SEND 仍各一次/请求。计时窗口存在一次边界偏移，原始控制／负载计数及解释在 B45 记录中。原始日志与产物已保留，构建／测试 cache 和 target 在收尾时清理。

## 已完成的底层工作，不要重新从零开始

- B13／B15：业务 worker 已有无锁队列、外部 inbox、本地环形 FIFO 和批量 work stealing。对应 [coroutine_queue.scoop](../../sysroot/src/scoop.concurrent/src/coroutine_queue.scoop)、[coroutine_local_queue.scoop](../../sysroot/src/scoop.concurrent/src/coroutine_local_queue.scoop)、[coroutine_worker.scoop](../../sysroot/src/scoop.concurrent/src/coroutine_worker.scoop)。不是只有 GC 有 work stealing。
- B18 起：每个 worker 驱动自己的 reactor，仍是一个逻辑 EventLoop、单 listener 和共享结构化任务树；资源及在途操作不随任务窃取迁移。[DESIGN.md](DESIGN.md) 记录了 owner、跨线程投递、Mio／Tokio／tokio-uring 参照。
- B19～B29：完成批次、owner 直接登记、本地／外部队列复用、资源等待槽、取消监听复用、受控恢复、尾挂起转交及 TCP 批量提交已实现。TCP 达到用户认可的阶段目标后已转向 HTTP。
- B34：当前固定 CLI 含可组合异常状态及完整 EH 内联。曾修复多层 catch/finally 覆盖异常记录的真实错误。当前固定 LLVM 来自 B28 源码构建及私有补丁。
- B36：完整字节响应已能合并 HTTP 写操作；B45 在此基础上进一步去掉中间 buffer。
- B42（`f27fa2d97`）：词法 coroutineScope 保留独立 Job，但复用当前 execution；Job 状态／监听统一同步。每 GET execution drive 7→3、外部恢复 5→3，内部恢复仍 25。不要误以为整个挂起调用链都已消除。

## 不应重复投入的方向与未闭合问题

| 已做的调查 | 结论／处理 |
| --- | --- |
| B35 消除大多数同步闭包 | 闭包数大幅减少，但 CPU 只改善约 3%～5%；诊断性内联规则已撤回。 |
| B37 合并头部计时与读操作 | CPU 下降但吞吐／P50 回退，未采纳。去掉头部和请求作用域仅作为上界，破坏语义，不能交付。B45 是连接级复用计时通知并消除中间 buffer，不能与该候选混淆。 |
| B38 同 Cone 挂起 CFG 合并 | 内部恢复数减少，整体仅约 5% 收益，撤回。 |
| B40 去除逐次根登记 | 约 5%～7% 收益，不足以解释 HTTP 数倍差距，撤回。 |
| B43 GC 支线 | 用户叫停，仅仓库外诊断；不要使用其候选 sysroot，不继续调 GC。 |
| 默认 GC 的协议程序慢档 | B42 已观察到四 worker 偶发低吞吐，全部成功样本应保留。不能只挑中位数好看的样本，也不能把暂停／测量窗口比当成 GC CPU 占比。 |
| 旧编译器测试漂移 | B28／B34 曾记录 MIR 函数编号、task-context 布局及产物快照等失败，部分经固定基线复现。B45 没有重跑它们，不能声称全仓绿；细节见对应轮次和外部日志。 |

## 接手后优先做什么

每轮先重新比较可验证的瓶颈，不能沿上一轮方向持续打磨。B47 已完成此前提出的请求／协议操作生命周期成组差分，也完整实现并撤回了操作合并候选；**不要再将“合并 h1Operation／withH1Idle”或“减少 idle 包装”当成尚未验证的大收益路线。** B46 的事件批次桥接同样已经验证过。

当前正式实现仍为 B45，B46 的正式路径 CPU 样本可继续作为起点；B47 的诊断路径与候选不是新底座。B47 两组管理都移除后，完整服务仍有 18.7／22.2 μs CPU，而 Go 为 7.2／8.7 μs，说明下一项评估必须覆盖生命周期之外的剩余路径。应重新比较消息表示与准备、分派及跨库调用的组合成本，先取得完整路径差分或归因，再选择结构改造；这里是待评估范围，不是已证明的新热点或固定路线。

B46 同轮完整 B45 相对协议连接多约 10.6／13.9 μs，包含功能与表示差异；不能将它们全部当作可删除开销。早先单独路由、同步闭包、同 Cone 挂起 CFG 合并和根登记的候选收益都有限，没有新证据就不重试。保留实际请求生命周期、middleware 可组合性、取消、截止时间、真实进度和清理；不为无 middleware、某条静态路由硬编码捷径，不启动 GC、P99 或 JSON 支线。

## 固定产物和复现入口

以下路径都在当前机器，原始日志不在 Git 中。**不要在这些目录上直接重建覆盖，新的实验另建目录。** B45 的一些 bin／tools 是指向旧轮的链接，不能删除其目标。

| 用途 | 路径 |
| --- | --- |
| 本轮固定产物、脚本、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261112-round45` |
| 当前正式候选 SDK | `B45/candidate/sysroot` |
| 当前完整／协议 ELF | `B45/bin/scoop-full-after`、`B45/bin/scoop-protocol-after` |
| 本轮之前的正式底座 | `/home/chenxu/benchmarks/http-improvements-20261112-round42/candidate/sysroot` |
| 前版完整／协议 ELF | `B42/bin/scoop-full-after`、`B42/bin/scoop-protocol-after` |
| 固定 CLI | `/home/chenxu/benchmarks/http-improvements-20261112-round34/candidate/cargo/release` |
| 源码 LLVM 22.1.8 SDK | `/home/chenxu/benchmarks/http-improvements-20261112-round28/candidate/llvm/installed` |
| C compiler／headers 包装 | `/home/chenxu/benchmarks/http-improvements-20261112-round29/candidate/cc-with-headers` |
| B44 分层诊断及结果 | `/home/chenxu/benchmarks/http-improvements-20261112-round44` |
| 固定 Go net/http | `/home/chenxu/benchmarks/http-20261111/bin/nethttp-server` |

表中 B45／B42 表示各自完整根目录。B45 目录内：

- `build.sh`、`validate.sh`：本轮真实构建／验收命令。先复制并调整到新目录再用；`build.sh` 会从当前仓库复制源文件。
- `inputs/full_server.scoop`、`inputs/protocol_server.scoop`：固定工作负载。
- `screen.py`、`extend.py`、`run.py`、`latency.lua`：完整测量入口；每个样本在 `results/<配置>/result.json`，另有原始 wrk、stdout／stderr。路径已存在时拒绝覆盖，重跑要用新目录或新的 mode。
- `verify_http.py`：1／2／4 worker、每组 32 客户端、大小交替正文逐字节核验。
- `count_resources.py`：独立 uprobes 计数，以同连接 1,000 个 GET 减 0 GET 控制；与性能测量分开。LoopTimer 的 managed 符号必须与实际固定二进制匹配，代码重建后不要盲用旧哈希。
- `logs/test-scoop.http.asyncio.log`、`logs/test-scoop.http.server.log`：322 项验收结果。`logs/validation.log` 是两组完成标记。
- `manifest.json`、`summary.json`、`candidate.patch`、`source/`：产物来源、32 样本汇总、实现补丁和本轮源码快照；`verification/`、`diagnostics/` 保存正文核验及独立计数，`cleanup.json` 记录清理范围。
- `symbols-full-after`、`symbols-protocol-after`：保留符号用于后续 CPU 归因。使用 perf 时注意内联归因，避免把子树成本与 self 重复相加。

快速复测固定前后产物的例子（先把 `bench_next` 换成未使用目录）：

```sh
bench_prior=/home/chenxu/benchmarks/http-improvements-20261112-round45
bench_next=/home/chenxu/benchmarks/http-handoff-check
mkdir "$bench_next"
cp "$bench_prior/run.py" "$bench_prior/latency.lua" "$bench_next/"
ln -s "$bench_prior/bin" "$bench_next/bin"
ln -s "$bench_prior/tools" "$bench_next/tools"
cd "$bench_next"
python3 - <<'PY'
import run
for repeat in range(2):
    variants = ("scoop-before", "scoop-after")
    for variant in variants if repeat == 0 else reversed(variants):
        run.run_sample(variant, 1, "small", seconds=6, warmup=2,
                       repeat=repeat, mode="handoff-check")
PY
```

脚本服务核为 `0,2,4,6` 的前 N 个，客户端核为 `8,10,12,13,14,15,16,17`，端口 `18081`；worker 不是进程总线程数，统计包含进程全部线程 CPU。另机必须重新核对 CPU 拓扑。性能跑批期间不要编译、跑测试或开 profiler；所有样本前后核验正文，wrk 本身不逐个核验正文。闭环 P50／P80 未校正 coordinated omission，短测两次不代表长期分布。

初始需求的 Gin／Axum、多负载和 GC 全矩阵仍属于长线 benchmark 范围，最近几轮有意只用静态 HTTP 与 Go 参照定位主瓶颈；不要把当前交接当作原始完整 benchmark 已完成。
