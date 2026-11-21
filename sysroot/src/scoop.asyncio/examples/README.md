# scoop.asyncio 示例

先完成[运行准备](../../../README.md#运行准备)，以下命令在本目录执行。

| 文件 | 内容 |
| --- | --- |
| [tasks.scoop](tasks.scoop) | 两个并发定时任务、await、取消与 finally 清理 |
| [tcp_echo.scoop](tcp_echo.scoop) | 同一事件循环内的 TCP listener/client、缓冲读写和半关闭 |
| [pty.scoop](pty.scoop) | 启动本地 shell，排空伪终端输出并获取退出状态 |

```sh
scoop run --release tasks.scoop
scoop run --release tcp_echo.scoop
scoop run --release pty.scoop
```

任务示例的完整说明见 `scoop example async/tasks`（[正文](../../../share/scoop/examples/topics/async/tasks.md)）。

TCP 示例绑定 `127.0.0.1:0`，由系统分配空闲端口，输出 `Hello over TCP!` 后结束；不需要外部服务器。`useAsync` 关闭连接和 listener，`NativeByteBuffer.use suspend` 在挂起期间保持缓冲有效。

伪终端示例输出 `hello-from-pty`。交互程序可以在独立任务中持续读取并同时写入，收到进程退出状态后仍读到 EOF。显式关闭可结束仍在运行的直接子进程。
