# HTTP 逐层替换与解析传输诊断

当前渐进实验以 `manual_server.scoop` 和 `progressive/` 为入口，先测最简 TCP 服务，再在同一个程序中逐步换入正式实现。每一步完成验证和相邻对照后重新判断最大增量，按结果继续拆分；具体步骤见下文“从手写 TCP 服务逐层替换”。`thin_server.scoop` 和 C 工具保留用于较早的解析／传输诊断，不能用其旧结果替代当前版本的渐进测量。

这些程序定位正常、无正文 GET 的主路径成本，不是另一个生产 HTTP 实现。`thin_server.scoop` 的四种模式使用同一个 AsyncTcpStream、相同响应字节、每连接复用的输入／输出 buffer：

| 模式 | 请求输入 | 响应输出 |
| --- | --- | --- |
| `wire` | 只寻找头部结束分隔符 | 提前编码的完整响应 |
| `native` | 正式 llhttp 和 C 字段／事件桥接，单次 FFI 消费结果 | 同上 |
| `codec` | 正式 Http1Decoder，构造 Scoop 消息对象 | 同上 |
| `encode` | 同上 | 每次调用正式 Http1Encoder，再合并正文 |

每种模式支持 `/small` 的 1 KiB、`/large` 的 64 KiB 和 `/__metrics`。正文为重复的 `x`。`native` 使用正式 C 解析器，但诊断消费端只选择目标，不向调用者暴露字段；`codec`／`encode` 还包含目标转 String 的成本。输入头可以分段；性能工作负载每连接只有一个在途请求。不要发送 pipeline、请求正文或 Upgrade，也不要据这些程序判断 HTTP 完整语义的兼容性。`wire` 不做完整协议校验。正常接收／发送仍经现有协程和 TCP 后端。

与 AsyncHttp1Connection／完整 server 对照时，差额包含它们提供的流式 body、并发方向、取消、超时、关闭及请求处理合同，不能把诊断程序直接作为生产替代。这里只计量正常请求，不测试或调节 GC 策略。64 KiB 响应在薄路径可一次发送，而当前协议连接层按 64 KiB 块分段；用于定位共同小响应主路径的正式表只取 1 KiB。

## 构建

选用已固定的 sysroot 和配套 CLI。以下 `bench_llvm` 必须指向从项目源码和私有补丁构建的 LLVM；不需要重建编译器或系统库。设置 `bench_sdk`、`bench_cli`、`bench_llvm`、仓库外的 `bench_work` 后，在仓库根目录运行：

```sh
mkdir -p "$bench_work/bin" "$bench_work/cache" "$bench_work/target"
bench_source="$PWD/tests/benchmarks/http-layers"
bench_http="$bench_sdk/src/scoop.http/native"
bench_llhttp="$bench_sdk/vendor/llhttp"

"$bench_llvm/bin/clang" -O2 -g -fno-omit-frame-pointer \
  -Wall -Wextra -Werror -Wno-unused-parameter \
  -I"$bench_http" -I"$bench_llhttp/include" \
  "$bench_source/codec_micro.c" \
  "$bench_http/data.c" "$bench_http/http1.c" "$bench_http/http1_callbacks.c" \
  "$bench_llhttp/src/api.c" "$bench_llhttp/src/http.c" "$bench_llhttp/src/llhttp.c" \
  -o "$bench_work/bin/codec-micro"

"$bench_llvm/bin/clang" -O2 -I"$bench_http" -I"$bench_llhttp/include" \
  -c "$bench_source/http_diag.c" -o "$bench_work/bin/http_diag.o"
"$bench_llvm/bin/llvm-ar" rcs "$bench_work/bin/libhttpdiag.a" "$bench_work/bin/http_diag.o"

# Reuse the TCP benchmark's Linux listener configuration and metrics helper.
"$bench_llvm/bin/clang" -O2 -I"$bench_sdk/include" \
  -Decho_metrics=bench_metrics -Decho_configure_tcp=bench_tcp_options \
  -c tests/benchmarks/tcp-echo/metrics.c -o "$bench_work/bin/metrics.o"
"$bench_llvm/bin/llvm-ar" rcs "$bench_work/bin/libbenchmetrics.a" "$bench_work/bin/metrics.o"

PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop build "$bench_source/thin_server.scoop" \
  --release --sysroot "$bench_sdk" --cache-dir "$bench_work/cache" \
  --target-dir "$bench_work/target" --library-path "$bench_work/bin" \
  --unwind-prefix "$bench_sdk/lib/x86_64-unknown-linux-gnu/unwind" \
  --keep-symbols -o "$bench_work/bin/thin-server"
```

平台 C compiler／headers 需要额外设置时，沿配套 SDK 的正常 `--cc` 参数传入。

## 从手写 TCP 服务逐层替换

`manual_server.scoop` 是不依赖 HTTP 库的起点，只使用 AsyncTcpStream、NativeByteBuffer 和少量手写报文处理。它识别头部结束与起始行，支持受控基准中的无正文 GET／HEAD、三个固定路径、分段输入和连续请求；不实现通用 HTTP 字段校验、请求正文、升级、超时或完整会话语义。每条连接复用 16 KiB 输入与 64 KiB 输出缓冲，静态响应预编码，输出按同一 64 KiB 上限写入。

`progressive/` 保存连续补丁，各步骤都是修改同一个 `server.scoop`，不另存重复程序。`application` 及之后的步骤使用提供 `executeHttp1` 的 SDK：

| 步骤 | 补丁 | 本步变化 |
| --- | --- | --- |
| `manual` | 起点 `manual_server.scoop` | 手写请求头扫描，预编码完整响应 |
| `decoder` | `01-decoder.patch` | 正式 Http1Decoder 替换手写解析，输出保持整包 |
| `split` | `02-split-output.patch` | 预编码头与正文分别填入同一输出缓冲，单独测表示／复制变化 |
| `encoder` | `03-encoder.patch` | 正式 Http1Encoder 编码头并维护正文定界状态 |
| `connection` | `04-connection.patch` | AsyncHttp1Connection 接管 codec、缓冲、正文结束及操作生命周期 |
| `idle` | `05-idle.patch` | 仅启用与 server 相同的 30 秒读／写 idle 超时 |
| `scope` | `06-request-scope.patch` | 每请求增加现有 coroutineScope，保留独立 Job 和结构化收尾 |
| `fair` | `07-fairness.patch` | 每请求结束后增加一次 yieldCoroutine |
| `application` | `08-application.patch` | 复用 HttpApplication 的单条请求执行；调用方继续接收头、管理连接与 yield |
| `server` | `09-server.patch` | 同一个 application 交给 HttpServer，加入头部计时、容量登记与关闭协调 |

`scope` 和 `fair` 是 B52 发现组合边界增量最大之后补入的对照。B53 再以 `HttpApplication.executeHttp1` 拆出应用执行：请求对象、body 视图、middleware／路由、响应执行和请求清理共用完整 server 的实现；caller 每连接生成请求 ID，完整 server 使用其登记与标识符逻辑。应用入口内部已创建独立请求作用域，因此替换手写 scope；空正文的结束也由执行器负责，不再沿用 caller 的 bodyInput 缓冲。公开入口多一层挂起调用也属于实际接入成本，不能把差额全部归给路由。借用入口不接管 transport，WebSocket 升级仍走完整 server 路径。

连接步骤使用数组响应入口和复用缓冲读取空正文，不沿用旧协议程序的中间缓冲与 `readAllBytes`；两者不能混称同一个基线。各层有实际语义差异，有限诊断服务不能代替完整 server。

每一步检查实际响应，测吞吐、每请求 CPU、P50／P80 和 RSS；完整原版 server 与 Go 作为同轮参照。相邻两版正反顺序各运行一次，保持负载、SDK、绑核、连接数、默认 GC 和计时方式相同；计数与性能测量分开。多次相邻对照的差额不是可以精确相加的成本账本，短测波动及所有成功样本均应保留。此程序的性能不代表完整 HTTP 实现。

先将起点复制到仓库外，使用已构建的 `benchmetrics`、固定 SDK 和 CLI；不需要 `libhttpdiag.a`。命令行参数为 worker 数与端口：

```sh
cp "$bench_source/manual_server.scoop" "$bench_work/server.scoop"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop fmt --check \
  --sysroot "$bench_sdk" "$bench_work/server.scoop"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop check "$bench_work/server.scoop" \
  --release --sysroot "$bench_sdk" --cache-dir "$bench_work/cache"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop build "$bench_work/server.scoop" \
  --release --sysroot "$bench_sdk" --cache-dir "$bench_work/cache" \
  --target-dir "$bench_work/target" --library-path "$bench_work/bin" \
  --unwind-prefix "$bench_sdk/lib/x86_64-unknown-linux-gnu/unwind" \
  --keep-symbols -o "$bench_work/bin/manual-server"
BENCH_TCP_NODELAY=1 taskset -c 0 "$bench_work/bin/manual-server" 1 18081
```

基线完成网络验证和测量、保留 ELF 后，再应用第一份补丁。重新构建时更改 `-o` 的名称，保留每个相邻对照的产物：

```sh
git -C "$bench_work" apply "$bench_source/progressive/01-decoder.patch"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop fmt --check \
  --sysroot "$bench_sdk" "$bench_work/server.scoop"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop check "$bench_work/server.scoop" \
  --release --sysroot "$bench_sdk" --cache-dir "$bench_work/cache"
```

后续补丁按表中顺序应用。九个补丁已通过 `git apply --check`，应用后的源码与各阶段固定源码逐字节一致。原链的固定源码、ELF、脚本和相邻增量见 [B52 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十二轮从最简-tcp-http-server-逐步替换并定位组合边界)；B53 的应用接入、完整 server 回归与新相邻增量见 [B53 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十三轮拆出应用请求执行并量出连接管理的净增量)。

## 测量

`codec-micro MODE COUNT SPLIT COUNT_ALLOC` 重用 parser，输入为 46 字节的 `GET /small HTTP/1.1`，仅带 Host。`llhttp` 模式没有用户回调，是协议核心下界；`bridge` 使用生产回调、字段和事件队列，读取事件并全部释放。SPLIT 为 0 时整条输入，为 1 时每次输入一字节。CPU 时间不含 parser 建立和销毁，输出 checksum 和请求数，构建时必须保留 assert。

```sh
taskset -c 0 "$bench_work/bin/codec-micro" llhttp 1000000 0 0
taskset -c 0 "$bench_work/bin/codec-micro" bridge 1000000 0 0
BENCH_TCP_NODELAY=1 taskset -c 0 "$bench_work/bin/thin-server" 1 18081 native
```

独立计数程序沿相同 C 构建命令增加 `-DALLOC_COUNT -Wl,--wrap=malloc,--wrap=calloc,--wrap=realloc,--wrap=free`，另存为 `codec-alloc`，以 `bridge 1000 0 1` 运行。带计数的时长不进入性能表。字节数是请求的分配容量之和，不是峰值占用或 allocator 实际分配大小。

网络实验先逐字节验证正文，再运行独立绑核客户端；对各模式交替正序／逆序运行，使用相同 worker、连接数、预热和测量时间。吞吐测试与 profiler／uprobes 分开。`/__metrics` 的 C 输出可能与连接结束日志共享一行，读取时按 `METRICS ` 标记解码，不能要求标记位于行首。实际结果与诊断边界见 [HTTP 主路径记录](../../../docs/asyncio-backends/PROGRESS.md#第四十四轮http-解析传输与语言边界的主路径定位)。
