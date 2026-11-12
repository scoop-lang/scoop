## 运行

保存 `existing_connection.scoop`，执行：

```sh
scoop run --release existing_connection.scoop
```

程序在本机回环地址分配临时端口，同时运行一个客户端。一个请求结束后自动关闭两端连接与 listener，不需要外网或第二个终端。

## 预期结果

程序 stdout：

```text
200
Hello, borrowed connection!
```

## 关键说明

`routing` 创建的同一个 `HttpApplication` 既可交给 `HttpServer`，也可通过 `executeHttp1` 处理已有 `AsyncHttp1Connection` 上的当前请求。调用方提供真实连接元数据和请求 ID，在接收请求头之后、消费正文之前调用执行入口。

`httpRequestOptions` 配置正文总量、聚合读取上限、请求总期限与完成回调。这份不可变配置也可以赋给 `httpServerOptions { request = options }`，或用 `request { ... }` 配置组设置。请求执行会建立独立的 Job 与 `HttpContext`，复用 middleware、路由、错误映射、响应输出和清理；完成回调在请求资源与结构化子任务结束后调用。

这个例子只执行一次请求。需要复用连接时，由调用方接收下一条请求、提供新的 ID、检查 `isReusable` 并安排循环公平性。头接收超时、连接容量、关闭协调等仍是调用方的职责，完整 server 提供这些能力。

## 常见误用

不要预先读取 incoming 的正文，也不要把它传给另一条连接或并发执行同一请求。成功执行不会自动关闭健康连接；未读正文、取消或协议错误仍可能使连接关闭，外层必须保留 `useAsync` 收尾。请求总期限在应用请求管线中启动，不覆盖之前的头接收。

此入口借用连接，没有 transport 接管权。返回 WebSocket 升级结果会在提交前拒绝为 400；需要升级时使用 `HttpServer`。HTTP/2 继续使用 server 已有的执行路径，本主题只演示 HTTP/1.1。
