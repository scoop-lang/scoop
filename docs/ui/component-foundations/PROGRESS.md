# 控件基础补齐实施记录

日期：2026-11-04。状态：实施中，V1 已完成，后续为 S1／C1。

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
| S1 阅读位置与滚动意图 | 待实施 |
| C1 组合验收 | 待实施 |

本轮实现先验证 macOS aarch64，最后补做受影响生产库与代表组合的 x86_64 Rosetta 验证。Windows／Linux 后端实现及构建／桌面验收、accessibility、原生控件交互和 shadcn 控件库不在本轮范围。

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
