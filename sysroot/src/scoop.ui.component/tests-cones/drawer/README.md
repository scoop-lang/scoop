# 抽屉触摸与鼠标验证

使用已准备的开发 sysroot 和当前配套库产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/drawer --filter 'contact drawer' --release
```

鼠标和触点共用四方向吸附、正文边界、把手 only、焦点、嵌套、关闭拒绝、取消与卸载场景；触点另验证正文有余量时实际滚动、边界向内滚动及向外拖动的竞争；零阈值下的按下和原地 Moved 保留候选，实际位移才决定方向。

触点载荷通过受控 native 队列进入实际 Window、Surface 和 UI，等待实际呈现；不代替真实触屏／笔或 Windows／GTK 原生入口验收。每个 Application 用例需要独立进程，鼠标场景用 `--filter 'native drawer'` 选择；桌面须允许窗口呈现与聚焦。验证结果见[交互实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
