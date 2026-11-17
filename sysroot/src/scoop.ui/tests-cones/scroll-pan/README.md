# 共享直接滚动与弹性边界

使用当前开发 sysroot 和依赖产物，在 macOS 图形会话分别运行以下用例；每个用例使用独立的 Application 进程：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'direct scroll composes'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'direct scroll yields'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'elastic contact boundaries'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'elastic native scrolling'
```

原有场景以明确 Hard 边界检查嵌套位移、阈值、缩放、点击／拖动归属及取消。弹性场景检查合法范围、内外层唯一拉伸、反向恢复、Contain、短内容、内容尺寸变化、配置替换、减少动画、禁用和离屏停帧；原生 wheel 场景区分离散像素／行、连续 phase 和 momentum，零位移结束同样收尾。几何读取最近 Accepted 的布局和真实 InputArea 变换，回弹不能发出额外用户意图。

触点由受控 native 队列进入真实窗口，不能代替触屏硬件体验；其他平台与真实设备的最终范围见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)。需要普通应用身份时先用 --no-run 构建，再按 [Surface README](../../../scoop.surface/README.md) 运行测试应用包。
