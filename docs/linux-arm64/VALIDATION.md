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
- GNU 和 musl 各构建了 28 个非图形基础库。每种目标均在 release 模式下运行 35 个库及消费者测试 Cone，共 2,104 项，2,094 项通过，10 项按用例声明忽略，0 项失败。两种目标的 core 消费者各通过 142 项测试，HTTP 客户端各通过 254 项。[core 消费者](../../sysroot/src/scoop.core/consumer-tests) · [HTTP 客户端测试](../../sysroot/src/scoop.http.client/tests)
- Bookworm 上的构建覆盖了宽字符字符串、32 字节 ELF 常量池、musl 原子操作辅助函数和 Linux UAPI 头的实际使用。[宽字符常量修复](https://github.com/scoop-lang/scoop/pull/250) · [ELF 常量池修复](https://github.com/scoop-lang/scoop/pull/249) · [musl 最终链接修复](https://github.com/scoop-lang/scoop/pull/247) · [liburing 头文件修复](https://github.com/scoop-lang/scoop/pull/248)

## GC 与页大小

`getconf PAGESIZE` 在裸机返回 4096，在两个客体分别返回 16384 和 65536。三个环境的定向页归还和保护检查通过。在 64 KiB 页内核上，还运行了三个从实际 Scoop `.slib` 重链接的程序，以及 GNU、musl 各一个带 TLS 的 DSO 消费程序。它们的可执行文件 `PT_LOAD` 对齐均为 `0x10000`。[GC 页归还用例](../../runtime/tests/page_discard_test.c) · [64 KiB 页修复](https://github.com/scoop-lang/scoop/pull/245)

## 桌面与调试

o6 上的 Ubuntu ARM64 环境构建了全部 18 个 GNU 图形 Cone。X11 使用 Xvfb 和 Openbox，Wayland 使用 Weston headless，两者均启用 Mesa 软件渲染。两个会话各运行了一次双 EGL 窗口用例，检查实际绘制、像素、窗口尺寸变化及重新挂接。X11 下的 SNI／DBusMenu 用例验证了菜单操作和 watcher 重启后的恢复。[EGL 窗口用例](../../sysroot/src/scoop.surface/tests-linux/host.scoop) · [托盘协议用例](../../sysroot/src/scoop.tray/test-linux/protocol.scoop)

Bookworm 上的 GNU 动态 PIE 和默认静态 musl 程序均通过配套 LLDB 的 DAP 启动测试，包括源码断点、栈帧、局部变量和源码 TLS 变量 `threadCounter`。静态 musl 的原生双线程测试还验证了已初始化、零初始化和 64 字节对齐的 TLS 存储。GNU 异常用例在 `Derived.read` 的实际抛出行停住，回溯包含调用方 `main`，继续执行后完成捕获和清理。[静态 TLS](https://github.com/scoop-lang/scoop/pull/251) · [GNU 动态 TLS](https://github.com/scoop-lang/scoop/pull/252) · [配套 LLVM 与 LLD](https://github.com/scoop-lang/scoop/pull/253)

## 从发行包安装与部署

在 o6 的 Ubuntu 26.04.1 ARM64 环境中，从本地发行索引把候选版本 `0.0.0-dev.arm64` 安装到空目录。打包使用的编译器源码版本为 `ce2c02c0f`。发行清单包含共用源码、vendor 源码、GNU 宿主工具、GNU 与 musl 各自的基础库，以及 GNU 图形库，共六个归档。[组件清单与发行流程](../../.github/workflows/release.yml) · [ARM64 安装器](https://github.com/scoop-lang/scoop-install/tree/218a900)

默认安装后编译并运行了 GNU 程序。追加 musl 目标后，通过系统的 `/usr/bin/musl-gcc` 构建并运行了默认静态程序和显式动态 PIE，`readelf` 结果与上表一致。发行包内的 `scoop-lldb-dap` 也在生成独立 `.debug` 文件的 GNU 程序上通过了源码断点、回溯与 TLS 变量检查。[调试验收程序](../../tooling/vscode-scoop/test/debugger/program.scoop)

追加 GNU GUI 组件后，安装的工具链构建了一个使用 GTK 和 Skia 的窗口程序，在 X11 和 Wayland 上分别完成绘制并关闭。使用 `scoop bundle` 生成 deb（`arm64`）、RPM（`aarch64`）及 AArch64 AppImage。解包后分别运行了 deb 和 RPM 中的程序，AppImage 通过自身的 `APPIMAGE_EXTRACT_AND_RUN=1` 模式启动。三种部署布局均成功读取随包资源并输出预期文本。[打包验收示例](../../tooling/scoop-bundle/examples/resources)

## 编译器与维护工具

在 Bookworm 上运行 `cargo test --workspace --no-fail-fast --locked`，共 6,532 项。首轮通过 6,521 项，11 项失败。修复后整组复验 `scoop-codegen`（410/410）和 `scoop-toolchain`（72/72）。`scoopc` 首轮通过 97 项，余下 3 项跨 Cone 产物与类型依赖用例在同一轮定向复验中全部通过。[runtime 测试栈帧](https://github.com/scoop-lang/scoop/pull/255) · [启动网关](https://github.com/scoop-lang/scoop/pull/256) · [并行标记与移动](https://github.com/scoop-lang/scoop/pull/259) · [Swift 输入更新](https://github.com/scoop-lang/scoop/pull/257) · [泛型产物](https://github.com/scoop-lang/scoop/pull/258) · [类型依赖投影](https://github.com/scoop-lang/scoop/pull/260)

`cargo fmt --all --check` 和 `cargo clippy --workspace --all-targets --locked` 完成。fixture runner 的 Python 测试共 86 项，跳过 4 项。发行维护脚本在 o6 的 Ubuntu ARM64 环境运行了 83 项，跳过 9 项。两组均无失败，94 个 Python 文件通过 Ruff 0.16.10 格式与 lint 检查。[fixture runner](../../tests/fixture_runner/README.md) · [发行脚本](../../scripts/README.md)

## 正式 CLI fixture

在 o6 的 Bookworm 上，对两个目标分别运行 `tests/run_fixtures.py --all`。每轮均选中全部 4,368 项，并实际执行适用的用例。下表为首轮报告，最后一列列出修复或更换配套环境后逐项复验的结果。

| ARM64 目标 | 首轮通过 | 按平台条件不适用 | 首轮失败 | 首轮启动的进程 | 失败项复验 |
| --- | ---: | ---: | ---: | ---: | ---: |
| GNU | 4,310 | 51 | 7 | 13,420 | 7/7 通过 |
| musl | 4,305 | 56 | 7 | 13,378 | 7/7 通过 |

两种目标均复验了文档目标索引、局部值优化、native 初始化、缺少锁文件的目标诊断、SSA 根和命令版本。musl 还复验了 native C 的目标宏。GNU 的 Lucide 图标裁剪用例在 o6 的 Ubuntu 发行环境用干净安装的候选工具链通过，包括不引用图标、单个图标、全部图标及增加未引用资源后的产物检查。[局部值优化](https://github.com/scoop-lang/scoop/pull/261) · [锁文件诊断](https://github.com/scoop-lang/scoop/pull/262) · [SSA 根与异常边](https://github.com/scoop-lang/scoop/pull/263) · [Lucide 裁剪用例](../../tests/fixtures/compiler-correctness/lucide-pruning/fixture.toml)

全量首轮沿用了移植期间的优化版命令，其中 `scoopc` 与其他四个命令的构建提交不同。第一批定向复验使用的五个配套命令在 Bookworm 上均报告 `8c42202b2`，Ubuntu 候选包的五个命令均报告 `ce2c02c0f`。最终配套 LLD 的 GNU 与 musl 链接路径另以独立重链接、64 KiB 页执行和发行包运行复验。[命令版本用例](../../tests/fixtures/driver/tool-versions/fixture.toml)

## Linux x86_64 回归

nuc12（Ubuntu 26.04.1 x86_64）的 GNU ld 2.46 在依赖文件中原样写出含空格的安装路径，首次回归发现解析时遗漏了 `libunwind.a`。修复后，x86_64 的 GNU ld 和 ARM64 的 LLD 22.1.8 都能从实际链接中识别完整归档路径，包括带空格、`#` 和美元符号的目录。两台机器都用合入修复后的 `9190c87a0` 重建了开发命令。Bookworm 上再次通过 GNU 动态和 musl 显式动态的 CLI 构建、独立重链接、运行和版本检查。[Linux 链接器路径修复](https://github.com/scoop-lang/scoop/pull/264)

x86_64 定向验证通过 4 项链接工具链与输入选择测试、10 项 ELF 常量池测试、8 项 runtime image、网关与 GC 标记测试。GNU 和 musl 各选 10 个正式 CLI fixture。每种目标首轮通过 9 项，需要从源码构建 runtime 的 `floating-ffi` 补齐开发 sysroot 的 vendor 源码和 unwind 头文件后通过。覆盖了 musl 默认静态与显式动态、带 TLS 的共享库、C 浮点与小值、SSA 根、独立重链接和跨 libc 产物诊断。[ELF 常量池](../../compiler/slib/src/link_object/literal_pools.rs) · [native DSO](../../tests/fixtures/driver/linux/native-dso/fixture.toml) · [浮点 FFI](../../tests/fixtures/language/floating/arithmetic/ffi/fixture.toml)
