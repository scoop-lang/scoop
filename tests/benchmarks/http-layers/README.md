# HTTP 逐层替换与解析传输诊断

当前渐进实验以 `manual_server.scoop` 和 `progressive/` 为入口，先测最简 TCP 服务，再在同一个程序中逐步换入正式实现。每一步完成验证和相邻对照后重新判断最大增量，按结果继续拆分；具体步骤见下文“从手写 TCP 服务逐层替换”。没有单一大头时，按已知成本从大到小组合优化主要几项，再测完整路径；正确性通过且整体有确认收益就保留，即使幅度较小。`thin_server.scoop` 和 C 工具保留用于较早的解析／传输诊断，不能用其旧结果替代当前版本的渐进测量。

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

## 继续拆分应用请求执行

`application/` 是从上述 `08-application.patch` 的结果分出的补充对照，三个补丁依次作用于同一个 `server.scoop`。它们使用现有公开 API，不修改协议连接或应用执行器：

| 步骤 | 补丁 | 本步变化 |
| --- | --- | --- |
| `direct` | `01-direct.patch` | 全局 middleware 按原始 target 选择预建结果并 `ctx.respond`，跳过正式目标解析和路由 |
| `parsed` | `02-parsed.patch` | 仅改用 `ctx.request.path`，换入目标校验、路径解码及 query 解析 |
| `routed` | `03-routed.patch` | 保留一个全局 middleware，改为调用 `next()` 并使用原来的三条路由 |

另将 `routed` 与未加 middleware 的原始 `application` 单独比较，量出空 middleware 委托的成本。`direct` 本身仍有 middleware 生命周期和短路检查；`parsed → routed` 同时换入路由与 `next()` 委托，不能叫作纯路由树成本。`path` 返回已解析对象中的字符串，不额外重建路径。

`application_layers.scoop` 用于进一步拆开只能从同一 Cone 访问的请求执行部件。将**固定 SDK 中未修改的 server 源码**复制到仓库外，保留原 Cone coordinate，仅改为 executable 并加入该文件；这使诊断可以调用真实内部实现，无需公开内部构造器或修改生产库。所有模式使用同一个 ELF，最后一个参数选择模式：

| 模式 | 本步接入 |
| --- | --- |
| `protocol` | 原 AsyncHttp1Connection、独立请求作用域、直接数组响应、协议正文关闭、caller ID 与 yield |
| `request` | 加入原 H1 输出适配器、Expect 状态、HttpRequest／正文视图及视图清理 |
| `context` | 加入原 RequestDeadline（默认 None）、HttpContext 及精确类型绑定，仍直接发送数组 |
| `response` | 用原 `ctx.respond` 替换数组响应，执行响应认领、准备、状态和进度记录 |
| `direct` | 用原 `executeHttp1` 执行全局短路 middleware；也接回共享错误、完成及清理编排 |
| `application` | 同一执行器接入原来的三条路由 |

这些模式仅用于受控无正文请求。内部 `protocol` 使用 caller ID、`incoming.body.close()` 和额外诊断调用，不等同于原 `fair` 的空正文读取路径；前几个模式也没有完整应用的错误映射和完成观察。先对照预编译库的 `application` 与同 Cone 的 `application`，校准编译单元与调用形式的影响，再解释内部相邻增量；不得用内部程序的绝对性能替代正式 server 性能。

```sh
python3 - "$bench_sdk/src/scoop.http.server" "$bench_work/application-cone" \
  "$bench_source/application_layers.scoop" <<'PY'
from pathlib import Path
import shutil
import sys

source, destination, helper = map(Path, sys.argv[1:])
destination.mkdir()
shutil.copytree(source / "src", destination / "src")
manifest = (source / "Cone.toml").read_text().split("[test-dependencies]")[0]
(destination / "Cone.toml").write_text(manifest.replace('kind = "library"', 'kind = "executable"'))
shutil.copyfile(helper, destination / "src/benchmark.scoop")
PY
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop fmt --check \
  --sysroot "$bench_sdk" "$bench_work/application-cone/src/benchmark.scoop"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop check "$bench_work/application-cone" \
  --release --sysroot "$bench_sdk" --cache-dir "$bench_work/cache"
PATH="$bench_cli:$bench_llvm/bin:$PATH" scoop build "$bench_work/application-cone" \
  --release --sysroot "$bench_sdk" --cache-dir "$bench_work/cache" \
  --target-dir "$bench_work/target-application" --library-path "$bench_work/bin" \
  --unwind-prefix "$bench_sdk/lib/x86_64-unknown-linux-gnu/unwind" \
  --keep-symbols -o "$bench_work/bin/application-layers"
BENCH_TCP_NODELAY=1 taskset -c 0 "$bench_work/bin/application-layers" 1 18081 protocol
```

同样先验证每个模式，再按相邻步骤交替测量；使用外部脚本时可以将模式命名为 `local-protocol` 等以区分预编译库程序。三个补充补丁已逐字节重建固定源码，上述同 Cone 重建方式也已与实际测量源码核对。所有原始输出、同 Cone 的源码副本和 ELF 留在仓库外，仓库仅保存诊断入口与必要的差异补丁。结果及其限制见 [B54 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十四轮拆分应用成本并重新选择协议连接层)。

## 继续拆分协议连接接入

`connection/01-transport.patch` 从 `encoder`（应用到 `03-encoder.patch`）分支，只把输入泵和输出函数的参数从 AsyncTcpStream 改为 AsyncByteStream。编解码、缓冲和报文保持相同，先量出通过已有传输接口调用的成本。

`connection_layers.scoop` 保存 B55 开始时的内部拆分实验，使用 **B53 固定 SDK 中的原 `scoop.http.asyncio` 源码**。它直接访问该版本的内部消息状态；不要用后续已经调整状态结构的源码替代基线。按上节方式保留 Cone coordinate、改为 executable 并加入此文件，四个模式共享一个 ELF：

| 模式 | 本步接入 |
| --- | --- |
| `codec` | 正式 codec、AsyncByteStream 接口、手写输入泵及 64 KiB 分块输出 |
| `session` | 原连接所有权、请求关联、H1 输入／输出对象、空正文读取和消息完成；仍使用手写输入泵和分块输出 |
| `receive` | 用原 `receiveRequest` 替换输入泵及手工消息登记，接入正式事件接收与读操作管理 |
| `connection` | 用原数组 `sendResponse` 替换手工发送，接入完整响应进度、写操作与 flush |

手工消息登记只处理受控无正文请求，复用原 `advanceH1End`、`completeH1Output` 和关闭判断；前几个模式不具备完整操作的取消／错误处理语义。四个模式的响应仍按至多 64 KiB 分块；内部连接的输出缓冲在建连时预留 64 KiB。内部 `codec` 的请求载体保存完整 head 和 keep-alive 结果，调用形态也不同，因此两端分别以原 `transport` 与原 `connection` ELF 校准。相邻差额包含这些实际接入变化，不能作为单个函数或对象的独立成本。

使用固定 B53 SDK，先按下列方式准备源码；后续格式检查、静态检查、构建参数沿上节命令，将输入改为 `protocol-cone`，输出另存为 `protocol-layers`：

```sh
python3 - "$bench_sdk/src/scoop.http.asyncio" "$bench_work/protocol-cone" \
  "$bench_source/connection_layers.scoop" <<'PY'
from pathlib import Path
import shutil
import sys

source, destination, helper = map(Path, sys.argv[1:])
destination.mkdir()
shutil.copytree(source / "src", destination / "src")
manifest = (source / "Cone.toml").read_text().split("[test-dependencies]")[0]
(destination / "Cone.toml").write_text(manifest.replace('kind = "library"', 'kind = "executable"'))
shutil.copyfile(helper, destination / "src/benchmark.scoop")
PY
BENCH_TCP_NODELAY=1 taskset -c 0 "$bench_work/bin/protocol-layers" 1 18081 codec
```

先验证各模式的分段、pipeline、并发大小 GET／HEAD 响应，再完成两端校准及三个相邻对照。原程序、诊断程序和生产候选必须分别保存，生产库的改动另用原完整连接／server 源码重建后比较；内部诊断不替代生产回归。传输接口与内部三步的测量、两端校准和事件队列候选的撤回依据见 [B55 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十五轮拆分协议连接成本并撤回事件队列候选)。

## 继续拆分独立请求作用域

`scope/` 从原 `idle`（应用到 `05-idle.patch`）分支，在同一个请求处理区域连续替换三步，最后回到原 `scope` 源码。使用固定 B53 SDK 和原传输、协议连接、30 秒读写 idle 配置：

| 步骤 | 补丁 | 本步接入 |
| --- | --- | --- |
| `binding` | `01-binding.patch` | 用 `context(currentJobOrNone()!!)` 重新绑定当前连接自己的 Job，测实际 push／restore 和查找成本 |
| `detached` | `02-detached.patch` | 改用原 `nonCancellableScope<Unit>`，加入独立 Job、CoroutineScope、失败处理与结构化收尾 |
| `scope` | `03-parent.patch` | 改用原 `coroutineScope<Unit>`，加入父 Job 的关联、取消传播和完成时移除 |

重新绑定同一 Job 仍执行实际 Context push／restore；固定实现不会因对象身份相同跳过 push。该步骤使用每条连接自己的 Job，避免把所有请求的操作监听集中到共同父 Job 上。它保留原操作语义，却不提供独立请求 Job；`detached` 则按已有 API 隔离父取消。两者都是受控无正文请求的成本对照，不能作为完整请求作用域的替代实现。

各步骤保留固定 ELF，先验证分段／pipeline／并发大小 GET／HEAD 响应，再按原 `idle → scope` 总对照和三个相邻步骤交替测量。不同组的短测差额不能精确相加；四 worker 的 CPU 利用率与吞吐可能同时变化，应同时查看每请求 CPU、延迟及全部样本。三份补丁已逐字节重建已测源码；结果、独立计数和跨层采样见 [B56 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十六轮拆分请求作用域并重新检查跨层成本)。

## 在相同分层程序上验证通用优化

B57 保留了 invoke 根表、已知同步闭包展开和源级 suspend CFG 合并三项组合。对照双方从同一源码底座分别构建编译器、runtime 和系统库，再用相同的手写起点、正式 `connection` 与完整 `server` 源码构建六个 ELF。这样可以验证通用改动对各层的影响，同时保留解析、连接和应用执行的独立接入方式。三项只做了整体对照，不将旧单项百分比相加。

本机固定的 B57 正式工具及 SDK 可用于后续实验，新的输出与 cache 仍放在新的 `bench_work` 中：

```sh
bench_round=/home/chenxu/benchmarks/http-improvements-20261113-round57
bench_sdk="$bench_round/candidate/sysroot"
bench_cli="$bench_round/toolchain/bin"
bench_llvm=/home/chenxu/benchmarks/http-improvements-20261112-round28/candidate/llvm/installed
```

B57 runtime 合同为 35、metadata 为 12，CLI 与 SDK 必须配套；同源码 control 的工具及 SDK 分别保存在 `control-tools/bin` 和 `control/sysroot`。固定程序为 `bin/scoop-<control|candidate>-<manual|protocol|server>`，其中 `protocol` 对应渐进链的 `connection`，不是早期原版协议程序。两个版本使用同一份 `inputs/<stage>/server.scoop`；原始结果、逐字节网络验证、独立 CPU 采样及源码归组留在仓库外。

完整 server 的 1 KiB 单／四 worker CPU 下降约 **12.4%／10.8%**，64 KiB 下降约 **7.0%／10.0%**，手写和协议层也均有收益。后续诊断必须在新底座重测所选区间，旧 B54～B56 的相邻差额不能继续作为当前成本。具体数值、覆盖范围与剩余问题见 [B57 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十七轮保留根表闭包展开与挂起调用合并的组合收益)。

B58 在 B57 上重新构建 fair、direct、parsed、routed、application，连同手写起点、正式协议连接、完整 server 和 Go 完成同轮比较。手写 → 协议连接总增量为 **3.750／4.970 μs/请求**；应用内部 fair → direct 为 **3.138／3.580 μs**，仍包含请求执行的多个功能。各区间用于定位实际接入成本，不能视作独立函数成本或精确相加。

随后保持同一 B57 工具链，将四个锁的共享原子同步主体从泛型回调包装中拆出，使实际同步闭包在现有增长上限内展开；HTTP 正常端点跳过空 middleware 包装和不需要的 fallback 搜索。两方从同一输入路径重建三个固定程序，保留异常、取消、Context、请求进度和清理。完整 server 小响应 CPU 下降约 **5.9%／4.4%**，大响应下降约 **3.7%／3.8%**；三层均有实测收益，组合全部保留。新的正式资源为：

```sh
bench_round=/home/chenxu/benchmarks/http-improvements-20261113-round58
bench_sdk="$bench_round/candidate/sysroot"
bench_cli="$bench_round/toolchain/bin"
bench_llvm=/home/chenxu/benchmarks/http-improvements-20261112-round28/candidate/llvm/installed
```

B58 的 `toolchain` 链接 B57，ABI 35／metadata 12 保持。正式对照使用 `bin/scoop-<control|candidate>-<manual|protocol|server>`；不带 variant 的旧链接仅用于开始时的分层诊断。独立单连接 100 GET 减 0 GET 分配追踪与所有计数器精确对齐，完整 server 的闭包对象由 89 降到 54；它与 64 连接压测的分配量分开解释。原始优化报告、计数和新的 source Cone 归组留在仓库外，具体范围、RSS 变化与剩余问题见 [B58 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十八轮缩小同步包装并精简正常请求分派)。

B59 在 B58 上再次重建中间程序，手写 → 协议连接增量为 **3.477／4.387 μs/请求**，fair → direct 为 **3.202／3.033 μs**。实际单连接追踪显示完整 server 每请求 102 次加锁，其中 Job 只读查询及内联调用占 24 次。随后将取消／完成按 release／acquire 发布，并复用 HTTP/1 写入后的剩余字节数，保留真实进度与生命周期合同。完整 server 加锁次数 **102→74**，协议 **74→52**；完整 server 每请求多一个 AtomicInt 及 Job 布局增加的 16 bytes，共 **40 bytes**，协议分配保持。

组合的小幅收益保留：单／四 worker 手写 CPU **−1.7%／−0.7%**、协议 **−0.8%／−1.5%**；完整 server 小响应 CPU **−2.5%／+0.3%**，吞吐 **+2.5%／+1.4%**。大响应单 worker CPU −0.6%，四 worker CPU +0.1%、吞吐 −0.9%，全部成功样本均纳入；这些短测小变化不代表长期分布。当前正式资源更新为：

```sh
bench_round=/home/chenxu/benchmarks/http-improvements-20261113-round59
bench_sdk="$bench_round/candidate/sysroot"
bench_cli="$bench_round/toolchain/bin"
bench_llvm=/home/chenxu/benchmarks/http-improvements-20261112-round28/candidate/llvm/installed
```

本轮 76 个无仪表样本共 **129,999,833 个请求**，无 wrk 错误；锁与分配追踪、CPU profile 和性能计时分别执行。初版测试的类型约束和 EOF 断言已修正，相关复测及 moving／minor 发布测试通过。原始失败、全部结果、符号、`lock-comparison.json`、`allocation-comparison.json` 与复现脚本位于 B59，详见 [B59 记录](../../../docs/asyncio-backends/PROGRESS.md#第五十九轮发布-job-只读状态并减少-http-输出状态读取)。下一轮从 B59 SDK 重建所选中间程序，再判断最大成本；不把本轮优化前的差额继续当成新底座成本，不覆盖固定资源。

## 测量

`codec-micro MODE COUNT SPLIT COUNT_ALLOC` 重用 parser，输入为 46 字节的 `GET /small HTTP/1.1`，仅带 Host。`llhttp` 模式没有用户回调，是协议核心下界；`bridge` 使用生产回调、字段和事件队列，读取事件并全部释放。SPLIT 为 0 时整条输入，为 1 时每次输入一字节。CPU 时间不含 parser 建立和销毁，输出 checksum 和请求数，构建时必须保留 assert。

```sh
taskset -c 0 "$bench_work/bin/codec-micro" llhttp 1000000 0 0
taskset -c 0 "$bench_work/bin/codec-micro" bridge 1000000 0 0
BENCH_TCP_NODELAY=1 taskset -c 0 "$bench_work/bin/thin-server" 1 18081 native
```

独立计数程序沿相同 C 构建命令增加 `-DALLOC_COUNT -Wl,--wrap=malloc,--wrap=calloc,--wrap=realloc,--wrap=free`，另存为 `codec-alloc`，以 `bridge 1000 0 1` 运行。带计数的时长不进入性能表。字节数是请求的分配容量之和，不是峰值占用或 allocator 实际分配大小。

网络实验先逐字节验证正文，再运行独立绑核客户端；对各模式交替正序／逆序运行，使用相同 worker、连接数、预热和测量时间。吞吐测试与 profiler／uprobes 分开。`/__metrics` 的 C 输出可能与连接结束日志共享一行，读取时按 `METRICS ` 标记解码，不能要求标记位于行首。实际结果与诊断边界见 [HTTP 主路径记录](../../../docs/asyncio-backends/PROGRESS.md#第四十四轮http-解析传输与语言边界的主路径定位)。
