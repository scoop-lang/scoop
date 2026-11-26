## 运行

将示例保存为 `main.scoop`，在 macOS 或 Linux 上运行以下命令。

```sh
scoop run --release main.scoop
```

## 预期结果

程序依次输出三行 `true`。第一行确认创建的是 FIFO。第二行确认通过符号链接读到的设备号和 inode 与目标相同。第三行确认关闭跟随时读到链接自身。

## 文件权限与身份

`0x180` 对应八进制 0600，实际权限还受进程 umask 影响。每次 `unixMetadata` 调用都从同一份 stat 或 lstat 结果生成全部字段。不同调用之间文件可以变化。删除节点后，文件系统可以重用它的 inode。

示例使用两个直接依赖。`scoop.platform` 提供路径和临时目录，`scoop.os.nix` 提供 Unix 扩展。
