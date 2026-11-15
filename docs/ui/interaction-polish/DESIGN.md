# UI 交互细节打磨设计

日期：2026-11-16。状态：实施中，实际交付与验证见[实施记录](PROGRESS.md)。本文承接已经交付的[基础交互](../foundations/DESIGN.md)、[控件基础](../component-foundations/DESIGN.md)及[原生菜单](../../specs/SCOOP-UI-NATIVE-MENUS-SPEC.md)，并与[accessibility 设计](../accessibility/DESIGN.md)共同安排后续工作。本文中的配置名称描述目标形状，正式签名、默认值和平台合同在对应批次实施前写入所属 spec；不能把本文作为新增 API 已可用的依据。

任务拆分、依赖、三平台与设备验收、附加能力核查及最终完成条件统一见[实施计划](PLAN.md)。计划使用 S0～S11 覆盖本文和完整 accessibility；下文 U0～U5 保留为设计内容的分组对照。

## 1. 现状与本轮目标

本轮完善真实桌面应用中的滚动、文本、鼠标数据拖放、触摸和手势，以及相应的视觉与无障碍体验。先修复现有公共能力在平台输入端的缺口，再增加实际缺少的能力。

| 项目 | 当前已具备 | 本轮需要完成 |
| --- | --- | --- |
| 滚动条 | ScrollArea／VirtualList／Textarea 共用行为配置、系统偏好、hover 宽度与深浅、闲置隐藏、独立命中条带和重复操作 | 当前验证见实施记录；统一 Overlay／gutter、遮挡显露、滚动语义及实窗／平台验收待继续 |
| 原生文本菜单 | 原生 popup、七种编辑角色及可用性；文本宿主与输入控件已接入三种菜单模式、右键选区、键盘入口和目标会话 | 前台实窗与 Windows／GTK 验证、触摸入口、受保护用途及系统文本服务；已验证范围见实施记录 |
| 多击选择 | 公共编辑器已有双击选词、三击选逻辑行、按词／行继续拖选，且已有实窗用例 | Windows 连续点击计数、Linux 系统阈值、平台真实事件及 Unicode／投影组合回归 |
| 拖动 | 公共轴向 `Drag`、阈值接管、取消、速度；用于滚动条、Slider、Resizable 等 | 数据 offer、drop target、Copy／Move 协商、预览、完成与取消，覆盖窗口内、同程序跨窗口及跨程序 |
| 文本拖放 | 已有拖选和越界自动滚动 | 从选区发起数据拖动、落点 caret、移动／复制、编辑策略与撤销 |
| 触摸／手势 | 鼠标指针及带相位的滚轮；控件已有部分鼠标拖动交互 | 独立 contact、取消、触摸滚动、长按、缩放／旋转等手势，以及既有控件接入 |
| 视觉效果 | 动画、spring、部分控件回弹、主题 `Full／Reduced` | 通用滚动回弹、惯性来源去重、跟随系统减少动画；完善高对比度、缩放与命中体验 |
| accessibility | 完整设计；低层 Window 语义资源和 AppKit 基础投影已接入，原生定向测试已覆盖属性／动作／退休 | 完成 UI 声明与 Accepted 几何、全文本／虚拟集合、三平台 provider 和实际客户端验收，并补本轮交互的语义动作、非拖动替代入口、文本菜单及拖放通知；完整范围见统一实施计划 |

源码核查的几个具体结论：

- [ScrollBarControl](../../../sysroot/src/scoop.ui.component/src/scroll/bar.scoop)现已接入独立宽度／透明度过渡、闲置隐藏和按住重复，共享行为由 ScrollbarFeedback 管理；[ScrollBarsLayout](../../../sysroot/src/scoop.ui.component/src/scroll/decoration.scoop)的 `Auto` 只按滚动范围决定是否显示，不表示一段时间后隐藏。Textarea 仍固定预留解析后的最大条宽；行为已共享，统一占位选项与遮挡显露待继续。
- [editorPress](../../../sysroot/src/scoop.ui/src/text/editor/pointer.scoop)及[编辑实窗用例](../../../sysroot/src/scoop.ui/tests-cones/editing/tests/scenarios.scoop)已有二／三击逻辑。S1 已在 [Windows 输入](../../../sysroot/src/scoop.window/native/windows/input.c)补齐三击及更多次数，并将 [Linux 输入](../../../sysroot/src/scoop.window/native/linux/input.c)改为消费 GTK 自身 count，移除固定 0.5 秒和 5 个物理像素的重复判断。Windows 生产计数算法的本地定向测试已通过；两个平台的原生输入用例仍待目标环境运行，不能据共享编辑行为或算法测试声明整条输入链通过。
- [公共 Drag](../../../sysroot/src/scoop.ui/src/interaction/drag.scoop)只负责一段鼠标轴向交互，没有跨应用传输和落放目标。[PointerEvent](../../../sysroot/src/scoop.window/src/input/pointer.scoop)也尚无 touch contact 或数据拖放事件。
- S1 已将 Window 系统偏好经 UI 根传给 [ThemeSource.read(scope)](../../../sysroot/src/scoop.ui.style/src/theme/source.scoop)和[文本选项](../../../sysroot/src/scoop.ui/src/text/editor/options.scoop)。caret 默认继承；四套主题消费减少动画、高对比度、减少透明度和文字缩放。已通过的离屏／实窗范围与剩余系统热切换验证分别记录，不以公共接口接入代替全部控件及设备验收。

## 2. 需要一起补齐的细节

以下内容直接影响本轮交互，应纳入实现或组合回归；已有功能以核查和修复为主。

| 方面 | 本轮要求 |
| --- | --- |
| 系统偏好 | 常显／自动隐藏滚动条、减少动画、高对比度、减少透明度、文字缩放、caret 闪烁及宽度；只采用系统实际提供的信息 |
| 键盘与焦点 | Tab／Shift+Tab、组内导航、可见焦点、弹层关闭后的恢复、菜单键／Shift+F10；滚动、调整大小和重排有可完成同一目的的键盘或命令入口 |
| 指针反馈 | 文本／链接／可拖动／不可投放等正确光标；捕获丢失与 Escape 收尾；轨道／箭头按住重复及平台允许的跳转到点击位置 |
| 文本细节 | Unicode 词与 grapheme、软换行和硬换行、RTL、非活动选区、拖选自动滚动、IME 候选定位、菜单期间选区保持；密码用途与显示遮罩分离 |
| 无 hover 的使用方式 | 足够的触摸目标、可发现的操作按钮、长按菜单与文本选区调整；不能把必要功能只放在 hover 状态 |
| 几何与动态内容 | 多显示器和 DPI 切换、RTL、窗口缩放、嵌套裁剪、虚拟列表回收、数据重排、滚动期间内容增高及阅读锚点保持 |
| 生命周期与能耗 | 菜单／拖放中禁用、卸载、模态切换、关窗；隐藏与动画完成后停帧，deadline 取消，迟到事件不会恢复旧交互 |

系统拼写／语法检查、词典／翻译、Services、自动替换、听写和触觉反馈列为能力核查项：公开 API 能在同一编辑模型上正确工作时逐项接入，不能把整个系统文本服务套件作为本轮普通编辑菜单的前置。Linux PRIMARY／中键粘贴也按实际平台能力单列，不用普通 Clipboard 冒充 PRIMARY。

本轮不扩展到移动端窗口后端、富文档编辑引擎、手写识别、通用文件管理器或自定义手势插件框架。图像数据、虚拟文件／延迟生成文件和大文件流式拖出，在首批纯文本、URL／文件列表闭环之后按真实消费者增加。

## 3. 分层与平台偏好

| 层 | 职责 |
| --- | --- |
| `scoop.window` 及 native 后端 | 原生点击计数／输入设置、系统交互与无障碍偏好、touch／原生 gesture、数据拖放会话与格式转换、原生菜单／文本服务；使用现有 owner 和 native 生命周期 |
| `scoop.surface` | 接触序列与归属、坐标转换、事件取消、最近 Accepted 场景的路由、系统会话接管；不判断某个控件的业务含义 |
| `scoop.ui` | 公共手势组合、拖放源／目标接入、滚动呈现、文本选择／编辑、scope 资源与语义动作协调 |
| `scoop.ui.component` | ScrollArea／VirtualList／Textarea 等共享策略；Input／Textarea 默认菜单、控件触摸行为及必要的替代操作 |
| `scoop.ui.style` 与主题 | 收缩／展开的几何、颜色、透明度、过渡及对比度；不持有隐藏计时器、拖放会话或可写编辑模型 |

系统偏好由 Window 读取并通过已有 owner 通知变化，UI 根解析后向实际消费者传播。未知／不可查询与明确关闭是不同值；上层不导入 AppKit、Win32 或 GTK，也不在每帧查询 OS。偏好接口不依赖 accessibility provider 已启用，普通应用启动时就应生效。

原生设置的单位在 Window 边界明确转换，包括点击距离、拖动阈值、caret 物理宽度，以及闪烁的整周期与单次切换间隔。滚轮按 OS 已应用的方向和实际行／像素单位消费，不再次反转“自然滚动”，也不把用户文字缩放当作显示器 DPI。

缺省使用 `FollowSystem`。控件显式覆盖继承根配置，根配置继承系统已知值，其余使用明确的库默认值；“主题名为 macos／fluent”不决定实际平台行为。减少动画的有效结果由根策略统一约束，普通主题的 `Full` 不应重新开启已被系统或应用关闭的位移效果。应用需要用户可选覆盖时使用明确的应用设置，不由一个皮肤静默覆盖。

已知平台依据：

- macOS 的 `NSScroller.preferredScrollerStyle` 与变化通知反映系统滚动条偏好；`NSWorkspace` 提供减少动画等显示偏好。不能把这些 API 当作能读取所有系统私有动画时长的接口。
- Windows 的 `UISettings.AutoHideScrollBars` 明确服务自定义滚动条，并提供变化通知；该属性从 Windows 10 1903 起可用，应核对项目最低版本。高对比度、客户区动画、caret、滚轮及点击阈值使用对应公开系统设置。
- Linux 基于当前 GTK 4.18／libadwaita 基线。使用已有的 `gtk-overlay-scrolling`、`gtk-enable-animations`、点击／长按／拖动阈值及 caret 设置；在线 GTK 文档中新增加的属性须先核实版本，不能直接加入基线。

没有可查询偏好时，回退到当前主题的固定条宽、溢出时常显、无闲置隐藏，caret 保留 500ms；应用仍可显式启用并配置动态行为。实际预设参数在实现批次中记录，不声称逐像素复刻系统私有控件。

## 4. 滚动条与滚动体验

### 4.1 把可见条件、交互和外观分别表达

沿既有 receiver builder 暴露下列独立维度，ScrollArea、VirtualList、Textarea 及其组合消费者保持一致：

| 维度 | 目标选项及行为 |
| --- | --- |
| 是否需要条 | 保留“有范围才显示／始终显示／完全隐藏”的区分；不继续用 `Auto` 同时指代溢出和闲置隐藏 |
| 行为预设 | 跟随系统、固定显示、显式定制；预设解析为同一套普通配置 |
| 占位 | Overlay 或固定预留 gutter；hover 和淡出不重新分配正文宽度 |
| 自动隐藏 | 可关闭；可设置闲置时长、淡出时长及重新出现的过渡 |
| 出现条件 | 滚动时、进入视口时、进入滚动条边缘时、键盘焦点／操作时；允许分别配置 |
| 悬浮宽度 | 可独立关闭；idle／hover／drag 宽度及展开／收缩时间；禁止因变宽使 thumb 抓取位置跳动 |
| 悬浮深浅 | idle／hover／drag 的颜色或透明度及过渡；可关闭，不能与宽度动画强绑定 |
| 命中 | 与绘制宽度分开的最小命中厚度、触摸目标及边缘唤醒区域；遵守 clip 和前景层次 |
| 轨道与按钮 | 轨道是否显示、分页或跳转、端部按钮、按住重复的初始延迟／间隔；默认采用可取得的平台惯例 |

减少动画时立即切换宽度／颜色并完成隐藏，不为了关闭动画而永久禁用用户明确要求的闲置隐藏。系统要求常显时，FollowSystem 不再闲置隐藏；仍只在存在范围时显示的策略与零范围禁用外观分别保留。

### 4.2 活动与隐藏

1. 有效滚动、指针进入配置区域或键盘操作使滚动条出现；hover 展开并加深，离开后回到 idle 宽度和颜色。
2. 鼠标按住、thumb 拖动、触摸滚动、native momentum、相关键盘操作期间，不启动会在交互中途隐藏目标的计时。结束后重新计算闲置 deadline。
3. 键盘／辅助技术操作滚动容器时，即使鼠标不在窗口，也能得到位置反馈。普通布局锚点修正不反复刷新闲置计时。
4. 一个控件只预约下一次实际需要的 deadline；动画由现有 Surface 时钟推进。静止隐藏后没有常驻轮询，隐藏／卸载取消排期。
5. 完全隐藏的条不能用透明输入区域截走正文点击。边缘 hover 唤醒可以观察指针而不消费按下；重新出现的可操作几何仍须等待 Accepted。
6. thumb 拖动保持按下点相对 thumb 的比例；尺寸、内容范围、DPI 或主题变化时重算合法几何，目标失效按原取消通路结束。

Overlay 条覆盖正文边缘时，caret／焦点自动显露应计入实际遮挡；固定 gutter 的布局在 idle／hover／hidden 之间保持稳定。RTL 的垂直条位置及双轴角部仍沿现有方向规则。

### 4.3 回弹、惯性与嵌套滚动

滚动回弹是可关闭、可定制的呈现策略。提供跟随平台、硬边界和弹性边界的选择，弹性阻力、最大视觉位移、释放 spring 与惯性减速使用有限的正常物理参数；不另建动画循环。

逻辑 offset 始终位于合法范围；越界位移作为独立呈现值，与同一帧的输入、caret、锚点和 accessibility 几何一起变换。Scroll range、虚拟列表索引和阅读锚点不写入虚构的负偏移或超范围值。没有可滚内容时缺省不回弹，可显式开启短内容回弹。

嵌套滚动先按 Chain／Contain 处理实际残量。Chain 优先交给仍可消费的祖先，最终边界才允许一个容器表现回弹；Contain 留在当前容器。不让同一余量既驱动父级滚动又使子级回弹，也不把已经吸收的视觉位移再次交给父级。

OS 已提供 momentum 的滚轮／触控板流直接消费，不叠加自制惯性。软件惯性只用于没有原生惯性的直接触摸 pan；传统离散鼠标滚轮缺省不附加惯性。新触摸、拖动、显式定位或取消中断旧运动，停止时回到合法位置。减少动画时禁用额外回弹、模拟惯性和装饰性平滑定位，保留直接输入产生的真实滚动。

## 5. 文本选择与原生上下文菜单

### 5.1 多击选择

保持现有单击定位、Shift 扩选、双击词和三击逻辑行合同。逻辑行包含对应硬换行；软换行产生的视觉行不自动等同逻辑行。双击／三击后继续拖动分别按词／逻辑行扩选，保留方向；四击及更高连续计数沿现有三击单位处理，直到序列重置。

原生计数可用时直接使用；需要补足时，Window 按系统时间和距离阈值、窗口／按钮／设备及取消状态计数。Windows 要补三击，Linux 优先消费 GTK 提供的 count；真实拖动、取消、超时或超距离使旧多击序列结束。不能在每个 TextEditHost 重新实现平台计数，也不能只向测试注入 count=3 就宣称原生输入通过。

回归覆盖中文、空格／标点、emoji ZWJ、组合字符、RTL、空行、行尾、自动换行、显示投影、单行／多行、只读选择，以及越界自动滚动。文本单位始终来自 graphics.text；不增加按 ASCII 空格分词的替代实现。

### 5.2 菜单模式与系统能力

文本宿主提供关闭、标准原生菜单、优先系统默认菜单三种模式，内置文本框缺省选择“优先系统默认菜单”。没有可正确复用的系统默认菜单时，落到同一套标准原生菜单；应用关闭内建入口后可使用现有 `nativeContextMenu／contextMenu` 定制，避免同时弹出两份菜单。

标准原生菜单通过现有 Menu／PopupSession 和编辑命令实现，至少含平台适用的 Undo、Redo、Cut、Copy、Paste、Delete、Select All；顺序、标签、快捷键和分组遵从平台惯例。S3 已补独立的 DeleteSelection 命令及可用性，空选区不删除相邻字符；实际目标验证见实施记录。菜单动作与键盘使用同一编辑入口和最新状态，禁止只通过制造按键完成操作。

“系统默认菜单”指由原生文本服务实际提供并能正确连接当前文档的项目。AppKit 的 NSTextView 菜单定制入口、Win32 EDIT 的内建菜单及 GTK TextView 的 extra-menu 都依附各自文本控件；当前 Scoop 内容是自绘宿主，不能仅因能创建 NSMenu／HMENU／GMenuModel 就声称获得了完整默认菜单。

S0.3 本地原型已确认 AppKit 公开的 `NSTextView.defaultMenu` 可以在不创建文本控件时返回系统模板；当前系统模板含 Cut／Copy／Paste 等项，也含依赖原生文档与编辑服务的富文本、拼写、替换、转换和排版动作。模板存在不代表后者已接入 Scoop。实施须逐项匹配真实能力，保留系统可用的标签与结构，不能把所有项目直接挂到自绘 view 后声称拥有同样行为。

先用原型逐项验证可共享的公开服务。不能为了取菜单创建另一份隐藏可编辑文档、第二个 IME 宿主或撤销栈；允许的只读／短期原生投影仍以当前编辑模型和 revision 为准。无法保持相同动作含义的服务不显示，能力和回退在实现记录中明确。平台没有提供独立公共入口，不代表菜单无法做成原生外观，也不应阻塞标准菜单交付。

### 5.3 目标、选区与编辑一致性

- 右键、macOS 平台认可的辅助点击、菜单键／Shift+F10，以及触摸长按进入同一上下文菜单请求。键盘定位使用已提交 caret／选区锚点，不能用过期坐标或窗口原点代替。
- 右键在当前选区内时保留选区；选区外的定位／选词按平台惯例解析，并允许调用方选择保留或定位。平台无明确策略时回退到在点击位置放置 caret。打开菜单本身不剪切、不输入，也不重置撤销历史。
- 未聚焦文本目标先按正常规则取得焦点；当前 composition 的结束／保留沿同一编辑合同处理。菜单 tracking 不被误判成切换到另一编辑文档，候选框仍只有一个原生连接。
- 菜单请求保存发起宿主和实际会话。执行时该宿主必须仍存在、启用、可输入且为合格编辑目标；用户已切换到另一个编辑器时，旧菜单取消或拒绝动作，不重定向到新焦点文档。
- 只读文本保留 Copy／Select All 等实际能力；disabled 不执行动作。Undo／Redo、选区、复制策略及剪贴板变化正确影响项目可用性，执行时仍检查实际条件。不能展示看似启用却永远无实现的系统服务。
- 外部文本导入、原生菜单、辅助技术和拖放都经过同一编辑策略与历史。菜单粘贴继续使用 Paste 来源；只有确需区分的新来源才扩展 TextEditSource。

明确的受保护文本用途独立于 `textMask`：切换明文显示不自动解除 accessibility 保护，也不自动开放向外的数据拖放或系统文本服务。保持现有复制策略的明确选择；Copy Allow 不等于允许 AX／UIA／AT-SPI 返回明文。受保护控件缺省不允许拖出原文，允许按正常策略粘贴／拖入。数字、邮件等输入用途和软键盘提示按平台能力提供，字符合法性仍由编辑策略判断。

## 6. 数据拖放

### 6.1 同一会话覆盖三个范围

| 范围 | 路径 |
| --- | --- |
| 窗口内 | 公共 source／target、当前场景命中、同一 effect 协商和完成规则；支持实际条目重排 |
| 同程序跨窗口 | 默认进入原生拖放，由接收窗口自己的 UiRoot 路由；不扫描其他窗口的挂载树或转发伪造鼠标事件 |
| 跨程序 | 同一原生会话发布／消费标准格式，区分源与接收方结果；外部接收方不需要 Scoop 语义树 |

一次拖动只有一个活动会话。允许离开窗口的 source 从起始选择原生通路，不能离窗后再启动一份竞争会话；仅窗口内使用的重排可以使用相同公共合同的本地路由。跨窗口的拖放会话与源窗口的普通鼠标 capture 分离。

原生后端分别使用 AppKit dragging session／destination、Windows OLE `IDataObject／IDropSource／IDropTarget`、GTK／GDK drag source／drop target。低层值描述来源、可用格式、允许的 Copy／Move／Link、建议动作、预览与实际结果；Link 只有业务实现其含义时才公布，普通文本不虚构 Link 行为。

首批标准内容为纯文本、URL 和真实文件列表；格式协商允许一个来源提供多个表示。Window 做实际 UTType／clipboard format／MIME 转换，不将进程内对象地址编码成跨进程数据。当前本地重排需要的条目 key 可以由 source 的正常会话上下文持有，不建立通用对象序列化或跨进程对象桥。

### 6.2 路由、读取与完成

1. 达到拖动阈值前仍是点击／选区候选；接管后取消冲突的 Press、文本拖选或滚动候选，松开时不额外点击。
2. Enter／Over 只查看格式、位置、修饰键与当前业务接受条件，发布可投放高亮和 effect；不在每次 move 读取文件或全文，也不修改业务集合。
3. 目标优先级遵循最近 Accepted 场景的前景命中、逻辑路由和模态边界。子目标拒绝时可沿明确的父目标链尝试，不穿透到视觉后方无关兄弟。
4. Drop 后才取得所选表示，按平台实际需要异步读取。数据未到、业务未接受时结果是处理中；处理成功后才报告相应完成。只接受 Copy 的目标不能显示 Move 光标。
5. Copy 成功不删除源；Move 仅在目标实际接受、原生协议确认对应结果且源仍能完成相应删除时，由源删除一次。失败、Escape、拒绝或取消均保留源。已经接受数据后才发现来源冲突或删除失败时，报告实际的“目标已接收、源未删除”，不能假报完整移动或删除新的来源内容；外部进程的内部持久化不属于库可承诺的原子事务。
6. 来源／目标删除、禁用、模态限制或窗口关闭，取消相关会话／读取并清除高亮、caret、preview、deadline 和临时引用；完成／取消只有一个终态。系统会话跨应用造成源窗口失活时，普通 capture 可以结束，不能仅因此取消合法的跨程序拖放。

跨窗口与外部拖放不能依靠全局屏幕坐标推算 target，特别是 Wayland。OS 给出目标窗口中的事件位置，各根只转换自己的逻辑／物理坐标。预览及不可投放光标遵守 OS ownership；原生 preview 与本地 preview 不重复绘制。

原生拖放可能进入 tracking loop 或同步调用 target。沿已有 deferred native operation／owner 控制通路启动，不能在 render／layout 内进入系统拖放循环。同步请求需要实际结果时，复用 accessibility 设计中的有限 owner 派发边界，分别规定查询、动作和等待；不允许 owner 等待自己的队列，不在持锁时等 managed 回调，也不另建事件循环。原生回调数据的保留使用既有 ForeignCallback／handle 与 native 所有权。

### 6.3 文本拖放

文本宿主提供独立的拖出、接收开关和允许的 effect。普通可编辑 Input／Textarea 缺省启用拖出与接收；只读文本和 SelectableText 只允许按复制策略拖出 Copy，受保护文本缺省关闭拖出。应用可以分别关闭两端，不能为了禁用拖放而禁用普通拖选或文本编辑。启用状态更新立即作用于当前候选和落放资格。

在既有非空选区中按下时先保留选区，达到拖动阈值后按启用配置发起文本数据拖动；未成为拖放的点击在释放时完成普通定位。选区外仍正常拖选；Shift 扩选、双击／三击单位选择、只读及遮罩行为不能被新的 source 抢走。

目标显示独立的插入 caret，普通选区在 Drop 被接受前保持。靠近边缘时复用已有自动滚动与残量链；滚动后等 Accepted 更新落点，不能把旧局部坐标套在新的正文上。虚拟项以 key 定位，只保留正在操作的必要项，不为所有可能目标创建 View。

文本落放经正式编辑事务执行：

- 只读来源可按复制策略拖出 Copy，不发布 Move；只读目标不接受文本插入。受保护用途继续执行第 5.3 节的限制。
- 同一文档 Move 在一次编辑中完成源删除、落点偏移校正、插入和最终选区，并作为一个 undo 项；落在原选区内或不产生位置变化时无操作。
- Copy／外部拖入是一次插入和一个 undo 项。同程序不同文档的 Move 分别进入两个文档各自历史，不承诺一次 Undo 跨文档回滚。
- 拖动开始记录来源文档、范围及 revision；正文变化后不能凭旧 offset 删除另一段文字。首批对来源正文变化撤销 Move 资格，完成前仍可协商时改为 Copy 或拒绝；删除前再确认实际来源，已经无法改变外部协议结果时按第 6.2 节报告未完成移动并保留源。单纯选区变化不改变已捕获的来源文本。
- Drop 时重读当前目标与策略，处理 composition 后重新确认落点范围。单行归一化、长度、字符与应用 Reject／Transform 均适用；拒绝不修改文档或历史，Move 不删除来源。
- 目标只接受部分文字或策略变换会丢失来源内容时，不能以完整 Move 成功删除整段来源；在提交前协商 Copy 或拒绝，并使完成结果与实际选择一致。同文档移动由目标事务完成源删除，结束回调不得再删一次。
- 增加可辨认的 DragDrop 编辑来源；缺省复用外来文本粘贴的清洗逻辑，具体策略入口在文本 spec 中统一修订，不能因不是 Paste 来源而意外绕过已有导入限制。
- 数据或回调延迟期间，目标若卸载、换文档或正文发生使落点失效的变化，取消该次落放；不向新的焦点或新复用的列表条目插入。

文件列表拖入普通文本框不自动读取文件内容；应用明确选择插入 URL／路径或交给上传、打开等业务目标。导入成功与文件业务处理成功分别由其实际入口决定。

## 7. 触摸输入与手势

### 7.1 输入事实

保留鼠标、直接触摸和笔的来源区别，触摸提供 contact 身份、开始／移动／结束／取消、时间与窗口坐标；压力等数据仅在系统实际提供时出现。鼠标现有聚合按钮模型不伪造成 touch contact。设备身份未知时保持未知，contact 身份只保证其实际接触生命周期的路由。

触控板提供的原生滚动、缩放、旋转／swipe 与直接触屏不同。macOS 触控板事件不能当作 Windows／Linux 触屏已验收；Window 转交原生 gesture，UI 只对原始触摸缺少的识别进行组合。

保留 native 与合成鼠标的关联，避免同一接触重复触发按钮、菜单或拖动。Windows OLE 对触摸／笔的启动有特殊要求：官方要求从其合成的鼠标消息处理路径发起 DoDragDrop；后端必须保留必要的原生启动信息，同时只向 UI 交付一次语义交互，不能为去重把这条原生启动路径也删掉。

### 7.2 必要手势与归属

| 手势／消费者 | 必须明确的行为 |
| --- | --- |
| Tap／多次 tap | 正常 Press 和多击选择；手指开始滚动后不再激活原按钮 |
| Long press | 进入配置的上下文菜单／选区操作；移动超阈值、多指接管、取消或卸载时撤销 timer；长按后不再补一次 tap |
| Pan／scroll | 一指直接滚动、方向锁定、嵌套残量、释放惯性；文本、Slider、Drawer／Carousel 等按实际轴向和起始区域竞争 |
| Pinch／rotate | 公共手势事件和普通图像／内容变换消费者；支持中心点、累计及增量值、开始／结束／取消；不隐式缩放全应用 |
| Swipe | 作为有限方向动作或 pan 释放结果；Drawer／Carousel 复用已有状态，不把 OS 保留的全局边缘手势强行截走 |
| 文本选区 | 长按选词、可触达的选区端点调整、caret 定位和边缘滚动；复用现有文档范围与布局映射 |

每个 contact／相关 contact 组在一个时刻有明确归属。沿现有输入路径登记候选，越过系统／显式阈值后接管；失败的候选收到取消。只有明确声明可组合的操作才同时进行，例如内容变换的 pinch＋rotate；不为所有控件开放任意识别器优先级插件。

第二根手指到来、触点数量改变、几何失效、捕获丢失和系统接管均有明确转换与取消出口。减少动画不禁用手势本身，只收束释放后的额外运动。

触摸选区把手和必要操作不能依赖 hover。可触达命中区域可以大于绘制部件，但不得穿过父 clip、覆盖无关目标或改变阅读顺序。软键盘出现后沿平台提供的视口／遮挡信息显露活动编辑器；不伪造未知键盘高度。无该接口的平台记录限制，仍保留正常 IME 与键盘输入。

## 8. accessibility 同步

保留现有的一份语义树、真实动作入口、文本模型和按 key 虚拟化设计，不新增独立的 agent／拖放语义树。同步修订的具体合同见[accessibility 设计](../accessibility/DESIGN.md)：

- 平台偏好、键盘操作、对比度、文字缩放及非 hover 命中能力，与 provider 建设同时推进；不能在 role／label 接通后才处理。
- 自动隐藏滚动条不使滚动容器的范围、Scroll／Reveal 动作失效；弹性位移不改变语义范围，位置查询使用同一 Accepted 变换。
- ShowContextMenu 等实际动作可以通过辅助技术打开对应原生菜单；原生菜单由原生 provider 报告，UI 不重复导出其条目。
- 拖动重排、调整范围及缩放等能力提供具体的键盘／命令替代入口，并映射到适用语义动作。跨程序拖放继续使用原生数据协议，不能假设两边共享辅助技术模型。
- 双击／三击、触摸选区、菜单编辑和 Drop 都更新同一选区、编辑历史及文本范围。文本／selection 通知在结果发布后发送，拖动 hover 和回弹的每一帧不触发重复播报。
- 受保护用途与显示投影分离；拖放 preview、原生菜单服务及语义文本访问不能意外泄露原文。

## 9. 设计交付分组

以下分组覆盖文档、实现、实际消费者和必要验证，均待实施，不预先占用编译器里程碑编号。具体执行顺序与任务状态由[统一实施计划](PLAN.md#3-批次与依赖)安排。

| 批次 | 交付 | 完成条件 |
| --- | --- | --- |
| U0：平台事实与边界 | 系统偏好合同、点击计数修复；系统文本菜单及原生拖放 owner 原型；与 accessibility P0 共享必要边界核查 | 三平台真实点击流、设置来源与版本明确；菜单／拖放同步回调、取消与关窗可安全完成 |
| U1：文本交互 | 多击组合回归、文本原生菜单选项、Delete selection、目标／选区保持 | Input／Textarea／SelectableText 使用相同规则；键盘和原生菜单产生一致编辑结果 |
| U2：滚动条 | 平台预设、独立配置、隐藏／hover 动画、gutter、命中及重复操作 | ScrollArea／VirtualList／Textarea 行为一致；系统常显与减少动画可热切换，静止停帧 |
| U3：鼠标数据拖放 | 公共 source／target、标准数据、窗口内／跨窗口／跨程序、文本拖放 | 三个范围的 Copy／Move、失败不删源、撤销、取消、跨 DPI 与虚拟项组合通过 |
| U4：触摸与手势 | contact 路由、tap／long press／pan／pinch／rotate／swipe、文本端点与控件接入 | 真触屏与触控板分别验证；无重复合成输入，滚动不误点，竞争与取消完整 |
| U5：滚动物理与组合 | 回弹、必要的软件惯性、系统设置、完整视觉／键盘／辅助技术组合 | 原生 momentum 不叠加，嵌套只消费一次，逻辑范围正确，低运动设置生效且结束后停帧 |

公共偏好、点击修复和标准原生菜单可按各自已具备的接口先交付；虚拟化、完整文本访问和三平台 provider 在计划中各有明确任务。accessibility 从 S0／S2 开始，随对应交互接入；具体依赖及尚需合并验证的组合以计划为准。

## 10. 规范与材料同步

| 文档 | 实施前需写入的合同 |
| --- | --- |
| [Window spec](../../specs/SCOOP-WINDOW-SPEC.md) | 点击计数、偏好、touch／gesture、数据传输生命周期、线程与同步返回、目标平台范围 |
| [Surface spec](../../specs/SCOOP-SURFACE-SPEC.md) | 多 contact 路由／取消、原生会话接管、Accepted 命中及必要的同步查询 |
| [UI spec](../../specs/SCOOP-UI-SPEC.md) | 公共拖放／手势、文本事务与选择、受保护用途、滚动呈现、生命周期；更新已交付范围声明 |
| [Component spec](../../specs/SCOOP-UI-COMPONENT-SPEC.md) | 滚动条和文本选项、现有控件接入、行为默认值及键盘替代入口 |
| [Theme spec](../../specs/SCOOP-UI-THEME-SPEC.md)与[Style spec](../../specs/SCOOP-UI-STYLE-SPEC.md) | 状态几何／配色、系统偏好解析结果、动态效果及对比度 |
| [Window 菜单 spec](../../specs/SCOOP-WINDOW-MENUS-SPEC.md)与[UI 原生菜单 spec](../../specs/SCOOP-UI-NATIVE-MENUS-SPEC.md) | 文本菜单模式、Delete selection、目标保持、平台服务及回退 |
| [文本 spec](../../specs/SCOOP-GRAPHICS-TEXT-SPEC.md) | 仅补实际缺少的单位、范围保持或几何合同，既有 Unicode／排版算法继续复用 |

应用可用写法变化时按[编程材料更新工作流](../../scoop-programming/UPDATE-WORKFLOW.md)同步声明、builder、现有消费者、示例和 skill。设计阶段不把拟议调用写入可运行示例或生成 API 文档。语言、compiler pipeline 和 runtime 对象模型没有本任务所需的变更，不安排相应重构。

## 11. 验收

纯算法、策略解析、计时状态、文本事务和范围映射使用所属 Cone 测试。真实菜单、设备、原生拖放、外部进程和 provider 使用现有实窗／外部集成测试，统一 release，复用预编译系统库和共享 cache。文档核查或模型单测不能替代 native 验收。

| 组合 | 必须观察到的结果 |
| --- | --- |
| 滚动条 | ScrollArea／VirtualList／Textarea 共用行为配置、系统偏好、hover 宽度与深浅、闲置隐藏、独立命中条带和重复操作 | 当前验证见实施记录；统一 Overlay／gutter、遮挡显露、滚动语义及实窗／平台验收待继续 |
| 输入与运动 | 传统鼠标轮、触控板含 momentum、直接触摸、键盘、语义滚动；新输入中断旧运动，嵌套不双重消费或双重回弹 |
| 多击 | OS 真实连续点击抵达共享编辑器；系统阈值变化、二／三击继续拖选、Unicode／换行／投影、失焦与取消 |
| 文本菜单 | 选区内外右键、辅助点击、键盘、长按；只读／密码／禁用、Undo／Redo／Delete、剪贴板变更、菜单期间 composition、目标切换及关闭 |
| 拖放范围 | 同一窗口、同程序两窗口、独立进程，双向导入／导出；Windows 同时覆盖实际外部 OLE，Linux 分别覆盖 Wayland／X11；不能用两个本地 source／target 代替跨程序 |
| 拖放结果 | Copy／Move／拒绝／Escape、格式不支持、异步读取失败、源正文改变、目标换文档、同文档落回原选区、过滤与 undo；失败不删源、不重复插入 |
| 原生重入 | tracking 期间同程序目标可处理 Drop，owner 不等自己；真实关窗／退出清理完成；迟到回调不使用已释放对象 |
| 触摸 | 真单／多指、长按撤销、pan 与 Press／Slider／文本冲突、触点增减、系统取消、合成鼠标去重、非 hover 入口与软键盘 |
| 几何 | 跨 DPI／显示器、主题／字号变化、Portal／模态、虚拟项回收／重排、Accepted／Discarded；位置反馈与实际输入、语义查询一致 |
| accessibility | 对应平台 VoiceOver／Narrator／Orca 与 AX／UIA／AT-SPI 客户端能读、操作、观察结果；键盘完成同等任务，滚动条隐藏不移除滚动能力 |
| 能耗 | 等待下一次 deadline 时不持续刷帧；隐藏、取消、卸载和回弹结束后没有多余唤醒；动画不重排全文或遍历全部虚拟项 |

建议共享一个交互展示场景：可配置滚动条的长文／虚拟列表、单行与多行编辑、可重排条目、第二窗口和文件／文本接收区；复用当前控件，不建立新的展示框架。用独立原生编辑器与文件管理器验证外部互操作，实际有触屏的设备验证直接触摸。macOS 触控板、Rosetta、注入事件或单一平台通过均不能代替其他设备／平台结果。

## 12. 依据与核查边界

设计初稿阶段读取规范与上述源码，核查本机 AppKit SDK、官方文档及 GTK 4.18.6 设置源码；该阶段没有修改运行代码或运行新的实窗、触屏、拖放及辅助技术验收。后续实现与验证以 [PROGRESS.md](PROGRESS.md) 为准。

- Apple：[NSScroller.preferredScrollerStyle](https://developer.apple.com/documentation/appkit/nsscroller/preferredscrollerstyle)、[减少动画](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayshouldreducemotion)、[NSTextView 菜单定制](https://developer.apple.com/documentation/appkit/nstextviewdelegate/textview(_:menu:for:at:))、[触控板事件](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/HandlingTouchEvents/HandlingTouchEvents.html)。本机 NSView SDK 同时核实既有 `beginDraggingSessionWithItems:event:source:`；不使用新 SDK 才增加的手势 overload 作为最低平台前提。
- Microsoft：[自绘滚动条的系统偏好](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings.autohidescrollbars?view=winrt-26100)、[系统设置](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow)、[EDIT 文本操作与默认菜单](https://learn.microsoft.com/en-us/windows/win32/controls/edit-controls-text-operations)、[DoDragDrop 及触摸启动限制](https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-dodragdrop)、[WM_POINTERDOWN](https://learn.microsoft.com/en-us/windows/win32/inputmsg/wm-pointerdown)。
- GTK：[4.18.6 设置源码](https://github.com/GNOME/gtk/blob/4.18.6/gtk/gtksettings.c)、[overlay scrolling](https://docs.gtk.org/gtk4/property.ScrolledWindow.overlay-scrolling.html)、[TextView extra-menu](https://docs.gtk.org/gtk4/property.TextView.extra-menu.html)、[DragSource](https://docs.gtk.org/gtk4/class.DragSource.html)、[DropTargetAsync](https://docs.gtk.org/gtk4/class.DropTargetAsync.html)、[Gesture 序列归属](https://docs.gtk.org/gtk4/class.Gesture.html)。在线文档核查时为 4.24.1，具体新增 API 仍须与 4.18 基线逐项对照。
