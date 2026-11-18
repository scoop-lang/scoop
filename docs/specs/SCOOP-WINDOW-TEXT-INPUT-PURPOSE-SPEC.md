# 输入用途、软键盘与遮挡

本规范补充 Window 的文本连接和 UI 文本宿主，不改变输入事务、正文验证或前台激活规则。

## 1. 用途与请求

`TextInputPurpose` 为 Normal、Digits、Decimal、Email、Url、Telephone、Password、Search。`Window.setTextInputPurpose(connection, purpose): RequestStatus` 为当前文本连接设置平台输入提示；成功返回 Applied，后端没有公开映射时返回 Unsupported。连接必须属于该窗口且仍为活动代次，旧代次报告 InvalidState。提示不限制合法字符、不改正文或选区，不替代 TextEditPolicy、protectedText 或显示投影；保护用途不可因切换明文显示而被撤销。

`Window.requestSoftwareKeyboard(connection, visible=true): SoftwareKeyboardRequest` 请求该活动连接显示／隐藏平台软键盘。SoftwareKeyboardRequest 为 Requested、Unavailable、Unsupported：Requested 仅表示系统接受请求，不证明键盘已显示；失效／非活动连接或系统拒绝请求为 Unavailable，没有公开接口为 Unsupported。不会请求应用前台焦点、启动系统辅助功能程序或重复重试。参数中的窗口身份非法仍为普通参数错误。

## 2. 状态与平台映射

`SoftwareKeyboardState` 为 Unknown、Hidden、Shown(LogicalRect?)。Unknown 表示平台没有可用信息；Shown(None) 表示已知显示但不遮挡本窗口。矩形以接收窗口内容逻辑坐标表示实际交叠区域，随窗口位置、DPI 和系统通知更新。`Window.softwareKeyboard` 返回当前状态，变化通过 `WindowEvent.SoftwareKeyboardChanged(state)` 交付；相同状态不重复通知。状态不按显示请求推测，也不按设备名称或窗口高度猜测。

Windows 使用 TSF 的 InputScope 属性，应用已有 STA owner 中的 IInputPaneInterop／InputPane2 请求，以及 IFrameworkInputPaneHandler 的 Showing／Hiding 通知。公开接口不可用时明确返回对应状态。原生通知只更新 native 值并入 Window 队列，不在 COM 回调中重入应用布局或编辑。关闭前撤销窗口登记并断开 context，迟到通知不能访问已退休窗口。

GTK 使用 GtkIMContext 的 input-purpose／input-hints；Password 设置私密用途。activate_osk 接收当前真实输入的 GdkEvent，必要时按该公共接口允许的空事件请求；返回 false 为 Unavailable。GTK 4.18 没有通用的显式隐藏或遮挡查询接口，隐藏请求为 Unsupported，状态保持 Unknown，不通过 focus-out／focus-in 伪造隐藏。macOS 当前自绘 NSTextInputClient 没有对应的通用键盘用途、软键盘请求或遮挡接口，这些请求为 Unsupported、状态为 Unknown。

## 3. UI 文本宿主

TextEditOptions 增加 `inputPurpose=TextInputPurpose.Normal`、`softwareKeyboard=true`。输入用途在活动文本连接建立及配置改变时发布，不为未变化的正文或每一帧重复设置。protectedText=true 时使用 Password 原生提示，显示投影切换不撤销它；inputPurpose.Password 本身仅是原生提示，不擅自修改复制、语义或投影策略。

TextEditHost 在成功的 Touch／Pen 短按、多击定位或选区把手操作后按 softwareKeyboard 请求显示；尚未建立连接时保留一次请求，连接就绪后执行。滚动、数据拖出、只读、禁用、隐藏或卸载不补发请求。每次明确触摸操作至多请求一次，系统拒绝后不安装重试 timer。`TextEditHost.requestSoftwareKeyboard(visible=true)` 也提供同一结果供自定义控件显式使用。Input／Textarea 及其 builder／包装透传两个选项；OTP 缺省按 pattern 选择 Digits 或 Normal，显式用途覆盖该缺省。

`TextEditHost.requestFocus(origin=Programmatic, softwareKeyboard=false): Boolean` 将自定义文本框的外侧命中区接到同一逻辑编辑焦点；成功且显式要求软键盘时复用上述一次触摸请求与连接就绪路径，仍遵守宿主 softwareKeyboard 开关与只读策略。标准输入框外边距使用普通 Press 判定成功短按，保留正文选择，拖动或父级滚动取消时不请求键盘。

实际遮挡变化对活动文本宿主触发一次 caret 显露，沿现有 Accepted 几何、焦点显露路径和 ScrollState 执行；不移动文档选区、不抢回被用户滚动或新焦点取代的显露请求。只调整与 caret 交叠的真实遮挡，浮动键盘按可用空间选择最近的可见侧；没有能容纳 caret 的已接受可滚动区域时停止，不循环请求布局。Unknown 保留原有视口，隐藏或解除遮挡后不复原过时的滚动位置。

## 4. 验证

用途、代次、请求结果及纯几何计算使用普通库测试；原生 ABI 和通知生命周期使用对应 native 辅助。真实设备的键盘显示、布局类型与系统遮挡仍按平台矩阵验收，不能用直接注入通知宣称通过。验证不获取前台焦点，不复测与本次变更无关的已通过功能。
