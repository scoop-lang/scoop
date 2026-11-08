# Scoop 系统桌面通知规范

本规范落实 [GUI 扩展设计](../desktop-ui/GUI-EXTENSIONS.md) G7，属于普通 Cone `scoop.desktop` 的 `scoop.desktop.notification` 包；不依赖 UI、Surface 或 Skia。应用标识、owner 线程与桌面资源基础见 [Taskbar／Dock 规范](SCOOP-DESKTOP-SPEC.md)。

## 1. 启动与归属

应用在正常 main 入口、启动 Application 之前调用 `prepareNotifications(applicationId): NotificationLaunch`，并在 Resumed 后调用 `DesktopApplication.openNotifications(launch): NotificationCenter` 接管。标识必须与 ApplicationOptions.applicationId 一致。一个进程只有一个活动 NotificationLaunch，一个 DesktopApplication 只有一个活动 NotificationCenter。关闭资源后可以重新创建；要接收系统在启动早期交付的动作，首次准备必须位于应用启动之前。

NotificationLaunch 只准备真实原生入口：macOS 的 UserNotifications delegate、Windows 的 COM 激活类工厂，以及 Linux 的会话服务连接。它不创建业务窗口、不启动独立业务事件循环，不改动系统通知权限或安装注册。系统过早交付的激活数据先保存在原生队列，接管后交给 NotificationCenter 的第一个观察者；不恢复上一进程的对象或闭包。

macOS 裸 CLI、bundle identifier 不匹配或 Windows 缺少安装期通知注册时仍可创建资源并查询设置，结果为 NotConfigured。查询设置不发送通知，也不触发授权。迟于应用启动的 macOS 准备可以用于当前进程，但不能承诺启动响应；能力查询必须反映这一点。

NotificationCenter、启动资源和 operation／subscription 的使用与关闭都要求创建线程。关闭中心也关闭它接管的启动资源；关闭启动资源同样停止中心。DesktopApplication 关闭时先关闭通知中心。关闭只停止本地结果、观察和原生借用，不撤回 OS 已接受的通知，也不等待用户操作。

## 2. 公共值与异步结果

`NotificationId(value: String)` 与 `NotificationActionId(value: String)` 是不同值类型，作用域是当前应用；实际使用入口要求它们非空且不含 NUL。`NotificationAction(id, title)` 的标题非空。`NotificationContent(title, body = "", payload = "", sound = Default, actions = [])` 在构造时检查标题、正文和动作：使用纯文本，标题非空，全部文本不含 NUL，动作 ID 不重复。sound 为 Default 或 Silent。

平台自身的限制在请求入口返回 InvalidContent，不截断标识或动作。例如 Windows 的 toast tag 最多 16 个 UTF-16 code units。原生保留 ID 与 payload 的准确内容；动作使用内部原生前缀区分系统默认动作和自定义动作，公开 ID 不改变。

NotificationCenter 提供以下操作，异步完成回到应用现有队列；回调不会在发起方法返回前执行：

| 操作 | 成功结果 |
| --- | --- |
| `settings(callback)` | NotificationSettings |
| `requestPermission(callback)` | NotificationSettings |
| `post(id, content, callback)` | NotificationSubmission.Submitted |
| `withdraw(id, callback)` | NotificationWithdrawal.Requested 或 NotFound |
| `observe(callback)` | 订阅 NotificationEvent，返回 Closeable |

前四项返回 NotificationOperation，并以 `Result<T, NotificationError>` 完成一次。operation.close 取消本地回调，不声称取消已经提交给 OS 的操作。参数和关闭／线程错误同步抛出；服务、权限、部署与原生请求错误异步返回。通知错误区分 Unsupported、PermissionRequired、Disabled、NotConfigured、Unavailable、InvalidContent 和 NativeFailure，可附带原生错误码或信息。

Submitted 只表示系统 API 接收了请求，不表示用户看见、收到或已读。重复使用同一 ID 请求替换原通知；OS 可能再次提醒。withdraw 的 Requested 只表示撤回请求已提交；没有本进程映射且无法查询历史的平台返回 NotFound。

## 3. 设置与事件

NotificationSettings 包含 availability 和 capabilities。availability 为 Available、PermissionRequired、Disabled、NotConfigured 或 ServiceUnavailable。capabilities 分别说明 actions、replace、withdraw、activation、closedEvents 和 coldActivation。临时服务离线不是永久 Unsupported。

requestPermission 仅在 OS 提供独立授权 API 时请求权限；其他平台返回实际设置，不弹出自制权限对话框。权限变更后调用方重新查询 settings；不添加定时轮询。

NotificationEvent 包含：

- Activated(id, action: NotificationActionId?, payload)：None 表示默认点击；自定义动作带原 ID。
- Closed(id, reason)：只报告后端确实给出的 Expired、Dismissed、Withdrawn 或 Unknown，不合成定时到期事件。
- Failed(id, error)：OS 在初次提交返回后实际报告的失败。

启动时的激活保留到第一个观察者安装；此后按当前订阅广播。缓冲激活与后续事件按接收顺序进入同一应用队列，后来的事件不能越过已经缓冲的激活。取消订阅或关闭后已经排队的回调不再进入业务代码。回调异常沿应用现有错误路径处理，不从外部系统线程直接抛入 managed runtime。

## 4. 原生实现

### macOS

使用 UNUserNotificationCenter、UNNotificationRequest 和 category/action。必须使用真实 app bundle；prepare 在 AppKit finishLaunching 之前安装通知 delegate，保留原 delegate，并把不属于本库的事件转交它。关闭时仅当 delegate 仍由本库持有才恢复原值。

请求标识与 category 使用本库前缀，userInfo 保存公开 ID 与 payload。注册 category 时保留既有类别，串行合并当前请求的动作，避免并发请求覆盖其他通知类别。授权使用系统 alert／sound 选项。原生 completion 和 delegate 在收到数据后复制／保留普通原生值，经主 run loop source 唤醒 owner 后交付。默认点击与自定义动作可随系统启动响应交付；关闭事件仅在 UserNotifications 实际报告时产生。

### Windows

使用 Windows.UI.Notifications WinRT ABI、ToastGeneric XML、显式 AppUserModelID 及 INotificationActivationCallback。XML 文本和属性正确转义，动作参数编码独立的 ID、action 与 payload；默认点击和具名动作共用激活路径。tag 使用原始 NotificationId，group 固定为本库通知组；原生长度不满足时失败。

Windows Dismissed 表示当前 toast 展示结束，通知可能仍留在通知中心；不把该回调解释为用户已经读取或消息历史已被删除。撤回通过通知历史 API 执行，用户禁用新通知不妨碍清理已有通知。

Windows 已有正确安装注册、但从未提交通知的应用可能尚无通知设置记录，get_Setting 返回 ERROR_NOT_FOUND。这个具体结果按系统默认允许提交处理；不为查询设置发送试探通知。首次 Show 创建系统记录，后续查询返回实际设置；其他服务或原生错误不据此忽略。

prepare 读取安装注册中的 CustomActivator CLSID，在当前进程注册 COM 类工厂；正常 main 入口在 COM 冷启动时同样执行 prepare。COM 回调先复制参数到线程安全原生队列，用 message-only HWND 唤醒 owner；该窗口不参与业务窗口计数、绘制或任务栏。WinRT Failed／Dismissed 事件使用同一路径。关闭先停用队列、注销类工厂与事件，再在创建线程释放 COM／WinRT 资源；迟到回调不得使用已关闭的 managed callback。

prepare 在创建 COM／消息窗口之前复用 window 后端的 DPI 准备入口，确保进程第一次创建 HWND 时已经使用同一个 Per-Monitor V2 模式；不能在通知创建了 HWND 后才尝试改变进程 DPI 模式。该入口只是两个实际创建路径共享的原生初始化，不增加另一套窗口生命周期。

NSIS／MSI 安装器按 `bundle.windows.notifications = true` 写入开始菜单快捷方式的 AppUserModelID、ToastActivatorCLSID，以及 AppUserModelId／CLSID LocalServer32 注册。CLSID 按应用 identifier 稳定生成，只是 COM 对象标识。LocalServer32 指向安装后的应用，使用正确的路径引用；卸载移除本安装的条目。运行期发送通知不写注册表或创建快捷方式；缺少注册返回 NotConfigured。

### Linux GNU

使用 GIO／GDBus 的 org.freedesktop.Notifications。GetCapabilities、Notify、CloseNotification 和信号接入创建线程的 GLib main context；不启动替代通知服务。body-markup 服务收到经过标记转义的纯文本，desktop-entry hint 使用应用标识；Silent 使用系统 suppress-sound hint。

维护公开 ID 与服务 uint32 ID 的正常映射；串行提交 post／withdraw，保证同一公开 ID 的连续发送能引用第一次提交的服务 ID。信号携带的动作只路由本资源记录的通知。服务 owner 改变时清除旧映射；服务恢复后新操作正常进行，不自动重发旧通知。该路径不承诺进程退出后激活或查询上一进程的通知历史。

## 5. 验证与 UI

普通 Cone 测试验证值、异步结果与取消、关闭后迟到回调、部署未配置、服务缺失，以及通知不依赖业务窗口。实际桌面／安装验证覆盖发送与替换、纯文本与 Unicode 动作参数、撤回、真实通知点击，以及 macOS／Windows 冷启动；未获授权或未能实际观察的部分明确记录。

UI 包装在 ApplicationScope 或显式 scope 中持有中心，提供显式发送／撤回方法及订阅。重建描述不发送通知；scope 退休只取消本地 operation／观察，不撤回业务通知。综合示例使用与包元数据一致的 applicationId，显示设置和实际平台能力。
