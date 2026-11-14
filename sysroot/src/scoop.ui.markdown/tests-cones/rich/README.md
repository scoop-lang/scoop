# 富文本原生窗口验证

本 Cone 使用 `scoop.test`，在一个真实窗口中验证行内与跨块富文本输入、中文 IME 组合及取消、剪切／粘贴／删除、整篇替换与统一历史、连续 Enter／输入／边界合并、列表层级、表格新增行及跨单元格输入、RTL 导航、只读切换、空文件输入和大文档的活动行排版。覆盖待后台解析时立即全选、回到单段输入，以及末块为代码时的整篇编辑。测试只在窗口获得系统焦点、输入连接启用、对应布局已接受后执行下一步；超时会报告当前步骤及焦点状态。

`MarkdownLocalSourceWindowTests` 通过公开 `markdownEditor` 点击代码、Mermaid、块／行内公式及 mdBook 指令，验证局部源码的全选、中文组合取消、修改与撤销重做、未闭合指令修复、Done／Escape 返回、Reset 关闭面板及只读。所有编辑共用原文档与历史，并保留区间外源码。 代码工具栏还验证 Copy code、Language / info 字段的原生组合、组合期间外部正文修改、Enter 与 Apply、未知语言、冲突围栏切换、一次撤销及恢复源码焦点；同时核对当前快照高亮。此用例也保存并恢复原系统剪贴板。

`MarkdownFormattingWindowTests` 验证公开格式工具栏的真实按钮点击、粗体／斜体／删除线／代码快捷键、跨块反向选区和混合状态、折叠光标后续中文／emoji 输入、组合取消与接受、退出代码后的换行、不可表示的表格代码转换拒绝、键盘操作工具栏、统一撤销重做、模式切换、只读以及卸载后的格式命令状态。

`MarkdownTargetWindowTests` 通过真实 Tab／Space 打开 Link／Image 面板，在原生输入字段中修改 URI、标题与图片替代文字，验证单次撤销、取消、焦点恢复、文档变化后的草稿拒绝、只读状态和原生组合结束后的目标编辑。

`MarkdownBlockWindowTests` 验证标题快捷键、真实列表／表格菜单、菜单关闭后的焦点恢复、块命令接受中文组合、分步撤销、列表类型和层级、空表格行／列中的原生输入、对齐与删除后定位、只读／源码模式拒绝，以及插入代码后直接进入局部源码。

`MarkdownSearchWindowTests` 验证查找字段的原生组合和取消、逐字查询、Enter／Shift-Enter 与 Cmd-G 前后跳转、隐藏后继续查找、替换字段的输入及 Replace all 按钮、共享撤销／重做、正则捕获、错误表达式与模板、三模式和只读切换。超过 1 MiB 的文档检查源码／富文本两侧到末尾匹配的实际滚动，卸载后禁止替换并保留借用资源。

`MarkdownSearchBackgroundTests` 使用独立 Application 进程，不创建窗口。通过实际 worker 队列阻塞检查连续查询的合并、旧结果退休、当前文档身份、失败后的恢复、隐藏后的废弃，以及后台工作尚未结束时关闭 controller。它与其他原生用例分别启动，遵守一进程一次 Application 的合同。

`MarkdownNavigationWindowTests` 验证原生行列字段的提交、非法输入与 IME 取消，大纲、TOC 与后退／前进按钮，三模式／只读导航，旧解析范围拒绝，以及超过 1 MiB 文档在两侧的末尾滚动。富文本标题定位到可编辑文字边界，源码和只读保留原始目标；只读目录点击验证后退仍回到点击前的位置。`MarkdownNavigationStateTests` 在独立进程检查反向选区、编辑／撤销／重做的位置映射、128 项历史、Reset 和借用资源生命周期。

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

局部源码、格式、链接／图片、块操作、查找、导航与折叠用例使用同一应用包，把筛选器分别改为 `MarkdownLocalSourceWindowTests`、`MarkdownFormattingWindowTests`、`MarkdownTargetWindowTests`、`MarkdownBlockWindowTests`、`MarkdownSearchWindowTests` 、`MarkdownSearchBackgroundTests`、`MarkdownNavigationWindowTests`、`MarkdownNavigationStateTests` 或 `MarkdownFoldingWindowTests`，并各设一个空的 `SCOOP_TEST_RUN_DIR`。原生用例按独立进程串行运行，避免争用原生焦点和重复创建 Application。

`MarkdownFoldingWindowTests` 验证源码／富文本折叠按钮、键盘越过隐藏内容、正反删除先展开、嵌套标题状态、中文组合保护、修改头部与旧范围拒绝、只读导航、跨折叠全选剪切及统一撤销、搜索／程序化光标自动展开。大文档阶段从两栏 Host 的实际 measurement 检查可见行数量和滚动范围，确认折叠后仅测量少量可见行；关闭后的借用资源仍可用。

示例路径用于 aarch64-apple-darwin；使用其他本机 target 或 `--target-dir` 时按实际构建输出调整。事件输出沿 `scoop.test` 的既有协议：必须出现所选用例的 begin、带空 failures 的 end 和最终 complete；单独的 `open` 退出成功不能证明测试通过。失败详情同时保存在相同事件目录的 stdout／stderr 文件。核对后清理本轮 `/tmp/scoop-markdown-rich-app` 和 `/tmp/scoop-markdown-rich-events`，保留正常 Cone 构建缓存。
