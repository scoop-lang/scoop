# 控件基础补齐实施记录

日期：2026-11-04。状态：P1–S1 及 C1 共十一项已完成 macOS 实施与验收。

实现分支为 `codex/ui-component-foundations`。每个大项完成定向 macOS 验证后提交、推送，再把最新 origin/main 合入实现分支；只对后续实际改动或合并冲突影响的部分复验。临时脚本和原始输出在仓库外使用 `/Users/chenxu/repos/scoop-work/ui-component-foundations`，每项结束清理不再需要的文件，保留后续会复用的构建缓存。

设计见 [DESIGN.md](DESIGN.md)，平台合同、布局、交互和文本的细节分别见 [PLATFORM.md](PLATFORM.md)、[LAYOUT.md](LAYOUT.md)、[INTERACTION.md](INTERACTION.md)、[TEXT.md](TEXT.md)。本记录与已完成的 [A–F 基础交互记录](../foundations/PROGRESS.md) 分开，避免把此前验收当作本轮交付。

| 批次 | 状态 |
| --- | --- |
| P1 平台键盘合同 | 已完成 |
| P2 文本服务与源码边界 | 已完成 |
| L1 布局协议与空间分配 | 已完成 |
| L2 方向、Wrap 和 Grid | 已完成 |
| I1 公共激活与拖动 | 已完成 |
| I2 弹层、候选项和悬浮 | 已完成 |
| T1 编辑事务策略 | 已完成 |
| T2 文本投影与选择 | 已完成 |
| V1 装饰与图像 | 已完成 |
| S1 阅读位置与滚动意图 | 已完成 |
| C1 组合验收 | 已完成 |

本轮已完成 macOS aarch64 分批验证和 x86_64 Rosetta 的生产库及代表组合补验；Rosetta 不等于真实 Intel 硬件验证。Windows／Linux 后端实现及构建／桌面验收、accessibility、原生控件交互和 shadcn 控件库不在本轮范围。

## 设计阶段

- 核对现有 Window／Surface／UI／Component／Style 的规范、实现和 A–F 验收记录，确定现有能力与真正缺口。
- 写明 Primary／Super、编辑命令、剪贴板／IME 和字体 fallback 的分层；约定上层无 OS 分支，Windows 后端按同一公共合同接入。
- 写明重新测量与 lazy 最终收尾、受控 portal 与退出动画、编辑策略与 composition／位置映射的约束，给出逐批迁移和定向测试范围。
- 本阶段只有设计与文档导航改动，没有新增 API、native 实现或控件，也未执行生产库构建和运行测试。

后续每批只记录实际修改、验证范围、结果、剩余失败和必要限制；原始输出存放仓库外，不把规划用例记为已通过。

## P1：平台键盘合同

- window 提供 ShortcutChord、Primary 解析、精确匹配、等价比较和当前键盘布局标签；Surface 的绑定、scope 和 host 派发迁入 shared。Modifiers 保留真实 Super，并增加可选 AltGr 状态；macOS Fn 只表示实际按住的修饰键。
- AppKit 标准编辑键位移入 window，UI 只执行已交付的语义命令，Enter／PrimaryEnter 提交使用同一 chord。已迁移现有绑定、示例与组合测试，不保留旧 Binding 配置或 UI 后备键位表。
- aarch64 macOS release：window／surface／ui 生产库构建成功；Scoop 格式化与三个 Cone 的 tests 类型检查通过，修改的 native 源码通过严格编译警告检查。
- 定向运行 22 项全部通过：window 的 Unicode、两种 Primary 解析、标签所有权、真实 AppKit 编辑与 Fn 共 7 项；Surface 的 DSL／作用域／宿主路由共 13 项；UI 编辑器与模态组合各 1 项。无未闭合失败。Windows 只覆盖共享解析算法，不代表其后端验收；x86_64 留到 C1。
- 已清理这一项不再使用的独立类型检查锁文件；保留后续会复用的生产库、测试构建目录和共享 Cone cache。

## P2：文本服务与源码边界

- Clipboard 及 IME 文档／caret 的声明、owner 和参数检查迁入 window 共享源码；Darwin 文件仅提供原生转换和发布。剪贴板读写归一化 CRLF／CR 为 LF，写前拒绝 NUL，读取到内容 NUL 报 Unsupported。Surface 使用同一个 caret 适配，删除忽略连接与 revision 的旧路径。
- graphics.text 为 sans-serif／serif／monospace 提供真实系统 family，集合创建时解析并保存；显式同名字体优先，关闭 fallback 时不加入系统别名，已有 Paragraph 保留自己的字体快照。三套主题已有的偏好列表直接使用该共同 fallback。
- 移除 ui／component／style／三套主题生产源码的 Darwin 条件；window.asyncio 已有的共享宿主与 platformWatch 分层继续复用。PLATFORM.md 已列出后续 Windows／Linux 分支所需入口，本轮没有补写或验收这些后端。
- aarch64 macOS release：window、graphics.text、window.asyncio、surface、ui、style、三套主题及 component 共 10 个生产库构建成功。修改的 Scoop 通过格式化，window／graphics.text 的 tests 类型检查通过，native 源码通过严格警告编译。
- 5 项定向测试全部通过：剪贴板往返／NUL／原生失败／owner，原生文档修订与范围查询，以及三项字体类别／别名覆盖／禁用 fallback。没有复跑 P1 的快捷键、编辑器或模态测试。清理了本项的独立检查锁文件，保留依赖缓存与下一项草稿。

## L1：布局协议与空间分配

- MeasureContext／LazyMeasureContext 允许放置前重新测量，相邻同约束复用最后成功结果；失败清除本次结果，放置后拒绝再测量。每次节点测量清空旧 metrics／placements，不缓存历史尺寸来配另一套子树布局。
- lazy 候选集合在根或 portal 最终测量后统一提交；中间试测不 trim、不切换输入参与状态，也不验证临时省略的 pinned 项。最终只收尾实际测量的子树，沿用已有按需挂载、保留和失败退出规则。
- Row／Column 与公共 Flex 共享 basis／grow／shrink、min／max 冻结分配、交叉轴 stretch／baseline、主轴留白、padding 和显式 clip。已迁移 builder、示例及旧布局测试。无界且未改变 Content basis 时保留自然测量，避免嵌套布局重复施加无分配需求的紧约束。
- aarch64 macOS release：ui、style、三套主题和 component 共 6 个受影响生产库构建成功；Scoop 格式化与 5 个相关 Cone 的 tests 类型检查通过。依赖重建服从现有产物指纹规则，没有扩大到编译器或其他平台验证。
- 15 项定向测试全部通过：Flex 分配／冻结／极大权重／基线／配置 4 项，Frame／Stack／参数 3 项，builder 5 项；真实 lazy、绘制和控件布局组合各 1 项。组合覆盖不同约束与失败恢复、临时省略焦点项、实例与局部 State 保留、80＋8＋212 的实际文本换行、行内控件、裁剪和静止停止提交。无未闭合失败。
- 清理本项独立检查锁文件，保留后续复用的产物、Cone cache 和 L2 草稿；x86_64 仍在 C1 统一验收。

## L2：方向、Wrap 和 Grid

- UI 增加继承的 LayoutDirection 与透明 direction 作用域；ComponentScope、普通／lazy 测量和 Scene 使用同一有效方向。方向变化保留实例、State、输入区域和焦点，显式局部覆盖停止继承传播；portal 继续沿逻辑父级继承。
- 导航默认继承布局方向，显式方向独立。ParagraphHost 与编辑器的缺省段落方向跟随布局，显式 ParagraphStyle／TextDirection 保留覆盖。Overlay 的逻辑 Start／End 在碰撞前转换为物理侧；LogicalInsets 与物理 Insets 分开，Flex／Frame／Stack／Absolute 采用逻辑水平位置，不镜像绘图或文字。
- Wrap 使用 Flex 分配和真实基线，按有限主轴换行，支持行间距及明确裁剪。Grid 提供 Fixed／Auto／Fraction、显式位置及跨度，先解析内容需求，再按最终列宽测量高度；有限 Fraction 分配剩余空间，不被自然文本宽度撑大。共享表格列宽使用普通值，不扫描虚拟数据。
- aarch64 macOS release：ui、style、三套主题和 component 共 6 个受影响生产库构建成功；58 个修改／新增 Scoop 源文件格式化通过，8 个相关 Cone 的 tests 类型检查通过，文档本地链接与 diff 检查通过。
- 9 项定向测试全部通过：方向／逻辑弹层算法 2 项，Wrap／Grid／逻辑几何／配置 5 项，真实方向切换及布局各 1 项。窗口组合验证 LTR→RTL→LTR 后的同一焦点与局部状态、显式覆盖、portal、真实文本，以及两种方向下 Wrap 基线、Grid 跨列／文本换行和 Frame／Stack／Absolute 的提交位置。测试场景的锚点登记与 Closing 后查询问题已修正，无未闭合失败；未把 IME 方向组合记作已验收。
- 清理本项独立检查锁文件和已使用的布局草稿，保留复用产物／缓存及下一项交互草稿。水平虚拟列表的方向与阅读位置一起在 S1 补齐；x86_64 仍在 C1 统一验收。

## I1：公共激活与拖动

- Surface 提供可独立关闭的事件／完成订阅、isHovered 和按下序列 claim。活动祖先接管后只取消一次更深候选，后续 move／release 沿所属路径交付；显式取消、多个按钮、关闭重入和终端通知共用原路由及清理边界。
- UI 提供 scope 所有、借用 InputArea 的 Press／InteractionState 和轴向 Drag。激活使用真实快捷键规则、当前动作和 uncancellable release 清理；Drag 在阈值与轴向条件成立后接管，坐标取已提交变换，重复／倒退时间戳产生零速度，回调关闭 owner 时先收束序列。
- Button／Checkbox 与滚动条接入公共行为；Button 增加普通 View 内容入口，文字入口共用实现。首次 render 与 onMount 之间通过普通 State 发布挂载资源，避免提前解引用 Press。component 增加连续／离散范围、thumb 最小间距、相邻区域 resize 和实际锚点 snap 的纯值计算。
- aarch64 macOS release：surface、ui、style、三套主题及 component 共 7 个受影响生产库构建成功，Scoop 格式化、7 个相关 Cone 的 tests 类型检查、文档本地链接及 diff 检查通过。
- 14 项定向测试全部通过：Surface claim／订阅 5 项，范围／尺寸／吸附 5 项，builder 1 项，真实 Press／Drag、控件和滚动条组合各 1 项。组合覆盖 disabled、键位修饰与 repeat、被阻止的释放、阈值和轴向拒绝、已提交变换、重复／倒退时间戳、显式取消、同步关闭窗口，以及自定义 Button 子树与六种主题状态。类型检查发现的 Option 比较及旧快捷键 import、控件首次 render 早于 onMount 的失败均已修正；修正后只复验受影响的控件组合，没有重跑已通过且未改动的部分。
- 清理本项独立检查锁文件和已使用的交互草稿，保留后续会复用的产物、Cone cache 及 I2 草稿。Windows／Linux 和 x86_64 仍未在本项验收。

## I2：弹层、候选项和悬浮

- Surface 增加 scope 所有的窗口指针观察，正常路由完成后仍覆盖空白区域、已消费事件及窗口失焦／关闭；按事件开始时快照交付，订阅可独立关闭。
- UI 增加受控 portal open Binding、可拒绝关闭请求、有限 Presence 和 Accepted 后的 onClosed。退出立即停用输入并保留同一子树，模态屏障直到成功移除帧才释放；退出中重开保留实例，关闭回调中的较新焦点意图不被恢复覆盖。点位弹层使用逻辑窗口坐标，无需伪造 LayoutAnchor。
- ActiveItems 将活动候选与真实焦点分开，复用已有方向／网格导航并按稳定 key 显露；Typeahead 使用 Unicode 查询和 scope deadline。Hover 共享触发区与逻辑后代内容区，提供延迟、有限三角走廊及窗口终端取消。现有 portal API、builder 和 UI／Component 调用方已统一迁移。
- aarch64 macOS release：surface、ui、style、三套主题及 component 共 7 个生产库构建成功；修改源码格式化、相关 Cone 类型检查、文档本地链接和 diff 检查通过。
- 16 项定向测试全部通过：Surface 观察 2 项、UI 公共交互／弹层定位 10 项及 modal／overlay／errors／新增组合各 1 项。窗口组合覆盖关闭拒绝与忽略 setter、退出重开、隐藏后的 Accepted 屏障、owner 退休、跨间隙悬浮、1,000 项候选的显露与重排、Unicode 查询及同一编辑焦点／IME 连接，最后在绘制借用中关闭 UI 并验证资源延后释放。
- 本批发现并修正了关闭回调的焦点检查点、已关闭动画的强制收尾和绘制中退休的求值边界。测试中等待 Accepted 的嵌套 Option 判断及最后窗口退出策略也已修正；修正后只复验受影响的窗口组合，没有重跑已通过的其他批次。无未闭合失败。
- 清理本项独立检查锁文件、使用完的草稿和诊断输出，保留复用的生产／测试产物、共享 Cone cache 及 T1 草稿。Windows／Linux 与 x86_64 均未在本项验收。

## T1：编辑事务策略

- UI 通过公共 TextEditPolicy 统一单行处理、粘贴／自定义区间变换、字符接受和 grapheme 长度限制；Input／Textarea 及 builder 传入同一不可变策略。普通程序替换保留直接模型语义，applyEdit 提供显式遵循策略的入口。
- 原生 batch 先按原始坐标准备完整候选，再映射已提交部分及末尾 raw preedit；策略中途更新不打断组合，最终拒绝恢复组合基线并确认原生 origin。非法变换、回调异常和重入在发布前失败；正文、方向选区、composition 与历史作为一个结果发布。
- 长度限制保留原有尾部和多操作之间未改的内容，不切断组合字符或 emoji ZWJ。Undo／Redo 回放最终差量；普通拒绝不拆散连续键入，组合边界单独收尾。Cut 先准备删除，再成功写剪贴板，最后发布，拒绝或写入失败均保留文档和历史。
- aarch64 macOS release：ui、style、三套主题与 component 共 6 个受影响生产库构建成功。28 项定向运行全部通过：编辑状态及单行处理 19 项、策略事务 8 项、真实窗口宿主组合 1 项。窗口组合覆盖活动 preedit 中切换策略、拒绝提交后的原生快照、粘贴变换、拒绝 Cut 与剪贴板失败；无未闭合失败。
- UI 与三个相关组件 Cone 的 tests 类型检查、20 个修改／新增 Scoop 文件的格式化检查，以及文档链接和 diff 检查通过。已清理本项草稿、原始日志和独立检查锁文件，保留构建缓存及 T2 准备资料；没有复跑此前大项或其他平台。

## T2：文本投影与选择

- 编辑宿主提供不可变 TextProjection／TextDisplay、grapheme 遮罩、原文 caret／selection 几何与明确的 Copy／Cut 策略。模型、历史和 native 文档始终保存原文；显示映射与自己的布局一同保留到 Accepted，视觉命中和导航不借用另一帧的映射。
- 新增公共 SelectableTextHost 和 component 的 SelectableStyledText／builder，沿 ParagraphHost 保留样式与行内对象。方向选区、词／行选择、纯文本复制及 keyed 范围动作共用现有导航、FocusTarget 和 Press；文本拖选通过同一指针认领取消链接点击，没有额外编辑模型或 IME 连接。
- ComponentScope 允许同一挂载内嵌套 InputArea，范围动作使用这一能力；跨挂载借用仍在普通归属边界拒绝。正文和动作范围变化按 Accepted 几何恢复输入，失败帧保留旧布局，截断恢复保留行内实例。
- aarch64 macOS release：ui、style、三套主题及 component 共 6 个受影响生产库构建成功。18 项定向运行全部通过：投影 5 项、导航 5 项、真实编辑／选择窗口组合 1 项、组件 builder 6 项、外部 DSL 消费 1 项。窗口组合覆盖活动 preedit 中切换显示、原文复制／剪切、组合字符删除、链接转拖选、文本替换、截断和 Discarded 帧；最终资源释放且停止请求帧。
- UI tests 类型检查、31 个修改／新增 Scoop 文件格式化、文档链接和 diff 检查通过。修正了测试中的类型推断、receiver 引用及程序选区重启连接后的等待边界，并修正宿主内部焦点交接时的拖选候选收尾；修正后只复跑受影响的窗口组合。无未闭合失败。清理本项草稿、原始日志和独立检查锁文件，保留复用缓存及 V1 草稿；系统中文输入法与 x86_64 留到 C1。

## V1：装饰与图像

- graphics 提供四个物理椭圆圆角、共同归一化及内缩／外扩轮廓，Path 与 Canvas 使用同一几何。Style 的四边宽度／颜色和逻辑圆角先按方向解析；不同颜色的边框在共同内外轮廓内分区合成，半透明接缝保持一次覆盖。
- 三套主题、部件几何、Style builder 及已有 Button／Checkbox／Frame／Input／Textarea 已同步迁移。背景、clip、阴影和 focus ring 共用解析后的物理轮廓，不保留旧的统一 border／cornerRadius 字段。
- Component 增加借用 Image／Path 的展示与 builder、Contain／Cover／Fill／None、逻辑对齐和显式裁剪；逻辑自然尺寸与 viewBox 明确分开，不增加加载器或控件专用 native。替换和卸载不关闭调用方资源。
- aarch64 macOS release：graphics、graphics.text、surface、ui、style、三套主题及 component 共 9 个受影响生产库构建成功。53 项定向运行全部通过：图形几何 3 项、装饰 19 项、主题 18 项、组件样式及内容适配 12 项、真实布局窗口组合 1 项。覆盖非对称圆角、厚边框、半透明接缝、Raster／Metal、六种主题、方向切换、内容替换与行内布局；窗口最终停帧并释放挂载资源。
- 图形／样式／组件和已迁移测试／示例的类型检查、54 个 Scoop 文件格式化、native 严格编译警告、文档链接和 diff 检查通过。收尾修正了旧字段调用、lambda 返回及测试中的浮点比较；裁剪蒙版和路径绘制的抗锯齿差异按共同轮廓及一像素边缘验证，没有要求两种光栅路径逐字节相同。修正后只复跑失败用例，无未闭合失败。
- 清理本项草稿、原始输出和独立检查锁文件，保留复用的生产／测试产物、Cone cache 及 S1 准备记录。Windows／Linux 和 x86_64 均未在本项验收。

## S1：阅读位置与滚动意图

- ScrollState 的请求与 Accepted 快照分开，连续 scrollBy 在尚未呈现的请求上累加并返回实际消费位移；嵌套 wheel 和编辑器边缘拖选使用该返回值。滚动观察区分 User／Programmatic／LayoutCorrection，保留各请求自己的序号，支持边界输入、订阅关闭、同步重入和异常后的继续交付。
- VirtualListHandle 提供可观察的已提交阅读区间、稳定 key／项内偏移、末端状态及 restore。恢复仅按存在的 key 定位，按需测量并收敛偏移；用户新请求、源替换及卸载取消旧恢复。水平 RTL 保留物理滚动坐标，同时按逻辑方向排列、翻页和保持阅读位置。
- aarch64 macOS release：ui、style、三套主题及 component 共 6 个生产库构建成功。25 项定向运行全部通过：滚动状态／意图 9 项，虚拟范围／尺寸索引／阅读几何 11 项，以及滚动视口、可变列表、RTL 列表、嵌套滚动条和 lazy 生命周期 5 个真实窗口组合。覆盖 prepend、异步 ImageView 增高、失败帧、布局后的新请求、动画接续、恢复取消、方向切换及资源释放。
- RTL 组合发现祖先与子项同轮失效时可能漏更新，且普通协调漏传保留 lazy 条目的方向。更新队列改为先祖先后后代，仅跳过实际已更新节点；方向变化同步传播到保留项。已补充同源 Props 更新与局部 State 同时变化的回归，并复验 RTL 和已有 lazy 生命周期场景；没有重跑无关批次，无未闭合失败。
- 6 个相关 Cone 的 tests 类型检查、34 个修改／新增 Scoop 文件格式化、文档本地链接和 diff 检查通过。清理本项草稿、原始输出和独立检查锁文件，保留后续复用的产物、Cone cache 与 C1 准备资料。系统中文输入法和 x86_64 仍留在 C1，Windows／Linux 本项未验收。

## C1：公开 API 组合与两架构验收

- [布局组合](../../../sysroot/src/scoop.ui.component/tests-cones/layout/tests/combination.scoop)用普通 View、Flex、Press、Drag、resizePair、ScrollViewport 和 snapTarget 组成可调整侧栏、固定按钮加弹性输入、不同宽度的吸附内容带。真实拖动后切换方向，保留 keyed 局部 State、焦点和 IME 连接；按实际测量锚点吸附，取消恢复起点，拖动不额外触发子项点击，结束后停帧。
- [候选弹层](../../../sysroot/src/scoop.ui.component/tests-cones/overlays-items/tests/active.scoop)展示 window 从同一 ShortcutChord 生成的标签，ShortcutScope 用该 chord 开关虚拟候选列表，输入框持续持有焦点与连接。保留已有受控退出、悬浮走廊、typeahead 和关闭中资源借用组合，并复验独立的嵌套模态场景。
- [文本组合](../../../sysroot/src/scoop.ui/tests-cones/editing/tests/ime.scoop)使用已启用的 macOS 系统简体拼音和带 CGEvent 的实际按键解释路径，覆盖原文预编辑、单字素长度约束、遮罩、方向切换、组合提交拒绝、撤销／重做、分槽显示、组合字符／emoji 粘贴、复制和拖选。分槽使用一份编辑模型；测试保存并恢复输入源与剪贴板。平台调用只在 window 的 native 测试辅助中，不进入 UI／Component 生产源码。
- 可变列表与 RTL 虚拟列表的既有组合再次通过，覆盖稳定 key、已提交阅读位置、prepend、异步图像增高、恢复、手动中断、祖先与子项同轮更新，以及挂载资源释放。
- aarch64 macOS release 的六个真实窗口组合全部通过。收尾修正了测试的窗口重显焦点、逐键交付、系统 IME 按键构造和标量／UTF-8 索引转换；只复跑受影响组合。C1 没有新增生产接口或修改生产实现。
- x86_64 release：十一项受影响生产库和七个公共依赖共十八个库全部构建成功，六个窗口组合全部在 Rosetta 下通过；两架构共十二个组合运行通过。原有 core 预编译产物使用旧编译器格式，按当前工具链重建本次依赖，并用现有 runtime-only 入口准备 runtime。未增加旧格式兼容路径，也未重建编译器或无关系统库。
- Rosetta 的悬浮组合首轮收到非脚本位置的原生鼠标移动，按悬浮规则关闭；无代码改动的单独复跑通过。原生窗口组合需要前台焦点及稳定指针，本轮无未闭合失败。
- UI、Component、Style 和三套主题的生产源码入口均无 OS 条件、平台专用调用或额外 native 源码。Windows／Linux 仍按 [平台交接清单](PLATFORM.md#6-源码整理与-windows-接入)另行实现和验收；本轮不声称这些后端已可用。
- 三个相关 Cone 的 tests 类型检查、十五个 Scoop 文件的格式检查、native 严格编译警告、文档本地链接及 diff 检查通过。清理本项临时诊断程序、原始日志、草稿和独立检查锁文件，保留可复用的生产／测试产物及 Cone cache。

## 合入 coroutine executor 后的迁移与复验

- 合入 origin/main 的 `b86db1482`（coroutine executors），窗口集成使用新的 `eventLoop().attach` 和 receiver scope API。同步 Window／UI 规范及设计：应用作用域保存 owner executor，默认任务排队启动，任意真实挂起后的外部完成回到保存的 executor；后台任务通过显式 executor 执行，UI 更新通过 `withExecutor(loop.executor)` 或捕获的应用作用域交回 owner。Context binding 继续只表达依赖传播。
- 移除窗口、Surface 和下载示例中 23 处 lambda 的显式 `suspend` 前缀，并同步文档示例；函数声明与函数类型保留挂起效果。下载控制器补齐排队正文开始前的取消，直接结束本次状态，避免等待从未进入的 finally；回归确认目的文件保持原内容，随后仍能正常开始下载。
- Surface 的真实窗口组合新增后台 executor、Channel、UDP、blocking 成功与异常、捕获 owner scope、显式切回 UI 及绑定传播断言。原生 modal／menu 循环期间持续接收后台回传并绘制，根取消后继续驱动后台非取消清理及 owner 收尾。窗口组合验证默认任务排队及恢复位置；无窗口退出组合同时在 macOS 启用，等待真实接收挂起后再退出，避免固定延时和空测试运行。
- aarch64 macOS release：本次依赖范围内 29 个生产库构建成功，包含 UI／Style／三套主题／Component、窗口集成和 HTTP client。8 项测试全部通过：窗口、无窗口退出、Surface 组合各 1 项，下载示例 5 项；下载组合覆盖真实 HTTP、后台摘要、启动前取消、传输及校验取消、任务替换、双窗口隐藏／关闭和停止请求帧。
- x86_64 macOS release：宿主组合所需的 11 个生产库构建成功，窗口、无窗口退出、Surface 3 项测试全部在 Rosetta 下通过。两架构共 11 项运行通过；本次未重复 x86_64 的整套 UI／下载构建，Rosetta 也不代表真实 Intel 硬件验收。Windows／Linux 仍在独立分支补齐。
- 本次 runtime ABI 更新后，按当前源码同步构建编译器及 `scoop-pm` 等配套命令，并准备两架构 runtime 及上述依赖；旧 package manager 导致的 ABI 拒绝已通过重建闭合，准备文档已明确工具须配套更新。复验按受影响组合集中进行，没有运行编译器全量测试或重跑 P1–S1 的其他用例。
- timer 与下载应用的 aarch64 release 可执行程序构建成功。11 个修改／新增 Scoop 文件格式检查、文档本地链接及 diff 检查通过；相关 UI 基础库与示例没有遗留旧 EventLoop 构造／runWithResult 或显式 suspend lambda。无未闭合失败；清理本轮临时构建脚本、诊断采样和原始日志，保留生产／测试产物及共享 Cone cache。
