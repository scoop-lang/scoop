# HTTP 性能任务交接（B60，2026-11-13）

任务已恢复推进。长线目标尚未完成：裸 TCP 已达到用户认可的 Go 80%～90% 水平，完整 HTTP 静态服务的小响应 CPU 仍为 Go 约 2.03～2.05 倍。当前工作分支是 `perf/http-asyncio-optimizations`。每轮先提交，再合并远端 `main`、推送并清理临时文件；每轮重新评估最大瓶颈。用户明确要求：找不到单一大头时，按已知成本从大到小，将主要几项整体优化一轮；**正确性通过、整体有可确认的收益就保留，即使幅度较小，也要考虑其对通用路径的价值。** 详细历次记录在 [PROGRESS.md](PROGRESS.md)，不要把早期记录中的“下一步”当成当前指令。

**当前正式底座为 B60：保留 Duration 加减、缓冲借用和请求初始化的组合优化，继承 B57～B59 的改进。** 正式 SDK 为 `B60/candidate/sysroot`，配套 CLI 为 `B60/toolchain/bin`（经 B59 链接 B57）；runtime 合同为 35、metadata 为 12，不能混用旧 B34 CLI／ABI 34 SDK。B60 从 `29eadca923` 开始，使用相同 B57 工具链及同一输入路径重建 control／candidate，候选重建 18 个库，沿用 B57 runtime。始终使用 B28 源码构建且带私有补丁的 LLVM 22.1.8；HTTP 基准源码、默认 GC 和参数保持一致。Git 中两项未跟踪的用户打包脚本仍须保留。

B55 收尾已合并远端 `main` 的 `6e9baf029`，其中共享完成批次字段的源码更新进入 B57 两套重建产物。B58 收尾合并了 `e6aa6c170`，B59 再次 fetch／merge 时 main 未变化。B57～B60 仅在 Linux 实机运行；B57 对其他目标的代码生成／根位置规则有相应单测覆盖，没有执行 macOS／Windows 实机验收。

B53 的独立 H1 应用执行与 B54～B56 的分层诊断继续保留。B57 重新限制了旧闭包候选的适用范围，并补齐根表与挂起合并的移动 GC、异常及 native callback 验证；它不是直接采纳旧 B35 的成本豁免，也不是把旧候选百分比相加。B50、B55 等已撤回实现不进入新底座。

## B60 最新结果：保留时间运算、缓冲借用与请求初始化的组合

先从最简 TCP 起点重测 36 组分层对照。手写 → 协议连接增量 **3.526／4.632 μs/请求**，fair → direct 请求执行增量 **3.155／3.325 μs**；B59 最大单项 self 约 3%，仍无单一大头。结合时间源码约 **3.11%／2.46%** self、每请求 29 个 Duration，以及五个残余借用包装闭包，选择三组共享成本整体优化。

Duration.plus／minus 直接使用规范化秒／纳秒，在进位或借位后检查最终溢出，删除无调用者的旧相加辅助函数；class、不变量和结果对象保持。NativeByteBuffer.beginBorrow 直接使用原加锁／finally 解锁，保留范围、借用保活和游标规则。请求 deadline 初始化合并为一次状态更新；H1／H2 完成观察器按需捕获计时起点，动态 deadline 仍从独立保存的管线起点计时。公开使用方式、编译器、runtime 和 GC 参数保持。

64 连接、默认 GC、固定绑核，预热 2 秒＋测量 8 秒，正反各一次；CPU 单位 μs/请求：

| 响应／阶段 | 单 worker CPU：前 → 后 | 四 worker CPU：前 → 后 | 吞吐变化：单／四 worker |
| --- | --- | --- | --- |
| 1 KiB 手写 TCP | 4.251 → 4.279（+0.7%） | 5.782 → 5.704（−1.3%） | −0.6%／+2.6% |
| 1 KiB 正式协议连接 | 7.716 → 7.546（−2.2%） | 10.391 → 10.163（−2.2%） | +2.3%／+5.1% |
| 1 KiB 完整 server | 15.052 → 14.698（−2.3%） | 18.554 → 17.916（−3.4%） | +2.5%／+4.7% |
| 64 KiB 完整 server | 27.775 → 27.987（+0.8%） | 31.277 → 30.682（−1.9%） | −0.7%／+3.7% |

完整小响应的四次配对 CPU 均下降，P80 **1,216→1,197.5／514.5→490 μs**；单 worker 大响应两次方向相反，全部纳入。独立单连接追踪确认协议每请求少 **280 bytes／5 个闭包**；完整 server 少 **344 bytes／7 个对象**，包括六个闭包、一个 Duration，同时可选计时让 H1 frame 增加 8 bytes。28 个 Duration 仍保留；减法结果的分配位置改变不等于消除对象。64 连接压测中完整小／大响应分配下降约 **3.1%／3.8%**。

**76 个无仪表样本、131,515,825 个请求**无错误。七个库普通 release **1,152 项**、借用 moving／minor **16 项**全部通过，另逐字节核验 **4,312 个**网络响应及 **400 个**分配探针响应。没有验收失败；构建、测试、计数、CPU 采样与计时分别进行。小幅组合收益按用户要求保留，未做单项消融；手写单 worker 和大响应单 worker 的回退同样保留。

新 profile 共 **30,978 个**独立样本，server／core／http.asyncio self 为 **20.07%／16.28%／13.47%**（单 worker）及 **19.77%／16.20%／11.82%**（四 worker）。时间源码降至约 **1.16%／0.97%**，最大单项 executeHttp1Request 约 **3.13%／2.98%**；仍无单一大头。完整小／大响应 CPU 为 Go 约 **2.03～2.05／1.92～2.04 倍**，长线目标未完成。下一轮重新测协议与请求执行边界，不默认继续优化时间运算。完整记录见 [B60](PROGRESS.md#第六十轮减少时间运算与缓冲借用开销)。

## B59 历史结果：保留 Job 状态查询与 HTTP 输出组合的小幅收益

先在 B58 上重建中间程序，重新测得手写 TCP → 协议连接总增量 **3.477／4.387 μs/请求**，fair → direct 请求执行增量 **3.202／3.033 μs**。实际 acquire 样本多数在 CAS 后的跳转附近；单连接计数中完整 server 每请求 **102 次**加锁，Job 只读查询及内联调用占 **24 次**，没有失败 CAS 重试。它不代表高并发无争用，也不足以把重复 poll 作为主攻方向。

Job 取消异常与完成结果按 release／acquire 顺序发布，isCompleted、isCancelled、ensureActive 只读已发布状态；状态变更、监听登记与摘取仍受原锁保护。HTTP/1 输出复用每次 writeFrom 后的实际剩余字节数，并省去首块重复 clear，保留进度核对、短写、异常、flush、idle 与清理。语言规范先行，公开签名、编译器、runtime 与 GC 参数保持。

64 连接、默认 GC、固定绑核，预热 2 秒＋测量 8 秒，正反各一次；CPU 单位 μs/请求：

| 响应／阶段 | 单 worker CPU：前 → 后 | 四 worker CPU：前 → 后 | 吞吐变化：单／四 worker |
| --- | --- | --- | --- |
| 1 KiB 手写 TCP | 4.288 → 4.215（−1.7%） | 5.891 → 5.851（−0.7%） | +1.6%／+0.5% |
| 1 KiB 正式协议连接 | 7.843 → 7.778（−0.8%） | 10.297 → 10.143（−1.5%） | +0.9%／+0.5% |
| 1 KiB 完整 server | 15.260 → 14.886（−2.5%） | 18.748 → 18.805（+0.3%） | +2.5%／+1.4% |
| 64 KiB 完整 server | 27.998 → 27.825（−0.6%） | 31.589 → 31.633（+0.1%） | +0.6%／−0.9% |

完整 server 小响应 P80 **1,210→1,199.5／486.5→479 μs**；大响应四 worker P80 **765.5→770 μs**。单 worker 小响应两次配对 CPU 都下降；四 worker 的小变化全部保留，不写成所有负载均改善。完整 server 分配增加约 **40 bytes/请求（0.34%～0.36%）**：新 AtomicInt 占 24 bytes，继承 Job 的 `Deferred<Unit>` 布局增加 16 bytes。协议分配保持。独立计数中完整 server 的锁获取 **102→74**，协议 **74→52**；这些次数不能直接换算 CPU。

**76 个无仪表样本、129,999,833 个请求**无 wrk 错误。普通 release 覆盖 **1,174 项**；初版测试的 AtomicRef 类型约束和 EOF 异常断言已修正，HTTP 异步层首轮 164／165 通过，相关容器 4／4 复测通过。两个跨线程发布测试在 moving／minor 共 **4 次**通过；另核验 **4,312 个**网络响应与 **1,000 个**计数探针响应。全部构建、测试、计数、采样和计时分别执行，原始失败及闭合日志保留。

新 profile 共 **30,161 个**独立样本，server／core／http.asyncio self 为 **20.20%／16.27%／12.76%**（单 worker）及 **19.24%／16.21%／11.83%**（四 worker）。四个 acquire helper 合计 **7.77%／8.51%**；最大单项 self 约 **3.08%／2.64%**，仍没有单一大头。组合收益虽小，按用户要求保留，未做单项消融。完整小／大响应 CPU 仍为 Go 约 **2.08～2.13／1.92～2.02 倍**，目标尚未完成；下一轮从新底座重测协议与请求执行边界，不把已消除的查询加锁继续作为默认重点。详见 [B59 记录](PROGRESS.md#第五十九轮发布-job-只读状态并减少-http-输出状态读取)。

## B58 历史结果：保留同步包装与请求分派组合

先用 B57 底座重建应用中间程序，按最简 TCP、正式协议连接、请求作用域、请求执行、解析、路由和完整 server 重新比较。手写 → 协议连接总增量为 **3.750／4.970 μs/请求**；应用端最大一步 fair → direct 为 **3.138／3.580 μs**，idle／作用域／让出组合为 **2.305／2.150 μs**，正式路由接入为 **1.821／1.963 μs**。这些包含实际功能与调用形态，不能当成纯组件成本或精确加总。

实际优化报告显示，原 NativeByteBuffer 锁包装消耗 134 个内联增长单位，回调因只剩 5 个单位而保留分配。将四个锁的共享加锁／解锁拆出后，包装增长为 77，回调成功展开；同步顺序与 finally 保留。正常请求在 middleware 为空时直接执行下一层，fallback group 仅在无端点、405 或默认 OPTIONS 时匹配。公开 API、编译器、runtime 和 GC 参数保持。

64 连接、默认 GC、独立绑核，预热 2 秒＋测量 8 秒，正反各一次，CPU 单位 μs/请求：

| 响应／阶段 | 单 worker CPU：前 → 后 | 四 worker CPU：前 → 后 | 吞吐变化：单／四 worker |
| --- | --- | --- | --- |
| 1 KiB 手写 TCP | 4.468 → 4.347（−2.7%） | 5.990 → 5.945（−0.8%） | +2.6%／+2.3% |
| 1 KiB 正式协议连接 | 8.130 → 7.847（−3.5%） | 10.873 → 10.332（−5.0%） | +3.5%／+9.8% |
| 1 KiB 完整 server | 16.182 → 15.226（−5.9%） | 19.795 → 18.915（−4.4%） | +6.2%／+6.5% |
| 64 KiB 完整 server | 29.015 → 27.950（−3.7%） | 32.615 → 31.382（−3.8%） | +3.8%／+3.5% |

完整 server 小响应吞吐 **61,858→65,723／183,744→195,727 req/s**，P80 **1,267.5→1,209.5／540.5→476.0 μs**；压测分配降至约 **11,172／11,244 bytes/请求**（−14.6%）。手写／协议分配下降约 29.3%～29.5%／21.3%；手写 RSS 高约 2.1%～3.6%，完整 server RSS 略降。独立单连接追踪中，完整 server 闭包对象 **89→54**，总分配 **13,639→11,727 bytes/请求**；这些计数与 64 连接负载分开解释，所有原始计数均对齐。

**76 个无仪表样本、127,079,052 个测量请求**无 wrk 错误，包括 36 个分层样本和 40 个组合样本。普通 release **1,171 项**、定向 moving／minor **34 项**通过，另核验 **4,312 个**网络响应及 **600 个**分配探针响应。新测试覆盖参数端点与更具体 fallback 分组共存的正常选路、405 和 404。全部测试、计数、采样与压测独立运行。

新 profile 共 **30,408 个**独立样本，server／core／http.asyncio 的 self 为 **19.26%／18.01%／12.40%**（单 worker）和 **18.61%／17.39%／11.46%**（四 worker）。四个 acquire helper 合计 **9.70%／10.00%**，最大单项约 3.4%；原主体此前展开在调用方，不能把新符号占比视为新增成本或锁争用。B57 的 core 分组曾误标为 concurrent，本轮已更正文字记录，原始样本和比例不变。

组合整体有确认收益，全部保留；本轮未做单项消融。完整小／大响应仍为 Go 约 **2.12～2.15／1.96～2.03 倍 CPU**。下一轮先用新底座校准协议与请求执行边界，再比较共享加锁、实际 I/O 和生命周期的生成调用及对象分配，不默认继续缩小旧闭包。详细结果、复现与限制见 [B58 记录](PROGRESS.md#第五十八轮缩小同步包装并精简正常请求分派)。

## B57 历史结果：保留三项通用成本优化

managed invoke 在机器栈映射中登记一个 root region table，替代逐次 push／pop 及线程 compiler-root 链；正常和异常后继继续从 GC 更新后的 canonical storage 恢复引用。同步闭包只对精确已知的 invoke，或仅通过局部别名同步调用该参数的 wrapper，给予普通增长上限内的展开优先级。源级 suspend 合并允许真实挂起、yield、EH 和 pending cleanup，按实际复制体积累计增长，保留递归／深度限制及既有完成协议。没有按 HTTP 函数名添加特例。

使用同一份 B52 手写起点、B52 `connection` 和 B53 三路径完整 server 源码，分别以两套配套工具和库重建。64 连接、独立绑核、TCP_NODELAY、默认 GC，预热 2 秒＋测量 8 秒，正反各一次；每格取该组两次中位数，CPU 单位 μs/请求：

| 响应／阶段 | 单 worker CPU：前 → 后 | 四 worker CPU：前 → 后 | 吞吐变化：单／四 worker |
| --- | --- | --- | --- |
| 1 KiB 手写 TCP | 5.160 → 4.451（−13.7%） | 6.907 → 6.049（−12.4%） | +16.0%／+18.6% |
| 1 KiB 正式协议连接 | 9.095 → 8.025（−11.8%） | 11.923 → 10.845（−9.0%） | +13.4%／+7.2% |
| 1 KiB 完整 server | 18.642 → 16.338（−12.4%） | 22.435 → 20.007（−10.8%） | +14.2%／+12.0% |
| 64 KiB 完整 server | 30.961 → 28.788（−7.0%） | 36.116 → 32.510（−10.0%） | +7.8%／+11.0% |

完整 server 小响应吞吐为 **53,579→61,211／161,797→181,267 req/s**，P80 为 **1,431.5→1,282.5／603.0→544.5 μs**，每请求分配下降约 **12%**；手写／协议层分配分别下降约 **19.8%／18.5%**。大响应分配下降约 **11.3%**，P80 下降约 **6.7%／9.8%**。手写程序 RSS 高约 1%～2.6%，完整 server RSS 低约 1%～2.8%，全部样本保留；分配量不等于存活量。三个 ELF 的 text 段分别缩小 **5.2%／1.9%／2.0%**。

**40 个无仪表样本、64,827,820 个测量请求**无 wrk 错误，六个程序另逐字节核验 **2,352 个**网络响应。五个库的 **912 项**普通 release 测试通过；MIR／LIR／slib 共 **863 项**通过，codegen 初次 400／410 通过、修正后相关 39 项复测全部通过。CLI／GC fixture 合计 **13 组、22 个变体、62 个进程**通过，新挂起组合另在 debug／release × moving／minor 四次通过。格式、clippy 和 C 警告检查完成；旧 MIR 快照漂移在 control 复现，版本固定向量与过时测试断言已修正，失败及闭合日志保留。

优化后单／四 worker 用户态 cycles 为 **6,904／24,193 个**独立样本，全部只计一次。旧 compiler-root push／pop 符号和热点已移除；最大的单个 HTTP 声明约 **2.82%／2.29%**。按声明的精确 source Cone 身份聚合 self，server 为 **19.82%／19.83%**、core 为 **18.03%／16.67%**、HTTP asyncio 为 **11.28%／10.87%**。这些组仍包含多个语义与实现成本，不能全视为可消除开销，也不能与 inclusive 树相加。下一轮以新底座重新测请求执行／分派及协议读写的组合增量，追踪分配来源；不继续默认打磨旧根维护或 Job 父子链表。

本轮只证明三项组合的整体收益，未做逐项消融。完整 server 小响应 CPU 仍为同轮 Go 的约 **2.29／2.29 倍**，大响应约 **2.10／1.98 倍**。没有重测 Gin／Axum、所有库或全部 fixture；历史 SSE 根因与 B51 原版协议四 worker 回退未据此关闭。固定目录为 `/home/chenxu/benchmarks/http-improvements-20261113-round57`；复现与验收边界见 [B57 记录](PROGRESS.md#第五十七轮保留根表闭包展开与挂起调用合并的组合收益)。

## B56 历史结果：请求作用域与跨层采样

使用固定 B52 `idle`／`scope`，复测总 CPU 增量为 **1.82／1.02 μs/请求**。从同一请求区域逐步接入现有 API，1 KiB、64 连接、默认 GC、独立绑核，预热 2 秒＋测量 8 秒，正反各一次：

| 相邻替换 | 单 worker 增量 | 四 worker 增量 |
| --- | ---: | ---: |
| `idle → binding`：重新绑定当前连接的 Job | +0.43 μs | +0.11 μs |
| `binding → detached`：原 `nonCancellableScope` | +1.81 μs | +1.43 μs |
| `detached → scope`：原 `coroutineScope`，恢复父子关系 | 持平 | +0.16 μs |

中间步骤只用于受控无正文请求，不能替代完整取消合同。四 worker 的 CPU 利用率与吞吐同时变化，绑定程序两个样本为 **297,231／249,929 req/s**，全部保留；不同组差额不能精确相加。单 worker 分配约 **7,162／7,210／8,651／8,731 bytes/请求**，不是存活量，也不能直接换算 CPU。

独立 1,000 GET 减零 GET 计数中，四阶段内部恢复为 **4、4、7、7**，外部恢复／drive 均为 **2／2**，RECV／SEND 均为 **1／1**。独立作用域每请求有一次 Job 完成和 Scope enter／close；父关联只在最后一步增加一次。Context 差分为 **−2／998／998／998** 次，控制窗口多出两次绑定，保留该边界偏移，不解释成负调用。探针另逐字节核验 **4,000 个**响应。

手写起点和 B53 完整 server 的单／四 worker 用户态 cycles 共 **51,994 个**独立样本。沿唯一调用边映射生成代码，再按 typed template identity 合并泛型实例；完整 server 的根登记 self 为 **9.07%／10.09%**，三个锁 helper 的单项占比约 **1.16%～2.44%**，仍没有一个 HTTP 函数解释主要差距。self 与 inclusive 不能相加；该排名当时只提供下一轮组合评估的依据，B57 的采纳另有完整实现、正确性验证与整体测量。

同轮手写／完整 server／Go 的 CPU 为 **5.15／18.61／7.20 μs**（单 worker）和 **6.95／22.27／8.79 μs**（四 worker）；完整 server 仍为 Go 的约 **2.58／2.53 倍**。单 worker 分配约 **2,302／14,863／2,221 bytes/请求**，具体类型和创建位置尚未重新追踪，不能沿用 B32 旧分布。**44 个无仪表样本、71,544,259 个请求**无 wrk 错误；两个新程序另核验 **784 个**网络响应。未重跑未改生产库的全套测试、大响应性能、Gin／Axum 或其他平台，历史未闭合项保持。

复现入口见 [作用域分层说明](../../tests/benchmarks/http-layers/README.md#继续拆分独立请求作用域)，完整边界见 [B56 记录](PROGRESS.md#第五十六轮拆分请求作用域并重新检查跨层成本)。固定目录为 `/home/chenxu/benchmarks/http-improvements-20261113-round56`；本轮没有另建生产 SDK 或生产候选，两个新程序使用 B53 原库。

## B55 结果：协议连接分项与候选撤回

从原 `encoder` 只将读／写函数的 AsyncTcpStream 参数改成现有 AsyncByteStream，CPU 仅增加 **0.09／0.07 μs**。随后把 B53 未修改的 `scoop.http.asyncio` 源码放到仓库外同一 Cone，四个模式共享一个 ELF，逐步替换消息状态、接收及发送。1 KiB、64 连接、默认 GC、独立绑核，预热 2 秒＋测量 8 秒，正反各一次，CPU 单位 μs/请求：

| 内部接入步骤 | 单 worker 增量 | 四 worker 增量 |
| --- | ---: | ---: |
| `codec → session`：原消息关联、输入／输出对象与完成 | +0.93 | +1.37 |
| `session → receive`：原输入泵、事件消费和读操作 | +1.14 | +1.25 |
| `receive → connection`：原数组响应、写操作和 flush | +0.96 | +0.98 |

内部模式包含实际调用与缓冲差异，手工部分只处理受控无正文请求，不具备完整取消／错误语义。两端分别校准：内部 codec 的 CPU 持平／高 1.8%，内部完整连接高 1.8%／0.1%。各相邻组不能精确相加，也不能把它们冒充单个对象或函数的成本。

试过保留 decoder 原批次、让连接仅缓存一条 lookahead 事件，省去将全部事件搬入第二个 ArrayDeque 的步骤。保持请求关联、消息顺序、取消、首字节计时与升级后缀。原版／候选相同源码的完整 server CPU 为 **18.72→18.00／22.50→22.27 μs（−3.8%／−1.0%）**；协议程序为 **9.05→8.92／11.60→11.73 μs**，其四 worker 吞吐 **287,289→277,491（−3.4%）**、P80 **340.5→358.5 μs**。**实现和新增测试已撤回，不把小幅完整服务收益当成主要瓶颈的证据。** 这些协议程序是逐层连接程序，不是 B51 原版协议 ELF。

候选普通 release `http.asyncio` **166**、`http.server` **167** 项通过，其中两项新测试覆盖批量混合正文／trailers／下一消息、informational／HEAD 与计时顺序。首次静态检查遗漏 Unit 返回，修正后通过；原失败日志保留。七个新程序／模式共核验 **2,744 个**分段／pipeline／并发大小 GET／HEAD 响应。**72 个无仪表样本、125,308,204 个测量请求**无 wrk 错误；没有做本轮 profiler／uprobes、大响应性能、完整客户端／Rust／CLI fixture、Gin／Axum 或其他平台验收。

同轮正式完整 server 与 Go 的 CPU 为 **18.46／22.30** 对 **7.18／8.70 μs**，仍约 **2.57／2.56 倍**。B51 协议四 worker 回退与历史 SSE 间歇故障根因仍未闭合。固定目录 `/home/chenxu/benchmarks/http-improvements-20261113-round55` 中，`source/local` 与 `bin/scoop-local` 是 B53 内部诊断；`candidate/sysroot`、`source/candidate`、`candidate.patch` 和 `*-after` ELF 是撤回材料，**不能作为下一轮正式底座**。传输补丁、内部入口和复现方式见 [基准说明](../../tests/benchmarks/http-layers/README.md#继续拆分协议连接接入)，完整记录见 [B55](PROGRESS.md#第五十五轮拆分协议连接成本并撤回事件队列候选)。

## B54 结果：应用分项与重新评估

先从原 `application` 派生三个公开组合：原始 target 的短路 middleware `direct`、改用 `ctx.request.path` 的 `parsed`、委托 `next()` 到原三条路由的 `routed`。`fair → direct` 仍增加 **4.02／4.57 μs**；目标解析增加 **1.78／1.33 μs**，后续路由与 `next()` 委托增加 **1.32／1.66 μs**。空委托 middleware 另有 **0.32／1.37 μs** 的对照；它不能从不同组中精确扣除，`direct` 也并非没有 middleware 成本。

随后将未修改的 server 源码放到仓库外同一 Cone，逐步接入原内部实现，六个模式共享一个 ELF。先校准完整应用路径，同 Cone 比预编译库的 CPU 低 **1.7%／1.9%**；因此内部相邻增量用于定位，绝对性能不能替代生产基线。1 KiB、64 连接、默认 GC、独立绑核，预热 2 秒＋测量 8 秒，正反各一次，CPU 单位 μs/请求：

| 内部接入步骤 | 单 worker 增量 | 四 worker 增量 |
| --- | ---: | ---: |
| 请求／正文视图、H1 适配器与清理 | +1.08 | +0.95 |
| Context、默认 deadline 与精确绑定 | +0.65 | +0.94 |
| `ctx.respond`、响应状态及进度 | +0.80 | +0.86 |
| 原执行器、短路 middleware 与共享编排 | +0.61 | +0.76 |
| 正式分派替换短路 middleware | +2.18 | +2.21 |

**本轮重新测量 B52 固定 `encoder → connection` ELF，CPU 为 6.38→9.09／8.55→11.50，即 +2.72／+2.95 μs，仍大于这些应用分项。** 后续从该区间继续逐步换入正文 reader／writer、消息与操作生命周期等组成。这里的 `connection` 是逐层程序，不是 B51 原版协议程序，不能用它关闭旧四 worker 回退。

同轮完整三路径 server 与 Go 的 CPU 为 **18.73／22.28** 对 **7.13／8.73 μs**，仍约 **2.63／2.55 倍**。**96 个无仪表样本、113,458,612 个测量请求**无 wrk 错误。三个新公开程序与六个内部模式共验证 **3,528 个**分段／pipeline／并发大小 GET／HEAD 响应；独立计数另验证 **6,000 个**。内部六模式恢复次数为 **8、9、10、13、14、16**，外部恢复／drive 均为 **3／3**，RECV／SEND 均为 **1／1**；计数不能直接换算成 CPU。

本轮没有生产实现改动，未重跑完整 Cone／Rust／CLI fixture，也未测大响应性能、Gin／Axum 或其他平台。固定目录为 `/home/chenxu/benchmarks/http-improvements-20261113-round54`；`source/local` 和 `bin/scoop-local` 仅作内部诊断，`candidate/sysroot` 是未修改的 B53 独立副本。源码、三个补丁及复现方式见 [基准说明](../../tests/benchmarks/http-layers/README.md#继续拆分应用请求执行)，完整边界见 [B54 记录](PROGRESS.md#第五十四轮拆分应用成本并重新选择协议连接层)。

## B53 已交付：应用执行可独立接入

新增公开 `HttpApplication.executeHttp1(connection, incoming, info, requestId, options)`，在调用方管理的 H1 连接上执行当前未消费请求。请求 Job、Context、body 视图、middleware／路由、错误映射、响应、期限和清理与完整 server 共用；`HttpRequestOptions` 可独立创建或配置到 `HttpServerOptions.request`。原平铺请求选项已直接迁移。借用入口不移交 transport，WebSocket 升级在提交前返回 400；完整 server 的升级与 H2 路径保持原合同。用法见 [库说明](../../sysroot/src/scoop.http.server/README.md)。

在原 `fair` 程序中逐步换入应用执行，再换成同一三路径完整 server。1 KiB、64 连接、独立绑核、默认 GC，预热 2 秒＋测量 8 秒，正反顺序各一次，CPU 单位 μs/请求：

| 相邻替换 | 单 worker | 四 worker |
| --- | --- | --- |
| `fair → application` | 11.76 → 18.09（**+6.33**） | 14.73 → 21.39（**+6.66**） |
| `application → server` | 17.82 → 18.13（+0.31） | 21.64 → 22.47（+0.83） |

最大区间仍包含请求／正文视图、应用分派、响应执行与清理，不能称为纯路由成本。公开挂起调用、caller 标识符与空正文收尾也属于实际接入差异；多个短对照不能精确相加。两端的内部恢复均为 **16 次／请求**，小响应 RECV／SEND **1／1**，大响应 **1／2**。计数与计时独立，不能按恢复次数分摊 CPU。

重构前后同一完整 server 源码的 1 KiB／64 KiB、单／四 worker 对照，吞吐 **−1.3%～持平**、CPU **+0.4%～+1.4%**。本轮三路径完整 server 同 Go 的 CPU 为 **18.78／22.55** 对 **7.18／8.78 μs**，仍约 **2.62／2.57 倍**。没有重新构建 B51 原版多路完整程序，也没有重测未修改的原协议程序；B51 协议回退仍未闭合。

普通 release 服务端 **167 项**、预编译消费者 **6 项**通过；两个新 ELF 逐字节验证 **784 个**网络响应，独立探针另验证 **4,000 个**。**40 个无仪表样本、40,156,024 个测量请求**无 wrk 错误。九个补丁可以逐字节重建各固定源码。详细数值、公开消费者修正与范围见 [B53 记录](PROGRESS.md#第五十三轮拆出应用请求执行并量出连接管理的净增量)。历史 SSE 间歇故障根因未修复，不称全仓绿。

固定目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round53`，其中 `candidate/sysroot` 是独立副本；新的应用与三路径完整 server 分别是 `bin/scoop-policy-application`、`bin/scoop-policy-server`。`fair` 与 `before` 链接 B52 固定 ELF。API 文档使用另行配套的当前源码工具；这些文档工具不替换 B34 性能工具链。

新发布主题 `http/existing-connection` 已从正式 JSON 还原并实际运行，示例阅读器 **9 项**测试通过；API 文档生成与 `scoop api search/show` 验证通过。规范、公开声明、真实调用、示例和 skill 库导航已同步，文档工具固定在 `B53/doc-tools/bin`。

## B52 渐进链基线：当时最大的完整请求执行组合区间

以 B51 固定 SDK 构建最简手写程序，顺序为 `manual → decoder → split → encoder → connection → idle → scope → fair → server`。仓库保留 [起点及复现说明](../../tests/benchmarks/http-layers/README.md) 和八个连续补丁；所有补丁均可重建出已经测量的固定源码。各阶段通过格式、静态检查、release 构建及 392 次真实网络验证。

1 KiB、64 连接、默认 GC、独立绑核、预热 2 秒＋测量 8 秒，每个相邻对照正反顺序各一次。关键 CPU 增量如下，单位为 μs/请求：

| 替换步骤 | 单 worker | 四 worker |
| --- | --- | --- |
| 正式解码 | +0.69 | +1.02 |
| 正式编码 | +0.35 | +0.36 |
| 协议连接 | +2.79 | +3.34 |
| 独立请求作用域 | +2.07 | +1.52 |
| `fair → server` 剩余职责 | **+7.32** | **+7.57** |

先测得 `idle → server` 为最大增量 **+8.82／+10.75 μs**，再在该区间补入请求作用域与公平 yield；最后一步仍最大，吞吐 **86,892→53,100／248,883→164,909 req/s**。它包含请求对象与 body 视图、应用分派／middleware、响应执行、头部计时、server 登记和收尾，**不能称为单独的路由成本，也不等于可全部消除的开销**。完整相邻表和波动边界见 [B52 记录](PROGRESS.md#第五十二轮从最简-tcp-http-server-逐步替换并定位组合边界)。

实际组合障碍已明确：codec 可以独立接入，连接可接已有 AsyncByteStream，但公开的 HttpApplication 执行仍只能随 HttpServer 从 listener 接入；HttpContext、ResponseOutput 和 H1 输出对 ServerState／ServerConnection 的依赖尚未拆开。本轮只建立诊断与证据，没有声称已改善这些生产接口。下一轮按真实 server 调用链拆清职责，继续逐步换入并测量。

独立计数中，小响应九阶段均为每请求 1 次 RECV、1 次 SEND；`fair → server` 的外部恢复和 execution drive 都保持 3，内部恢复 **8→16**。四个代表阶段的 64 KiB 输出也统一为 1 次 RECV、2 次 SEND；本轮没有计量大响应性能。恢复次数不能直接换算成 CPU 成本。

手写起点仅覆盖受控无正文 GET／HEAD、三个路径、分段输入和连续请求，CPU **5.09／6.94 μs**；不能当作完整 HTTP 性能。同轮 B51 原版完整 server 为 **18.10／22.36 μs**，Go 为 **7.17／8.71 μs**，仍相差约 **2.53／2.57 倍**。旧协议四 worker 两次吞吐 **245,318／226,620 req/s**；同一 `idle` ELF 跨组也有明显波动，所有样本保留，B51 协议回退仍未闭合。

本轮 **96 个无仪表样本、173,626,789 个测量请求**无 wrk 错误；九阶段网络验证 **3,528 个**、探针另逐字节验证 **13,000 个**响应。计数与性能测量独立运行，未重跑未改生产库的全套 Cone、CLI／Rust 或其他平台。固定目录 `/home/chenxu/benchmarks/http-improvements-20261112-round52` 的 `candidate/sysroot` 是指向 B51 的链接，**不能在该链接上覆盖重建**。

## B51 最新结果与收益边界

重新采集 B49 完整服务／协议程序的单／四 worker CPU cycles 后，完整服务的解码／桥接约占 13.09%／12.14%，操作／idle 约占 12.09%／11.64%。本轮按批次移交 native 事件、直接访问字段视图并复用不可变数组快照；同步开始／结束 H1 操作，合并准备、方向与 idle 状态。事件所有权、取消、超时、实际进度和清理均保留；没有修改编译器、runtime 或 GC。

完整服务与同次重建的 B49 原版对照，64 连接、默认 GC、独立绑核，正反顺序各一次、预热 2 秒＋测量 8 秒，中位数：

| 正文／worker | req/s（B49 → B51） | CPU μs/请求 | P50 μs |
| --- | --- | --- | --- |
| 1 KiB／1 | 46,843 → 54,976 | 21.36 → 18.20 | 1,205.5 → 1,011 |
| 1 KiB／4 | 142,151 → 165,450 | 26.04 → 22.22 | 367 → 311.5 |
| 64 KiB／1 | 28,741 → 32,145 | 34.82 → 31.15 | 2,015.5 → 1,820.5 |
| 64 KiB／4 | 89,298 → 99,645 | 40.16 → 36.41 | 592.5 → 536.5 |

包含双 worker 的完整矩阵中，小响应吞吐 +16.1%～17.4%、CPU −14.0%～14.8%；大响应吞吐 +9.9%～11.8%、CPU −9.1%～10.5%，P50／P80 与中位 RSS 均改善。完整服务小响应仍需 Go 约 **2.54 倍 CPU**，大响应约 **2.26～2.27 倍**，长线目标尚未完成。两项实现一起测量，没有当前底座上的四种组合对照，不能分别归因或声称已证明协同作用。

**协议程序四 worker、小响应有回退，不能称所有路径都加速。** 初次对照后又补一组正反顺序，四次全部纳入：吞吐中位数 **249,873→240,622（−3.7%）**，CPU **14.90→13.15 μs（−11.7%）**，P50 203→178.5 μs，P80 **389→428 μs（+10.0%）**。原版吞吐范围 247,165～253,575，候选 233,702～250,824。它伴随默认 GC 并行度和暂停变化，未独立证明因果，也未调 GC。协议单 worker、小响应 CPU 则从 12.71 降至 9.92 μs，大响应单／四 worker 从 27.56／31.70 降至 24.59／28.75 μs。

独立 1,000 GET 减 0 GET 探针：普通 HTTP native 调用每请求 **20→2**，另有 **9 次 GCLeaf**；内部恢复 **21→16**，drive／外部恢复仍各 3，RECV／SEND 仍各一次。最终 release 验收 `http` 313、`http.asyncio` 164、`http.server` 160、`http.client` 254、server consumer 6，共 **897 项通过**；另逐字节核验 **6,608 个网络响应**，两个 C 诊断工具构建及四种 codec 检查通过。旧客户端产物按新依赖正常重建；首次客户端测试因脚本总时限 45 秒过短中断，按源码中的多组 15 秒等待调整到 900 秒后完整通过，原日志保留。

最终候选 **32,281 个独立 cycles 样本**中，解码／桥接为 **5.60%／5.13%**、操作／idle **8.42%／7.28%**；剩余请求／连接 **20.74%／21.19%**、路由／middleware **13.44%／12.78%**。最大单函数归属仍只有约 4%，应按下一轮逐层替换实验定位实际增量，不能把这些组全部视为可消除成本。

详见 [B51 记录](PROGRESS.md#第五十一轮组合事件桥接与操作管理并确认收益边界)。**52 个无仪表样本、53,525,442 个测量请求**无 wrk 错误；计数、采样、测试均独立运行。正式目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round51`，SDK 为 `candidate/sysroot`，完整／协议 ELF 为 `bin/scoop-policy-combined`／`bin/scoop-policy-protocol-combined`。该目录中的 `scoop-full-after` 等旧别名仍指向 B49，不能误用。

## B50 已排除：请求目标解析复用

先复测 B49 完整服务、协议程序和 Go：1／4 worker、64 连接、1 KiB，每请求 CPU 分别为 **21.46／26.07**、**12.97／14.73**、**7.17／8.74 μs**。完整服务与协议程序仍差约 **8.49／11.34 μs**。复核 B49 指令栈时，目标解析约占 7.45%，其中路径解码约 4.18%；两者嵌套，不能相加，也不能直接当作可消除的 CPU 比例。

完整实现过 H1 连接内按原始方法和 target 字节复用最近成功的不可变解析结果。保持惰性解析时机、严格文本访问，以及每请求的字段、正文、middleware、路由和生命周期。与同次重建的 B49 原版正反顺序对照：

| 负载／worker | req/s（原版 → 候选） | CPU μs/请求 |
| --- | --- | --- |
| 重复 URI／1 | 47,636 → 49,001 | 21.01 → 20.41 |
| 重复 URI／4 | 140,694 → 150,933 | 26.16 → 24.60 |
| 每次新 query／1 | 43,686 → 44,081 | 22.92 → 22.75 |
| 每次新 query／4 | 131,679 → 129,985 | 27.76 → 28.11 |

重复 URI 的 CPU 仅下降 **2.8%／5.9%**；无法复用时基本持平或略退。独立计数已确认每 1,000 GET 的目标解析和路径解码都从 1,000 次降到 2 次，剩余两次来自测量标记切换 URI；实际消息开始、RECV、SEND 仍各 1,000。因此不是候选未命中造成小收益，**实现、规范和新增测试均已撤回，不再细调这一方向**。

候选格式、静态检查与 HTTP Server release **162/162** 测试通过。本轮 28 个无仪表样本、29,567,866 个测量请求无 wrk 错误，探针另逐字节核验 2,000 个响应。详见 [B50 记录](PROGRESS.md#第五十轮重新比较请求成本并撤回解析复用候选)。原始目录 `/home/chenxu/benchmarks/http-improvements-20261112-round50` 的 `candidate/sysroot`、`candidate.patch`、`source/candidate/` 和候选 ELF 都是撤回材料，不能作为下一轮底座。

## B49 已交付的响应复用

同一不可变字节 HttpResult，在没有响应修改、beforeCommit 或压缩策略的 H1 请求中，按版本和方法复用最近一个纯计划；编码器按相同 head、方法快照和定界复用成功的头部字节。handler／middleware、响应认领、消息长度状态、实际 I/O、关闭判断和清理仍逐请求执行；其他准备条件走原流程。两份 HTTP spec 已同步，没有修改编译器、runtime 或 GC。

完整服务、64 连接、默认 GC、独立绑核，正反顺序各一次，预热 2 秒＋测量 8 秒，中位数：

| 正文／worker | req/s（B45 → B49） | CPU μs/请求 | P50 μs |
| --- | --- | --- | --- |
| 1 KiB／1 | 39,520 → 45,975 | 25.32 → 21.73 | 1,686.5 → 1,199.5 |
| 1 KiB／4 | 120,433 → 142,526 | 30.05 → 26.04 | 425.5 → 366.5 |
| 64 KiB／1 | 26,324 → 28,728 | 38.06 → 34.81 | 2,194 → 2,018.5 |
| 64 KiB／4 | 80,325 → 90,139 | 44.24 → 40.21 | 657 → 590.5 |

包含双 worker 的完整矩阵中，小响应吞吐 +14.7%～18.3%、CPU −12.7%～14.2%；大响应吞吐 +9.1%～12.2%、CPU −8.5%～9.1%。每请求新建 HttpResult 的单／四 worker 对照基本持平，没有明显回退。协议连接小响应 CPU 从 14.93／16.77 降至 12.88／14.80 μs。**完整服务小响应仍需 Go 约 3 倍 CPU，大响应约 2.4～2.5 倍，不能视为长线任务完成。**

独立探针中，每 1,000 个已暖静态 GET 的通用准备入口 1,000→0、头部校验编码 1,000→1，消息开始、RECV 和 SEND 仍各 1,000。最终普通 release 验收 `http` 309、`http.asyncio` 164、`http.server` 160，共 **633 项通过**；并发大小／GET／HEAD 交替另逐字节验证 **4,608 个响应**。初次 SSE 生命周期故障在 B45 原版也复现；隔离构建负载后，基线和候选完整 server 套件各复跑 3 次均通过，原失败保留，未修复其根因。

最终候选的 **6,953 个独立指令样本**中，响应准备 0.73%、编码 0.56%，已不支持继续细磨这两层。最大的剩余 HTTP 分组是请求／连接执行 **18.35%**，随后是操作／idle **13.03%**、路由／middleware **11.66%**；请求组内 `executeRequestPipeline`／`serveHttp1` 分别约 3.54%／3.41%，没有一个占据主要差距的单函数。GC 栈 12.73% 只作解释；指令占比不等于 CPU 占比或可完全消除的成本。

详见 [B49 记录](PROGRESS.md#第四十九轮复用不可变响应准备与编码并重新定位剩余成本)。52 个无仪表样本、48,958,855 个测量请求无 wrk 错误；探针及采样与性能测量分开。原始目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round49`，其 `candidate/sysroot`、`bin/scoop-full-after` 和 `bin/scoop-protocol-after` 是 B51 的前版对照。

## B48 评估：分派、响应准备与实际指令工作量

先分别省去目标解析／路由／middleware，以及响应 draft／回调／通用正文准备，再测两组合并。保留请求作用域、截止时间、收尾及协议操作；这些固定静态路由的程序缺少其他必要语义，只作成本上界，不能交付。64 连接、1 KiB、默认 GC、独立绑核，正反顺序各一次，预热 2 秒、测量 8 秒：

| 路径 | 1 worker CPU μs/请求 | 4 workers CPU μs/请求 |
| --- | ---: | ---: |
| B45 原版 | 25.97 | 30.14 |
| 省去分派 | 22.80 | 27.12 |
| 省去响应准备 | 23.99 | 28.17 |
| 两组同时省去 | 21.10 | 25.09 |
| Go net/http | 7.17 | 8.72 |

分派上界约 3.2／3.0 μs，响应准备约 2.0／2.0 μs；两组同时省去仍需 Go 约 2.9 倍 CPU。因此没有继续写静态路由捷径或 draft 缓存候选。

独立单 worker 硬件计数显示，差距主要表现为执行指令过多：

| 路径 | 用户态指令/请求 | 用户态 cycles/请求 | IPC |
| --- | ---: | ---: | ---: |
| B45 完整服务 | 232,286 | 96,600 | 2.40 |
| 省去分派与响应准备 | 194,100 | 79,092 | 2.45 |
| B45 协议连接 | 127,234 | 52,045 | 2.44 |
| Go net/http | 39,471 | 17,220 | 2.29 |

这是用户态计数，不能与含内核态的总 CPU 倍率等同；计数窗口覆盖负载启动和少量尾部工作。各项 core counter 运行比例为 100%，分支失误率约 0.36%～0.64%。IPC 接近不排除局部停顿，但没有证据把整体数倍差距归于分支预测或代码布局。

正式完整服务／协议程序另有 7,057／6,985 个独立指令样本。最近 HTTP 边界互斥归类中，完整服务请求与连接管理约 16.0%，协议操作／idle 11.4%，响应编码 11.2%，分派 9.6%。请求管理内最大单函数归属仅约 3.1%；没有一个已证明可消除的单点占据大部分工作。正文已有批量内存复制；字节构造器虽有逐元素工作，B46 的响应编码 CPU 整组也仅约 7.2%，未据此实施局部优化。GC 栈约 11.9% 指令，只作解释，不启动 GC 支线。

最后将**原版 server 与 HTTP 连接库源码放进同一 Cone**，以既有编译器测量当前库边界对普通内联的影响。无删减功能，格式、静态检查、构建及大小正文核验通过；完整服务交替对照的单／四 worker 吞吐仅 **+0.55%／+0.17%**，CPU **−0.46%／+1.66%**。未采纳库合并，也不将此结果推广为所有跨库优化都无效；该诊断未实现跨库挂起 CFG 合并。

详见 [B48 记录](PROGRESS.md#第四十八轮评估分派与响应成本并定位指令工作量)。28 个无仪表样本、25,840,140 个测量请求无 wrk 错误；硬件计数和指令采样单独保存。原始目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round48`。`source/dispatch|response|both/` 是删减语义的上界诊断，`source/unified/` 与当前 `candidate/sysroot` 是同 Cone 诊断；都不是生产底座。

## B47 评估：成组定位后撤回操作生命周期候选

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

以上是 B47 的单项结论。B51 后来将完整操作实现与事件桥接组合，在 B49 底座上重新测量并采纳，见本文最新结果。

## B46 归因与事件桥接结论

同机重测 B45／Go，64 连接、1 KiB 正文、默认 GC、独立绑核，正反顺序各一次、预热 2 秒＋测量 6 秒：

| worker | 完整 B45 req/s／CPU μs | B45 协议连接 req/s／CPU μs | Go net/http req/s／CPU μs |
| --- | --- | --- | --- |
| 1 | 39,415／25.39 | 67,804／14.77 | 139,217／7.17 |
| 4 | 120,316／30.67 | 216,154／16.75 | 429,331／8.80 |

完整服务／协议程序各取得 6,734／6,194 个独立用户态 CPU 样本。按最近 HTTP 边界互斥归类，完整服务的请求执行与连接管理约 16.3%，但该组最大独立函数归属仅约 3.3%；解码与 native 桥接约 11.8%，HTTP/1 操作生命周期与 idle 约 10.6%，路由及 middleware 约 9.9%，响应编码约 7.2%。GC 栈约 11.4%，只作样本解释，不启动 GC 支线。原生栈与内联限制、实际分组脚本均保留；这些粗分组不能与 inclusive 子树相加，也不是可全部消除的成本。

因此试过按批次接管 native 事件、直接读取字段视图、保留已有不可变数组快照，并只给有界无分配 getter／批次取出使用 GCLeaf。空 Trailers、分段输入、消息顺序与关闭后的已返回数据仍保留。独立 1,000 GET 减零 GET 的计数确认普通 native 调用由 20 次降到 2 次，另有 9 次 GCLeaf；RECV／SEND 均保持一次。**完整服务前后交替实测只有单／四 worker 吞吐 +2.6%／+3.7%，CPU −2.9%／−6.4%；单 worker P50 还回退 6.3%。候选已撤回，不再细调桥接。** 本轮没有采纳新的 HTTP 性能实现。

详细数值、样本波动、验证范围见 [B46 记录](PROGRESS.md#第四十六轮重新归因并撤回低收益事件桥接候选)。原始目录为 `/home/chenxu/benchmarks/http-improvements-20261112-round46`，其中 `candidate.patch` 和 `source/` 是撤回候选；`profiles/` 是 B45 正式底座采样；`summary.json` 汇总 20 个样本。该轮未改变正式实现，后续已由 B49 推进。

B51 后来在 B49 上测量并采纳事件桥接与操作管理的组合；B46 这组历史数值不能用于单独解释组合收益。

## 用户明确的优先级

- 优先主路径、架构和数倍差距。完成 TCP 主线后，当前只推进 HTTP 协议／连接／请求执行层。
- 先解决静态内容，JSON 和其他序列化暂时不动。当前关注吞吐、每请求 CPU、内存、P50／P80，不把 P99 当主要目标。
- 用户已叫停 GC 支线。保留默认 GC 进行公平测量；GC 计数可以解释样本，不能因此又转向阈值、nursery 或 GC worker 调参。
- 正确性通过且整体有可确认收益的改动应保留，即使只有几个百分点；通用编译／执行路径的价值也要考虑。每轮重新排序成本，不反复打磨同一局部分支；没有单一大头时，按已知成本从大到小组合优化主要几项。不能用取消作用域、超时、清理或协议语义换取表面吞吐。
- B51 结束后，先用直接 TCP 的最简手写 HTTP server 建立基线，再逐步替换现有各层；以实际增量找最大瓶颈，同时改善功能正交性和可组合性。
- 每轮在该分支先提交，再 fetch 并 merge 远端 `main`，随后推送。先完整实现一项，再集中格式化、检查和必要测试，不逐个小修改跑大回归。
- 源码／spec／测试遵守仓库 [AGENTS.md](../../AGENTS.md)。输出中文，代码标识符和注释英文。
- 原安装目录 `~/.scoop` 和历史固定二进制保持。新候选、cache、日志与原始 JSON 都在仓库外；完成后清本轮不再需要的 cache／target，不动依赖底座或仓库原有大 target。
- 重建编译器时必须使用从源码构建并带项目私有补丁的 LLVM；参考 [scripts/README.md](../../scripts/README.md) 和打包流程。不能替换为发行版 LLVM。下载使用 `https_proxy=http://192.168.2.106:10809`。
- `scripts/package_linux.py` 和 `scripts/tests/test_package_linux.py` 是用户已有的未跟踪文件，不属于本任务，不修改、不删除、不提交。

## B44 分层诊断保留的结论

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
- B34：旧固定 CLI 已含可组合异常状态及完整 EH 内联，曾修复多层 catch/finally 覆盖异常记录的真实错误。B57 配套 CLI 继承这些能力并加入本轮三项优化；固定 LLVM 仍来自 B28 源码构建及私有补丁。
- B36：完整字节响应已能合并 HTTP 写操作；B45 在此基础上进一步去掉中间 buffer。
- B42（`f27fa2d97`）：词法 coroutineScope 保留独立 Job，但复用当前 execution；Job 状态／监听统一同步。当时每 GET execution drive 7→3、外部恢复 5→3，内部恢复仍 25。B51 新计数为 B49 原版内部恢复 21→候选 16，drive／外部恢复保持 3；不要继续把 25 当作当前基线。

## 不应重复投入的方向与未闭合问题

| 已做的调查 | 结论／处理 |
| --- | --- |
| B35 消除大多数同步闭包 | 当时诊断性成本豁免已撤回。B57 改为仅优先展开精确已知且同步使用的闭包，并保持普通累计增长上限，随组合验证后采纳；不沿用旧百分比。 |
| B37 合并头部计时与读操作 | CPU 下降但吞吐／P50 回退，未采纳。去掉头部和请求作用域仅作为上界，破坏语义，不能交付。B45 是连接级复用计时通知并消除中间 buffer，不能与该候选混淆。 |
| B38 同 Cone 挂起 CFG 合并 | 旧单项候选当时撤回。B57 修正实际复制体积的增长计算，补齐真实挂起、EH、pending cleanup 与 GC 组合验证后随三项组合采纳。 |
| B40 去除逐次根登记 | 旧单项候选当时撤回。B57 删除旧线程 compiler-root 链，以机器栈表保活 invoke 根，补齐移动 GC／异常／callback 验证后随组合采纳。 |
| B43 GC 支线 | 用户叫停，仅仓库外诊断；不要使用其候选 sysroot，不继续调 GC。 |
| B50 请求目标解析复用 | 解析入口几乎全部消除，但重复 URI 的 CPU 仅下降 2.8%／5.9%；每次新 query 基本持平或略退。候选已撤回，没有新证据不重试。 |
| B51 协议四 worker、小响应 | 四次全部纳入后吞吐 −3.7%、P80 +10.0%，同时 CPU −11.7%；默认 GC 行为变化随样本保留，未修复或证明因果。B52 同轮参照仍有明显波动，没有闭合此回退，不调 GC 掩盖问题。 |
| 默认 GC 的协议程序慢档 | B42 已观察到四 worker 偶发低吞吐，全部成功样本应保留。不能只挑中位数好看的样本，也不能把暂停／测量窗口比当成 GC CPU 占比。 |
| 旧编译器测试漂移 | B57 在同源码 control 复现六个 MIR 函数编号快照漂移并更新，另修正现有 192-byte image 的过时测试断言；本轮 ABI／metadata 固定向量已同步。B57 只关闭实际覆盖项，不代替全仓验收，原始失败日志保留。 |
| B49 验收中的 SSE 生命周期故障 | 并行构建期间，候选和 B45 原版均在同一测试发生内存增长；隔离后两者各 3 次完整 server 验收通过。原始失败保留，未修复根因，不能把一次通过当作故障已解决。 |

## 接手后优先做什么

用户要求**每轮重新评估最大瓶颈，并通过逐步替换同时改善功能正交性和可组合性**。B53 已补入独立应用执行，B54～B56 已细分应用、协议连接与请求作用域；B57～B60 已保留四轮共享成本的组合优化，小幅收益及其代价均按实际结果记录。

1. 以 B60 SDK、B57 配套 CLI 和本轮同路径重建的三个候选程序为正式参照。B60 开始时的相邻差额只用于选择复测边界，不能当成优化后仍未变化的成本；先在新目录重建协议接入与请求执行／分派的相关中间程序，重新排序。
2. 新 self 中 server 约 19.8%～20.1%、core 约 16.2%～16.3%、HTTP asyncio 约 11.8%～13.5%；最大单项约 3%。重点比较 receiveH1Request、executeHttp1Request、serveHttp1 中的操作准备、请求数据构建、读写与收尾。四个 acquire helper 合计约 8.0%～8.3%；B59 已消除 Job 只读查询的加锁，B60 又合并了期限初始化的临界区，不沿用旧次数推算可消除 CPU。新独立追踪中普通对象约 6,823 bytes、闭包 1,856、frame 1,616、adapter 1,032；总分配较 B59 少 344 bytes/请求。时间源码已降至约 1%，不默认继续打磨 Duration 或已删除的包装。继续结合 CPU、调用与分配证据选择主要组合。
3. 继续从最简 TCP 程序逐步接入既有实现，必要时用同 Cone 诊断拆边界并校准两端。没有单一大头时，从主要几项组合优化着手，以完整原版 server、协议连接、手写起点及 Go 的同轮对照决定保留；整体有确认收益即保留，不因旧单项幅度小而排除。需要单项归因时另做消融，不能相加历史百分比。
4. 保持 workload、传输形态、默认 GC、绑核和计时一致；保留全部成功样本，计数／profiler 与无仪表压测分开。保留请求 Job、middleware Context、HEAD、未读正文、取消、期限、真实输出进度与清理，并覆盖共享实现对 H2／WebSocket 的影响。真实接入需求才支持继续拆 API，不为实验公开内部构造器或建立平行生产框架。

B57 根登记热点已移除，B58 已减少每请求 35 个闭包，B59 消除了 Job 只读查询的加锁，B60 再减少六个闭包并简化时间运算，不再把旧分布当作默认重点；Job 父子关系在 B56 也不是主要增量。B50 解析复用和其他撤回候选保持各自历史结论，不省去必要语义，不启动 GC、P99 或 JSON 支线。

## 固定产物和复现入口

以下路径都在当前机器，原始日志不在 Git 中。**不要在这些目录上直接重建覆盖，新的实验另建目录。** B60 的 baseline／toolchain／tools 和多轮 bin 指向历史固定资源，不能删除其目标。

| 用途 | 路径 |
| --- | --- |
| B60 组合优化、分层、分配诊断、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261113-round60` |
| 当前正式 SDK／配套 CLI | `B60/candidate/sysroot`、`B60/toolchain/bin`（经 B59 链接 B57；runtime 35／metadata 12） |
| B60 同路径 control／candidate ELF 与符号 | `B60/bin/scoop-<variant>-<stage>`、`B60/symbols-<variant>-<stage>` |
| B60 前版 SDK | `B60/baseline/sysroot`（链接 B59 正式 SDK） |
| B59 组合优化、分层、锁与分配诊断、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261113-round59` |
| B59 历史正式 SDK／配套 CLI | `B59/candidate/sysroot`、`B59/toolchain/bin`（链接 B57；runtime 35／metadata 12） |
| B59 同路径 control／candidate ELF 与符号 | `B59/bin/scoop-<variant>-<stage>`、`B59/symbols-<variant>-<stage>` |
| B59 前版 SDK | `B59/baseline/sysroot`（链接 B58 正式 SDK） |
| B58 组合优化、分层与分配诊断、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261113-round58` |
| B58 历史正式 SDK／配套 CLI | `B58/candidate/sysroot`、`B58/toolchain/bin`（链接 B57；runtime 35／metadata 12） |
| B58 同路径 control／candidate ELF 与符号 | `B58/bin/scoop-<variant>-<stage>`、`B58/symbols-<variant>-<stage>` |
| B58 前版 SDK | `B58/baseline/sysroot`（链接 B57 正式 SDK） |
| B57 组合优化、全部原始结果与采样 | `/home/chenxu/benchmarks/http-improvements-20261113-round57` |
| B57 历史正式 SDK／CLI | `B57/candidate/sysroot`、`B57/toolchain/bin`（runtime 35／metadata 12） |
| B57 同源码前版 SDK／CLI | `B57/control/sysroot`、`B57/control-tools/bin`（runtime 34／metadata 11） |
| B57 三层 control／candidate ELF 与符号 | `B57/bin/scoop-<variant>-<stage>`、`B57/symbols-<variant>-<stage>` |
| B56 作用域诊断与原始结果 | `/home/chenxu/benchmarks/http-improvements-20261113-round56` |
| B55 协议诊断与撤回候选记录 | `/home/chenxu/benchmarks/http-improvements-20261113-round55` |
| B54 应用分项、协议复测、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261113-round54` |
| B54 内部诊断源码／ELF | `B54/source/local`、`B54/bin/scoop-local`（不是生产库候选） |
| B53 应用接入、脚本、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261112-round53` |
| B53 历史 SDK | `B53/candidate/sysroot` |
| B53 应用／三路径完整 server ELF | `B53/bin/scoop-policy-application`、`B53/bin/scoop-policy-server` |
| B53 前版／fair 参照 | `B53/bin/scoop-policy-before`、`B53/bin/scoop-policy-fair`（链接 B52 固定产物） |
| B52 渐进诊断、脚本、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261112-round52` |
| B52 九阶段源码／ELF／symbols | `B52/inputs/<stage>/server.scoop`、`B52/bin/scoop-policy-<stage>`、`B52/symbols-<stage>` |
| B51 性能底座产物、脚本、原始结果 | `/home/chenxu/benchmarks/http-improvements-20261112-round51` |
| B51 性能基线 SDK | `B51/candidate/sysroot` |
| B51 原版完整／协议 ELF | `B51/bin/scoop-policy-combined`、`B51/bin/scoop-policy-protocol-combined` |
| B51 前版 SDK | `/home/chenxu/benchmarks/http-improvements-20261112-round49/candidate/sysroot` |
| 同次重建的前版完整／协议 ELF | `B51/bin/scoop-policy-control`、`B51/bin/scoop-policy-protocol-control` |
| B34 历史固定 CLI | `/home/chenxu/benchmarks/http-improvements-20261112-round34/candidate/cargo/release` |
| 源码 LLVM 22.1.8 SDK | `/home/chenxu/benchmarks/http-improvements-20261112-round28/candidate/llvm/installed` |
| C compiler／headers 包装 | `/home/chenxu/benchmarks/http-improvements-20261112-round29/candidate/cc-with-headers` |
| B44 分层诊断及结果 | `/home/chenxu/benchmarks/http-improvements-20261112-round44` |
| 固定 Go net/http | `/home/chenxu/benchmarks/http-20261111/bin/nethttp-server` |

表中 B59／B58／B57／B56／B55／B54／B53／B52／B51／B49 表示各自完整根目录。B59 的 `candidate.patch` 对应 `892e34e28` 上的库及测试变更；`build_candidate.sh`、`validate_candidate.sh`、`validate_remaining.sh`、`measure_combined.py`、`measure_large.py` 保存实际构建、测试修正后的继续验收与测量命令。`prepare_lock_sites.py`、`profile_locks.py`、`summarize_locks.py` 记录实际 ELF 的入口／CAS／调用方，分配和 cycles 脚本继续保留。结果在 `measurement-report.json`、`validation-report.json`、`lock-comparison.json`、`allocation-comparison.json`、`profiles/` 和 `diagnostics/`。原 `scoop-<stage>` 链接用于开始时的分层诊断，正式新对照须用带 `control-`／`candidate-` 的程序。B58 原始报告与 `ef2223165` 上的候选补丁继续保留。

B57 的 `inputs/` 保留两套构建共用的三个源码，`patches/candidate.patch` 对应以 `459ccd13f` 为底座的已测编译器／runtime 变更；CLI 的版本字符串记录该构建底座，具体候选以此补丁和 manifest 区分。原构建／验收脚本保留实际命令，临时 worktree 和 cache 在收尾清理；复用时先在新目录恢复对应源码并调整路径，不覆盖固定 SDK、ELF 或结果。

B53 的 `build_library.sh`、`build_stage.sh`、`validate.sh`、`verify_stage.py` 用于库、程序和普通 release 验证；`measure_step.py`、`measure_large.py`、`measure_reference.py` 保存当时对照入口，`count_stage.py` 保存独立计数；`summary.json`、`measurement-report.json` 和 `verification/patches.json` 保存全部结果。

B52 目录内：

- `build_stage.sh`、`verify_stage.py`：各阶段真实构建和分段／pipeline／并发 GET／HEAD 验证入口。复用时先复制并改到新目录，避免覆盖固定产物；`candidate/sysroot` 仅链接 B51 SDK。
- `measure_start.py`、`measure_step.py`、`measure_reference.py`、`run.py`、`latency.lua`：起点、相邻替换和结束参照；原始结果在 `results/`，同名目录拒绝覆盖。
- `count_stage.py`、`diagnostics/`：同连接 1,000 GET 减 0 GET 的独立恢复／SQE 计数，逐字节验证正文。
- `summarize_steps.py`、`summarize.py`、`step-summary.json`、`summary.json`、`measurement-report.json`、`manifest.json`：全部样本的相邻增量、汇总、计数、验证及范围；`verification/patches.json` 保存连续补丁的复现结果。
- `inputs/`、`bin/`、`symbols-*`、`logs/`、`verification/` 和清理记录继续保留；九阶段名称见上文。`scoop-policy-full`／`scoop-policy-protocol-prior` 是指向 B51 正式 ELF 的参照链接。

B51 目录内：

- `build_combined.sh`、`build_client.sh`、`validate.sh`、`validate_native.sh`：本轮真实构建／验收命令。先复制并调整到新目录再用；构建脚本会从当前仓库复制源文件。客户端测试包含多组长截止时间，使用 900 秒总时限。
- `inputs/full_server.scoop`、`inputs/protocol_server.scoop`、`inputs/fresh_server.scoop`：完整服务、协议和每次新建响应的固定工作负载。
- `screen_combined.py`、`extend_combined.py`、`confirm_protocol.py`、`run.py`、`latency.lua`：测量入口；每个样本在 `results/<配置>/result.json`，另有原始 wrk、stdout／stderr。路径已存在时拒绝覆盖，重跑要用新目录或新的 mode。本轮未测 fresh 工作负载。
- `verify_http.py`：1／2／4 worker、每组 32 客户端、大小与 GET／HEAD 交替的逐字节核验。
- `count_combined.py`、`diagnostics/count-summary.json`：同连接 1,000 GET 减 0 GET 的独立 HTTP 桥接、协程和 SQE 计数，使用各自 ELF／symbols。
- `logs/test-*.log`、`validation-artifacts.json`、`bin/tests-*`：897 项验收及保留的五个测试 ELF。`check-tests-consumer-stale-client.log` 与 `test-scoop.http.client-timeout45.log` 保存已闭合的产物／脚本失败。
- `manifest.json`、`summary.json`、`measurement-report.json`、`candidate.patch`、`source/`：产物来源、52 样本汇总、实现补丁和源码快照；`verification/`、`diagnostics/` 保存正文核验及计数，`cleanup.json` 记录清理范围。
- `symbols-full-combined`、`symbols-protocol-combined`、`profile_cycles.py`、`attribute.py`、`partition.py` 与 `profiles/scoop-policy-combined-w*-small/`：最终候选的 cycles 归因。`scoop-after`／`protocol` 采样与对应旧别名属于 B49。注意内联和原生栈限制，不把子树与 self 重复相加。

快速复测固定前后产物的例子（先把 `bench_next` 换成未使用目录）：

```sh
bench_prior=/home/chenxu/benchmarks/http-improvements-20261113-round60
bench_next=/home/chenxu/benchmarks/http-handoff-check
mkdir "$bench_next"
cp "$bench_prior/run.py" "$bench_prior/latency.lua" "$bench_next/"
ln -s "$bench_prior/bin" "$bench_next/bin"
ln -s "$bench_prior/tools" "$bench_next/tools"
cd "$bench_next"
python3 - <<'PY'
import run
for repeat in range(2):
    variants = ("scoop-control-server", "scoop-candidate-server")
    for variant in variants if repeat == 0 else reversed(variants):
        run.run_sample(variant, 1, "small", seconds=8, warmup=2,
                       repeat=repeat, mode="handoff-check")
PY
```

脚本服务核为 `0,2,4,6` 的前 N 个，客户端核为 `8,10,12,13,14,15,16,17`，端口 `18081`；worker 不是进程总线程数，统计包含进程全部线程 CPU。另机必须重新核对 CPU 拓扑。性能跑批期间不要编译、跑测试或开 profiler；所有样本前后核验正文，wrk 本身不逐个核验正文。闭环 P50／P80 未校正 coordinated omission，短测两次不代表长期分布。

初始需求的 Gin／Axum、多负载和 GC 全矩阵仍属于长线 benchmark 范围，最近几轮有意只用静态 HTTP 与 Go 参照定位主瓶颈；不要把当前交接当作原始完整 benchmark 已完成。
