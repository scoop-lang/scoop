# 轮播触摸与鼠标验证

使用已准备的开发 sysroot 和当前配套库产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/carousel --filter 'contact carousel' --release
```

鼠标和触点共用吸附、方向、绑定拒绝／替换、键盘、子项点击、取消与停帧场景。另用真实队列中的两根触点验证关闭 hover／focus 暂停后仍暂停自动播放、最后结束后重新等待完整 interval。

触点载荷通过受控 native 队列进入实际 Window、Surface 和 UI，等待实际呈现；不代替真实触屏／笔或 Windows／GTK 原生入口验收。每个 Application 用例需要独立进程，鼠标场景用 `--filter 'native carousel'` 选择；桌面须允许窗口呈现与聚焦。验证结果见[交互实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
