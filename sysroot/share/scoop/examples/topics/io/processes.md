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
spawn failed before a child was created
```

## 关键说明

`ProcessCommand` 分别保存可执行路径与参数；带空格的参数也无需自行拼接引号。所有标准流显式使用 Pipe，`communicate` 写入 stdin、关闭输入并同时收集 stdout/stderr，最后取得 `ExitStatus`。先保存 `ProcessOutput`，到展示边界才把本例已知的 UTF-8 协议解码为文本。

退出 0 且输出为空仍是成功；非零退出有实际退出码和 stderr；`spawn` 失败会抛出 `PlatformException`，此时没有可等待的子进程。`Signaled` 是另一种退出状态，不能伪装成正常退出 0。调用方负责决定哪些失败允许降级。

`use` 保证已创建的进程句柄关闭。当前示例只执行会自行结束的子程序；`close` 等待进程，不负责强杀。通信的截止时间用法随对应库能力发布。

## 常见误用

先完整读 stdout 再读 stderr 可能因另一条管道塞满而死锁，应使用 `communicate`。不要把空输出等同于启动失败，也不要用 `catch { "" }` 丢失退出状态。`decodeUtf8` 对非 UTF-8 输出会报错；调用真实工具前先确定输出编码和机器可读格式。
