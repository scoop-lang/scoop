# 浏览状态的原生直接输入

使用公开 TextInputStarter 和 TextEditHost。浏览区域先接收真正的原生文本，首个 Commit／Preedit 才创建编辑状态及实际编辑宿主；同一原生连接在真实焦点进入编辑器时继续，不保留隐藏文本框。

39 个原生阶段验证首个 Unicode 提交、后续文字、单次撤销／重做、空提交、拒绝开始、延迟挂载期间的连续 preedit、提交时的策略，以及外部改写、取消、失焦、禁用、目标关闭和根关闭。原生文档与候选位置分别核对，候选位置使用新编辑器的真实 Accepted caret。

Shift＋Space 浏览快捷键及其重复按键消费后调用 `cancel()`，已经排队的空格不会创建编辑器。另一个场景在首批文本排队后、`onStart` 之前连续取消两次，核对旧批次失效且只重建一次连接；之后的普通输入仍能正常接管新文档。

两轮系统拼音通过 CGEvent 键盘路径从浏览态输入 nihao，核对首次组合已经进入真实编辑器、连接未重建、Escape 取消和空格选词。结束时恢复原输入源。受控的 NSTextInputClient Unicode／preedit 场景单独保留，不把受控字符串提交当作系统输入法验证。

先使用匹配源码的本机预编译 Surface／UI 产物，然后执行 `scoop check --tests --release .` 与 `scoop test --release .`。桌面需已解锁且测试窗口能取得焦点。本场景只记录 macOS aarch64 结果，不代替其他平台或 VoiceOver 验证。
