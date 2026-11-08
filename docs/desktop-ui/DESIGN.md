# 原生桌面能力与 UI 组件设计

日期：2026-11-07。状态：按批次实施中。下文描述完整目标，已实现的 API 以所属规范为准；各批次完成与平台验证状态见[实施记录](PROGRESS.md)，不能将尚待实施的类型、方法和组件视为现有能力。

本设计覆盖 `scoop.window` 的 native menubar、popup menu、窗口外观与自绘窗框、窗口归属与模态、原生文件对话框，以及独立 `scoop.tray` Cone 的托盘能力。UI 基础与通用组件沿用 `scoop.ui`／`scoop.ui.component`，托盘 UI 包装另放 `scoop.ui.component.tray`。实施时按批次先更新所属规范，再修改实现、调用方与测试，不提前把目标行为写成当前实现事实。

窗口关系、一次 raise／持续 topmost、模态范围、原生打开／保存文件面板及对应控制器的详细合同见配套的[窗口归属、模态与原生文件对话框设计](WINDOWS-AND-DIALOGS.md)。下文统一记录分层、资源、实施依赖与验收边界。

后续的 Skia SVG／Lucide、系统与临时字体选择、Taskbar／Dock 原生功能、桌面通知，以及 Windows 圆形示例修复见[GUI 扩展设计](GUI-EXTENSIONS.md)。该文为待实施设计，不改变本文已有批次的交付状态。

依据：[Window 规范](../specs/SCOOP-WINDOW-SPEC.md)、[Surface 规范](../specs/SCOOP-SURFACE-SPEC.md)、[UI 规范](../specs/SCOOP-UI-SPEC.md)、[组件规范](../specs/SCOOP-UI-COMPONENT-SPEC.md)、[API 风格](../specs/SCOOP-API-STYLE.md)。UI 的布局、构造、失效和资源归属沿用已经完成的[整改合同](../ui/REMEDIATION-PLAN.md)，尤其是完整布局结果、按需更新和 Accepted 几何发布边界。

## 1. 目标、范围与现状

本次设计解决六组实际使用场景：

1. 应用使用原生菜单栏、右键菜单和按钮触发的 popup menu；支持子菜单、勾选、单选、禁用、快捷键和标准操作。
2. 托盘应用在没有业务窗口时仍可更新图标与菜单、响应激活、创建新窗口，并有序退出。
3. 窗口使用系统窗框、内容延伸进标题栏，或者完全自绘标题栏与边框；系统继续负责其实际支持的移动、缩放和窗口操作。
4. 窗口可呈现透明、圆角或任意轮廓；调用方能区分视觉形状、原生输入区域与窗框交互，并明确判断当前后端能否满足要求。
5. 父窗口打开原生模态确认窗口，按 Owner／Application 范围限制输入；非模态工具窗口保持在 owner 上方，一次提到前面与持续全局置顶分别表达。
6. 使用系统打开／保存文件及选择目录面板，获得类型化路径结果，并正确处理嵌套模态、取消、owner 关闭和无窗口托盘应用。

本轮覆盖现有 Darwin aarch64／amd64、三个 Windows x86_64 target，以及 `x86_64-unknown-linux-gnu` 的 Wayland／X11 后端。Linux 窗口、Surface 与 UI 已合入，新增能力直接扩展 GTK 4.18+／libadwaita 1.4+ 宿主和 GtkGLArea，不重新建立 XCB／libdecor 窗口后端。Linux musl 不包含当前 GUI 资源。支持的 OS／SDK 基线沿现有发行配置和 Window 规范。

Linux 当前交付与验证边界见 [Window 规范第 12 节](../specs/SCOOP-WINDOW-SPEC.md#12-linux-wayland-与-x11-后端)和 [Linux GUI 实施记录](../linux-gui/PROGRESS.md)。现有 X11／Wayland 验证证明窗口与 UI 基础可复用，新增菜单、托盘、模态、文件面板和自绘窗框仍需各自验收；Wayland 的全局定位／置顶限制也不会因接入 GTK 而消失。

本次不扩展为一般原生 widget 工具箱，不新增通用 effect 系统、平台插件注册框架、独立任务树或事件循环。原生对话框首批覆盖文件／目录面板，以及承载 UI 内容的真实模态窗口；不顺带加入所有系统消息框、任意 Shell 虚拟对象或嵌入式 native child 控件。完整 accessibility 桥接与显示模式切换仍按各自需求实施。

### 1.1 已有基础与缺口

| 部分 | 当前事实 | 本设计需要补齐 |
| --- | --- | --- |
| 应用宿主 | 一次 Application／run、owner 线程、业务与控制队列、proxy、asyncio 附着与有序关闭 | 菜单、托盘和 popup／modal／file dialog session 的归属、事件与收尾 |
| 窗口属性 | decorations、resizable、transparent、按钮、层级；macOS 标题栏透明／隐藏标题／full-size-content | 明确的窗框模式、系统交互、原生区域、能力与实际窗框几何 |
| 多窗口 | 独立非模态顶层窗口、WindowLevel 与 requestFocus | 原生 owned 关系、Tool 角色、一次 raise、模态范围及依赖子窗口的销毁顺序 |
| 系统文件面板 | 尚无公开入口；已有原生嵌套循环接入和 Path／OsString | 打开、保存、目录选择，类型化 session、取消与原生编辑命令路由 |
| Linux GUI | GTK4／libadwaita 的 Wayland／X11、多窗口、输入／IME／剪贴板，GtkGLArea／Skia 与 UI 已接入 | GTK 原生菜单、关系／模态、文件面板、自绘区域；独立托盘 Cone 的 SNI／DBusMenu 接入 |
| Windows 无装饰 | decorations=false 使用 WS_POPUP；resizable=true 仍使用 WS_THICKFRAME | 在保留系统缩放行为时控制可见窗框，不能只翻转两个布尔值 |
| 透明呈现 | Metal／DirectComposition 已有预乘 alpha 呈现路径和透明合成测试 | 运行中外观同步、形状与输入区域的独立合同 |
| UI 更新 | State、Props、Element.update、scope 资源归属；逻辑更新不依赖 drawable | 应用级原生挂载、原生回调归属、原生附属声明 |
| 自绘菜单 | DropdownMenu／ContextMenu／Menubar 支持 View、样式、受控关闭及 Portal | 原生菜单的受约束模型与组件，不继承无法实现的自绘保证 |

源码依据：[窗口样式](../../sysroot/src/scoop.window/native/windows/window.c)、[AppKit 外观](../../sysroot/src/scoop.window/native/darwin/properties.m)、[UI 宿主](../../sysroot/src/scoop.ui/src/host/host.scoop)、[状态观察](../../sysroot/src/scoop.ui/src/state/observation.scoop)、[现有菜单值](../../sysroot/src/scoop.ui.component/src/menus/values.scoop)。

Windows 的 Window／Surface 后端已存在，但上层控件基础还有独立的[Windows 交接项](../ui/component-foundations/PLATFORM.md)。新增组件的 Windows 验收必须以这些实际依赖完成为前提，不能把 Win32 或 D3D12 已存在等同于整个 UI 依赖闭包已经可用。

## 2. 分层与基本决定

| 层 | 公共职责 | 平台相关实现 |
| --- | --- | --- |
| `scoop.window` | 菜单、窗框配置、系统操作、区域；窗口关系、模态与文件对话框；能力、事件与资源关闭 | AppKit、Win32／DWM／Shell；GTK4／libadwaita／GDK 的 Wayland／X11 后端 |
| `scoop.tray` | TrayIcon、托盘能力／事件、菜单使用、图标锚点与邻近窗口定位 | 独立 native 源码和系统依赖；AppKit StatusItem、Windows Shell、Linux SNI／DBusMenu |
| `scoop.window.asyncio` | 菜单、拖动、模态及文件面板原生循环中的 I/O 推进；session 等待、取消与退出组合 | 复用已有 watch、Job、唤醒和 native 命令执行边界 |
| `scoop.graphics`／`scoop.surface` | 形状、alpha、绘图；已提交布局到原生区域的数据转换；呈现与区域发布 | Metal／DirectComposition／GtkGLArea 与当前 Skia renderer |
| `scoop.ui` | State／Props 更新、应用与窗口挂载、原生附属声明、回调失败归属、清理 | 不导入 AppKit／Win32 API，不维护 OS 键位或装饰规则 |
| `scoop.ui.component` | 原生菜单组件、WindowChrome、区域组件；工具／模态窗口与文件面板控制器 | 只消费公共能力和结果，不按 OS 名称选择行为 |
| `scoop.ui.component.tray` | nativeTrayIcon 及其菜单／图像／事件包装 | 显式依赖 scoop.tray 与 scoop.ui.component；通用 UI 不反向依赖它 |

核心决定：

- `window` 不依赖 UI、Skia 或 asyncio，不接收 View／State／Binding／GPU Image。UI 提交普通值，native 返回类型化事件。
- 一次 Application 使用一套 owner、主循环与退出协议。原生菜单、托盘、窗框及对话框操作复用系统循环和现有接入，不创建独立 GUI 宿主。
- 能力、请求、实际状态分开表达。可选降级必须来自调用方明确选择，不能静默丢弃不支持的参数。
- 自绘菜单与原生菜单分别保留各自完整合同；实际相同的 key、Binding、动作和快捷键语义可以复用。
- 项目未发布，允许直接演进 decorations 等旧接口并迁移调用方，不保留新旧模式并行的兼容层。

### 2.1 托盘 Cone 与依赖边界

`scoop.tray` 单向依赖 `scoop.window`，复用 Application、Menu、桌面坐标和显示器值；`scoop.window` 不声明托盘依赖，不包含 TrayIcon／TrayEvent／TrayCapabilities，也不为托盘增加系统链接项。Linux 托盘可能需要的额外 native 库、头文件、运行依赖与准备脚本归 scoop.tray，不能进入普通窗口的依赖闭包。

托盘 UI 放在独立的 `scoop.ui.component.tray` Cone。应用只使用窗口、菜单或通用组件时不带入 scoop.tray；需要托盘时显式声明该 Cone 或托盘组件 Cone。普通组件库不能通过重新导出 nativeTrayIcon 间接恢复这条依赖。

菜单模型仍由 window 提供。拟在 `scoop.window.native` 增加窄的 `NativeMenuLease`：登记一个 NativeMenuUseId、借出对应平台的菜单句柄／模型、报告 tracking 起止与菜单激活，并保活该原生使用。托盘只能借用，不能销毁 Menu 拥有的 NSMenu／HMENU／GMenuModel。window 事件只识别 NativeMenuUseId，托盘层再把它关联到 TrayIcon，不在 window 的枚举中引用托盘类型。

借用时同时登记取消／释放通知。Menu 被显式关闭时，由使用方结束自己的 tracking、解除绑定并归还 lease；不能让 window 直接调用托盘私有实现，也不能无限保留一份已经关闭的菜单。通知与完成沿 owner 关闭控制边界执行。

托盘还需要同一 owner 上的应用资源登记：收到应用关闭通知、保活原生宿主、经既有队列派发业务与关闭控制，并在自身原生使用结束后解除登记。这个拟新增的 `scoop.window.native` 入口服务独立托盘 Cone；复用现有 mailbox／ForeignCallback／关闭协议，不另建事件总线或任务系统，也不占用 asyncio 已使用的单一 ApplicationAttachment。

普通托盘事件由 scoop.tray 的类型化订阅交付，原生回调只排队；关闭时即使业务队列已停止，也必须经控制通路取消 tracking、撤销注册并解除菜单／应用使用。以上跨 Cone 入口在 D1／D2 与实际托盘调用方一起定稿，不把私有 owner 结构或内部 C 符号当作公开 ABI。

## 3. 能力、请求与实际状态

### 3.1 查询对象

拟新增 `ActiveApplication.desktopCapabilities()` 与 `Window.capabilities()`，返回查询时自有快照。前者描述菜单栏、菜单及文件对话框服务；后者描述窗框、系统交互、外观、区域，以及关系／模态的有效组合。托盘查询单独由 `scoop.tray.trayCapabilities(application)`／`TrayIcon.capabilities()` 提供。它们是正常外部查询，不放在每帧遍历中调用。

能力按实际操作分组，使用具体 enum／值字段，不提供字符串 feature 注册或任意属性字典：

| 分组 | 必须能区分的内容 |
| --- | --- |
| 菜单栏 | 应用全局菜单栏／每窗口菜单栏；实际可用的附着目标 |
| 菜单 | 支持的条目与角色、图标形式、可转换的快捷键；tracking 期间的更新限制 |
| 托盘（scoop.tray） | 注册入口、模板图标、激活／上下文菜单事件；图标矩形查询、事件定位提示、宿主显示菜单与程序化显示菜单分别表达 |
| 桌面定位（window） | 目标显示器工作区、系统条的实际可查询范围及普通顶层窗口的全局定位；scoop.tray 据此判断图标邻近定位是否可用 |
| 装饰 | 系统装饰、客户端装饰、内容延伸；模式由调用方直接设置还是需要系统确认 |
| 系统交互 | 系统边缘缩放、从输入发起移动、从输入发起指定边缘缩放、系统窗口菜单分别表达 |
| 原生外观 | 支持的颜色／圆角偏好／阴影等具体操作，及当前模式下是否适用 |
| 区域 | 系统默认区域、同时裁剪外观与输入的 window region、独立 input region、整窗忽略鼠标分别表达 |
| 窗口关系与层级 | Owned 顶层关系、Tool 角色、一次 raise 与持续 topmost 分开；层级受 owner 约束时不能承诺独立设置 |
| 模态 | Owner／Application 范围与 Automatic／SeparateWindow／Sheet 的可用组合；嵌套展示限制 |
| 文件对话框 | 单选／多选／保存／目录选择，Owner／Application 目标，实际支持的筛选与其他选项 |

不要用单一 `supportsCustomWindow` 覆盖这些能力。例如 AppKit 的系统边缘缩放不证明它提供任意 `beginResize(edge)`；Win32 的 window region 不证明它有独立输入区域接口。

alpha 呈现同时依赖窗口和实际 renderer。`WindowCapabilities` 只描述窗口侧能力，Surface 描述自身的 alpha 呈现合同；UI 的形状策略在这两个具体边界取得结果，不增加全局能力注册服务。

静态 OS 能力与当前操作资格也有区别：窗口正在关闭、输入上下文已失效或目标当前不可操作，不应把平台能力永久改成不支持。第一阶段只在创建、相关模式改变或真实环境通知后按需重查，不增加周期探测。

### 3.2 请求与失败

沿用 `RequestStatus.Applied／Requested／Unsupported` 和现有 WindowException：

- Applied 表示该操作在合同规定的边界已应用，不隐含系统显示结果可被任意查询。
- Requested 表示已接受建议或待执行的请求，实际装饰、位置、窗口状态等由原生通知／查询确认。
- Unsupported 表示实际后端或当前组合不能提供该能力；不能用它掩盖本库尚未完成的实现。
- Unavailable 表示支持的服务当前不可用，例如所需托盘宿主不可用；无效生命周期使用 InvalidState，冲突的资源使用使用 ResourceBusy。
- 必需创建属性不支持时创建失败。Native failure 保留 operation、backend 和 DWORD／HRESULT 等原生 code。

能力查询不替代实际操作结果。正常参数、类型、归属和 ABI 检查保留；不为一次未变化的描述在 UI、window、native 多次重放整树检查。

### 3.3 实际窗框几何

保留 WindowMetrics 的内容尺寸与 DPI 职责，另提供与其配置对应的 `WindowFrameMetrics` 快照，描述实际装饰模式、内容可用区域、系统按钮占用区域和可查询的原生窗框度量。未知信息明确为 None，不能填入猜测的标题栏高度。

对外统一使用窗口呈现内容的逻辑坐标；原生外框坐标、物理像素、AppKit 底部原点由后端转换。内容延伸模式下，绘制区域可以覆盖标题栏，但 UI 能查询应避开的系统按钮／工具栏区域。DPI、装饰模式或系统布局改变时，将同一配置的 WindowMetrics 与窗框快照一起更新后再通知；内容尺寸不变但可用区域变化也必须产生通知。

## 4. 原生对象、身份与生命周期

| 对象 | 所有者与用途 | 关闭行为 |
| --- | --- | --- |
| `Menu` | Application owner 创建；拥有一棵原生菜单树与条目身份 | 撤销关联使用、取消 tracking，安全退出原生调用后释放整棵树 |
| `MenuBarBinding` | 菜单栏目标上的显式附着；借用 Menu | 解除附着，不替调用方关闭 Menu |
| `PopupSession` | window 提供的一次窗口 popup，保留菜单与目标的实际使用 | 取消幂等；退出 tracking 后记录一次结果并解除使用 |
| `NativeMenuLease` | window.native 提供的跨 Cone 菜单使用；使用身份不含托盘类型 | 结束原生使用后归还；Menu 关闭等待仍活动的使用退休 |
| `TrayIcon` | scoop.tray 创建的应用级资源；拥有图标注册与 TrayEvent 来源 | 移除系统注册、撤销菜单使用与后续激活，解除应用资源登记 |
| `TrayMenuSession` | scoop.tray 提供的一次程序化托盘菜单展示 | 退出 tracking 后记录一次终态并解除使用；不扩展 window 的 PopupSession 为托盘对象 |
| 窗框区域登记 | 归属具体窗口；UI 挂载只拥有自己的登记 | 撤销当前登记，不能移除其他仍存活使用者的区域 |
| `ModalSession` | 一次原生模态展示；借用实际 Window，记录输入范围 | 结束展示、隐藏窗口并释放本次限制；Window 仍走显式销毁 |
| `FileDialogSession<T>` | Application owner 创建；目标为 Owner(window) 或 Application，保存类型化结果 | 原生面板安全退出后完成一次结果，取消与退出仍走控制通路 |

MenuId、MenuItemId、PopupSessionId、NativeMenuUseId、ModalSessionId、FileDialogSessionId 属于 window，TrayIconId／TrayMenuSessionId 属于 tray，均为不同类型。所属 Cone 在同一 Application owner 上分配身份，由创建／插入操作返回，编号在一次 Application 生命周期内不复用；UI key 和原生 ID 不混用。移除后迟到的事件不能命中新创建的同名条目。

Win32 command ID、Shell callback ID、DBusMenu item ID 等原生数值由所属 Cone 与公共 ID 分开映射，遵守各自实际位宽，不截断公共 Long 身份。映射退休和排队事件一起处理，不能把旧原生编号的迟到消息交给复用该编号的新对象。

一份 Menu 原生树同一时间只允许一个附着或 popup 使用位置；重复使用报告 ResourceBusy。已附着菜单因用户操作展开自身子菜单不构成第二个使用位置。菜单描述可以复用，多个窗口、菜单栏与托盘各自创建实例。这样保持 NSMenu／HMENU 的实际父子与附着归属，不在后台隐式克隆活动原生树。

菜单树的插入、属性更新、重排和移除使用稳定 MenuItemId。普通属性变化不重新分配 ID；移除父项会退休其子项。原生模型不保存业务闭包，业务动作通过 ApplicationEvent 分发，上层再按当前挂载和 ID 映射到当前回调。

ApplicationEvent 增加 Menu、Popup 等 window 自有事件，不增加 Tray 分支。菜单激活携带 MenuId、MenuItemId 和发生时的附着来源；来源区分应用菜单栏、窗口菜单栏、PopupSession 与 NativeMenuUseId。托盘菜单借用入口报告最后一种来源，由托盘使用者／UI 包装识别对应图标；菜单动作只经这条路径执行，TrayEvent 不再重复交付同一次动作。应用菜单栏另保留事件发生时的活动 WindowId，没有时为 None，不在稍后派发时重新按焦点选择窗口。

所有创建、更新、查询与 close 在 owner 线程执行。跨线程操作使用现有 ApplicationProxy。字符串、像素、区域与事件 payload 在边界取得自有存储；native 不保存 managed 裸地址或短期借用。

ApplicationEvent 另增加模态与文件 session 的类型化归属／完成通知。Owned 关系与 session 会改变窗口的实际销毁条件：owner 必须等 owned 后代的 Surface lease 和相关原生展示安全结束，再销毁自己的句柄。详细关系与结果合同见[配套设计](WINDOWS-AND-DIALOGS.md)。

## 5. 原生菜单

### 5.1 模型与操作

第一阶段的条目为 Action、Checkbox、Radio、Submenu、Separator。公共显示字段包含文字、enabled、可选图标与快捷键。单选关系由上层模型决定，native 显示已发布的选中状态；它不建立另一份独立业务状态。

`Menu` 提供创建、按父项插入、更新显示／状态、重排和移除操作。创建与插入返回稳定身份，索引使用 Long。builder 仅构造描述，不创建 NSMenu／HMENU、读取 Binding 或执行回调。

同一原生项目通过“业务动作”或“标准角色”二选一表达执行语义，避免系统操作和用户动作各执行一次。标准角色覆盖实际接入的应用退出、窗口最小化／缩放、Cut／Copy／Paste／SelectAll／Undo／Redo；AppKit 的应用菜单行为作为明确的角色或平台选项接入。未列入的系统服务菜单不以空条目占位。

Quit 沿用 Application 的退出请求和有序关闭，不直接调用进程 exit 或 NSApplication.terminate。编辑角色按实际编辑目标分发：系统文件面板的字段由 native responder／控件处理；Scoop 内容区的目标产生独立 TextEditCommand，交给 Surface／UI 当前有效输入作用域，沿用选择、IME、撤销与可用性规则，不伪造 KeyEvent。

### 5.2 菜单栏归属

底层附着目标明确区分应用与窗口：AppKit 使用应用 mainMenu；Win32 使用目标 HWND 的菜单栏；Linux 使用当前 AdwWindow 内的 GtkPopoverMenuBar，首批按每窗口菜单栏声明能力，不假定桌面提供全局应用菜单。不存在的目标返回 Unsupported。UI 的跨平台映射见第 9 节，不让低层把一个应用菜单偷偷附着到所有窗口。

Linux 复用 GMenuModel／GSimpleActionGroup 构建菜单和动作，原生菜单栏挂在现有 AdwToolbarView 的附属位置，保留 GtkGLArea 内容容器。菜单造成的内容分配变化通过 WindowMetrics 通知；GTK 原生控件的布局不进入 Scoop UI 的 Row／Column。

同一菜单栏目标只有一个有效 MenuBarBinding；替换必须明确解除原绑定。Win32 的 SetMenu 不负责销毁旧菜单，本库按自己的所有权关闭；AppKit 同样不能让临时包装对象决定 mainMenu 的寿命。

### 5.3 快捷键与标准操作

复用 ShortcutChord／Primary／键盘布局标签。原生菜单声明的快捷键是实际动作绑定；现有自绘菜单的 shortcut 字段只显示标签，两者不得混淆。

第一阶段只接受能够准确转换到当前原生菜单系统的 chord。物理键、多段序列或其他不能保持语义的形式明确拒绝，不用显示标签冒充已注册；需要额外快捷键路由时再按实际需求扩展。

AppKit 需要调整当前 SWView.performKeyEquivalent 对 Command／Control 的截获，让原生菜单与编辑输入按确定的优先级分发。Win32 在 TranslateMessage／DispatchMessage 前接入 accelerator，命中后不再重复翻译同一消息。活动菜单栏范围内的重复快捷键在发布前按平台解析结果报告错误，UI 不再为同一原生菜单另注册一份 ShortcutScope。

Linux 通过 GTK 的 action／shortcut 入口落实相同优先级，GTK signal 只排队，业务动作在当前 GMainContext 派发结束后执行。原生菜单与现有 GDK 输入归一、AltGraph、IME 消费规则协调，同一按键不能再触发一份 Scoop 快捷键动作；GTK 文件面板中的编辑目标继续由 GTK 处理。

Scoop 编辑目标的 enabled／checked 等状态提前发布为 native 可读快照；系统的同步菜单验证不能等待 UI render 或可挂起回调。原生文件面板的编辑状态交给实际 native 目标，不能沿用后台 UI 编辑器的可执行快照。菜单命令遵守同窗口输入范围与原生窗口模态，不能绕过被阻塞的 owner。

### 5.4 Popup 请求与完成

`Window.showPopup(menu, placement)` 返回 PopupSession，不同步返回“选中了哪项”。placement 区分窗口内容坐标的指针位置与锚定矩形；键盘触发使用已提交的锚点，不编造鼠标坐标。窗口坐标和桌面坐标不通过单一 DPI 混合转换。

Linux 使用 GtkPopoverMenu 附着到现有窗口 widget，并从已提交内容坐标转换锚点；Wayland popup 的父 surface 和输入上下文由 GTK/GDK 处理。关闭 signal 和菜单 action 转为排队事件，不额外启动 GtkApplication.run 或嵌套 managed loop。

菜单激活和 popup 完成是两类信息：激活事件只执行一次条目动作；完成结果为 Selected(MenuItemId) 或 Cancelled(reason)，供调用方更新展示状态，不再次执行动作。取消原因区分普通取消、显式关闭、目标关闭与应用退出。

每个 session 只记录一次终态。应用仍接受业务派发且挂载有效时交付完成回调；卸载或退出后不调用已经退休的组件回调，内部完成和资源释放仍须执行。关闭不能依赖稍后某个普通 proxy 业务闭包成功运行。

同一 Menu 正在附着或 tracking 时不能另行用于 Window.showPopup。scoop.tray 通过 NativeMenuLease 绑定菜单，来源记录为对应 NativeMenuUseId，不要求先创建业务窗口。支持程序化显示的托盘后端提供 TrayIcon.showMenu，返回 tray 自有的 TrayMenuSession；仍是同一个菜单使用位置，tracking 时不重入。仅能由宿主响应用户输入显示菜单的后端，不能把程序化 showMenu 也声明为支持。

### 5.5 Tracking 中更新

菜单更新先更换逻辑描述和当前动作资格。native 能安全立即应用的显示字段直接更新；结构变动及系统不能安全接受的修改留到 tracking 结束，返回 Requested，保留当前显示树所需资源。

删除、禁用或卸载立即阻止后续业务激活，不能因为旧原生项目还在显示就继续执行已退休回调。正在显示的 native 项目只在退出相关调用后释放。复用同一 key 的新挂载不能接收旧实例事件。

参数检查在修改前完成；原生失败沿现有错误通路处理并关闭受影响使用，不能声称操作系统提供整棵菜单的事务回滚。此规则不要求建立通用事务或资源版本框架。

## 6. 独立托盘 Cone、图片与定位

### 6.1 TrayIcon 合同

`scoop.tray` 拟提供 `TrayIcon.create(application, options)`，创建登记到当前 Application 的应用级资源，配置图标、可选 tooltip 和菜单使用方式；运行中可更新图片、提示文字与菜单。它不作为 ActiveApplication 的内置成员，不依赖某个业务窗口，关闭最后一个窗口不自动删除它。TrayEvent 订阅沿当前 owner 队列串行执行。

激活事件区分主激活、上下文菜单请求与键盘来源；平台真正提供按钮、位置或次数时才携带相应值。指针位置与弹出锚点分开：键盘激活也可能有系统提供的图标锚点，但不能据此伪造鼠标事件；未提供坐标同样正常。菜单绑定策略与自定义激活策略明确选择，不能承诺绑定菜单后仍能收到所有平台的同一组点击回调。

注册成功表示系统接受图标注册，不保证图标在通知区域主栏可见；Windows overflow 和 Linux 宿主策略不由 Scoop 控制。无法查询的实际可见性不返回伪造的 true。

AppKit 使用 NSStatusBar／NSStatusItem.button，绑定 menu 会改变默认点击行为。Accessory 等 activation policy 继续使用已有平台配置，创建 tray 不擅自修改应用 policy 或 LastWindowPolicy。

Win32 由托盘 Cone 在现有 owner 线程持有专用隐藏顶层 HWND，接收 Shell 回调与 TaskbarCreated；不依赖窗口库内部 HWND 的私有消息分发，也不创建第二个主循环。scoop.tray 声明自身所需的 shell32 等链接项，使用 NOTIFYICON_VERSION_4，正确处理鼠标与键盘通知。Shell 恢复时只重新注册存活图标及其最新状态；菜单的 foreground／dismiss 行为按 Shell 要求处理。

Linux 本轮选择 StatusNotifierItem（SNI）与 DBusMenu，使用现有 owner 上的 GLib/GIO 派发。SNI 注册、图像转换、菜单协议适配、Watcher／Host 变化及可能增加的 native 依赖全部归 scoop.tray；缺少托盘宿主时报告 Unavailable，不把图标注册等同于显示成功。宿主重启后按实际服务通知重建仍存活的注册，不轮询、不自动安装桌面扩展。

SNI 的菜单接口使用 `com.canonical.dbusmenu`，与 GMenuModel 的 GTK D-Bus 导出协议不同。托盘层通过 NativeMenuLease 读取／映射公共菜单，并优先选用确认仍维护、兼容 GTK4／GLib 的协议实现；额外依赖在该 Cone 中声明。不能直接把 GTK 的菜单导出当成 DBusMenu，也不能为了使用现有 GtkMenu API 在同进程混入 GTK3。XEmbed 不列入本轮托盘实现。

### 6.2 图像边界

Menu 与 TrayIcon 复用 window 提供的轻量 CPU 图像值：明确宽高、紧密排列的 straight-alpha RGBA8 数据，以及需要的多分辨率表示。它不含托盘类型或额外 native 依赖。检查尺寸、乘法溢出与字节长度；各所属 Cone 按目标格式复制／转换并拥有 NSImage／HICON／HBITMAP 或 SNI pixmap。

模板图标与普通彩色图标具有不同呈现意图。只有确实提供系统模板适配的后端才声明支持；调用方可提供不同表示并按能力明确选择。缩放、透明边缘、浅色／深色和高 DPI 是图标验收内容。

不让 window 或 tray 依赖 GPU Image，不隐式读取 UI View 或每帧截图。PNG／JPEG 解码及 Path／View 导出在已有图形或明确的资产准备入口完成；若需要从图形资源导出 CPU 像素，补一个显式转换操作，而不是扩大图标参数可接受的资源种类。

### 6.3 图标、系统条与工作区的几何

在托盘旁显示菜单或自绘窗口是第一阶段使用场景。需要取得的几何分为图标锚点、目标显示器工作区和可查询的系统条边界，三者不能互相代替。

| 平台／协议 | 自有图标的位置 | 系统条与可用区域 |
| --- | --- | --- |
| Windows Shell | Shell_NotifyIconGetRect 通过 NOTIFYICONIDENTIFIER 查询图标的屏幕矩形；部分 version 4 通知另提供 anchor point | SHAppBarMessage(ABM_GETTASKBARPOS) 查询系统任务栏矩形；GetMonitorInfo.rcWork 提供目标显示器工作区。任务栏查询不是所有副屏／第三方 appbar 的完整枚举 |
| macOS AppKit | 从 NSStatusItem.button 的 bounds 转到其 window，再用 NSWindow.convertToScreen 转成桌面矩形；在实际展示上下文中确定宿主屏幕 | NSStatusBar.thickness／NSMenu.menuBarHeight 提供厚度／高度信息，NSScreen.visibleFrame 提供避开当前菜单栏、Dock 等区域的工作区；高度不等于每个显示器上当前可见菜单栏的精确矩形 |
| Linux StatusNotifierItem | Activate／ContextMenu 的 x、y 是宿主给出的 screen-coordinate hint；标准协议没有图标矩形查询 | 不保证面板矩形、边缘或每显示器工作区；可由宿主显示导出的原生菜单，不能据此推定可定位任意普通窗口 |
| X11 XEmbed（本轮不采用） | 自有嵌入图标窗口可通过 X11 几何及到 root 的坐标转换定位 | 不能将这种图标窗口能力套到本轮由宿主绘制图标的 StatusNotifierItem |

Win32 窗口内的原生 menubar 是另一种对象，可以通过 GetMenuBarInfo 查询菜单栏、通过 GetMenuItemRect 查询项目矩形；它与 Shell 任务栏分开。macOS 的系统状态栏按钮本身已经是足够精确的锚点，不需要先取得整条系统菜单栏矩形。

拟增加以下普通查询与值；TrayGeometry／TrayAnchor 属于 scoop.tray，DesktopRect／Monitor 与工作区查询仍属于 scoop.window：

- `TrayIcon.geometry(): TrayGeometry?`：返回本次系统查询得到的图标 DesktopRect 和可识别的 MonitorId。后端支持查询但当前没有可用几何时返回 None；未提供该能力与真实原生失败仍遵守第 3 节的错误区分。
- `TrayAnchor`：在激活／上下文菜单事件中分别表示本次图标矩形快照或系统给出的 PointHint，不把提示点伪装成零尺寸图标矩形。系统没有可靠信息时为 None。
- `Monitor.workArea: DesktopRect?`：在显示器快照中增加工作区。当前 Monitor 只有 desktopBounds，各平台 native monitor ABI 同步扩展；Darwin／Win32 读取 NSScreen.visibleFrame／MONITORINFO.rcWork。X11 读取窗口管理器实际发布的 EWMH 工作区并说明其虚拟桌面／显示器语义；没有可靠信息时为 None。Wayland 的 GdkMonitor.geometry 不能冒充避开面板后的工作区。

系统条的完整几何按真实公开能力提供平台查询，并注明覆盖范围；不能用“屏幕矩形减工作区”反推一个精确 taskbar／menubar 矩形，也不把推测的边缘作为 OS 已报告事实。

几何是查询时／事件发生时的快照。每次显示前按需重新查询图标和工作区，处理任务栏自动隐藏、overflow、图标重排、屏幕切换与 Explorer 重启；不要长期保存启动时的位置。拿到矩形不证明图标位于主栏或当前对用户可见。AppKit 多屏状态栏的实际事件来源与隐藏行为、Windows overflow 的可用锚点均需要真实桌面验收。

所有公共矩形使用既有 DesktopRect 语义。AppKit 的屏幕原点和坐标单位由后端转换；Win32 使用当前 DPI awareness 下的屏幕坐标，允许负坐标。不能将整张桌面统一除以某个窗口的 scaleFactor；弹出窗口的逻辑尺寸和间距在目标显示器的上下文中转换。

### 6.4 在托盘旁显示菜单与窗口

原生菜单优先使用 TrayIcon 已绑定的菜单路径或支持的 TrayIcon.showMenu，让后端利用真实图标视图／系统锚点定位并处理边界。菜单可以由宿主正确显示，即使应用拿不到图标矩形；这两项能力独立。AppKit 的 NSPopover 也支持直接锚定视图，但该系统 API 不使本次自动增加一套 NSPopover 内容宿主。

普通自绘弹出窗口使用既有 Window 生命周期，拟提供 `TrayIcon.placeWindow(window, placement): RequestStatus` 作为窄定位操作。它只定位同一 Application 内已有的窗口，窗口创建、内容挂载、显示、激活和关闭继续由调用方管理。

定位过程为：

1. 取得当前图标矩形、目标显示器及其工作区；精确矩形不可得时，仅在调用方明确允许的情况下使用本次事件的 PointHint。两者都不可用时报告 Unavailable，不使用屏幕原点或历史光标位置。
2. 根据已确定的窗口尺寸、对齐、间距与首选方向产生候选位置。自动方向根据锚点与工作区的实际关系选择，不写死 macOS 向下／Windows 向上。
3. 首选侧放不下时按配置翻转，再沿边平移到工作区内；缩小窗口必须显式允许，并满足现有尺寸约束。工作区未知时，只有明确允许使用完整显示器边界的策略才能继续。
4. 按目标显示器的 DPI 与实际外框换算位置，调用正常窗口请求。跨屏造成实际尺寸或 DPI 变化时按新配置重新定位；Requested 仍不代表已经到达目标位置。

定位与窗口激活分开，放到图标附近不隐含抢焦点或更改置顶策略。默认只在打开与明确重新定位时计算；需要持续跟随图标的使用者按真实通知重新查询，不增加全局轮询。菜单栏／任务栏自身移动但没有完整通知保证时，也不承诺永久跟随。

Wayland 的标准 xdg_toplevel 不提供任意全局定位；xdg_popup 需要合法的父 surface，不能把另一客户端面板上的 tray 图标直接当作自己的父 surface。因此，SNI 的 x、y hint 或“能显示托盘原生菜单”都不证明 placeWindow 可用。只有实际桌面协议支持该行为时才提供；否则应用明确选择由系统普通打开窗口，不能返回伪造的 Applied。

## 7. 窗口装饰、形状与系统交互

### 7.1 窗框模式

将当前 decorations 布尔值演进为 `WindowFrame` 的两个明确分支：

- System：平台默认原生宿主提供可见装饰和窗口按钮，包含 AppKit／Win32 窗框及 GTK/libadwaita 标题栏；其配置区分正常内容区与 Extended 内容延伸。Extended 保留宿主窗口功能，同时让应用内容进入标题栏范围。
- Client：应用负责可见标题栏和边框，系统不绘制这部分装饰。resizable、窗口按钮允许的操作、窗口层级等仍是独立属性；底层保留实现系统行为所需的 style／协议。

Client 模式不要求应用一定绘制标题栏：不挂载 WindowChrome 即可构成纯无边框窗口，无需再增加语义重复的 None 模式。系统阴影、窗口 region 与背景透明仍独立表达。

这里的 System／Client 区分平台宿主装饰与 Scoop 自绘装饰，不等同于 Wayland 协议中的 SSD／CSD。当前 libadwaita 标题栏虽然由客户端 GTK 绘制，仍属于 System；切换 Client 时必须隐藏该标题栏并按公共配置处理 GTK 背景、圆角和阴影，保留内容 widget 与 GtkGLArea。不能同时显示两套标题栏。

Extended 不是忽略系统装饰的通用开关。后端不能提供时明确返回 Unsupported；提供时发布实际内容可用区域与系统按钮位置。AppKit 的 titlebar 透明、titleHidden、fullSizeContent 按这组语义整理，平台专有细节继续留在 platform.macos。

上述 System／Client 描述实际采用模式。需要系统协商的后端以最终配置为准；UI 不能因为请求返回 Requested 就立即假定系统窗框已经消失。只有实际 Client 模式才发布替代系统装饰的自绘窗框；若允许采用其他模式，按应用明确选择的降级策略更新布局。

实施时同步迁移 WindowAttributes、builder、运行中 setter、native ABI、示例和测试。旧 decorations 的替换关系写入规范，不保留同时互相覆盖的两套窗框属性。

### 7.2 自绘外观与原生外观

自绘边框的颜色、宽度、圆角与路径由 graphics／UI style 处理。原生外观设置表达系统真正接受的选项，不能把任意 Double 半径或边框宽度包装成所有 OS 都支持的参数。

| 平台 | 本次采用的原生路径 | 能力边界 |
| --- | --- | --- |
| Win32 | 保留所需窗口 style，通过 WM_NCCALCSIZE 控制客户区，通过 WM_NCHITTEST 与 DwmDefWindowProc 接入窗框；DWM 设置支持的原生外观 | Windows 11 的边框色、标题栏色、文字色和圆角偏好分别探测；可见边框厚度是查询值。window region／分层 alpha 等组合可能影响系统圆角 |
| AppKit | 系统标题栏、full-size-content、系统窗口按钮与 contentLayoutRect；无装饰窗口沿既有 key／main window 能力 | 原生按钮占用区域需准确提供；AppKit 的 contentBorderThickness 不能当成任意 CSS 式窗口外边框 |
| GTK／X11 | 现有 AdwWindow／AdwHeaderBar；GDK 系统移动／缩放与输入 region；必要的 Xlib／Shape 能力 | 默认装饰由 libadwaita 提供；额外窗口管理请求依赖实际 WM；alpha 合成需要 compositor，不能改回旧 XCB 宿主 |
| GTK／Wayland | 现有 GTK 客户端装饰、GdkToplevel 系统交互和 GdkSurface 输入 region | 由 GTK/GDK 管理 surface 与协议；普通窗口无任意全局定位／topmost。不能绕过 GTK 单独提交 wl_surface 状态或另建 libdecor 窗口 |

平台专有的颜色／圆角配置分别放在 platform.windows、platform.macos 的明确选项中；共享组件通过能力和实际几何工作。常见主题名不能成为操作系统检测方式。

Linux 原生外观沿现有 libadwaita 能力，只暴露确实支持的本窗口操作；GTK CSS 可修改不等于所有配置都属于稳定的原生外观合同。标题栏、菜单栏与内容区几何按 GTK 的真实分配转换，保留当前内容区不含宿主标题栏／阴影的 WindowMetrics 语义。

### 7.3 三种区域

视觉形状、原生输入区域和窗框命中语义是三个独立接口：

| 区域 | 表达的事实 | 不能代替的能力 |
| --- | --- | --- |
| 绘制／视觉轮廓 | 像素 alpha、clip、路径、圆角与阴影 | 透明像素不自动承诺跨应用鼠标穿透 |
| 原生 window／input region | 系统如何裁剪窗口，以及哪些位置接收指针输入 | Surface InputArea 只能在窗口已收到事件后选择组件 |
| 窗框命中区域 | Client、Caption、Resize(edge)、系统按钮角色 | 不改变窗口在桌面上的输入覆盖范围 |

window 接收轻量、自有的 `WindowRegion` 数据，使用窗口逻辑坐标中的矩形区域集合表达二进制覆盖。外框坐标转换与 DPI 量化由后端负责，规范明确边界取整；window 不接收 graphics.Path 或 GPU 资源。

任意轮廓的视觉绘制保留 Path。需要把该轮廓用作原生区域时，由 graphics／surface 显式转换为区域数据，优先复用 Skia 已有区域／路径栅格化能力，不新写路径布尔或像素拾取引擎。二进制输入区域与抗锯齿像素的差异必须明确，不能承诺逐像素一致的半透明命中。

拟提供的操作明确分开：

- 设置 window region：可能同时裁剪外观和输入，Win32 SetWindowRgn 属于此类。
- 设置独立 input region：保持绘制内容，只改变原生输入覆盖；只在能保持这一语义的后端提供。
- 整窗忽略鼠标：与上述两种区域不同，不能通过忽略 UI 回调模拟。
- 恢复系统默认区域：明确移除覆盖配置，不把空集合同时解释为默认和完全穿透。

不能用 SetWindowRgn 冒充独立 input region，不能用 Win32 HTTRANSPARENT 冒充跨进程穿透。AppKit 首先沿透明窗口和真实 renderer 验证形状行为；没有与独立 input region 等价的公开能力时，不承诺任意独立输入轮廓。Linux 优先使用 GdkSurface.set_input_region，并查询实际后端的 input-shape 支持；X11 额外 bounding shape 只在不会破坏 GTK surface 管理时接入。Wayland opaque region 只是合成优化提示，不能充当形状或输入裁剪。

GTK 的区域坐标基于实际 native surface，须通过 GtkWidget／GtkNative 的真实变换换算，不能把 Scoop 内容区坐标直接当成包含标题栏／阴影的 surface 坐标。区域发布还要与 GTK 自身的分配／装饰更新协调，避免刚提交的自绘区域被宿主覆盖；该组合在 D1 验证，不以每帧无条件重设兜底。

UI 的形状配置显式区分“只要求视觉轮廓”和“输入也必须匹配”。后一种要求不能满足时失败；只有应用明确选择允许矩形输入范围时才降级。未验证的 renderer／窗口组合先完成第 12 节的验证，不直接当作已支持或以 Unsupported 跳过实现。

### 7.4 原生命中与区域优先级

`WindowHitRegion` 描述 Client、Caption、Resize(edge) 和系统按钮角色，使用已转换到窗口逻辑坐标的自有区域。系统原生按钮的保留区域先由后端处理，其余区域按已发布的顺序决定命中；未命中时为普通 Client。

WindowDragRegion 的交互子项需要明确排除：按钮、文本编辑和其他输入区域继续接收正常输入，显示文字或空白可以保持 Caption。UI／Surface 在提交时根据真实区域组装结果，native 不检查组件类型。重叠规则、clip、变换、LTR／RTL 和最大化时的 resize 资格都由同一份已提交状态决定。

WM_NCHITTEST 等必须立即返回的 OS 回调只读取 native 快照，不同步调用 managed render、measure、业务命中回调或可挂起函数。禁用、卸载、失焦后的动作资格可以立即撤销；新位置或新目标必须等待 Accepted 几何。

Windows 自绘最大化／还原按钮需返回对应 HTMAXBUTTON 等语义，处理系统需要的非客户区消息，以保留 Windows 11 Snap Layout。系统已经执行的动作不能再由 UI 点击回调重复执行；视觉 hover／pressed 状态按实际消息反馈。

### 7.5 系统操作与输入上下文

补齐 beginMove、beginResize(edge)、showWindowMenu 的公共操作合同。操作要求当前有效的输入派发上下文；后端保存需要的 NSEvent、按键／按钮来源或 Wayland serial，不向调用方暴露原生借用，也不建立额外的授权凭据系统。

可能进入原生嵌套循环的操作先排入 native 命令队列，等当前业务 handler 和 session pump 返回后执行。期间主循环继续推进有限事件批次与 asyncio；不通过递归进入同一个 managed pump 维持活动。原生 API 本身立即返回时不人为增加阻塞。

系统接管拖动可能不再向原输入目标发送普通鼠标释放。输入层必须取消／结束原按下序列，避免残留 pressed、拖动或随后额外触发 click；失效输入与取消沿现有 PointerCancelReason 和路由合同处理。

Win32 使用系统窗框命中和系统操作。AppKit 明确接入 performDrag(with:)；保留系统边缘缩放与公开 beginResize 分别声明，不能借助手动修改 frame 假装后者是系统缩放。Linux 经 GdkToplevel.begin_move／begin_resize／show_window_menu 使用真实 GDK 事件、device 和时间等上下文，由 GDK 处理 X11／Wayland 协议；不在新组件中复制输入 serial 管理。

WindowControls 优先保留可用的系统按钮。完全自绘按钮只调用底层已支持的窗口语义，macOS zoom 与全屏不混用；本次不因为绘制了绿色按钮就补一套未设计的全屏状态机。

## 8. Surface 与窗口的提交边界

菜单标题、enabled／checked、托盘图片以及不依赖布局的窗口配置走逻辑更新；窗框区域和形状位置走已提交几何。两条路径共用 owner，但不能互相作为执行前提。

Surface 在实际 scene 组装时收集窗框／原生区域候选，使用最终选中的 LayoutResult、当前 transform／clip 和 WindowMetrics。只有帧被 Accepted 后，才把对应候选交给 window 发布；Discarded 不发布本轮区域，不重放试测中生成的候选。

沿用现有 Accepted 的含义，不等待 GPU 扫描到屏幕才更新命中。不同 OS 的像素呈现与区域变更不一定原子，设计不额外承诺同步扫描；真实呈现和系统命中之间的过渡按平台验证记录。

区域使用实际布局版本／配置判断是否仍有效。DPI、模式或尺寸使旧配置失效时，按既有输入规则撤销不再合法的区域；系统已经接管的 resize 继续由 native 处理。Window.frameRevision 在原生失效时更新，Surface 在发布前比较，覆盖绘制回调改变窗框但通知尚未派发的情况。关闭／卸载立即移除登记，不等待下一帧。场景缓存可以复用未变化的区域数据，但不能重放一次性提交回调，或为纯颜色变化重新测量全部标题栏。

透明属性变化要覆盖窗口和 renderer：CAMetalLayer.opaque 在安装时读取窗口值，之后由窗口背景变化同步；DirectComposition 保持预乘 alpha swapchain，并验证与当前 HWND 样式、region 的组合。失败不能留下窗口认为透明而 renderer 按不透明处理的已成功状态。

Linux 延续 GtkGLArea／Skia Ganesh GLES 的合成路径，验证预乘 alpha 与 GTK 背景、圆角、阴影、输入区域及菜单栏的组合。内容物理尺寸使用现有 widget 整数 scale factor，不把显示器分数缩放再次乘入 framebuffer；不绕过 GTK 创建额外 EGL window surface。区域和菜单布局变化依赖真实 GTK 分配及 Accepted 边界。

系统标题栏／菜单栏改变可用内容区域时，重新发布实际 metrics 并使必要布局失效。Win32 的尺寸约束、requestSize、DPI 与 frame 计算需要统一考虑是否存在原生菜单，移除 AdjustWindowRectExForDpi 中固定“无菜单”的假设；多行菜单不能按常量高度处理。

低层区域发布仍允许没有 Surface 的 window 使用者提交自有快照；UI 适配层额外遵守 Accepted 边界，不要求普通 native 使用者为了设置区域创建 renderer。

## 9. UI 挂载与原生附属声明

### 9.1 应用级挂载

新增 UiHost 的应用级挂载入口，绑定当前 ActiveApplication，返回可关闭的 UiApplication。一个 UiHost 至多有一个应用根，仍可有多个 UiWindow。应用根不创建隐藏 UI 窗口、Surface、默认字体或 GPU context。

应用根使用 `ApplicationComponent`／`ApplicationScope`：组件持久保存状态，render 产生带稳定 key 的 `ApplicationContent`；内容只描述原生附属项。原生附属项采用 `ApplicationElement` 的 Props、onMount、update、onUnmount 协议，没有 measure／scene。第一阶段只需要应用组件根与这些条目，菜单子树是菜单数据，不增加通用无窗口布局树。

ApplicationContent／ApplicationElement 通过正常组件协议接受库提供的实现；UI 核心不定义包含 TrayIcon 的封闭条目枚举。nativeTrayIcon 在独立组件 Cone 中实现同一协议，不需要核心为每个外部组件增加类型分支。

这是现有挂载内核的窄扩展。复用 State、DependencyCollector、Props 替换、key 协调、失效队列、单次实例归属和资源清理；不另建响应式库、订阅线程、定时刷新或独立 reconciler。StateSignal 当前绑定 MountedNode，需要将实际失效接收者抽为窗口节点和应用挂载共用的内部入口，不改变 State 写入即通知的语义。

ApplicationScope 提供当前应用入口、resource／onDispose 和所属资源事件登记，不提供 nativeWindow、InputArea、Canvas 或布局方法。ComponentScope 保持必需的窗口能力，并增加取得同一应用入口的必要桥接；不把现有必需字段普遍改成 None。

构造作用域、首次 render、onMount、失败清理和关闭顺序沿用当前 UI 合同。render 只生成描述；创建原生菜单、注册托盘、修改窗口等操作在挂载／update 或所属业务回调执行。业务任务继续由现有协程作用域管理。

`runUiApplication` 当前是 LastWindowPolicy.Exit 的单窗口便利入口，不适合作为托盘应用默认入口。托盘示例使用显式 ApplicationOptions 和 UiHost 应用根；关闭最后一个窗口后，应用根继续接收 State 更新和原生事件。

### 9.2 窗口附属组件

窗口内的 popup 触发器、ContextMenu 和 WindowChrome 继续用普通 Component／Element。窗口菜单栏通过透传正文的包装或根附属位置挂载，不成为 Row／Column 中一个零高度却产生 gap 的视觉子项。

一个窗口只允许一个当前窗口菜单声明和一个根窗框配置。普通子组件只能登记属于自己的交互区域，不能通过“最后一次写入”争抢整个窗口的菜单或窗框模式。

UiHost.attach 继续借用调用方创建的 Window。UiWindow.close 撤销 UI 资源、区域和附着，不销毁 Window，也不自动恢复历史窗口属性。需要切回系统窗框时显式更新配置；移除 WindowChrome 后保留 Client 模式可以用于真正无装饰窗口。

本次不增加声明式创建所有顶层 Window 的树协议。应用根的动作可以使用 createWindow／attach；工具／对话框的窄控制器拥有自己创建的 Window 和 UiWindow，卸载时结束 session、关闭 UI 并 requestDestroy，与现有借用入口分开，详见[配套设计第 7 节](WINDOWS-AND-DIALOGS.md#7-ui-包装与使用方式)。

### 9.3 原生组件与控制器入口

| 拟定入口 | 描述结果／归属 | 行为 |
| --- | --- | --- |
| nativeApplicationMenuBar | 应用级附属项 | 声明应用默认菜单内容 |
| nativeWindowMenuBar | 窗口正文包装 | 声明当前窗口的菜单覆盖，正文布局透传 |
| nativeMenuButton | 普通 View | 保留 UI 按钮，使用已提交锚点显示原生菜单 |
| nativeContextMenu | 普通 View 包装 | 处理右键、菜单键与 Shift+F10，保留必要焦点 |
| nativeTrayIcon（scoop.ui.component.tray） | 应用级附属项 | 响应式图标／菜单与激活回调；仅显式依赖托盘组件 Cone 时可用 |
| windowChrome | 普通 View 包装 | 根窗框策略、标题栏内容槽与正文布局 |
| windowDragRegion／windowResizeRegion／windowControls | 普通 View／区域组件 | 声明系统交互语义，按能力提供可用行为 |
| nativeToolWindowController／nativeDialogController | scope 拥有的显式窗口控制器 | 创建真实 Owned 工具窗口或模态窗口，负责自己的窗口与 UI 生命周期 |
| fileDialogController／nativeOpenFileButton／nativeSaveFileButton | scope 资源／普通 View | 显式打开系统文件面板，管理 pending、取消与类型化结果；render 不触发显示 |

应用与窗口入口在名称、作用域和返回类型上区分，不依赖返回类型重载选择。各入口遵守必需参数前置、configure 只构造值、最新 Props 和 receiver 边界规则；正式签名、默认值和清除语义在对应批次的规范中一次写全。

### 9.4 菜单栏的跨平台映射

UI 宿主按菜单栏放置能力处理应用默认菜单和窗口覆盖：

- 应用全局菜单栏：选择当前合格 key／main 窗口的覆盖；没有覆盖或对应窗口已关闭时使用应用默认菜单。焦点切换沿现有 owner 更新边界切换，不为每个窗口同时向系统附着 mainMenu。
- 每窗口菜单栏：每个窗口使用自己的覆盖或应用默认描述；同一默认描述为不同窗口建立独立 Menu 实例。没有窗口时没有菜单栏，应用根与托盘仍可存在。

应用默认菜单不是一个共享 HMENU／NSMenu／GTK 菜单实例。描述与业务模型可以共用，各实际附着保存独立原生身份和明确的事件来源。一次操作只触发对应来源的当前回调。

宿主依据 MenuCapabilities.applicationMenuBar／windowMenuBar 完成这项明确映射，UI 组件不检查 OS 名称。关闭或失焦时的选择规则在多窗口验收中覆盖，不能把已经销毁窗口的菜单留作活动菜单。

### 9.5 NativeMenuContent 与状态更新

原生组件使用独立的 NativeMenuContent：稳定 key、文字、enabled、动作或角色、checkbox／radio Binding、子菜单、图标和可转换的 ShortcutChord。任意 View／ButtonStyle／FocusOptions／Portal 动画不进入该模型；不在现有 MenuContent 上加 native=true 后静默忽略这些字段。

NativeMenuContent 的 builder 只保存值、Binding 与回调。真正的绑定读取发生在所属 render 求值中：

1. render 读取当前 enabled、checked、radio 选择和图片，生成本轮原生显示快照并收集依赖。
2. 挂载／update 按作用域内完整 key 路径协调 MenuItemId、原生对象和当前回调。兄弟项 key 不重复，不同子菜单可使用同名局部 key。
3. 仅有原生显示或回调变化时不请求 Surface 帧；修改原生菜单栏导致内容几何变化的情况由窗口通知驱动布局。
4. 原生激活通过 UI 所属边界执行，读取最新 Props／模型；checkbox 写入 Binding 后重读接受值，radio 保留类型化业务值，不在 native 维护另一份选择状态。

不能只在 Element.update 中读取 Binding：当前任意 State 依赖只在 render 收集。回调替换也不能依靠闭包在首次构造时的捕获。重排复用身份，移除再添加形成新原生身份，tracking 中更新遵守第 5.5 节。

原生 popup 用显示请求、session 终态和 onClosed 表达生命周期，不照搬自绘菜单的关闭否决、closeOnSelect=false 或退出动画。系统已经关闭后，受控值没有同步改变不能导致下一轮 render 自动重开；再次打开必须来自新的明确请求。

### 9.6 几何、事件与错误边界

普通按钮／右键触发器可复用 InputArea.visibleBounds、toWindow 等最近成功提交的几何。需要允许外部 LayoutAnchor 时，再增加读取已接受锚点的窄接口；不读取正在试测或未提交的布局，也不为基本右键菜单先建立新锚点系统。

UI 提供针对 Menu／ModalSession／FileDialogSession 等实际资源的类型化事件登记与注销；托盘组件 Cone 自行订阅 TrayEvent，并通过 NativeMenuUseId 关联菜单动作，UI 核心不导入托盘类型。组件不能替换 ApplicationHandler，不能各自监听所有全局事件后猜测归属。窗口资源回调沿 SurfaceWindow.dispatch，应用资源沿应用根错误入口执行；无窗口异常不伪造 WindowId。

窗框快照的真实变化通过同样的所属观察入口更新 WindowChrome 使用的状态，使相关 render／布局失效；不让组件逐帧查询 native 属性，也不把创建时的期望配置长期当作实际配置。

菜单选择、tray 激活、对话框完成、状态变化与普通按钮使用同一逻辑更新队列。标准编辑命令先区分 native 编辑目标与 Scoop 内容区，再遵守当前活动输入范围；当前编辑器已有 execute(TextEditCommand)，只补齐菜单来源和可执行状态的窄桥接，不建立通用命令总线。

默认错误处理沿现有宿主规则关闭受影响资源并报告；窗口级错误不能无条件关闭其他正常窗口。应用根失败先关闭其菜单／托盘及订阅，再由显式应用错误策略决定是否退出。异常不得穿越 C／ObjC 回调栈。

## 10. 后端实现与依赖选择

本轮延续现有 C／Objective-C 薄层，以 AppKit、Win32／DWM／Shell、GTK4／libadwaita／GDK 实现所需能力。window 与 tray 分别拥有源码和 native 依赖，通过明确的菜单借用与应用资源登记共享 owner、队列及关闭协议，不建立第二套宿主。

| 位置 | 主要改动 |
| --- | --- |
| window 共享源码 | 原生资源与 typed ID、事件、能力与窗框值；窗口关系／模态／文件 session、销毁依赖、输入上下文与 native 命令排队 |
| window Darwin／Windows native | 菜单、快捷键、窗框／区域／DPI；窗口关系、模态、AppKit／COM 文件面板；公开 native 菜单借用 |
| window Linux native | GTK 菜单／action、宿主窗框与 GDK 区域／系统操作、transient／模态范围、GtkFileDialog 与现有 GMainContext 派发 |
| tray 各平台 native | NSStatusItem、Shell 图标及隐藏消息 HWND、SNI／DBusMenu 与宿主恢复；独立应用资源的关闭 |
| Surface renderer／提交 | alpha 同步，原生区域候选与 Accepted 发布；保持 HWND／NSView／GtkGLArea 和输入连接 |
| UI／component／component.tray | 应用根、原生附属协议、事件归属、组件与菜单模型；托盘包装在独立 Cone |
| Cone 配置 | 按真实使用者声明 native 输入／链接项；Linux 额外托盘依赖只进入 scoop.tray 及显式消费者 |

调研时参考的成熟库为 [winit 0.30.13](https://github.com/rust-windowing/winit/releases/tag/v0.30.13)、[muda 0.21.2](https://github.com/tauri-apps/muda/releases/tag/muda-v0.21.2) 和 [tray-icon 0.26.1](https://github.com/tauri-apps/tray-icon/releases/tag/tray-icon-v0.26.1)。后两者仍有近期维护，可参考角色、图标与平台适配；本阶段不为了引入它们改变 Cone 的 Rust 依赖／staticlib 构建流程。平台成熟 API 与现有 Skia 能力优先复用，引用或改写代码时保留来源与许可。

Linux 新增依赖在选定前核对维护状态与 GTK 主版本。调研时 libayatana-appindicator 的现成构建仍依赖 GTK3／dbusmenu-gtk3，不能直接纳入 GTK4 窗口进程。优先复用适用的 GLib 协议部分；若没有合适实现，只在托盘 Cone 内以现有 GDBus 补所需 SNI／DBusMenu 适配，不创建通用 D-Bus 框架。准备、打包和缺失依赖诊断跟随该 Cone，window 的 GTK 依赖不承担这部分成本。

原生菜单、系统拖动、模态窗口和文件面板分别验证 AppKit tracking／modal modes、Windows 嵌套消息循环，以及 Linux GTK/GIO 异步派发。当前业务回调返回后才执行可能阻塞的 native 操作；不能因既有循环接入已验证就省略实际展示、取消、I/O 与有序关闭验收。

## 11. 收尾与失效顺序

正常卸载先撤销业务事件登记和输入资格，再停止新请求，取消 popup／tracking，解除菜单栏与托盘使用，最后在原生不再访问时释放菜单、图像与回调持有。普通属性替换只退休被替换对象，不全量关闭无关资源。

应用退出将菜单／托盘／活动 session 纳入现有有序关闭：停止业务投递，撤销托盘注册与菜单附着，请求结束活动原生交互；让所需控制通知与 asyncio 收尾继续推进，再释放剩余原生资源并停止主循环。Exiting 阶段与最终清理的顺序按 Window 规范更新，不能在旧循环仍引用对象时直接释放。

window 通过应用资源登记通知 tray 收尾，不遍历托盘私有对象。tray 先停止新激活、结束菜单使用和原生注册，再解除 NativeMenuLease 与应用登记；窗口宿主不能在这些使用退休前停止。GTK/GIO 的取消完成与 D-Bus 回调退休继续在现有主上下文推进。

拥有子窗口的 owner 先使后代进入 Closing，结束相关 ModalSession／FileDialogSession，等待后代 Surface lease 与原生调用退场，再按子到父顺序销毁句柄。Win32 会自动销毁 owner 的 owned HWND，不能只检查父窗口自己的 lease。退出确认在 CloseRequested／ExitRequested 阶段进行，Exiting 后不再创建确认窗口。

业务关闭后已经接受的普通 post 不保证执行；清理不得依赖它。重复 close／cancel 幂等，迟到事件只完成必要的 native 退休，不调用已卸载的组件。清理异常沿现有 ResourceUseException 等规则合并，继续关闭其他实际资源。

## 12. 实施顺序与完成条件

每批同时包含对应 spec、实现、实际使用者与必要验证。本设计本身不将这些批次标为完成。

| 批次 | 工作 | 前置与完成条件 |
| --- | --- | --- |
| D1 合同与平台确认 | 固定资源、Cone 边界、native 菜单借用／应用资源登记、能力及 UI 应用根合同 | 验证 GTK 与各平台窗框／区域、模态范围、文件面板和无窗口操作；核定 Linux 托盘依赖及桌面服务限制 |
| D2 Menu／Popup | 菜单树、稳定身份、菜单栏附着、popup、快捷键、编辑命令来源与 NativeMenuLease | AppKit／Win32／GTK 普通程序可使用；更新、取消与关闭通过，GTK 派发不递归执行业务 |
| D3 独立 Tray Cone | scoop.tray、图像与菜单、图标锚点／工作区、定位、服务恢复与无窗口收尾 | D2；额外依赖只属于 tray；Linux 验证 SNI 宿主存在／缺失／重启，精确定位只在实际支持时提供 |
| D4 Frame／Region | 窗框模式、实际几何、系统拖动／缩放、原生外观与区域 | D1；每个平台准确声明能力，实际系统操作／点击穿透验证通过 |
| D5 Window 关系／Modal | Owned、Tool、一次 raise、模态范围／展示、输入与焦点、依赖后代 lease 的关闭 | D1；Owner 与 Application 范围准确，工具窗口保持相对叠放，嵌套取消与子到父关闭通过 |
| D6 Shell Dialog | 打开／多选／保存／目录面板、Path 结果、COM／AppKit／GtkFileDialog、取消与原生编辑 | D5；实际 GTK／portal 与其他平台面板下 I/O、菜单编辑、owner 关闭与无窗口应用通过 |
| D7 UI／Surface 适配 | 应用级挂载、状态观察共用入口、原生资源事件、Accepted 区域、alpha 同步 | D2～D6 对应入口与现有 UI 整改合同；无窗口更新、错误隔离、空闲停帧通过 |
| D8 组件与组合 | 通用原生组件、工具／模态／文件控制器；独立 scoop.ui.component.tray 包装 | D7；普通组件无托盘依赖，三平台按现有依赖完成必要组合验证 |
| D9 综合 Demo | 最后实现一个普通 Cone demo，完整展示这一组新增能力 | D8；显式依赖窗口、托盘及对应组件，提供运行说明和真实平台差异演示 |

菜单、窗框、窗口关系与独立托盘 Cone 按各自依赖推进；Linux 直接基于已合入的 GTK 后端实施，Wayland／X11 分别验证。D7／D8 可按已完成的低层入口逐项交付，Windows 组件验收仍需完成既有控件基础依赖。D1 的原生验证用于确认 API 边界，只有确定的系统限制才进入能力不支持清单。

最后编写一个完整的桌面能力 demo，统一展示原生菜单栏／popup、托盘与邻近弹窗、可定制／异形窗口、owned 工具窗口、Owner／Application 模态、一次 raise／持续 topmost、原生打开／保存／目录面板，以及响应式 UI 包装和有序退出。demo 使用公共 API，明确引入托盘组件 Cone；在 macOS、Windows、Linux Wayland／X11 上展示实际支持的操作，能力缺失时说明原因并演示明确选择的降级，不伪造结果。

示例入口位于 [native-desktop](../../sysroot/src/scoop.ui.component/examples/native-desktop/README.md)。正常启动提供交互展示；`--help` 不创建原生 Application，`--smoke` 在显示会话内验证有界的窗口、菜单、模态、文件面板和无窗口应用流程。实际平台结果与交互限制继续记录在[实施记录](PROGRESS.md)。

代表使用场景：

- 多窗口编辑应用：应用默认菜单、窗口覆盖、当前编辑器 Cut／Copy／Paste／Undo，以及窗口关闭后的菜单切换。
- 托盘应用：没有业务窗口时更新状态与图标，在图标旁显示菜单或自绘窗口，边缘空间不足时翻转／平移，关闭窗口后继续运行，显式退出完成 I/O 与 native 收尾。
- 自绘标题栏：按钮和输入框正常交互，空白区系统拖移，边缘缩放，系统菜单及支持平台的 Snap；切换 DPI 后位置正确。
- 异形窗口：显示轮廓与透明阴影，分别选择只要求视觉形状或要求输入匹配，在不支持组合上获得明确结果。
- 文档工具与退出确认：工具窗口保持在文档窗口上方而不阻塞它；Owner 确认只阻塞父窗口，Application 确认约束整个应用；关闭时先结束原生展示再释放后代与父窗口。
- 原生文件面板：从普通窗口、当前模态窗口或无窗口托盘应用打开／保存；选择路径、取消和原生失败结果准确，文件名输入与后台 UI 编辑命令互不混淆。

## 13. 验收与文档同步

普通值、菜单模型、Binding、差量更新和组件行为使用所属 Cone 的 scoop.test；需要真实窗口的用例沿现有测试 Cone 组织。只有链接／ABI／诊断或确需外部进程控制的内容使用 fixture。普通验证使用 release，GC stress 只用于 callback／原生持有的定向正确性测试。

| 范围 | 必须覆盖的结果 |
| --- | --- |
| 状态与身份 | key 重排不重建存活项目；同名新实例不接旧事件；最新回调；Binding 拒绝／变换；隐藏窗口与无窗口时仍更新 |
| 菜单行为 | 子菜单、checkbox／radio、禁用、快捷键一次执行、编辑命令范围；tracking 中修改、取消、卸载和目标关闭 |
| 托盘 | 鼠标／键盘来源、矩形与 PointHint／未知区分、图标替换、DPI／主题；注册成功不冒充可见；Shell 重建与关闭竞争 |
| 托盘定位 | 多显示器与负坐标、目标 DPI、工作区、自动隐藏／overflow；显示前重查、边缘翻转／平移；无法查询或定位时明确失败／按显式策略降级 |
| 窗框 | 系统／Extended／Client 模式、系统按钮保留、Caption 排除交互子项、最大化后的边缘规则、Snap 与系统操作 |
| 形状与呈现 | alpha 正确；视觉／输入区域可不同；跨应用真实穿透；失败帧不发布区域；DPI／clip／变换与模式切换 |
| 关系与模态 | Owned 不冒充全局 topmost；raise 与焦点分开；Owner／Application、原生输入、嵌套恢复、隐藏／最小化和焦点恢复 |
| 文件面板 | 单选／多选／保存／目录、原生路径编码、覆盖确认、取消与失败；native 编辑目标正确；选择保存位置不写文件 |
| 模态收尾 | owner 关闭等待 owned 子窗口 lease 与原生面板退场；完成一次；I/O／timer／控制取消在真实模态循环中继续推进 |
| Linux 集成 | Wayland／X11 的 GTK 菜单、窗框区域与模态范围；GtkFileDialog／portal 完成与取消；SNI 服务存在、缺失和重启；保留无全局定位／topmost 等真实限制 |
| Cone 隔离 | 普通 window／UI／component 的解析与链接闭包不含 scoop.tray 或额外托盘库；显式托盘 consumer 能构建运行，缺失依赖在其所属 Cone 正常诊断 |
| 生命周期 | 重复 close、资源替换、回调异常、owner 关闭、活动原生循环退出；其他窗口错误隔离 |
| 工作量 | 纯原生状态变化不产生 UI 帧；未变化菜单不重建原生树；仅颜色变化不重测窗框；静止时无周期唤醒 |

跨进程穿透、真实用户拖动、Snap、混合 DPI、菜单栏切换和系统托盘行为使用真实桌面验证，必要时由外部测试程序参与；合成 InputArea 事件不能代替 OS 命中验收。无法验证的硬件／桌面条件明确记录，不把未执行写成通过。

Linux 验证区分 X11／XWayland、Wayland、桌面 Shell／托盘宿主、GTK／libadwaita 版本和实际 GPU。现有 Linux GUI 记录中的 GNOME Wayland 多窗口隐藏重显后 GtkGLArea 分配限制仍需保留，不能用原有软件显示测试通过或新 demo 可启动替代全部硬件验收；本设计不为该上游问题引入延迟重试。

实施时需要同步的规范与消费者：

| 改动 | 同步文档／调用方 |
| --- | --- |
| 菜单、native 使用、应用资源登记、桌面定位、窗框、模态与文件面板 | SCOOP-WINDOW-SPEC、windowing 设计与 WINIT-MAPPING、window／asyncio 测试及示例 |
| TrayIcon、托盘事件／能力／定位、独立依赖与退出 | 新增 SCOOP-TRAY-SPEC；scoop.tray README、Cone 与准备／打包入口、各平台测试 |
| alpha、区域转换与 Accepted 发布 | SCOOP-SURFACE-SPEC；实际新增 graphics 转换入口的所属规范；Surface renderer 与测试 |
| 应用根、scope、原生附属协议与错误 | SCOOP-UI-SPEC；UI 设计、生命周期与多窗口／无窗口测试 |
| NativeMenuContent、窗口／文件面板控制器、独立托盘包装与综合 demo | SCOOP-UI-COMPONENT-SPEC；明确 scoop.ui.component.tray 归属，更新各 Cone README、示例、拥有／借用边界和组合测试 |

本设计不要求修改语言语义、runtime GC 契约、编译 pipeline 或产物格式。原有 FFI／ForeignCallback／NativeSafe 合同直接适用。构建与链接变化限定为普通 Cone 的真实 native 输入与系统库。

## 14. 平台依据

以下资料于 2026-11-07 调研，用来解释设计边界；实施结果仍以目标 OS 上的实际验证为准。

窗口归属、模态和原生文件面板的官方依据集中在[配套设计第 9 节](WINDOWS-AND-DIALOGS.md#9-平台依据)。

- 当前 Linux 基线：[Window 规范第 12 节](../specs/SCOOP-WINDOW-SPEC.md#12-linux-wayland-与-x11-后端)、[Linux GUI 实施记录](../linux-gui/PROGRESS.md)、[GTK 宿主源码](../../sysroot/src/scoop.window/native/linux/window.c)。
- GTK：[GtkPopoverMenuBar](https://docs.gtk.org/gtk4/class.PopoverMenuBar.html)、[GtkPopoverMenu](https://docs.gtk.org/gtk4/class.PopoverMenu.html)、[GdkSurface.set_input_region](https://docs.gtk.org/gdk4/method.Surface.set_input_region.html)、[GdkToplevel.begin_move](https://docs.gtk.org/gdk4/method.Toplevel.begin_move.html)。
- 托盘依赖核对：[libayatana-appindicator 构建依赖](https://github.com/AyatanaIndicators/libayatana-appindicator/blob/main/CMakeLists.txt)。

- Apple：[NSMenu](https://developer.apple.com/documentation/appkit/nsmenu)、[NSApplication.mainMenu](https://developer.apple.com/documentation/appkit/nsapplication/mainmenu)、[NSStatusItem.menu](https://developer.apple.com/documentation/appkit/nsstatusitem/menu)。
- Apple：[NSWindow.performDrag(with:)](https://developer.apple.com/documentation/appkit/nswindow/performdrag(with:))、[contentLayoutRect](https://developer.apple.com/documentation/appkit/nswindow/contentlayoutrect)、[RoundTransparentWindow 示例](https://developer.apple.com/library/archive/samplecode/RoundTransparentWindow/Introduction/Intro.html)。旧示例仅证明公开形状路径的来源，不替代当前 Metal 组合验收。
- Microsoft：[SetMenu](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setmenu)、[TrackPopupMenuEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-trackpopupmenuex)、[TranslateAcceleratorW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-translateacceleratorw)。
- Microsoft：[Notification Area](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area)、[TaskbarCreated](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar#taskbar-creation-notification)。
- 托盘位置：[Shell_NotifyIconGetRect](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyicongetrect)、[NOTIFYICONDATA 的通知锚点](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw)、[NSStatusItem.button](https://developer.apple.com/documentation/appkit/nsstatusitem/button)、[NSWindow.convertToScreen](https://developer.apple.com/documentation/appkit/nswindow/converttoscreen(_:))。
- 系统条与工作区：[ABM_GETTASKBARPOS](https://learn.microsoft.com/en-us/windows/win32/shell/abm-gettaskbarpos)、[MONITORINFO.rcWork](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfo)、[NSScreen.visibleFrame](https://developer.apple.com/documentation/appkit/nsscreen/visibleframe)、[NSMenu.menuBarHeight](https://developer.apple.com/documentation/appkit/nsmenu/menubarheight)。
- 窗口菜单栏：[GetMenuBarInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmenubarinfo)、[GetMenuItemRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmenuitemrect)。
- Linux 托盘：[StatusNotifierItem 的 Activate／ContextMenu 定位提示](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem/)。
- Microsoft：[Custom Window Frame](https://learn.microsoft.com/en-us/windows/win32/dwm/customframe)、[DWM Window Attributes](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)、[Snap Layout](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-snap-layout-menu)。
- Microsoft：[SetWindowRgn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn)、[WM_NCHITTEST](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)、[Layered Windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows)。
- X.Org：[Shape 扩展](https://www.x.org/releases/current/doc/xextproto/shape.html)；freedesktop：[EWMH 移动／缩放](https://specifications.freedesktop.org/wm-spec/latest/ar01s04.html)。
- Wayland 上游协议镜像：[wl_surface／wl_region](https://github.com/wayland-mirror/wayland/blob/main/protocol/wayland.xml)、[xdg-shell](https://github.com/wayland-mirror/wayland-protocols/blob/main/stable/xdg-shell/xdg-shell.xml)、[xdg-decoration](https://github.com/wayland-mirror/wayland-protocols/blob/main/unstable/xdg-decoration/xdg-decoration-unstable-v1.xml)。
