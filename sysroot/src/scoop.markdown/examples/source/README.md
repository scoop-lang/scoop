# 无 UI 的 Markdown 文件处理

本 Cone 通过 path 依赖 `scoop.markdown`，文件访问仅在应用侧依赖 `scoop.platform`。不需要窗口、图形环境或外部 Markdown 解析器。

不传参数时运行原始源码事务演示；传入两个文件路径时读取第一份文件，展开其中的 mdBook include，并将派生 Markdown 写到第二个路径：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.markdown/examples/source --release -- input.md expanded.md
```

例如 input.md 中的 `{{#include parts/chapter.md}}` 相对 input.md 所在目录读取，嵌套指令继续使用各自文件的目录。支持整文件、`:2`、`::10`、`:2:`、`:2:10` 和 `:anchor`；代码围栏内的 include 保留为代码正文。源 snapshot 和输入文件保持原样；`writeExpandedMarkdown` 是应用显式选择的输出操作。有错误诊断时命令停止输出并报告原指令所在行。

[processing.scoop](src/processing.scoop) 展示三个公共消费入口：

- `FileMarkdownResolver`：词法路径解析，使用文件的修改时间、变化时间和大小作为普通更新依据；只在需要时读取正文，读取期间文件变化作为当前读取失败报告。
- `processMarkdownFile(path)`：显式建立 snapshot、profile 和处理 session，返回展开后的解析、依赖、诊断与来源映射。
- `writeExpandedMarkdown(result, destination)`：通过 `Utf8Writer` 分块写出所选择的派生 snapshot。

需要持续编辑的应用应复用一个 `MarkdownProcessingSession`，以取得版本未变时的结果复用；源文档的保存继续使用原始 snapshot。应用有更准确的文件通知版本时，可以替换示例 resolver 的版本实现。

[processing 测试](tests/processing.scoop) 在临时目录内验证实际读取、单独输出、保留原文件和更新检测，完成后删除目录；同时从另一个 Cone 实现自定义 topic 处理器，检查生成链接的来源映射。[结构编辑测试](tests/editing.scoop) 使用公开 API 添加带格式和对齐的表格列、粘贴矩形 TSV、缩进任务项并转换有序列表、按行内显示范围编辑及精确撤销；`MarkdownTextContent.setStyle` 和 `styleState` 验证跨块格式事务与状态查询。段落拆分／合并返回 `MarkdownBlockEdit`，包含尚未发布的差量与结果源码 caret，无 UI 程序通过 `applyTo(document)` 发布，并可使用普通反向差量撤销。其他测试覆盖生成 DSL、扩展语法和快照。

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown/examples/source --release
```

[navigation 测试](tests/navigation.scoop) 使用独立标题索引生成目录、解析 Unicode fragment，并转换物理行／标量列与源码位置；不依赖任何 UI Cone，不修改原始文件。
