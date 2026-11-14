# 原生 Markdown 预览

从仓库根目录运行：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/preview --release -- docs/specs/SCOOP-SPEC.md
```

示例使用公开的 `MarkdownEditorState`、`MarkdownParseController`、`MarkdownImageController` 和 `markdownPreview`。应用持有线程池，组件关闭自己的解析与图片控制器；窗口关闭后才回收线程池。链接点击把目标交给应用，本示例将其输出到终端。

预览显示普通 Markdown、表格、代码、脚注与来源定位，并按文档位置读取本地 PNG／JPEG。可显式启用已安装的扩展工具：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/preview --release -- document.md \
  --mermaid /opt/homebrew/bin/mmdc \
  --mathjax /opt/homebrew/bin/node /absolute/path/node_modules/@mathjax/src /opt/homebrew/bin/rsvg-convert
```

MathJax 模块需由宿主提前安装，例如 `npm install --prefix /absolute/tool-directory --save-exact @mathjax/src@4.1.3`；mmdc 也需要可用的 Puppeteer 浏览器。库不在编辑过程中安装工具。无适配时显示可定位的替代文字／源码对象；工具失败保留源码并显示图像错误。原文件不会因预览、复制或生成显示内容而改写。
