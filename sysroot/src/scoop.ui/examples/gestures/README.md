# 手势与等价操作

示例当前按 macOS 目标构建，使用 `ComponentScope.transformGesture` 缩放和旋转普通自绘卡片，同时接受原生手势与直接接触的双指变换；`ComponentScope.swipeGesture` 处理直接接触或系统交付的离散 swipe。缩放只改变卡片，范围为 25%～400%；变换行为报告累计值和增量，业务操作使用增量。

运行：

```sh
scoop run --release sysroot/src/scoop.ui/examples/gestures
```

将指针放在卡片区域，用触控板捏合／旋转；系统交付 swipe 时，向左下一张、向右上一张。在能交付原始接触的设备上，单指在 500 ms 内水平移动至少 48 个逻辑单位并释放，也切换一次。SwipeGestureOptions 可配置轴、启用、native／contacts／mouse 来源、distance 和 maxDuration；示例采用默认配置，鼠标不会触发 swipe。系统保留的全局手势不由示例接管。每个操作都有普通按钮，可用 Tab 导航、Enter／Space 激活，也通过相同业务命令提供 accessibility Invoke。按钮具有名称、焦点绑定和实际 Accepted 几何；卡片与文字有普通语义节点，没有额外的手势语义树或实时播报。

在能交付原始接触的平台与设备上，双指缩放／旋转沿同一行为操作卡片，一指抬起结束本次变换；contacts 可单独关闭，contactThreshold 可调整接管阈值。macOS 的触控板仍使用 AppKit 已识别的手势。受控触点窗口回归与真实触屏／笔验证分开记录，后者随 Windows／GTK 集中验收；Window 不伪造系统未交付的原始触点。自定义识别器的后续接口继续使用同一 contact 路由与成组归属边界，普通滚轮不触发缩放或 swipe。
