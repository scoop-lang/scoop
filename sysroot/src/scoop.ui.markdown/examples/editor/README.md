# 三模式 Markdown 编辑器

本示例打开已有文件，提供源码、所见即所得和双栏模式，支持右侧只读、源码锚点滚动同步、可调栏宽、共享撤销／重做，以及保存按钮／Cmd-S。保存接受当前组合后写出同一份源码快照；纯模式切换和预览不重排 Markdown。Cmd-Q 退出示例，未保存的编辑不写回文件。

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/editor --release -- /absolute/path/document.md
```

可选参数与预览示例相同：`--mermaid /path/to/mmdc`；`--mathjax /path/to/node /path/to/@mathjax/src /path/to/rsvg-convert`。工具由用户预先配置；Mermaid 以扩展图像块显示，源码围栏保留。普通本地 PNG／JPEG 使用文件 URI 解析。

从后台宿主启动时，可用现有 `scoop bundle` 将 release 输出打包成 `Bundle.toml` 指定的 macOS app，再用 `open -n ... --args /absolute/path/document.md` 启动；应用参数仍传入同一 main。示例不自动安装图像工具。

当前普通段落、标题和表格单元格支持行内富文本输入。块级 Enter／Tab、跨块替换等按[实施记录](../../../../../docs/markdown/PROGRESS.md)继续完善；源码栏可以编辑所有内容。点击链接由示例显示目标，实际打开行为由宿主决定。
