## 运行

```sh
scoop run --release main.scoop
scoop build --release main.scoop -o text-report
./text-report > report.txt
```

在真实终端中将窗口缩到小于 24 列，再运行一次。设置 `NO_COLOR`（包括空字符串）可关闭本例的颜色。

## 预期结果

重定向输出是没有控制序列的两行：

```text
Scoop text report
bytes=18, scalars=7, graphemes=4
```

宽终端标题为绿色；窄终端改用短标题 Scoop，详情按终端的普通换行规则显示。NO_COLOR 存在时标题无颜色。

## 关键说明

先通过 `isTerminal` 检查 stdout，重定向分支不打开终端会话。真实终端分支通过 `TerminalSession` 读取列数并在 use 结束时关闭资源；关闭前刷新普通 stdout。

字符串 `A中é👩‍💻` 的 UTF-8 字节、Unicode 标量和字素数量各不相同。`charLength` 统计标量，`graphemes(sample).size` 统计字素；二者都不等于所有终端中的显示列宽。此例不按任一计数进行截断或补空格对齐，窄屏只切换已知 ASCII 标题。

## 常见误用

不要把 ANSI 控制序列写入重定向输出，或用字节下标截断 UTF-8。需要列对齐时应采用明确的终端宽度策略，并测试组合字符、emoji 与宽字符。终端可用性和颜色偏好是两个独立条件。
