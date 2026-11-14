# 三模式原生窗口验证

本 Cone 使用 `scoop.test`，通过公开 `markdownEditor` 在同一个真实窗口中验证模式切换、中文组合、共享历史、左右只读、源码范围定位、双向滚动、分栏缩放与被丢弃的帧、视图状态替换及卸载资源。测试使用超过 1 MiB 的中英混合文档；操作等待原生焦点、输入连接及已接受帧。

剪贴板覆盖公开 ViewState 的命令路由、Markdown 源码原样粘贴、显式纯文本粘贴、富文本语义插入与单次通知。测试保存并恢复原系统剪贴板。

macOS 后台宿主可将同一份 `scoop test` 程序打包为标准 app 启动，不降低焦点要求。从仓库根运行：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.ui.markdown/tests-cones/editor --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.ui.markdown/tests-cones/editor --release --no-run
~/.scoop/bin/scoop --sysroot ~/.scoop bundle \
  --executable sysroot/src/scoop.ui.markdown/tests-cones/editor/target/aarch64-apple-darwin/release/scoop.tests/markdown-editor-window/tests/markdown-editor-window \
  --target aarch64-apple-darwin \
  --config sysroot/src/scoop.ui.markdown/tests-cones/editor/Bundle.toml \
  --output-dir /tmp/scoop-markdown-editor-app --format app
mkdir /tmp/scoop-markdown-editor-events
open -n -W \
  --env SCOOP_TEST_RUN_DIR=/tmp/scoop-markdown-editor-events \
  '/tmp/scoop-markdown-editor-app/aarch64-apple-darwin/prebuilt/org.scoop.tests.markdown-editor/app/Scoop Markdown Editor Verification.app' \
  --args --scoop-test-filter MarkdownEditorWindowTests
cat /tmp/scoop-markdown-editor-events/events.jsonl
```

必须核对事件中出现用例 begin、空 failures 的 end 和 complete；`open` 退出成功本身不能证明通过。失败详情保存在 `0.stdout`／`0.stderr`。按实际本机 target 调整路径，核对后清理本轮临时 app 和事件目录，正常 Cone 构建缓存保留。
