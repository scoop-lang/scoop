# scoop.http.server

纯 Scoop 的应用层 HTTP server，复用 `scoop.http`、`scoop.http.asyncio`、TLS、异步文件和压缩库。路由、middleware 与资源寿命由普通函数和结构化任务表达，不使用反射或 DI 容器。

`routing { ... }` 显式注册端点并构造不可变 `HttpApplication`。`HttpServer.listen` 可直接接收 application，或通过末尾回调配置路由；默认监听 `127.0.0.1:8080`，沿用 `HttpServerOptions` 的容量和超时设置。已有 listener 仍用 `takeOwnership` 接管；两种方式返回同一个 `HttpServer`，`shutdownGracefully` 排空，`close` 取消并等待实际清理。

下面直接使用现有事件循环；`listen` 负责创建 listener 和启动 server，`localAddress` 可读取实际绑定地址（包括 `port = 0` 分配的端口）：

```scoop
import scoop.asyncio.*
import scoop.http.server.*

fun main() {
    eventLoop().run {
        HttpServer.listen(this) {
            get("/", suspend { _ -> text("Hello") })
            group("/api") { group("/v1") { get("/status", suspend { _ -> text("ok") }) } }
        }.useAsync suspend { server -> server.awaitTermination() }
    }
}
```

需要指定端口时传 `port = 3000`，指定监听地址时传 `host`（`IpAddress`）。已有 application 可直接调用 `HttpServer.listen(application, scope, port = 3000)`；需要底层 backlog 或 reuseAddress 时继续自行创建 listener。事件循环与父 scope 由应用提供。

已有 `AsyncHttp1Connection` 时，可用 `application.executeHttp1` 执行当前尚未消费的请求。该入口复用相同的 middleware、路由、请求作用域、响应和清理；调用方保留头接收、请求 ID、连接管理与循环调度。`HttpRequestOptions` 可独立配置，也可赋给完整 server 的 `request`，或通过 `httpServerOptions { request { ... } }` 设置正文限制、总期限与完成回调。健康连接可在执行完成后复用；未读正文或失败仍可能关闭连接。完整且自动退出的例子见 [existing_connection.scoop](examples/existing_connection.scoop)。

单条执行借用连接，WebSocket 升级需要 `HttpServer` 拥有并接管 transport。HTTP/2 继续使用完整 server 的现有路径。

| 能力 | 公共入口与行为 |
| --- | --- |
| 路由与请求管线 | 静态／参数／末尾通配路径、分组、HEAD／OPTIONS、`HttpContext`、`HttpMiddleware`、`HttpResult` |
| 既有 H1 连接接入 | `HttpApplication.executeHttp1`、独立可复用的 `HttpRequestOptions`、调用方管理连接 |
| 传输与生命周期 | H1.0／H1.1、明文 H2 prior knowledge、TLS／ALPN、独立请求任务、容量、时限、GOAWAY、完成通知 |
| 提取 | 字段／Cookie、JSON／form、query DTO、`adapt`、静态 extractor |
| SSE | `sse`、逐事件 flush、可选有界心跳、`setDeadline` |
| Multipart | `body.multipart`、单次 part 流、限制、显式文件保存与外层 EOF 校验 |
| 压缩 | `responseCompression` 协商输出，`requestDecompression` 选择解码策略，`openRawStream` 显式读取原始正文 |
| 文件 | `file`／`staticFiles`、目录首页、指定 SPA fallback、同句柄 metadata 与读取、条件请求、单 Range、多区间回退 |
| WebSocket | `webSocket`、显式 Origin／子协议、可选 permessage-deflate；HTTP 清理后启动独立会话，支持 WSS |

普通流式响应在请求 middleware 资源作用域内执行。WebSocket 的 `onSession` 在 HTTP 清理后执行，接收独立 `WebSocketContext`；需要的业务值应先复制，会话自行取得资源。`staticFiles` 沿普通 OS 路径与 symlink 语义工作，不提供文件系统隔离。

SPA 在路由配置回调中使用现有文件 handler；以下片段需要 `import scoop.platform.fs.Path`：

```scoop
val root = Path.fromString("public")

val entry = root.resolve(Path.fromString("index.html"))

val application =
    routing {
        get("/", suspend { _ -> file(entry) })
        get("/assets/{*path}", staticFiles(root.resolve(Path.fromString("assets"))))
        get(
            "/{*path}",
            staticFiles(
                root,
                staticFileOptions {
                    indexDocument = Some("index.html")
                    fallbackFile = Some(entry)
                }
            )
        )
    }
```

已有文件正常发送；缺失的 GET／HEAD 路径回退到指定入口文件，目录首页优先。`/assets` 使用更具体且没有 fallback 的路由，因此缺失资源仍为 404。入口文件自身缺失、权限错误或非法路径按原错误规则处理。fallback 的 MIME、HEAD、条件请求与 Range 均使用实际入口文件；相对 fallbackFile 路径以进程当前目录为基准。末尾通配不匹配空后缀，因此挂载根仍显式注册。路由分组的 fallback 只处理未匹配路径，不接管文件 404；如 SPA 通配路径下还包含 API，应注册更具体的 API 端点及其通配 404 端点。

完整合同见 [HTTP server 规范](../../../docs/specs/SCOOP-HTTP-SERVER-SPEC.md)，总体设计及验证范围见 [设计目录](../../../docs/http-server/DESIGN.md)与[实施记录](../../../docs/http-server/PROGRESS.md)。服务端 mTLS／动态 SNI、HTTP/3 与 H2 WebSocket 不在本库当前范围内。

可运行的程序见 [examples](examples/README.md)，包含简单启动、既有连接接入、嵌套路由、query／JSON、SSE 和带完整前端资源的 SPA。

公开 API 的自动化消费测试保存在独立 [consumer-tests Cone](consumer-tests/Cone.toml)：

- [H1 typed query／JSON 与 gzip](consumer-tests/tests/application.scoop)
- [H2、SSE 与 gzip multipart](consumer-tests/tests/http2.scoop)
- [静态路径、范围和 304](consumer-tests/tests/files.scoop)
- [简化启动、嵌套路由与 SPA](consumer-tests/tests/startup_spa.scoop)
- [认证、Origin、子协议和 WebSocket 压缩会话](consumer-tests/tests/websocket.scoop)

先按仓库准备脚本说明准备目标 sysroot；consumer 使用已发布的目标 `.slib`，不会隐式重建系统库。验证应使用显式的工作目录、共享 cache 与 release profile，按当前变更选择 server 测试 filter 或上述小型 consumer Cone。所有创建临时文件的用例使用显式 `TMPDIR` 或当前目录。
