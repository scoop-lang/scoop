# Linux ARM64 验收记录

原生编译和基础库验收在 o6 的 Debian 12.15（Bookworm）上进行。宿主为 AArch64，使用 4 KiB 页、glibc 2.36 和 musl 1.2.3。编译器使用仓库补丁版 LLVM 22.1.8。桌面和发行组件在同一台 o6 的 Ubuntu 26.04.1 ARM64 环境验证，页大小专测另启动 ARM64 Linux 客体内核。[三阶段设计](DESIGN.md)

## GNU 与 musl 链接

| ARM64 目标和模式 | 实际 ELF 解释器 | 运行结果 |
| --- | --- | --- |
| GNU，默认动态 PIE | `/lib/ld-linux-aarch64.so.1` | 跨 Cone 编译和独立重链接的两个程序均通过 GC 和异常测试 |
| musl，默认静态 | 无 `PT_INTERP` 和 `DT_NEEDED` | 静态程序和重链接结果均通过同一测试 |
| musl，显式 `--link-mode dynamic` | `/lib/ld-musl-aarch64.so.1` | 动态程序和重链接结果均通过同一测试，动态依赖包含 musl 的 `libc.so` |

`scoop build . --release --target aarch64-unknown-linux-musl --link-mode dynamic` 生成使用 musl 加载器的 ARM64 可执行文件。独立的 `scoop-link` 也接受该选项。用 GNU 目标重链接 musl 的 `.slib` 时实际得到 E2404。[CLI 构建与重链接](../../tests/fixtures/driver/linux/cli-program/fixture.toml) · [musl 动态用例](../../tests/fixtures/driver/linux/cli-program/pie.fixture.toml) · [Linux 共享库用例](../../tests/fixtures/driver/linux/native-dso/fixture.toml)

- 原生 GNU 的 7 个和 musl 的 8 个定向 fixture 均通过。覆盖 C ABI 的 HFA 和栈参数、Scoop 小值、深递归、原生回调、DSO 符号版本和线程局部存储。GNU 动态、musl 静态与动态的异常展开均执行了清理、捕获及释放。[AAPCS64 互调用例](../../tests/fixtures/compiler-correctness/direct-c/linux-aapcs64/fixture.toml) · [展开探测](../../scripts/check_linux_unwind.py)
- 两套目标均已用本机工具生成 `scoop.tls.slib` 和 `scoop.asyncio.slib`。Bookworm 上的构建覆盖了 32 字节 ELF 常量池、musl 原子操作辅助函数和 Linux UAPI 头的实际使用。[ELF 常量池修复](https://github.com/scoop-lang/scoop/pull/249) · [musl 最终链接修复](https://github.com/scoop-lang/scoop/pull/247) · [liburing 头文件修复](https://github.com/scoop-lang/scoop/pull/248)

## GC 与页大小

`getconf PAGESIZE` 在裸机返回 4096，在两个客体分别返回 16384 和 65536。三个环境的定向页归还和保护检查通过。在 64 KiB 页内核上，还运行了三个从实际 Scoop `.slib` 重链接的程序，以及 GNU、musl 各一个带 TLS 的 DSO 消费程序。它们的可执行文件 `PT_LOAD` 对齐均为 `0x10000`。[GC 页归还用例](../../runtime/tests/page_discard_test.c) · [64 KiB 页修复](https://github.com/scoop-lang/scoop/pull/245)

## 桌面与调试

o6 上的 Ubuntu ARM64 环境构建了全部 18 个 GNU 图形 Cone。在 X11 和 Wayland 上各运行了一次双 EGL 窗口用例，检查实际绘制、像素、缩放及重新挂接。X11 下的 SNI／DBusMenu 用例验证了菜单操作和 watcher 重启后的恢复。[EGL 窗口用例](../../sysroot/src/scoop.surface/tests-linux/host.scoop) · [托盘协议用例](../../sysroot/src/scoop.tray/test-linux/protocol.scoop)

Bookworm 上的 GNU 动态 PIE 和默认静态 musl 程序均通过配套 LLDB 的 DAP 启动测试，包括源码断点、栈帧、局部变量和源码 TLS 变量 `threadCounter`。静态 musl 的原生双线程测试还验证了已初始化、零初始化和 64 字节对齐的 TLS 存储。GNU 异常用例在 `Derived.read` 的实际抛出行停住，回溯包含调用方 `main`，继续执行后完成捕获和清理。[静态 TLS](https://github.com/scoop-lang/scoop/pull/251) · [GNU 动态 TLS](https://github.com/scoop-lang/scoop/pull/252) · [配套 LLVM 与 LLD](https://github.com/scoop-lang/scoop/pull/253)
