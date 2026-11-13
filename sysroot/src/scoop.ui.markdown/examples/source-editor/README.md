# Markdown 源码编辑器示例

纯 Scoop 桌面程序，通过 `markdownSourceEditor(editor)` 挂载源码视图。参数为现有 Markdown 文件；运行后点击正文即可输入，Cmd-S 接受当前输入法组合并保存当前快照。文件读写由应用负责，UI 控件只处理文档和编辑状态。

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/source-editor --release -- /absolute/path/to/document.md
```

视图按源码物理行虚拟排版，自动换行，使用原生选择、剪贴板和输入命令。支持中文组合输入、按 grapheme 删除、跨行选择、Tab／Shift-Tab 缩进及统一撤销／重做。关闭窗口不会自动保存，请在关闭前使用 Cmd-S。当前示例是源码模式；富文本和双栏接入后将更新完整示例。
