# Window accessibility 边界测试

本 Cone 在 macOS 使用真实 AppKit 语义对象，验证低层 provider 的属性发布、状态与角色、UTF-8 值写入、实际回调结果、Context、重入、节点／provider／窗口退休和异常。它不绘制 UI 控件，不作为 VoiceOver、Accepted 场景或完整 UI 接入通过的证据。

普通 `scoop test . --release` 在独立的原生主队列任务中执行 NSAccessibility 方法，确保动作不被业务回调的重入保护误接收。所有 native 检查失败均返回对应源码行号；最后一个动作故意抛出异常，验证正常退出和 callback 退休。

跨进程验证使用本目录的 `native/client.c`，通过公开 AXUIElement C API 连接测试进程，按实际 identifier 查找并执行动作。不需要新增服务或第二个编辑模型。先在仓库外编译客户端，再设置 `SCOOP_ACCESSIBILITY_CLIENT` 为其绝对路径运行同一个 Cone：

```sh
clang -std=c11 -Wall -Wextra -Werror native/client.c \
  -framework ApplicationServices -o /absolute/work/ax-client
SCOOP_ACCESSIBILITY_CLIENT=/absolute/work/ax-client scoop test . --release
```

客户端必须具有系统 Accessibility 权限；测试只检查现有权限，不弹出提示或修改系统设置。客户端完成一次请求后退出，owner 按实际结果继续下一个场景；最终异常收尾仍由进程内调用覆盖，避免外部测试进程因应用退出丢失完成通知。外部客户端退出状态与 AppKit 方法测试分别记录。

外部客户端会验证按钮／切换、拒绝及成功的文本值设置、聚焦与解除焦点、隐藏／禁用，以及自身、provider 和窗口退休后的保留引用。完整文本范围、虚拟条目、关系、精确命中和读屏器任务沿 UI 总计划继续。
