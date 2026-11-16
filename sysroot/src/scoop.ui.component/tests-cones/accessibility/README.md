# 内置控件无障碍语义

使用当前 macOS 工具链和开发 Cone 产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/accessibility --release
```

测试挂载真实 Label、Button、Checkbox、Input／Textarea，以公开 NSAccessibility 方法查询和执行动作；ButtonGroup、InputGroup 参与实际组合。覆盖静态标签合并、独立交互后代、名称与关系覆盖、只换回调、纯文字比较后的名称更新、关系目标退休后的继续更新、三态与拒绝 Binding、行内 alternateText、Unicode 选区、文本写入及 undo、只读与保护用途、隐藏及关闭后旧引用。隐藏窗口期间先验证逻辑更新不依赖测量或绘图。

该用例不依赖实际鼠标事件、焦点或窗口呈现，也不代替外部 AX 客户端、VoiceOver 和实窗几何验证。后者继续使用 UI 的 accessibility-scene 及组件交互用例；构建成功不记为相应交互通过。
