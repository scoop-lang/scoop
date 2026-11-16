# UI 无障碍接口规范

状态：S2 实施中。本文规定公共语义资源、逻辑更新和 Accepted 场景的接入；底层值及原生发布见 [Window 无障碍接口规范](SCOOP-WINDOW-ACCESSIBILITY-SPEC.md)。内容合并、复杂控件关系、完整文本与集合、内置控件及其他平台继续按[统一计划](../ui/interaction-polish/PLAN.md)推进，不据本批基础接口声明完整 accessibility 已完成。

## 1. 资源与父子关系

`ComponentScope.semantics(role, parent = None): SemanticsHandle` 在构造／挂载阶段创建语义资源；首次登记时为当前根创建 Window provider，根关闭时关闭它，保留借用的原生 Window。没有语义声明的根不创建空树。当前工厂按 Window 能力在 Darwin 提供；SemanticsHandle、路由与场景实现不含平台分支。未来平台使用同一声明，不增加并行接口。

`SemanticsHandle` 为 scope 管理的 Closeable，提供只读 id、role、isClosed 和 geometry。id 使用 Window 的 SemanticNodeId，重排及属性更新保持身份。角色固定；替换角色创建新资源并关闭旧资源，不能让旧原生引用指向新角色。调用方可提前 close，卸载仍自动清理，重复 close 无操作。

一个挂载可以登记多个节点。缺省父节点来自最近祖先挂载的有效子项路由，最终落到 provider 根；本挂载的多个节点默认是兄弟，顺序为登记顺序。显式 parent 必须属于本挂载或其逻辑祖先、在同一活动根内；不允许指向后代挂载或其他根。自身内部嵌套通过同一挂载中先创建的父节点表达。

`ComponentScope.routeSemanticsChildren(parent: SemanticsHandle?)` 指定普通子挂载使用的语义父节点；Some 要求资源属于本挂载，None 恢复从祖先继承。它不移动本挂载中其他节点。普通 Component、布局、方向包装与没有登记节点的 Element 默认透明；阅读顺序沿实际挂载子项／数据顺序，独立于场景层叠顺序。Lazy 仅处理已挂载条目，不为发布普通树枚举未实现的数据项；完整虚拟集合另按 S7 接入。

关闭单个 UI handle 只退休这个资源。框架先把仍由其他活动 scope／handle 拥有的直接语义子节点接到其原父级，清除已关闭的子项路由，然后按当前挂载关系重新发布；不会因替换父角色而误退役仍存活的子控件。显式子节点对已关闭父资源的引用同样移向原父级。真正卸载时，该挂载及其后代各自的资源全部退休。UI 不暴露底层 AccessibilityNode，避免绕过此归属规则。

## 2. 属性、动作与逻辑发布

`handle.update(properties, actions = SemanticActions(), relations = SemanticRelations())` 替换 Window SemanticProperties、最新动作与关系；关系使用同一活动根的 handle.id，不改变角色、身份、父子归属或业务状态。检查、名称／说明／错误解析、只读／禁用／受保护文本规则沿底层公共合同。名称或值改变立即使旧几何失效；只替换同能力处理器不要求绘制，也不重复原生属性通知。render／measure／scene／draw 不能创建、更新或关闭语义资源；Scene 只登记几何，挂载构造与正常 update 可修改资源。

标签文字变化经底层关系依赖更新引用者，无需重建消费它的 Element 或请求 drawable；显式空名称优先于标签。单独关闭 UI handle 后指向它的 labelledBy／describedBy／errorMessages／controls 自动清除，handle.relations 只读返回当前关系，不按相同位置或新角色转接到替代节点。bindFocus 保留当前关系，不重新提交已清理的旧引用。Scene 捕获并核对底层 contentRevision，标签依赖变化后的迟到 Accepted 场景不能恢复旧几何。内容合并／忽略策略继续独立实现，关系不自动吞掉标签的交互子项。

属性、父子关系与实际焦点在根的一次正常逻辑更新结束后一起发布。独立 handle 更新也通过现有 UiHost 队列唤醒该根，不请求 drawable，不增加另一套调度器。结构未变时不重新遍历挂载树；同一轮重复修改按最终值合并。失活／关闭立即拒绝旧动作，不等待下一帧。

每个已登记动作先检查根与节点生命期、所属 InputScope 的实际输入边界和当前 UI 执行状态，再经 Surface 的同步错误边界调用现有业务 handler。没有登记动作时不公布对应能力。动作中写入 State 后执行已排队的正常更新，发布实际结果再返回；不模拟指针事件，也不绘制或 pump 原生循环。UI 正在 render／布局／宿主派发时返回 Busy；异常关闭对应根并沿现有错误路径处理。handler 可关闭自己或根，正常完成仍按已经执行的动作返回 Completed，不能继续解引用退休的节点。

`handle.bindFocus(target: FocusTarget?)` 借用本挂载的实际 FocusTarget，None 解除绑定；同一目标不能同时绑定两个活动语义节点。绑定使 provider 的 focused 反映真实活动键盘焦点，查询不改变焦点。若 actions 未显式提供 focus，绑定提供经 FocusHandle 的聚焦／解除焦点动作；需要后续布局显露的请求返回 Busy，直到实际获得焦点才可返回 Completed，不把排队成功当成聚焦完成。目标关闭后不再报告焦点，动作返回 Unavailable。显式 focus handler 仍使用绑定目标作为实际焦点事实。

## 3. 场景与几何

`SceneContext.semantics(handle, bounds: Rect)` 只接受本挂载的活动 handle，在当前 Canvas 坐标与裁剪中捕获独立几何。每个节点每帧只能登记一次；不创建 InputArea。登记作为 SceneCommand 参与缓存记录和重放，重放时重新应用当前祖先变换与 clip，不保存上一帧的输入位置。opacity 为零或没有绘图不自动隐藏语义节点。

根统一在 FrameCompletion.Accepted 发布候选几何；Discarded 释放候选，保留仍匹配当前语义属性的上次几何。尚未呈现、当前 Accepted 场景未登记或名称／值已变化的节点报告 None；被自身 clip 完全裁掉时，保留完整 bounds，并以 visibleBounds=None 表示裁剪。后续匹配场景恢复几何，节点身份不变。帧生成后发生名称／值变化或节点退休，旧候选不能重新使其几何有效。

几何来自 graphics 的 CanvasGeometry，与真实绘制使用同一变换和精确 clip 交集；Window 只取得窗口逻辑坐标的 bounds／visibleBounds，不据矩形中心承诺可点击点。本批不公布尚未接入的原生精确命中能力。滚动条留白、滚动变换、Portal 和其他场景变换均在登记点生效；完整 Reveal 与虚拟化能力继续使用既有滚动／焦点路径。

## 4. 验证边界

用真实第三方 Element 验证无 InputArea 的文本、多节点、透明挂载、子项路由、重排／角色替换、最新 handler、禁用／隐藏／模态限制、自删与关根、跨根错误和旧对象退休。无 drawable 的属性更新必须独立验证；场景缓存命中、Accepted／Discarded、裁剪、未放置和属性与候选帧不匹配必须通过实际场景验证。

原生方法的进程内调用、外部 AX 客户端与 VoiceOver 分别记录。检查与构建不能代替实窗通过；其他平台只在其 provider 与实际工具验收后声明支持。
