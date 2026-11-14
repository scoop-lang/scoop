# 源码原生窗口验证

本 Cone 使用 `scoop.test` 在真实窗口中验证原生输入、中文组合与取消、双向选区、撤销重做、指针与键盘导航、只读和虚拟布局。局部源码覆盖物理行中间的范围、首尾删除边界、局部全选、外部编辑后的范围移动，以及超过 1 MiB 的局部区域翻页；范围外文字保持不变，原生输入仍使用光标附近片段。测试只在窗口焦点、输入连接与已接受布局就绪后执行下一步。

从仓库根运行；macOS 使用标准应用包启动，使系统可以正常激活测试窗口：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.ui.markdown/tests-cones/source --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.ui.markdown/tests-cones/source --release --no-run
~/.scoop/bin/scoop --sysroot ~/.scoop bundle \
  --executable sysroot/src/scoop.ui.markdown/tests-cones/source/target/aarch64-apple-darwin/release/scoop.tests/markdown-source-window/tests/markdown-source-window \
  --target aarch64-apple-darwin \
  --config sysroot/src/scoop.ui.markdown/tests-cones/source/Bundle.toml \
  --output-dir /tmp/scoop-markdown-source-app --format app
mkdir /tmp/scoop-markdown-source-events
open -n -W \
  --env SCOOP_TEST_RUN_DIR=/tmp/scoop-markdown-source-events \
  '/tmp/scoop-markdown-source-app/aarch64-apple-darwin/prebuilt/org.scoop.tests.markdown-source/app/Scoop Markdown Source Verification.app' \
  --args --scoop-test-filter MarkdownSourceWindowTests
cat /tmp/scoop-markdown-source-events/events.jsonl
```

示例目标为 aarch64-apple-darwin；其他目标按实际输出路径调整。必须核对 `begin`、`failures` 为空的 `end` 及最终 `complete`，不能只看 `open` 的退出码。失败详情在相同事件目录的 stdout／stderr 文件中。核对后清理本次应用包与事件目录，保留正常 Cone 构建缓存。
