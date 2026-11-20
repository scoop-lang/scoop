# Window 拖放接收测试

Darwin 用例使用真实 Application／Window 与公共 `registerDropTarget`，原生辅助在主队列调用 NSDraggingDestination 的公开协议方法，使用独立 NSPasteboard、延迟数据提供者和真实临时文件。辅助对象提供 NSDraggingInfo 元数据，不读取 Window 内部资源，也不修改系统剪贴板。

测试覆盖文本／空文本／URL／完整文件列表、同项多表示、操作协商、元数据阶段不读正文、无损读取与拒收、handler 替换、禁用、关闭、迟到方法、原生重入、数据读取期间变更、Context 与应用失败收尾。临时 pasteboard 和文件在 finally 清理；保留旧 NSView 只用于验证资源退休。

Windows 定向回归使用不激活的屏幕外 Window，原生辅助登记生产 OLE 接收对象并调用真实 IDropTarget 方法。来源在 Enter 后模拟暂时拒绝跨 STA 格式查询，验证鼠标移动及周期更新仍稳定协商文本／URL／多文件，修饰键和允许操作仍生效，正文仅在 Drop 读取；带非零尾部填充的文本／URL 仍按终止符完整读取。读取失败、查询期间禁用／关闭、读取期间关闭及退休对象调用仍正确拒绝。该用例不请求前台焦点，不启动可见人工验收示例。

```sh
scoop check sysroot/src/scoop.window/tests-cones/drag-drop --tests --release
scoop test sysroot/src/scoop.window/tests-cones/drag-drop --release
```

开发时另传配套的 `--sysroot` 和 `--cone-path`。进程内协议方法测试不等于真实 dragging session；source、同窗／跨窗路由、跨程序双向拖放、文本事务、窗口 DPI、触摸来源与其他平台仍按[统一计划 S6](../../../../../docs/ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)验收。

Linux 的 `LinuxDragInputTest` 使用独立 X11 显示器送入真实协议按压、移动与释放。它先取消待启动请求，再读取活动 GTK 拖放图标的像素、逻辑尺寸及指针相对位置。可用 `xvfb-run -a env GDK_BACKEND=x11 scoop test sysroot/src/scoop.window --release --filter LinuxDragInputTest --ignored` 运行；设置 `GDK_SCALE=2` 可验证双倍缩放。
