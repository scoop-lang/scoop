# UI 内容名称合并与忽略

使用配套工具链和当前 Window／Surface／UI 开发产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility-content --release
```

macOS 用例使用一个 Application 和十二个稳定语义节点。先保存真实 NSAccessibility 对象，再在隐藏窗口中更新 State；名称、合并与忽略必须无需 measure／scene 就能发布。临时恢复窗口可见性时读取公开导航关系，并从原生主队列执行实际动作。

覆盖纯标签合并、ignored 包装展平、hidden 子树排除、显式名称／labelledBy／内容的优先级、显式空串、按 key 重排，以及动作、FocusTarget 绑定和公开选区的动态资格变化。普通 Group、带子控件的文本、可聚焦／可选择文本和受保护文本保持独立；合并后增添真实动作的文字可以反向引用原容器名称，不被已经撤下的旧内容关系误判为名称环。解除合并或忽略恢复原身份，不重建挂载；忽略容器保留后代并拒绝自身旧引用的动作。

共用的 native 辅助只调用公开 NSAccessibility 协议，不读取生产私有状态。此用例验证普通 owner 更新与原生方法，不代替 Accepted 场景、外部 AX、VoiceOver 或其他平台验证；完整范围见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)。
