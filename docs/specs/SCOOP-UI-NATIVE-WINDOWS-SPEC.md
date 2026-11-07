# UI 原生窗口与文件对话框组件规范

本文规定 `scoop.ui.component` 的显式控制器，原生归属、输入范围和完成条件沿用 [Window 模态规范](SCOOP-WINDOW-MODAL-SPEC.md)及[文件对话框规范](SCOOP-WINDOW-DIALOGS-SPEC.md)。普通同窗口 Dialog／Sheet 仍使用 Portal。

## 1. Scope 与共同规则

控制器由创建它的 ComponentScope 或 ApplicationScope 拥有；创建入口将其登记为 scope 资源。打开动作进入所属 dispatch，返回 Boolean：成功接受一次请求为 true，已有活动请求或 scope 已退休为 false。原生拒绝和创建错误保持所属错误边界，不能把 Unsupported 当作已成功。所有公开操作在应用 owner 线程执行，render 不调用 open。isPending／isOpen 以及 window 的变化可被 render 观察。

窗口控制器只销毁自己创建的 Window／UiWindow，UiHost.attach 本身继续借用 Window。创建失败回收已取得资源；关闭先结束 session，再关闭 UiWindow 并 requestDestroy，实际原生销毁等待低层使用退休。显式关闭控制器或 scope 卸载停止业务回调；取消一次展示保留控制器，以便完成后再次打开。每次请求的回调只执行一次；组件触发器通过 Props 取得最新回调。

## 2. 工具窗口

`ComponentScope.nativeToolWindowController(): NativeToolWindowController` 固定以该 scope 的 nativeWindow 为 owner。`open(create: (ComponentScope) -> C, onClosed: () -> Unit = {}, attributes = windowAttributes(), appearance = RequireTool, fonts = None): Boolean` 创建隐藏 Owned 窗口，挂载成功后显示。attributes 的 relation、visible 和 role 由控制器决定，其余配置沿用；requestActivation 决定展示后的明确焦点请求。`NativeToolAppearance` 为 RequireTool、PreferTool、Normal；PreferTool 在 toolWindow 能力缺失时明确允许普通 Owned 外观，Normal 总使用普通外观。相对叠放能力仍按 WindowManagementCapabilities 报告，不升级为全局 AlwaysOnTop。

`window: Window?` 借用当前窗口，`isOpen` 表示未请求关闭的活动窗口；`closeWindow()` 关闭本次窗口，`close()` 永久退休控制器。等待 Destroyed 前再次 open 返回 false。窗口原生 CloseRequested 调用 closeWindow；父窗口关闭沿 Window 的后代规则收尾。onClosed 在真实 Destroyed 后交付；隐藏、显示、raise 和窗口属性可通过明确借用的 window 操作。

## 3. 类型化原生模态窗口

窗口／应用 scope 均提供 `nativeDialogController<T>(target: DialogTarget): NativeDialogController<T>`。target 固定为明确 Owner 或 Application，不因 owner 关闭自动替换。`open(create: (NativeDialogResponse<T>, ComponentScope) -> C, onResult: (NativeDialogResult<T>) -> Unit, attributes = windowAttributes(), options: ModalOptions? = None, fonts = None): Boolean` 创建隐藏窗口、挂载 UI 后开始真实 ModalSession。默认 options 根据 target 选 Owner 或 Application 范围及 Automatic 展示；明确 options 按 Window 校验。attributes 的 relation、visible 由控制器决定。

`NativeDialogResponse<T>` 属于一次展示，提供借用 window、`accept(value): Boolean` 和 cancel()；旧 response 不影响以后新展示。首次 accept／cancel 决定结束请求，重复请求不改变已提交值。accept 在原生完成前只记录值并 dismiss；只有实际 ModalResult.Completed 才返回 Accepted(value)。取消和失败保留实际 ModalCancelReason／WindowException。NativeDialogResult<T> 为 Accepted(T)、Cancelled(ModalCancelReason)、Failed(WindowException)。

控制器提供可观察 isPending、借用 window 和 cancel()。等待 ModalCompleted 时仍为 pending，不能复用其活动窗口；完成时先释放拥有的 UI／窗口、清除 pending，再执行 onResult，允许回调打开下一次。原生 CloseRequested 取消本次展示。由 UI 挂载失败造成的 open 失败按 UiHost 已有规则报告，并清理隐藏窗口，不伪造 Accepted 或原生 Failed。

## 4. 文件面板控制器

窗口／应用 scope 均提供 `fileDialogController(): FileDialogController`。控制器提供 openFile(target, onResult, options = OpenFileOptions())、openFiles(target, onResult, options = OpenFileOptions())、saveFile(target, onResult, options = SaveFileOptions()) 和 selectDirectory(target, onResult, options = DirectoryDialogOptions())，均返回 Boolean。每次请求都显式给出 DialogTarget，单选／保存／目录的回调为 FileDialogResult<Path>，多选为 FileDialogResult<Array<Path>>，不做字符串路径转换。

isPending 可被 render 观察；cancel() 请求取消当前面板，等待真实完成期间仍为 pending。完成先清除当前 session 和登记，再执行本次 onResult；取消或失败不自动重开。close 永久退休控制器并取消当前请求，卸载后不执行迟到回调。保存只返回路径，不创建或写入文件。

## 5. 文件按钮

`nativeOpenFileButton(text, target, onResult, configure = {})` 和 `nativeSaveFileButton(text, target, onResult, configure = {})` 使用明确的 DialogTarget 与类型化路径回调。builder 继承普通 ButtonBuilder 的内容外观、字体、焦点、enabled 和 key，分别增加 OpenFileOptions／SaveFileOptions。内部的 NativeFileButton 复用 Button 和 scope.fileDialogController；pending 时禁用重复触发，完成后恢复。只在点击或键盘激活时打开，render、取消和外观更新都不自动重开。

NativeFileButtonProps 接受 ButtonContent、NativeFileRequest（Open 或 Save，分别携带对应 options）、target、onResult，以及普通按钮的 enabled、style、theme、focus、invalid、variant、size、group、labelTarget。每次点击读取最新 Props；完成回调也转发到当前 Props。更新 target／options 仅影响下一次请求，当前 session 始终保留创建时的原生目标。组件卸载关闭控制器，并停止迟到回调。
