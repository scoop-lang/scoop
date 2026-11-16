# UI 语义场景

本用例需要已解锁、能够呈现和获得焦点的 macOS 桌面。使用配套工具链和当前开发产物：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility-scene --release
```

复用逻辑用例的公开 NSAccessibility 查询辅助，在独立 Application 中验证真实 Accepted 场景、缓存重放时的新祖先变换／裁剪、零透明度、未放置后恢复、名字变化时清除旧几何、原生焦点请求及 FocusTarget 退休、身份保持和重新调整窗口时的 Discarded 路径。窗口系统允许调整请求在当前帧之后应用，因此 Discarded 次数为零或一；收到 Discarded 时必须保留上次匹配几何。

重叠的矩形与带孔 Path 使用真实屏幕坐标调用 accessibilityHitTest，验证路径孔洞、裁剪边界、Rect 缓存重放、Path 每帧捕获、未放置及名称失效后的撤下，以及场景层叠重排后仍保留原生节点身份。候选位置在 Accepted 前不可见，Discarded 同时保留矩形与路径的旧快照。

两种几何由持有语义资源的 Component 的后代 Element 登记，焦点继续绑定原 Component 的 InputArea；同时验证其他分支／后代资源和重复登记拒绝。缓存重放不改变这套归属规则。

零透明度检查后恢复可见绘图，使缓存具备可重放的有限 Drawing；跳过后恢复的子树在 Accepted 后才重新取得帧参与资格，这次资格变化会使先前非活动场景失效一次。用例先完成恢复帧，再单独检查名称变化与窗口隐藏／显示：逻辑属性及时发布，匹配的矩形几何通过缓存重放恢复，不把透明绘图的裁剪或重新参与场景产生的正常失效计入名称更新。

`--no-run` 只构建可执行文件，不能据此记录实窗通过。该用例不代替外部 AX 客户端和 VoiceOver 验收。
