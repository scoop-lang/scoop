# Window 拖放接收测试

该 Cone 使用真实 Application／Window 与公共 `registerDropTarget`，原生辅助在主队列调用 NSDraggingDestination 的公开协议方法，使用独立 NSPasteboard、延迟数据提供者和真实临时文件。辅助对象提供 NSDraggingInfo 元数据，不读取 Window 内部资源，也不修改系统剪贴板。

测试覆盖文本／空文本／URL／完整文件列表、同项多表示、操作协商、元数据阶段不读正文、无损读取与拒收、handler 替换、禁用、关闭、迟到方法、原生重入、数据读取期间变更、Context 与应用失败收尾。临时 pasteboard 和文件在 finally 清理；保留旧 NSView 只用于验证资源退休。

```sh
scoop check sysroot/src/scoop.window/tests-cones/drag-drop --tests --release
scoop test sysroot/src/scoop.window/tests-cones/drag-drop --release
```

开发时另传配套的 `--sysroot` 和 `--cone-path`。进程内协议方法测试不等于真实 dragging session；source、同窗／跨窗路由、跨程序双向拖放、文本事务、窗口 DPI、触摸来源与其他平台仍按[统一计划 S6](../../../../../docs/ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)验收。
