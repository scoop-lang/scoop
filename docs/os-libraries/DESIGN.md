# Scoop 操作系统扩展库设计

设计草案，2026-11-26。建议以 `scoop.os` 为命名前缀，按目标发布五个普通 library Cone。首批从 Unix 文件元数据和命名管道开始。建议 iOS 的 UIKit 能力归入 `scoop.os.ios`，首版 `scoop.os.nix` 发布到 macOS 和 Linux。两项选择可以独立调整。

## 1. 库边界

调用方需要在不同系统上完成同一项操作时，使用 `scoop.platform` 已有的文件、目录、进程、网络和时间接口。需要操作系统特有的语义时，显式依赖对应的 `scoop.os.*` Cone。例如，查询文件大小用 `path.metadata().size`，比较两个 Unix 目录项是否指向同一文件则用设备号和 inode。两种调用共用同一个 `Path`。[平台库规范](../specs/SCOOP-PLATFORM-SPEC.md) · [库 API 风格](../specs/SCOOP-API-STYLE.md)

| 拟发布的 Cone | 首版目标 | 首个公开 package 与用途 |
| --- | --- | --- |
| `scoop.os.nix` | macOS arm64、x86-64，Linux x86-64 GNU、musl | `scoop.os.nix.fs` 提供 Unix 文件元数据和命名管道 |
| `scoop.os.macos` | macOS arm64、x86-64 | `scoop.os.macos.fsevents` 观察目录树变化 |
| `scoop.os.linux` | Linux x86-64 GNU、musl | `scoop.os.linux.io` 提供可由 fd 监听的内核事件计数器 |
| `scoop.os.windows` | Windows x86-64 的 MSVC/UCRT、LLVM-MinGW/UCRT、LLVM-MinGW/MSVCRT | `scoop.os.windows.registry` 读取注册表值 |
| `scoop.os.ios` | iOS arm64 真机、arm64 模拟器 | `scoop.os.ios.app` 查询 UIKit 应用的受保护数据状态 |

五个 Cone 使用 `scoop` group，分别位于 `sysroot/src/<Cone 名>/`。应用可只依赖其中一个，同时使用 Unix 公共 API 和 Linux 扩展时再声明两个直接依赖。扩展库按实际使用的类型依赖 `scoop.platform`。首个 iOS 查询只需要 core 和 UIKit。异步调度、窗口事件循环和终端会话分别由 `scoop.asyncio`、`scoop.window`、`scoop.terminal` 提供。[Cone 与 package 规则，语言规范 §12](../specs/SCOOP-SPEC.md) · [当前目标目录](../../compiler/target-data/targets.toml)

`nix` 的每个公开接口都应在表中列出的 macOS 和 Linux 目标上具有一致的语义。后续可按此原则扩展文件描述符、文件归属、进程组和本机 IPC。比如，基于文件系统路径的 Unix 域套接字适合在 `nix` 定义公共接口。Linux 的抽象套接字地址独立于文件系统，适合放在 `linux`。macOS 和 iOS 若有共同的 Apple 原生实现，可以共用该段实现，有跨两端的实际调用方时再决定公开入口。[Unix 域套接字地址语义](https://man7.org/linux/man-pages/man7/unix.7.html)

## 2. iOS 是否应当单列

评估对象是普通第三方 iOS 应用。引入 `scoop.os.ios` 可以给 UIKit 专属能力一个明确入口。另一个问题是 iOS 应用能否同时依赖 `scoop.os.nix`，这取决于该 Cone 对使用者承诺哪些能力。

| 候选能力 | 普通 iOS 应用中的依据 | 对公开接口的影响 |
| --- | --- | --- |
| 文件元数据 | 在 iPhoneOS 和 iPhoneSimulator 27.2 SDK 上，arm64 目标均通过了 `stat`、`lstat`、`fstat` 的声明检查。应用也有读取自己容器内文件属性的用例。 | `unixMetadata` 是值得在三端共享的候选，需在真机上核对符号链接、文件身份和路径错误。 |
| 文件描述符与 FIFO | 同一检查中，`openat`、`read`、`write`、`dup`、`mkfifo` 均可编译。 | 已有 fd 的读写与创建命名管道是两种能力。`createFifo` 在应用容器中的创建、打开和传输仍待真机验证。 |
| 子进程与进程组 | 普通 iOS 应用不能自行派生子进程。 | 若 `nix` 提供启动子进程、重定向并等待退出的组合，iOS 无法承诺这套语义。 |
| 受保护数据 | UIKit 的 `isProtectedDataAvailable` 查询当前应用的数据保护状态。 | 归入 `ios.app`，与文件 API 是否将来加入三端共享无关。 |

SDK 检查使用 `xcrun clang -fsyntax-only`，覆盖设备与模拟器的目标头文件。设备上的沙盒及文件系统行为仍以实际应用调用为准。[Apple 文件系统指南](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html) · [Apple DTS 对子进程的说明](https://developer.apple.com/forums/thread/747499) · [UIKit 数据保护状态](https://developer.apple.com/documentation/uikit/uiapplication/isprotecteddataavailable)

Scoop 给子进程设置工作目录所用的 `posix_spawn_file_actions_addchdir` 及其 `_np` 版本也在 iOS SDK 标为不可用。另一些接口虽可调用，用途却不同。例如 `getuid()` 可取得真实 UID，而普通 iOS 应用通常以同一 `mobile` 用户运行，应用隔离由沙盒实施。[Scoop 子进程实现](../../sysroot/src/scoop.platform/native/process.c) · [Apple 平台安全说明](https://support.apple.com/en-ie/guide/security-pdf/sec15bfe098e/1/web/1)

首版建议让 macOS、Linux 的调用方可以依赖 `nix` 的完整公开接口，iOS 真机和模拟器则独立发布 `scoop.os.ios`。这样可以按 Unix 程序的需要规划进程和 IPC 接口，代价是 iOS 调用方暂时无法以同一条 import 使用其中的文件子集。目前 `scoop.platform` 的 POSIX 源集选择 macOS 和 Linux，两个 iOS sysroot 也只列出 core 和 std。[平台清单](../../sysroot/src/scoop.platform/Cone.toml) · [发行目标配置](../../scripts/release.toml)

若同一份库代码确实需要在三端使用 `openat`、`fstat` 等接口，再评估给 iOS 发布 `scoop.os.nix.fs` 子集。届时用该调用方验证相同的路径表示、错误和 fd 所有权，并先解决 `Path`、`PlatformException` 等直接依赖在 iOS 上的供应。判断依据是该用例在真机上成立，以及共享公开 API 能省去多少调用方适配。

## 3. Unix 首批接口

下列签名属于 `scoop.os.nix.fs`。其中 `Path` 和 `FileMetadata` 来自 `scoop.platform.fs`。

```scoop
public struct UnixFileMetadata(
    val basic: FileMetadata,
    val userId: UInt,
    val groupId: UInt,
    val deviceId: ULong,
    val inode: ULong,
    val linkCount: Long
)

public fun Path.unixMetadata(followLinks: Boolean = true): UnixFileMetadata
public fun Path.createFifo(mode: Int)
```

`unixMetadata()` 默认调用一次 `stat`。`followLinks = false` 时调用一次 `lstat`，观察最后一个目录项自身，包括指向缺失目标的符号链接。`basic` 和其余字段从同一份 `struct stat` 生成。把已有 POSIX 实现中转换文件种类、权限和时间的部分提取给两个调用方共用，Scoop 侧沿用 `decodeFileMetadata`。[现有 POSIX 元数据转换](../../sysroot/src/scoop.platform/native/file_metadata.c) · [现有 Scoop 元数据解码](../../sysroot/src/scoop.platform/src/native/file_metadata.scoop)

`userId`、`groupId` 是所有者和所属组，`deviceId`、`inode` 分别来自 `st_dev`、`st_ino`，较窄标识按目标位宽保留位模式后零扩展。`linkCount` 是硬链接数，按库的计数约定转换为 `Long`，超出表示范围时抛 `ArithmeticException`。设备号与 inode 组合可比较当次观察到的文件身份，删除后的 inode 可能被重用。

`createFifo(mode)` 创建文件系统中的命名管道节点，成功返回 `Unit`。调用者显式给出权限，例如 `path.createFifo(mode = 0x180)` 请求八进制 0600。沿用 `requireFileMode` 检查权限位，实际读写权限受进程 umask 影响。用目标头文件把已检查的值转换为 `mode_t` 后调用 `mkfifo`。Darwin 的 `mode_t` 是 16 位，直接照搬现有测试里的 `UInt` extern 声明会丢掉这个 ABI 差异。已有同名节点时保留 `EEXIST`，打开端点和删除节点分别使用现有 `File.open` 和 `Path.delete`。[已有 FIFO 调用方](../../sysroot/src/scoop.asyncio/test-platform/unix/file_running_work.scoop) · [现有 mode 检查](../../sysroot/src/scoop.platform/src/native/file_options.scoop) · [Darwin 类型定义](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/_types.h) · [mkfifo 语义](https://man7.org/linux/man-pages/man3/mkfifo.3.html)

两个接口都通过 `Path.withNativePath` 同步借用原始路径字节。失败在原生调用返回时捕获 errno，按已有 `PlatformException` 提供 operation、`NativeError(PosixErrno, code)` 和原始路径上下文。首批操作只需 `Path`。将来若需要按打开的文件操作，再给 `File` 设计限定借用期的入口，目前的 `File.withHandle` 是 Cone 内部成员。[路径借用](../../sysroot/src/scoop.platform/src/fs/native_strings.scoop) · [File 所有权](../../sysroot/src/scoop.platform/src/fs/file.scoop) · [平台错误合同](../specs/SCOOP-PLATFORM-SPEC.md)

## 4. 各平台的第一个用例

### Linux 事件计数器

`EventFd.open()` 以初值 0 创建非阻塞、close-on-exec 的普通计数器，由实现 `Closeable` 的对象独占。`tryAdd(value: ULong): Boolean` 在加值成功时返回 `true`，剩余容量不足时返回 `false`，`ULong.MAX_VALUE` 是非法参数。`tryTake(): ULong?` 取出累计值并清零，当前为空时返回 `None`。`false` 和 `None` 分别对应写、读时的 `EAGAIN`。被 `EINTR` 打断且尚未传输时重试，其他系统错误保留原码。窗口后端的唤醒 fd 和现有监听测试都是这个用例。[Linux eventfd(2)](https://man7.org/linux/man-pages/man2/eventfd.2.html) · [窗口唤醒代码](../../sysroot/src/scoop.window/native/linux/application.c)

与现有 `watchReadable` 对接时，`EventFd` 提供标注 `@Unsafe` 的方法 `fun <R> withFileDescriptor(block: (Int) -> R): R`。回调期间保活所有者，fd 仅供同步借用。若回调内运行窗口循环，监听须在借用结束前完成注销并退出在途回调。借用期间重入 `close()` 抛 `IllegalStateException`，跨线程使用与关闭由调用方同步。正常关闭先使对象失效，再执行一次原生 close。重复关闭无操作，已关闭的对象拒绝新的读写或借用。[现有监听与注销用例](../../sysroot/src/scoop.window/tests-cones/watch/test-linux/watch.scoop)

### Windows 注册表

先支持从 `RegistryHive.CurrentUser` 和 `RegistryHive.LocalMachine` 读取 `REG_DWORD` 与 `REG_SZ`。在 `RegistryHive` 上提供方法 `readDwordOrNone(subkey: OsString, name: OsString): UInt?` 和 `readStringOrNone(subkey: OsString, name: OsString): OsString?`。窗口主题读取 DWORD，桌面通知从注册表读取字符串。首批目标均为 x86-64，默认查询进程的原生注册表视图，空值名选择键的默认值。[窗口主题调用](../../sysroot/src/scoop.window/native/windows/properties.c) · [桌面通知调用](../../sysroot/src/scoop.desktop/native/windows/notification_text.cpp)

调用 `RegGetValueW` 时按具体类型筛选，字符串查询关闭环境变量展开。名称入参拒绝内嵌 NUL，读取的字符串去掉结尾的终止符，其余 UTF-16 code unit 由 `OsString` 无损保存。缺失的键或值返回 `None`，已存在的空字符串返回 `Some(OsString.fromString(""))`。值在两次读取之间变长时按 `ERROR_MORE_DATA` 重新取数。类型错误、拒绝访问等失败使用函数直接返回的 `LSTATUS` 构造 Win32 域的 `NativeError`。通过 manifest 声明 `advapi32`。[Microsoft RegGetValueW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-reggetvaluew) · [OsString 与原生错误](../specs/SCOOP-PLATFORM-SPEC.md)

### macOS 目录树变化

用 FSEvents 观察一个目录树从订阅成功开始的变化。建议入口为 `DirectoryWatch.open(root: Path)`，所有者实现 `Closeable`。调用方用 `nextBatch(): Array<DirectoryEvent>?` 等待一批事件，每项保留系统报告的路径（`Path`）、事件 ID（`ULong`）和标志位（`UInt`）。FSEvents 可以合并多个变化。`MustScanSubDirs` 指示递归扫描事件指出的目录，`UserDropped` 或 `KernelDropped` 指示从整个监听根目录重新扫描。[Apple FSEvents 编程指南](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/UsingtheFSEventsFramework/UsingtheFSEventsFramework.html)

系统回调在原生 dispatch queue 上复制本批数据，由 `nextBatch()` 的调用线程构造 Scoop 值。同一时刻允许一个等待者，第二个并发读取抛 `IllegalStateException`。关闭时停止订阅、唤醒等待者并等在途原生回调退出，再释放 stream 和上下文。已取出的批次可交付，剩余批次丢弃，关闭后的 `nextBatch()` 返回 `None`。只有布尔失败信息的启动操作用 `PlatformException` 的 `Other` 和空 `nativeError` 表达。该 Cone 通过 manifest 声明 CoreServices framework。[原生调用与线程状态，运行时规范 §3.5、§4](../specs/SCOOP-RUNTIME-SPEC.md)

### iOS 受保护数据

`isProtectedDataAvailable(): Boolean` 读取当前 UIKit 应用的 `UIApplication.isProtectedDataAvailable`，供应用决定何时重新打开受保护文件。返回值描述查询时刻的系统状态，单个文件能否读取还取决于其保护类别。Objective-C 包装通过现有 C ABI 返回布尔值，由本 Cone 声明 UIKit framework。入口限于已经启动的完整 UIKit 应用，因为取得应用实例的 `sharedApplication` 在 SDK 中标为 app extension 不可用。[Apple UIKit 接口](https://developer.apple.com/documentation/uikit/uiapplication/isprotecteddataavailable?language=objc) · [Apple 原生源码选择，语言规范 §12.2.1](../specs/SCOOP-SPEC.md)

## 5. 构建与发布

生产者用 `[[sources]]` 对整个 Cone 的生产源码设置表中所列条件，native 源码、系统库和测试源按同一目标选择。清单值 `darwin` 匹配 macOS，`ios` 匹配 iOS 真机与模拟器，`env = "simulator"` 只用于确有差异的模拟器文件。桌面 sysroot 的现有准备脚本按有匹配源码的 Cone 和依赖拓扑构建。交付 iOS 库时，将 `scoop.os.ios` 加入设备与模拟器两份 `system_cones`。[源码与依赖条件，语言规范 §12.2](../specs/SCOOP-SPEC.md) · [sysroot 准备脚本](../../scripts/prepare_sysroot.py)

例如，一份同时面向桌面和 iOS 的应用可在清单中选择 Unix 文件扩展。下列是它的 `Cone.toml` 片段，`src/common` 放各端通用代码，`src/unix` 中的文件导入 `Path`、`unixMetadata` 或 `createFifo`。

```toml
[dependencies]
"scoop:scoop.platform" = { version = "0.1.0", when = { os = ["darwin", "linux"] } }
"scoop:scoop.os.nix" = { version = "0.1.0", when = { os = ["darwin", "linux"] } }

[[sources]]
path = "src/common"

[[sources]]
path = "src/unix"
when = { os = ["darwin", "linux"] }
```

PM 在定位依赖和求解版本前按目标选边，类型检查只看到该目标选中的源码。用户消费本目标的预编译 `.slib`。若消费者无条件声明了当前目标未发布的库，依赖定位阶段会报告缺少该目标产物。[包管理规范](../specs/SCOOP-PACKAGE-SPEC.md) · [产物与链接，编译器规范 §2.6–2.8](../specs/SCOOP-IMPL-SPEC.md)

可能阻塞的调用走现有 NativeSafe FFI，使用 errno 的简单接口可用 `@Extern(captureErrno = true)`。需要转换目标 C 类型、结构体布局或系统回调时，由所属 Cone 携带少量 C 或 Objective-C 源码。创建资源中途失败时，先保存该次操作的错误，再释放已经取得的原生资源。[FFI 错误捕获，语言规范 §13.4.2](../specs/SCOOP-SPEC.md)

## 6. 交付与验收

第一步交付 `nix.fs`，让现有 asyncio Unix 测试通过条件测试依赖直接使用 `Path.createFifo`。随后 Linux 计数器、Windows 注册表和两个 Apple 库可以按各自用例独立交付。每项实施时先把确定的公开行为写入领域规范，随实现更新对应的编程材料。[编程材料更新流程](../scoop-programming/UPDATE-WORKFLOW.md)

| 用例 | 可观察的验收结果 |
| --- | --- |
| Unix 文件系统 | 创建 FIFO 后，`unixMetadata().basic.kind` 是 `FileKind.Fifo`。重复创建保留 `EEXIST`。符号链接默认返回目标的设备号与 inode，关闭跟随后能观察链接本身，即使目标缺失。现有 asyncio 用例继续在 FIFO 上传输字节。 |
| Linux 计数器 | 累加 2 和 3 后读出 5，再读为空。把计数器加到上限时再次加 1 返回 `false`。窗口监听用例在注销后仍可由 owner 取走计数，关闭后拒绝新操作。 |
| Windows 注册表 | 在当前用户的临时测试键读取高位为 1 的 DWORD、空字符串和 Unicode 文本，分别检查缺失值与类型不匹配，结束后清理测试键。 |
| macOS FSEvents | 观察临时目录，修改其中的文件，验证通知可引导调用方重新枚举到修改结果。允许系统合并通知。等待期间关闭 stream 能让读取者退出，释放后无回调访问原生上下文。 |
| iOS 受保护数据 | 用 UIKit 宿主消费目标产物，在模拟器上与同次场景的 UIKit 查询对照。涉及设备锁定与解锁的行为，在启用了数据保护的真机上对照观察并单独记录。[iOS native-app 验收入口](../../tests/ios-native-app/README.md) |

普通库行为放在所属 Cone 的 release 测试中。跨目标选择和预编译产物链路用代表性的真实 consumer 覆盖，iOS 使用现有 `native-app` 与 XCFramework 宿主路径。表中各项均为拟验收行为。
