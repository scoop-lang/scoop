# UI 窗框与系统交互组件规范

`scoop.ui.component` 在 [Window 窗框规范](SCOOP-WINDOW-FRAME-SPEC.md)与 [Surface 区域规范](SCOOP-SURFACE-FRAME-SPEC.md)之上提供以下包装。能力以当前 ActiveApplication.desktopCapabilities().frame 和实际 WindowFrameMetrics 为准，组件不检查 OS 名称。

## 1. 根窗框与布局

`windowChrome(content, configure = {}): View` 使用 WindowChromeProps／WindowChromeBuilder。默认 frame = Client、fallbackFrame = None、titleBar = None、titleBarHeight = 32、transparent／shadow = None、clip／windowShape／inputShape = None，builder 的 key 为 Position。frame 表示明确请求；仅在调用者显式提供 fallbackFrame 时，Unsupported 才尝试后备模式，后备也不支持则报告真实错误。transparent／shadow 的 None 不修改原属性，Some 写入明确值；卸载不恢复历史属性。

WindowChrome 取得 scope.claimWindowFrame，保证同一 UiWindow 只有一个根配置。原生配置只在输入改变时更新；frameMetrics 通过所属观察入口驱动布局，不逐帧查询原生窗口。它始终保留标题栏槽和正文槽的子项位置，切换有无标题栏不改变正文身份。

titleBar 是可选自绘内容槽。Client／System(Standard) 时在客户区顶部使用 titleBarHeight；System(Extended) 使用实际 contentLayout 和系统按钮矩形留出原生空间。正文从标题栏或系统保留区之后开始，标题栏避开实际系统按钮；已有 Insets／Frame 可继续组合正文样式。需要原生拖动时将标题栏内容包装在 windowDragRegion 中。

## 2. 视觉与原生区域

`WindowChromeShape` 为 Bounds、Rounded(radius)、Ellipse、Path(graphics.Path)。圆角半径有限且非负；Path 是调用者保有的不可变路径，组件借用它。Bounds／Rounded／Ellipse 随实际组件尺寸建立路径。clip 只裁剪本组件绘制；windowShape 发布原生窗口裁剪，inputShape 发布独立鼠标覆盖。三者独立，不把视觉 alpha 当作系统命中区域。

原生 Shape／Input 必须由平台能力支持，明确请求不支持能力时报错。None 释放该组件原有的区域并恢复默认，空 Path 仍是空区域。所有区域在同一 Accepted 帧发布，保留 clip、变换与比例，Discarded 不替换原区域；视觉 Path 在 scene 同步使用后释放临时几何。只支持透明和输入区域的平台可以使用 clip + transparent + inputShape 组合，不宣称支持原生 windowShape。

## 3. 拖动与缩放

`windowDragRegion(content, configure = {})` 的 builder 提供 enabled = true、doubleClickMaximize = true、windowMenu = true、key = Position。它使用普通 InputArea 和未被后代消费的默认动作：真实左键按下调用 Window.beginMove；双击按配置切换最大化；右键在 windowMenu 能力存在时调用 showWindowMenu。按钮、文本编辑等后代已消费的输入保留自身行为，禁用拖动不禁用正文交互。

`windowResizeRegion(edge, content, configure = {})` 提供 enabled = true 和 key = Position。支持 resizeHitRegions 时发布相应的 WindowHitRole.Resize，其他具有 beginResize 能力的平台从真实左键输入启动原生 resize。明确启用但两种能力均缺失时报 Unsupported。位置仅来自 Accepted 的包装区域，卸载／禁用立即撤销对应命中资格。

## 4. 窗口按钮

`windowControls(configure = {})` 的 WindowControlsProps／builder 提供 buttons = WindowButtons()、size = ButtonSize.IconSm、style = buttonStyle()、theme = None、key = Position（仅 builder）。false 的按钮不显示；顺序为最小化、最大化／还原、关闭。外观和键盘行为复用普通按钮与主题。

支持 systemButtonHitRegions 时为按钮发布真实系统角色，并将 FrameButtonChanged 的 hover／pressed 状态用于相同按钮呈现，保留 OS 的最大化悬停等能力；其他平台调用原生最小化／最大化请求。关闭调用 Window.requestClose，仍经过应用的退出确认，不直接销毁窗口。系统已经处理的非客户区按钮输入不再由普通按钮执行一次。窗口关闭、输入边界变化和卸载沿所属区域与 scope 清理。

Windows DWM 颜色／圆角和 macOS titlebar 专属项继续通过 Window 的平台扩展及其能力查询配置；组件的视觉边框使用已有 Frame／Style。两者都不被当作跨平台必然可用的字段。
