# 事件路由、焦点导航与快捷键

状态及场景协议见 [基础交互能力设计](DESIGN.md)。这里扩展现有 Surface 输入模型，不另建一套只给控件库使用的事件系统。

## 1. 路由与命中分开

InputArea 继续表示可命中的目标，Surface 增加可以没有命中形状的 `InputScope`。窗口根、模态域、ShortcutScope 和组合控件都可使用 scope。UI 根据逻辑挂载祖先连接 scope／area，Surface 的直接调用方显式传入所属父引用。

建立父关系时检查同窗口、仍活动和无祖先环，沿用普通对象关系的局部检查；不因输入事件频繁就每次重新校验整棵路由树。关闭父 scope 时退休相应路由资格，UI 挂载生命周期仍负责资源和子组件收尾。

路由父关系与显示场景独立：portal 子菜单可以画在窗口根层，仍向触发控件所在逻辑作用域冒泡。没有焦点时，键盘从活动窗口的根 scope 开始；不丢弃没有区域焦点的窗口级输入。

命中仍以最近成功帧的 transform／clip 和反向绘制顺序决定唯一目标。父路由节点不必先画一个矩形才能接收事件；没有局部几何的 scope 使用窗口位置，有几何的 area 可查询自身局部位置。不得为无几何 scope 编造 localPosition。

一次事件先确定合法域、目标和路由快照。处理器使用最新版本；沿途已关闭／禁用的节点跳过，不能把事件重定向到背后的兄弟。由回调引起的挂载变化按现有更新边界处理，后续事件再使用新路径。

## 2. capture、bubble 和默认动作

可路由输入的顺序为：合法域过滤 → capture（根到目标父）→ target → bubble（父到根）→ 默认动作（目标到根）。事件 capture 阶段与 pointer capture 是两个概念。

`RoutedEventContext` 保存 target、currentTarget、phase、time 和以下操作：

| 操作 | 结果 |
| --- | --- |
| `stopPropagation()` | 不再调用后续传播处理器，不自动取消默认行为 |
| `preventDefault()` | 取消这次事件的默认动作，仍允许观察者收到传播 |
| `consume()` | 同时停止传播和取消默认动作，方便普通按钮／菜单使用 |

处理器统一采用 `(RoutedEventContext, InputEvent) -> Unit`；InputScope 提供 captureHandler、handler 和 defaultAction。InputArea.scope 是带几何的作用域，InputScope.area 可查询关联区域；旧 InputResult 不再作为结果类型。Button／Checkbox／Surface 示例使用同一条派发链。普通默认动作不在中途向父节点重新派发一次相同事件。

默认动作沿原路径由内向外尝试。每个节点先执行该控件的默认行为，再匹配这个节点的快捷键；一个动作明确消费事件后停止。这样文本框可以消费 Ctrl/Cmd+C，未消费的保存快捷键仍可到达外层 scope。Tab 遍历、最上层弹层 Escape、滚动和组内导航都登记在其实际作用域，不用一张跨控件的全局优先级表。

每个活动模态域／窗口根的默认动作还包含该域的弹层关闭代理，先于该处的普通 ShortcutBinding。它在内层输入行为未消费 Escape 时选择最上层适用弹层，即使焦点仍在触发器而非 portal 内也能关闭；一次只执行一个关闭动作。模态截断 capture／bubble／default 的域外路径，不改变挂载归属。

Surface 的 InputBoundary 先提供上述输入域：最后登记的未关闭边界生效，scope 退休后保持背景屏障，直到上层在适当帧完成边界显式 close。InputScope.isInputAllowed 为尚未放置的目标提供同一准入判断。当前域根的 beforePointerPress 在首次按钮按下前作出消费／继续决策，不更换已固定的目标，也不在同一按钮序列里反复执行；完整弹层策略由 UI 层会话接入。

不同事件的可取消性明确区分：

| 事件 | 派发合同 |
| --- | --- |
| Pointer down／up／move、Wheel、Keyboard | 可路由，可取消默认行为；保留来源、重复、合成及单位信息 |
| Entered／Left、FocusChanged、ModifiersChanged | 状态通知，不因某个处理器取消而破坏其他节点的状态收尾 |
| Pointer Cancelled | 向旧路径必要节点完整交付，不被中途的消费截断，也不产生 click |
| TextInputBatch、IME Enabled／Disabled | 交付对应输入会话；不冒泡给祖先任意再次修改文档 |

InputScope.onEventFinished 在一次可路由输入结束后完成不可拦截的清理；被阻止传播或取消默认行为的释放仍须清除旧按下状态，不能重新触发激活。

默认焦点变化是 pointer down 的默认行为，可被 preventDefault 取消；这使点击菜单项等行为不必先错误聚焦背景再补救。具体按钮的按压／释放／点击状态仍由控件定义。

## 3. 按下序列与滚动消费

保留现有“第一次按下固定路径直到所有按钮释放”的合同。移动、释放和取消交付给该路径，即使 pointer 已移出目标；hover 可以按当前位置独立更新。模态启用、目标退休和原生取消会结束该序列。

本批不增加通用手势竞争，也不让普通 stopPropagation 把 pointer capture 转给别的目标。滚动条和 Slider 可以靠现有按下路径完成拖动；未来真正需要拖动交接时再扩展显式 capture 转移，而非先建仲裁框架。

Wheel 在默认阶段携带尚未消费的两个轴。ScrollArea 回报实际消费量，父只处理剩余部分；用完后停止。`Contain` 可以明确阻止剩余量继续上交。phase／momentum 结束仍送到参加过该次序列的有效接收者完成收尾。具体单位和范围规则见 [SCROLLING](SCROLLING.md)。

## 4. FocusHandle 与 FocusScope

Surface 继续持有每窗口唯一逻辑焦点和 OS 活动状态。UI 的 `FocusHandle` 是对目标的稳定引用，能表达立即聚焦或等待显露／首次提交的请求；它不复制 Surface 的焦点状态。控件公开一个可选 handle 属性，用户不必获得 Button／Input 的内部 InputArea 才能聚焦。

每个 focus target 登记 `tabStop` 和作用域内的 `order: Int = 0`，启用状态直接取自 InputArea／InputScope 的逻辑祖先。按 order 升序、同值按逻辑挂载顺序遍历，不影响布局和绘制；负 order 只表示排在更前，不暗含退出 Tab 的语义。`tabStop=false` 才退出 Tab 序列，仍允许合法的程序化聚焦；disabled／卸载／模态外目标不可聚焦。

普通隐藏／未实现目标不直接接收焦点。ScrollArea 中尚需显露的已挂载目标和虚拟项，先通过公开 reveal 接口滚动／实现，再等成功帧兑现请求。focus target 的“可保持当前输入会话”和“可作为新的 Tab 目标”分开；当前焦点不会仅因滚动裁剪而丢失。组合输入期间另外保持 caret 可见，规则见 SCROLLING，候选窗位置只能来自有效几何。

`FocusScope` 保存逻辑顺序、默认目标与边界策略：

- 普通 scope 到末尾后继续父 scope，不自动围困 Tab。
- 窗口根默认按本窗口顺序循环，不越过 OS 去切换别的原生窗口。
- 模态 scope 将循环与程序化请求限制在该域；嵌套模态暂停外层域，详见 LAYERS。
- 没有可用子目标时，可聚焦显式的 scope 根代理，不把焦点留在已卸载节点。

`requestFocus` 是请求，不调用原生激活去抢应用焦点。等待显露期间若用户提出更新的焦点意图，旧请求取消。原目标关闭、失效 key 或 owner 关闭都结束待处理请求；不能无限引用旧 View 或过期索引。

## 5. Tab 与组内导航

Tab／Shift+Tab 在内层控件未消费时执行遍历，跳过禁用和不能显露的目标。Input 默认交出 Tab；普通 Textarea 也默认交出 Tab，只有显式配置插入 Tab 字符时才消费，不因为多行就把焦点困住。

`KeyNavigation` 是公共的无样式导航助手，使用控件提供的有序成员／行列位置和 FocusHandle，提供方向、Home／End、可选循环和禁用跳过；焦点策略不等于选择策略。Tabs 可以 focus 后自动激活，也可以 Enter／Space 才激活；RadioGroup、Menu 等由组件 spec 明确选择语义。

组默认使用 roving tab stop：组中只有活动成员加入 Tab，箭头在组内移动；不会让一个菜单的几十项都变成独立 Tab 停靠点。横向导航按 writing direction 映射左右，纵向不因 RTL 反转。列表／网格由逻辑邻接关系决定下一项，不用画面最近距离猜测下一目标。typeahead 是菜单／列表基于文本和一次性超时的导航策略，不是系统快捷键。

已实现的 KeyNavigation 通过 NavigationOptions 整体配置不可变成员数组，Grid 成员使用显式 row／column；同行／同列查找跳过缺失和禁用项，Home／End 使用当前行的逻辑首末。返回组时优先当前有效焦点，再按 Surface 成功聚焦历史选择，未使用过的组选择首项；不覆盖原有输入处理器记录第二份焦点。具体 API 与配置冲突规则见 UI spec §7.3.2。

虚拟列表提供按 key 访问的导航代理。移动到未挂载项时先定位 key、实现并 reveal，再聚焦；列表是否采用一个 roving Tab stop 或顺序访问行内编辑控件由组件明确选择。顺序访问模式在需要越过一行时准备下一项并寻找其首／末个目标，遇到无可用目标的行继续推进到有限数据结束，不预挂载全列表建立 Tab 表。

焦点导致的滚动显露由内向外进行：先让目标进入最近滚动 viewport，再把该 viewport 中的目标区域向外层 reveal。最终以成功帧的位置判断完成，避免父子依据不同旧坐标来回滚动。

## 6. ShortcutScope

快捷键是当前应用已收到的键盘事件的默认动作，不等于系统级全局 hotkey。本批支持单次按键组合；多步 chord、键位配置文件和全局命令注册中心都不是这组基础控件的前置。

`ShortcutBinding` 包含 logical 或 physical key 二选一、modifier 要求、是否接受 repeat、enabled／canExecute 以及 action。`Primary` 在 macOS 映射 Super/Command，在其他桌面平台映射 Control。修饰键按归一后的完整组合匹配，未写的 Shift／Alt 等不能被悄悄忽略；CapsLock 等锁定状态不作为普通修饰条件。

默认使用 logical key 表达随键盘布局变化的功能键位；确实需要硬件位置的场景显式选择 physical。Character 匹配使用 Window 提供的逻辑键，不从输入文本猜按键；普通字母键按不受 locale 影响的大小写归一匹配，Shift 由 modifier 单独区分，避免 CapsLock 改变保存等绑定。dead key 和 IME preedit 不是快捷键文本。

默认只对真实 key-down 执行一次，不因 key-up 或 synthetic release 再执行；重复触发必须明确开启。完全相同的归一化组合在同 scope 同时启用时报配置冲突，不由注册先后随机覆盖。跨 scope 按内到外选择第一个实际执行的绑定，disabled／canExecute=false 继续向外。

编辑目标所在的本地行为先取得编辑按键；裸字符、dead key 和输入布局使用的 AltGr 不能被祖先的普通 shortcut 抢作文本插入替代。编辑焦点下的祖先绑定默认要求明确命令修饰键或非文本 NamedKey；菜单 typeahead 使用自己的目标默认行为。调用方确需覆盖某个本地行为时，通过显式 capture + preventDefault 实现，并承担完整行为，不增加隐式“强制优先”整数。

已识别的编辑命令即使当前不能修改文档也由编辑目标消费，例如空选区 Copy 或只读框 Cut；不能因此意外触发外层同组合的文件操作。ShortcutBinding 自身的 canExecute=false 继续向外，与编辑目标已经认领的默认命令是两种情形。

应用级绑定登记在活动窗口根之外的宿主 fallback，但模态边界先截断。需要在 Dialog 内有效的保存／确认动作在该域登记。原生窗口系统保留的组合只按平台实际交付处理，不声称 Scoop 能接管未收到的按键。

## 7. 与 IME 的顺序

macOS 当前先执行 `interpretKeyEvents`，组合处理／确认已消费的 key 不再交付普通编辑，Command／Control 组合按 Window 规则送 Keyboard。UI capture 和 ShortcutScope 都发生在这条原生边界之后，不能承诺先于输入法截获所有键。

具体结果必须一致：

- 中文组合期间 Enter 只确认组合，不能同一次提交表单；Escape 先由输入法取消，不能顺带关闭 Dialog。
- Ctrl/Cmd+C 在可复制的编辑目标中执行 Copy，不再触发应用同组合的其他动作。
- 未被本地编辑消费的 Ctrl/Cmd+S 可以沿允许域触发保存。
- Tab 若已被 IME 消费，不再移动焦点；普通编辑状态下再按本文件执行遍历。

文本只由 TextInputBatch 进入编辑状态，不把 Keyboard.Character 再插一次。FocusScope 改变编辑目标时继续等待旧 IME Disabled 边界；旧队列不能流入新编辑器。公共编辑助手的具体事务见 [TEXT-EDITING](TEXT-EDITING.md)。

## 8. 定向验收

验证精确 capture／target／bubble／default 顺序及两个取消标记独立；重叠兄弟不接力、portal 路由、不带几何的 root shortcut、handler 中卸载、完整 Cancelled；Tab 稳定顺序、roving、RTL、disabled、虚拟 reveal 和模态焦点恢复；键盘布局、repeat／synthetic、绑定冲突、编辑命令优先及 IME 消费后的单次动作。焦点和 shortcut 测试复用同一派发入口，不能只直接调用控件回调。
