# shadcn 控件实施记录

分支：`codex/shadcn-components`。按 [设计](DESIGN.md) 的 P0–P9 顺序推进；本轮只在 macOS 验证，最终补 x86_64 Rosetta。每个独立变更验证后提交、推送并合并 `origin/main`。尚未完成的目录项不计为交付。

## P0a：完整配色与主题

- ThemePalette 直接迁移为完整的语义颜色对、独立 input／ring、五色 ChartPalette 和 SidebarPalette。三个已有主题及控件、示例、测试同步迁移，没有旧字段别名或运行时缺省补色。
- 新增独立 `scoop.ui.style.shadcn`，复用公共 ShapeTheme／ThemePainter；实现 Base Nova／Neutral 的 Light／Dark 配色、14／20 文字、32 高操作表面、16 标记和 3 宽焦点环。源 OKLCH、sRGB 转换结果及 MIT 许可证随源码保留。
- ThemeMotion 的显式 Reduced 模式在局部 transition 解析后生效；Button／Checkbox 消费同一策略。此项不包含 OS 动态偏好接入。
- macOS aarch64 release：17 个所需生产库构建通过；主题测试、控件库和现有主题示例的 tests 类型检查通过。主题的 21 项定向测试全部通过，覆盖四套配置、原主题几何／Raster／Metal／资源回归以及 shadcn token、几何／像素和减少动态。25 个修改／新增 Scoop 文件格式检查及 diff 检查通过。
- 本地预编译 UI、Skia 和文本后端落后于现行源码，分别导致接口缺失、PathOps 缺符号和旧 Rust std 的 compact-unwind personality 超限；按已有准备脚本重建后全部闭合，没有修改编译器或链接策略。GN、Skia 与 Rust 文本依赖构建保存在 `../scoop-dep`，外部下载使用指定代理；本项原始日志、过期参考归档和独立检查锁已清理。
- 后续分别交付 P0b 的统一焦点可见性与 P0c 的真实窗口 Gallery；P1–P9 尚未实施，64 项目录当前完成数仍为 0。x86_64 留在整体收尾；Windows／Linux 未验证。

## P0b：统一焦点可见性

- Surface 的同一聚焦入口记录 Pointer／Keyboard／Programmatic 来源；Automatic／Always／Never 策略决定可见指示。真实按键只改变来源和必要指示，不改焦点修订号；同目标、同指示的重复请求不产生额外帧需求。
- FocusHandle、FocusOptions、Tab／roving／lazy 导航和延迟显露沿同一路径传递来源，模态关闭恢复目标原有来源。Press 与文本宿主同时公开实际 focused 和 focusVisible，主题几何、绘制及局部 focused 覆盖只消费后者。文本宿主的 Automatic 包含可编辑及只读输入的指针焦点。
- 仅指示变化通过独立通知发布，不重发 FocusChanged、不重建 IME、不改选区。Surface 的 74 项相关定向测试通过；其中新测试覆盖真实／synthetic 键盘、三种策略、OS 失焦／恢复、失败请求、禁用／关闭及 IME 所有权。原有重复聚焦测试捕获的额外刷新已修复并复验通过。
- macOS aarch64 release 的四个真实窗口用例全部通过：Tab／延迟显露／句柄生命周期、模态恢复、Button／Checkbox 的指针与键盘焦点及 Metal 停帧、文本输入／只读／IME／主题切换／虚拟移除。八个受影响生产库构建、相关 tests 类型检查、38 个 Scoop 文件格式检查及 diff 检查通过；没有扩大到全仓测试。
- 已清理本项 Surface／焦点／模态测试输出和独立检查锁；保留后续控件批次会复用的主题／控件测试目录及共享编译缓存。P0c Gallery 尚未交付，64 项目录完成数仍为 0；x86_64 留到收尾，Windows／Linux 未验证。


## P0c：状态外观与真实窗口 Gallery

- Button／Checkbox 增加 invalid；Input／Textarea 迁到同一主题 validation。OutlineStyle／OutlineRing 同时表达焦点与错误状态，最终几何只保留一个环；错误状态独立于焦点，局部边框和 validation 覆盖继续有效。四套主题提供完整验证外观，选中 Checkbox 保留 primary 边框。
- 禁用透明度作为整体合成值，覆盖背景、内容、边框和状态环；shadcn 使用 0.5，旧主题保留各自禁用配色。Nova Input 补齐明暗禁用填色。文本留白点击沿 Pointer 来源聚焦，主题示例与分阶段绘制消费者同步迁移。
- 新增 `scoop.ui.component/examples/shadcn` 的实际原生窗口、搜索与三类控件状态预览。主题、方向、页面和搜索切换保持应用模型；正文使用同一 TextEditorState 与原生 IME，未建立另一套绘制或交互实现。实际窗口核对了明暗模式、紧凑按钮、侧栏顶对齐和 RTL 布局。
- macOS aarch64 release：6 个受影响生产库重建通过；控件测试、主题测试、Gallery、既有主题示例和文本控件测试类型检查通过；24 项主题测试及 2 项 Gallery 测试全部通过。真实窗口测试覆盖 Space 激活、焦点、中文 marked／commit、主题／方向／页面切换、选择保持、停帧及资源释放。42 个 Scoop 文件格式检查及 diff 检查通过。
- 首轮窗口测试暴露的销毁后焦点查询已修复；低 alpha 像素检查按预乘通道比较，避免反预乘放大 8 位舍入，容差保持 2。一次性检查锁已清理，继续复用外部依赖与共享编译缓存。P0 已闭合，64 项目录完成数仍为 0；后续从 P1 基本内容与操作开始，x86_64 留在整体收尾，Windows／Linux 未验证。

## P1a：Button、Badge 与 Spinner

- Button 增加六种视觉变体和八种尺寸，保留 String／StyledText／View 内容；新增 ActionContentStyle 回调，让 Button／Badge 的自定义图标、文字和 Spinner 使用同一解析结果。局部尺寸、文字和状态覆盖继续优先；四套主题保持可用。
- Badge 提供六种变体及纯展示／显式动作入口，动作模式复用 Press、焦点和最新回调；撤销动作立即解除原目标。Button 与 Badge 共用表面布局和 painter，未保留平行绘制实现。Link 的文字下划线复用实际段落选择几何。
- Spinner 使用带来源许可的 Lucide loader-circle 路径与 scope-owned Animatable；停止、Reduced、零尺寸、透明、视口裁剪、窗口隐藏和卸载均收束帧需求。普通滚动区按成功帧的保守 clip 交集设置既有 isFrameEnabled，滚回后沿原时间轴恢复，不增加 UI 调度机制。
- macOS aarch64 release：生产控件库、Gallery 和原有交互测试构建通过；13 项控件变体／样式／DSL 测试、2 项 Gallery 测试及 1 项原有控件窗口回归全部通过。Gallery 的 43 阶段窗口用例覆盖 IME、回调替换、禁用／撤销目标、减少动态、离屏／恢复、隐藏／恢复、停止及卸载；实际窗口复查六变体、八尺寸、明暗、Badge 与 RTL。末次示例内边距调整另经格式／类型检查与窗口复查。
- 首轮 Spinner 绘制在借用子 Canvas 时访问 SceneContext 的错误已修复。此环境的后台窗口请求未取得系统键盘焦点，显式激活测试窗口后两项窗口测试通过，未放宽断言。原有低 alpha／主题回归继续复用已完成结果，没有全仓复验。
- 本批完成 Button、Badge、Spinner，目录累计 **3／64**；其余 P1 与 P2–P9 继续实施。一次性检查锁清理，外部 Lucide 文件保存在 `../scoop-dep/lucide`，生产与后续复用的测试缓存保留。x86_64 留在整体收尾，Windows／Linux 未验证。

## P1b：内容面板、分隔线与比例布局

- 新增 Card、Alert、EmptyState，以类型化标题／说明和明确的 View 区域组合内容。Card 提供 Default／Sm、头部分隔线和 footer；Alert 提供 Default／Destructive、图标及动作；EmptyState 提供两种媒体、居中限宽正文和可选虚线边框。区域使用稳定 key，窄宽动作换行、增删相邻区域与主题切换保留正文挂载及焦点。
- Separator 支持横纵方向、明确长度、厚度与空线；AspectRatio 处理有限上限、兼容下限、冲突父约束及无界自然尺寸，最后按同一外框测量与裁剪。Empty 的虚线路径仅在几何变化时重建，替换及卸载释放资源；有限宽度下外框占满可用空间，正文仍保持建议宽度。
- Gallery 增加五个可操作页面，包含窄宽／区域切换、按钮和输入组合、明暗／RTL，以及同一借用图像的 16:9／1:1、Cover／Contain 对照。图像在挂载阶段创建并通过 State 发布，作用域负责释放；示例不下载外部图片。
- macOS aarch64 release：生产库、Gallery 程序及测试构建通过；4 项定向单元测试、1 项面板窗口用例和 1 项搜索测试全部通过。窗口用例覆盖无界 Alert、Empty 满宽／自然尺寸、正文焦点保留、最新回调、可选区域增删、宽窄／尺寸／主题／方向变化、切页及停帧。人工复查五页排版、明暗／RTL、虚线和图片裁剪；16 个新增／修改 Scoop 文件的格式、相关 tests 类型检查及 diff 检查通过。
- 本批发现并修复 Alert 在无界宽度下丢失自然尺寸、Empty 外框未占满有限父宽，以及图片示例在首次 render 读取尚未创建资源的问题。只复验受影响的面板和示例，没有重跑前两批完整交互或全仓测试。一次性检查锁与测试窗口已清理，保留下一批复用的产物和共享缓存。
- 本批完成 Card、Separator、AspectRatio、Alert、Empty，目录累计 **8／64**。Avatar／InputGroup 等后续控件的跨家族组合随所属批次补齐；其余 P1 与 P2–P9 继续实施。x86_64 留在整体收尾，Windows／Linux 未验证。

## P1c：Avatar、Skeleton 与 Progress

- Avatar 提供三种尺寸、图片／加载／失败回退、状态点与自定义角标，以及有稳定 key、重叠和显式计数的头像组。图片由调用方持有，替换不关闭借用资源；Gallery 用自身绘制的两张头像覆盖替换、明暗／RTL，并与 Empty 组合。graphics 补齐头像边框实际使用的 Darken／Lighten 图层混合。
- Skeleton 支持矩形／圆形、明确或内容尺寸、静态／脉冲模式；切换保留正文挂载、测量和内部状态，同时撤下隐藏正文的输入与动画帧资格。普通 Element 未放置子树在 Accepted 后更新已有 AnimationGroup，恢复沿原时间轴；没有增加可见性扫描或时钟。
- Progress 支持合法范围内的确定值、钳制显示、未知总量、标签／自定义数值和 RTL；首次直接显示，后续使用主题选择过渡，Reduced 最终生效。Spinner、Skeleton、Progress 共用小型循环动画与成功帧可见性处理，未知进度以整条轨道判断可见性。
- macOS aarch64 release：10 个受影响生产库及 Gallery 构建通过；5 项组件定向测试、1 项混合模式 Raster 测试、3 项 Gallery 窗口用例和 1 项搜索测试全部通过。新增窗口用例覆盖图片借用／卸载、隐藏正文焦点请求的结束、内部计数保留、正文循环动画停帧／恢复、零宽、透明、Reduced、滚动离屏、窗口隐藏及确定／未知切换。原 Spinner／IME 与内容面板用例复验通过；新增 Progress／头像组实际尺寸断言通过。
- 实窗发现的数值段落占满父宽问题已修复：共享标题／动作布局先取动作自然宽，必要时再按上限换行。首轮测试错误地把 requestFocus 的“接受异步请求”当成“已经取得焦点”，已按既有合同改为检查隐藏目标最终无焦点且请求结束，未更改焦点生产逻辑。所有修改的 Scoop 文件格式、相关 tests 类型检查、C++ 修改行格式及 diff 检查通过；人工复查头像、Skeleton、Progress、明暗／RTL 和目录滚动。
- 本批完成 Avatar、Skeleton、Progress，目录累计 **11／64**。未重复全仓测试；本项检查锁、一次性 graphics 测试输出、退役的操作测试输出及测试窗口已清理，继续复用组件／Gallery 产物、共享编译缓存与 `../scoop-dep`。x86_64 留在整体收尾，Windows／Linux 未验证；其余 P1 与 P2–P9 继续实施。

## P1d：Typography 与 Kbd

- Typography 提供 H1–H4、P、Blockquote、InlineCode、Lead／Large／Small／Muted，以及项目符号列表和共享 Grid 列宽的文字表格。默认选择与复制复用 SelectableStyledText；显式 StyledText 保留原样式，行内 code 使用带基线、圆角背景和替代文字的实际 View，复制结果包含原代码文字。没有增加 Markdown／Typeset 或跨段落编辑模型。
- Kbd 支持文字、自定义 View、ShortcutChord／ShortcutLabel，以及原生分组和自由组合；窗口负责平台标签，快捷键提示保持平台顺序。Gallery 的 Increment 按钮和提示复用同一个 ShortcutChord，实际绑定由页面作用域持有，卸载即解除。
- macOS aarch64 release：生产组件库、Gallery 程序与测试构建通过；4 项组件定向测试、1 项新增窗口用例和 1 项搜索回归全部通过。22 阶段窗口用例覆盖真实 Select All／Copy、UTF-8 选区与代码替代文字、主题／RTL／窄宽后的选区保留、行内动作、撤销／恢复选择、快捷键与 Space 激活、页面卸载及最终停帧；剪贴板完整暂存并恢复。实际布局另断言键帽尺寸／基线、表格自然宽与窄宽换行、行内代码在足够宽度下保持自然行高。
- 实窗发现并修复行内 code 被段落最大宽度撑满、短键帽文字偏离中心的问题：两者先取自然宽度，再按可用空间收缩。首轮测试错误地假定窗口会将逻辑字符转成大写，已按实际原生标签修正；没有修改窗口标签或快捷键匹配。最终格式、相关 tests 类型检查和 diff 检查通过；人工复查所有文字角色、列表／表格、行内内容、键帽／按钮组合、明暗及 RTL。
- 本批完成 Typography、Kbd，目录累计 **13／64**。没有重建编译器或图形后端，也未重复前批全套交互或全仓测试；一次性检查锁、手工窗口日志和测试窗口已清理，保留后续复用的产物、共享缓存与外部依赖。P1 剩余 ButtonGroup、FieldLabel、Item，P2–P9 继续实施；x86_64 留在整体收尾，Windows／Linux 未验证。

## P1e：FieldLabel 与真实控件关联

- 新增 FieldTarget 与 FieldLabel，Input／Textarea／Checkbox 通过 labelTarget 接入同一作用域关联。标签使用非聚焦 Press，不增加 Tab 停靠点；文本标签保留选区并请求焦点，Checkbox 标签读取最新 Binding 并执行一次选择动作。当前 FocusOptions 替换、目标换绑、禁用、隐藏、卸载、模态输入域和跨窗口均沿真实控件资格处理；按下期间换绑取消旧点击。
- 普通文字沿主题提供 14／14、500 的标签外观，支持 StyledText、字体／段落与局部装饰；无关联或禁用时整体透明度 0.5。原有 Label 保持展示用途。目标 enabled 不变时不重复发布，父组件可正常订阅关联状态。Gallery 新增名称／备注／订阅的 Label 页面，提供禁用与加载恢复，复用现有输入模型和 Skeleton。
- macOS aarch64 release：生产组件库、Gallery 程序与测试构建通过；新增 32 阶段双窗口用例、原有控件窗口回归和搜索测试共 3 项全部通过。覆盖真实指针、Tab／Space、选区保留、最新 Binding／FocusHandle、按压换绑取消、禁用／隐藏／卸载、模态域外与跨窗口拒绝、只读 IME、资源释放及停帧。最终相关 tests 类型检查、136 个 Scoop 文件格式及 diff 检查通过；实窗复查排版、标签点击、明暗／RTL、禁用和加载恢复。
- 首轮测试的系统焦点与双窗口呈现准备问题已闭合；浮动测试窗口按实际窗口 ID 激活。只读输入按既有合同不开启 IME，原生文本注入应被拒绝，测试预期已修正；没有更改焦点或 IME 生产策略。没有重建编译器／图形后端或扩大全仓测试，一次性检查锁、手工窗口日志和测试窗口已清理，保留后续复用的产物及共享缓存。
- 本批完成 Label，目录累计 **14／64**。P1 剩余 ButtonGroup、Item，P2–P9 继续实施；x86_64 留在整体收尾，Windows／Linux 未验证。

## P1f：ButtonGroup 与连接表面

- 新增 ButtonGroup／ButtonGroupProps、明确类型的 ButtonGroupEntry 及接收者 DSL；button 的四种内容、input、text、separator、显式位置的 item、原样 add 和嵌套 group 均有实际入口。Button／Input／Textarea 的 group 接收普通 GroupPosition，不读取不透明 View 的内部结构。规范第 16 节、公共 API 和库／Gallery README 已同步。
- 横纵连接共用边框内缩、圆角、内容区域和焦点环几何；RTL 按逻辑起止侧处理，明确的局部外侧圆角保留。排列复用 Flex，纵向按条目自然宽度受约束拉伸；输入框可增长，空／零尺寸合法。InputScope 活动观察只请求必要重绘，焦点条目最后呈现，Tab 顺序和原控件动作保持声明顺序。
- Gallery 增加 Button Group 页面，包含横纵切换、禁用、文字前缀加 Input、Sm／IconSm split action 和嵌套自定义条目。手工检查明暗／RTL、自然宽度、接缝焦点环、输入及 Tab／Space 提交；地址前缀显式使用 LTR 文本方向。
- macOS aarch64：连接几何／圆角优先级／DSL 配置 3 项测试通过；新原生窗口测试的 17 阶段通过，覆盖实际测量、焦点绘制顺序、Tab、中文组合输入、方向／主题更新、按 key 重排、最新回调、禁用、移除和卸载停帧。既有 Gallery 输入／按钮窗口回归及搜索测试各 1 项通过；组件与 Gallery 静态检查、生产库／应用／测试构建、144 个 Scoop 文件格式检查及 diff 检查通过。
- 复核中修正了 activity 快照被当成响应式读取、纵向文字使组撑满父宽的问题；测试将固定高度断言放到足够宽的场景，窄宽继续验证合法收缩。一次性检查锁、手工窗口日志和测试窗口已清理，保留共享编译缓存与后续复用产物；未重建编译器或外部图形依赖，未重复全仓测试。
- 本批完成 Button Group，目录累计 **15／64**。P1 剩余 Item，后续 P2–P9 继续实施；x86_64 留在整体收尾，Windows／Linux 未验证。

## P1g：Item 内容行与独立动作

- 新增 Item／ItemGroup／ItemSeparator、类型化 Props 和接收者 DSL。三种变体与尺寸、标题／说明、媒体、独立动作和头尾区域均有实际入口；普通容器使用 contentItem，避免与 Flex.item 冲突。标题默认一行、说明两行省略，图片借用 Cover 适配并按尺寸裁剪，组可混排普通内容与分隔线。
- 行动作复用 Press 的 default 路由，内部按钮和文本选择各自消费输入；可选区域、窄宽、主题和 RTL 更新保留正文。支持最新回调／FocusOptions、展示与动作模式切换、整行禁用和减少动态。自然测量保留中间行排列，头尾仍可拉伸，修正重复拉伸改变动作排列而沿用旧行高的问题。
- Gallery 新增 Item 页面，展示全部变体、尺寸和媒体组合，可切换详情、行行为、禁用及宽度。手工检查明暗、RTL、图片圆角、文字省略和 240 宽度下的动作换行；规范第 17 节、API 和两份 README 已同步。
- macOS aarch64 release：生产库、Gallery 应用／测试构建通过，155 个 Scoop 文件格式及相关 tests 类型检查通过。2 项组件测试、新增 23 阶段窗口测试、既有面板回归和搜索测试共 5 项通过；覆盖真实点击／Space／Enter、子动作隔离、拖选复制、选区与挂载保留、最新焦点／回调、模式切换、禁用取消、资源借用及卸载停帧，另断言自然／有限／零尺寸、媒体尺寸、省略与明确段落配置、头尾分隔线和动作不重叠。焦点环测试已纠正将描边宽度加进 outset 的错误预期。
- 一次性检查锁、手工窗口日志和测试窗口已清理，保留共享缓存、后续复用产物及参考资源；没有重建编译器或外部图形依赖，没有扩大全仓验收。本批完成 Item，**P1 的 16 项全部完成，目录累计 16／64**；后续进入 P2，x86_64 留在整体收尾，Windows／Linux 未验证。


## P2a：三态 Checkbox、Switch 与 Toggle

- CheckboxProps 统一使用 Binding<CheckState>，Boolean 工厂只映射原绑定；Unchecked／Checked／Indeterminate 对应已有 Off／On／Mixed，混合态激活后选中，保留既有自有 Boolean 入口。Switch 提供 Default／Sm、轨道／滑块样式、标签关联、RTL 和不占布局的扩展点击区；颜色和位移共用一个选择动画。Toggle 提供 Default／Outline、三种尺寸和四种内容入口，复用 ActionInk／ButtonStyle，业务选中值与临时按下状态分开。
- Gallery 新增三态全选组合、Switch 页面及 Toggle 图标／文字／Preview 组合。所有 setter 使用最新绑定并允许拒绝，不保留镜像 State；焦点、标签和动画沿既有作用域管理。唯一直接构造旧 Boolean CheckboxProps 的交互用例已迁移。
- macOS aarch64 release：组件生产库、Gallery 程序与测试构建通过，相关三个 Cone 的 tests 检查通过。18 项定向测试全部通过：4 项新组件测试、10 项受影响的操作样式／DSL 回归、30 阶段新窗口用例，以及旧标签、Gallery 输入和搜索回归。覆盖混合态、Boolean 适配、尺寸／零约束／RTL、焦点与 invalid、外部拒绝写入、最新 Binding／FocusHandle／FieldTarget、Space／Enter 差异、重复键、禁用／拖出／卸载取消、独立子按钮、资源借用和停帧。
- 手工复查明暗／RTL、Switch 两种尺寸、Toggle 全变体和 Preview 内容，以及 Mixed→全选→清空的实际组合。手工自动化部分后台短点击未生效，使用前台复核和键盘确认；完整激活／取消结论来自原生窗口状态断言。169 个 Scoop 文件格式及 diff 检查通过，构建期间发现的公开 Style 构造与导入别名问题已修正。
- 本批完成 Checkbox、Switch、Toggle，目录累计 **19／64**。一次性检查锁、窗口日志和测试窗口已清理，继续保留共享缓存、组件／Gallery 产物及外部依赖；没有重建编译器或图形后端。按用户新反馈，下一步先集中修复已交付控件的状态颜色、动画序列及个别绘制问题，再继续 P2；x86_64 留在整体收尾，Windows／Linux 未验证。

## 已有控件反馈修复与系统配色

- 共享颜色插值改为预乘 alpha，消除透明背景进入 hover 时先变暗再变亮的闪色。shadcn Action、Button／Badge 变体、动作 Item 和 Toggle 为 pressed 提供独立终点，松开后返回当前 hover；业务选中值不充当瞬态按压。Skeleton 默认色增加 12% foreground 混色，保留原有 2 秒脉冲与减少动态／不可见停帧规则。
- Gallery 的程序聚焦按钮和其他辅助按钮使用 Outline；Alert 示例绘制完整圆圈图标。Separator 示例两条竖线原本分别显式使用 1／2 厚度，现统一为默认 1；没有修改分隔线绘制算法。
- shadcnTheme 接受完整 ThemePalette，并从同一配色生成文字和部件。新增 macOS 主线程入口 macosSystemPalette，读取指定明暗外观下的 AppKit 语义色快照；Gallery 默认 System，提供 System／Neutral 切换，保留独立明暗选择与全部应用状态。切换或重新聚焦时刷新系统色，其他平台继续使用 Neutral。
- macOS aarch64 release：主题 27 项、组件 10 项、Gallery 3 项定向测试全部通过。新增真实窗口测试逐帧检查 System／Neutral × Light／Dark 下四类控件的 hover／press／release／leave 颜色序列、16 次激活、Skeleton 脉冲幅度与 Reduced 停帧；原有加载控件生命周期回归通过。手工复查系统明暗、配色切换、圆圈图标与等宽分隔线；相关 Cone 格式／类型检查、原生代码静态检查和生产库／Gallery 构建通过。
- 测试初轮存在未取得系统焦点导致的超时，手工阶段也检测到真实鼠标与自动输入并行；按实际窗口 ID 激活并单独运行后全部通过，没有放宽颜色或生命周期断言，也未改输入策略。末次页脚文案调整另经格式、类型检查及应用构建；没有因此重复整组窗口测试。
- 本批修复已有项目，目录仍为 **19／64**，后续继续 P2。一次性探针、检查锁和窗口日志清理，保留可复用产物与共享缓存；未重建编译器、下载外部依赖或扩大到全仓测试。x86_64 留在整体收尾，Windows／Linux 未验证。

## P2b：Input OTP 与共享固定槽文本几何

- 新增 InputOtp、分组／分隔内容、类型化 Props 与 DSL；复用一个 TextEditorState、TextEditHost 和原生输入会话。支持数字、字母数字与自定义策略，粘贴仅清理新增片段，按 grapheme 限长；原始 preedit 可暂时扩展槽位，提交后统一执行策略。onComplete 只来自真实正文提交，外部替换和被 onEdited 改写的过期结果不触发。
- 共享文本层增加 TextEditCells／TextEditLayout，统一固定槽 shaping、命中、导航、选区、光标和滚动后的 IME 几何；保留段落布局与 Accepted 帧资源。OTP 提供组内接缝、焦点／错误外观、标签、只读／禁用、RTL、窄宽及零尺寸处理；Gallery 新增真实预览页面。
- macOS aarch64 release：7 个受影响生产库及 Gallery 构建通过，相关源码格式、tests 静态检查和 diff 检查通过。41 项共享文本测试、4 项 OTP 组件测试、既有 91 阶段原生编辑回归、Gallery 搜索测试及新 32 阶段 OTP 原生窗口测试，共 48 项定向测试全部通过。手工核对 System 明暗、RTL、Unicode、分组接缝、窄宽和完成计数。
- 原生测试修正了排队按键的过早断言、外部选择更新与 IME 会话重建的时序，以及使用空段落行高推算非空正文光标的错误预期；矩形仍精确比较。长阶段函数按场景拆分以降低静态检查开销。既有编辑回归按当前版本的 Accepted 帧同步，并修正 Selectable 失败帧检查取错布局的问题。
- 本批完成 Input OTP，目录累计 **20／64**。一次性原生探针、诊断日志和检查锁已清理，保留共享缓存与后续复用的测试产物；未重建编译器、LLVM 或外部图形依赖。按用户要求，下一批先独立去除 UI 类型匹配的反射依赖，再继续 P2；仅在 macOS 验证。
