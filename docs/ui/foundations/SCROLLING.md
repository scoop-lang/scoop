# 滚动、绘制缓存与实例缓存

状态及场景协议见 [基础交互能力设计](DESIGN.md)。用户要求的“有缓存／无缓存”同时覆盖绘制缓存和子项实例缓存，二者与 virtual scroll 独立。

## 1. 三个独立维度

| 维度 | 无／关闭 | 有／开启 | 改变什么 |
| --- | --- | --- | --- |
| 绘制缓存 | 直接绘制本帧可见内容 | 复用未失效的绘制记录／像素块 | 绘制与光栅工作；不决定 State 或挂载生命周期 |
| 虚拟项实例缓存 | 离开实现范围且无活动交互后正常卸载 | 保留有限个离屏 key 的原实例与资源 | 再次进入时是否复用局部 State／挂载资源 |
| Virtual scroll | 普通 ScrollArea 先挂载所有显式 children | 只按视口、overscan 和活动交互创建所需子项 | build／mount／measure 的数量 |

普通 ScrollArea 本来就保留整个内容树；关闭绘制缓存不会卸载离屏内容。虚拟列表关闭实例缓存也仍保留可见项和 overscan 项；这不是每个滚动事件都销毁重建一遍。

拟议容器接口由 `ScrollState`、一个或两个滚动轴、内容和绘制策略组成。`VirtualList` 另接 `ItemSource`、extent 策略、overscan 与实例保留策略。首批虚拟化支持单主轴列表，普通 ScrollArea 可双轴滚动；二维虚拟网格另按真实 Data Table 需求扩展。

缺省先选直接绘制、离屏不保留；调用方按内容成本显式开启缓存。实例上限是实际容器拥有资源的局部容量，不引入跨 UI 的预算或计费系统。

## 2. ScrollState 与嵌套滚动

`ScrollState` 保存逻辑单位的 offset、viewportExtent、contentExtent 和范围，提供 `scrollBy`、`scrollTo`、`reveal(rect, alignment)`。范围改变时收敛 offset；没有可滚动范围时 offset 为零。内容位置为布局原点减 offset，viewport 对像素和命中使用同一个 clip。

直接拖动和 wheel 更新 offset，使用场景位置失效，不仅因偏移变化就重建子项 Props 或作废静态内容图像。依赖滚动位置的应用仍可显式观察合并后的状态通知。测量时得到的 extent／收敛结果先作为布局内部结果，退出布局后通知观察者，不在 measure 中写 State 或重入布局。

滚动条复用已有主题绘图，但新增实际行为：thumb 长度／位置由范围计算，拖 thumb 映射到 offset，点击轨道按页移动，范围为零时隐藏或禁用。minimumThumbSize 不改变逻辑滚动范围。键盘 PageUp／PageDown／Home／End 只在所属滚动区域或其导航代理成为默认动作接收者时生效，不能抢走文本编辑器的行／文档导航。

Wheel 的默认动作从目标向祖先逐个消费：子项在一个轴到边界时仅交接该轴残量；可配置 `Contain` 在该区域停止链式滚动。`preventDefault` 取消整次默认滚动，普通 `stopPropagation` 不隐式消费位移。残量始终保留 Lines／Pixels 的原始单位及方向，每个容器按自身变换与 lineStep 换算消费比例，不能把子容器的像素结果直接当成父容器的行数。

Surface 接口为 `RoutedEventContext.scrollRemaining` 与默认阶段的 `consumeScroll(lineStep, apply)`。后者使用当前接收区域在本次事件开始时固定的几何，把残量交给 apply；容器返回实际使用的局部位移，Surface 换回原单位再扣除。每层分别提供 lineStep；ScrollArea 将滚轮的内容位移取反后更新 offset，并返回实际移动量。Contain 在本层后调用 preventDefault。ScrollCancelled 与按钮 Cancelled 分开，结束滚动不会误终结仍在进行的 thumb／其他鼠标拖动。

Pixel delta 复用 Surface 已完成的一次 DPI 归一，不再乘除 backing scale。保留 phase 和 momentum；带 phase 的序列固定起始滚动路由直至正常结束／取消，避免内容移动后误切到另一个兄弟滚动区；无 phase 的离散 wheel 按当次命中。源路径退休时停止向其已关闭节点交付，并完成取消，不重新寻找背后的兄弟目标。

首批用现有原生 wheel／触控板动量，程序平滑滚动复用 Animatable；不同时实现一套自制触摸惯性与手势竞争。显式平滑滚动在新用户滚动开始时取消，动画终点按最新范围收敛，停止后撤销帧需求。

### 2.1 普通滚动容器的公开合同

`scoop.ui` 提供 `ScrollState(initialOffset)`、`ScrollSnapshot(offset, viewport, content)`、`ScrollAlignment` 和随 ComponentScope 管理的 `ScrollViewport`。State 的 snapshot／offset／viewport／content／maximum／isAttached／isAnimating 可在 render 中观察；只在相应值改变时通知。普通滚动不要求内容 render 读取 State，因此不会仅因 offset 改变重建子树。State 使用创建线程，所有公开写操作与 State 一样禁止在 render／measure／scene／draw 中调用。

初次布局前范围和可见 offset 为零，构造位置、scrollTo 和 reveal 保留意图，在首个成功布局后收敛；尚未确定范围时 scrollBy 在已有的位置意图上累加。reveal 使用内容局部坐标，支持每轴 Nearest／Start／Center／End；已经包含视口的超大目标在 Nearest 下不跳动。脱离视口后 State 保留最后结果，可供重新挂载恢复位置。

`scope.scrollViewport(state)` 绑定一个实际视口，同一 State 不能同时绑定两个视口。登记的 state 可以整体替换，先检查新绑定是否合法，失败保留旧绑定；关闭立即解绑并停止动画。自定义容器在 measure 调用 `layout(viewport, content)` 取得本帧快照，在自己的 scene 调用 `present(context)`。layout 不发布可观察状态；只有 Accepted 提交范围与位置，Discarded 不更新。迟到的旧绑定回调无效，布局之后的新滚动请求优先于旧帧位置，并按新范围请求必要的下一帧。

`animateTo(offset, spec)` 要求已绑定视口，允许在其首次布局前排队；未绑定时报错。它复用该 Surface 的 Animatable，不另建时钟。新 wheel（包括零位移 MayBegin）、滚动条操作、scrollTo／scrollBy／reveal 或 cancelAnimation 会在最后发布的 offset 停止旧动画。动画采样只在 Accepted 后发布；范围缩小时目标重新收敛，没有剩余移动时结束。isAnimating 包括等待首个布局的显式动画。

`scoop.ui.component.ScrollArea` 接受一个显式 content View、可选外部 State、Vertical／Horizontal／Both、width／height、lineStep、Chain／Contain、Auto／Always／Hidden 滚动条、ScrollbarOptions、FocusOptions 和主题。滚动轴需要有界父约束或明确尺寸；非滚动轴有界时填满，无界时采用一次子测量结果。内容每帧只测量一次，在滚动轴使用无界约束；同一视口 clip 约束像素和命中，内容以负 offset 平移。

首批滚动条覆盖在视口边缘，不改变内容约束；双轴条在角落互相让出厚度。拖动按主题给出的实际 thumb travel 映射范围，轨道按一页、可选箭头按 lineStep 移动。输入使用最后 Accepted 的几何，禁止用被丢弃帧的条位置处理拖动；释放被 preventDefault 时仍清除拖动状态。点击滚动条消费默认聚焦，保留子编辑器的焦点。范围为零时 Auto 隐藏，Always 显示禁用状态。键盘 PageUp／PageDown／Home／End 只在事件 target 就是滚动视口且无修饰键时执行，子项导航由子项处理。

## 3. 真正的按需子项协议

现有 `element(children: Array<View>)` 保持 eager 语义。不能先创建全部 View／Component，再通过 clip 宣称 virtual。框架增加与普通元素并列的 `lazyElement` 描述和受控的 `LazyMeasureContext`，供独立库编写列表、树列表等实际容器。

`ItemSource` 提供当前数据快照的 count、`keyAt(index)`、`indexOfKey(key)` 和 `build(index)`；索引使用 Long。稳定 key 在这个 lazy parent 内唯一，`Position` 不用于可插入／重排的数据。数据源改变通过普通 Props／State 更新，不用内容哈希审核；index/key 一致性在实际访问边界检查，不为首屏挂载先构建全列表。

协议核心是 `measureItem(index, constraints)` 与 `placeItem(index, origin)`：

1. 容器根据 viewport、offset、已知 extent 选择下一项。
2. 框架在受控的子项准备边界匹配 key／shape，按需要 build／render，完成 onMount 和 update，再测量该项。
3. 普通 measure 函数仍不能任意挂载子树；只有这个入口能请求框架准备当前 source 中的子项。build／render 的 State 依赖归该项的逻辑节点。
4. 每个本轮选中的项在本轮约束下只测量一次；容器可以根据结果继续请求后续项。保留但未选中的缓存项不受普通直接子数组“必须测量全部”的规则约束。
5. 本轮结束后发布放置计划，处理离开范围的保留／淘汰。失败按既有树关闭／资源清理规则收尾，不留下半挂载的资源。

子项准备与测量是两个明确的执行阶段，即使后者因 `measureItem` 暂停在调用栈上。生命周期回调按原规则可以取得资源或写 State；其导致的其他节点失效留到下一逻辑更新轮，不递归 flush 当前布局。父 measure 和子 render 的纯度规则没有因此放开，也不承诺全树不可变状态快照。

数据删除或 key／shape 改变时，即使没有 drawable，也在正常逻辑更新中退休已失效的缓存项。未曾请求过的项没有 Component、InputArea 或挂载资源；数据模型本身可由应用长期持有。

## 4. 固定与可变 extent

固定 extent 直接通过 offset 和 itemExtent 计算范围；extent 必须为有限正数。可变 extent 使用有限正的估计值，加上已测量尺寸索引，通过前缀和定位起始项；优先复用普通数组／树结构实现该局部算法，不建立通用布局查询框架。

一次布局从估算位置开始，逐项测量到 viewport 与 overscan 被覆盖，或数据结束。实际零高度项合法，每步推进索引，不能停在同一项循环。约束／字体／内容改变会使相关测量失效；旧位置估计可用于定位，不伪装成精确尺寸。首批不提供任意 intrinsic 多轮协商。

稳定阅读位置用“首个可见 key ＋ 该项内偏移”表达。前方插入／删除、已测量项高度改变时，优先保持这个锚点；锚点被删除时选原位置附近的存活项。宽度或字体变化后重新测量，偏移按新项尺寸收敛。需要聊天底部跟随时显式选择 `FollowEnd`：只有原先已在末端时跟随新增内容，用户向上阅读时不拉回。

`scrollToItem(key, alignment)` 先用索引估计定位，再依据实际尺寸修正；在一次实际帧中的项测量不反复重入。尚需修正时按需继续下一轮，并在真正显露后完成焦点请求；不承诺未知高度列表第一次估计就像素精确。

具体公开入口采用 ItemExtent.Fixed／Variable、VirtualListAnchor.KeepVisible／FollowEnd 和可选 VirtualListHandle。ScrollState 仍是唯一位置状态，handle 只保存一次按 key 的定位意图。尺寸前缀使用已访问项的稀疏索引；布局通过 ScrollAdjustment 携带所依据的 requestRevision，确保较新的滚动请求优先，成功提交才发布修正和阅读锚点。

超长内容使用 ScrollViewport.present 的 contentOrigin 表达附近原点。VirtualList 先在 Double 中计算条目前缀与本帧 offset 的差，再呈现局部位置；前缀索引、ScrollState 与阅读锚点仍保留完整逻辑坐标。视口边界在内容平移之前采集，焦点显露继续把窗口位置映回完整内容坐标，不要求图形后端相消两个巨大的平移。

## 5. 实例保留策略

公开接口为 `ItemRetention.Dispose` 与 `KeepAlive(maxOffscreenItems)`，由 lazyElement 和 VirtualList 的 retention 参数接收。后者保留同 key、同 shape 的实例，按最近一次测量的时间优先淘汰最久未使用的非活动离屏项；同帧使用次序不承诺。容量须非负，零与 Dispose 相同；上限不包含本次必须实现的 viewport／overscan。不能把 key A 的 Component 改装成 key B 来节约构造，不依赖控件自己清空旧状态。

保留项继续 mounted：Props／State／订阅和已登记资源存在，正常逻辑更新继续执行；本帧不测量／装配／绘制，不保留屏幕命中区域。这里沿用既有未放置子树语义，不新增“暂停订阅”的隐藏生命周期。资源较重或持续接收数据的项可以选择 Dispose，将需持久保存的业务状态放入列表外模型。

未选中的缓存项显式关闭子树帧资格，内层缓存项还保留自己的资格状态；仅测量而未放置的项也不会短暂恢复帧需求。ComponentScope 的动画和 ScrollViewport 复用 Surface AnimationGroup，把子树资格与每个动画自己的 isFrameEnabled 合并，避免恢复时覆盖本地选择；有限动画使用窗口一次性期限完成，无限重复不安排离屏周期唤醒。缓存中的 State 更新和 requestFrame 不自行请求绘制，新输入边界等改变窗口交互的操作仍要求必要帧。

当前键盘焦点、待聚焦请求、尚未结束的指针序列、IME 会话或仍登记的模态输入边界所依附的项临时固定在实现集合中，避免在交互中途卸载。输入边界从登记起就固定所属项，不等待首次自动聚焦；非当前边界和等待移除帧的屏障也计入。普通聚焦项可以被 viewport 裁掉像素，但继续有有效路由／焦点会话；焦点保持资格与新 Tab 目标资格分开。键盘导航／编辑会请求 reveal。

活动 composition 额外保护 caret 可见性：相关滚动容器将该 caret 纳入 reveal 约束，不接受会将它完全滚出视口的偏移，直到组合结束。这避免依赖平台未提供的“保持组合但强制隐藏候选窗”能力。应用显式隐藏编辑器、删除数据或关闭窗口时，仍按正常 IME 取消流程结束组合，然后解除保护；不为保留一行而阻止真实删除。

活动交互结束后立即恢复普通淘汰资格。触控板接触 Ended 后可能再收到 momentum，故尚保留的滚动路由也固定其起点条目；每窗口至多一个候选，惯性结束／取消、下一段滚动或路径失效后解除，不为等待候选请求连续帧。数据真的删除该 key 或 owner 关闭时优先真实退休，按 Surface 规则取消 pointer／组合并安排焦点回退；固定不阻止删除。实例保留不自动保留其图像块，两种缓存各自淘汰。

## 6. 绘制缓存

### 6.1 记录与像素分开

绘制记录保存不可变绘图命令，避免再次调用昂贵的用户绘图逻辑，但每次重放仍可能重新光栅化。像素缓存才直接复用已绘制区域。对外的首批滚动策略为 `None` 和 `Tiles`；Tiles 可在内部复用记录，但不能只实现 SkPicture 重放就宣称已交付像素缓存。

不为整个超长内容分配一张 bitmap。Tiles 只保留 viewport 附近的有限内容坐标块，大小和 overscan 是本容器的参数；不在每次滚动时栅格化完整文档。离开缓存范围的图像正常释放。对于简单列表，None 仍是合理选择。

graphics 需要补充绘制记录器和与当前 render target 兼容的离屏目标／只读合成快照。GPU 缓存保持在兼容 device／context 内，提交和采样顺序由现有后端协调；禁止把当前 `RenderTarget.snapshot()` 的隐式 CPU 读回放入滚动热路径。CPU Raster 路径用同一可观察行为验证。关闭／目标重建／GPU 尚在使用时，资源沿 graphics 引用与完成规则释放。

### 6.2 有效性与几何

块以内容局部坐标定位，滚动只改变合成 transform。内容和布局、绘制资源、有效 scale、颜色空间或影响该块的效果改变时失效；resize 若改变子内容布局也必须失效。仅 viewport 平移或最终 clip 改变，不无条件重绘已有内容块。

场景装配照常登记 InputArea、anchor、子项顺序和 caret，命中缓存只跳过绘图块／光栅步骤。公开 `draw` 块只有在提供可靠的绘制范围并满足缓存合成条件时才进入缓存；无界自绘或直接依赖目标背景的混合可以继续直绘，不能错误缓存后改变视觉。

布局 bounds 不是绘制 bounds。字形 overhang、阴影、模糊和光晕可能越界：缓存范围必须包含实际绘制及滤镜采样扩展，相邻块用扩展范围绘制后只合成核心区，避免边缘接缝。无法确定有限范围时回退该内容的直接绘制；不以 viewport 小矩形裁断本来可见的效果。原本的 Style／Canvas clip 与滤镜顺序保持不变。

一个控件 hover 或 caret 闪烁只使包含它的相关内容缓存失效；纯滚动不使静态正文图像失效。缓存边界宜把频繁变化的选择／caret 与稳定正文分开，具体拆分由编辑适配完成，不让通用缓存识别“编辑器”类型。

## 7. 四种组合与验收

| 绘制策略 | 实例策略 | 离开并返回 viewport |
| --- | --- | --- |
| None | Dispose | 正常卸载／重建，进入后直绘 |
| Tiles | Dispose | 实例正常重建；只有所属内容仍有效的块可复用，重新挂载改变内容时失效 |
| None | KeepAlive | 复用原实例和局部状态，进入后直绘 |
| Tiles | KeepAlive | 实例与有效图像分别复用；任一被淘汰不要求另一种同步淘汰 |

图像不保有输入处理器或 Component，从而不会反向延长已卸载组件生命周期。控件自身布局／绘制缓存随其实例正常销毁；由滚动容器拥有的块可以独立保留，但重建子项必须通过普通失效规则更新它，不能仅按相同 key 假定像素相同。

覆盖固定／可变高度、零高度、插入／删除／重排、未知尺寸修正、key 对应实例和 onUnmount 计数；验证多层滚动残量、反向／双轴、DPI、phase、缓存图像的真实复用；覆盖活动编辑项暂留后淘汰、离屏动画停帧、内容带阴影／圆角／滤镜的边界及失败帧。计数只用于验证真实算法复杂度，不变成运行时的通用执行预算。
