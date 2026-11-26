# Scoop 自绘 UI 的 iOS 移植方案

调研基线为 2026-11-26 的 `main`（`97e9ee1f0`），沿用现有最低部署版本 iOS 17.0。首版目标是让同一份 Scoop 控件代码在 arm64 iPhone 和 arm64 iOS 模拟器上显示、响应触摸、编辑中文，并能由 VoiceOver 操作。首个宿主只开一个场景，窗口大小按实际视图计算，以便同一页在 iPad 分屏中使用。

建议保留 `scoop.ui` 的布局、绘制、事件路由和控件。`scoop.platform` 按文件选入移动端可用的系统服务，`scoop.window` 保留窗口和输入模型并实现 UIKit 宿主，`scoop.desktop` 继续作为桌面扩展。先跑通一个 `Label` 和一个能计数的 `Button`，再在同一页加入 `ScrollArea` 和 `Input`。各阶段都能用这个应用验证新增能力。

## 调研基线与已验证进展

| 路径 | 已验证 | 仍需验证 |
| --- | --- | --- |
| Scoop → UIKit 宿主 | `aarch64-apple-ios` 与 `aarch64-apple-ios-sim` 都能生成动态框架，合成 XCFramework 并链接 UIKit 宿主。两个 Mach-O 的平台分别是 `IOS`、`IOSSIMULATOR`，最低版本均为 17.0。模拟器运行已有 protected-data 用例通过。 | 在 iPhone 真机上签名安装并运行宿主，以及 Scoop 自绘窗口上屏。 |
| `scoop.platform` 最小集合 | 正式 Cone 已选入两目标的时间、线程、文件、目录和内存实现。模拟器的 `appInit` 对照 Foundation 核实容器和可执行文件路径，读取随包资源，并完成加锁、缓冲区往返和临时文件写读删除。 | 阶段 6 在签名安装的设备上复验同一应用。 |
| Skia 和图形绑定 | 在两个 SDK 下检查了 29 个现有原生编译单元。两个目标各有完整 Skia 与文字后端。独立应用在 iPhone 模拟器上将“中文🙂中文”排成 3 行，文字覆盖 908 个像素。已验证色块、SVG 图标、字形回退及 UTF-8 命中边界。 | 设备切片还需处理 `IOSurface.tbd` 的旧版本符号重定向，再复验独立应用链接。Metal 上屏在阶段 3 验收。 |

调研时的发布清单为两个 iOS 目标各提供 runtime、`scoop.core`、`scoop.std` 和 `scoop.os.ios`。后者已有查询受保护数据状态的 UIKit 调用。按生产依赖展开，`scoop.ui.component` 连自身共涉及 14 个 Cone，其中 `scoop.window`、`scoop.surface`、`scoop.graphics`、`scoop.graphics.text`、`scoop.graphics.svg`、`scoop.icons.lucide`、`scoop.ui`、`scoop.ui.style`、`scoop.ui.component` 共 9 个在调研基线上缺少 iOS 生产源选择。具体主题还要由应用选入，首屏建议为 `scoop.ui.style.shadcn` 增加 iOS 源文件条件。`scoop.desktop`、托盘及其控件扩展都在这条依赖链之外。[发布清单](../../scripts/release.toml) · [控件清单](../../sysroot/src/scoop.ui.component/Cone.toml) · [现有 iOS 宿主验收](../../tests/ios-native-app/README.md)

## 三个库的接口取舍

表中的“复用”指保留现有 Scoop 接口或数据模型。“iOS 定制”列出需要新实现的宿主行为。“仅桌面”指该桌面接口继续由桌面目标提供。标为“按需”的能力可以在有移动端调用方时单独实施。

### `scoop.platform`

| 处理 | 接口与范围 | iOS 处理及使用方 |
| --- | --- | --- |
| 复用 | `MonotonicTimeSource.now()`、`SystemClock`、`currentThreadId()`、`Mutex`、`ConditionVariable` | 选入对应的时间和线程后端，沿用标准库的 `scoop.time.TimeSource`。UI 用单调时钟计算动画和截止时间，用线程身份确认主线程归属。 |
| 复用 | `OsString`、`Path`、`File.open`、`OpenOptions`、`Path.readAllBytes()`、`ByteSource`、`ByteSink`、`NativeByteBuffer` | 选入 POSIX 字符串、路径、文件和缓冲区实现。字体文件和资源可沿用字节读取。有需要时 `Pipe.open()` 也可用于进程内 I/O。 |
| iOS 定制 | `homeDirectory()`、`temporaryDirectory()` 及 bundle 资源路径来源 | 前两者分别从 Foundation 的 `NSHomeDirectory()`、`NSTemporaryDirectory()` 取得。可执行路径复用已有的 Apple `executablePath()` 实现。随包字体、图片由宿主用 bundle 定位，再用 Scoop 文件 API 读取。临时文件在可写目录创建。`native-app` 入口的 `arguments()` 按现有契约返回空序列。 |
| iOS 定制 | 目标判定与 Apple 平台差异 | 清单中 `os = "darwin"` 表示 macOS，`os = "ios"` 覆盖设备和模拟器，`env = "simulator"` 可区分后两者。运行时则把两个 iOS 目标都报告为 `OperatingSystem.Darwin`。检查滚动物理和拖动编辑中按 Darwin 判断的分支，分别按触摸手势和实际修饰键处理。 |
| 仅桌面 | `Command`、`ChildProcess`、`Command.spawnPtyNative`、`suspendCurrentProcess()` 及 PTY/终端作业控制 | 从 iOS 平台集合排除进程启动、伪终端及终端作业控制的声明与原生源。普通文件描述符读写可独立选入。 |
| 按需 | `SystemRandom.fill`、`resolve`、`TcpStream`、`TcpListener`、`UdpSocket` | 随机数需接 iOS SDK 支持的熵源。网络 API 可独立移植。基础控件的首屏没有这些依赖。 |

调研基线的清单在 iOS 上选入整个 `src/`，却只在 macOS 和 Linux 选入 `platform/posix`，导致编译时缺少 `OsString.withPlatformString`、`Path.withPlatformPath` 和 `createPlatformTemporaryPath`。阶段 1 的正式清单已按目录选入 `src/{concurrent,env,fs,io,memory,native,system,time}`、`platform/posix` 下的 `files.scoop`、`temporary.scoop`、`system_version.scoop` 及对应 C 源。把 macOS 选择的 40 个原生 C 文件送入设备 SDK 做语法检查，有 3 个直接失败，分别是 `random.c` 的 `<sys/random.h>`、`process.c` 的 `posix_spawn_file_actions_addchdir_np` 和 `pty_posix.c` 的 `<libproc.h>`。这三个文件继续按桌面目标选择。[平台清单](../../sysroot/src/scoop.platform/Cone.toml) · [原生后端](../../sysroot/src/scoop.platform/native/) · [目标和产物规范](../specs/SCOOP-IMPL-SPEC.md) · [native-app 入口规范](../specs/SCOOP-SPEC.md)

### `scoop.window`

| 处理 | 接口与范围 | iOS 处理及使用方 |
| --- | --- | --- |
| 复用 | `WindowId`、`WindowMetrics`、`WindowActivity`、`ContactId`、`ContactEvent`、`KeyEvent`、`InteractionPreferences` | 保留窗口身份、逻辑尺寸/像素尺寸/比例、活动状态、接触与键盘事件。宿主填写真实值。安全区需要单独传给布局，单靠 `WindowMetrics` 的三个现有字段无法表达。 |
| 复用并接主线程 | `ApplicationProxy.post`、`Window.createDeadline`、`WindowDeadline`、`Window.acquireNativeWindow`、`NativeWindowLease` | 用系统主队列唤醒窗口任务，按截止时间回调。为 `NativeWindowHandle` 增加 iOS 表示，限定 `UIWindow`、`UIView` 的借用期。 |
| iOS 定制 | 应用启动、场景连接/断开、焦点、可见性、旋转和缩放 | UIKit 宿主拥有主循环，在进程内调用一次 `scoop_app_init`。每个 `UIWindowScene` 持有窗口、视图及其 Scoop 根。场景失活时取消进行中的接触，普通前后台切换保留根。场景断开时取消计时和帧请求，释放本场景的回调和租约。依据视图 bounds、safe area 和显示比例更新布局。 |
| iOS 定制 | `ContactKind.Touch` / `Pen`、硬件 `KeyEvent`、`ShortcutModifiers.primary` | 接入 `touchesBegan/Moved/Ended/Cancelled`，启用多点触控并在一次接触期间保持 ID 稳定。一次触摸只产生一条输入流。复用现有点击、长按和滚动识别。按实际设备上报 Pencil 和硬件键盘，iPad 键盘的主快捷键使用 Command。 |
| iOS 定制 | `TextInputConnection`、`TextInputBatch`、`TextInputState`、`Utf8Range`，以及 `Window.setImeAllowed`、`publishTextInputState`、`publishTextInputCaret`、`requestSoftwareKeyboard` | 由可成为第一响应者的视图实现 `UITextInput`，桥接选区、marked text、提交、删除和光标几何。把系统 `NSRange` 的 UTF-16 单位转换为 Scoop UTF-8 字节边界。同一编辑批次只派送一次。键盘遮挡通过 `UIKeyboardLayoutGuide` 反映到可视区。 |
| iOS 定制 | `Window.clipboard`、`Clipboard.readText/writeText`、`Window.createAccessibilityProvider`、`InteractionPreferences`、`Window.observedTheme` | 用户触发的复制粘贴接系统剪贴板，PRIMARY selection 保持不可用。将节点的稳定 ID、角色、名称、边界与动作映射到 UIKit 无障碍容器和元素。用 `textScaleFactor`、`reduceMotion` 传递辅助偏好，窗口主题随系统浅色或深色外观更新。 |
| 仅桌面 | `Application.run` 的阻塞入口、`ApplicationProxy.requestExit`、`LastWindowPolicy.Exit`，以及 `desktopCapabilities()`、`systemTaskbarBounds`、`systemMenuBarHeight`、`requestPosition`、`beginMove/beginResize`、窗口层级和最小化/最大化、桌面模态窗口、原生标题栏和桌面菜单栏 | 桌面保留进程退出、全局桌面坐标、窗口管理和系统菜单集成。控件树内的菜单、弹层及编辑命令继续复用。 |
| 按需 | `ActiveApplication.openFile/saveFile/selectDirectory`、系统跨应用拖放、`requestHapticFeedback` | 如有文件选择需求，接 `UIDocumentPickerViewController`，在接口中明确复制到容器还是在授权期访问外部 URL。跨应用拖放和触感另接系统实现。页内拖动可先走现有路由。 |

`Application.run` 启动阻塞循环，`runUiApplication` 在恢复事件中创建桌面窗口。iOS 宿主在 `appInit` 返回后，从场景连接回调把事件投给 `UiHost.onEvent`。两层宿主从该事件取得 `ActiveApplication` 及其 `ApplicationProxy`，此时再用 `UiHost.attach(window, create)` 挂载根节点。输入候选位置、命中测试和 VoiceOver 元素框都取最近一次已接受提交的几何。宿主回调沿用 `foreignCallback` 的函数、上下文和释放约定。[窗口源码](../../sysroot/src/scoop.window/src/) · [UI 宿主](../../sysroot/src/scoop.ui/src/host/host.scoop) · [UIKit 场景生命周期](https://developer.apple.com/documentation/uikit/managing-your-app-s-life-cycle) · [UITextInput](https://developer.apple.com/documentation/uikit/uitextinput) · [键盘布局](https://developer.apple.com/documentation/uikit/uikeyboardlayoutguide) · [UIKit 无障碍容器](https://developer.apple.com/documentation/uikit/uiaccessibilitycontainer)

### `scoop.desktop`

| 处理 | 接口与范围 | iOS 决策 |
| --- | --- | --- |
| 仅桌面 | `ActiveApplication.openDesktop`、`DesktopApplication.attachWindow/setIcon/setBadgeText/setDockMenu/setJumpListTasks/observeMenuBinding` | 保留桌面宿主、Dock、文本徽章、跳转列表和菜单绑定。iOS 应用图标由 Xcode 资源提供。 |
| 仅桌面 | `DesktopWindow.setIcon/setOverlayIcon/setProgress/setMiniwindowCaption/setThumbnailTooltip`，以及 `DesktopImage`、`DesktopProgress`、`JumpListTask` | 保留桌面窗口、任务栏和缩略图装饰。`scoop.ui.component.desktop` 和托盘扩展继续按桌面目标选入。 |
| 按需复用 | `NotificationId`、`NotificationContent`、`NotificationAction` 等通知数据类型 | 若移动应用需要本地通知，可复用相同字段和动作标识。移动端授权、投递、撤销与点击回调由 UIKit 宿主接 `UNUserNotificationCenter`。当前 `prepareNotifications` / `DesktopApplication.openNotifications` 的所有者关系保留在桌面。iOS 的数字徽章也应使用系统的数字语义。 |

首版 iOS 依赖图可以完全排除 `scoop.desktop`。仍要处理的是 **`scoop.window` 自己声明的桌面接口**，以及 `surface` 和 `ui` 初始化时对它们的调用。[桌面应用接口](../../sysroot/src/scoop.desktop/src/application.scoop) · [桌面窗口接口](../../sysroot/src/scoop.desktop/src/window.scoop) · [通知接口](../../sysroot/src/scoop.desktop/src/notification/) · [Apple 通知中心](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)

## 上屏前要改的依赖

1. **解除两个无条件的桌面调用。** 创建 `SurfaceWindow` 时，`WindowRegionStore` 通过 `SurfaceHost.frameCapabilities()` 查询 `active.desktopCapabilities().frame`，连带查询菜单、桌面定位、窗口管理和文件对话框。把这里收窄到真正使用的窗口装饰能力，或在注册装饰区域时再查询。每次 `UiHost.attach` 又会调用 `menuBars.attached(window)`。`UiMenuBars` 的登记和事件转发，以及 `ApplicationOwner` 和 `ApplicationScope` 引用的桌面菜单、模态与文件对话框类型，都要随桌面源一起选择。基础控件中的 `src/native/` 标题栏、原生菜单和控制器也要核对引用，保留 `src/menus/` 等树内控件。这样首屏无需伪造桌面能力。[SurfaceWindow 构造](../../sysroot/src/scoop.surface/src/shared/host/window.scoop) · [SurfaceHost 查询](../../sysroot/src/scoop.surface/src/shared/host/host.scoop) · [UI 根挂载](../../sysroot/src/scoop.ui/src/host/host.scoop) · [控件原生集成](../../sysroot/src/scoop.ui.component/src/native/)
2. **图形按设备和模拟器分别构建。** 为 Skia 配置 `target_os="ios"`、`target_cpu="arm64"`、`ios_min_target="17.0"`，模拟器另设 `ios_use_simulator=true`。当前固定版本的上游 GN 同时加 `-arch arm64` 和 `-arch arm64e`，与桌面 `-mcpu=generic` 组合时，探针在 arm64e 指针认证处编译失败。只选 `arm64` 后通过。分别构建含 Ganesh Metal、SkParagraph、HarfBuzz、CoreText、Apple Unicode 和 SVG 的完整 Skia，以及 `scoop.charset` 所需的 `libscoop_text_backend.a`。阶段 6 在本地沿用现有包结构，将文字后端放入相应目标的基础库包、Skia 放入图形库包。[系统库打包](../../scripts/package_toolchain.py) · [Skia 构建脚本](../../scripts/build_skia.py) · [文字后端脚本](../../scripts/build_text_backends.py) · [固定版本的 Skia 构建规则](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/gn/skia/BUILD.gn)
3. **让 UIKit 视图承载 Metal。** `scoop.surface` 的现有 Darwin 渲染器依赖 `NSView` 和 AppKit。新增由 `UIView` 持有 `CAMetalLayer` 的实现，沿用 `WindowRenderer.configure/setFrameDemand/acquire` 契约。现有 Ganesh Metal 入口 `MetalContext` 位于 `scoop.graphics.native.macos`，把仅依赖设备、队列和纹理的部分供两端共用。把点尺寸乘显示比例得到像素尺寸，用 `CAMetalDisplayLink` 按帧需求驱动。零尺寸或暂时取不到 drawable 时返回 `FrameAcquisition.Deferred`。只在 `FrameCompletion.submission` 为 `FrameSubmission.Accepted` 时公布几何。场景失活时停止请求新帧，应用进入后台的回调返回前确保已提交的 GPU 命令完成调度，回前台再请求一帧。[现有渲染器](../../sysroot/src/scoop.surface/native/darwin/renderer.m) · [提交结果](../../sysroot/src/scoop.surface/src/shared/frame/completion.scoop) · [CAMetalDisplayLink](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink) · [Metal 后台规则](https://developer.apple.com/documentation/metal/preparing-your-metal-app-to-run-in-the-background)
4. **首屏同时接通基础语义。** `Label`、`Button` 的语义注册会经 `scope.semantics` 创建窗口级 provider。首屏实现名称、按钮角色和激活动作，从第一个演示页即可验证一次辅助操作只触发一次按钮回调。滚动、编辑与辅助偏好在后续阶段扩展。[控件语义注册](../../sysroot/src/scoop.ui.component/src/semantic_platform.scoop) · [UI 语义后端](../../sysroot/src/scoop.ui/src/accessibility/platform.scoop)

## 实施顺序与阶段验收

各阶段先更新受影响的库规范，再改实现并同步使用示例。源文件和依赖选择沿用 Cone 的目标条件。UIKit 桥使用已有的 Objective-C `.m` 和 C ABI，Skia 的 Objective-C++ 由上游构建。公用逻辑用所属 Cone 的 release 测试。阶段 1 至 5 先在模拟器完成全部功能和验证，阶段 6 再把同一组用例放到真机执行。[平台规范](../specs/SCOOP-PLATFORM-SPEC.md) · [窗口规范](../specs/SCOOP-WINDOW-SPEC.md) · [Surface 规范](../specs/SCOOP-SURFACE-SPEC.md) · [native-app 运行时契约](../specs/SCOOP-RUNTIME-SPEC.md) · [编程材料更新约定](../scoop-programming/UPDATE-WORKFLOW.md)

| 阶段 | 交付 | 通过条件 |
| --- | --- | --- |
| 1 · iOS 平台基础 | 为两个目标准备本地可消费的 `scoop.platform` 子集，接通目录、时间、线程、文件与内存。 | 用正式 sysroot 为两个目标链接 `native-app`。在模拟器由 `appInit` 验证 `arguments()` 返回空数组、同线程 ID 稳定、单调时钟不回退、加锁执行、缓冲区字节往返、临时文件写读删除，并实际读到随包资源。 |
| 2 · 离屏图形与文字 | 为两个目标准备 Skia 和文字后端。选入 `math`、`charset`、`graphics`、`graphics.text`、`graphics.svg`、`icons.lucide` 的所需源。 | 用独立 Scoop 应用分别链接两个 SDK。在模拟器离屏绘制色块及一个 SVG 图标，读回已知区域像素。排版中文和 emoji，验证非空字形输出、换行及命中返回有效的文本边界。 |
| 3 · 单场景首屏 | 接通 UIKit 场景、`window`、`surface` 和 `UiHost`，移出桌面能力查询和菜单登记。选入 shadcn 主题，接入基本触摸、标签语义与按钮动作。 | 在 iPhone 模拟器打开同一页。点一次按钮计数恰加一。旋转后点新位置仍命中。进后台再返回保留计数和唯一根节点。后台停止提交，动画结束且无更新的页面停帧，重新请求能出帧。语义树可读到标签和按钮并可激活按钮。 |
| 4 · 滚动、输入与编辑 | 增加 `ScrollArea` 和 `Input`，接通多指触摸及取消、`UITextInput`、键盘避让和文本剪贴板。 | 在 iPhone 模拟器区分轻点和拖动，取消触摸后无误点。用中文拼音组合、选词并提交一次，再编辑含 emoji 的文本。核对选区、删除、光标与候选位置。键盘弹出后输入框可见，复制粘贴往返一致。在 iPad 模拟器核对硬件快捷键与浮动键盘避让。 |
| 5 · 无障碍与尺寸适配 | 完成滚动和编辑节点、辅助焦点、Dynamic Type、外观、安全区与可变视口。 | 在模拟器通过辅助功能检查器读取名称、角色及阅读顺序，执行按钮、滚动和编辑动作。放大系统字号后内容仍可操作，切换外观和减弱动态效果能传入控件。iPad 分屏及旋转后，绘制、触摸与无障碍焦点框对齐。 |
| 6 · 本地集成与桌面回归 | 在本地组装两个目标的 `lib-base-<target>`、`lib-gui-<target>`，动态框架按现有流程组成 XCFramework。 | 在独立应用中只消费本地安装的预编译库，Xcode 嵌入并签名。从同一工程在 arm64 模拟器及 iPhone 真机启动，完成首屏、编辑和后台恢复。在真机用中文键盘实测组合输入，开启 VoiceOver 验证朗读顺序、双击按钮、滚动和编辑。运行受影响桌面目标的既有窗口装饰、菜单和退出用例，确认拆分后仍可用。 |

复验阶段 1 时，按宿主运行说明执行 `verify.py --platform --include-device`，模拟器应输出 `scoop-ios-platform-ok`。另用独立工作目录执行 `--protected-data --include-device`，核对原有的 `scoop-ios-protected-data-ok`。两次命令中的设备步骤均为本地交叉编译与链接。真机上屏与完整交互在阶段 6 一并验收。本轮交付源码和本地验收结果，iOS UI 目标包留待后续工作完成后发布。系统文件选择、跨应用拖放、通知和 `scoop.window.asyncio` 留给实际移动端调用方，托盘继续随桌面扩展交付。[宿主运行说明](../../tests/ios-native-app/README.md) · [XCFramework 打包](../../scripts/package_ios_xcframework.py)
