# 共享直接滚动、惯性与弹性边界

使用当前开发 sysroot 和依赖产物，在 macOS 图形会话分别运行以下用例；每个用例使用独立的 Application 进程：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'direct scroll composes'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'direct scroll yields'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'elastic contact boundaries'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'elastic native scrolling'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter nativeMomentumReboundsAtTheBoundaryBeforeItsEnd
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'contact inertia shares'
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release --filter 'contact inertia transfers'
```

原有场景以明确 Hard 边界检查嵌套位移、阈值、缩放、点击／拖动归属及取消。弹性场景检查合法范围、内外层唯一拉伸、反向恢复、Contain、短内容、内容尺寸变化、配置替换、减少动画、禁用和离屏停帧；原生 wheel 场景区分离散像素／行、连续 phase 和 momentum，零位移结束同样收尾。几何读取最近 Accepted 的布局和真实 InputArea 变换，回弹不能发出额外用户意图。

`nativeMomentumReboundsAtTheBoundaryBeforeItsEnd` 检查惯性撞边立即回弹、尾段不重启 spring、回弹完成后不重复弹起、接触释放到 momentum 的衔接、反向滚动、双轴独立消费及新手势取消。用受控原生滚轮事件保留真实相位和队列路径，实际触控板手感另做人工验证。

两项惯性用例用带明确时间的原生 Touch／Pen 样本覆盖 18／31 步：父子共用停止距离、来源与祖先的独立开关、Contain、缩放、停住后释放、系统 momentum 去重，以及一轴到边界而另一轴继续、余速进入 spring。释放后 claim 清理，新输入、显式定位、根减少动画、禁用、公共 Pan.cancel、配置／尺寸变化及关闭都必须停止旧运动；惯性帧不增加用户定位意图。其他用例明确关闭软件惯性，只检查各自的直接位移或边界行为。

触点由受控 native 队列进入真实窗口，不能代替触屏硬件体验；其他平台与真实设备的最终范围见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)。需要普通应用身份时先用 --no-run 构建，再按 [Surface README](../../../scoop.surface/README.md) 运行测试应用包。
