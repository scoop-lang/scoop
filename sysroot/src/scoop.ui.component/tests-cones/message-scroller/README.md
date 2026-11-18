# 消息列表的阅读与运动验证

使用已准备的开发 sysroot 和当前 UI／Style／Component 产物，在 macOS 图形会话分别运行；每个用例需要独立的 Application 进程：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/message-scroller --release --filter 'message turns preserve reading'
scoop test sysroot/src/scoop.ui.component/tests-cones/message-scroller --release --filter 'message reading holds through moving content'
```

基本场景覆盖原生输入、prepend、流式增高、恢复、末端留白和静止停帧。运动场景通过带明确时间的受控 native contact 队列，验证离开末端后进入 Holding、运动中 prepend 或可见行增高保持阅读位置、Following 在回弹时接收新消息，以及惯性实际回到末端后恢复自动跟随。数据收缩造成的末端位置继续 Holding；显式按消息定位结束旧惯性，丢弃帧不提前替换阅读位置，卸载后不保留帧需求。

测试使用当前测量与离屏估计形成的真实滚动范围；它不代表真实触屏／笔或其他平台验收。需要普通应用身份的宿主先使用 `--no-run` 构建，再按 [Surface README](../../../scoop.surface/README.md) 运行本地测试应用包。
