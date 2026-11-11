# TCP echo 分层诊断

这个基准用相同 TCP 工作负载区分 native 后端、语言／FFI、managed I/O 操作机制和完整协程路径。它用于判断架构改动应落在哪一层；薄层版本没有完整异步 API 的语义，不能直接当作产品替换方案。

| 层次 | 保留内容 | 相对完整路径去掉的工作 |
| --- | --- | --- |
| C native | 仓库的 Linux io_uring、TCP 请求、完成批次和真实退休实现 | Scoop runtime、managed 操作、协程 |
| 薄 Scoop | 同一 native 后端、Scoop 线程／GC 附着、普通 FFI、连接状态机、保活 buffer 的生命周期借用 | 通用 LoopOwner／LoopOperation、每次借用、Job 取消登记、协程执行 |
| managed 回调 | 同一 AsyncRuntime、每连接 Job、真实 TcpTransfer、每次 buffer 借用、取消订阅、owner 登记、ready 和关闭路径 | 每次读写的编译器协程挂起／恢复、executor 投递；用保存的真实 Job 代替 ambient Context 查询 |
| 完整 Scoop | 原 `server.scoop` 的公开异步 API 和完整协程路径 | 无 |
| Go | 原 `server.go` | 独立对照，其网络后端不同 |

`callback.scoop` 为每次传输创建独立的单次 Continuation，不复用已经完成的 continuation。每个连接仍通过普通 `scope.launch` 拥有自己的 Job；错误／EOF 恢复外层协程，沿原 `useAsync` 和 buffer 生命周期收尾。`raw.scoop` 让一次借用覆盖整个连接，并用线程直接推进 native 完成；不提供通用取消和调用方 Context 恢复。C 与薄 Scoop 在连接建立时按轮转分配 owner，传输阶段不迁移连接。完整协程版保留 work stealing，因此层间差值同时包含调度、局部性、GC 和推进批次变化，不是逐函数的精确耗时分账。

先用安装的 `~/.scoop` 工具链和固定的依赖准备仓库外副本：

```sh
python3 tests/benchmarks/tcp-echo/layers/prepare.py \
  --repo "$PWD" --output /absolute/new/diagnostic-source
```

输出包含诊断 asyncio Cone、两个 Scoop 入口和 `native.c`。脚本复制现有 `prepareTcpTransfer` 的实际准备／清理逻辑，额外入口接收连接的 Job；生产源码和安装 sysroot 均不改变。修改的是诊断库副本，需将该库构建到独立 artifact 目录，然后用独立 sysroot overlay 构建两个入口。仍使用既有 `metrics.c`、相同 `echo-client` 和 `measure.run_sample`。

所有服务器使用参数 `workers port connections bufferBytes`；C 版本仅 Linux。C 构建应使用相同 Scoop native 源码和 vendored liburing 2.15，并使用 `-O3 -fno-strict-aliasing`。其 `METRICS` 中 GC 字段为零，含义是没有 managed GC 分配，**不表示没有 native malloc**。RSS 仍由同一 harness 读取。全部版本均使用 TCP_NODELAY、单端口、相同消息和复用 buffer 大小；客户端逐次验证完整响应及序号。

先验证 64 KiB 消息／257 字节 buffer 的短读短写，再按随机顺序重复测量；构建与测量分开。性能判断使用普通程序的吞吐、CPU/echo、P50、RSS 和 managed 分配。没有为该诊断关闭 GC、取消完整程序的正常检查或改变生产线程调度器。

B25 固定产物、构建／测量脚本和全部原始数据在 `/home/chenxu/benchmarks/http-improvements-20261111-round25`。正式配置为 1／4 worker、64 连接、1 KiB 消息、64 KiB buffer，各三次，预热 1 秒＋测量 4 秒。具体结论见 `docs/asyncio-backends/PROGRESS.md` 第二十五轮。
