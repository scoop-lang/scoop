# 内置控件无障碍语义

使用当前 macOS 工具链和开发 Cone 产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/accessibility --release
```

测试挂载真实 Label、Button、Checkbox、Input／Textarea、SelectableStyledText 与 Typography，以公开 NSAccessibility 方法查询和执行动作；ButtonGroup、InputGroup 参与实际组合。覆盖静态标签合并、独立交互后代、名称与关系覆盖、只换回调、纯文字比较后的名称更新、关系目标退休后的继续更新、三态与拒绝 Binding、行内 alternateText、Unicode 选区、文本写入及 undo、只读与保护用途、隐藏及关闭后旧引用。只读选择还覆盖替代文字映射、最新业务回调仅调用一次、禁用选择、拒绝写入，以及 Typography 两分支的覆盖值与旧对象退休。隐藏窗口期间先验证逻辑更新不依赖测量或绘图。

该用例不依赖实际鼠标事件、焦点或窗口呈现，也不代替外部 AX 客户端、VoiceOver 和实窗几何验证。后者继续使用 UI 的 accessibility-scene 及组件交互用例；构建成功不记为相应交互通过。

Field 场景复用同一 Application，检查自动标签、说明和去重错误、静态标题的显式目标、独立 FieldLabel／FieldText／FieldError、自定义标签里的按钮、显式名称／空串／关系优先级、控件与注释分别换绑，以及兄弟注释关闭／重建后控件仍及时更新、目标禁用时自定义标签的子按钮同步不可用。整个过程保留原输入节点和文本模型，不为测试另建应用循环。

ScrollArea 的默认容器角色、命名覆盖和内容路由也在无 drawable 条件下检查；未取得 Accepted 范围时不提供横／纵滚动条对象。实际滚动、隐藏条的能力及重新绑定见相邻 scroll-accessibility 场景。

必填场景覆盖 Input、Checkbox 与 Button 的真实 Field 配置，动态启停、控件与注释换绑、独立来源关闭／重建、多来源中仍有一个必填时保持状态、禁用时保留必填、只读属性能力和旧对象退休；范围与文本模型保持原有事实。
