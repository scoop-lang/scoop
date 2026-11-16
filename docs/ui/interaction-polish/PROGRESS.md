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
| S2 | 进行中 | Window／AppKit 基础投影及 UI scope、逻辑发布、焦点绑定、关系、内容合并／忽略、Accepted 场景与精确命中已实现；基础控件、只读文本及 Field 注释关系的原生回归通过，场景构建通过但实窗待验；滚动及其余控件、复杂关系、激活点和外部客户端仍待完成 |
| S3 | 进行中 | DeleteSelection、两类文本宿主的三种菜单模式、目标绑定、选区策略、键盘入口和独立 protectedText 用途已接入；AppKit 菜单与定向测试通过，真实文本菜单因锁屏待验；多击组合、文本 provider／系统服务和其他平台仍待继续 |
| S4 | 进行中 | 共享滚动条行为、配置入口、命中、轨道跳转及重复操作已接入；统一 Gutter／Overlay 与边缘显露已接入，本批 43 项定向测试及四个原生场景的源码检查／构建通过；滚动语义和实窗／平台验收待继续 |
| S5 | 进行中 | 编辑／只读宿主的正文、选区、替代文字映射与 ShowContextMenu 已接入，Input／Textarea、SelectableStyledText／Typography 及包装自动发布宿主语义；macOS 原生方法与编辑策略定向测试通过；Field 标签／说明／错误和只读必填属性沿原连接发布；实际菜单呈现、范围几何／单位导航／保留范围、invalid／busy 等其余状态、复杂焦点与模态待继续 |
| S6～S11 | 待开始 | 保持统一计划的完整范围，未将基础 API 交付视为上层交互或 provider 已完成 |

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

### UI 语义资源、逻辑发布与场景接入

- ComponentScope.semantics 创建 scope 管理的稳定资源，根在首次登记时创建 provider。普通布局透明，多节点及显式内部父子关系按实际挂载顺序发布；子挂载路由可单独指定。关闭一个 handle 保留其他资源拥有的子节点，真正卸载和关根退休全部所属对象。工厂随当前 Window 能力仅在 Darwin 源中提供，资源、路由与场景代码共用，没有空 provider 或运行期平台分支。
- 属性和最新 handler 沿 UiHost 现有逻辑队列发布，不请求 drawable；只有结构变化才遍历挂载树。原生动作检查生命周期、输入边界和 UI 执行状态，经 Surface 错误边界执行并刷新实际 State 结果；普通 scope.dispatch 同样进入宿主回调边界。动作自删或关根保留实际完成结果，异常只关闭对应 UI 根。FocusTarget 绑定读取真实活动焦点，目标退休清除原生焦点事实。
- SceneContext.semantics 通过真实 Canvas 变换与 clip 捕获值型几何，进入缓存记录和重放。仅 Accepted 更新发布几何，Discarded 保留仍匹配的旧值；名称／值变化立即撤下旧几何，未放置不退休身份。捕获后释放 CanvasGeometry，不为属性发布保留绘图资源，也不把矩形中心当成可点击点。
- UI 生产／测试检查及 release 构建通过；两个测试 Cone 和共用 native 辅助检查通过，Objective-C 格式及严格编译检查通过。原生逻辑测试实际通过，覆盖隐藏窗口中 State 与直接 handle 更新、无 measure／scene、阅读顺序、角色替换、最新 handler、Unicode 与拒绝、禁用／隐藏／只读、原生动作自删、两种回调重入、根解除／重附着、旧对象退休和异常收尾。辅助只使用公开 NSAccessibility 协议，不读取生产私有结构。
- 独立场景测试的源码检查与 release 可执行文件构建通过，覆盖目标为缓存重放后的祖先变换／裁剪、零透明度、未放置、名称变更、原生焦点及目标退休、Accepted／Discarded。本机仍为 ScreenIsLocked=true，实际呈现及焦点验证尚未运行；没有据构建结果声明通过，也没有将进程内方法调用记为外部 AX／VoiceOver 通过。

已同步正式 UI 无障碍 spec、Window／UI 引用、设计状态、README 与编程库导航。README 的自定义 Element 示例实际检查通过；UI API 已生成并用 search／show 核对工厂的目标与默认值、SemanticsHandle 只读属性和方法、子项路由、场景入口与 import。临时示例和站点已清理，保留开发产物、共享缓存与待实窗执行的程序。内置控件默认语义、关系／内容合并、完整文本／集合、精确命中和其他平台仍按 S2 及后续批次推进。

### 受保护文本用途与显隐分离

- TextEditOptions 的 protectedText 明确声明受保护用途，默认 false，与显示 projection、只读状态及复制策略分别保存。Default 在受保护用途或内置遮罩时禁用 Copy／Cut，显式 Allow／Deny 保留原意；显示明文不会通过投影隐式开放复制。Paste、DeleteSelection、编辑策略和撤销沿现有事务，不增加文档或 IME 宿主。
- Input／Textarea／InputOtp 及 builder 提供同一标记，InputGroup、ButtonGroup、Command、Combobox、SidebarInput 完整透传。已有密码示例始终声明 protectedText=true，只切换显示投影。用途或复制策略改变后沿原菜单能力刷新，迟到命令继续检查当前资格；完整文本 provider、拖放和系统服务随所属批次消费此事实，没有因添加标记而声明这些能力完成。
- 20 个变更的 Scoop 文件格式检查通过；UI 和 Component 的生产／测试源码检查、release 生产构建及密码示例检查通过。公开接口变化涉及的 Style／四套主题产物已按顺序更新。复用同一组件测试程序，6 项 receiver DSL、3 项 ButtonGroup、4 项 InputGroup 测试全部通过，覆盖标记透传与快照保留。
- 既有编辑器组合用例增加受保护文本的明文显示、默认复制／剪切拒绝、显式允许、IME 连接及模型保持；原生菜单增加 Copy／Cut 禁用、Paste／Delete 可用、撤销和选中菜单项后的策略变化。源码检查与 release 可执行文件构建通过，桌面仍未恢复，未据构建结果声明这些前台交互通过。

同步 UI／Component／原生菜单 spec、accessibility 与交互设计、README 和编程库导航。两份 API 站点已生成并核对 protectedText 的类型、默认值、只读 Props 与可写 builder、导入路径；临时站点已清理。保留开发产物、共享缓存和待实窗执行的程序，其他平台继续留到集中验证阶段。

### 只读可选文本的原生菜单

- SelectableTextOptions 增加与编辑宿主相同的菜单配置，SelectableTextHost 提供键盘式入口；SelectableStyledText 与 Typography 沿现有 options／selection 完整透传。两类宿主共用一个原生会话控制器，只读菜单保留 Copy／SelectAll 及有效分隔线，复制沿 Accepted StyledText 的替代文本语义。
- 支持选区内保持方向、选区外 Preserve／PlaceCaret／SelectWord／平台策略、右键、平台辅助点击及菜单键／Shift+F10。共享 Press 对辅助按下不建立普通点击候选，菜单关闭或禁用也不会将 Control＋点击变成行内链接激活或文本拖选。
- 替换编辑模型前取消旧绑定；只读正文或行内替代文字变化也取消，单纯样式变化保留。焦点／选择回调换绑停止打开，复制开关和配置变化刷新资格；已选中但尚未交付的原生动作仍受同一绑定与最新能力约束。
- 受影响源码格式检查、UI 生产／测试检查与 release 构建通过，两项菜单配置／键盘入口测试实际通过；Style、四套主题和 Component 生产产物按依赖顺序更新。复用既有编辑实窗程序增加十阶段只读菜单场景，源码检查与 release 可执行文件构建通过，覆盖命令过滤、键盘锚点、方向选区、选词、辅助点击、迟到复制、替代文字换绑及样式变化。本机仍锁屏，前台运行尚待验证。

正式 spec、设计／计划、两库 README 和编程导航已同步。UI API 已生成并核对只读菜单配置及方法；使用公开 Component 入口的独立示例实际检查通过。临时 API 站点与示例已清理，保留后续所需的产物、缓存和实窗程序。没有因这批共用菜单接入而声明完整系统文本服务、文本 provider 或其他平台已经完成。

### 语义名称、说明、错误与控件关系

- SemanticRelations 为同一活动 provider 内的有序节点 ID 引用，提供 labelledBy、describedBy、errorMessages、controls；Window／UI update 同时替换事实、动作和关系，节点的只读 relations 返回清理后的当前值。替换前检查归属、重复、自身和名称依赖环，合法的非名称关系循环不被父子树规则误拒绝。目标退休自动清除引用，不跟随同位置或新角色转接。
- 名称、说明和新增 error 属性保留显式值优先及未知／空字符串差异；关联静态文字支持传递名称和非受保护 value。变更沿实际反向依赖更新，原生 getter 不读取业务状态或遍历名称图。hidden 标签可以贡献显式引用的文字，同时继续留在导航树外。
- 名称／值及名称依赖变化使旧几何失效，contentRevision 统一供 UI 候选场景核对；说明、错误和 controls 更新保留匹配的几何。UI bindFocus 读取已清理的当前关系，不重新提交退休引用。AppKit 映射单标签 TitleUIElement／ServesAsTitleForUIElements、多标签合成名称、LinkedUIElements 及包含说明／错误的 Help，并随发布发送适用通知。
- 12 个变更的 Scoop 文件格式检查、Window／UI 生产检查、两个 UI 测试 Cone 与 Window 测试 Cone 检查通过；生产与新增测试 Objective-C 严格编译检查通过。Window 的单应用测试实际通过，覆盖关系解析、依赖更新、错误更新、名称环／重复／自身引用拒绝、合法循环、几何版本、hidden、清理与旧对象，并保留原有原生动作、重入和关闭回归。
- UI 的无 drawable 逻辑测试实际通过：字段引用其他挂载的静态文字，隐藏窗口中的直接标签更新能改变原生字段名称，没有 measure／scene；跨根关系在修改前拒绝。场景测试改用标签依赖，源码检查与 release 可执行文件构建通过。本机仍为 ScreenIsLocked=true，Accepted 场景的前台运行仍待验证。

同步正式 Window／UI 无障碍 spec、两份设计、计划、README、测试说明和编程导航。两库 API 已生成并用 search／show 核对关系字段、error、update 默认值、只读关系／内容版本与导入路径；临时站点已清理。仅按当前消费者更新 Window、Surface、UI 产物，组件和主题待实际消费新接口时再按依赖更新。完整内容合并、控件默认语义、文本／虚拟集合、其他平台与外部客户端继续按统一计划推进。

### 内容名称合并与忽略节点

- SemanticProperties.ignored 仅忽略本节点并展开其后代，hidden 继续排除整个子树；忽略／恢复保留身份，不改变真实焦点，旧引用不能执行被忽略节点的动作，后代仍受祖先禁用约束。AppKit 按发布的父子／阅读顺序维护子项索引，普通名称和几何更新不重建索引。
- Window contentLabels 提供独立于 SemanticRelations 的内容名称回退，优先级低于显式 name 与 labelledBy，普通 update 保留内容引用；空字符串与未知值继续分开，目标退休自动清理。名称环检查从关系替换移到 publish 的实际名称解析，支持同一轮撤下旧内容依赖再建立新关系；显式名称已覆盖的依赖可发布，实际混合名称环失败保留旧原生投影，修正后重试成功。
- UI mergeLabels 默认关闭，开启后只合并无动作／焦点／语义子项且非受保护的 Text 叶子。ignored 包装允许继续查找后代，普通 Group、带子控件的文本、可聚焦文本与嵌套合并容器保持独立；资格或结构变化统一恢复／合并，纯文字变更复用既有依赖更新，不重走挂载树或请求 drawable。
- Window／UI 生产源码和受影响测试 Cone 检查、三个依赖产物的 release 构建、Scoop 格式与 Objective-C 严格编译检查通过。Window 原生测试实际通过，新增 UI 内容合并测试与既有无 drawable 逻辑回归实际通过，覆盖优先级、名称环恢复、展平、动态动作／焦点资格、真实原生动作、重排身份、隐藏／忽略和关闭；隐藏窗口中的纯文字更新没有 measure／scene。

已同步两份正式无障碍规范、设计与计划、README、测试说明和编程导航，生成并查询 Window／UI API，核对 contentLabels、ignored、mergeLabels 的访问器、默认值和 import；临时 API 站点已清理。桌面仍为 ScreenIsLocked=true，未重复启动依赖实际呈现的场景或将本批进程内原生方法记为外部 AX／VoiceOver 通过。内置控件、精确命中、完整文本与虚拟集合及其他平台继续按完整计划实施。

### 已接受场景的原生精确命中

- graphics 所需的不可变 native 命中快照已独立提交，并经 [PR #143](https://github.com/scoop-lang/scoop/pull/143) 合入 main；工作分支与开发 graphics／text 产物已更新。Window 的 NativeSemanticHitTest 只描述配套的 context／retain／release／contains，不依赖 graphics，也不在原生 getter 中回调 Scoop。节点与已发布投影分别保留引用，普通 geometry 赋值清除命中，内容失效、替换、退休和关闭释放各自引用。
- UI 的 Rect／Path 语义登记捕获真实变换、路径 clip、viewport 与层叠顺序；Rect 参与缓存重放，Path 与已有输入登记一致使该场景不缓存。候选、节点及原生投影寿命分别管理，只有内容版本匹配的 Accepted 场景生效；Discarded、场景失败与根关闭释放候选。AppKit 从屏幕点转换到当前窗口逻辑点，按前景顺序查询，支持 ignored 祖先映射和独立后代；阅读顺序不随场景顺序变化，没有命中快照时不猜矩形结果。
- 13 个变更的 Scoop 文件格式、6 个 native 文件格式及生产／测试 Objective-C 严格编译检查通过；Window／UI 及受影响测试 Cone 检查、Window→Surface→UI 的 release 构建通过。Window 原生测试实际通过，覆盖带孔路径、源对象关闭、层叠／阅读顺序、窗口边界、禁用／隐藏／忽略、内容失效、仅矩形赋值及发布失败保留旧快照。UI 无 drawable 逻辑与内容合并两组回归实际通过。
- UI 场景程序新增重叠矩形与带孔 Path、屏幕坐标查询、祖先裁剪、缓存重放、未放置、内容失效、层叠重排和 Accepted／Discarded 组合；源码检查与 release 可执行文件构建通过。桌面仍为 ScreenIsLocked=true，实际呈现及焦点场景未运行；进程内 NSAccessibility 方法结果不代替外部 AX、VoiceOver 或其他平台验收。ActivationPoint 的真实输入资格与可点击位置仍待接入。

正式规范、两份设计、计划、README、测试说明与编程导航已同步；Window／UI API 已生成并查询 setGeometry、NativeSemanticHitTest 的字段和 import，以及 SceneContext 的 Rect／Path 重载。临时 API 站点已清理，保留共享开发产物与待实窗运行的程序。内置控件、完整文本／集合、其余平台和本轮交互能力继续按统一计划推进。

### 纯文本正文、选区与真实编辑事务

- Window 的 Text／TextField 以已知非保护 value 提供正文，selection 保留公共 UTF-8 方向选区；AppKit 复用原文本偏移转换，提供长度、选中文本、UTF-16 单范围和纯文本 attributed string。范围端点不得切断 surrogate；选择可用于只读文本，选区替换检查实际正文版本与当前选区，拒绝禁用／只读写入。选区变化只更新逻辑事实与通知，不清除节点几何。原生发布使用已持有的内部版本，修复了发布过程中访问公共 contentRevision 被重入保护拒绝的问题，没有放宽公共边界。
- UI 的 updateTextEditor 借用同一挂载的 TextEditHost，发布现有模型、选区和真实焦点，保留名称、说明、错误与关系；捕获模型及版本，拒绝旧目标写入。Accessibility 来源与 Paste 共用外来文本清洗，再执行同一单行、字符／长度、Reject／Transform 和 undo 事务；纯选区结束 composition 沿 EditCommand，不重复清洗正文，也不抢焦点。公开选区使 Text 保持独立语义节点。protectedText 与复制开关／显示投影独立，禁止正文、长度和选区读取，保留实际允许的全文替换。
- 16 个 Scoop 文件与 7 个 native 文件格式检查、生产／测试 Objective-C 严格编译检查、Window／UI 及定向 Cone 源码检查通过；Window→Surface→UI 的 release 产物已更新。TextEditPolicyTest 的 9 个值测试实际通过，覆盖新来源、清洗、拒绝与 undo／redo。Window 原生回归、UI 逻辑动作、内容合并和新增文本编辑四个 Cone 实际通过；新用例覆盖 Unicode 范围、单选区／多选区拒绝、只读／禁用、策略、composition、受保护读写、无 drawable 发布和关闭后旧对象。
- 受共用 native 测试接口变化影响的 UI 场景程序已重新构建成功；需要真实呈现／焦点的场景及既有 IME 快照实窗回归仍待桌面恢复后验收，未把进程内方法结果记为外部 AX／VoiceOver 通过。本批复用的 UTF-8→UTF-16 转换算法没有改变；范围几何、单位导航、retained range、内置表单和其他平台继续按 S5／S10 推进。

已同步正式规范、设计、计划、库与测试 README、编程导航及原文本策略设计。Window／UI API 通过 search／show 核对 selection、setSelection、replaceText、updateTextEditor 的签名／默认值／import 与 Accessibility 来源；临时站点在核对后清理，保留开发产物、共享缓存与待实窗运行的程序。

### 只读富文本选区与语义菜单动作

- updateSelectableText 将同一挂载的 SelectableTextHost 接入 Text 节点，正文保留行内 alternateText，选区复用已有区间替换映射；选择部分替代文字覆盖整个对象，caret 使用下游端点，空替代文字按端点 affinity 处理。普通 update 和选择通知刷新原 handle，不替换业务回调，不创建可写文档。宿主按不可变内容身份复用正文与映射，拖选不反复复制全文；内容替换、受保护发布和关闭清理缓存，旧内容动作不能转向新内容。
- SemanticActions.showContextMenu 接到 AppKit ShowMenu 与同一 owner／输入边界，两个文本宿主使用原生菜单会话和 Accepted 锚点。明确菜单请求可先取得实际焦点；未就绪、已有会话或模式禁用不假报成功，只读仍可打开真实可用菜单。原生动作能力增删发送 LayoutChanged，能力相同的 handler 替换不重复通知，也不重建阅读索引。原生菜单继续提供自己的语义。
- 18 个 Scoop 文件与 4 个 native 文件格式、生产／测试 Objective-C 严格编译检查、Window／UI 和三组相关 Cone 源码检查通过。Window→Surface→UI 的 release 开发产物已更新；新增只读富文本、扩展的原生语义动作以及原有编辑语义三个 Cone 实际通过，覆盖 Unicode 与替代文字偏移、方向选区、拒绝／只读／隐藏、内容与样式替换、菜单能力开关、无 drawable 更新和旧对象退休。正文投影复用调整后，只读用例再次通过。
- 实窗编辑程序通过 release 可执行文件构建，两个菜单场景改为从原生主队列调用公开 ShowMenu，并检查返回值、tracking、项目可用性及模型／剪贴板结果。桌面仍为 ScreenIsLocked=true，实际菜单呈现与焦点场景未运行；动作通知的外部客户端观察、AX／VoiceOver 和其他平台也未据进程内方法结果记为通过。

正式规范、设计、计划、使用说明和编程导航已同步；新建只读语义与既有编辑实窗的 README 说明入口和验证边界。Window／UI API 通过 search／show 核对 showContextMenu 与 updateSelectableText 的默认值、签名和 import；临时站点在核对后清理，保留后续需要的开发产物、缓存及实窗程序。

### 基础控件默认语义与组合资源归属

- Label、Button、Checkbox、Input／Textarea 的 Props 和 builder 提供 ComponentSemantics。静态正文保留 alternateText，按钮合并纯标签并保留独立操作子项；最新 onClick 沿原稳定入口，Checkbox 用原 Binding 确认真实结果。输入框使用原 TextEditHost 发布正文、选区、只读、受保护用途和菜单；placeholder 不充当名称，既有 description 作为默认说明／错误，重复装饰文字不另占阅读节点。ButtonGroup、InputGroup、Command、Combobox、Sidebar、原生文件按钮及文字包装同步传递覆盖值。
- 语义几何允许绘制 Element 登记本挂载或逻辑祖先的资源，保留真实 FocusTarget、原有输入与资源归属；兄弟、后代、跨根和重复登记仍拒绝。Rect／Path 的组合场景用例已覆盖此路径并重新构建。UI handle 记住上次成功的关系声明，未变化时复用 provider 已清理的当前关系；关闭标签后继续更新控件不会重提退休引用，显式更换关系仍须验证。
- Scoop 格式检查、native 辅助代码严格编译、UI 与组件（含测试）源码检查通过；必要的 UI、SVG、图标、style／主题和 component release 开发产物已更新。UI 原有语义动作回归与新的基础控件原生回归实际通过，覆盖无 drawable 的逻辑更新、最新回调、纯文字比较、名称／关系覆盖、关系退休、独立子按钮、三态／拒绝／禁用、Unicode 文本选区与 undo、只读、保护用途和旧对象退休。
- 场景程序完成 release 构建；真实呈现、焦点、外部 AX／VoiceOver 和其他平台仍未据进程内结果记为通过。组件只在下层已有 provider 的 Darwin 目标创建真实资源，Windows／Linux 的 provider 及其余控件继续按 S2／S5／S10／S11 推进，不以透明的未接入目标关闭计划。

正式 UI／Component spec、设计、计划、README 与编程导航已同步。Component API 经 search／show 核对覆盖值、Props／builder、默认值和 import，README 示例实际检查通过；临时站点与示例在核对后清理，保留后续所需开发产物、缓存和实窗程序。


### 内置只读文本默认语义

- SelectableStyledText 使用原 SelectableTextHost 发布正文、选区及真实菜单能力；选择通知先刷新同一语义资源，再调用一次最新业务回调。Props／builder 提供 ComponentSemantics，Typography 的可选与不可选两分支传递相同覆盖值，切换分支退休旧对象。
- 六个 Scoop 文件格式检查、Component 及扩展测试 Cone 源码检查通过；组件 release 开发产物已更新。基础控件原生回归实际通过，新增覆盖行内替代文字的部分选择映射、最新回调、只读写入拒绝、禁用选择，以及 Typography 分支替换后的正文、名称和旧对象失效。
- 正式规范、设计、计划、README 与编程导航已同步；API search／show 核对两类 Props／builder 的新字段、默认值及 import，84 个本地 Markdown 目标检查通过。临时 API 站点及不再使用的输出已清理，保留后续需要的开发产物和共享缓存。

本批未改变下层文本范围映射；真实呈现、菜单焦点、外部 AX／VoiceOver、标题级别和行内范围动作的专门语义仍按 S5／S11 验收。


### Field 注释关系与动态连接

- FieldTarget 复用原 FieldLink，把 FieldLabel、FieldText 与 FieldError 的实际节点连接到 Button、Checkbox、Input／Textarea。注释 Group 合并静态内容，保留自定义标签内的独立按钮；错误去空去重与视觉一致，项目符号为装饰。注释关闭／重建在不重新渲染控件父级的情况下刷新关系，换绑不重建输入或编辑模型。
- Field 可显式指定 target，缺省从 label 取得；与已有 label 指向不同目标时报告普通参数错误。独立标题／说明／错误支持 target 和 ComponentSemantics。显式文字、空串及每类非空关系保留优先级，自动关系只使用同窗口的活跃节点。
- 生产及定向 Cone 源码检查、Scoop 格式检查与 release 构建通过；扩展的原生回归实际通过，覆盖注释生命周期、动态文字与错误、独立子按钮、覆盖优先级、分别换绑、静态 title 和文本模型保留。新增禁用用例先复现了标签容器仍报告可用的问题；已同步实际 InputArea 的 enabled，禁用及恢复的原生查询复跑通过。
- 既有 Field 交互程序完成 release 可执行文件构建；桌面锁定下没有运行呈现、指针或焦点验证，也未把进程内查询记为外部 AX／VoiceOver 通过。正式规范、两份设计、计划、README 和编程导航已同步；API 查询核对 Props、builder 及继承字段，README 示例实际检查通过，102 个本地 Markdown 目标检查通过。临时站点和示例已清理。

required／invalid 等表单状态、复杂焦点、模态和其余控件仍按 S5／S11 推进，本批只交付上述真实注释关系及验证。

### Field 必填属性

- SemanticProperties.required 默认 false，AppKit 使用公开的 isAccessibilityRequired 只读属性。状态变化发送适用的属性布局通知，不当作正文值变化；UI 的标签合并与两个文本宿主保留该事实，不触发编辑或焦点动作。
- Field.required 沿现有 FieldTarget／FieldLink 进入 Button、Checkbox、Input／Textarea。每个 Field 拥有自己的必填声明，同窗口任一活跃来源为 true 时目标为必填；来源关闭、换绑和全部恢复 false 直接刷新现有节点。没有新增验证器或命名覆盖状态。只有同窗口目标已提供语义时才把重复的 requiredText 当作装饰，其余情形继续保留提示阅读。
- 18 个 Scoop 文件与 4 个 native 文件的格式检查、Objective-C 严格编译检查通过。Window／UI 含测试的源码检查、Window／Surface／UI／style／shadcn／Component 的 release 产物构建通过；只更新实际依赖这些接口的产物，没有每次执行全库测试。
- Window 原生回归验证必填变化保留正文版本、几何和精确命中。Component 原生 Cone 检查及实际运行通过，覆盖动态启停、独立来源关闭／恢复且父级不重新渲染、多来源保留、控件／声明分别换绑、禁用时仍必填、只读属性能力及旧对象退休；原有文本、选择、关系和动作回归一并通过。

正式规范、两份设计、计划、README 与编程导航已同步，Window API 查询核对 required 的公开属性、默认值和导入路径；临时 API 站点已清理，本批文档的 149 个本地链接目标检查通过。实窗呈现、外部 AX／VoiceOver 和其他平台未据进程内方法结果记为通过；invalid／busy、复杂焦点、滚动语义及其余完整计划继续推进。
