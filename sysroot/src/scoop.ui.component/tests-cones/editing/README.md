# 文本控件与触摸配置

使用配套工具链和已准备的依赖产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/editing --release
```

macOS 实窗测试覆盖 Input／Textarea 的模型、IME、主题切换、滚动、模态输入和虚拟化。单独的触摸场景挂载 Input、Textarea、InputOtp、InputGroup 的两类文本控件及 ButtonGroup 输入框，使用正常 Props／builder 配置关闭触摸、短按定位、长按选词及 None 长按，再逐项验证二／三击选择与 multiTap 关闭后的连续短按。各控件还执行实际把手拖动，检查模型与焦点；单独关闭 selectionHandles 后保留短按，原位置的拖动不再移动 caret。菜单关闭时保留选词，稳定模型与选区直接来自文本宿主。可用 `--filter 'text touch options reach'` 单独运行。

触摸场景通过受控 native contact 队列运行，并等待实际 Accepted 帧，不能代替真实触屏设备。原生菜单、行内链接竞争、文本投影和取消／重入卸载由[共享文本宿主场景](../../../scoop.ui/tests-cones/editing/README.md)验证。各 Application 场景应在独立进程中运行，桌面会话必须允许呈现和聚焦窗口；实际通过范围见[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
