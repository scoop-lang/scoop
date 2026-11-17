# 直接接触滚动的窗口验证

使用已准备的开发 sysroot 和当前 Surface／UI 产物，在 macOS 图形会话运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/scroll-pan --release
```

窗口用例通过受控 native contact 载荷进入真实 Window 队列，再经过公共解码、Surface 路由和 UI ScrollPan；不代表 macOS 触屏硬件，也不代替 Windows／GTK 的原生设备验收。需要普通应用身份的宿主可先用 `--no-run` 构建，将可执行文件放入本地测试应用包，再按 [Surface README](../../../scoop.surface/README.md) 的方式启动。

单个 Application 中的 48 步场景覆盖 tap 与阈值、嵌套 Chain／Contain、硬边界反向、锁轴与自由双轴、变换后的位移、子 Drag 独占、第二触点、被拦截的移动／结束、外部定位与回调内重入、配置变化、视口换绑、禁用和关闭。Accepted 帧与实际队列收齐后才推进断言，结束核对区域、视口、行为及 Window 资源收尾。

独立的 13 步边界场景覆盖 `claimAtBoundary=false` 在首尾边界让出、反向移动不恢复旧候选、已开始的 Contain 滚动保持归属，以及选项更新取消活动接触。两项 Application 用例应分别按 `--filter` 启动独立进程。
