# 三模式 Markdown 编辑器

本示例打开已有文件，提供源码、所见即所得和双栏模式，支持右侧只读、源码锚点滚动同步、可调栏宽、共享撤销／重做，以及保存按钮／Cmd-S。保存接受当前组合后写出同一份源码快照；纯模式切换和预览不重排 Markdown。Cmd-Q 退出示例，未保存的编辑不写回文件。

模式栏下方使用公开的 `markdownFormattingToolbar`，提供粗体、斜体、删除线与行内代码；对应快捷键为 Cmd-B、Cmd-I、Cmd-Shift-X、Cmd-E。格式操作支持跨块选区；无选区时控制后续键入，不产生空标记。按钮显示混合状态，纯源码模式或右侧只读时禁用。Link／Image 打开文字／替代文字、URI 和标题面板，可插入、修改或移除当前目标；Cancel 不改变文档。图片 URI 相对当前文件解析，编辑器沿用本地图片适配展示，文件保存／上传由宿主应用提供。

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/editor --release -- /absolute/path/document.md
```

可选参数与预览示例相同：`--mermaid /path/to/mmdc`；`--mathjax /path/to/node /path/to/@mathjax/src /path/to/rsvg-convert`。工具由用户预先配置；Mermaid 以扩展图像块显示，源码围栏保留。普通本地 PNG／JPEG 使用文件 URI 解析。

从后台宿主启动时，可用现有 `scoop bundle` 将 release 输出打包成 `Bundle.toml` 指定的 macOS app，再用 `open -n ... --args /absolute/path/document.md` 启动；应用参数仍传入同一 main。示例不自动安装图像工具。

普通段落、标题和表格单元格支持富文本输入、跨块替换与格式操作，Enter／Tab 可操作列表和表格。点击代码、公式、Mermaid 或自定义指令可在当前栏打开局部源码面，Done／Escape 返回。其余功能与性能验收见[实施记录](../../../../../docs/markdown/PROGRESS.md)；源码栏可以编辑所有内容。点击链接由示例显示目标，实际打开行为由宿主决定。
