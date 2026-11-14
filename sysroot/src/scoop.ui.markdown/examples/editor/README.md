# 三模式 Markdown 编辑器

本示例打开已有文件，提供源码、所见即所得和双栏模式，支持右侧只读、源码锚点滚动同步、可调栏宽、共享撤销／重做，以及保存按钮／Cmd-S。保存接受当前组合后写出同一份源码快照；纯模式切换和预览不重排 Markdown。Cmd-Q 退出示例，未保存的编辑不写回文件。

源码栏显示物理行号及 Markdown 标记高亮，代码 token 沿用语言高亮。Enter 保留空格／Tab 缩进；在已识别、尚无正文的未闭合围栏首行末尾补出结束围栏。输入括号可包裹选区或自动配对，键入自动补出的右括号会越过它，空配对可一次退格删除。辅助操作可撤销；粘贴和输入法组合保持原文。宿主通过 MarkdownSourceOptions 分别控制 lineNumbers、autoIndent、autoPairs、autoFences。

模式栏下方使用公开的 `markdownFormattingToolbar`，提供粗体、斜体、删除线与行内代码；对应快捷键为 Cmd-B、Cmd-I、Cmd-Shift-X、Cmd-E。格式操作支持跨块选区；无选区时控制后续键入，不产生空标记。按钮显示混合状态，纯源码模式或右侧只读时禁用。Link／Image 打开文字／替代文字、URI 和标题面板，可插入、修改或移除当前目标；Cancel 不改变文档。图片 URI 相对当前文件解析，编辑器沿用本地图片适配展示，文件保存／上传由宿主应用提供。

块工具栏提供 Block、List 和 Table 菜单：可转换标题／段落，增减引用，改变列表类型和层级，插入代码块、分隔线与表格，并增删表格行列、设置列对齐。Cmd-0／1…6 对应段落／标题；菜单操作后可以继续输入，修改使用同一撤销历史。新增代码块直接进入局部源码编辑。

Find／Replace 或 Cmd-F／Cmd-Alt-F 打开原生全文查找栏，提供大小写、整词和正则选项。查询字段 Enter／Shift-Enter、Cmd-G／Cmd-Shift-G 前后跳转；Escape 在组合结束后返回编辑区域。Replace 修改当前匹配，Replace all 一次修改所有匹配并共用文档撤销；只读栏禁用替换。正则替换支持 `$1`、`${name}`、`$$`。搜索包含原始 Markdown 的标记和目标 URI，不展开外部 include；高亮与滚动使用同一源码映射。

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/editor --release -- /absolute/path/document.md
```

可选参数与预览示例相同：`--mermaid /path/to/mmdc`；`--mathjax /path/to/node /path/to/@mathjax/src /path/to/rsvg-convert`。工具由用户预先配置；Mermaid 以扩展图像块显示，源码围栏保留。普通本地 PNG／JPEG 使用文件 URI 解析。

从后台宿主启动时，可用现有 `scoop bundle` 将 release 输出打包成 `Bundle.toml` 指定的 macOS app，再用 `open -n ... --args /absolute/path/document.md` 启动；应用参数仍传入同一 main。示例不自动安装图像工具。

普通段落、标题和表格单元格支持富文本输入、跨块替换与格式操作，Enter／Tab 可操作列表和表格。点击代码、公式、Mermaid 或自定义指令可在当前栏打开局部源码面，Done／Escape 返回。代码面额外提供 Language / info、Apply 和 Copy code；Enter 也可提交完整信息字段，语言与属性均可编辑，复制只取代码正文，修改和撤销仍共享文档历史。代码高亮覆盖预览、源码和局部源码，未知语言显示原文；公式与 Mermaid 继续沿图像扩展展示。

其余功能与性能验收见[实施记录](../../../../../docs/markdown/PROGRESS.md)；源码栏可以编辑所有内容。本文件锚点由编辑器定位，其他链接由示例显示目标，实际打开行为由宿主决定。

大纲按钮显示／隐藏虚拟标题列表，导航栏提供 Back／Forward 和行列输入。Cmd-L 聚焦行列字段，输入 `12:3` 后 Enter 跳转，Escape 返回编辑栏。大纲、[TOC] 和本文件锚点共用源码导航历史，切换模式或只读不会改变原文；[TOC] 内容由预览派生，保存继续保留原指令。

标题和 front matter 首行旁的三角按钮同步折叠两栏；代码围栏只在源码栏折叠。`markdownFoldingToolbar` 提供当前区域及全部折叠／展开，可用键盘访问。展开父标题保留子标题的折叠；搜索和导航会展开目标。正在组合的文字不能被收起，边界删除先展开隐藏内容，所有折叠操作都保留原文件与撤销历史。
