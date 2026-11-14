# 原生 Markdown 预览

从仓库根目录运行：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop run sysroot/src/scoop.ui.markdown/examples/preview --release -- docs/specs/SCOOP-SPEC.md
```

示例只使用公开的 `MarkdownEditorState`、`MarkdownParseController` 和 `markdownPreview`。应用持有线程池，组件关闭自己的解析控制器；窗口关闭后才回收线程池。链接点击把目标交给应用，本示例将其输出到终端。

预览显示普通 Markdown、表格、代码、脚注与来源定位。当前图片及行内扩展显示可选择的替代文字／源码对象；真实资源加载与扩展图像渲染按编辑器后续阶段接入。原文件不会因预览、复制或生成显示内容而改写。
