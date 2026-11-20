# UI 拖出准备与帧接入测试

macOS 的 release Cone 测试使用真实 UI 根、InputArea、Press、DragSource、Accepted 场景和 AppKit 输入事件方法。在仓库根执行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/drag-source --filter UiDragSourceSessionTest --release
scoop test sysroot/src/scoop.ui/tests-cones/drag-source --filter UiContactDragSourceTest --release
scoop test sysroot/src/scoop.ui/tests-cones/drag-source --filter UiDragSelectionPreviewTest --release
```

每个类在独立进程运行，窗口只提交场景，不请求前台激活。使用最近 Accepted 的平移／缩放几何核对来源局部位置和窗口阈值。鼠标类验证未达阈值保留点击、没有数据只准备一次、更新处理器保留本次完成回调、启动前取消一次完成、prepare 内取消、禁用与关闭 UI 根撤销回调，以及借用视图恢复。接触类通过明确标注的受控 payload 检查 Touch／Pen 身份、事件时间、逻辑阈值、拒绝数据保留点击、第二触点、阻止开始／移动、关闭输入开关和卸载；不把注入当作实际触屏或原生接触拖放。

选区预览用例读取真实拖放位图的 RGBA 像素，验证视口裁切、长行、两轴同时裁切、自然边缘及小数像素密度下的热点。

原生输入通过真实 view 方法注入，成功准备后在当前 Window 事件结束前取消或关闭请求，因此不进入 NSDraggingSession。这个测试验证 UI → Window 的准备及收尾，不代表实际 OS 拖动、外部应用或辅助技术任务已完成；它们使用 [双窗口示例](../../examples/drop-target/README.md) 分别验收。纯阈值与挂载资源测试另见 [drag_source.scoop](../../tests/drag_source.scoop)。
