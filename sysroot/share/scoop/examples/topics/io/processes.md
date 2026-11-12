## 运行

保存两个文件，先构建子程序，再运行父程序。可执行路径作为一个参数传入，不经过 shell：

```sh
scoop build --release child.scoop -o child
scoop run --release main.scoop -- ./child
```

Windows 上使用 `-o child.exe`，并传入 `./child.exe`。

## 预期结果

父程序 stdout：

```text
stdout: Hello, 子进程!
stderr: child diagnostic
exit 0
empty: 0 bytes, exit 0
failed: exit 7, child failed
timeout: partial output
spawn failed before a child was created
```

## 关键说明

`Command` 分别保存可执行路径与参数；带空格的参数也无需自行拼接引号。所有标准流显式使用 Pipe，`communicate` 写入 stdin、关闭输入并同时收集 stdout/stderr，最后取得 `ExitStatus`。先保存 `ProcessOutput`，到展示边界才把本例已知的 UTF-8 协议解码为文本。

退出 0 且输出为空仍是成功；非零退出有实际退出码和 stderr；`spawn` 失败会抛出 `PlatformException`，此时没有可等待的子进程。`Signaled` 是另一种退出状态，不能伪装成正常退出 0。调用方负责决定哪些失败允许降级。

`communicate(timeout: Duration, input: List<Byte>? = None)` 使用同一个单调期限覆盖输入、两个输出和最终等待。默认 `capture` 给命令五秒；等待模式给一秒并捕获 `ProcessTimeoutException`，异常中的 stdout/stderr 是已经收到的部分字节。到期后库关闭本次端点、终止并回收所拥有的 child，再抛出异常；部分输出不保证包含管道中尚未读取的尾部。示例 child 在等待前立即刷新输出，先前输出因而可在超时后使用。

负时长在接管端点前拒绝；零值不等待管道进展。清理失败用 `ResourceUseException` 保存主异常和清理错误，应继续保留这些信息。没有超时参数的重载会持续等待；`use` 中的普通 `close` 也只关闭并等待，不负责强杀。库只管理直接拥有的 child，后代即使仍持有 pipe 也不会令有限通信无限等待，但不会被一并终止。

## 常见误用

先完整读 stdout 再读 stderr 可能因另一条管道塞满而死锁，应使用 `communicate`。不要把空输出等同于启动失败，也不要用 `catch { "" }` 丢失退出状态。`decodeUtf8` 对非 UTF-8 输出会报错；调用真实工具前先确定输出编码和机器可读格式。
