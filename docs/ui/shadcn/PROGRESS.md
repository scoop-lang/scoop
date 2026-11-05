# shadcn 控件实施记录

当前分支：`codex/shadcn-menus-and-selection`，承接 `codex/shadcn-components`。按 [设计](DESIGN.md) 的 P0–P9 顺序推进；本轮只在 macOS 验证，最终补 x86_64 Rosetta。每个独立变更验证后提交、推送并合并 `origin/main`。尚未完成的目录项不计为交付。

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

## 独立重构：UI 描述类型匹配去反射

- ComponentDescription／ElementDescription／LazyDescription 通过普通泛型 is 检查同时匹配 Props 与实例静态类型，完整保留嵌套泛型实参。删除 ViewShape 的反射类型字段及 requireClassInstance，UI 各库源码不再直接引用 typeInfo、TypeInfo、TypeRef 或 TypeShape；未新增类型 ID、注册表、字符串身份或 compiler／runtime 接口。
- 候选节点仍按直接父级下的 key／位置选择；匹配只返回布尔值，后续 Props 更新、卸载与挂载流程不变。Empty、Direction、Portal 及模态类别保持既有区别，同类型替换工厂闭包保留实例。三个公开协议仍为接口，具体 class 静态类型与每次新挂载创建新实例改为明确 API 使用约定，活动实例重复挂载检测保留。UI 规范和设计已先行同步，原反射拒绝值类型的专门错误场景已删除。
- macOS aarch64 release：格式、相关 tests 静态检查、7 个受影响生产库及 3 个定向测试程序构建通过；13 项测试全部通过。新增 4 项纯匹配测试和 1 项真实隐藏窗口生命周期测试覆盖三类节点、完整泛型实参、判断无副作用、工厂次数、不同局部状态的 keyed 重排与位置复用、类型变化重挂载及 36 次实例资源释放；既有组合状态与 5 项 Portal 测试、包含 21 个场景的错误清理回归，以及 32 阶段 OTP 实窗回归通过。
- 本批不增加控件目录项，仍为 **20／64**，随后继续 P2。一次性检查锁和错误回归输出目录已清理，继续保留共享缓存、OTP／UI 测试产物和外部依赖；没有重建 LLVM、编译器或执行全仓／全部控件测试。


## P2c：Radio Group 与 Toggle Group

- 先修订 UI／控件规范及选择模型。KeyNavigation 增加可选 preferredTab 和接受导航请求后的 onNavigate，默认仍保持原焦点导航行为；Radio 用它表达已选 Tab 入口和方向选择。没有新增反射、类型身份、选择镜像或导航状态机。
- 新增普通泛型 Choice、RadioGroup／ToggleGroup、完整 Props 和 DSL。Radio 提供单选／空选、说明与自定义 indicator 卡片、FieldTarget、禁用／invalid；Toggle Group 提供可清空单选、多选、两种变体、三种尺寸、横纵、间距／连边及自定义图标正文。两者保留 keyed 实例与内部焦点句柄，更新时读取最新绑定；同值、重复 key、位置 key 在新元数据快照入口报告配置错误。
- 复用 Press、Theme Radio 几何／painter、Toggle／ActionInk 和原 ButtonGroup 的接缝及焦点层级。Radio 禁用透明度覆盖整项一次；移除外部 FocusHandle 时恢复该 key 的内部 handle。Gallery 增加两页，实际检查 System／Neutral、明暗、LTR／RTL、选择、卡片、连边、间距和键盘 focus-visible。
- macOS aarch64 release：先格式化和静态检查 UI、component、Gallery 及新窗口 Cone，再定向运行 **6 个测试全部通过**：ChoiceGroupTest 4 项、新 ChoiceGroupWindowTest 1 项（43 步）、既有 VirtualNavigationTest 1 项。覆盖元数据错误、未知值及顺序保留、拒绝 setter、keyed 复用、外部 handle／标签／绑定替换、Radio 与 Toggle 的键盘差异、禁用跳过、拖出／禁用／卸载取消、零尺寸、RTL、明暗切换及资源收尾。7 个受影响生产库和 Gallery 构建成功；UI 库仍无 typeInfo／TypeInfo／TypeRef／TypeShape 直接引用。
- 本批完成两项，目录累计 **22／64**；P2 继续 Slider、Input／Textarea、InputGroup 与 Field。一次性检查锁、导航回归输出与手工窗口已清理，保留共享缓存、分组回归／Gallery 产物和 `../scoop-dep` 参考。没有重建编译器或 LLVM，也未扩大到全仓／全部控件测试；x86_64 留在整体收尾，Windows／Linux 未验证。

## P2d：Slider

- 先补齐 Slider 规范，新增 Props、DSL、单／双／多 thumb、横纵、RTL／reversed、离散／连续范围、minimumGap、逐项焦点与 FieldTarget。复用 RangeSpec、Drag、FocusTarget、InteractionMotion 和已有主题绘图；按下标保留实例，拖动只使用 Accepted 几何，没有反射、额外输入体系或受控值镜像。
- 轨道点击与拖动、保留抓取偏移、主轴方向键、Home／End／Page、提交和取消均闭合；拒绝 setter 不合成提交，外部值／范围／数量更新保留业务结果。禁用、失焦、阻止释放、Escape、零尺寸和卸载沿同一取消出口收尾。连续间距按边界加减的 Double 舍入保持 move／validate 一致，重叠与错误顺序仍拒绝。
- macOS aarch64 release：先格式化与相关 tests 静态检查，生产控件库和 Gallery 构建成功；**11 项定向测试全部通过**，包括 5 项 Slider 值测试、5 项既有范围测试和 1 项 64 步原生窗口测试。覆盖稳定 thumb／焦点与工厂次数、最新绑定、12 次提交／8 次取消、标签及句柄替换、连续／离散范围、窄宽／失败帧／零尺寸、明暗／方向变化、静止停帧和资源释放。
- Gallery 增加可操作 Slider 页，人工检查 System／Neutral、明暗、RTL、反向、窄宽、错误／禁用、竖向多 thumb、标签聚焦、键盘和减少动态下的真实拖动。原生测试初轮捕获到测试注入之外的窗口外鼠标移动，导致活动拖动按正常语义钳到上限；单独运行并避免重复窗口激活后，最终无库内诊断的用例通过，未放宽断言或修改输入策略。
- 本批完成 Slider，目录累计 **23／64**；P2 继续 Input／Textarea、InputGroup 与 Field。一次性检查锁及手工窗口清理，保留共享缓存、Slider 回归／Gallery 产物及 `../scoop-dep` 中的上游参考；未重建编译器、LLVM 或外部图形依赖，未运行全仓或全部控件测试。x86_64 留在整体收尾，Windows／Linux 未验证。

## P2e：Input／Textarea 与 Input Group

- 先补齐控件规范，新增 TextFieldChrome.Framed／Bare；内容模式沿用同一 TextEditHost，去除内层外框、边框占位和禁用合成层，保留 padding、正文、描述及滚动条。Textarea 默认内缩按字号缩放为横向 10、纵向 8。Input Group 提供一个明确编辑槽、四向 addon、文字／独立按钮／自定义 View、根状态及稳定焦点句柄；不遍历不透明 View，也不新增反射或文本模型。
- 按原父级和 key 保留 addon，物理位置与声明顺序分开；修改外观、句柄、回调或模型不重建同类型宿主。输入与附加按钮各自处理动作及 Tab，整体禁用透明度只应用一次。原生回归发现并修正零尺寸组仍放置子节点却跳过绘制的问题，现由未放置节点和 Accepted 帧撤销输入与焦点。
- Gallery 补齐 Input 的密码显隐、数字策略、只读与应用文件选择动作，新增 Textarea 和 Input Group 页面。手工检查 System／Neutral、明暗、RTL、四向／窄宽 addon、错误／禁用外框、中文多行输入与提交、密码显隐、数字限长和样例文件动作；修正示例父布局拉伸窄宽及焦点环贴近裁剪边界的问题。API 与两份 README 已同步。
- macOS aarch64 release：相关 Cone 先格式化与 tests 静态检查，生产库及 Gallery 应用／测试构建成功；**9 项定向测试全部通过**，包括 Input Group 4 项、文本样式 2 项、Gallery 搜索 1 项、既有 91 阶段文本编辑回归，以及新 48 阶段 Input Group 实窗测试。覆盖 preedit／中文提交、复制／撤销、掩码、只读、禁用、模型／句柄／回调替换、输入类型切换、自定义 Bare／Framed 切换、keyed 实例及局部状态保留、四向布局、窄宽／零尺寸、静止停帧与资源收尾。初轮另修正零建议宽度的测试预期，以及原生只读拒绝输入的返回值断言；未放宽生命周期或编辑行为断言。
- 本批完成三项，目录累计 **26／64**，P2 下一项为 Field。为消费已合入主线的 runtime ABI 34／metadata ABI 11，对齐了五个宿主命令、runtime 和系统库；使用现有 LLVM 与外部依赖缓存，没有重建 LLVM 或图形工具链。一次性检查锁及手工窗口清理，保留共享缓存、定向回归／Gallery 产物与 `../scoop-dep` 参考。UI 库仍无 typeInfo／TypeInfo／TypeRef／TypeShape 直接引用；未运行全仓或全部控件测试，x86_64 留在整体收尾，Windows／Linux 未验证。

## P2f：Field 家族与设置表单

- 先补齐控件规范第 23 节，新增 Field、FieldSet、FieldGroup／FieldContent、标题／说明／legend／错误列表及带文字分隔线。Field 使用固定控件槽，横／纵／响应式和逻辑起止放置只改变测量与位置；标题、说明、错误和集合 legend 增删保持正文实例、选区与 IME。错误过滤空字符串并按首次出现去重，自定义内容优先；required 支持本地化说明，校验及控件状态继续由业务明确传入。
- FieldLabel 增加 View 内容，用同一 FieldTarget 关联 choice card；内部 Checkbox 消费动作后外层不再激活，保持非 Tab 标签合同。Field／分组不新增焦点区域、输入模型、反射或类型身份。Gallery 增加可操作的设置表单，组合 Input Group、OTP、Textarea、Checkbox、Switch、Radio 和 Slider，提供提交校验、重置、宽度／布局／禁用／只读切换；DatePicker 留在 P7 接入。API 和两份 README 已同步。
- macOS aarch64 release：三个 Cone 的格式与 tests 静态检查通过，生产库、Gallery 应用及相关测试程序构建成功；**8 项定向测试通过**，包括 4 项 Field 组件测试、设置表单与 Gallery 搜索各 1 项、既有 FieldLabel 窗口回归，以及新的 40 阶段 Field 实窗用例。覆盖实际宽度阈值、RTL、可选槽、嵌套集合、keyed 状态／工厂次数、preedit／中文提交、标签换绑、choice card 内外点击各一次、键盘、只读／禁用、零尺寸恢复、卸载与停帧。
- 手工复核发现并修正默认布局裁掉输入框焦点环、分隔文字占满宽度后两侧线条消失的问题，窗口测试补充外扩绘制区域与分隔内容居中的断言。System／Neutral、明暗／RTL、横纵／窄宽、单多错误、中文编辑和有效提交已实窗检查。另修正了零尺寸时将 FocusHandle 的“接受待处理请求”误当作“立即取得焦点”的测试预期，继续断言焦点清除、请求取消及恢复后不自动聚焦。221 个 Scoop 文件格式与 diff 检查通过。
- 本批完成 Field，**P2 全部完成，目录累计 27／64**，随后进入 P3。复用共享缓存与现有 LLVM，未重建编译器或外部工具链，未重复全仓／全部控件测试；一次性检查锁和手工窗口清理，保留后续复用产物。7 个 UI 库源码仍无 typeInfo／TypeInfo／TypeRef／TypeShape／requireClassInstance 直接引用。x86_64 留在整体收尾，Windows／Linux 未验证。


## P3a：Direction 与 Scroll Area

- Direction 沿用框架的透明作用域，新增独立 Gallery 页面，组合继承方向、局部 LTR、逻辑 Start 边框／留白、混合文字和方向键选择组；热切换保持编辑值、选择与点击计数，没有平行方向实现。
- ScrollArea 增加 style／focusOutline，复用 Theme 的表面与焦点环、StylePainter 和原有 ScrollViewport；圆角同时裁剪正文、条和命中，装饰不改变内容测量。修复按实际焦点直接画固定矩形及忽略 FocusVisibilityChanged 的问题，Automatic／Always／Never 正常控制指示。
- 共用滚动控制器把竖条放在逻辑 End，双轴角落在同一侧让位；成功提交新方向后取消旧拖条。方向变化保留物理 offset 与实例，横向键盘按逻辑起终点处理，wheel／拖条／reveal 仍是物理坐标。Gallery 覆盖三种轴、嵌套残量、条可见性、宽度、禁用和焦点显露。
- macOS aarch64 release：6 项布局／几何测试与 3 项真实窗口测试全部通过，包含明暗 shadcn 条几何、圆角与焦点环像素、指针／键盘指示、RTL 换侧与拖动取消、State 替换、禁用、零尺寸恢复、嵌套残量、显露、停帧和生命周期；既有 Direction／VirtualList 回归同时验证 Portal／文本方向、阅读位置、稳定 key 与实例保留。生产控件库、Gallery 构建及相关 Cone 格式／tests 类型检查通过。
- 首轮窗口用例的零尺寸恢复失败定位为夹具只在 measure 读取尺寸开关，已改为显式 Props 并复验通过。手工复查 Light／System、Dark／System、Dark／Neutral 与 RTL；修正 Gallery 外层裁剪焦点环的留白，以及窄宽配置受父级下限约束的问题。
- 本批完成 Direction、ScrollArea，目录累计 **29／64**；P3 后续继续 Collapsible、Accordion、Tabs、Breadcrumb、Pagination。未重建编译器或 LLVM，未重复全仓／全部控件测试；一次性窗口、三套本轮 native 测试输出和独立锁清理，生产／Gallery／控件测试产物、共享缓存与下载源保留。7 个 UI 库源码仍无反射描述及 requireClassInstance 直接引用。x86_64 留在整体收尾，Windows／Linux 未验证。

## P3b：Collapsible 与 Accordion

- 先补齐 UI／控件规范。Collapsible 使用受控 Boolean、单一 Button／Press 触发器、Lucide 箭头和 Presence 高度过渡；支持 Dispose／KeepMounted、退出中重开、独立程序入口、嵌套、Reduced 热切换与最新绑定／回调。Accordion 复用同一实现，提供 Single／Multiple、禁止全关、Default／Card、稳定 Choice key、普通 Tab 与只移动焦点的方向键导航。
- KeepMounted 初始关闭仍挂载正文，关闭完成不测量、不放置，继续接收 Props／State 更新。新增 MeasureContext.skipChild 显式表达该布局选择，未测量且未跳过仍报错；同轮跳过后不能再测量或放置。ComponentScope.isFrameEnabled 复用现有 AnimationGroup，关闭内容立即停止其帧请求，退出 Presence 留在外层；输入和 IME 同时撤销，重新打开只在 Accepted 非零几何后恢复输入。
- macOS aarch64 release：格式与相关 tests 静态检查、7 个生产 UI 库和 Gallery 构建通过；**7 项定向测试全部通过**。4 项纯逻辑测试覆盖选择、拒绝 setter、外部值保留及配置错误；35 步原生窗口测试覆盖工厂次数、局部状态、关闭 Props 更新、IME、退出重开、换绑、Reduced、零尺寸、窗口隐藏、keyed 编辑重排、禁用和停帧，并检查 skipChild 的错误边界。既有 Presence／交互所有权回归及包含 21 个场景的 UI 错误清理回归通过。
- 首轮实窗发现普通 Element 的“所有子项须测量”与关闭 KeepMounted 的合同冲突，已先修订规范并增加显式跳过操作，再完整复验该窗口。基础库更新期间一次检查遇到并行发布的依赖产物变化，待发布结束后重新检查通过。未放宽生命周期、输入或停帧断言。
- Gallery 新增两页，实查 Dispose／KeepMounted、加载内容、嵌套、独立触发、分隔线／卡片、键盘焦点环、System／Neutral、明暗和 RTL。API 与 README 同步。本批完成两项，累计 **31／64**，接下来继续 Tabs、Breadcrumb、Pagination；没有重建编译器／LLVM或扩大到全仓／全部控件测试。一次性窗口、三套本轮 native 输出与独立检查锁清理，共享缓存、生产库、Gallery 和下载源保留；Windows／Linux 未验证，x86_64 留在整体收尾。

## P3c：Tabs

- 先补齐控件规范 §25，再实现受控 Tabs、真实成员 DSL、Default／Line、横纵／RTL、Auto／Manual、禁用与逐项内容／焦点／样式。复用 KeyNavigation、Press、ActionInk 与 DisclosureBody；固定列表和面板父级，不增加反射身份、选择镜像或生命周期体系。KeepMounted 的非活动正文继续逻辑更新，跳过布局、输入／IME 和帧请求；Dispose 在切换时卸载。
- macOS aarch64 release：格式、相关 tests 静态检查、生产库及 Gallery 构建通过；4 项 Tabs 模型／样式测试、4 项既有 ActionVariantTest 和 1 项 36 阶段原生窗口用例全部通过。覆盖拒绝 setter／换绑／实际值通知、None／未知／禁用外部值、键盘与真实中文 IME、keyed 重排、轴／变体／主题切换、工厂次数和局部状态、隐藏正文更新与停帧、窄宽／零尺寸、窗口隐藏恢复和卸载。末次字体继承及禁用选中条调整经格式、类型检查、生产构建与 Gallery 复查。
- Gallery 新增草稿表单、局部计数／挂载次数、自定义图标和加载内容；实查两种变体、Manual 焦点与选择分离、重排保留、禁用、明暗 System／Neutral 和 RTL。API 与 README 同步，累计 **32／64**；继续 Breadcrumb／Pagination。未重建编译器／LLVM，未重复全仓或全部控件测试；一次性窗口、Tabs 原生输出及独立锁清理，共享缓存与后续复用产物保留。七个 UI 库源码仍无反射描述及 requireClassInstance 直接依赖，Windows／Linux 未验证，x86_64 留在整体收尾。


## P3d：Breadcrumb 与 Pagination

- 先补齐控件规范 §26–27。Breadcrumb 提供链接／当前项／省略号／自定义槽、Chevron 或自定义分隔符、长项省略、换行与 RTL；复用普通 Button 和 Wrap，重排及首项改变保持正文槽。Pagination 提供受控 Long 页码、自动边界／邻近范围、显式页码、缺口省略、本地化、compact 与内容／焦点配置；空数据和外部越界值不暗写模型，setter 实际采纳后才通知，不取业务数据。
- macOS aarch64 release：格式、相关 tests 静态检查、生产库和 Gallery 构建通过；4 项模型／样式／DSL 测试和 1 项 20 阶段原生窗口用例全部通过。覆盖空页、单页、缺口合并、Long 上界、错误配置、拒绝 setter／换绑／实际值通知、动态页数、真实 Tab／Space／Enter、keyed 焦点与自定义局部状态保留、窄宽／RTL／主题切换、逻辑禁用、未放置时等待焦点请求的取消、恢复和静止停帧／卸载。用例时序按真实键盘分发修正，并遵守既有 FocusHandle 等待放置的合同。
- Gallery 增加可展开路径与实际页码驱动的数据视图，实查正常／禁用、空数据／外部页码、拒绝选择、局部回调、compact、本地化、明暗 System／Neutral 和 RTL；实窗发现的数字居中与窄宽示例约束已修正并复查。API、模型说明和 README 同步。P3 首次交付完成，累计 **34／64**；Breadcrumb 的 Dropdown 组合在 P5 接入。下一批进入 P4 弹层和焦点。未重建编译器／LLVM，未重复全仓或全部控件测试；一次性窗口、导航 native 输出与独立锁清理，共享缓存、生产库、Gallery 与下载源保留。Windows／Linux 未验证，x86_64 留在整体收尾。

## P4a：独立弹层焦点与 Popover

- 先修订 Surface／UI／控件规范。InputBoundary 增加 FocusOnly，焦点和键盘约束独立于指针屏障；Portal 以 OverlayMode 区分三种模式，OverlayFocusOptions 取代 ModalOptions，所有调用方同步迁移。复用原 focusRevision、FocusScope、Accepted 帧和活动登记，不增加第二套焦点状态或反射依赖。非模态关闭前焦点已离开本层时不抢回焦点，背景非模态层不能越过现有焦点域。
- 新增 Popover Props／DSL、普通 Button 触发器和外部 anchor、标题／描述、表单正文、可选焦点模式、关闭 veto 与实际绑定值通知。复用 Portal 定位、退出重开和嵌套生命周期；默认采用 Base Nova 的 288 宽、10 padding／spacing、8 圆角及 100ms 淡入缩放位移，支持 Reduced Motion、字体缩放及 System／Neutral 配色。
- macOS aarch64 release：格式、相关 tests 静态检查、生产库和 Gallery 构建通过；2 项 Portal 类型／DSL、2 项 Popover 元数据／错误、既有模态实窗和新增 32 阶段 Popover 实窗，共 **6 项定向测试通过**。覆盖三种模式、背景 pointer／wheel 与焦点隔离、拒绝绑定／veto、最新回调、Tab／Escape／Space、实例／局部状态及工厂次数、退出重开、新焦点优先、窄宽翻转、禁用恢复、嵌套显式／默认焦点和完整卸载。窄宽之后恢复窗口尺寸再测试横向嵌套，避免要求零宽目标获得焦点；未改变可见性合同。
- Gallery 实查表单编辑／保存、嵌套层、三种模式、Reduced Motion、明暗 System／Neutral 和 RTL；鼠标打开嵌套层默认聚焦首项可选中文本，⌘A 与 Tab 到关闭按钮验证正常。临时诊断已删除。本批首次交付累计 **35／64**；P4 后续完成 Tooltip／HoverCard、共享箭头装饰及 Popover 接入，再完成 Dialog／AlertDialog／Sheet。一次性窗口、检查锁与本轮定向测试输出清理，保留共享缓存、生产库、Gallery 和下载参考；没有重建编译器或 LLVM，也未运行全仓或全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。


## P4b：Tooltip、HoverCard 与共享箭头

- 先补齐控件规范 §29–30、Popover 箭头及 UI 的显式悬浮关闭规则。Tooltip／HoverCard 复用 Hover、Portal 和同一动画时钟；可选 Binding 直接读取当前模型，原触发器保留唯一 Press 与焦点，Tooltip 正文不可交互，HoverCard 正文支持按钮、选择文字和嵌套 Popover。Escape／触发激活关闭后，在 hover 与有效焦点都离开前抑制重开；非模态路由代理不再接受焦点，点击提示正文不夺走背景焦点。
- 增加共享 PopupArrow，随实际物理侧、anchor 中心、边缘约束及面板动画绘制；Popover 接入可选箭头。Tooltip 采用 12 字号、12／6 padding、自然宽度及最大宽度换行；系统半透明前景色与背景先合成不透明底色，保持可见色阶并遮住后方内容。Gallery 补齐延迟／禁用／受控提示、四向换行、Follow 及嵌套详情示例。
- macOS aarch64 release：格式、相关 tests 静态检查、生产库与 Gallery 构建通过；4 项组件测试、新增 37 阶段实窗、既有 32 阶段 Popover 和悬浮／受控退出回归，共 **7 项定向测试通过**。覆盖延迟取消与通路、焦点／触发激活、关闭抑制、拒绝绑定／换绑、最新回调、短文本收缩与长文本换行、实例／局部状态保留、RTL／主题切换、Reduced Motion、禁用／卸载和静止停帧。实窗发现并修正捕获事件订阅入口、短提示被撑宽及系统色透底问题。
- Gallery 实查箭头四向、系统明暗与 Neutral 深色、RTL、长文换行、自定义 Kbd、原动作次数、HoverCard 正文动作、嵌套详情及逐层 Escape。累计 **37／64**，P4 后续为 Dialog／AlertDialog／Sheet；七个 UI 库源码仍无反射描述直接依赖。清理本轮独立锁、窗口与 native 输出，保留共享缓存和可复用产物；未重建编译器或 LLVM，未运行全仓／全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。

## P4c：Dialog／AlertDialog／Sheet

- 先补齐组件规范 §31–32，新增三个受控模态组件与 DSL。复用 Portal、Presence、焦点域和滚动控制；Dialog 提供标题、正文、固定页脚及关闭槽，DialogFooter 按实际宽度横排或反向纵排。AlertDialog 默认聚焦取消，确认动作及等待状态由应用决定，不自动关闭；Sheet 支持四边、逻辑 Start／End、明确尺寸与自然尺寸。
- 正文滚动模式、主题、方向、尺寸及退出中重开保留同类型实例和局部状态；关闭请求经过当前 Binding 与 veto，背景点击不穿透。遮罩与面板分别淡入，Dialog 缩放、Sheet 位移，Reduced Motion 关闭动画。Gallery 增加中文可编辑表单、嵌套详情、保存结果、关闭拦截恢复、模拟待完成操作和四边滚动设置面板。
- macOS aarch64 release：相关 Cone 先格式化与 tests 静态检查，生产库与 Gallery 构建通过；3 项组件测试与 44 阶段实窗测试，共 **4 项定向测试通过**。覆盖受控拒绝／换绑、回调、默认取消及明确初始焦点、嵌套恢复、外部点击、退出重开、工厂次数／局部状态、窄宽页脚、四边／RTL、滚动／固定页脚、零宽／禁用及卸载释放。零宽测试按既有 Surface 合同区分裁剪后保留已有焦点、清除后等待有效几何再聚焦，没有修改生产焦点语义。
- Gallery 实查系统明暗与 Neutral 深色、RTL、四边 Sheet、滚动正文和固定动作、中文编辑保存、嵌套逐层 Escape、关闭拦截恢复、AlertDialog 两种尺寸与默认取消、确认后等待及完成操作、Full／Reduced Motion。累计 **40／64**，P4 完成，下一批为 P5 菜单与选择；七个 UI 库仍无反射描述直接依赖。清理本轮窗口、native 输出与独立锁，保留共享缓存和可复用产物；未重建编译器或 LLVM，未运行全仓／全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。


## P4 反馈修复：模态内容的焦点环裁剪

- 修订组件规范 §31，移除页脚动作排列和 AlertDialog 标题排列的多余矩形裁剪。正文已有的 16 内边距归入滚动内容，视口包含这段留白；控件自身的测量尺寸、位置与焦点环 outset 算法保持一致，环的边框宽度不重复计入外扩。
- macOS aarch64 release：组件及模态／Gallery tests 静态检查、生产库和 Gallery 构建通过。既有 44 阶段模态实窗回归通过，新增实际 Canvas 裁剪后完整焦点环范围的断言，覆盖普通正文、窄宽页脚、滚动及 Sheet；原有焦点恢复、四边／RTL、零尺寸与卸载继续通过。Gallery 实查 System 浅色 AlertDialog 取消按钮、Sheet 普通输入及 System 深色长滚动输入，确认四边完整且滚动条不再覆盖输入框外环。
- 本批是既有组件修复，目录仍为 **40／64**，后续继续 P5。清理本轮模态测试输出、日志和独立锁，保留共享构建缓存及继续使用的 Gallery。Windows／Linux 未验证，x86_64 留在整体收尾。

## P5a：DropdownMenu／ContextMenu／Menubar

- 先补齐组件规范 §33，交付共享 MenuContent／MenuEntry、类型化勾选／单选、菜单行和三个入口的真实成员 DSL。复用 KeyNavigation、Typeahead、Hover、ScrollControl 与 Portal，覆盖嵌套子菜单、右键／键盘菜单、横向菜单栏、当前 Binding／回调和关闭策略；普通点击、选择值、活动焦点和打开状态各有明确归属。
- UI 增加 ComponentScope.moveFocus，用于根菜单关闭并恢复入口后的 Tab 续行；OverlayFocusOptions 的 autoFocus=false 保留悬浮子菜单的关闭恢复，restoreTarget 使 Modal／FocusContained 菜单栏返回当前入口，沿用原有焦点会话和 revision。行缩进在测量时按逻辑方向解析，窄宽保留图标／快捷键的自然宽度并省略标题；修正同值高亮发布引起的持续刷新，loop=false 的菜单栏在首尾停留。
- macOS aarch64 release：格式、相关 tests 静态检查、生产库和 Gallery 构建通过。5 项 UI 配置／定位测试、5 项菜单模型／DSL 测试及 1 项 **72 阶段实窗测试**通过；实窗覆盖真实键鼠、两层子菜单、Tab 续行、关闭拒绝／重开、重排与实例保留、长列表显露、深色／RTL、三种焦点模式、打开及关闭后的停帧、禁用／零尺寸和卸载。原生测试等待真实系统焦点，执行时需要宿主应用在前台，不放宽焦点门槛。
- Gallery 实查浅色 System、深色 Neutral、窄宽与 RTL、图标／缩进／快捷键、菜单栏完整动画和当前入口恢复；补齐 Breadcrumb 省略路径、ButtonGroup split action、右键保留主动作及菜单进入 AlertDialog 的组合，取消按钮焦点环完整。累计 **43／64**，P5 后续为 Select、Combobox、Command 与 NavigationMenu。清理一次性菜单窗口测试输出、日志及独立检查锁，保留共享缓存、生产库、Gallery 和已下载参考；没有重复全仓或全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。


## P5b：Select

- 交付类型化 Select、分组与禁用项、Default／Sm、placeholder／invalid、受控开关、FieldTarget、自定义内容和 ButtonGroup 连接表面。公共 ContentAligned 消费 LayoutMetrics.overlayAnchor；选中行与 trigger 对齐，缺失选择回退侧边定位，长列表初始显露后滚动不移动面板。
- 候选复用菜单的真实 roving 焦点、typeahead、选择和关闭通路；支持拒绝 setter／veto、外部未知值、Tab 续行与动态候选。修复键盘导航后旧 hover 行残留高亮。退出中重开重新对齐当前选择；Portal 沿同一会话重新安排初始焦点，并继续尊重新的显式焦点意图。滚动按钮按需使用原生 deadline，离开／关闭／禁用／卸载停止。
- macOS aarch64 release：UI、五套样式库、组件生产库和 Gallery 构建通过；7 项定位、4 项 Select 状态／DSL／样式和 5 项菜单模型测试通过。Select 41 阶段、菜单 72 阶段、模态弹层 44 阶段实窗回归通过，覆盖重开聚焦、动态删除、鼠标／键盘、滚动停止、卸载与原有 focus ring 裁剪修复。前台恢复后已完成此前被系统焦点限制阻断的验收。
- Gallery 实查明暗 System／Neutral、RTL、窄宽、两种定位、长列表滚动、单行省略、自定义图标和连接处完整焦点环；修正深色 input 透明度的叠乘及局部 hover 覆盖，并定向复验。累计 **44／64**，后续继续 Combobox／Command／NavigationMenu。清理本轮实窗输出、日志与独立锁，保留共享缓存、生产库、Gallery 和下载参考；未重建编译器／LLVM，未运行全仓或全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。

## P5c：Command

- 完成 Command／CommandSource／CommandContent 与条目、分组、分隔、图标、平台快捷键和空结果 DSL；复用 cmdk 的 MIT 评分规则，采用 Unicode 标量索引和展开映射，支持自定义评分及直接消费外部结果。Props／已提交 query 不变时复用筛选结果；VirtualList 只创建可见候选。
- 输入使用唯一 TextEditorState，公开 committedText 供候选控件观察；活动 key 与编辑焦点分离，支持首项高亮、禁用跳过、重排保留、外部绑定及 Up／Down／Home／End／Enter。原生 IME 先解释按键，仍在组合时不导航、不提交候选，也不因普通编辑默认动作间接提交 preedit；外层取消事件得到尊重。
- macOS aarch64 release：5 项模型测试、41 阶段真实窗口测试通过，覆盖评分黄金值／Unicode、分组与非法数据、组合输入／取消／提交、筛选缓存、受控状态替换、5000 项虚拟化、零约束、RTL、禁用与卸载。重建受影响的 UI、五套 style、component 和 Gallery，静态检查通过。
- Gallery 实查明暗 System／Neutral、窄宽长空提示、RTL、Dialog 内中文搜索／执行／焦点恢复、10,000 项末项导航和执行。修复单行省略文字显式方向覆盖 RTL 的问题并复看，焦点环完整。累计 **45／64**。清理一次性测试输出、日志、独立测试构建和锁，保留共享缓存、生产库、Gallery 及下载参考；未运行全仓或全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。

## P5d：Combobox

- 完成类型化单／多选 Combobox、ChoiceSource／ChoiceContent 和分组 DSL；提供 Unicode 包含匹配、自定义过滤、直接消费外部结果及自定义候选。选择、已提交查询、活动 key 和开关分别受控，拒绝 setter 保留查询与弹窗，缺失领域值保留并允许自定义显示。候选与 Command 共用不抢焦点的 Press 行和 ActiveItems 路由。
- Inline 输入支持展开／清空、可换行 chips 及删除和方向键；Popup 模式在面板内使用同一 TextEditorState。关闭列表时仍正确保护 IME 候选键，Tab／Escape／veto 和关闭恢复复用 Portal。Bare 输入不再重复承担独立控件高度，多选单行外框为 32；候选及芯片使用逻辑内缩。Gallery 主滚动内容增加内部留白，修复贴视口边缘的焦点环裁剪。
- macOS aarch64 release：5 项模型测试、51 阶段真实窗口测试通过，覆盖 Unicode／分组、100 万项来源不枚举、选值拒绝与规范化、组合输入／中文提交、芯片删除和清空、5000 项虚拟化、缺失选值、受控替换、外层取消、窄宽／零尺寸、禁用及卸载。组件静态检查、生产库与 Gallery 构建通过；只重建实际变化的组件，复用现有编译器和基础库。
- Gallery 实查浅色 System、深色 Neutral、RTL／窄宽、invalid、完整焦点环、Popup 搜索和关闭、自定义值及 10,000 项末项导航／选值。累计 **46／64**，P5 剩余 NavigationMenu。清理本轮实窗输出、日志、独立测试构建和锁，保留共享缓存、生产库、Gallery 及下载参考；未运行全仓或全部控件测试。Windows／Linux 未验证，x86_64 留在整体收尾。
