# 文本拖放宿主接入测试

此 macOS Cone 使用真实 UiHost／TextEditHost／SelectableTextHost、固定测试字体和 Accepted 窗口帧，调用 SWView 的公开 AppKit destination 与鼠标输入方法。接收和来源场景在同一个 Application 中顺序执行。使用已准备的 sysroot 和当前 UI 开发产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/text-drop --release
```

覆盖窗口平移后的文本落点、Text／URL Copy、单行清洗与撤销、接收开关／只读／禁用、相同正文及修订下的模型重绑定、遮罩映射、composition 接受与拒绝、边缘滚动及 Leave 停止，以及接收回调关闭根后仍返回真实结果。预编辑注入等待真实窗口焦点及原生选区发布，不只依据绘制帧数判断输入连接就绪。使用独立命名的 pasteboard，不修改系统剪贴板；native 方法在 AppKit 主队列、普通 owner 派发栈之外执行，不绕过库的重入检查。

来源场景覆盖选区内普通点击、可编辑／只读、受保护默认拒绝与独立显式导出、Shift、多击、关闭拖出，以及 SelectableTextHost 的 Copy。真实选区绘制几何决定按下位置；达到阈值后检查已准备的原生数据与操作，再在系统会话启动前禁用来源，验证准备资源撤销与不误报完成。

运行需要可显示窗口并授予键盘焦点的 macOS 桌面会话。超时诊断包含阶段、Accepted 帧数、窗口／输入焦点及原生选区是否就绪；停在预编辑前置条件不能视为测试通过，也不能仅凭它判断锁屏。

协议探针没有创建 NSDraggingSession。该测试验证真实帧、目标协议与来源准备的接入；完整系统会话、跨程序数据、实际视觉检查和 VoiceOver 仍单独验收。纯模型策略、Unicode、长度限制、失败原样保持和 undo 分组见 [text_drop_edit.scoop](../../tests/text_drop_edit.scoop)；同文档 Move、来源正文修订、有损拒绝与删除事务见 [text_drag_edit.scoop](../../tests/text_drag_edit.scoop)。完整范围见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md#s6-三个范围的数据拖放与文本拖放)。
