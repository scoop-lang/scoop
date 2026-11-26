# iOS native-app 宿主示例

使用装有 iOS Simulator SDK 和 runtime 的 macOS Xcode、LLVM／LLD 22.1 和 Python 3.11 以上版本。以下命令从仓库根目录执行。

```sh
cargo build --release -p scoop -p scoopc -p scoop-linker -p scoop-pm --bins
python3 scripts/prepare_sysroot.py --target aarch64-apple-ios-sim \
  --scoop target/release/scoop
```

构建自己的 `native-app` 时，二进制名和 install name 应与 framework 名一致。应用的 C 头声明自己的函数表布局，组装脚本把它与 `scoop_app.h` 一起公开。

```sh
target/release/scoop build /path/to/Cone.toml --release \
  --target aarch64-apple-ios-sim --sysroot sysroot \
  --linker /path/to/ld64.lld \
  --install-name @rpath/ScoopApp.framework/ScoopApp -o target/ios/ScoopApp
python3 scripts/package_ios_xcframework.py --name ScoopApp \
  --bundle-id dev.example.scoop-app --simulator target/ios/ScoopApp \
  --header /path/to/AppApi.h --output target/ios/ScoopApp.xcframework
```

需要设备切片时，用 `prepare_sysroot.py --target aarch64-apple-ios` 准备同目标的资源，再以该 target 和同一个 install name 把设备二进制构建到另一目录。将 `--device /path/to/device/ScoopApp` 和 `--simulator target/ios/ScoopApp` 一起传给组装脚本。

在 Xcode 的宿主 target 添加 XCFramework 并选择 **Embed & Sign**。Swift 用 `import ScoopApp`，Objective-C 用 `@import ScoopApp;`。宿主第一次调用 `scoop_app_init` 成功后保存返回的函数表，后续把配对的 context 传给每个回调；应用可通过函数表关闭业务资源。

## 真正启动模拟器宿主

先启动一个 arm64 模拟器，用 `xcrun simctl list devices booted` 获取 UUID。

```sh
python3 tests/ios-native-app/verify.py --simulator SIMULATOR_UUID \
  --work-dir target/ios-native-app-verification --linker /path/to/ld64.lld
```

脚本以同一个 XCFramework 构建、签名和启动 Swift 与 Objective-C 宿主。Swift 另启动一个进程验证初始化异常返回。两个宿主初始化时都经 SDK 的 `m` 库调用 `cos`，覆盖系统库别名的显式符号绑定。两种宿主都在主线程和工作线程调用 Scoop，Scoop 回调宿主服务并检查实际搬迁计数及回写后的对象引用。输出应包含 `scoop-swift-moving-gc-ok`、`scoop-objc-moving-gc-ok` 和 `scoop-swift-init-failure-ok`。

加上 `--include-device` 可同时组装设备切片，并交叉编译和链接两份真机宿主 App。用 Xcode 的 iOS 开发签名和包含该设备的 provisioning profile 安装，首次启动时按设备提示信任该开发签名。以 `SCOOP_GC_STRESS_MOVE=1` 启动，检查同样的成功标记。

```sh
xcrun devicectl device process launch --device "$DEVICE_ID" --console \
  --terminate-existing --environment-variables '{"SCOOP_GC_STRESS_MOVE":"1"}' \
  dev.scoop.ios-probe.swift
```

使用 `dev.scoop.ios-probe.objc` 验证 Objective-C 宿主。在 Swift 启动命令末尾加 `-- --fail` 验证初始化异常返回。

## iOS 平台基础库

用本地开发 sysroot 构建平台库，模拟器命令如下。设备切片使用 `aarch64-apple-ios`。这两份本地库供开发宿主消费。

```sh
target/release/scoop build sysroot/src/scoop.platform --release \
  --target aarch64-apple-ios-sim --sysroot sysroot \
  -o sysroot/lib/aarch64-apple-ios-sim/scoop.platform.slib
```

在模拟器验证命令中加 `--platform` 并使用独立工作目录。宿主把 Foundation 的目录及随包资源路径传给 Scoop，对照应用主目录、临时目录和可执行文件，并从 bundle 读出文本、在容器中创建和删除文件。该用例同时通过主线程调用时钟、互斥锁和缓冲区，成功标记为 `scoop-ios-platform-ok`。

## iOS 离屏图形和文本

按[原生依赖准备说明](../../scripts/README.md)构建对应目标的 Skia、Rust 文字后端和上一节的 `scoop.platform`。用同一个目标依次预编译 `scoop.math`、`scoop.charset`、`scoop.graphics`、`scoop.graphics.text`、`scoop.graphics.svg`、`scoop.icons.lucide` 到本地 sysroot。再用独立工作目录运行上述模拟器命令，加上 `--graphics`。

这个宿主通过 Scoop 绘制色块、Lucide 加号和换行的中文与 emoji，验证颜色读回、系统字体、UTF-8 命中边界和 GBK 往返。控制台输出实际行数、覆盖像素数及 `scoop-ios-graphics-ok`。

## UIKit 场景与窗口

在同一 sysroot 为两个目标预编译 `scoop.window`。用独立工作目录运行上述命令并加 `--window --include-device`。iPhone 模拟器会创建真实的 `UIWindowScene`，对照 UIKit 核对窗口、视图、安全区、显示比例和像素尺寸。工作线程投递、重绘合并、一次性定时和断开后的原生借用回收通过后，输出 `scoop-ios-window-ok`。设备宿主在本步骤交叉链接。随后模拟器以 `--ime-transactions` 再次启动宿主。这个用例在真实的 UIKit 第一响应者上组合、换选、提交和删除，核对含 emoji 的 UTF-16 系统选区与 Scoop UTF-8 选区，验证旧光标和关闭的文本连接会被拒绝。成功标记为 `scoop-ios-ime-ok`。

安装上述 `--window` 宿主后，可通过 XCTest 在同一模拟器送入轻点、拖动和双指点击。最后在仍按住时切走 key window，检查接触取消且抬手不会重复结束。测试通过 Scoop 收到的事件检查每个触点的身份和逻辑／像素坐标。

在 iPad 模拟器安装同一宿主，下面命令的 destination 改为该 iPad，将 `testWindowTouches` 换成 `testHardwareKeys`。用例经 UIKit 实际派送 Command-K 和向下方向键，核对逻辑和物理快捷键匹配、修饰键、显示标签以及按下与释放的配对。

文本用例还会通过 UIKit 的 responder chain 执行复制与全选，并执行系统 undo manager 的撤销与重做。分别在编辑器开启和关闭键盘后验证命令可用性、交付次数及原文。

```sh
xcodebuild test -project tests/ios-native-app/interaction/Interaction.xcodeproj \
  -scheme Interaction -destination "platform=iOS Simulator,id=$SIMULATOR_UUID" \
  -parallel-testing-enabled NO -only-testing:InteractionTests/InteractionTests/testWindowTouches
```

在测试用模拟器上显式运行剪贴板互通用例。它会替换该模拟器的通用剪贴板。Scoop 写入空文本以及带 HTML 表示的中文和 emoji，UIKit 核对原生条目，再写回另一段文本供 Scoop 读取。成功标记为 `scoop-ios-clipboard-ok`。

```sh
xcrun simctl launch --console --terminate-running-process "$SIMULATOR_UUID" dev.scoop.ios-probe.window --clipboard
```

选择 `testTextEditMenu` 会打开 UIKit 的编辑菜单，由 XCUITest 点选应用翻译的复制项，核对动作只交给打开菜单的目标。

在同一 iPhone 模拟器选择 `testSceneDrag` 可用系统触摸把一行数据拖到接受区，再拖向拒绝区。Scoop 核对原始触点只取消一次、本地对象和文本保持一致、成功的 Move 只投递一次。

在窗口宿主加 `--accessibility` 运行，可用 UIKit 检查已发布的名称、角色、裁剪后边界、按钮激活和祖先滚动。节点退休后再次调用其旧系统元素会被拒绝。成功标记为 `scoop-ios-accessibility-ok`。同一 XCUITest 命令选择 `testWindowAccessibility`，由另一个进程按按钮角色查询实际系统元素。

## 公共 Surface 首帧和恢复

准备好 `scoop.graphics`、`scoop.window` 和 `scoop.surface` 后，给上面的 `verify.py` 命令添加 `--surface --include-device`，并指定新的工作目录。模拟器应用在场景连接时挂载 `SurfaceContent`，检查首个接受帧、空闲停帧、显式重绘及关闭次序，成功输出 `scoop-ios-surface-ok`。在同一模拟器执行上述 XCUITest 命令，把测试方法改为 `testSurfacePresentation`，用真实截图核对初次绘制为绿色，按 Home 并重新激活后同一内容根绘制蓝色。

## 公共 UI 控件

在模拟器验证命令中添加 `--ui`，使用新的工作目录。该模式构建并安装普通 `UiHost` 应用，自动运行 `testUiControls`。测试用系统无障碍标识找到实际的 Label、Button 和 Input，检查竖屏与横屏点击、第一响应者输入、ScrollArea 滚至末行以及 Home 后恢复。所需库为 `scoop.ui.component`、`scoop.ui.style.shadcn` 及其依赖。`--include-device` 同时交叉链接设备宿主。

## UIKit 受保护数据

在上述模拟器验证命令加上 `--protected-data` 并使用另一个工作目录，会构建普通 release 应用。UIKit 启动后，宿主通过 Scoop 回调查询数据保护状态，与同一时刻的 UIKit 返回值对照。成功标记为 `scoop-ios-protected-data-ok`。

同时加上 `--include-device` 会生成 `device/protected.app`。在启用密码和数据保护的设备上开发签名并安装该宿主，以 `dev.scoop.ios-probe.protected` 启动并观察控制台。锁屏、等待数据保护生效、解锁，记录 `will-lock` 与 `did-unlock` 通知中的两次查询。前者发生在状态即将改变时。宿主会在后台短暂继续采样，`background scoop=0 UIKit=0` 表示两次查询都观测到数据已受保护。

在验证命令中选用 `--asyncio`，可在模拟器的主运行循环验证一次计时恢复、长定时子任务随会话关闭而取消，以及主线程收到最终关闭结果。成功标记为 `scoop-ios-asyncio-ok`。

## UI Gallery

先按 [Gallery 构建说明](../../sysroot/src/scoop.ui.component/examples/shadcn/README.md)把 ScoopGallery 安装到 iPhone 和 iPad 模拟器。两个设备都使用 `dev.scoop.gallery`。用上文的 `xcodebuild test` 命令指定对应 destination，并用下表的方法名替换 `-only-testing:InteractionTests/GalleryTests/方法名` 的最后一段。

| 设备 | 方法名 | 观察结果 |
| --- | --- | --- |
| iPhone 竖屏 | `testGalleryCatalog`、`testGalleryNavigation` | 逐项打开、返回全部 69 个页面；搜索、Home 恢复、主题、RTL 和减少动态可操作。 |
| iPhone，软件键盘选简体拼音 | `testGalleryChineseSettings` | 逐键输入 nihao 并点选系统候选“你好”，填写邮箱和六位验证码后，Settings 显示保存成功。 |
| iPad 宽窗口 | `testGalleryWide`、`testGalleryForms`、`testGalleryPaymentUndo` | 双栏同时显示目录和详情。保存表单、切页保留 Workspace 草稿、横向滚动付款表并删除和撤销一行。 |
| iPad 宽窗口 | `testGalleryTimers`、`testGalleryTreeDrag`、`testGalleryConversation`、`testGalleryLeavingConversation` | 检查 Toast 延时完成、Tree 加载失败后重试、触摸拖动文件后保留选中行、消息流完成及离页取消。 |
| iPad 开启窗口化应用 | `testGalleryResizedWindow`、`testGalleryBesidePhotos` | 分别拖动系统窗口右下角和从窗口控制选择「左右」与照片并排。两种窄窗都应保留当前详情，可返回目录，再恢复全屏。并排测试在预装「照片」的中文或英文模拟器运行。 |

安装 `--ui` 宿主后，`InteractionTests/testSystemPinyin` 可单独验证编辑控件接收系统候选及失焦后的正文。`InteractionTests/testEditorClipboard` 在真实编辑控件上使用 Command-A/C/X/V，核对剪切后输入新文字再粘贴的结果。`InteractionTests/testLargeTextControls` 使用 `xcrun simctl ui "$SIMULATOR_UUID" content_size accessibility-large` 切换系统字号后运行，结束时将其恢复为之前的值。
