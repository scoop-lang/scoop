# Scoop 窗口归属与模态规范

状态：D5 实施合同。窗口基础、输入、native lease 和应用关闭沿用[窗口规范](SCOOP-WINDOW-SPEC.md)，窗框沿用[窗框规范](SCOOP-WINDOW-FRAME-SPEC.md)。本规范不改变语言、runtime ABI 或编译流水线。平台运行验证以[实施记录](../desktop-ui/PROGRESS.md)为准。

## 1. 窗口关系、角色与叠放

WindowAttributes 增加 relation 和 role，默认分别为 WindowRelation.Independent 与 WindowRole.Normal。WindowRelation.Owned(window) 创建具有独立客户区、DPI、Surface 和 WindowId 的顶层窗口；ownerWindow 与 role 创建后只读。owner 必须属于同一个活动 Application，且未进入 Closing。关系只在创建时建立，因此不会形成环；不提供 reparent、嵌入式 child window 或公共窗口分组。

WindowRole.Tool 请求原生工具面板角色，不隐含全局置顶或模态。AppKit 使用 NSPanel，Win32 使用 WS_EX_TOOLWINDOW；X11 设置 UTILITY 类型提示，Wayland 没有对应通用协议时创建请求返回 Unsupported。调用方可按能力明确改用 Normal。

Owned 使用 AppKit child window、Win32 owner HWND 或 GTK transient-for。DesktopCapabilities.windows.ownedOrder 以 OwnedWindowOrder.Native／WindowManagerHint 描述原生维持和 WM 提示的区别；Linux 不承诺任意 compositor 都维持相对顺序。移动跟随保留系统差异。

AppKit 的 orderOut 会撤销原生 child 关系；在可见 parent 上重新 addChildWindow 又会显示 child。因此隐藏时保存逻辑 owner，下一次显示时恢复原生关系，不能为维持隐藏窗口的 NSWindow.parentWindow 属性而意外显示它。sheet 展示期间使用 sheetParent，结束并隐藏后同样保留逻辑关系。

requestRaise 对可见、未最小化的窗口请求一次提高顺序，不请求键盘焦点，成功受理返回 Requested；隐藏或最小化状态为 InvalidState。AppKit orderFront、Win32 SetWindowPos(NOACTIVATE) 和 X11 raise 可用；Wayland 返回 Unsupported。requestFocus 仍是独立操作，沿用原生显示并申请激活的行为，同时更新可见意图；被模态阻塞、因 owner 隐藏而受抑制、或模态展示尚未启动的窗口不能借此申请焦点或提前显示。

setWindowLevel 保留 Normal／AlwaysOnTop／AlwaysOnBottom。Owned 不接受 AlwaysOnBottom；有 owned 后代的窗口也不接受破坏原生相对顺序的置底请求。Win32 owner 的 topmost 会传播到 owned 窗口，能力中的 ownerTopmostPropagation 如实报告；observedWindowLevel 查询实际层级，未知返回 None，不能把请求值当作观察值。Owned Normal 表示不额外要求 topmost；若 owner 当前是 topmost，Win32 不通过对 child 设置 NOTOPMOST 意外降低 owner。

窗口保存调用方的可见意图。owner 隐藏或最小化时，owned 后代临时隐藏；恢复时只恢复仍希望显示的窗口。显式隐藏／最小化 owner 前先取消依赖其展示的模态。应用整体失去前台、切换桌面、暂时遮挡不构成取消。原生按钮触发的最小化也按实际观察更新后代。

## 2. 模态范围与展示

对已经创建、尚未显示的 Window 调用 showModal(ModalOptions)，返回 ModalSession。ModalOptions 包含 scope: ModalScope（Owner／Application，默认 Owner）与 presentation: ModalPresentation（Automatic／SeparateWindow／Sheet，默认 Automatic）。Window 内容和 Surface 可以在调用前挂载；模态不会替换客户区或改变其绘制所有权。

Owner 只阻塞直接 owner，不阻塞其独立主窗口或兄弟工具窗口。要求有效、可见、未最小化、未被阻塞的 owner。Application 阻塞同一 Application 的其他窗口；活动模态窗口及从它开始的嵌套模态分支可以交互。新建窗口立即遵守现有范围。一个直接 owner 同时只接受一个活动模态，冲突返回 ResourceBusy；Application 模态只能从当前活动模态分支嵌套，不接受互相阻塞的平级请求。

DesktopCapabilities.windows.modal 分别报告 ownerSeparateWindow、ownerSheet、applicationSeparateWindow、nestedSheet，supports(scope, presentation) 只表达平台组合，不能代替状态检查。Automatic 在 AppKit Owner 范围选择 Sheet，其余选择 SeparateWindow，不改变 scope。显式 Application + Sheet 不支持。AppKit 使用普通 beginSheet/endSheet；嵌套调用以当前 sheet 为直接 owner，不使用 critical sheet 绕过排队。独立 Owner 模态保持 child 关系并由原生窗口输入资格限制 owner，不以 runModal 扩大作用范围。

Win32 模态采用实际 EnableWindow 与 owned 关系；GTK 采用 transient-for、每窗口原生组以及发布到原生 widget 的输入资格，组只隔离 GTK 自身的 modal grab，实际阻塞范围由相同的 session 集合决定。AppKit Application 模态使用真正的 runModalForWindow。原生同步输入回调只读取已发布状态，不调用 managed 布局、渲染或业务代码。

Window.isInputBlocked 和 WindowEvent.InputBlockedChanged 表达窗口层施加的实际模态限制。进入阻塞时撤销按下／捕获和 IME 组合，PointerCancelReason.Modal 区分这次系统接管；恢复不伪造按下或字符。原生客户区、焦点资格、快捷键、菜单及编辑命令都遵循限制。排队前确定的菜单目标不会在派发时换成其他焦点窗口；已失去资格的目标不会执行旧命令。Quit 保持应用退出请求语义。绘制、I/O、deadline、Closing 和 Destroyed 正常推进。

不同 session 的限制取并集；结束内层只释放它自己的限制。Win32／GTK 保存首次阻塞前的 enabled／sensitive 状态，最后一个限制解除时恢复该状态，不无条件启用原本禁用的窗口。

已在原生边界形成的文本事务与取消仍按原顺序交付，不能丢弃其中一批造成文本序号、文档投影与应用状态失步。新的文本输入由原生焦点／输入上下文与控件禁用限制；快捷键和菜单动作在执行前再次核对其窗口资格。

进入模态阻塞时，以 PopupCancelReason.TargetBlocked 结束目标窗口已有或待显示的 popup；不解绑其菜单栏。阻塞后禁止新 popup／系统拖移请求，也取消已经准备但尚未执行的系统操作，不能在模态快速结束后重放旧输入。

## 3. Session 生命周期

ModalSession 保存唯一 ModalSessionId、借用的 window、options、实际 presentation、state 与 result。state 为 Pending／Showing／Ending／Completed；result 为 None 或一次 ModalResult.Completed／Cancelled(reason)／Failed(WindowException)。ModalCancelReason 包含 Cancelled、WindowHidden、WindowClosed、OwnerHidden、OwnerClosed、ApplicationExit。结果对应原生展示生命周期，业务确认值由上层另行保存。

dismiss() 请求正常完成，cancel() 请求显式取消；两者幂等，第一项结束原因获胜。结束展示隐藏借用窗口，不销毁它。显式隐藏模态窗口、关闭窗口／owner 或退出都结束相关 session。用户点击关闭按钮仍发 CloseRequested，由应用决定 dismiss、requestDestroy 或继续展示。

创建函数只准备并排队原生命令，先返回 session。原生命令在 managed 派发退出且 I/O watch 重新挂接后执行；已有原生模态循环中可以运行新一层命令，但 managed pump 不重入。取消和关闭直接进入控制路径，不等待外层阻塞调用返回。无事件时依赖现有主循环休眠，不增加周期轮询。

session 持有展示窗口及 owner 的 native 使用，直到阻塞调用返回、sheet 完成回调退休、原生展示解除且隐藏完成。只有随后才存储终态并排队 ApplicationEvent.ModalCompleted(id, result)。退出时仍完成内部结果和释放，但不重新投递已停用的业务回调。成功解除限制后只向仍存在、可见、未最小化且可交互的 owner 或上一层请求恢复焦点，遵循系统激活策略。

## 4. 关闭依赖

requestDestroy 将本窗口与所有 owned 后代纳入 Closing，禁止新窗口关系、展示与 lease。向每个窗口递送 Closing，让 Surface 等拥有者释放资源；取消展示和 popup，继续推进主循环与 asyncio 收尾。后代原生对象必须先于 owner 销毁：只有本窗口已递送 Closing、自己的 lease 为零、owned 后代全部 Destroyed 才能销毁它。不得依赖 Win32 自动销毁 owned HWND 或 GTK destroy-with-parent 绕过这个顺序。

requestExit 对全部窗口执行同一协议，最后等待原生操作、应用资源与 attachment 退休。退出确认必须发生于 CloseRequested／ExitRequested，Exiting 后拒绝重新打开模态。

## 5. 验证范围

普通 Cone 测试覆盖属性、真实原生关系、两主窗口与工具窗、Owner／Application 及嵌套范围、新建窗口、原有禁用状态恢复、菜单目标、取消竞争、父子关闭及未释放 lease。AppKit 验证 sheet／嵌套 sheet 与实际 application-modal 循环，Linux 分别验证 X11／Wayland 分组和输入资格；Windows GUI 程序完整编译并留给交互桌面运行。现有 asyncio 原生循环组合继续验证 I/O 和取消能够推进。测试不把合成输入与桌面视觉结果混为同一验收结论。
