# ScrollArea／VirtualList 的直接触摸验证

使用已准备的开发 sysroot 和当前 Surface／UI／Style／Component 产物，在 macOS 图形会话分别运行两个用例；每个用例需要独立的 Application 进程：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/touch-scroll --release --filter 'touch scroll area preserves'
scoop test sysroot/src/scoop.ui.component/tests-cones/touch-scroll --release --filter 'touch virtual list recycles'
```

测试通过受控 native contact 载荷进入真实 Window 队列、Surface 路由和实际控件。验证 Touch／Pen、tap 与自定义阈值、单独关闭触摸滚动、禁用／卸载取消、归属接管后不补点击，以及 VirtualList 在十万项来源中回收旧行。像素读回检查滚动条在手指停留时保持、抬起后隐藏；同一等待期间检查没有额外 Accepted 帧。

这些用例不代表 macOS 触屏硬件，也不代替 Windows／GTK 的原生编译、设备输入或辅助技术验收。需要普通应用身份的宿主先使用 `--no-run` 构建，再按 [Surface README](../../../scoop.surface/README.md) 运行本地测试应用包。
