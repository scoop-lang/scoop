# 富文本原生窗口验证

本 Cone 使用 `scoop.test`，在一个真实窗口中验证行内与跨块富文本输入、中文 IME 组合及取消、剪切／粘贴／删除、整篇替换与统一历史、连续 Enter／输入／边界合并、列表层级、表格新增行及跨单元格输入、RTL 导航、只读切换、空文件输入和大文档的活动行排版。覆盖待后台解析时立即全选、回到单段输入，以及末块为代码时的整篇编辑。测试只在窗口获得系统焦点、输入连接启用、对应布局已接受后执行下一步；超时会报告当前步骤及焦点状态。

`MarkdownLocalSourceWindowTests` 通过公开 `markdownEditor` 点击代码、Mermaid、块／行内公式及 mdBook 指令，验证局部源码的全选、中文组合取消、修改与撤销重做、未闭合指令修复、Done／Escape 返回、Reset 关闭面板及只读。所有编辑共用原文档与历史，并保留区间外源码。

`MarkdownFormattingWindowTests` 验证公开格式工具栏的真实按钮点击、粗体／斜体／删除线快捷键、跨块反向选区和混合状态、折叠光标后续中文／emoji 输入、组合取消与接受、统一撤销重做、模式切换、只读以及卸载后的格式命令状态。

富文本用例同时验证真实系统剪贴板的 text/plain／text/markdown、独立多块粘贴、一次撤销、显式纯文本／Markdown／TSV、存在但为空的格式和只读命令。测试保存并恢复原剪贴板的全部项目与格式。

从仓库根运行：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.ui.markdown/tests-cones/rich --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.ui.markdown/tests-cones/rich --release --filter MarkdownRichWindowTests
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

局部源码或格式用例使用同一应用包，把筛选器分别改为 `MarkdownLocalSourceWindowTests` 或 `MarkdownFormattingWindowTests`，并各设一个空的 `SCOOP_TEST_RUN_DIR`。三个窗口用例串行运行，避免争用原生焦点。

示例路径用于 aarch64-apple-darwin；使用其他本机 target 或 `--target-dir` 时按实际构建输出调整。事件输出沿 `scoop.test` 的既有协议：必须出现所选用例的 begin、带空 failures 的 end 和最终 complete；单独的 `open` 退出成功不能证明测试通过。失败详情同时保存在相同事件目录的 stdout／stderr 文件。核对后清理本轮 `/tmp/scoop-markdown-rich-app` 和 `/tmp/scoop-markdown-rich-events`，保留正常 Cone 构建缓存。
