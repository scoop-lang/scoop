# UI 语义逻辑更新与原生动作

使用配套工具链、当前 UI 和 Window 开发产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility --release
```

用例只有一个 Application，先从可见窗口保存真实 NSAccessibility 对象，再隐藏窗口，验证 State 驱动的名字更新和直接 handle 更新仍发布，期间 measure／scene 不运行。之后恢复可见性，验证原生 Invoke／SetValue／ShowContextMenu、最新 handler、完成／拒绝与只读、disabled／hidden、透明挂载及阅读顺序、跨根／后代 parent 拒绝、父角色替换、动作自删／重建、根解除／重附着、动作及普通 scope.dispatch 中的重入、自关根和异常错误边界。ShowContextMenu 在这里验证协议动作和资格；实际菜单会话由[文本编辑实窗](../editing/README.md)验证。

原生辅助只调用公开 NSAccessibility 方法，持有原生对象以检查退休后的行为；不读取生产私有结构。动作从原生主队列发起，完成后回到普通 owner 队列检查，不在业务回调中冒充外部同步动作。这个测试不要求收到 drawable，但也不验证 VoiceOver 或跨进程 AX；几何、焦点与缓存重放使用独立的 [accessibility-scene](../accessibility-scene/README.md)。

字段通过 labelledBy 引用另一个挂载中的静态文字；隐藏窗口中的直接标签更新必须改变原生字段名称，不能要求字段重新 render 或产生一帧。关系指向其他根时在修改前拒绝，读回的关系保持原值。
