# 富文本原生窗口验证

本 Cone 使用 `scoop.test`，在一个真实窗口中验证行内与跨块富文本输入、中文 IME 组合及取消、剪切／粘贴／删除、整篇替换与统一历史、连续 Enter／输入／边界合并、列表层级、表格新增行及跨单元格输入、RTL 导航、只读切换、空文件输入和大文档的活动行排版。覆盖待后台解析时立即全选、回到单段输入，以及末块为代码时的整篇编辑。测试只在窗口获得系统焦点、输入连接启用、对应布局已接受后执行下一步；超时会报告当前步骤及焦点状态。

从仓库根运行：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.ui.markdown/tests-cones/rich --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.ui.markdown/tests-cones/rich --release
```

macOS 从后台宿主启动裸程序时可能不接受激活请求。此时可以把同一份 `scoop test` 生成的程序打包成标准应用，经 Launch Services 启动。`Bundle.toml` 仅提供本地测试应用的 metadata，不修改测试入口或焦点要求：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.ui.markdown/tests-cones/rich --release --no-run
~/.scoop/bin/scoop --sysroot ~/.scoop bundle \
  --executable sysroot/src/scoop.ui.markdown/tests-cones/rich/target/aarch64-apple-darwin/release/scoop.tests/markdown-rich-window/tests/markdown-rich-window \
  --target aarch64-apple-darwin \
  --config sysroot/src/scoop.ui.markdown/tests-cones/rich/Bundle.toml \
  --output-dir /tmp/scoop-markdown-rich-app --format app
mkdir /tmp/scoop-markdown-rich-events
open -n -W \
  --env SCOOP_TEST_RUN_DIR=/tmp/scoop-markdown-rich-events \
  '/tmp/scoop-markdown-rich-app/aarch64-apple-darwin/prebuilt/org.scoop.tests.markdown-rich/app/Scoop Markdown Rich Verification.app' \
  --args --scoop-test-filter MarkdownRichWindowTests
cat /tmp/scoop-markdown-rich-events/events.jsonl
```

示例路径用于 aarch64-apple-darwin；使用其他本机 target 或 `--target-dir` 时按实际构建输出调整。事件输出沿 `scoop.test` 的既有协议：必须出现用例 `0` 的 begin、带空 failures 的 end 和最终 complete；单独的 `open` 退出成功不能证明测试通过。失败详情同时保存在 `0.stdout`、`0.stderr`。核对后清理本轮 `/tmp/scoop-markdown-rich-app` 和 `/tmp/scoop-markdown-rich-events`，保留正常 Cone 构建缓存。
