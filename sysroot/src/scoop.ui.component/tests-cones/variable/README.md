# 可变尺寸列表与阅读锚点验证

使用已准备的开发 sysroot 和当前 UI／Style／Component 产物，在 macOS 图形会话分别运行；每个用例需要独立的 Application 进程：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/variable --release --filter 'variable lists preserve reading keys'
scoop test sysroot/src/scoop.ui.component/tests-cones/variable --release --filter 'variable reading anchors survive inertia'
```

基本场景验证按 key 定位、恢复、prepend、行高和视口变化、末端跟随、留白、丢弃帧及卸载。运动场景通过带明确时间的受控 native contact 队列进入真实窗口，检查惯性期间的阅读锚点、回弹中的末端跟随、显式定位优先级、范围收缩与空列表；十万项来源保持有限的同时实现数量。数据变更以 Accepted 几何和修正通知核对，惯性采样不产生新的用户定位意图。

离屏的可变尺寸项仍使用来源的估计值，实际实现后再按测量结果修正。测试不能将尚未实现的 prepend 项当作已知真实尺寸，也不能用纵向滚动请求断言横轴必须到达最大位置。

这些用例不代替真实触屏／笔或其他平台验收。需要普通应用身份的宿主先使用 `--no-run` 构建，再按 [Surface README](../../../scoop.surface/README.md) 运行本地测试应用包。
