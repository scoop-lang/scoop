# UI 文本编辑与原生交互

使用配套工具链和当前依赖产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/editing --release
```

macOS 实窗用例复用多个 TextEditHost 和一个 SelectableTextHost，覆盖实际键盘／指针、选择、组合输入、显示投影、滚动、原生菜单和生命周期。需要可以呈现和聚焦窗口的桌面会话；编译通过不代表运行通过。

编辑宿主通过 updateTextEditor 发布语义，只读宿主在正常 update 和选择通知中调用 updateSelectableText。两个菜单场景从原生主队列调用公开 accessibilityPerformShowMenu，检查返回结果、实际 tracking、Copy／SelectAll／编辑项目和真实模型／剪贴板变化。菜单与普通右键／键盘入口使用同一目标、锚点、策略和会话，仍覆盖只读／禁用、受保护用途、焦点／文档变化及迟到命令。

右键辅助事件显式设置原生按钮编号，避免 NSEvent 构造器保留左键编号；Shift+F10 携带 NSF10FunctionKey，避免把空字符串送入 AppKit 文本替换。每次菜单操作等待 PopupCompleted 后再进入下一阶段，保留原生 tracking 中的检查与修改，不把发送取消请求当成会话已经结束。禁用行为在宿主挂载期间检查，已卸载宿主另行检查调用拒绝；链接拖选从折叠选区开始，以覆盖普通扩选路径。

无需 drawable 的正文、选区、偏移映射和动作回归分别见[文本编辑语义](../accessibility-text/README.md)及[只读富文本语义](../accessibility-selectable/README.md)。实窗、外部 AX 与 VoiceOver 的实际通过范围见[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)，完整范围见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)。
