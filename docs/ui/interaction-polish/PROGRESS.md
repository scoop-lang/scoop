# UI 交互打磨与 accessibility 实施记录

更新：2026-11-16。工作分支：`codex/ui-interaction-accessibility`。范围与完成条件以 [PLAN.md](PLAN.md) 为准。

## 执行约定

按本轮要求先在 macOS 完成本地功能与验证，再集中进行 Linux／Windows 验证；不逐个小改动远程重跑。每个可交付变更提交并推送后，获取并合并 `origin/main`。意外发现的编译器／底层依赖缺口使用独立修复分支、PR 与主线合并，更新工具链后回到本分支。

下载使用本轮指定代理，本地依赖复用 `~/repos/scoop-dep`。Linux 后续使用 `ssh nuc12`、`~/repos/` 下独立目录及 `~/.scoop`；Windows 后续使用 `/usr/bin/ssh 192.168.2.67`、E: 独立目录及 `~/.scoop`。原始日志留仓库外，必要缓存保留，临时核查文件在不再需要时删除。

## 当前状态

| 批次 | 状态 | 已完成与剩余 |
| --- | --- | --- |
| S0 | 进行中 | 已核对本地工具链、AppKit 偏好与通知、NSTextView 默认菜单模板；AppKit 基础语义查询／动作原型通过，外部 AX 窗口枚举异常待桌面恢复后复验；系统文本服务、拖放与其他平台验证待继续 |
| S1 | 进行中 | Window 偏好快照／观察 API 与三个 native 后端已实现，macOS 定向测试通过；UI 根覆盖／通知、caret 与主题消费已接入；四套主题的离屏测试通过，新增实窗热切换因本机锁屏待验；Windows／GTK 点击计数已修正，算法测试通过，平台输入及其余输入细节待验 |
| S2 | 进行中 | Window 语义节点、发布／退休和 AppKit 基础投影已接入；UI scope／场景／控件、关系／内容合并、精确命中和外部客户端仍待完成 |
| S3 | 进行中 | DeleteSelection 角色／命令、三种文本菜单模式、宿主与模型绑定、选区策略、键盘入口和各输入控件透传已实现；AppKit 菜单与 UI 定向测试通过，真实编辑菜单因锁屏待验；受保护用途、多击组合、系统文本服务和其他平台仍待继续 |
| S4 | 进行中 | 共享滚动条行为、配置入口、命中、轨道跳转及重复操作已接入；统一 Gutter／Overlay 与边缘显露已接入，本批 43 项定向测试及四个原生场景的源码检查／构建通过；滚动语义和实窗／平台验收待继续 |
| S5～S11 | 待开始 | 保持统一计划的完整范围，未将基础 API 交付视为上层交互或 provider 已完成 |

## 已交付变更

### 系统交互偏好的 Window 接口

- `InteractionPreferences` 区分未知值与明确关闭，覆盖滚动条、运动、对比度、透明度、文字缩放、caret 和双击间隔；保留物理宽度、宽高比和时间单位的差异。
- ActiveApplication／Window 读取 owner 已发布的快照，观察者经普通队列通知；先发布后通知，去重，回调内登记／注销、窗口关闭与应用退出按实际生命周期处理。
- AppKit 使用公开查询与通知；GTK 使用 4.18 基线设置和 libadwaita；Windows 使用 Win32 设置与按接口查询的 UISettings 2／4／5。Windows／Linux 的代码尚待目标环境构建与运行，不据 macOS 结果声明通过。
- 同步 Window spec、公开注释、库 README 与编程材料导航。定向 Cone 的原生通知用当前进程内替身驱动，不修改全局系统设置，也不冒充人工设置热切换验证。

本地验证使用 `/Volumes/Data/home/chenxu/repos/scoop-install` 中配套的 `scoop` 与 sysroot（工具 revision `6bd92d3d7a14e4614320008b33586df8da178d85`），测试显式通过 path 依赖编译当前 Window 源码。仓库原有开发 sysroot 与该工具的 runtime ABI 不同，没有混用其预编译库；当前主线相对该工具未发生语言／runtime 合同修改。改动源码先通过 Scoop fmt／check 与 native 格式检查。

`window-preferences` 的两个测试已通过：不可用值／显式关闭的区别，以及 AppKit 通知、快照发布、去重、观察者增删、owner 检查和关闭。Window 生产／测试语义检查和既有 `window-deadlines` 生命周期回归也通过。真实系统设置的人工热切换、设备输入及其他平台仍按 S0／S11 的矩阵验收。

### UI 根继承与文本光标

- `UiPreferences`／receiver builder、UiHost／单窗入口、UiWindow 动态覆盖和 ComponentScope 快照／观察已接入；None 跟随系统，显式 false／零保留原意。不同根独立解析，已覆盖的系统变化不请求帧；通知支持重入、取消和根错误收尾。
- render／update 的显示依赖及 measure／scene／draw 的版本记录沿已有缓存失效路径处理；偏好读取不增加持续帧。组件 update 仍只登记明确的默认字体和交互偏好，不扩展为任意 State 跟踪。
- TextEditHost 默认继承闪烁和宽度，Input／Textarea／OTP 及其组合入口、示例与测试同步改为可选覆盖。系统物理宽度按窗口 scale 换算，宽高比使用排版光标高度；局部逻辑宽度优先。每份测量与 Accepted 帧保存采用的光标样式，旧帧查询不会读到下一帧的宽度。
- 根关闭撤销应用级偏好观察；文本宿主还观察实际 DPI 变化。偏好更新保留编辑模型、选择和历史，闪烁继续使用既有 WindowDeadline。

开发资源使用配套安装与仓库外 `scoop-ui-polish-work/cones` 的普通 `--cone-path`：定向构建 Window、Surface、UI、Style 与四套主题，复用其余安装产物和共享 cache；没有复制整套 sysroot 或修改安装目录。主题在此批次仅按 UI 接口依赖重建，减少动画／高对比／减少透明度／文字缩放的主题消费仍待下一批实现。

本批 macOS 验证：UI 生产／测试和 Component 测试代码的语义检查通过；46 个变更的 Scoop 文件格式检查通过。UiPreferences 两个值测试通过；独立 UI 偏好窗口测试通过，覆盖四种显示读取路径、实际 AppKit 通知、两根覆盖隔离、重入／注销、物理宽度与比例宽度、Accepted 查询、模型／选择／历史保留、闪烁启停／停帧和回调失败退休。既有 `ui-text-editing`（含真实系统 IME、投影、选择及滚动）与 `ui-layout-results`（布局 A→B→A、保留测量和 lazy 选择）定向回归通过。没有运行无关的全仓测试或远程矩阵；本批仍未声明完整 accessibility provider 或主题适配完成。

已定向生成 UI／Component API 文档，并用 `scoop api search/show` 核对 UiPreferences、UiHost.attach、UiWindow.preferences 的访问器，以及 caretBlink／caretWidth 的可选类型、默认值与导入路径。生成站点在核对后清理，保留后续构建需要的 Cone 产物与 cache。

### 主题消费根交互偏好

- 新增 ThemePreferences、Theme.withPreferences、ThemeSource.resolve／read(scope)。来源的基础 State 与根解析结果分开，重复解析不叠加字号；内置控件与自绘示例统一使用所属 scope，第三方主题实现与测试包装同步新入口。
- 四套 ShapeTheme 预设同时更新 tokens 和实际表面、复选框、单选框、滚动条的部件数据。高对比度调整文字／标记与状态边框，保留独立焦点外环，取消整体禁用透明度；减少透明度合成默认填充并移除默认装饰。文字缩放保留字体族／字重／相对行高；减少动画使用既有 ThemeMotion，局部过渡不能覆盖根设置。
- Style 生产／测试语义检查、Component 全部测试源码及新增实窗测试源码检查通过。两个修改后的主题／下载示例生产检查通过，Window.asyncio 按已变更的 Window 接口定向重建。四套主题、明暗模式的三个新增测试通过，覆盖解析复用、不同偏好／基础主题切换、非法缩放、稳定状态对比度、部件实际几何，以及减少透明度的离屏像素。焦点边框在组合状态中低于 3:1 的问题已修正并通过该回归。
- 新增共享来源的双窗口测试，检查真实 Label／Input 度量、编辑模型／历史／焦点保留、动画启停与停帧、局部来源覆盖及恢复。原 ThemeAccess 的纯值来源测试移入这个实窗用例，未新增测试专用的 scope 构造入口；既有帧复用测试显式固定其需要的动画与字号配置，避免机器偏好改变测试前提。
- 实窗测试及原有 ComponentFrameReuseTest 均在首帧前超时。已核对 macOS 控制台会话：ScreenIsLocked=true；此时系统暂停窗口呈现。该项仍待桌面解锁后运行，不能据离屏或语义检查声明实窗通过。已向用户请求在方便时解锁，期间继续不依赖显示的工作；没有改动系统安全设置。
- 已生成并查询 Style API 文档，核对 ThemePreferences、withPreferences、ThemeSource.resolve／read 的签名与导入路径；库 README、Component spec 和编程材料同步。临时站点已清理，保留更新后的 Style、四套主题及 Component 产物和共享缓存。

本批仅完成主题偏好的公共接入与离屏验证，全部控件派生样式、真实设备／读屏器、三平台 provider 和完整 S11 矩阵仍按计划推进。

### 原生连续点击计数

- Windows 在原生 down／double-click 消息上延续三击及更多次计数，使用消息时间、屏幕物理位置及当前系统时间／距离阈值；拖动、失焦或取消结束计数，正常最后一次 Up 后的 ReleaseCapture 保留历史。每个已按下按钮保留自己的次数，释放不受其他按钮或拖动重置影响。
- Linux 直接消费 GtkGestureClick 的 count，移除重复的固定阈值；核对 GTK 4.18 实现中设置变化、设备／按钮切换、拖动及 released 的配对行为。显式输入取消重置 GTK 控制器，重入取消与窗口销毁释放各有正常出口。AppKit 保留其原生计数。
- 新增独立 `window-clicks` Cone，直接编译 Windows 实际使用的计数函数；6 个测试在 macOS release 通过，覆盖连续／首次原生双击、阈值边界及热变更、消息时间回绕、窗口屏幕位置变化、拖出返回、取消和计数饱和。C11 与包含真实调用的 C++20 编译检查通过；未建立 Win32 API 模拟层。
- 新增 Win32 SendInput 三次点击用例，并扩展 GTK X11 原生输入测试；它们只发送按下／释放，由平台产生 count。既有 Win32 协议测试检查取消后的 DBLCLK 从 1 开始、Up 配对；这些用例尚待集中 Windows／Linux 验证，未将本地算法结果当作平台通过。Wayland 和实际人工设备输入保持独立待验。
- 同步 Window spec、公开注释、库 README、设计现状与编程材料。没有新增计数 timer 或改变公共 InputTime 的 Observed 合同；临时下载的 GTK 参考源码在核对后清理，保留后续需要的构建缓存。

### Window 语义资源与 AppKit 基础投影

- 新增 `scoop.window.accessibility` 包；低层 provider／node 为 owner 资源，节点身份在窗口生命期不复用。属性、动作、父子关系与阅读顺序通过明确发布更新，名称／值变化清除旧几何；未知、空文本、受保护文本、隐藏和离屏分别处理。没有新增 Cone、IR、runtime 或另一套事件循环。
- AppKit 使用 NSAccessibilityElement 接入现有内容 view。getter 读取已发布原生状态，实际动作经捕获 Context 的 ForeignCallback 执行；业务／watch／另一动作中重入返回 Busy，BOOL 仅对已完成动作返回成功，无返回值的 setter 用实际值验证结果。异常沿既有应用退出路径处理。
- 节点关闭立即拒绝旧对象动作，包括尚未生成原生对象的节点；provider／Window 关闭撤销注册并在回调退栈后释放 callback。父节点退休覆盖后代，重新附着不复用身份。几何使用窗口逻辑坐标，未知布局不公布 frame，零面积不计为 visible child；未实现的精确命中不以矩形中心冒充。
- 10 个 Scoop 文件格式检查、Window 与测试 Cone 语义检查、生产／测试 Objective-C 严格编译检查通过；独立 AX 客户端的 C11 严格编译通过。macOS release 的 `window-accessibility` 单一应用测试通过，覆盖十二个连续原生动作阶段，以及属性发布前后、关系／几何校验、Unicode 写入与拒绝、焦点、隐藏／禁用、重入、节点／provider／Window 退休和异常收尾。测试通过公开 NSAccessibility 方法检查，不读取生产私有状态。
- 外部 AX 客户端连接权限检查通过，但尚未完成跨进程动作：AXWindows 返回一项，该引用的远程 role 为 AXApplication；同一个窗口在进程内为 AXWindow。独立的标准 AppKit NSWindow／NSButton 对照在普通窗口级别也复现此现象，且进程内 isAccessibilityElement=true。本机仍锁屏，当前无法确定异常的具体系统原因；待桌面恢复后复验，不把进程内通过记为外部 AX 或 VoiceOver 通过，也不改动系统安全设置。
- 已生成 Window API 文档并用 `scoop api search/show` 核对 Darwin 工厂、导入路径、SemanticProperties／Actions、Boolean focus、可选几何访问器和不可公开构造的节点身份。同步正式 spec、README、设计状态及编程库导航。临时文档站点、标准 AppKit 对照源码和二进制在核对后清理；保留外部 AX 客户端和后续构建缓存。

本批是 S0／S2 的低层基础，工厂当前仅在 Darwin 目标公开；UI scope 与 Accepted 场景、默认控件、完整文本、虚拟集合、其他平台 provider 和真实屏幕阅读器仍按统一计划推进。

### 仅删除选区的菜单命令

- 新增 MenuRole.DeleteSelection、TextEditCommand.DeleteSelection 与 TextCommandAvailability.deleteSelection。没有选区时不删除相邻字符、不结束组合、不调用编辑策略或增加撤销；可用性要求显式声明，不从 IME 开关猜测。AppKit 原生字段使用 delete:，Windows 原生 Edit／RichEdit 使用 WM_CLEAR，GTK Scoop 内容沿既有命令派发；按实际平台继续验证。
- TextEditHost 的删除与 Cut 复用同一选区编辑准备路径，保留策略过滤、异常前不发布、独立撤销和方向／affinity；删除不写剪贴板。只读／禁用和焦点检查沿原宿主执行，SelectableTextHost 不提供删除。Surface 传递新增可用性和真实命令，保留 preventDefault、焦点／边界资格与静止停帧。
- macOS 的三个编辑事务测试通过：空选区与活动组合／redo 保留、Unicode 反向选择与独立 undo／redo、策略拒绝／异常。两个 Surface 路由测试通过，覆盖可用性继承、失焦、禁用、边界、默认动作阻止及没有合成键盘事件。生产／测试 Scoop 格式与语义检查、AppKit Objective-C 严格编译检查通过；Window、Surface、UI 开发产物已按依赖定向更新。
- 已扩展真实编辑器用例，检查菜单能力、删除／撤销、空选区、只读／禁用与剪贴板保留；原生菜单焦点用例同时检查 Scoop DeleteSelection 事件与实际 NSTextField 删除及空选区禁用。后者原先用禁用菜单项模拟后台状态，现改为条目正常启用、Scoop 发布全 false，准确验证系统字段沿自己的状态判断。这两组源码检查通过，前台运行仍待桌面解锁，没有据离屏结果声明菜单交互通过。
- S0.3 的独立 AppKit 原型确认：公开 NSTextView.defaultMenu 无须创建隐藏编辑器即可返回系统模板，但其中富文本、拼写、替换及排版动作仍依赖真实编辑服务。文本菜单后续按下述可执行能力接入，不能直接展示全部模板来假报支持。
- 已生成并查询 Window／UI API 数据，核对两个 DeleteSelection 枚举入口、七项可用性的默认值、TextEditHost.execute 与导入路径；同步相关 spec、README、设计现状及编程材料。临时菜单模板程序和 API 站点已清理，保留开发 Cone 与缓存供后续工作使用。

### 文本宿主的原生上下文菜单

- Window 提供不持有编辑目标的 TextContextMenuTemplate、七项标签覆盖和平台辅助点击查询。AppKit 从公开 NSTextView.defaultMenu 保留可执行的标准编辑项目并补齐缺项；Windows／GTK 使用标准菜单回退。SystemTemplate 明确表示系统模板的可执行子集，未接通的拼写、替换、翻译等服务没有伪装成可用菜单项。未取得的系统标签使用英文默认值，应用可逐项覆盖。
- TextEditHost 已接入 Disabled／StandardNative／PreferSystem、Platform／Preserve／PlaceCaret／SelectWord 和键盘／程序入口；指针选择及菜单锚点使用 Accepted 文本布局，点击选区内保留方向。打开菜单停止当前拖选和越界自动滚动。菜单首次请求才分配原生资源，动作绑定发起宿主、模型与会话，按当前资格与策略执行；失焦、换模型、禁用、配置变化及关闭使旧动作失效，完成通知不重复编辑，没有后台轮询。
- Input／Textarea／InputOtp 与 builder、InputGroup、ButtonGroup、Command、Combobox、SidebarInput 已透传配置；OTP 仍只有一个编辑宿主和长度／模式策略。检查过程中补齐了 ButtonGroup 重建 InputProps 时的透传，增加实际组合入口回归。
- AppKit 原生菜单 Cone 的两项非忽略测试通过，包含真实默认模板标题、七种命令、标签覆盖、辅助点击、菜单会话和资源收尾；原生焦点测试仍按既有前台要求单独待验。UI 的两个新增定向测试通过，覆盖配置快照、菜单键、重复／合成／释放事件及额外修饰键。组件生产与全部测试源码检查通过，receiver DSL 的六项测试和 ButtonGroup 的三项测试全部通过。组合链接中的过期 UI 依赖已通过按顺序更新 Style、四套主题和 Component 开发产物闭合。
- 既有真实编辑器用例增加二十个菜单阶段，覆盖右键与 Control+左键、选区保持／定位、菜单删除、Shift+F10、粘贴时读取新剪贴板与长度过滤、撤销、只读、关闭菜单、preventDefault、换焦点／模型、禁用和关窗后的迟到动作。源码检查与 release 可执行文件构建通过；本机仍锁屏，没有据这些构建结果声明前台文本菜单交互通过。Linux／Windows、受保护用途、多击组合、系统文本服务、语义菜单动作和触摸长按继续保留在完整计划中。
- 已同步 Window、UI、Component spec、README 和编程材料，生成并查询三个 Cone 的 API，核对默认值、标签类型、showContextMenu 和 builder 重载及有效 import。临时 API 站点已清理，开发 Cone 和待解锁运行的测试程序保留。


### 滚动条活动反馈与操作

- ScrollArea、VirtualList、Textarea 和 MessageScroller 提供不可变 ScrollbarBehavior 与 receiver copy；InputGroup 透传多行配置。按范围判断的 Auto／Always／Hidden 与闲置隐藏分别解析，FollowSystem 消费根的有效偏好，未知值回退常显；显式 autoHide、展宽、深浅、触发条件和各过渡时长可独立覆盖。
- 共享控制器用一份显隐动画、一个 native deadline 和一个非消费式指针观察管理两轴；原有按比例的 thumb 拖动保留。按住、hover、滚动路由／momentum、相关键盘和程序动画暂缓隐藏；LayoutCorrection 不续期。隐藏后撤下输入区域，重新出现等待 Accepted。命中条带独立于绘制宽度，主题保留主轴几何，交叉轴缩放完整绘制，消费本帧透明度和 showTrack；Material 的大内缩不会挤空细条。
- 增加 Page／Jump 轨道行为、轨道与箭头的可关闭按住重复；按下接管与最终释放沿既有输入路由，preventDefault 的释放仍清理。主题／范围／状态替换及禁用继续使用 Accepted 几何与原取消通路。减少动画立即完成过渡；减少透明度取消半透明及淡入淡出，保留自动隐藏。
- 主题库的语义检查、生产构建及四套主题依赖重建通过；组件生产／测试代码检查及生产构建通过。20 项定向测试全部通过：8 项新行为／时序／四主题细条绘图测试、4 项滚动布局、2 项滚动装饰和 6 项 receiver DSL。原生场景复用既有窗口，新增 17 个阶段覆盖隐藏点击穿透、静止停帧、边缘唤醒、按住翻页、被拦截的释放、跳转拖动和常显偏好热切换；源码检查与 release 可执行文件构建通过。本机仍锁屏，尚未运行这些前台交互阶段，不能据构建结果声明实窗验收通过。
- 本批保留现有 ScrollArea／VirtualList 的 Overlay 与 Textarea 固定占位；统一布局选项、Overlay 对 caret／焦点显露的遮挡处理和经 S2 接入的滚动语义仍属于 S4 未完成部分。真实设备／平台验证按总计划进行，不以值测试或构建通过代替。

已生成并查询 Style／Component API，核对 ScrollbarBehavior 的完整默认值、构造方式、Textarea／ScrollArea builder 重载、VirtualList／MessageScroller Props 和 showTrack；临时 API 站点已清理。

### 滚动条占位与边缘显露

- ScrollbarPlacement 提供 Overlay／Gutter，ScrollArea／VirtualList／MessageScroller 默认 Overlay，Textarea 默认 Gutter；receiver copy 保留既有选择。Gutter 在一次正文测量前扣除固定最大条带，End 侧和双轴角部使用共享几何，范围与反馈过渡不改变占位；虚拟列表分别保留外尺寸和实际正文视口。
- ScrollPadding 同时扩展可滚动范围与排除显露边缘，snapshot 区分含留白的 content、不含留白的 bodySize 和有效 visibleSize。Auto 只按原始正文溢出生成条，不由留白生成另一条。默认位置及无新定位意图时的左／上留白变化按 LayoutCorrection 保持正文位置；明确位置和动画意图优先。
- FocusTarget、文本 caret／拖选／翻页沿同一快照，正文输入区域和 Accepted 显露边界同步变换。TextEditHost 的纯测量回调在排版后一次解析留白；Textarea 无须反复排版。虚拟阅读锚点仅修正主轴，保留交叉轴显露位置。accessibility 设计明确 provider 的 Reveal 复用这条路径，不自行计算未扣遮挡的区域。
- UI 生产／测试语义检查与生产构建通过，滚动定向测试 13 项全部通过；公共滚动原生场景已增加首尾焦点显露、正文位置保持和 LayoutCorrection 验证，源码检查与 release 可执行文件构建通过。组件生产／测试源码检查和生产构建通过，30 项定向测试全部通过，覆盖占位、滚动几何、活动反馈、消息阅读模型、receiver 默认值和文本尺寸；Style 与四套主题已按依赖更新。普通滚动区、虚拟列表、文本控件三个原生场景分别补充占位热切换、RTL、阅读锚点／交叉轴保持、文档末端及实例／选区保留；三个场景的源码检查和 release 可执行文件构建全部通过。本机仍锁屏，四个场景的前台实际运行尚未完成，未据构建结果声明交互验收通过。

UI／Component API 已生成并核对 ScrollPadding、ScrollSnapshot 的 padding／bodySize／visibleSize、ScrollViewport.layout、TextEditHost.measure，以及 ScrollbarPlacement、Behavior／builder 的 placement 和 Textarea 的 Gutter 默认值、receiver 入口与导入路径。新增纯值测试已登记 Windows／Linux GNU 的目标选择；这些目标在集中跨平台轮次运行，Darwin 源选择保持不变。临时 API 站点已清理，保留开发产物与待前台运行的测试程序。
