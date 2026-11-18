# 语义节点显露

使用配套开发产物，在可呈现的 macOS 桌面运行普通显露回归；该路径不请求激活或聚焦窗口：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility-reveal --filter SemanticRevealWithoutActivationTest --release
```

用例从公开的 NSAccessibilityCustomAction 取得真实 action／target／selector，并在正常 native owner 边界调用。目标是没有 InputArea 或 FocusTarget 的 Text，位于两层带 ScrollPadding 的视口中；编辑器保留原有反向选区。已有 SemanticRevealTest 单独覆盖活动编辑焦点，需在允许取得焦点的环境按该名称过滤运行；每个应用场景使用独立进程，普通显露变更无需重复它。

验证最内层至外层的 Accepted 显露、原生首次返回未完成、完成后重试成功、窗口调整导致的 Discarded、逻辑滚动范围和实际矩形；查询动作列表不滚动。禁用、隐藏、忽略、InputScope 禁用、自有裁剪、未放置和名称失效均拒绝，静止状态等待真实帧需求停止。

新滚动意图、视口换绑和内容变化结束旧请求；显式业务 reveal 覆盖默认实现。根在请求返回后关闭，已经保留的原生自定义动作必须拒绝，编辑宿主与滚动资源正常收尾。

这是进程内公开原生协议的实窗验收，不代替外部 AX 客户端或 VoiceOver。
