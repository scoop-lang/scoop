# UI 原生资源与 Accepted 区域组合

普通测试 Cone，用真实 UiHost、Application 根、窗口根和 renderer 验证共享 State、错误隔离、popup／modal／文件完成观察及最近提交的窗框区域。

```sh
scoop test sysroot/src/scoop.ui/tests-cones/desktop --release --sysroot sysroot
```

macOS 和 Linux 默认执行；Linux 分别设置 `SCOOP_WINDOW_BACKEND=x11`／`wayland` 并使用实际显示会话。Windows 的完整程序默认编译但忽略 GUI 用例，在交互桌面使用 `--ignored` 执行。安装后的 Windows CLI 名为 `sco`，仓库开发构建为 `scoop.exe`。

测试覆盖最终 transform／clip、Discarded 保留旧区域、窗框通知、运行中透明度、改尺寸后的 Accepted、绘制回调内切换窗框后的候选失效、空覆盖与未放置恢复默认、禁用／InputBoundary／卸载立即撤销，以及 UI 关闭后借用 Window 存活。应用根失败保留窗口，窗口回调失败保留应用根，关闭最后窗口后应用根继续更新；静止状态不持续请求帧。

原生探针只读取实际平台状态：AppKit 读取输入视图消费的原生命中快照和 CAMetalLayer.opaque，Win32 发送 WM_NCHITTEST 并查询 GetWindowRgn，X11 查询 ShapeInput。Wayland 无等价的全局输入区域查询，仍运行真实 GTK／Surface 提交和生命周期组合，不能把该结果记作精确系统命中或用户拖动验收。软件显示会话与 macOS loginwindow 下的原生快照验证不替代前台实际拖动、缩放、焦点及最终屏幕呈现。结果见[实施记录](../../../../../docs/desktop-ui/PROGRESS.md)。
