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

## UIKit 受保护数据

在上述模拟器验证命令加上 `--protected-data` 并使用另一个工作目录，会构建普通 release 应用。UIKit 启动后，宿主通过 Scoop 回调查询数据保护状态，与同一时刻的 UIKit 返回值对照。成功标记为 `scoop-ios-protected-data-ok`。

同时加上 `--include-device` 会生成 `device/protected.app`。在启用密码和数据保护的设备上开发签名并安装该宿主，以 `dev.scoop.ios-probe.protected` 启动并观察控制台。锁屏、等待数据保护生效、解锁，记录 `will-lock` 与 `did-unlock` 通知中的两次查询。前者发生在状态即将改变时。宿主会在后台短暂继续采样，`background scoop=0 UIKit=0` 表示两次查询都观测到数据已受保护。
