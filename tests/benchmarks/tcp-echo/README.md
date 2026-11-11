# 裸 TCP echo 分层基准

用于先判断 `scoop.asyncio`、event loop 与 worker 调度的成本，再向上调查 HTTP。服务器只有 accept、read、write 循环，没有 HTTP、路由、JSON、请求级计时器或每请求日志。Scoop 与 Go 每个连接均复用一个 64 KiB 缓冲区，收到多少字节就原样写回，并正确处理短读、短写和 EOF。

- `server.scoop`：`workers=0` 使用单独 `eventLoop`；正数使用 `AsyncRuntime`，连接协程运行于 worker executor。
- `server.go`：每个连接一个 goroutine，`GOMAXPROCS` 对应请求的 worker 数。
- `client.go`：同一个 Go 客户端用于全部服务器；每连接仅有一个未完成请求，每次校验完整正文和变化的序号。连接建立后先预热，再计时，最后保持连接至测量器完成 CPU 快照。
- `metrics.c`：只在启动时为 Linux listener 设置 TCP_NODELAY、刷新就绪通知，并在整个工作负载前后打印既有 GC 指标，不参与 echo 热路径。
- `measure.py`：依次测量固定二进制，服务器与客户端绑定不同 CPU，保存原始输出和 `/proc` 快照。

所有服务器线程，包括 I/O owner、GC helper 和 Go runtime 线程，都受同一服务器 CPU 集合限制。因此 `AsyncRuntime(workers=1)` 与 Go `GOMAXPROCS=1` 都只有一个 CPU 的总资源，但前者还有独立的 I/O owner；线程数会另外记录。

## 构建

使用安装的 Scoop 编译器及已准备的库产物；产物、缓存和原始测量放在仓库外。下面的输出目录及可选库路径按当前环境填写：

```sh
bench_work=/path/outside/repository/tcp-echo
mkdir -p "$bench_work/bin"
go build -trimpath -o "$bench_work/bin/go-echo" tests/benchmarks/tcp-echo/server.go
go build -trimpath -o "$bench_work/bin/echo-client" tests/benchmarks/tcp-echo/client.go
cc -O2 -Iruntime/include -c tests/benchmarks/tcp-echo/metrics.c -o "$bench_work/metrics.o"
ar rcs "$bench_work/bin/libecho_metrics.a" "$bench_work/metrics.o"
~/.scoop/bin/scoop build tests/benchmarks/tcp-echo/server.scoop --release \
  --library-path "$bench_work/bin" -o "$bench_work/bin/scoop-echo"
```

消费本地改进后的固定库时传 `--cone-path`，从当前 runtime 源码构建时显式传 `--runtime-root` 与所需 `--unwind-prefix`；两者均写进该次构建记录。不要在计时期间构建或修改二进制。

## 测量

先以较小接收缓冲区验证短传输；它仅作正确性验证，不纳入正常性能表：

```sh
taskset -c 18 python3 tests/benchmarks/tcp-echo/measure.py \
  --scoop-server "$bench_work/bin/scoop-echo" --go-server "$bench_work/bin/go-echo" \
  --client "$bench_work/bin/echo-client" --work "$bench_work/short-transfers" \
  --workers 1 --connections 2 --payload 65536 --buffer 257 \
  --seconds 0.3 --warmup 0.1 --repeats 1
```

正常配置为 1 KiB 往返消息、64 KiB 每连接缓冲区、1／64 连接、1／4 worker，另测单独 event loop；各预热 1 秒、测量 6 秒、重复三次，固定随机次序：

```sh
taskset -c 18 python3 tests/benchmarks/tcp-echo/measure.py \
  --scoop-server "$bench_work/bin/scoop-echo" --go-server "$bench_work/bin/go-echo" \
  --client "$bench_work/bin/echo-client" --work "$bench_work/results" \
  --server-cpus 0,2,4,6 --client-cpus 8,10,12,13,14,15,16,17
```

CPU 编号必须按实际机器选择。脚本检查客户端与服务器集合不重叠，单 worker／单独 event loop 使用服务器集合的第一个 CPU。已完成的 `result.json` 会复用；新一轮测量使用新目录，不覆盖旧样本。

## 统计口径

吞吐单位是完整 echo 往返／秒，一个往返同时发送并接收 `payload` 字节。延迟从客户端开始发送到收齐回声，不含内容校验；这是闭环延迟，没有做 coordinated-omission 修正。直方图分位数使用桶上界，最大相对桶宽为 1/64；最大值与平均值直接统计。

CPU 使用率中 100% 为一个 CPU；每请求 CPU 包含用户态与内核态。CPU、RSS 和线程快照对应预热之后的测量窗口，客户端 CPU 单独记录，用于检查发压端是否饱和。

分配与 GC 指标取服务器启动前后差分，包含连接建立、预热、正常测量和退出，按含预热的全部成功往返数归一化，不能与仅稳态窗口的 GC 差分混用。Go 的 `PauseTotalNs` 仅记录其 STW 暂停，Scoop 的 `pause_ns` 记录收集阶段，不能直接比较二者来推断 GC CPU 成本。

原始日志与逐次 JSON 留在指定输出目录，仓库只保留基准源码、复现方式和必要的结果摘要。
