# Scoop 原生文件对话框规范

状态：D6 实施合同。沿用[窗口规范](SCOOP-WINDOW-SPEC.md)和[归属与模态规范](SCOOP-WINDOW-MODAL-SPEC.md)的 owner 线程、输入范围、原生命令与退出协议。实现状态及平台验证见[实施记录](../desktop-ui/PROGRESS.md)。本能力属于 scoop.window，Linux 使用已有 GTK/GIO，不依赖 scoop.tray。

## 1. 入口与类型

ActiveApplication 提供以下立即返回 session 的入口。系统面板不创建公开 Window，不挂载 Surface，也不接受自定义窗框。

| 入口 | 选项 | 成功值 |
| --- | --- | --- |
| openFile(target, options) | OpenFileOptions | Path |
| openFiles(target, options) | OpenFileOptions | 非空 Array<Path> |
| saveFile(target, options) | SaveFileOptions | Path |
| selectDirectory(target, options) | DirectoryDialogOptions | Path |

target 必须明确选择 DialogTarget.Owner(window) 或 DialogTarget.Application。Owner 关联并阻塞指定窗口，Application 阻塞同应用的全部 Scoop 窗口；没有业务窗口的应用也可以使用 Application 目标。省略 owner 不产生含糊的 modeless 行为。

返回 FileDialogSession<T>，包含唯一 FileDialogSessionId、state、result 和幂等 cancel()。FileDialogState 为 Pending／Showing／Ending／Completed；result 是 None 或 FileDialogResult<T>.Accepted(value)／Cancelled(reason)／Failed(WindowException)。FileDialogCancelReason 为 UserCancelled、Cancelled、OwnerHidden、OwnerClosed、ApplicationExit。用户按 Escape 或取消按钮属于 UserCancelled，不是错误。

ApplicationEvent.FileDialogCompleted(id) 只通知身份；调用方从对应 session 取得类型化结果，不把 Path 降为 String／Any。原生结果和显式取消按首次已经确定的终态处理，迟到 cancel 不覆盖已经形成的 Accepted／Failed。每个身份最多完成一次。

## 2. 选项与路径

OpenFileOptions 包含 title: String?、initialDirectory: Path?、types: FileTypes（默认 Any）。SaveFileOptions 增加 suggestedName: OsString?；DirectoryDialogOptions 仅包含 title 与 initialDirectory。FileTypes.Any 不限制扩展名，Extensions(Array<String>) 提供非空的允许扩展名集合。扩展名不含前导点、路径分隔符、NUL 或通配符／筛选器分隔符；原生配置在受理前复制，不借用调用方的可变数组。

初始目录和建议文件名是提示，实际位置可受系统历史及目录状态影响。建议文件名必须是一个非空的普通路径分量，不是绝对路径、点或点点。标题不允许 NUL。无效参数抛出 IllegalArgumentException；当前平台无法表达所要求的原生类型筛选时明确报 Unsupported，不静默删除选项。原生筛选控件的形态、扩展名自动补全以系统行为为准。

路径复用 scoop.platform.fs.Path 和 scoop.platform.system.OsString。Windows 直接复制系统 UTF-16 code units，经 OsString.fromUtf16 转换；Darwin 从文件 URL 的 fileSystemRepresentation 复制本地字节；Linux 使用 GFile.get_path。无本地文件系统路径的资源报告 Unsupported，不把 URI 伪装成 Path。不得经有损 display 字符串转换，不额外 canonicalize。

saveFile 的 Accepted 仅交付系统最终选择的路径。原生保存面板负责覆盖确认；库不创建、截断、写入或预留目标文件，不承诺稍后的 I/O 成功。

DesktopCapabilities.fileDialogs 分别报告 openFile、openFiles、saveFile、selectDirectory、owner、application、title、initialDirectory、suggestedName、extensions。能力表示后端实现的操作；服务暂不可用仍是 Unavailable，真实 native 失败保留 code。WindowException 增加可选 nativeDomain，用于保留 GTK/GIO 等原生错误域。

## 3. 输入范围与生命周期

Owner 目标必须属于同一个运行中的 Application，且可见、未最小化、可交互。一个直接 owner 同时只接受一个窗口模态或文件对话框，冲突为 ResourceBusy；不同独立 owner 可以分别展示文件面板。Application 文件面板与其他活动文件面板互斥，可以覆盖已有窗口模态，其结束不会解除已有窗口模态的限制。

文件面板的限制与窗口模态取并集，范围从受理持续到原生使用退休。新建窗口立即受现有 Application 范围限制。实际输入资格、旧 popup／系统操作取消、原有禁用状态恢复沿用窗口模态合同。系统文件面板的编辑、选择和快捷键由自身 native responder／控件处理，不能转发给被阻塞的后台 Scoop 编辑器。

受理后排队的原生展示只能在 managed handler 与 session pump 返回后启动。Win32 Show、AppKit runModal 可运行嵌套原生循环；GTK 使用现有 GMainContext 的异步回调。watch、deadline、I/O 和取消控制继续推进，不新增 UI 线程，不重入 managed pump，不靠周期轮询推进 session。

session 保留所借 owner 的 native lease。cancel、owner 隐藏／最小化／关闭、外层模态隐藏 owner 或 Application 退出请求取消真实面板。取消不能排在等待整个 native Show 返回的普通业务队列之后。原生已结束之前不能销毁 owner；AppKit 内层文件循环退出时，还必须继续结束已经取消的外层窗口模态。

原生调用返回、sheet／GTK 完成回调退休、面板结束并隐藏后，复制自有路径，释放 native 对象和 lease，存储终态，更新输入限制，再发完成事件。只向仍然可见、存在、可交互且没有进入结束阶段的 owner 请求恢复焦点。退出仍完成内部结果和释放，但不再派发已停用的业务回调。

## 4. 后端

- AppKit：NSOpenPanel／NSSavePanel；Owner 使用 beginSheetModalForWindow，Application 使用真正的 runModal，不用 modeless begin 冒充。allowedContentTypes 使用 UniformTypeIdentifiers framework；结束后 orderOut，复制 URL 的文件系统表示。
- Win32：IFileOpenDialog／IFileSaveDialog，使用 UI owner 线程的 COM STA。CoInitializeEx 的 S_OK 与 S_FALSE 均配对 CoUninitialize；RPC_E_CHANGED_MODE 是失败。Show 使用实际 owner HWND，Application 目标复用已有隐藏顶层宿主 HWND。设置 FOS_FORCEFILESYSTEM，按入口选择多选／目录／覆盖确认；Close 之后不读取选择结果。复制 SIGDN_FILESYSPATH，释放 IShellItem、数组和系统字符串。
- Linux GNU：GTK 4.18 基线的 GtkFileDialog open／open_multiple／save／select_folder 与对应 finish，传实际 parent 和 GCancellable。复用 GTK 的本地或 portal 路径，不另建 portal 客户端。finish 后复制本地路径，保留 GTK／GIO 错误域和 code，释放回调持有的资源。musl 仍没有 GUI 后端。

## 5. 验证

普通 Cone 验证四种入口、类型化路径、选项快照、取消竞争、Owner／Application 范围、窗口模态内打开面板、新窗口受限，以及 owner／应用关闭等待原生使用退休。真实面板存活时验证 asyncio 定时器和 I/O 进度，无窗口应用单独覆盖。保存选择不创建文件；路径编码不经有损文本。

Linux 分别验证 X11 与 Wayland 的 GTK 派发；实际 portal 与本地选择器的测试条件据实记录。Windows 完整编译 GUI 用例，SSH 仅运行可执行的非 GUI 测试，交互桌面随后验证。自动取消、原生属性查询和前台实际选择／编辑不能混写成同一验收结果。
