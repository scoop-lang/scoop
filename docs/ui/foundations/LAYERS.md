# 绝对定位、弹层与模态

状态及共用协议见 [基础交互能力设计](DESIGN.md)。本文件区分容器内的位置、同窗口显示层、独立原生窗口三个层次。

## 1. Absolute：容器内定位

`absolute` 是 `scoop.ui.component` 中的容器，使用公开测量／场景协议实现。每个子项的横、纵轴分别选择一种约束：

| 轴定位 | 测量与位置 |
| --- | --- |
| Start(offset, size?) | 从起边定位；没有 size 时在剩余空间内按内容测量 |
| End(offset, size?) | 从终边定位；测量后由容器尺寸反算位置 |
| Center(offset, size?) | 测量后相对中心定位 |
| Stretch(start, end) | 用两端确定 tight 尺寸，不再同时接收独立 size |

容器尺寸由父约束和自身明确尺寸确定，不由绝对子项反向撑开。有明确尺寸的轴先按约束收敛，否则填满该轴的有限最大值；无界父轴需要调用方指定尺寸，保证定位前已有有限 extent。相互矛盾或负的拉伸尺寸按普通参数错误处理。允许负偏移，默认不增加 clip。

每个普通直接子项仍只测量一次。padding 定义内容坐标原点，`zIndex` 只排序同层子项，相同值按逻辑顺序。Absolute 不突破祖先裁剪、不改变 Tab 顺序、不自动阻止重叠区域的输入。

因此 Badge、角标、局部遮罩和可重叠装饰可以只用 Absolute；下拉菜单需要下面的 portal。

## 2. Portal 与 LayoutAnchor

UiRoot 持有同窗口层出口。`portal` 是框架公开的描述形状：子项仍是原逻辑父节点的子项，实际场景移到该窗口的层出口。Portal 只能引用同根出口；跨窗口必须重新挂载为另一个根，不能保持同一 Component／InputArea 的双重归属。

Portal 的逻辑位置按普通 empty 的约束规则返回占位 metrics，不用弹层尺寸撑开原容器。其内容在逻辑更新时正常挂载，测量延迟到层布局阶段；该内容根在本帧只接受一次层约束，不能先用原父约束测量一遍再由出口重复测量。普通父容器看到的是已测量的占位，不需要了解层内布局。

`LayoutAnchor` 是可由元素场景绑定局部 Rect 的稳定引用，不要求锚点一定可点击。框架沿本帧场景变换得到窗口坐标；旋转锚点首批使用变换后包围矩形。锚点继承普通内容的可见性和 clip；portal 内容从出口的坐标与 clip 开始，但保留原逻辑事件／焦点作用域。

普通内容布局并登记 anchor 后，框架布局该 anchor 的弹层；嵌套弹层可以在父层完成后继续定位。只允许引用已确定位置的祖先／先行内容，不允许两个层互相依赖。自引用或实际依赖环在建立该关系时报告局部状态错误，不做每帧全树拓扑审计。

`OverlayPlacement` 的公开输入包括 anchor、preferredSide、alignment、gap、viewportPadding、是否匹配锚点宽度；输出包含实际 side、origin 和可用范围。定位过程为：

1. 用 viewport 扣除 padding，求主方向和反方向可用范围。
2. 给内容一次有界测量机会；最大主轴范围选两侧较大者，宽度匹配受 viewport 上限约束。
3. 内容放得下优先侧则使用优先侧，否则尝试反侧；两侧都不足时使用较大侧及已约束的滚动内容。
4. 在交叉轴 shift 到 viewport 内；箭头绘制使用最终位置，不再触发反复测量。

数据变化、anchor 滚动／布局／变换、窗口 resize／DPI、弹层自身尺寸改变，都在下一实际帧重新定位。绘制缓存命中不省略这个步骤；静止时不轮询 anchor。初次打开的层在得到有效 anchor 和成功帧前不参与命中。

锚点卸载时关闭其从属弹层。锚点完全移出有效滚动视口时，普通 Popover／Menu 默认关闭；Tooltip 隐藏。确需保持的应用显式选择 `KeepVisible`，仍使用有效 anchor 坐标并限于窗口 viewport，不沿用已经解绑的引用。关闭通知在帧／布局回调退出后发送。

## 3. 层顺序与关闭规则

层按所属模态域、逻辑嵌套关系和打开顺序排序；子弹层在其父层之上。普通局部 `zIndex` 不能把背景内容移到模态域之上。Tooltip 也属于其触发者所在域，背景 Tooltip 不突破新打开的 Dialog。

`OverlayHandle` 负责生命周期和关闭原因，位置／外观由控件决定。关闭原因至少区分 Escape、OutsidePress、AnchorUnavailable、Explicit、OwnerClosed。框架不在关闭时反射调用业务对象，控件通过当前回调更新其 opened 状态。

外部点击按层的逻辑成员判断；portal 子菜单属于父菜单内部，不因为矩形在父菜单外就误关父菜单。一串事件只由最上层适用的 dismiss 策略处理，不能用一次 Escape 连续关掉全部祖先。

模态外部点击总是被挡住，是否同时关闭由控件策略决定。PortalOptions.dismiss 使用 OverlayDismiss(escape=false, outsidePress=Ignore) 显式配置；OutsidePress 另有 CloseAndConsume 和 CloseAndContinue，模态不能配置 CloseAndContinue。非模态 Popover 可选择关闭并消费，或关闭并让当前原始事件继续到原目标；后者在第一次派发前决定，不重新合成／重投一次 pointer down。AlertDialog 默认不因外部点击关闭，显式操作按钮仍由组件组合。Escape 只处理真实、无修饰的非重复按下，避免长按关闭多层。

## 4. 同窗口模态

模态是 `InputBoundary + FocusScope + Layer` 的组合，遮罩只是外观。公开入口为 PortalOptions.mode = OverlayMode.Modal，focus = Some(OverlayFocusOptions(initialFocus=None, restoreFocus=true))，一次 Portal 挂载拥有内部模态会话。循环焦点组包含一个不可命中、tabStop=false 的窗口视口区域，供无子目标时程序化聚焦；不用另一套公开 Layer 或 ModalScope 管理器。模态必须同时做到：

- 指针不能命中背景，包括背景已经捕获的按下序列；激活屏障时向旧路径发送 Cancelled，不补发 click。
- Tab／Shift+Tab 和组导航受当前域限制；背景的程序化 focus 请求也不能越过域。
- 普通键盘默认动作和应用 ShortcutScope 在模态边界停止；模态允许的动作在域内显式登记。
- 同域中的子 Popover／Menu 可接受输入和焦点；打开嵌套模态后只允许最内层域活动。

屏障也截断域外的用户输入 capture／bubble，不仅过滤命中目标和默认动作；逻辑上拥有 Dialog 的背景节点不能借祖先处理器继续响应输入。资源归属、Props 和普通关闭／焦点状态通知仍按原来的逻辑树工作。

打开与关闭采用以下边界：

| 时刻 | 行为 |
| --- | --- |
| 接受打开请求 | 保存当前有效 FocusHandle，安装待显示输入屏障，取消背景按下序列；准备子树 |
| 首次成功提交 | 域内区域成为可用目标；按显式 initialFocus、首个 Tab stop、模态根程序焦点的顺序选择；不抢未激活 OS 窗口的焦点 |
| 关闭请求 | 立即禁止已退休层继续交互；在新帧移除层之前保持必要背景屏障，避免对仍可见模态穿透点击 |
| 移除层成功提交 | 恢复上一域；若原 handle 仍存在、启用且属于允许域，先滚动显露再恢复，否则选择该域的回退目标 |
| 首帧失败／无 drawable | 不交付新焦点，保留待处理请求；调用方可取消打开。窗口关闭直接清理全部域 |

InputArea 的即时关闭规则和最近成功帧几何继续有效；这里没有业务 State 回滚。焦点恢复只是请求，受用户期间主动选择和 OS 激活状态约束；新的显式焦点意图取代旧的恢复请求。UI 的 pending／已知窗口内被拒绝的请求通过 SurfaceWindow.reserveFocusRequest 推进同一个序号，避免只比较已成功聚焦的区域。非当前模态先关闭不移动焦点，后来关闭的当前模态沿保存的前驱关系越过已退休域。普通／模态是不同挂载形状，切换 modal 的有无直接退休旧会话；同一形状内更新策略保留会话。

Dialog／AlertDialog／Sheet 共享上述机制。居中、侧边进出和遮罩样式不需要三套模态管理器。打开 Dialog 后的锁滚动属于背景域被禁用的结果，不去改写背景 ScrollState。

## 5. 独立原生窗口模态

当前 Window 只能创建独立窗口，没有公开 owner／modality；同窗口 ModalScope 无法隔离应用另一个原生窗口，也无法突破系统窗口边界。

如果需要系统 sheet 或独立模态窗口，在 `scoop.window` 增加非阻塞的 `presentModal(owner)` 会话：child 必须是同应用活动窗口，owner 关系不得成环；会话从建立到结束阻止 owner 接受用户输入，owner 关闭时结束 child，child 结束后由原生窗口系统决定恢复激活。UI 根各自拥有实例和坐标。

macOS 使用原生 window-modal sheet／对应 owner 机制；其他平台使用实际可用的拥有窗口及禁用／恢复机制，再补平台验证。会话完成通过既有主循环事件／回调交付，不要求调用方进入阻塞 `runModal` 或再开 UI 循环。不把 topmost 或透明遮罩冒充原生 modality。

这项能力单独实施；复刻 shadcn 的 Dialog、Sheet、Popover 可以先完整落在同窗口方案上。原生窗口外扩的 popup、平台窗口装饰和多屏定位不混入 Absolute 的属性。

## 6. 定向验收

覆盖无界轴错误、负偏移、相同 zIndex 稳定顺序；滚动／裁剪容器内打开 portal，resize 后 flip／shift；嵌套子菜单点击归属；Esc 与外部点击只关闭适用层；模态打开时取消旧 capture；背景快捷键和程序化焦点被隔离；焦点原目标卸载、首帧失败、窗口失焦及 owner 关闭。原生模态另外验证真实系统 owner 关系，不能用普通界面截图替代。
