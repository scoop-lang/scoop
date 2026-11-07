# Scoop 原生菜单规范

日期：2026-11-07。本文为[桌面能力设计](../desktop-ui/DESIGN.md) D1／D2 的实施合同；各平台实际完成与验证状态见[实施记录](../desktop-ui/PROGRESS.md)。所属普通 Cone 为 scoop.window，线程、异常、坐标、ForeignCallback 与关闭规则沿用[窗口规范](SCOOP-WINDOW-SPEC.md)。

## 1. 模型与身份

ActiveApplication.createMenu() 创建空 Menu。Menu 提供 insert(description, parent = None, index = None)、update(item, description)、move(item, index)、remove(item)、items(parent = None)、description(item)、close() 和 state。parent 为同一 Menu 中的 Submenu；None 表示根，index 是对应父项子列表的 Long 下标，None 表示追加。move 只改变同一父项下的顺序。移除父项递归退休其子项；仍有子项的 Submenu 不能更新为其他类型。

MenuItemDescription 是不可变显示值，包含 text、kind、enabled、image、shortcut 和 command。MenuItemKind 为 Action、Checkbox(checked)、Radio(checked)、Submenu、Separator；MenuCommand 为 Action 或 Role(MenuRole)。角色包括 Quit、Minimize、Zoom、Cut、Copy、Paste、SelectAll、Undo、Redo，只适用于可激活项。子菜单与分隔符没有命令、快捷键或选中状态；分隔符没有文字或图像。文字不含 U+0000。checked 由应用发布，原生点击不建立另一份业务状态。

MenuId、MenuItemId、MenuBarBindingId、PopupSessionId、NativeMenuUseId 为不同值类型；一次 Application 中各类编号不复用。原生窄 command ID 与公共 Long ID 分别映射，不截断公共身份。普通属性和顺序变化保留 MenuItemId，删除后插入分配新 ID。原生树不保存业务闭包。

DesktopImage 由逻辑尺寸、非空的 DesktopImageRepresentation 数组和 template 标志构成。每份 representation 包含正的 PhysicalSize 和对应数量的不可变 RGBA8 字节，采用 straight alpha。尺寸与实际字节数在构造时验证，原生端复制所需存储；不依赖 Skia、Surface 或 GPU Image。不同分辨率表达相同逻辑图像。template 为平台支持时按系统外观着色的提示，不改变原始字节。

所有创建、读取、修改和 close 在应用 owner 线程进行。MenuState 为 Open／Closing／Closed；进入 Closing 立即停止新的使用与动作，实际原生使用安全退休后变为 Closed。GC 不替代 close；Application 退出关闭它创建的所有 Menu。

## 2. 能力与原生使用

ActiveApplication.desktopCapabilities().menus 返回 MenuCapabilities，分别描述 applicationMenuBar、windowMenuBar、popup 和 templateImages。AppKit 提供应用菜单栏；Win32 与 GTK4 提供每窗口菜单栏。所有三个后端提供窗口 popup、五种条目、图像和上述角色。特定快捷键能否准确转换通过 Menu 对其实际描述的验证确定，不能用标签冒充绑定。

ActiveApplication.attachMenuBar(menu) 返回应用级 MenuBarBinding；Window.attachMenuBar(menu) 返回窗口级 MenuBarBinding。不支持的目标抛出 WindowException(Unsupported)。一个目标同一时间只允许一个 binding；同一 Menu 只允许一个 binding、popup 或 native lease，冲突为 ResourceBusy。不同应用的对象不能组合。菜单栏根只含 Submenu，应用明确构造自己的应用／文件／编辑等入口。MenuBarBinding.close() 只解除附着；不关闭 Menu。

MenuBarBinding.id 为 MenuBarBindingId。isClosed 表示已请求关闭并停止动作；isDetached 表示原生 tracking 已结束、目标已实际解除附着。完成解绑后投递一次 ApplicationEvent.MenuBarDetached(id)，允许 UI 在原生使用退休后切换到新的菜单；同步解绑时可直接读取 isDetached，不必等待通知。退出期间通知可丢弃，实际状态与资源退休仍完成。目标尚未解绑时的新附着继续报告 ResourceBusy，不通过循环重试或固定延时猜测完成。

AppKit 使用 NSApplication.mainMenu；Win32 使用 SetMenu／DrawMenuBar；Linux 在现有 AdwToolbarView 中挂载 GtkPopoverMenuBar，复用 GMenuModel 和 GSimpleActionGroup。附着造成的内容分配变化沿原有 WindowMetrics 发布。

原生菜单结构更新只在不 tracking 时进行。tracking 中发布逻辑修改立即改变动作资格，不能再激活已删除、禁用或变更执行语义的项目；实际结构留到 tracking 结束应用，返回 Requested。无需延后的修改返回 Applied。修改前完成参数、归属和快捷键冲突检查；原生失败关闭受影响使用，按 WindowException 报告，不承诺 OS 无法提供的整树事务回滚。

## 3. 动作与快捷键

ApplicationEvent.Menu(MenuActivation) 只交付 MenuCommand.Action。MenuActivation 含 MenuId、MenuItemId、MenuSource；来源区分 ApplicationMenuBar(发生时的活动 WindowId?)、WindowMenuBar(WindowId)、Popup(PopupSessionId, WindowId) 和 Native(NativeMenuUseId)。原生事件排队时保存来源，不在稍后回调时按焦点重新选择。业务派发前再次检查仍有效的菜单项和原使用，关闭／退休不交付旧动作。

`Window.textCommandAvailability: TextCommandAvailability?` 声明 Scoop 内容区当前编辑目标的原生命令可用性。值包含 cut、copy、paste、selectAll、undo、redo（均默认 false），与 IME 开关独立；只读选择目标仍可复制。None 表示未提供内容区状态，沿用该窗口已有的 IME 目标判断；Some 全 false 明确禁用全部六个角色。原生文本字段继续由自身 responder／编辑控件验证，不使用后台 Scoop 状态。普通内容区按声明状态和 MenuItemDescription.enabled 共同判断，实际命令仍经 WindowEvent.TextCommand 交付；声明不替代当前输入资格检查。

MenuCommand.Role 由原生目标执行，不再交付一份普通 Menu 动作。Quit 产生 ExitRequested(User)，由应用决定是否开始有序退出。Minimize／Zoom 使用发生时的有效原生窗口。编辑角色针对 Scoop 内容时产生独立 WindowEvent.TextCommand(TextEditCommand)，不伪造按键；系统文件面板／原生编辑器保留自己的 responder／控件动作与可用性验证。UI 发布的 enabled 快照用于 Scoop 编辑目标，不覆盖系统面板的真实编辑状态。

ShortcutChord 复用 Primary 与既有逻辑键语义。只接受本后端能保持语义的单个逻辑字符或命名键；物理键、dead key、无法表示的 modifier／字符组合报 Unsupported。按实际原生解析结果检查同一菜单树中的重复绑定，冲突在修改前报参数错误。快捷键优先进入当前菜单栏，命中后不再向 Scoop 内容交付第二份 KeyDown／文本命令；AltGraph 与输入法已消费的文字不当作菜单快捷键。只有实际附着的菜单栏安装快捷键。

## 4. Popup session

Window.showPopup(menu, placement) 返回 PopupSession；PopupPlacement.Pointer(position) 使用内容逻辑坐标，Anchor(rect) 优先在已提交矩形下方展示，最终适配屏幕边缘由原生系统负责。非法坐标、归属和资源冲突在接受请求前报告；原生展示在当前业务回调／pump 返回后开始，不能在 handler 内阻塞原有派发。

PopupSession 提供 id、result 和 close()；result 为 None 或 Selected(MenuItemId)、Cancelled(PopupCancelReason)、Failed(WindowException)。取消原因区分 User、ExplicitClose、MenuClosed、TargetClosed、TargetBlocked、ApplicationExit。每次展示只记录一次终态；排队但尚未启动的展示也可以取消。已经完成的 session 再 close 无副作用。

ApplicationEvent.PopupCompleted(id, result) 是展示完成通知，Menu 动作才执行应用操作，两者不得重复执行动作。正常选择结束后，同一批次已排队的菜单动作仍可交付；显式关闭、菜单退休或应用退出会阻止未执行的旧动作。原生展示失败保留错误并完成 Failed；调用方可在完成事件处理错误。

Session 借用 Menu 与目标窗口，展示未安全退出前不能销毁原生菜单或窗口句柄。退出期间业务完成通知可以被丢弃，result 记录、取消和资源退休仍沿控制通路完成。AppKit／Win32 使用原生 tracking 循环；GTK 使用 GtkPopoverMenu 和其异步关闭信号。菜单期间 owner 和附着 I/O 继续推进，不创建第二套 managed loop。阻塞式 native operation 只从应用控制回调启动，不能从 watch 的外来回调内启动；必须让 watch 先返回并重新挂接其通知源。

## 5. 跨 Cone 菜单借用

scoop.window.native 的 Menu.acquireNativeMenu(requestClose) 返回 NativeMenuLease。lease 提供 NativeMenuUseId 和目标平台 NativeMenuHandle，保活 Menu 的原生树。AppKit 借出 NSMenu，Win32 借出 HMENU，GTK 借出 GMenuModel／GActionGroup；使用者不得销毁这些对象。原生数据属于当前 owner，句柄的访问与操作只能在该线程及 lease 有效期间发生。

独立使用者通过 beginTracking／endTracking 报告不能改动原生树的期间；beginTracking 清空本次选择，endTracking 返回本次原生使用选中的 MenuItemId?，包含标准角色，未开始或未选择时为 None。isTracking 查询实际原生使用，不能重入正在展示的菜单。通过 activate(item) 报告已映射的公共 MenuItemId；返回 false 表示条目或使用已退休／禁用。Win32 原生 command ID 使用 lease.itemForNativeCommand 查询对应的公共 ID，不截断转换。原生 NSMenu／GAction 已直接报告的动作不重复调用 activate。

Menu.close 或应用关闭立即停止原生动作（包含由原生端直接执行的标准角色），随后通知 requestClose；使用者负责结束自身 tracking、解除附着并 close lease。通知只发一次；退出控制通路不依赖普通业务队列。lease.close 不销毁 Menu；其原生使用仍活动时不能提前归还句柄。窗口库只保存 NativeMenuUseId，不认识 TrayIcon，也不引入托盘依赖。

使用者可以先调用 lease.cancel() 退休当前使用的动作并请求结束原生菜单，再等待自身展示退出、endTracking 和 close；cancel 不关闭 Menu，不允许在已取消的 lease 上开始新展示。GTK 借用模型的标准 action 属性使用 scoop-menu 命名空间；对应 GActionGroup 使用去除该命名空间的动作名，可按 GMenuModel／GActionGroup 公共接口只读适配其他原生菜单协议。
