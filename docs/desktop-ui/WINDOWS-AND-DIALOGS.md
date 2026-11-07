# 窗口归属、模态与原生文件对话框设计

日期：2026-11-07。状态：按批次实施。本文补充[原生桌面能力与 UI 组件设计](DESIGN.md)，覆盖原生顶层窗口之间的归属、相对叠放、模态，以及系统打开／保存文件对话框。D5 与 D6 的实际 API 分别以[窗口归属与模态规范](../specs/SCOOP-WINDOW-MODAL-SPEC.md)和[文件对话框规范](../specs/SCOOP-WINDOW-DIALOGS-SPEC.md)为准；各平台验证及 UI 包装的交付状态见[实施记录](PROGRESS.md)，尚未交付部分仍是拟定合同。

实施平台与主设计一致：Darwin 两个 target、Windows 三个 target，以及已合入的 Linux GNU x86_64 Wayland／X11 后端。Linux 复用 GTK 4.18+／libadwaita 1.4+ 与 GtkGLArea，现状和验证边界见 [Linux GUI 实施记录](../linux-gui/PROGRESS.md)。沿用 [Window 规范](../specs/SCOOP-WINDOW-SPEC.md)、[Surface 规范](../specs/SCOOP-SURFACE-SPEC.md)和 [UI 规范](../specs/SCOOP-UI-SPEC.md)，实施各批次时先同步所属规范。

## 1. 使用场景与已有基础

| 场景 | 所需的原生行为 | UI 的职责 |
| --- | --- | --- |
| 主窗口打开退出确认 | 确认窗口关联主窗口，显示期间按所选范围限制输入，关闭后解除模态并请求恢复焦点 | 绘制确认内容，决定继续退出或取消 |
| 浮动工具窗口 | 工具窗口保持在所属窗口上方，父窗口仍可交互，其他应用的普通窗口可以遮住这组窗口 | 工具内容与业务状态 |
| 临时把窗口提到前面 | 一次调整叠放顺序，不因此持续置顶 | 发起操作 |
| 持续置顶的小窗口 | 请求 OS 的 topmost／floating 层级，不因此阻塞其他窗口 | 显示内容与置顶开关 |
| 打开／保存文件或选择目录 | 系统面板负责原生导航、文件选择和确认，结果关联到发起请求的窗口或应用 | 消费路径，随后执行真正的文件 I/O |

设计基线的 `createWindow` 只创建独立、非模态的顶层窗口，已提供 `WindowLevel.Normal／AlwaysOnTop／AlwaysOnBottom` 和 `requestFocus`。D5 在这一基础上加入 owner、工具窗口、raise 与公开模态 session；原生文件对话框归 D6。基线中的 `runModalForWindow／MessageBox` 测试只证明嵌套原生循环可推进，不能代替各功能的独立实现与验证。

本文将用户场景中的 model 按 modal（模态）处理。top-level、一次 raise、持续 topmost、owned 与 modal 是不同维度，不能合成一个互斥的窗口类型枚举。

## 2. 顶层窗口、归属与叠放

### 2.1 关系与角色

拟在 `WindowAttributes` 增加 `WindowRelation.Independent／Owned(ownerWindow)`，创建后提供只读 `Window.ownerWindow`。这里的 ownerWindow 是业务窗口，不是已有的 Application owner 线程或内部 `WindowOwner` 对象。

Owned 仍是拥有自己内容区、DPI、桌面位置、Surface 和 WindowId 的顶层窗口。它不裁剪在父窗口客户区内，也不使用父客户区坐标。第一阶段不增加嵌入式 native child window：Win32 的 `WS_CHILD／SetParent` 不适用于这里的工具窗口和退出确认。

关系固定于创建时：owner 必须属于同一个运行中的 Application，且未进入 Closing；新窗口不能形成环。第一阶段不提供运行中换 owner，避免把未要求的重挂接、焦点迁移和 native lease 迁移一起引入。

窗口角色拟使用 `WindowRole.Normal／Tool`。Tool 请求系统的工具面板外观和相应窗口切换器约定，不自动施加模态或全局置顶；是否在任务栏／窗口切换器中出现按实际原生能力描述。非模态工具窗口的关联依靠 Owned，Tool 是额外的外观／角色请求；后端没有等价角色时，调用方可明确选择普通外观。`WindowFrame`、resizable 与透明度仍是独立配置。

### 2.2 三种叠放操作

| 意图 | 公共表达 | 保证与限制 |
| --- | --- | --- |
| 保持在所属窗口上方 | `WindowRelation.Owned(ownerWindow)` | 使用原生 owned／transient 关系，能力中区分系统维持与 WM 提示；不依靠失焦后反复 raise |
| 一次提到前面 | 新增 `Window.requestRaise(): RequestStatus` | 请求一次调整顺序；不隐含键盘焦点或持续 topmost |
| 持续全局置顶 | 已有 `setWindowLevel(AlwaysOnTop)` | 请求高于普通窗口的原生层级；不保证高于系统界面、安全桌面或其他所有 topmost 窗口 |

`requestFocus` 继续单独表达激活请求。后台程序能否激活或 raise 由 OS 决定，Requested 不能报告成已经获得焦点。纯工具窗口通常采用 Owned + Tool + Normal；点击主窗口不会使其遮住工具窗口，也不需要把工具窗口提升到全局 topmost。

Owned 的 Normal 表示不额外请求全局置顶，相对 owner 的原生约束仍然有效。Win32 将 owner 变为 topmost 时会影响 owned 窗口，不能声称这些层级完全独立；涉及已有 owned 窗口的层级修改必须报告实际影响。Owned 与 AlwaysOnBottom 等冲突组合不能破坏“在 owner 上方”的合同，按具体能力拒绝不成立的组合。

Linux 通过 gtk_window_set_transient_for 建立实际关系，由 GTK/GDK 传给 X11／Wayland。窗口管理器仅把它视为提示时，应如实说明该约束，不能把库保存的 ownerWindow 当作已观察到的相对 Z 序。严格要求保持相对顺序的调用方按能力选择；没有原生保证时不偷偷升级为全局 topmost。

相对叠放不等于跨平台一致的移动跟随：AppKit child window 会随 parent 移动，Win32 owned window 通常不会。第一阶段保留并说明这种原生差异，不为统一跟随效果建立窗口管理器。应用若要自定义定位，使用已有位置能力及主设计中的托盘锚定入口。

owner 隐藏或最小化时，所属工具窗口应暂时退出可见交互；owner 恢复后只恢复此前希望显示的窗口，不复活已被显式隐藏或关闭的工具窗口。后端分别利用原生行为与必要的可见性协调，不把临时隐藏覆写成调用方的新显示意图。

## 3. 模态的范围、展示与 session

### 3.1 输入范围与展示方式分开

非模态 Owned 窗口使用普通显示操作；需要模态时，对已创建的隐藏窗口调用拟定的 `showModal(options)`，返回 `ModalSession`，不在 Scoop 调用栈上同步等待用户选择。

| 值 | 语义 |
| --- | --- |
| `ModalScope.Owner` | 阻塞该窗口的直接 owner；需要有效的 ownerWindow。其他独立窗口仍可交互 |
| `ModalScope.Application` | 阻塞同一 Application 的其他窗口；嵌套时仅当前可交互的模态窗口／面板接收输入，不影响其他应用 |
| `ModalPresentation.Automatic` | 后端选择能保持所选 scope 的原生展示方式 |
| `ModalPresentation.SeparateWindow` | 请求独立外观的模态顶层窗口 |
| `ModalPresentation.Sheet` | 请求附着在 owner 上的原生 sheet |

Owner 的范围只指指定 owner，不自动传播到所有兄弟工具窗口；这与 Win32 owned 和 AppKit document sheet 的基本行为一致。业务需要阻塞整个应用时选择 Application。当前不增加任意窗口集合、排除名单或按控件过滤的模态规则；窗口内局部遮罩仍由现有 UI 输入作用域处理。

Automatic 只选择展示方式，不改变输入范围。例如在 macOS 上不能因独立 owner-modal 难以实现，就静默调用 `runModal(for:)` 扩大为应用模态。明确请求 Sheet 的后端若不能提供 sheet，则返回 Unsupported，不能把普通浮动窗称为 sheet。

同一直接 owner 同一时间只接受一个直接的活动模态请求，重复请求报告 ResourceBusy，不隐式排队。两个独立主窗口可以分别有 Owner 模态。嵌套请求应以当前可交互的模态窗口为 owner，例如 A → 确认窗口 C → 文件对话框 F；是否能采用嵌套 sheet 是独立的平台组合能力。

Owner 请求要求目标可见、未最小化且未被其他模态阻塞；调用方应先明确显示／恢复目标，再发起请求。不可用的 owner 不会被悄悄换成应用目标；native 命令真正执行前若目标已经关闭，则结束这次请求并释放使用。

同一应用的 Application 模态按嵌套活动分支处理，不允许两个互相阻塞的平级应用模态。新窗口若在 Application 模态期间创建，也必须从创建／显示起遵守当前输入范围。

### 3.2 原生输入与菜单

模态限制由 `scoop.window` 在原生窗口和输入入口落实，覆盖普通客户区输入、编辑目标资格，以及发往被阻塞窗口的快捷键／菜单命令；不能只让 UI 按钮回调提前 return。原生窗框遵守系统的模态策略，例如 sheet 期间仍可能允许移动父窗口，不能把模态解释为冻结所有窗口管理操作。paint、renderer 完成、业务状态更新、I/O、定时器和关闭控制继续推进。

在进入模态边界时，按现有输入合同结束捕获与按下序列，处理 IME 焦点离开；不伪造一次字符提交或 click。阻塞判断使用已发布的原生状态，同步 OS 回调不调用 managed render 或可挂起业务函数。

模态关系可以重叠，例如 C 阻塞 A，而 F 又阻塞 C。解除 F 时只释放 F 对 C 的限制，A 仍受 C 的限制；不能对每次完成无条件执行 `EnableWindow(TRUE)`。恢复时也须保留此前已有的原生禁用状态。

原生文件面板中的文本字段使用系统 responder／控件的 Cut、Copy、Paste、Undo 和 IME。主设计中的标准菜单角色要先判断实际原生编辑目标；只有目标属于 Scoop 内容区时才转为 `TextEditCommand`。不能把文件名栏的粘贴送到后台编辑器，也不能用后台 UI 的 canExecute 结果禁用原生面板的编辑菜单。

应用级命令按其真实目标判断：Quit 仍产生退出请求；针对某个被阻塞窗口的保存、关闭等命令不得绕过模态。用户主动操作其他可交互窗口时，命令来源固定到事件发生时的窗口，不在排队后重新按焦点猜测。

### 3.3 Session 的完成与窗口所有权

`ModalSession` 借用实际 Window，持有独立的 ModalSessionId，登记本次原生展示与输入限制。调用方先创建隐藏窗口、挂载内容，再显示；创建、挂载或显示失败都必须回收本次已创建资源。

session 从等待 native 命令执行到展示，再到结束；只记录一次终态。`dismiss()` 幂等地结束模态展示并隐藏窗口，结束原生使用后完成；它不直接销毁借用的 Window。需要回收窗口时再走 `requestDestroy`，UI 控制器按第 7 节代为安排。

对活动模态窗口显式隐藏或 requestDestroy 会请求结束其 session；不能留下不可见却持续阻塞 owner 的窗口。用户点击关闭按钮仍先进入已有 CloseRequested 决策；普通模态并不强迫业务接受关闭。确认内容的业务结果由上层控制器表达，低层只记录正常结束、取消原因或原生失败。

显式隐藏／最小化 owner 前，先结束依赖它展示的 Owner 模态和文件面板；应用整体被系统暂时隐藏、切换桌面或失去前台则沿用原生组的隐藏／恢复，不据此产生一次用户取消。

完成后请求恢复到仍存在、可见且可交互的 owner 或上一层模态窗口；先解除本次限制，再发起恢复。OS 可能拒绝后台激活，不提供“强制抢回焦点”的保证。

## 4. 平台映射与能力边界

| 能力 | Win32 | AppKit | Linux GTK／X11／Wayland |
| --- | --- | --- | --- |
| Owned 顶层关系 | 创建顶层窗口时传入 owner HWND；不用 WS_CHILD | `addChildWindow(_:ordered:)`，顺序为 above；与 sheet 关系统一管理 | GtkWindow.transient-for，经 GDK 传递实际协议；遵守 WM／compositor 策略 |
| Tool 角色 | WS_EX_TOOLWINDOW 等样式 | NSPanel／工具窗口配置；floatingPanel 不等于指定 owner | GTK4 没有适用于所有后端的统一工具窗角色；X11 可核验 UTILITY 提示，Wayland 不伪造等价能力 |
| 一次 raise | SetWindowPos 的相应顺序请求，保留 NOACTIVATE | orderFront 等公开操作，焦点请求另行处理 | X11 按原生顺序请求实现；GTK present 可能同时请求激活，不能冒充不激活的 raise；Wayland 无通用全局 raise |
| 持续 topmost | HWND_TOPMOST 与 owner 传播规则 | NSWindow.level 的 floating 等层级 | 复用现有 X11 ABOVE 请求；Wayland 无通用保证，不以 GTK API 名称推定支持 |
| Owner 模态 | Owned 顶层窗口及实际输入禁用 | 优先 document-modal sheet；独立 Owner 模态不能由 app-modal 冒充 | transient 加受限 GtkWindowGroup／原生输入范围；单独 set_modal(true) 不能保证只阻塞 owner |
| Application 模态 | 对同应用其他窗口原生禁用与恢复 | AppKit 应用模态 | GTK 模态及窗口层对当前所有窗口／分组的协调，保持实际应用范围 |
| Sheet | 无等价的标准 Win32 sheet | NSWindow.beginSheet／endSheet | 不把 GTK 普通对话框或 popover 声称为 AppKit sheet |

能力沿主设计的查询入口扩展：关系支持、Tool 角色、raise、层级组合，以及指定 scope／presentation 的可用组合分别表达。不要只增加一个 `supportsModal`；能力快照也不代替目标正在关闭、已被其他 session 占用等实际状态检查。

AppKit 的 beginSheet 可能将第二个 sheet 自动排队，本库在调用前落实 ResourceBusy。beginCriticalSheet 可以压在已有 sheet 上，但它表达 critical sheet；普通嵌套对话框不能无条件借它改变语义。D1 须验证“模态窗口中再打开文件面板”、sheet 与自绘窗框的组合，再将确认过的展示组合写入规范。

当前 AppKit 自定义 NSWindow 子类允许成为 key／main window，新增模态后必须与系统模态资格协调，不能继续无条件绕过限制。sheet 的样式、标题显示与位置由 AppKit 决定；若业务必须保持独立自绘外观，应请求 SeparateWindow，并接受该 scope／presentation 组合的真实结果。

Owned、sheet 及原生 panel 的 native 关系应由同一后端登记管理，避免为同一个窗口重复建立互相冲突的 child 与 sheet 关系。这是现有窗口 owner 数据的扩展，不新增独立窗口管理服务。

GtkWindowGroup 只限制组内的 GTK 模态／grab，一个窗口同时只能属于一个组。Linux 实现必须验证两个独立主窗口、兄弟工具窗口、嵌套模态以及从 Owner 转入 Application 模态时的范围，不能仅设置一个 modal 布尔值。分组是后端细节，不增加公共窗口分组框架；GTK 输入资格、命令资格和恢复仍由同一份活动 session 状态决定。

Linux 的临时窗口默认不启用 gtk_window_set_destroy_with_parent 的自动销毁行为；由 Scoop 先结束后代展示、释放 GtkGLArea／lease，再销毁窗口。原生 transient 关系不会替代第 6 节的资源关闭顺序。

## 5. 原生文件对话框

### 5.1 第一阶段接口与结果

文件对话框属于 `scoop.window` 的应用／桌面服务，可以从窗口或无窗口托盘应用发起。它们不创建公开的 Scoop Window，不给系统文件面板挂载 Surface，也不允许设置任意 WindowFrame 或 topmost 层级。

该服务不依赖 scoop.tray。Linux 复用窗口库已经需要的 GTK/GIO；仅有托盘图标及其额外依赖归独立托盘 Cone。

拟提供以下入口；`DialogTarget.Owner(window)` 表示关联并阻塞该窗口，`DialogTarget.Application` 表示应用模态，也适用于没有业务窗口的应用。省略 owner 不能使阻塞范围含糊不清。

| 拟定入口 | 成功值 | 用途 |
| --- | --- | --- |
| `ActiveApplication.openFile(target, options)` | `FileDialogSession<Path>` | 选择一个现有文件 |
| `ActiveApplication.openFiles(target, options)` | `FileDialogSession<Array<Path>>` | 多选，Accepted 的数组非空 |
| `ActiveApplication.saveFile(target, options)` | `FileDialogSession<Path>` | 选择保存位置与文件名 |
| `ActiveApplication.selectDirectory(target, options)` | `FileDialogSession<Path>` | 选择一个目录 |

路径复用 `scoop.platform.fs.Path`，文件名使用现有 `OsString`，不新增 UI Path。创建返回 session；参数、归属、已关闭目标和不支持的必需选项可在创建时失败，已经受理的 native 失败记录到该 session 的完成结果。

结果为 `Accepted(value)／Cancelled(reason)／Failed(WindowException)`。取消原因包括用户取消、显式 cancel、owner 关闭和应用退出；用户按 Escape 或取消按钮不是异常。每个 FileDialogSessionId 只完成一次，接受和取消竞争时不能同时交付两种结果。

ApplicationEvent 增加按 ModalSessionId／FileDialogSessionId 归属的事件。文件 session 保存对应类型的结果，完成通知通过身份找到该 session，不把路径降为 String／Any，也不伪装成某个无关窗口的普通输入事件。

### 5.2 选项与文件语义

第一阶段只提供共同且明确的选项：可选标题、初始目录提示、保存时建议文件名，以及 `FileTypes.Any／Extensions` 表达的允许扩展名集合。多选由 openFiles 表达，目录选择使用独立入口，不在参数中堆叠相互冲突的布尔值。

初始目录与建议文件名是系统展示的初始提示，系统历史记录及实际目录状态可能影响最终位置。扩展名是原生文件类型筛选；不承诺不同平台有相同的筛选下拉框。Win32 的多组命名筛选器、macOS 特有的 UTType 配置按实际需要另设明确平台选项，不能把 AppKit 没有的控件效果静默吞掉。

首批不加入任意 accessory View、异步 onFileOk 校验、批量目录选择或任意 Shell 虚拟对象。当前调用者需要可供普通文件 API 使用的文件系统路径，Windows 应请求 FOS_FORCEFILESYSTEM；不能为一个虚拟 Shell item 编造 Path。

保存面板负责原生覆盖确认和文件名展示，但 Accepted 只交付系统最终选择的路径。它不创建、截断、写入或预留目标文件，也不保证稍后的 I/O 一定成功。扩展名补全以系统返回的实际路径为准；应用仍负责自己的格式处理、读取或写入错误。

取回路径时使用平台已有的无损编码边界。Windows 的 UTF-16 经 `OsString.fromUtf16` 保留原生 code unit，再构造 Path；macOS 从文件 URL 的文件系统表示取得路径。不得经 display／有损字符串来回转换，不自动 canonicalize，不把 URL 的转义文本直接当成本地路径。

### 5.3 后端调用

| 平台 | 实现路径 | 关键约束 |
| --- | --- | --- |
| Windows | IFileOpenDialog／IFileSaveDialog，Show(owner HWND)，GetResult／GetResults | 复用 Common Item Dialog；FOS_ALLOWMULTISELECT、FOS_PICKFOLDERS 等按入口配置；HRESULT_FROM_WIN32(ERROR_CANCELLED) 映射取消 |
| macOS | NSOpenPanel／NSSavePanel；Owner 使用 beginSheetModal，Application 使用真正的 app-modal 入口 | `begin(completionHandler:)` 是 modeless，不能用它宣称实现 Application 模态；runModal 的阻塞留在 native 命令边界 |
| Linux | GtkFileDialog 的 open／open_multiple／save／select_folder 与对应 finish | 使用 GTK 自身的 portal／原生选择器路径；传真实 GtkWindow parent 与 GCancellable，结果回到现有 GMainContext |

Windows 的文件面板在现有 UI owner 线程使用 COM STA。由后端负责实际需要的 CoInitializeEx／CoUninitialize 配对，S_OK 与 S_FALSE 都需要配对释放；RPC_E_CHANGED_MODE 是初始化失败，不临时另起 UI 线程绕过。所有 COM 对象在正确 apartment 使用和释放，复制结果后释放 IShellItem／数组及系统分配的字符串。

`IFileDialog.Show` 会进入原生模态循环，但公共创建函数仍立即返回 session。Owner 目标使用对应 HWND，无业务窗口的 Application 目标复用现有隐藏顶层宿主 HWND，不创建伪造的公开 Window。取消通过 owner 控制路径调用 IFileDialog.Close；Close 后不再调用 GetResult／GetResults。应用模态还需覆盖同应用其他窗口，不能把 Show(hwndOwner) 自身的 owner 模态当成 Application 范围。

AppKit 在显示前配置文件／目录选择、多选、允许内容类型和初始值；使用 allowedContentTypes 时纳入实际所需的 UniformTypeIdentifiers 依赖。sheet 完成回调执行时面板可能仍在屏幕上，必须完成必要的 orderOut／结束展示后才通知可继续打开下一层窗口。

GTK 4.18 基线已包含 GtkFileDialog，无需再实现一套 portal 客户端，也不引入 GTK3 文件面板。Owner 目标传对应 GtkWindow；其 modal 选项表达对父窗口的限制，Application 范围仍由窗口层覆盖其他窗口。无 owner 时传空 parent，不为文件面板创建虚假的业务窗口。验证 GTK 在实际 portal 与本地选择器路径中的范围，保持同一公共结果。

Linux 取消使用 GCancellable，异步 finish 回调复制 GFile／GListModel 结果后释放原生资源并排队完成事件。通过 GFile.get_path 获取原生文件系统路径；若系统允许选择无本地路径的资源，报告明确的 Unsupported 结果，不把 URI 字符串伪装成 Path。用户取消与显式取消沿 session 原因映射，GTK/GIO 失败保留对应错误域和 code。

公共文件对话框能力分别描述四个入口、owner／application 目标及实际支持的选项。原生服务当前不可用是 Unavailable，COM 或系统调用失败保留原始错误；不能把平台不支持、本库未实现和用户取消混为同一个 None。

## 6. 原生循环、关闭与 native lease

所有对象仍归属同一个 Application owner。可能进入阻塞式原生循环的 Show／runModal 等命令，必须等当前业务 handler 和 managed session pump 返回后执行。原生循环内通过已有 CFRunLoop source、隐藏 HWND、watch 和控制消息推进任务；Linux 的 GTK／GIO 异步操作则直接复用现有 GMainContext／poll，不递归运行 managed pump，不增加独立 UI 线程或 GtkApplication.run。

GTK signal 和异步完成回调只更新 native 状态、复制结果或排队，业务 handler 在 GMainContext dispatch 之外执行。关闭时继续派发取消完成与资源退休，不能在 GtkFileDialog 回调仍持有 session 时释放 owner。现有 eventfd、deadline、watch 与按需 frame clock 的行为保持。

嵌套模态需要允许在上一层原生循环存活期间，处理下一次正常回调提交的原生命令；每次都遵守 managed 派发已退出的边界。取消、owner 关闭与退出是控制操作，不能排在“等待整个模态循环先退出”的普通任务后面。没有待处理事件时仍可休眠，不靠零间隔定时器轮询 session。

`scoop.window.asyncio` 可以提供 ModalSession／FileDialogSession 的 suspend 等待包装，复用现有 Job 和关闭协议。协程取消要请求结束实际原生展示；调用者不再等待，不能意味着 native session 立即被释放。原生持有解除前，Application 收尾仍须等待它。

窗口归属会改变现有销毁条件。当前 shutdown 只检查该窗口已递送 Closing 且自身 leases 为零；Win32 会在 owner HWND 销毁时自动销毁 owned HWND，因此仅检查父窗口自己的 lease 不够。

owner 的 requestDestroy 或应用退出应按以下顺序完成：

1. 将 owner 及实际 owned 后代纳入 Closing，阻止新展示和新 lease；向所属 UI 递送 Closing，使其撤销事件登记并关闭 Surface。
2. 取消相关 popup、ModalSession 与 FileDialogSession；让 native 控制和已有 asyncio 收尾继续推进。被阻塞不妨碍 Closing／Destroyed 等控制事件。
3. 从后代到 owner 解除原生展示，等待各窗口的 lease 归零，以及借用这些窗口的 Show／sheet 调用安全结束。
4. 先销毁 owned 子窗口，再销毁 owner；最后按现有顺序释放 Application 资源并停止主循环。

应用显式终止时不重新向每个后代询问是否愿意关闭。退出确认必须发生在 CloseRequested／ExitRequested 的决策阶段：用户同意后才 requestDestroy／requestExit。进入 Exiting 后再打开确认窗口会与关闭协议冲突，应拒绝。

Modal／FileDialog 完成、重复取消、owner 提前关闭、native 错误及异常均只收尾一次。卸载或退出后，内部终态与资源释放仍执行，但不再调用已退休 UI 回调；关闭错误沿现有 ResourceUseException 规则合并。

## 7. UI 包装与使用方式

`scoop.ui` 只增加必要的资源事件与模态观察桥接；系统输入限制和焦点恢复留在 window。原生模态 Window 的内容仍可使用普通 UI 组件、WindowChrome 和透明形状，原生窗口不意味着其中的确认按钮也必须由 OS 绘制。

组件层提供以下窄入口，不引入声明式管理所有顶层窗口的通用树：

| 拟定包装 | 所有权与触发方式 |
| --- | --- |
| `nativeToolWindowController` | 归属创建它的窗口 scope；显式 open 创建 Owned 非模态窗口和 UiWindow，按能力与显式策略选择 Tool 或普通外观；卸载关闭所拥有窗口 |
| `nativeDialogController<T>` | 归属窗口或应用 scope；显式 open 创建隐藏的 Window、挂载内容并开始 ModalSession；内容通过控制器提交类型化结果或取消 |
| `fileDialogController` | 管理一次文件面板请求、pending 状态与类型化结果；接受明确的 DialogTarget |
| `nativeOpenFileButton／nativeSaveFileButton` | 使用普通 UI 按钮样式，在用户动作中调用文件控制器；不另实现文件浏览界面 |

这些入口在名称和所有权上与 `UiHost.attach` 区分：attach 继续借用调用方的 Window；控制器只销毁自己创建的窗口。工具／对话框控制器关闭时先结束 session，关闭所持 UiWindow，再 requestDestroy；实际 native 销毁仍受第 6 节约束。

控制器创建和首次打开在所属业务更新边界执行，窗口级回调沿对应 SurfaceWindow.dispatch，应用级请求沿应用根错误边界；任一步失败都回收已创建的 session／UiWindow／Window。对话框内容的结果提交与原生完成合并为一次结果，完成前不复用该控制器的活动窗口。

render 只更新选项、可用状态与当前回调，不打开面板。打开来自用户动作或显式请求；pending 时重复请求返回忙，系统取消后不会因旧 State 仍为 true 而自动重开。scope 卸载取消请求，窗口 scope 不会偷偷把失效 owner 替换为 Application 目标。

普通 `Dialog／AlertDialog` 继续表示同窗口 Portal；nativeDialogController 创建真实顶层窗口，二者不通过一个会静默降级的 native 布尔值混用。工具窗口复用普通 UI 内容，不因非模态就失去 owner 生命周期约束。

典型调用流程：

| 场景 | 组合 |
| --- | --- |
| 主窗口退出确认 | 拦截 CloseRequested → 打开 Owned 确认窗口，Owner + Automatic → Cancel 只关闭确认 → Confirm 结束确认后 requestDestroy 主窗口 |
| 应用退出确认 | 接到 ExitRequested → Application 模态确认 → 用户同意后 requestExit；拒绝则继续运行 |
| 文档工具面板 | 以文档窗口为 owner，按能力选择 Tool 外观，Normal 层级与普通 show → 主窗口仍可交互；关闭主窗口时有序关闭工具面板 |
| 在确认窗口中选文件 | DialogTarget.Owner 指向当前可交互的确认窗口 → 文件 session 完成只恢复该确认窗口 → 不提前释放外层主窗口的模态限制 |
| 托盘命令打开文件 | 应用级 scope 显式使用 DialogTarget.Application；没有业务 Window 也能管理结果和取消 |

## 8. 实施落点与验收

主设计的 D5 负责窗口关系与模态，D6 负责原生文件对话框，D7／D8 接入 UI 与组件，最后 D9 通过一个综合 demo 完整展示本组能力。Linux 与其他平台同轮实施；D1 确认 GTK 模态分组／portal、独立 Owner 模态、嵌套 sheet、无窗口文件面板与窗框组合，以及实际派发中的取消和 I/O。未验证不能写成已支持，也不能用 Unsupported 掩盖尚未实施。

| 位置 | 本次新增职责 |
| --- | --- |
| window 共享源码／native ABI | 关系与角色、ModalSession／FileDialogSession、类型化事件、能力、输入范围及关闭依赖 |
| Darwin native | child／sheet 关系、模态资格、NSOpenPanel／NSSavePanel、完成与取消、原生编辑 responder 路由 |
| Windows native | owner HWND／工具样式、原生禁用恢复、一次 raise、COM STA 与 Common Item Dialog、依赖原生调用的销毁顺序 |
| Linux native | GTK transient／模态范围、分组与输入恢复、GtkFileDialog／GCancellable／GFile、GDK 能力及 GtkGLArea 安全关闭 |
| window.asyncio | session 等待和取消，既有 native loop 中的持续推进 |
| UI／component | 有限的拥有窗口控制器、文件面板触发器、当前回调、pending 状态和 scope 清理 |
| Cone 配置 | 实际新增 native 文件及系统链接项；Linux 窗口／文件面板复用现有 GTK/GIO，不引入独立 scoop.tray 的额外依赖 |

必要验收围绕以下组合组织，不按每个按钮重复全套编译器 fixture：

- A、B 两个独立主窗口：A 的 Owner 确认只阻塞 A，B 仍可操作；Application 模态则阻塞两者，关闭后恢复正确。
- Owned 工具窗口不会被 owner 遮住；普通 raise、Owned 相对叠放与 AlwaysOnTop 可区分；隐藏、最小化、恢复和 owner 层级变化符合实际合同。
- A → C → F 嵌套时，F 结束只恢复 C；关闭 A 时，C 的 Surface lease 和 F 的 native 调用未释放前，A 的原生句柄不会提前销毁。
- 真实原生窗框、键盘、快捷键、菜单和 IME 遵守模态；文件名字段的编辑命令由系统处理，不改变后台 UI 编辑器。
- 单选、多选、目录、Unicode／原生路径、新文件名、已有文件覆盖确认与筛选；取消不产生 Accepted，选择保存位置不执行文件写入。
- Escape、关闭按钮、显式 cancel、scope 卸载、owner 关闭、应用退出及 native 错误都只产生一个终态；旧回调不因重新打开而被复用。
- 模态／文件面板停留期间真实 I/O、定时器与必要重绘继续推进；控制取消可达、managed pump 不重入、空闲无轮询。
- Linux Wayland／X11 分别验证 GTK 分组及模态范围、portal／本地文件选择、GFile 路径与取消；将 WM 提示、缺少全局定位／topmost、已记录的 GtkGLArea 限制与未执行的桌面条件分开报告。

同步 SCOOP-WINDOW-SPEC 的关系、输入范围、事件、原生使用与退出合同，UI／组件规范的拥有与借用边界，以及相关示例和测试。Path／OsString 复用现有平台合同；只在实际增加平台入口时修改其规范。本次不需要语言、GC ABI 或编译 pipeline 变更。

## 9. 平台依据

以下官方资料于 2026-11-07 核对；设计未执行真实桌面验收。

- Microsoft：[Owned Windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#owned-windows)、[SetWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)、[EnableWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enablewindow)。
- Apple：[addChildWindow(_:ordered:)](https://developer.apple.com/documentation/appkit/nswindow/addchildwindow(_:ordered:))、[NSWindow.level](https://developer.apple.com/documentation/appkit/nswindow/level-swift.property)。
- Apple：[beginSheet](https://developer.apple.com/documentation/appkit/nswindow/beginsheet(_:completionhandler:))、[beginCriticalSheet](https://developer.apple.com/documentation/appkit/nswindow/begincriticalsheet(_:completionhandler:))、[NSApplication.runModal](https://developer.apple.com/documentation/appkit/nsapplication/runmodal(for:))、[worksWhenModal](https://developer.apple.com/documentation/appkit/nswindow/workswhenmodal)。
- Microsoft：[Common File Dialog](https://learn.microsoft.com/en-us/windows/win32/shell/common-file-dialog)、[IModalWindow.Show](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-imodalwindow-show)、[IFileDialog.Close](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifiledialog-close)、[CoInitializeEx](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coinitializeex)。
- Apple：[NSOpenPanel](https://developer.apple.com/documentation/appkit/nsopenpanel)、[NSSavePanel.begin](https://developer.apple.com/documentation/appkit/nssavepanel/begin(completionhandler:))、[beginSheetModal](https://developer.apple.com/documentation/appkit/nssavepanel/beginsheetmodal(for:completionhandler:))、[runModal](https://developer.apple.com/documentation/appkit/nssavepanel/runmodal())。
- freedesktop：[FileChooser portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html)、[Request.Close](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html)。
- GTK：[set_transient_for](https://docs.gtk.org/gtk4/method.Window.set_transient_for.html)、[set_modal](https://docs.gtk.org/gtk4/method.Window.set_modal.html)、[GtkWindowGroup](https://docs.gtk.org/gtk4/class.WindowGroup.html)、[GtkFileDialog](https://docs.gtk.org/gtk4/class.FileDialog.html)、[open](https://docs.gtk.org/gtk4/method.FileDialog.open.html)。
- 本仓库：[现有窗口销毁条件](../../sysroot/src/scoop.window/src/owner/shutdown.scoop)、[AppKit runloop modes](../../sysroot/src/scoop.window/native/darwin/application.m)、[Path](../../sysroot/src/scoop.platform/src/fs/path.scoop)、[OsString 与 Windows 编码合同](../specs/SCOOP-PLATFORM-SPEC.md)。
