# 原生手势与等价操作

macOS 示例使用 `ComponentScope.transformGesture` 缩放和旋转普通自绘卡片，并通过 Surface 的 `InputEvent.Gesture` 处理 AppKit 的离散 swipe。缩放只改变卡片，范围为 25%～400%；手势行为报告累计值和增量，业务操作使用增量。

运行：

```sh
scoop run --release sysroot/src/scoop.ui/examples/gestures
```

将指针放在卡片区域，用触控板捏合／旋转；系统交付 swipe 时切换卡片。系统保留的全局手势不由示例接管。每个操作都有普通按钮，可用 Tab 导航、Enter／Space 激活，也通过相同业务命令提供 accessibility Invoke。按钮具有名称、焦点绑定和实际 Accepted 几何；卡片与文字有普通语义节点，没有额外的手势语义树或实时播报。

此示例使用 AppKit 已识别的手势。它不代表直接触屏、多触点接触路由或自定义识别器已实现；这些沿 S8 继续。Window 不伪造系统未交付的原始触点，普通滚轮不触发缩放或 swipe。
