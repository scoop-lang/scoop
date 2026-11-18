# 抽屉触摸与鼠标验证

使用已准备的开发 sysroot 和当前配套库产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/drawer --filter 'contact drawer' --release
```

鼠标和触点共用四方向吸附、正文边界、把手 only、焦点、嵌套、关闭拒绝、取消与卸载场景；触点另验证正文有余量时实际滚动、边界向内滚动及向外拖动的竞争；零阈值下的按下和原地 Moved 保留候选，实际位移才决定方向。

`--filter 'native swipe drawer'` 单独运行 41 步原生离散滑动场景，覆盖相邻 snap、nativeSwipe／swipeToClose 开关、绑定拒绝／关闭 veto、正文滚动边界、真实把手区域、嵌套只关闭子抽屉、Top／逻辑 Start 的 RTL 方向、活动拖动被接替后不重复提交、停帧、禁用和卸载。外部 snap 状态与子抽屉同时变化时，也检查不替换 Props 即可同步父抽屉索引与显示高度，且不重复通知用户选择。

触点载荷通过受控 native 队列进入实际 Window、Surface 和 UI，等待实际呈现；不代替真实触屏／笔或 Windows／GTK 原生入口验收。每个 Application 用例需要独立进程，鼠标场景用 `--filter 'native drawer'` 选择；桌面须允许窗口呈现与聚焦。验证结果见[交互实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
