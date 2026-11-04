# 公共交互、弹层退出与阅读位置

本文属于 [控件基础设计](DESIGN.md)，规定已完成的 I1／I2／S1 合同。复用已有 InputArea／InputScope、capture／target／bubble／default、事件完成通知、焦点、InputBoundary、WindowDeadline 和 Surface 动画。

## 1. 可组合的激活行为

Button／Checkbox 迁移到 UI 的公共 [Press](../../../sysroot/src/scoop.ui/src/interaction/press.scoop) 行为与 `InteractionState`。行为借用调用方创建的 InputArea 和 scope，拥有自己的处理器登记／临时状态；关闭行为不关闭调用方的 area。通过 ComponentScope 登记清理，更新时接收最新 enabled、按键策略和动作。

`InteractionState` 提供 hovered、pressed、focused；选中／展开／校验错误仍是控件模型，不混入一次按下序列。行为使用已有默认动作和事件完成通知，不覆盖 area 唯一的 handler，不额外创建重叠命中区域。

- 鼠标按下只建立候选；释放时仍满足区域、enabled 和序列归属才激活。移出／移回、失焦、禁用、卸载和取消均更新 pressed。
- 默认 Space 在真实释放时激活，Enter 在真实按下时激活；是否接受 Enter 可配置，Checkbox 可以不接受。重复按下和 synthetic release 不产生额外激活。
- 激活键使用 window 的 chord 规则；带额外应用修饰键的 Space／Enter 不误触发普通按钮。
- 传播中止或默认行为被阻止时，事件完成通知仍清理已结束序列，不能留下永久 pressed。
- 触发动作可以同步改变 Props、移除自身或关闭窗口；先收束当前序列，再调用最新动作，不在回调之后假定 area 仍存活。

已有 Button 增加显式内容 View 的组合能力，文字按钮只是普通组合入口；第三方可以把图标、文字、badge 和布局接到同一个 Press。既有文本入口和新内容入口共享实际实现，不用 `asChild` 克隆描述或合并隐藏属性。

## 2. 按下序列归属与拖动

Surface 以固定鼠标按下路径和取消通知为基础，提供一次序列内的显式 claim：调用者必须是原按下路径上的活动 area／scope；只能接管当前窗口中仍按下的对应序列。

claim 成功时：

1. 给原来更深的活动接收者发送一次取消，结束其 Press／拖选候选；不能等待 mouse-up 再猜是否发生过拖动。
2. 后续 move／release 交给 claim 所属路径；保留正常的逻辑祖先路由和事件完成通知。
3. 同一序列只有一个已确认归属，重复 claim 不造成多次取消；不允许无关节点偷取另一个按下序列。
4. 失焦、窗口关闭、area 禁用／卸载及原生取消都统一终止；取消后迟到的 release 不能激活动作。

这是 Surface 内部输入路由的接管，不是 OS 全局鼠标锁定，也不增加通用 gesture arena。实际窗口外的移动／释放仍依赖 window 已有的鼠标序列与取消合同。

UI 的轴向 Drag 行为在按下后先保留候选，移动超过明确的逻辑距离阈值并符合轴向条件后才 claim。未达到阈值时不抢普通点击；不符合轴向时放弃候选，允许现有滚动／祖先默认行为处理。默认动作按既有目标到祖先顺序决定哪个行为先取得序列，不引入另一套竞争优先级。

Drag 输出开始位置、当前位置、轴向位移、速度和结束／取消原因。所有量使用明确的逻辑坐标及现有单调输入时间；重复时间点不能产生除零或无限速度。布局、缩放和滚动改变时使用已提交几何更新坐标转换，不从未提交的位置生成拖动结果。

滚动条迁移到同一序列处理；范围和分隔条可复用它。自动边缘滚动只在确有推进需要时安排下一次 owner deadline，松开、取消或不能再滚动时停止。

### 2.1 范围与吸附计算

component 提供小型纯值计算，直接服务 Slider／Resizable／Carousel／Drawer 的后续组合：

- 范围位置与数值转换、clamp、方向反转、离散 step、多个 thumb 的有序值和最小间距。离散运算以整数步号为基础，避免持续浮点累加；非整步的 max 不隐式增加一个特殊 step。
- 连续模式不量化；最小间距和当前约束不能满足时在配置更新前报告参数错误。多 thumb 默认不穿越邻居，活动 thumb 不因数值排序被换成另一个身份。
- 调整相邻区域时，在两侧 min／max 内分配同一位移，保持可分配总长度；不由左右两边各算一遍导致长度漂移。
- Snap 接收实际布局产生的锚点和释放速度，选择有限目标，再用现有 Animatable／spring 推进。布局改变时重新解析锚点；新拖动从当前值接管，取消旧目标。

这些计算不持有 Window、不创建时钟、不渲染 thumb 或 slide，也不自动管理 Carousel 的项目模型和 Drawer 的打开状态。

## 3. 活动项、选择与 typeahead

当前 KeyNavigation 直接移动真实 FocusHandle。保留这一模式，并把“寻找下一项”与“如何应用结果”分开，增加活动项模式：

- 候选用稳定 ViewKey 表示，提供有序位置、enabled 和搜索文字。数据仍由已有模型／ItemSource 提供，不复制另一份列表。
- 真实焦点模式继续用于 roving Tab、Radio／Tabs 等组合；活动项模式只更新当前高亮 key，并按需显露它，文本 InputArea 保持唯一真实焦点及 IME 连接。
- 焦点、活动项和选中值分别表达。方向键可以只移动活动项；Enter／Space／鼠标是否提交选择由调用方决定。单选／多选使用已有 Binding 和纯值操作，不自动把 hover、focus 等同于 selected。
- disabled 项跳过；删除、过滤或重排后按稳定 key 保留仍存在的活动项，没有候选时为 None。一次导航只遍历当前有限候选集合，不因更新无限重试。
- 虚拟项通过现有 lazy focus／按 key 显露协议接入；新请求覆盖旧请求。活动项模式显露滚动位置时不能调用 focusItem 抢走输入焦点。

Typeahead 是 scope 所有的短期输入缓冲和候选搜索：有效文字追加、重复单字符可循环查找、超时清空，关闭／失焦／数据域替换时取消 deadline。搜索使用调用方提供的匹配函数和数据侧 searchText，不为搜索创建所有 View，也不在基础层加入 fuzzy search 引擎。

字符和缓冲均保留 Unicode；匹配函数明确决定大小写、重音及归一化规则，不能把 `lowercaseAscii` 宣称为完整 Unicode 匹配。普通非编辑列表使用允许的 logical 字符；带输入框的 Combobox／Command 直接消费编辑模型的提交结果，不再从 KeyEvent 拼一次查询文本。IME preedit 不作为已提交搜索词。

## 4. 悬浮组合与定位

提供可组合的 Hover 行为：触发区域与 portal 内容组成同一逻辑悬浮组，支持进入／退出延迟、焦点触发和明确的关闭。两个区域之间的空隙不立即关闭内容；离开时用当前指针方向与已提交弹层矩形计算有限的通过区域，并以短 deadline 兜底。

子菜单接近通道只影响是否延迟关闭，不产生全局鼠标捕获。指针离开窗口、失焦、owner 卸载或实际关闭立即撤销等待。输入观察只在行为活动时登记，复用 Surface 路由，不启动屏幕级监听或永久 timer。

定位继续使用现有 flip／shift、viewport padding、锚点可见性和逻辑祖先。补充窗口逻辑坐标中的 `OverlayPlacement.AtPoint`，服务右键 Context Menu 和指针附近提示；键盘打开的菜单使用当前焦点区域的已提交矩形。AtPoint 不是屏幕物理坐标，也不需要制造一个零尺寸假组件来充当 LayoutAnchor。

`OverlayLayout` 暴露最终 bounds、side、anchor 和可用范围，供内容绘制箭头及计算动画原点。逻辑 Start／End 按 [布局方向](LAYOUT.md#4-方向继承与逻辑几何) 解析；不由每个 Popover／Menu 重新写碰撞算法。

## 5. 关闭请求、退出呈现与最终移除

现有 `onClose` 发生时子树已经停用，无法再执行退出呈现。本轮将 portal 改为受控 open 值：调用方提供 `Binding<Boolean>` 和内容 View，Binding 是期望打开状态的唯一来源。框架只保存进入／退出的呈现阶段，不再保存另一份可独立变化的 open 值。

### 5.1 请求与实际状态

Escape、outside press、`OverlayHandle.requestClose` 提交带原因的关闭请求。可选 `onCloseRequest(reason): Boolean` 在写入 open=false 之前运行；默认接受。拒绝只表示保持打开，不撤销已经发生的输入消费。请求被接受后写入 Binding，实际 open 变为 false 才开始退出；受控 setter 未采纳变化时保持打开。

调用方直接把 open 改为 false 是已决定的状态更新，不再调用 veto 回调。owner 卸载、窗口关闭和无法维持的锚点失效属于强制收尾，不能被 veto 无限保留。锚点 Close 策略会请求把 Binding 置为 false；即使 setter 未采纳，也结束当前呈现会话，直到有效锚点下再次发生打开请求才重开，不在每帧循环请求关闭。Hide／KeepVisible 继续沿用明确策略，不把暂时裁剪等同于 owner 已销毁。

重复请求在关闭／退出期间合并。回调可以同步关闭根或替换内容；框架在回调返回后检查当前 owner 和请求阶段，不能继续操作已退休实例。

### 5.2 Presence 与输入资格

公开有限的 Presence 组合，复用 Surface Animatable，供 portal 和以后 Collapsible／Accordion 使用。配置进入／退出动画，向 scene 提供阶段和连续进度；不建立通用动画编排图。

| 阶段 | 子树与视觉 | 输入与焦点 |
| --- | --- | --- |
| 未呈现 | 不保留应卸载内容；按实际需要保留外层控制模型 | 无内容输入 |
| 进入 | 挂载一次，按当前进度绘制 | 新目标等首个 Accepted 场景；模态可先阻止背景 |
| 已呈现 | 普通更新、布局与绘制 | 使用已提交场景 |
| 退出 | 保留同一子树及必要资源，继续采样退出进度 | 立即取消内部按下序列、编辑／焦点资格和新动作；不保留一份透明的可交互副本 |
| 退出完成 | 最终场景移除内容，按现有生命周期释放资源 | 模态屏障和焦点恢复等待确认移除的 Accepted 帧 |

初始 open=false 不挂载内容，也不建立模态屏障。退出不调用整棵节点的 deactivate 来提前破坏绘制资源，而是先停用输入资格；动画完成后再执行卸载。普通无动画关闭直接进入最终移除。`onClosed(reason)` 在最终移除被确认后至多通知一次。

owner 在窗口仍存活时卸载，会立即取消退出动画并收束内容资源；模态屏障由根保留到 Accepted 移除，完成通知同样等待这一边界。窗口／根本身真正关闭时才不等待未来帧，直接释放全部资源并以关闭原因通知。这样强制卸载也不会让仍显示着旧模态画面的窗口接收背景输入。

退出中 open 再变为 true 时，从当前动画值转向进入，复用 key、编辑模型和子树；恢复输入仍等待新的 Accepted 场景，不重放旧初始焦点请求。新的显式焦点请求优先于旧的关闭恢复。

### 5.3 模态和非模态的区别

模态退出期间继续保留 InputBoundary，背景不能抢焦点、收到键盘或被点透。屏障只在确认该层移除的 Accepted 帧后释放；Deferred／失败帧不能提前恢复背景。多层关闭沿现有栈和 focus revision 规则恢复，不覆盖用户已经发出的新焦点请求。

非模态内容退出后停止命中，不自动建立全窗口模态屏障。引起关闭的 outside press 是否继续，仍由 CloseAndConsume／CloseAndContinue 决定；Continue 使用预处理时冻结的目标，不在关闭后重新命中并触发另一目标。模态仍禁止 Continue。

窗口隐藏或暂时没有 drawable 时，动画时间继续推进，完成可由一次性 deadline 收束；屏障等实际画面移除再释放。窗口真正关闭则直接清理，不能等待永远不会到来的帧。

## 6. 滚动意图和公开阅读锚点

沿用 ScrollState、ScrollViewport、固定／可变 extent、reading anchor 和 FollowEnd。补的是公共观察与控制，不重写虚拟列表，也不把聊天规则放进 ScrollArea。

把主动滚动请求与布局修正区分：

- 用户的 wheel／拖动／键盘滚动标记为 User 意图；程序定位标记为 Programmatic。
- 数据 prepend、重新测量和保持阅读锚点产生的位移是 LayoutCorrection，不伪装成新的用户请求，也不使旧修正覆盖更晚的请求 revision。
- 意图在实际输入／定位动作时交付，画面位置仍以 Accepted 快照为准。即使本次方向已经到达边界，也能观察到用户确实发起了滚动操作。
- 观察通过现有 scope／状态订阅模式管理，不建立全局事件总线。普通 ScrollState 方法保留明确的调用含义，内部修正不借用公开滚动操作制造虚假意图。

VirtualListHandle 提供最后已提交的阅读快照，包括可见范围、首个实际阅读项的稳定 key、项内逻辑偏移和 atEnd。首帧之前没有快照；快照是值，不暴露内部 MountedNode、Canvas 或借用排版。

ScrollState 的请求与画面快照分开：scrollBy 返回按最近范围收敛的请求位移，嵌套 wheel 用它计算残量；offset 只随 Accepted 更新。水平虚拟列表按 LayoutDirection 排列，ScrollState 保留物理偏移，阅读锚点从逻辑起点计量；切换方向保留阅读 key，默认首屏位于逻辑起点。

补充按阅读锚点恢复的请求：key 仍存在时，先按现有 extent 估计定位，再按真实测量修正项内偏移；key 不存在时返回明确未找到，交由控件决定回退，不能悄悄用旧 index 指向另一条数据。用户新请求、关闭或数据源替换取消旧恢复。只修正必要项，不为了恢复位置预先测量整张列表。

FollowEnd 仍只在原来处于末端时跟随内容增长。Message Scroller 以后可结合滚动意图、文本选择和活动项状态决定暂停／恢复、定位新轮次和保存会话位置；“新轮次置顶”“底部留白”“未读提示”等策略不进入本轮基础实现。

## 7. 验收重点

- 按下后移出／移回、禁用、失焦、卸载、消费 release、synthetic 与 repeat；动作重入移除自身时没有残留 pressed 或重复激活。
- 子项按钮位于可拖动父项内，超过阈值接管后只拖动、不点击；反方向和小幅移动仍保留正确的原行为。测试原生取消、窗口外释放、变换及边缘滚动停止。
- 活动项导航保持编辑器焦点与 IME；过滤／删除／虚拟化时跟随 key；typeahead 只处理已允许的输入并在超时停止计时。
- 触发区与弹层之间通过间隙、嵌套菜单、延迟关闭时重入；卸载后没有迟到回调或常驻监听。
- Escape／outside press 的 veto、Binding 未采纳关闭、退出中重开、强制关闭、嵌套模态和新焦点覆盖；特别验证 Deferred 帧不释放屏障。
- 连续／离散范围、相邻尺寸约束、快速释放、重复时间戳、吸附目标改变；所有结果有限且方向一致。
- variable list prepend／删除、图像加载增高、手动滚动打断 FollowEnd、按 key 恢复、恢复中又滚动及失败帧；空闲后不再推进布局或动画。
