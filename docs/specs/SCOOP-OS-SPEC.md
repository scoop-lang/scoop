# Scoop 操作系统扩展库规范

五个普通 library Cone 分别提供 Unix 共同能力和各目标专有的操作。消费方显式声明直接依赖并导入公开 package。目标选择与原生链接沿用语言规范 §12.2，基础类型、错误和确定性关闭沿用平台库规范及语言规范 §11.18。[设计与目标选择理由](../os-libraries/DESIGN.md) · [语言规范](SCOOP-SPEC.md) · [平台库规范](SCOOP-PLATFORM-SPEC.md)

## Unix 文件系统

`scoop.os.nix.fs` 在 macOS 和 Linux GNU、musl 上由 `scoop.os.nix` 提供。`Path.unixMetadata(followLinks: Boolean = true): UnixFileMetadata` 返回一次 stat 或 lstat 的快照。值类型字段为 `basic: FileMetadata`、`userId: UInt`、`groupId: UInt`、`deviceId: ULong`、`inode: ULong`、`linkCount: Long`。basic 与其他字段取自同一份结果。设备号按目标位宽保留全部位再零扩展，链接数超出 Long 抛 ArithmeticException。关闭跟随只观察最后一个路径分量自身，悬空符号链接也可观察。

`Path.createFifo(mode: Int)` 创建命名管道节点并返回 Unit。mode 要求位于 0 到 0xfff，实际权限受 umask 影响。路径按原始字节传递，系统失败抛 PlatformException，包含 PosixErrno 和原始路径。重复创建保留 EEXIST。节点的打开、读写与删除使用已有 File 和 Path 操作。

## Linux 事件计数器

`scoop.os.linux.io.EventFd` 在 Linux GNU 和 musl 上由 `scoop.os.linux` 提供。open() 创建初值为零的非阻塞、close-on-exec 普通 eventfd。`tryAdd(value: ULong): Boolean` 成功返回 true，空间不足返回 false，ULong.MAX_VALUE 抛 IllegalArgumentException。`tryTake(): ULong?` 返回累计值并清零，空计数器返回 None。无进度的 EINTR 重试，其余系统错误保留 PosixErrno。

EventFd 实现 Closeable，有 isClosed 属性。`@Unsafe fun <R> withFileDescriptor(block: (Int) -> R): R` 同步借用原生 fd，调用者在回调结束前退订借入 fd 的监听，不得关闭、转移或留存 fd。借用中重入 close 抛 IllegalStateException。调用者同步跨线程的关闭与使用。正常关闭先使对象失效，再关闭 fd 一次。关闭后拒绝新读写和借用，重复关闭无操作。

## Windows 注册表

`scoop.os.windows.registry.RegistryHive` 在三个 Windows x86-64 目标上由 `scoop.os.windows` 提供。CurrentUser 和 LocalMachine 选择预定义根键，按当前进程的原生视图查询。`readDwordOrNone(subkey: OsString, name: OsString): UInt?` 仅读取 REG_DWORD，`readStringOrNone(subkey: OsString, name: OsString): OsString?` 仅读取 REG_SZ。空子键选择根键本身，空值名选择默认值。名称无损转 UTF-16，内嵌 NUL 抛 IllegalArgumentException。

键或值缺失返回 None。空字符串存在时返回 Some，去掉一个结尾的 UTF-16 终止符，保留其余 code units。字符串不展开环境变量。类型不匹配、访问失败等保留 RegGetValueW 直接返回的 Win32 错误码。读取变长结果时按 ERROR_MORE_DATA 重新取得完整值。

## macOS 目录事件

`scoop.os.macos.fsevents.DirectoryWatch.open(root: Path)` 在两个 macOS 目标上通过 CoreServices 订阅目录树从现在开始的 FSEvents。`nextBatch(): Array<DirectoryEvent>?` 阻塞到有一批事件或关闭。每个事件保留 `path: Path`、`id: ULong`、`flags: UInt`。MustScanSubDirs 要求递归扫描事件指出的路径，UserDropped 或 KernelDropped 要求扫描监听根。系统可合并变化，事件数量和每批边界由系统决定。

DirectoryWatch 实现 Closeable，同一时刻只允许一名读取者，第二个并发读取抛 IllegalStateException。close 可从另一线程发起，唤醒等待读取者并等待原生回调退出。已取走批次仍可交付，其余丢弃。关闭后 nextBatch 返回 None，close 幂等。没有原生错误码的 stream 启动失败抛 kind 为 Other、nativeError 为 None 的 PlatformException。

## iOS 受保护数据

`scoop.os.ios.app.isProtectedDataAvailable(): Boolean` 在 arm64 真机及模拟器上由 `scoop.os.ios` 提供。已启动的完整 UIKit 应用查询当前 UIApplication 的受保护数据状态，具体文件的可访问性取决于保护类别。调用依赖 UIKit sharedApplication，适用宿主不含 app extension。两个 iOS 目标各自提供本目标的预编译 Cone。
