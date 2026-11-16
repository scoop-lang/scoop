# Window 无障碍接口规范

状态：S0／S2 实施中。本文规定已有窗口内容的低层语义节点、AppKit 投影与同步动作边界；UI 资源与场景适配见 [UI 无障碍接口规范](SCOOP-UI-ACCESSIBILITY-SPEC.md)。内置控件、关系／内容合并、文本范围、虚拟集合和 Windows／Linux provider 继续按[完整实施计划](../ui/interaction-polish/PLAN.md)推进，不能以本批接口代替这些完成条件。实际构建／运行结果见[实施记录](../ui/interaction-polish/PROGRESS.md)。

## 1. 包、身份与寿命

`scoop.window.accessibility` 属于 Window Cone，不依赖 Surface、UI 或控件。SemanticRole、SemanticProperties、SemanticActions、SemanticActionResult、SemanticGeometry 与 SemanticNodeId 为公共值；AccessibilityProvider 和 AccessibilityNode 为 owner 线程资源。首批 `Window.createAccessibilityProvider()` 在 Darwin 暴露并实际连接 AppKit；其他平台沿同一接口接入自己的原生实现，不放置空 provider 或成功占位返回。

每个 Window 同时至多有一个内容 provider。createAccessibilityProvider 要求 Open；重复附着报 IllegalStateException。createNode(role, parent = None) 创建节点并返回 AccessibilityNode，parent=None 表示内容根下的直接子项。角色在该节点生命期固定；更换角色需退休并重新创建。节点 id 的窗口所属标识独立于原生地址，序号在窗口生命期递增、不复用；关闭 provider 后重新附着仍延续序号。不同应用运行产生的 WindowId 数字相同也不使旧语义 id 相等。

节点的 parent 与 order 表达语义阅读顺序，同序号按创建次序排列，不依据屏幕坐标排序。reparent(parent, order = 0) 要求父节点在同一活动 provider，拒绝自身或后代为父；失败不改变原关系。close 退休整个节点子树，随后修改节点或读取 geometry 报 IllegalStateException；id、role 和 isClosed 仍可读取，重复 close 无操作。provider.close 退休全部节点；Window 进入 Closing 时立即关闭 provider，不等待 drawable 或业务回调。已退休的原生对象可以由客户端继续持有，但不再读取或保活 Window／UI，不能执行动作。

## 2. 属性与发布

首批角色为 Group、Text、Button、Checkbox、TextField、ScrollArea。SemanticProperties 保存 name、description、automationId、value、enabled、readOnly、protectedText、checked 与 hidden；名称、说明、标识和值采用可选字符串，已知空字符串与未知值不同。checked 为可选 Off／On／Mixed。只有实际文本用途才设置 protectedText；受保护时不允许通过 value 发布明文，不因屏幕暂时显示正文而解除保护。hidden 排除整个节点子树但保留身份，恢复后使用原身份；它不表示仅忽略当前节点。update 在改变原状态前检查这些规则。

update(properties, actions = SemanticActions()) 替换属性和最新动作实现。值／名称变化使旧几何失效；新的匹配布局可通过 geometry 再发布。SemanticGeometry 保存窗口内容逻辑坐标中的 bounds 和可选 visibleBounds；后者必须在 bounds 内，且两者坐标有限、尺寸非负。None 表示几何不可用；visibleBounds=None 表示已经有合法布局但被自身裁剪，不表示节点退休或程序化隐藏。宽度或高度为零的可见矩形不进入 visible children，仍保留合法布局与语义身份。矩形仅描述外接区域，不能据矩形中心推断任意自绘形状的可点击点；精确场景命中在 UI 接入批次提供。

创建、修改和重排先修改 managed 状态，publish() 在正常 owner 更新完成后原子发布本批差量；未变节点不重复序列化和验证，单纯替换相同能力的 handler 不更新 native 属性。原生 getter 只读取已发布投影，不调用 Scoop handler 或布局。原生准备失败保留上一份投影，差量仍待处理；已经原子提交的结果不因随后平台通知失败回滚，错误按普通 native 操作失败报告。退休立即禁止旧对象动作，结构变更在 publish 时统一通知；新节点发布前不进入原生树。几何合法性在设置 geometry 的边界检查，失败保留原值。

focused 属性保存当前实际获得键盘焦点的活动节点或 None。它不代表辅助技术阅读位置，也不因为 Focus 请求已排队就改变。publish 先发布属性、结构和实际焦点，再发送适用的结构／值／焦点通知；无差量时不通知、不请求帧。几何仍由调用方使用匹配的 Accepted 场景提供，provider 不替调用方启动绘制。

## 3. 动作与同步边界

SemanticActions 按实际实现提供 invoke、toggle、setValue(String)、focus(Boolean)；focus 的 true 请求聚焦，false 请求解除该目标焦点。每个回调返回 SemanticActionResult：Completed、InvalidArgument、Unsupported、Unavailable、Rejected 或 Busy。缺少回调时不公布对应原生能力；enabled=false 拒绝交互，readOnly=true 拒绝值写入及切换。只有实际业务已执行才返回 Completed；Binding 拒绝不能提前改写投影，异步业务结束也不属于 Invoke 的完成条件。

AppKit getter 不进入 managed 代码。同步动作使用创建 provider 时登记的 Reusable ForeignCallback，沿已有 Context 捕获规则恢复创建上下文，包括 asyncio 集成的 scope。它只在 owner、应用未退出、窗口 Open 且输入未被模态阻止、没有进行中的 Window 业务／watch 回调或另一动作时执行。重入时返回 Busy，不等待自己的队列、不 pump、不布局；UI 适配另检查宿主派发、scope 回调、render／布局和实际输入边界，见 UI 无障碍规范。普通 handler 仍沿既有异步派发路径。

动作先保存回调与所需值，再执行；回调可以退休自身、关闭 provider 或 Window。执行后仅按资源仍存活的情况发布实际结果，不继续使用旧节点。异常在 managed 边界捕获，经正常应用错误与退出路径报告，不跨 C／Objective-C ABI；UI 后续将自身动作包在既有 Surface／根错误边界中。动作中可能打开的原生菜单／面板继续使用已有延迟 native operation 通路。

NSAccessibility 的 performPress 等 BOOL 动作仅在 Completed 时返回 YES。setAccessibilityValue 与 setAccessibilityFocused 为无返回值 setter：只在实际回调中改变业务状态，失败时保持旧值；调用方必须读取结果，不能把 setter 返回解释为写入成功。原生注册在释放 callback 前撤销；动作中关闭时，callback ownership 经现有 owner control 队列在调用退栈后释放。客户端晚到查询／动作不会访问已释放 context。

## 4. AppKit 投影与验证

使用 NSAccessibilityElement 与正式 NSAccessibility protocol，连接到现有内容 NSView。原生 Window、菜单和对话框继续提供自己的树；内容 view 不重复导出同一组节点。角色、名称、说明、identifier、值、enabled、checked、protected、焦点与实际动作由已发布节点决定。窗口逻辑矩形通过 AppKit 的 view／window 转换为屏幕坐标，窗口移动无需重写所有节点几何。

本批定向验证必须覆盖实际原生查询、动作结果、受控拒绝、发布顺序、重入拒绝、窗口／provider 关闭和客户端保留旧对象；外部 AX 客户端与 VoiceOver 的实际结果分别记录，直接调用 NSAccessibility 方法不冒充跨进程验收。完整文本范围、滚动动作、关系／内容合并、精确命中、虚拟集合、三平台与真实工具任务仍执行 S2～S11 的相应验证。
