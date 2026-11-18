# UI 文本编辑与原生交互

使用配套工具链和当前依赖产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/editing --release
```

macOS 实窗用例复用多个 TextEditHost 和一个 SelectableTextHost，覆盖实际键盘／指针、选择、组合输入、显示投影、滚动、原生菜单和生命周期。需要可以呈现和聚焦窗口的桌面会话；编译通过不代表运行通过。

`--filter 'selected glyph colors follow window activity'` 在两个真实窗口中取样编辑器与只读正文的选区背景、选中字形和未选文字。场景覆盖窗口切换、行内动作持有逻辑焦点、范围与前景热更新、关闭文字覆盖、折叠选区、静止停帧和卸载；实际字形取样位置来自同一固定字体的排版。外观切换保留编辑正文、撤销历史和未显式修改的修订，显式选择操作仍正常推进输入修订。

独立的文本触摸场景从 native 队列发送 Touch／Pen contacts，覆盖短按 grapheme 落点、长按选词和真实原生菜单命令、方向选区、内层／外层滚动及行内链接竞争。触摸开关、None 长按、菜单关闭、受保护投影、只读、缩放、阻止输入、第二触点、重绑、失焦和重入卸载均沿正常宿主验证；另从 accessibility 请求菜单，确认触摸配置不关闭语义动作。该受控事件测试不代替真实触屏设备验收。

多击场景在另一个独立窗口进程中运行，69 步覆盖编辑／只读宿主的词和逻辑行选择、第四次短按、Touch／Pen 来源切换、组合字符与 emoji、间隔／距离、显式关闭、失败或域外接触、Accepted 缩放、长按、区间投影和受保护遮罩。行内链接连续短按只激活动作；外部选区修改及选择回调中的修改／关闭会打断旧计数。可用 `--filter 'touch multi taps select'` 单独执行；原 66 步触摸场景使用 `--filter 'native contacts select text'`，键盘／鼠标／IME 组合使用 `--filter 'native editing system IME'`。

选区把手场景使用 `--filter 'touch handles adjust'`，从实际 caret 与绘制圆心发送接触，覆盖编辑／只读的 caret、两端、交叉、折叠、抓取补偿、组合字符与遮罩、缩放、父视口与编辑视口边缘滚动及结束停帧。阻止移动／结束、第二触点、外部修改和回调中的替换／关闭必须阻止迟到输入恢复旧选区；关闭把手仍保留普通短按。本场景继续使用真实宿主与 native 事件队列，不能代替真实触屏／笔的人工体验验收。

另外使用 `--filter 'touch handles follow RTL'` 验证 RTL 端点、同一批输入中的移动与抬起、系统取消、显式外部定位、禁用及半径／颜色定制。最终释放在等待新布局时保留最后的窗口位置，Accepted 后完成一次实际选区更新；旧接触不能覆盖取消后的模型。

编辑宿主通过 updateTextEditor 发布语义，只读宿主在正常 update 和选择通知中调用 updateSelectableText。两个菜单场景从原生主队列调用公开 accessibilityPerformShowMenu，检查返回结果、实际 tracking、Copy／SelectAll／编辑项目和真实模型／剪贴板变化。菜单与普通右键／键盘入口使用同一目标、锚点、策略和会话，仍覆盖只读／禁用、受保护用途、焦点／文档变化及迟到命令。

右键辅助事件显式设置原生按钮编号，避免 NSEvent 构造器保留左键编号；Shift+F10 携带 NSF10FunctionKey，避免把空字符串送入 AppKit 文本替换。每次菜单操作等待 PopupCompleted 后再进入下一阶段，保留原生 tracking 中的检查与修改，不把发送取消请求当成会话已经结束。禁用行为在宿主挂载期间检查，已卸载宿主另行检查调用拒绝；链接拖选从折叠选区开始，以覆盖普通扩选路径。

无需 drawable 的正文、选区、偏移映射和动作回归分别见[文本编辑语义](../accessibility-text/README.md)及[只读富文本语义](../accessibility-selectable/README.md)。实窗、外部 AX 与 VoiceOver 的实际通过范围见[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)，完整范围见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)。

`--filter 'elastic text viewport'` 单独验证文本回弹：固定输入视口覆盖空白边界，正文与 caret 一起移动，释放／根或局部减少动画后回到合法位置；连续像素滚动、显式 reveal 与卸载共用同一编辑状态和 Accepted 几何。

`--filter 'text inertia keeps accepted caret geometry'` 用带时间的原生接触检查直接拖动与释放减速的总距离。运动期间及停止后，caret 与合法 offset 使用同一 Accepted 变换，输入区域仍覆盖原视口，选区和只读外层位置不变；局部减少动画的热切换与持续启用、显式 reveal、卸载分别验证取消和停帧。原直接位移场景在编辑与只读父视口明确关闭软件惯性。
