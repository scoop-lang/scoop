# 惰性条目分区呈现

在本机 macOS 使用已准备的开发 sysroot 执行 `scoop check . --tests --release` 与 `scoop test . --release`。本 Cone 独立拥有进程中的一个 Application；不与另一个应用循环合并到同一个测试进程。

一亿项的惰性来源只构建四行，正文和顶部区域用 `SceneContext.item(key)` 呈现同一批条目。覆盖作用域间共享的呈现计数、变换／clip／滚动显露边界、聚焦行在区域间移动时保留输入身份、已测但未放置条目的退出、整体与逐项模式互斥，以及未知／Position／重复 key 与借用期限。场景期间禁止访问 source、build、挂载和测量。

`layout-results` 另外覆盖 A→B→A 选中布局对逐项 key 的限制，`errors` 验证遗漏已放置条目时的失败和资源收尾，`lazy` 与 `scene-cache` 保留原协议和缓存回归。运行只覆盖本地 macOS，不代表跨平台运行结果。

2026-11-22：本机 aarch64-apple-darwin 的格式／语义检查、上述五个 Cone 的 release 运行均通过；生成 scoop.ui API 文档后，`SceneContext.item` 的公开签名、注释和有效 import 查询通过。验证使用当前开发命令及显式更新的 sysroot，原始日志保存在仓库外。
