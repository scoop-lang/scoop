# Scoop UI Gallery

真实原生窗口，使用同一套 `scoop.ui.component`，右上角 Style 下拉框可选择 shadcn、macOS、Material、Fluent 的完整主题。示例目录保留 `shadcn` 名称。导航分为 Applications 和 Components，包含 66 项组件与材质的独立页面，以及四个可操作的应用组合。

目录和详情共用一棵挂载树。页面通过 Flex 的 layoutFor／itemFor 读取父容器逻辑宽度，达到 720 时并排显示，较窄时按当前导航位置显示目录或详情。目录的每组按钮用最小列宽 152 的 FlexGrid 自动分列，详情的 Frame 将正文宽度限制在 960。窗口拖动、旋转和分屏只改变子项的测量与放置，显式返回目录结束当前详情页面。实现见 [Gallery 布局](src/gallery.scoop)。

**Liquid Glass** 页提供 Frosted／Clear／Liquid 三种共享材质。滚动彩色底图或点击 Scroll colors，可观察固定表面处理当帧下层；Separate／Connect 与 Stretch／Round 更新稳定 key 的实际位置、尺寸和圆角，Reverse order 保留两项独立计数。Opacity、禁用和错误状态可组合操作。八个用途开关通过 ThemeTokens.glassSurfaces 应用于整个 Gallery，输入草稿和弹层使用真实控件。页面中的高对比度、减少透明度和文字缩放修改当前 UiWindow.preferences，System preferences 恢复跟随系统；Motion 与顶部设置共用主题策略。实体回退保留内容和操作。实现见 [材质与策略模型](src/glass/model.scoop)、[页面](src/glass/page.scoop)和[背景与融合](src/glass/preview.scoop)。桌面入口与 UIKit 的 GalleryScene.bind 绑定借用的 UI 根，不修改系统设置。

- **Settings app**：资料与偏好表单，组合中文输入、邮件与验证码校验、生日选择、勾选／开关／单选和存储滑块；固定操作区提交时会显露并聚焦错误字段。
- **Workspace app**：侧栏、文档／预览页签和可调面板，配合 File 菜单、命令搜索、重命名 Dialog、嵌套 Sheet 和活动 Drawer；窗口变窄后使用移动侧栏，草稿随应用模型保留。
- **Data app**：本地样例付款的筛选、排序、分页、选择和行内备注；行菜单的删除确认同时更新表格与图表，通知中的 Undo 恢复原记录及备注。
- **Conversation app**：消息、附件、可编辑的嵌套 Trace、异步回复与图片，以及带条件分支的问卷；支持插入历史、保存／恢复阅读位置和答案、错误回跳与重复提交防护，切页取消任务。

Bubble、Message、Marker、Attachment、Trace 页面提供窄宽、逻辑对齐、独立动作和真实异步更新。Bubble 展示全部变体及上下反应区；Message 可增删头像／页脚并保留草稿。Attachment 包含五种状态、三种尺寸、横纵媒体及可滚动图片组。Trace 支持嵌套编辑、受控／拒绝展开、链接动作及 Dialog／Sheet；页面和 Sheet 分别持有编辑器状态。切页取消任务，明暗、Style、RTL 与减少动态沿顶部设置。

Message Scroller 页面使用稳定消息 key，提供新回合、真实异步流式回复、历史插入、延迟图片、保存／恢复阅读位置和清空恢复。Auto follow 显式启用末端跟随；拖选正文、输入备注或向上滚动后保持阅读位置。顶部主题／RTL 与 Narrow 可组合检查；切页取消示例任务并释放图片。

Table 展示有限发票、共享列宽、footer 和独立动作。Data Table 展示付款列表的排序、邮件筛选、本页选择、列显隐／拖宽、行内备注与菜单、分页、虚拟滚动和调用方提供的结果页；备注保存于稳定行模型。可切换窄宽、主题及 RTL，或加载更多行检查实际交互。

Tree 页面使用含 180 个组件文件的目录数据，展示同一模型的单列／多列、祖先整行吸顶、级联勾选、目录多选、过滤／高亮、连接线、固定／可变行高及窄宽横向滚动。Jump to file 60 显示三层祖先；Size 表头排序、列边缘调宽、Details column 显隐和 Inspect 行内操作可组合使用，F2 进入行内操作、Escape 返回行。Collapse branches 后可把 notes.scoop 拖入 archive，也可悬停 src 展开或沿正文边缘滚动；排序时仅保留 Inside 移动。Load Remote 的首次异步请求故意失败，点击原父行的重试箭头后成功；切页取消尚未完成的加载，已加载数据和选择保留。Reset tree 恢复演示数据；主题、RTL 与减少动态沿 Gallery 顶部设置。示例只修改内存数据，不访问真实文件系统。

Tooltip 页展示原按钮动作、延迟、禁用按钮提示、四向箭头、长文换行及可拒绝的受控绑定；Hover Card 页展示可选文字、关注操作和嵌套 Popover。Popover 页可切换箭头，三页沿 Gallery 的 Style、明暗、RTL 与 Reduced Motion 配置。

组件页面包括普通、invalid、disabled、真实焦点状态和各公开变体。组件及组合应用的实际验证范围见 [实施记录](../../../../../docs/ui/shadcn/PROGRESS.md)，Tree 的定向验证与系统拖放限制见 [Tree 实施记录](../../../../../docs/ui/tree/PROGRESS.md)。

在准备好开发 sysroot 后运行：

```sh
SCOOP_SYSROOT="$PWD/sysroot" target/release/scoop run sysroot/src/scoop.ui.component/examples/shadcn --release
```

### 在 iPhone 和 iPad 上运行

iOS 使用同一份页面、模型和四套主题。可用宽度小于 720 点时先显示可搜索的目录；点条目后全宽展示内容，点 **Examples** 返回之前的滚动位置。**Appearance** 可切换样式、明暗、文字方向和动画。宽屏同时显示目录和示例。Settings、Workspace、Data、Conversation 都可直接从 Applications 打开。

在装有 Xcode、arm64 模拟器、GN 和仓库 LLVM 工具链的 Mac 上，从仓库根目录执行以下命令。将 `SCOOP_LLD` 指向该工具链的 `bin/lld`。本地基础库、图形库和原生依赖放入开发 sysroot。已有对应库时可直接构建示例。[开发工具链准备](../../../../../scripts/README.md)

```sh
demo=sysroot/src/scoop.ui.component/examples/shadcn/ios
python3 scripts/prepare_ios_ui.py --target aarch64-apple-ios-sim \
  --build-dir "$HOME/repos/scoop-dep/build/ios-ui"
python3 "$demo/build_framework.py" --sdk simulator --linker "$SCOOP_LLD"
xcodebuild -project "$demo/Gallery.xcodeproj" -scheme ScoopGallery \
  -destination 'generic/platform=iOS Simulator' -derivedDataPath "$demo/target/xcode" build
xcrun simctl list devices booted
xcrun simctl install "$SIMULATOR_UDID" "$demo/target/xcode/Build/Products/Release-iphonesimulator/ScoopGallery.app"
xcrun simctl launch "$SIMULATOR_UDID" dev.scoop.gallery
```

`SIMULATOR_UDID` 取自已启动设备的列表。也可在生成框架后打开 `ios/Gallery.xcodeproj`，选择 ScoopGallery 和目标模拟器运行。Window 场景连接时创建主题、根模型和字体，断开时释放它们并取消子任务。按 Home 返回后保留当前页面、草稿和阅读位置。[UIKit 场景入口](ios/src/app.scoop) · [共用页面](src/gallery.scoop)

真机开发运行还需用相同的 `prepare_ios_ui.py` 命令准备 `aarch64-apple-ios`，再运行 `python3 "$demo/build_framework.py" --sdk both --linker "$SCOOP_LLD"`。在 Xcode 的 Signing & Capabilities 选择自己的开发团队和可用的 Bundle ID，然后选中已配对的 iPhone 或 iPad 运行。两个切片的最低系统版本为 iOS 17。

如需复验本地拆包，在准备命令后加 `--package-dir tmp/ios-ui/packages`。已有库时可加 `--package-only`。基础包包含 runtime、异步定时和文字后端，GUI 包包含 Skia、窗口、组件及四套样式。模拟器和设备分别制包。

本期在 iPhone 模拟器逐页打开并返回了全部 69 页，用系统拼音填写并保存了 Settings，也用 VoiceOver 朗读目录并双击进入页面。iPad 模拟器验证了 Workspace 草稿、付款删除和撤销、Conversation 流式回复和离页取消，以及与「照片」并排时的窄窗导航。iPhone 13 Pro Max 上用同一工程的开发签名版验证了触摸导航、后台恢复、系统拼音保存，以及旁白的朗读顺序、双击、三指滚动和编辑。[分设备验收记录](../../../../../docs/ui/IOS-PORT.md#本期交付记录)

Gallery 的 macOS 入口在附着窗口前调用 `configureFpsReportingFromEnvironment()`。在同一命令前加 `SCOOP_SURFACE_FPS=1` 即可查看实际呈现帧率。Button 页可供静止对照，Spinner 或 Skeleton 页可测连续动画。输出字段见 [Surface README](../../../scoop.surface/README.md)。

顶部切换明暗、LTR／RTL 和 Reduce motion／Full motion，右上角 Style 下拉框选择整套视觉风格；导航支持搜索应用和控件。可以点击、Tab／Space 操作、输入中文，或通过“Focus the normal control”观察程序聚焦。Style 支持方向键、Enter 和文字搜索，切换后保留当前页面、文字、业务状态、方向、明暗及动态偏好；组件继续通过同一 ThemeSource 更新。iOS 的已验收操作见上节。

Style 始终提供 `shadcn`、`macOS`、`Material` 和 `Fluent`。其左侧的 `Colors: Preset／System` 独立选择配色来源：Preset 使用当前风格的预设配色，System 把系统强调色、背景、文字及分隔线应用于任意风格，同时保留该风格的字体、尺寸、轮廓及动效。切换 Style 保留配色来源，切换 Colors 保留 Style。macOS 默认 shadcn + System；只有启用 System 时，切换明暗或重新聚焦窗口才读取系统颜色。其他平台固定 Preset，系统取色适配只在 macOS 提供。

窗口测试遵守每进程一个 Application 的合同，须串行、按单个用例启动；`--list` 可列出名称，再以 `scoop test sysroot/src/scoop.ui.component/examples/shadcn --release --filter 'real gallery retains'` 等命令分别运行。普通模型测试可按容器筛选，例如 `--filter GalleryStyleTest` 验证样式切换和状态保持。

Liquid Glass 的定向验证可使用同一命令加 `--filter Glass`，覆盖两项模型测试和一个 35 阶段 macOS 原生窗口测试，包括滚动、材质与用途、形变与状态、AX 编辑和弹层、偏好、窄窗及停帧。本轮结果和平台限制见[玻璃材质验收记录](../../../../../docs/ui/liquid-glass/DESIGN.md#5-gallery-与组合验证)。

Badge 页面可以切换禁用、撤销动作和替换回调。Spinner 页面中的滚动面板用于观察离屏停帧与滚回恢复；切页、隐藏窗口、停止和减少动态均不会留下持续渲染。自定义 Button／Badge 内容通过 ActionContentStyle 消费解析后的文字、图标尺寸、颜色与间距。

Card、Alert、Empty 页面可切换可选区域和宽窄布局；Card 同时切换 Default／Sm 及头部分隔线，Empty 切换自然媒体／图标底板。正文按钮和输入在这些变化中保留状态。Aspect Ratio 使用示例自行绘制、随挂载释放的同一图像，展示 16:9／1:1 与 Cover／Contain 的组合。

Avatar 页面提供三种尺寸、角标、分组和计数，切换两张本地绘制的头像及空／加载／失败状态，并与 Empty 组合。Skeleton 可显示或隐藏带独立计数和活动图标的 Card，切换后原状态保留。Progress 提供确定值、未知进度、自定义范围及数值文字；两页均可滚动检查动画离屏与恢复，并切换减少动态。

Typography 页面包含所有文字角色、可切换宽度的列表／表格、中英混排与行内 code；选择首段后使用平台 Select All／Copy 可复制完整文字，open help 是独立行内动作。Kbd 页面展示原生单键帽／分组、方向键以及 Button 组合；聚焦 Increment 按钮后使用显示的快捷键会执行同一个计数动作，离开页面后解除绑定。

Label 页面把名称、备注和订阅标签关联到真实 Input／Textarea／Checkbox；标签点击保留输入选区，Checkbox 与 Space 使用同一个选择动作。可以切换整组禁用或加载状态，再恢复原表单内容。

Button Group 页面提供横纵按钮组、文字前缀加 Input、split action、Sm／IconSm 和嵌套组。切换明暗／RTL 后接缝跟随逻辑方向；点击与键盘操作保持原按钮动作，地址输入使用同一编辑模型。

Item 页面展示三种变体与尺寸、头像／图片／图标、可选头尾和分隔线。切换宽度时动作换行，切换详情保留可选择正文；整行动作和内部按钮分别计数。可以切换展示模式或整行禁用，检查明暗／RTL 下的排列与键盘行为。


Checkbox 页面新增三态全选组合：部分选中显示半选标记（Fluent 为方块，其他预设为横线），全选再清空使用同一组 Boolean 子项。Switch 页面包含两种尺寸及普通／invalid／disabled 状态，可点击关联标签、使用 Space，并切换减少动态。Toggle 页面展示两种变体、三种尺寸、图标／文字、自定义内容及 Preview 组合；按住与选中分别呈现，Space／Enter 读写同一绑定。

Input OTP 页面展示六位分组、四位字母数字、Unicode grapheme 自定义策略、禁用、只读与单槽错误。可切换窄宽和禁用、通过标签聚焦，并观察正文与完成通知计数；粘贴数字中的空白和连字符被移除，一次 Undo 撤销整个粘贴。输入法 preedit 保持原文，最终提交才过滤并截断。

Radio Group 页面包含空选／已选、禁用项、可切换方向／错误状态、外部选择、关联标签和自定义 plan 卡片。Toggle Group 页面包含可清空单选、多选、三种尺寸、连边／间距、横纵排列、重排及自定义图标正文；方向键只移动焦点，Space／Enter 激活。两页沿顶部切换 Style、明暗和 RTL。

Slider 页面提供单个、两个和三个 thumb、连续范围、竖向和禁用示例。可点击轨道、拖动 thumb、通过标签聚焦，或使用方向键、Home／End、PageUp／PageDown；minimumGap 保持相邻值的间距，Escape 取消拖动。页面可以切换反向、窄宽、invalid、禁用及减少动态，并分别统计提交与取消；配色、明暗和 RTL 继续沿用顶部设置。

Input 页面补充密码显隐、最多六位数字的编辑策略、只读复制和应用文件选择动作；文件示例明确选用样例文件，实际应用连接自己的选择器。Textarea 页面展示多行编辑、换行切换、水平／垂直滚动、Tab 插入／导航、⌘Enter 提交及只读／错误／禁用状态。

Input Group 页面展示四种 addon 位置、文字前后缀、Kbd、Spinner、独立按钮、多行消息和显式自定义输入。可以移动 keyed addon、收窄宽度及切换错误／禁用／只读；点击 addon 文字聚焦编辑器，按钮各自激活。顶部配色、明暗和 RTL 设置同样适用。

Field 页面提供带校验的设置表单，组合 FieldSet／FieldGroup、Input Group、六位 OTP、Textarea、Checkbox choice card、Switch、Radio Group、Slider 和 DatePicker。Save settings 首次显示业务错误并聚焦第一个无效输入，后续编辑更新错误，成功仅在示例内计数；Reset 恢复业务值。Layout 切换横／纵／响应式，Width 切换可用宽度；可同时检查 Style、明暗、RTL、只读和禁用。

Direction 页面组合继承方向的文本、编号标签、方向键选择组和逻辑 Start 边框，以及局部 LTR 代码输入。顶部热切换保留文字、选区与点击计数。Scroll Area 页面提供垂直、水平、双轴及嵌套视口，可切换条可见性、宽度、禁用与内层残量策略；Focus viewport 和 Reveal last field 分别演示键盘翻页和自动显露，圆角视口与主题焦点环共用绘制／命中几何。

Collapsible 页面比较 Dispose／KeepMounted 的局部草稿、计数与挂载次数，并提供嵌套内容、Spinner 和独立程序触发器。Accordion 页面提供单选／多选、禁止全关、Default／Card、禁用项及 keyed 重排；可在正文编辑时切换顺序，并用 Tab、方向键及 Space／Enter 检查导航与展开。顶部的主题、RTL 和减少动态设置同样生效。

Tabs 页面包含 Default 草稿表单和 Line 自定义图标／加载正文。可切换 Auto／Manual、横纵、Dispose／KeepMounted、禁用与顺序，清空外部选择，并观察局部计数和挂载次数。方向键导航、Space／Enter 选择和 Tab 进入正文沿同一焦点体系；主题、RTL、减少动态和布局变化保留已挂载草稿。

Breadcrumb 页面提供长路径、Chevron／Slash 分隔符、窄宽换行、禁用、展开中间路径和非交互省略号。Pagination 页面用页码驱动三条记录，比较自动范围与显式页码／本地化文字，并提供空数据、单页、外部越界值、拒绝 setter、禁用和 compact 切换；两页沿顶部切换主题、配色及 RTL。

Popover 页面在弹层中编辑和保存尺寸，包含嵌套弹层。可切换 NonModal／FocusContained／Modal、方向与对齐、关闭 veto、拒绝绑定及禁用；外部按钮计数用于观察继续／消费策略，关闭完成与绑定变化分别计数。顶部明暗、Style、RTL 与减少动态继续适用。

Dialog 页面编辑真实 profile 草稿，带固定操作区、滚动长正文和嵌套详情，可隐藏关闭按钮、拒绝绑定或 veto 关闭。Alert Dialog 展示默认／小尺寸和媒体、默认取消焦点，以及确认后保持打开的模拟请求；Sheet 展示四边、可调主轴尺寸、RTL、滚动表单和固定操作区。顶部 Style、明暗及减少动态继续适用。

Dropdown Menu、Context Menu、Menubar 页面共用工作区命令、嵌套团队菜单、勾选和布局单选，提供禁用、模态切换、窄宽、拒绝绑定及关闭 veto。Dropdown Menu 同时展示 Breadcrumb 省略路径和 ButtonGroup split action，删除动作进入 AlertDialog；Context Menu 可在原按钮上右键或聚焦后按 Shift+F10，主动作计数保持独立。Menubar 支持左右／hover 切换、上下进入候选、嵌套方向导航和 Tab 退出续行。

Select 页面包含分组单选、placeholder／未知值、禁用／校验、Default／Sm、选中行对齐与普通 popup、80 项长列表、hover 滚动、自定义候选／显示正文、Label 关联和 ButtonGroup 连接动作；宽度、方向、明暗和模态配置可直接切换。

Carousel 页面提供横纵、单项／多项、循环、窄宽和自动播放。拖动正文或使用导航按钮切换，Tab 聚焦视口后可用主轴方向键、Home／End 和 PageUp／PageDown；第一项的草稿和其他项目的保存计数持续保留。自动播放在整组悬浮、焦点进入、拖动和窗口隐藏时暂停，离开后重新等待完整间隔；顶部明暗、Style、RTL 与减少动态配置同样适用。

Drawer 页面提供 Bottom／Top／Start／End、垂直吸附、只拖把手、非模态和关闭 veto。正文包含草稿输入、长内容滚动和嵌套 Drawer；可以拖动、通过可聚焦把手的主轴方向键／Home／End 切换高度，或按 Escape 取消拖动。默认 450／400ms 进入退出、嵌套缩放与顶部 Reduced Motion 设置联动；明暗、Style 与 RTL 同样适用。


Sidebar 页面提供 Sidebar／Floating／Inset、Offcanvas／Icon／None、逻辑 Start／End、可点击 rail 和 Command+B。菜单包含独立项目动作、badge、可折叠子菜单、账号入口及 Skeleton；主区草稿使用外部编辑模型，宽窄布局切换后保留。窄布局使用 Drawer，可切换关闭拦截；明暗、Style、RTL 与减少动态沿顶部配置。

Toast 页面提供六种类型、六位置、展开／堆叠、可见数量、队列顺序和窄宽选择；可以一次排入五条、执行 Undo、关闭最新通知或暂时禁用交互。Save workspace／Simulate failure 运行应用作用域内的真实异步 Job，两秒后更新同一条 Loading 通知；提前关闭后，任务完成只更新页面结果，不复活通知。Gallery 使用既有 window.asyncio 宿主，正文输入、明暗、Style、RTL 和减少动态继续生效。


Calendar 页面展示单选、多选与范围、单／双月、窄宽、月／年下拉、周号、outside days、中文 locale、禁用日期和四周规划周期的自定义 CalendarPageSource。参考日期由示例模型明确提供；页面底部显示真实选择值。焦点与选择分别响应键盘操作，范围只有第二个端点接受后才提交。

Date Picker 页面展示普通日期、生日、双月范围与 7／30 日预设、严格 ISO 草稿、显式自然语言 parser，以及独立时间输入组成的 LocalDateTime。可用 Enter、移开焦点、Reset 和外部日期更新观察草稿处理；解析错误保留原文，组合输入优先。设置表单也加入生日 Field，继承禁用、readonly、方向和主题配置。


Questionnaire 页面提供中英文、单选／多选／文字、开发角色的条件题、明确 Skip、错误回跳、保存／恢复／重置、空问卷、禁用与真实异步提交。独立 Dialog 问卷在关闭后保留草稿，返回时继续填写；窄宽、明暗、Style 和 RTL 可直接切换。
