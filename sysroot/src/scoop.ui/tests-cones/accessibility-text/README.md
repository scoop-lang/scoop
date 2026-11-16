# UI 无障碍文本读取与编辑

使用配套工具链和当前 Window／Surface／UI 开发产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility-text --release
```

macOS 用例将同一挂载的 TextField 语义与 TextEditHost 通过 updateTextEditor 接入。共用 native 辅助只调用公开 NSAccessibility 协议；通过实际模型、策略计数、undo 和发布结果验证动作，不把无返回值 setter 的调用当作写入成功。

覆盖 emoji、组合字符与中文的 UTF-8／UTF-16 转换、方向选区、正文／选中文本／attributed string 读取、空范围、越界与 surrogate 中间端点拒绝；单范围设置和多范围拒绝、只读选择、禁用／只读写入拒绝、外来文本清洗、单行／最大长度／Reject、撤销、composition 收尾，以及受保护文本的读取限制和允许的全文替换。属性在隐藏窗口中无需 measure／scene 就能发布；选择不会抢焦点，关闭后保留的原生对象不能读到旧正文或修改模型。

本用例验证普通 owner 更新与进程内原生方法，不代替范围几何、Accepted 场景、外部 AX、VoiceOver 或其他平台验证；完整范围和实际结果见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)及[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
