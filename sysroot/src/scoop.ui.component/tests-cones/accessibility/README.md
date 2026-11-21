# 内置控件无障碍语义

使用当前 macOS 工具链和开发 Cone 产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/accessibility --release --filter .ComponentSemanticsTest
scoop test sysroot/src/scoop.ui.component/tests-cones/accessibility --release --filter .AdvancedComponentSemanticsTest
scoop test sysroot/src/scoop.ui.component/tests-cones/accessibility --release --filter .ModalSemanticBoundaryTest
```

测试挂载真实 Label、Button、Checkbox、Input／Textarea、SelectableStyledText 与 Typography，以公开 NSAccessibility 方法查询和执行动作；ButtonGroup、InputGroup 参与实际组合。覆盖静态标签合并、独立交互后代、名称与关系覆盖、只换回调、纯文字比较后的名称更新、关系目标退休后的继续更新、三态与拒绝 Binding、行内 alternateText、Unicode 选区、文本写入及 undo、只读与保护用途、隐藏及关闭后旧引用。只读选择还覆盖替代文字映射、最新业务回调仅调用一次、禁用选择、拒绝写入，以及 Typography 两分支的覆盖值与旧对象退休。隐藏窗口期间先验证逻辑更新不依赖测量或绘图。

该用例不依赖实际鼠标事件、焦点或窗口呈现，也不代替外部 AX 客户端、VoiceOver 和实窗几何验证。后者继续使用 UI 的 accessibility-scene 及组件交互用例；构建成功不记为相应交互通过。

Field 场景复用同一 Application，检查自动标签、说明和去重错误、静态标题的显式目标、独立 FieldLabel／FieldText／FieldError、自定义标签里的按钮、显式名称／空串／关系优先级、控件与注释分别换绑，以及兄弟注释关闭／重建后控件仍及时更新、目标禁用时自定义标签的子按钮同步不可用。整个过程保留原输入节点和文本模型，不为测试另建应用循环。Field 正文包装在 enabled=false 的 ContextMenu 中，检查 Input、Checkbox、Button 仍可用，随后禁用／恢复 Checkbox 仍按其自身状态更新。 该回归在旧组件产物上于 Input 可用状态断言失败，修复后在本机 macOS aarch64 的 release 配置下约 1.01 秒通过；相邻菜单 Cone 的 80 阶段键盘／嵌套禁用回归约 18.97 秒通过。组件和两个 Cone 的 check／release 构建、公开 API 文档严格生成与查询均通过。

ScrollArea 的默认容器角色、命名覆盖和内容路由也在无 drawable 条件下检查；未取得 Accepted 范围时不提供横／纵滚动条对象。实际滚动、隐藏条的能力及重新绑定见相邻 scroll-accessibility 场景。

必填场景覆盖 Input、Checkbox 与 Button 的真实 Field 配置，动态启停、控件与注释换绑、独立来源关闭／重建、多来源中仍有一个必填时保持状态、禁用时保留必填、只读属性能力和旧对象退休；范围与文本模型保持原有事实。

`AdvancedComponentSemanticsTest` 覆盖 Slider／Progress 的实际范围与拒绝、Tabs／Disclosure／Trace 的选择与展开、OTP 单一文本宿主及保护切换、Calendar 的行与表头、合并单元格、分页动作，以及真实文本编辑和 undo 对保留范围的映射。保留范围检查验证共享文档机制与 UI 宿主接线，不表示新增了 AppKit AXTextMarker 协议。

Calendar 回归还在保留原生按钮和已选日期引用时切换翻月文案，检查当前页面和选择不变，并通过同一按钮向后、向前翻页，最后恢复默认名称。该检查读取真实 NSAccessibility 名称和动作，不依赖系统前台焦点。

翻月文案批次在本机 macOS aarch64 完成组件库 check／release 构建、测试 Cone 的 check／release 构建及 `AdvancedComponentSemanticsTest` 原生运行；约 1 秒完成，failures 为空且正常退出。生成的公开 API 文档可查询两个新字段。

Select 的 8 个值检查阶段覆盖独立的显式名称、Field 标签与无标签目标的名称回退、外部改选、禁用选项、空标签、未知值、None、动态标签和整体禁用。自定义显示不替代 Choice 的实际语义值，读取不构造 View，外部更新不伪造用户选择通知，始终检查原来的原生节点。

该 Select 回归及相邻 `select` Cone 的 41 阶段真实交互在本机 macOS aarch64 的 release 配置下通过。两套测试消费当前工具链中已准备的组件产物；交互回归覆盖键盘、定位、滚动、拒绝更新、关闭和生命周期。

`ModalSemanticBoundaryTest` 检查 Dialog 打开后背景旧引用不可操作、拒绝关闭时仍保持打开、默认 “Close” 按钮可按语义名称找到并实际关闭，以及关闭后原背景身份恢复。两个新场景都使用 `requestActivation = false`，不设置或重试系统前台焦点。GUI 测试类各自在独立进程中执行；构建和运行结果记录在 [实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
