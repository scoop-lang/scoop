# Linux ARM64 移植设计

设计稿，2026-11-27。调研基线为 `main` 的 `33f712b0969a`。这一版支持 Linux 上的 amd64 和 Apple 平台上的 ARM64。移植完成后，用户可在 ARM64 Linux 上安装 Scoop，默认编译 glibc 程序，也可显式选择 musl。桌面库随 GNU 目标提供。

| 阶段 | 交付物 | 用户可以完成的操作 |
| --- | --- | --- |
| 一 · GNU 原生编译 | ARM64 GNU 宿主开发命令、目标 runtime、预编译 core 和 std | 从源码构建跨 Cone 程序，在 Linux 上执行，用同一批 `.slib` 独立重链接并再次执行 |
| 二 · musl 与基础库 | 两套 libc 各自的非图形系统库与 native 依赖，适配 64 KiB 页的 GC | GNU 使用 C/C++，musl 使用 C 并生成静态或动态程序，两套基础库均可运行所属 Cone 测试 |
| 三 · 发行、桌面与工具 | GNU 宿主工具包、两份基础库包、GNU 图形库包、安装器与调试器 | 从发行包安装，在 ARM64 Linux 上构建、调试和打包 CLI 或桌面应用 |

现有代码可复用 LLVM 的 AArch64 后端、Linux 线程与虚拟内存服务、ELF 映像枚举、AArch64 浮点环境，以及 Linux 链接器驱动。过程 ABI、ELF machine、机器帧和 libc 由目标目录分别选择。各阶段先修订对应规范，再提交实现与验收用例。[目标组合设计](../target-refactor/COMPOSITION.md) · [编译器与产物规范](../specs/SCOOP-IMPL-SPEC.md) · [运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)

三阶段的原生构建与运行结果见[验收记录](VALIDATION.md)。

## 实施与验收环境

| 环境 | 用途 |
| --- | --- |
| `o6` 的 Debian 12.15（Bookworm）ARM64 宿主 | 构建仓库补丁版 LLVM 及编译器，准备并运行 GNU、musl 基础库。宿主使用 4 KiB 页、glibc 2.36，Debian 提供 musl 1.2.3。 |
| `o6` 上的 16 KiB、64 KiB 页 ARM64 客体内核 | 用 `getconf PAGESIZE` 确认实际页大小，运行 GC 页归还和保护测试。64 KiB 环境另运行链接产物及其动态库。 |
| `o6` 上的 Ubuntu 26.04.1 ARM64 容器 | 验证发行基线、GNU 桌面库与安装包。该环境提供 glibc 2.43、musl 1.2.5，以及 GTK 4.22.4 和 libadwaita 1.9.1。 |

## 阶段一　GNU 宿主与基础编译

### 目标登记与调用约定

在目标目录登记 `aarch64-unknown-linux-gnu` 和对应的 GNU 宿主默认项。target profile 采用 `org.scoop-lang.target-profile/linux-aarch64-gnu/1`，backend 采用 `org.scoop-lang.backend-profile/llvm-22-1-linux-aarch64/1`。阶段二的 musl 复用该 backend，拥有独立的 target profile。Rust 与 Python 均从现有目录读取 `os = linux`、`arch = aarch64` 和所选 `env`。先更新实现规范 §1、§2.4～2.8，以及运行时规范 §3.2、§5。[目标目录](../../compiler/target-data/targets.toml) · [backend 登记](../../compiler/target-data/src/backend.rs)

| 机器合同 | 设计 |
| --- | --- |
| 指令与布局 | 小端 LP64，LLVM CPU 为 `generic`，基线为 Armv8-A，调用栈 16 字节对齐。仓库 Clang 22.1.8 对两个 Linux triple 给出的 data layout 均为 `e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32`。 |
| C ABI | 增加 Linux AAPCS64 分类。实测 Clang 的 8/16 位整数和 `_Bool` 参数、结果均无 `signext`／`zeroext`。复用已验证的 AArch64 聚合分类代码，对 HFA（1～4 个同种浮点成员）、寄存器耗尽后的栈对齐、普通小聚合及大聚合分别与 Clang 互调。大结构体由 caller 提供副本，间接返回地址走 `x8`。 |
| Scoop ABI | 沿用 interface 的 object／itab 双分量和 GC-free 小值规则。Scoop 小值直接传递的上限为 16 字节、对齐至多 8 字节。C ABI 单独处理 HFA，例如四个 Double 仍有自己的浮点传参规则。直接调用、函数指针和回调使用各自 ABI 的同一份物理签名。 |
| native 工具链 | 给现有 Linux machine 和 GCC／Clang 的 C bridge profile 增加 AArch64。Clang 使用目录中的 triple，GCC 使用匹配的 driver，generated C、用户 native、runtime 和 startup 使用一致的 CPU 基线与目标 headers。跨宿主准备时由 `--cc` 和 `--native-sysroot` 指定 Linux 开发资源。 |

Linux ARM64 另选 codegen 与 runtime 的初始调优配置。先用保留栈重读、调用现有 GC forwarding 函数的保守路径跑通验收。启用 Darwin ARM64 上测过的可选优化前，使用现有基准在 Linux ARM64 上比较结果。[代码生成调优](../../compiler/codegen/src/target/tuning.rs)

ABI 规划保留在 codegen。栈传参的对齐随参数计划传给发射器，例如 Clang 对含两个 Double 的 HFA 参数发出 `[2 x double] alignstack(8)`。用前置参数占满浮点寄存器的用例验证实际栈位置，覆盖直接调用和通过函数指针调用。[ABI 规划器](../../compiler/codegen/abi-plan/src/c.rs) · [C bridge 配置](../../compiler/target-data/src/c_bridge.rs) · [Arm AAPCS64](https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst)

小聚合的实参与返回值分别分类。以一个只含 `char bytes[15]`、结构体显式对齐到 16 字节的 C 类型为例，仓库 Clang 在 Linux AArch64 上用 `[2 x i64]` 传参、用 `i128` 返回。布局规划需同时保留成员要求的对齐和结构体最终对齐。用同一结构体在 C 与 Scoop 之间往返，能发现只测函数声明时容易漏掉的寄存器错位。[聚合 ABI 规划](../../compiler/codegen/abi-plan/src/aggregate.rs)

### ELF 对象、精确栈根与异常

**ELF 对象。** 在 LIR 增加小端 ELF64 AArch64 对象目标，在 slib 的 ELF reader 中按 `EM_AARCH64`、`e_flags = 0` 和 RELA 规则解析。元数据与 stackmap 的函数地址使用 `R_AARCH64_ABS64`，异常样例使用 `PREL32/64`，直接调用及跳转使用 `CALL26`／`JUMP26`。PIC 外部数据及线程局部数据还出现 GOT 页寻址和 `TLSDESC_*` 指令组。按实际发射形式区分重定位字段宽度、绝对指针和 TLS 用途，沿用现有 ELF 容器解析及系统 linker。用 Scoop 对象、generated C、native archive 和 DSO 检验这些生产者与消费者，另测混入 amd64 对象时的目标诊断。[ELF 架构选择](../../compiler/slib/src/link_object/elf/architecture.rs) · [Arm ELF ABI](https://github.com/ARM-software/abi-aa/blob/main/aaelf64/aaelf64.rst)

构建 release 版 `scoop.std` 时，LLVM 的 ADRP 和 128 位 LDR 会分别以 section symbol 加偏移引用 `.rodata.cst16` 内的常量。实际对象含 0、16 至 112 字节的多个元素偏移。写入 `.slib` 时按偏移取该元素的 16 字节参与 callable 指纹，保留原生对象的指令与重定位。阶段一用完整的 `scoop.std.slib` 检验这条路径。[常量池读取](../../compiler/slib/src/link_object/literal_pools.rs)

**栈帧。** 在设计阶段的 4 KiB 页 ARM64 Linux 容器中，仓库 LLVM 22.1.8 发射的同一个 statepoint 样例测得以下值。两种优化级别均按返回 PC 找到 stackmap，改写根槽后，生成代码读到了新引用。Darwin 解码器的固定公式 `FP = SP + 帧大小 - 16` 会拒绝这里的优化帧。[LLVM 探针基础](../../runtime/tests/linux_frames_test.ll) · [Darwin 帧恢复](../../runtime/src/platform/frame/darwin_aarch64.c)

| LLVM 优化级别 | stackmap 帧大小 | 实际 FP − SP | 引用槽地址 |
| --- | ---: | ---: | --- |
| `-O0` | 48 字节 | 32 字节 | `SP + 24` |
| `-O2` | 48 字节 | 16 字节 | `SP + 8` |

Linux AArch64 使用保留 FP 的固定大小帧，关闭 red zone 和影响遍历的尾调用。ELF `ManagedEntry` 在 C prologue 之前保存生成代码的返回 PC（LR）、调用点 SP 和 FP。每层按以下规则恢复。

1. 以返回 PC 精确查找已发布的紧凑 GC 表，取该函数的帧大小 `N`。含 safepoint 的帧至少 16 字节，大小是 16 的倍数。帧区间为 `[SP, SP + N)`，实际 FP 指向其中的 16 字节帧记录。
2. 普通根从 stackmap 指定的 SP（DWARF 31）或 FP（DWARF 29）加有符号偏移定位。普通引用槽宽 8 字节，region table 占 16 字节。对象生产时检查 location 种类、对齐、base/derived 配对和静态可判定的范围。依赖实际 FP 的边界在扫描时检查，槽位须完整位于帧内并避开保存的 FP/LR。
3. 上一层 SP 取 `SP + N`，上一层 FP 和返回 PC 分别取 `[FP]`、`[FP + 8]`。地址加法检查溢出，遍历遵守线程栈界与已发布的 managed boundary。codegen 保证 safepoint 处的 SP 与固定帧匹配，尤其是需要栈上传参的调用。

另一个双层探针给内层函数传入 10 个参数，其中两个整数从栈传递。按上述公式恢复外层，在 `-O0` 和 `-O2` 均匹配到两个返回 PC，内外层都读回改写后的引用。这两组探针观察到的普通根均相对 SP。实施时把嵌套调用纳入真实 runtime 回归，并验证 FP 相对槽、region table、大帧、间接参数、异常出口及跨 native callback 的栈段。精确根继续使用已有紧凑表，运行时直接消费寄存器选择和偏移。[stackmap 归一化](../../compiler/slib/src/link_object/stackmap_normalization/frame.rs) · [运行时栈段遍历](../../runtime/src/gc_binding/stack_roots.c)

**异常与汇编入口。** 使用 Itanium/DWARF 展开。实际带 catch 的 GNU、musl 对象均发出 LSDA 类型表编码 `0x9c`（间接、PC 相对、有符号 8 字节），纯 cleanup 为 `0xff`，LPStart 为 `0xff`，call-site 为 `0x01`。解码器据此定位 8 字节表项，沿用 Scoop 唯一全零 catch-all 表项的规则。源码类型匹配仍由 landing pad 后的 Scoop 代码完成。C 链接路径使用为该 target 构建的补丁版 LLVM libunwind。为 `ManagedEntry` 和 Float／Double 带 tag 结果提供 ELF 汇编符号、函数边界与必要的展开描述，接入现有 runtime 组件选择。验收时执行两阶段展开，并验证异常期间活跃的 managed 引用。[LSDA 解码](../../runtime/src/eh/lsda_encoding.h) · [runtime 组件选择](../../compiler/toolchain/src/runtime.rs) · [异常展开用例](../../runtime/tests/eh_unwind_test.ll)

### GNU 阶段验收

在 4 KiB 页的原生 ARM64 Linux 上构建配套的 `scoop`、`scoopc`、`scoop-link` 和负责解析依赖的 `scoop-pm`，使用同一套仓库 LLVM 22.1.8 SDK。用现有 `[system_cones]` 配置先选择 `scoop.core`、`scoop.std`，显式准备目标 runtime、unwind 和开发 sysroot。普通程序消费 `lib/aarch64-unknown-linux-gnu/` 中的预编译输入，默认链接动态 PIE。[sysroot 准备入口](../../scripts/prepare_sysroot.py) · [Linux 链接模式](../../compiler/toolchain/src/final_link/linux.rs)

准备顺序是先在目标目录登记 GNU triple，并在 `release.toml` 的对应目标表中填写 `cpu = "generic"`，再构建目标 libunwind，然后准备 runtime 和两个系统 Cone。`build_llvm_unwind.py` 直接从该表取 CPU，缺少这项时，即使 LLVM 已能发射 AArch64 对象，开发 sysroot 也无法从头准备。[libunwind 构建脚本](../../scripts/build_llvm_unwind.py) · [目标构建配置](../../scripts/release.toml)

- 扩展现有 Linux CLI fixture。一个库 Cone 提供有 C 边界的函数，应用跨 Cone 调用、分配对象、触发收集并捕获异常。先运行 `scoop build` 的结果，再由 `scoop-link` 从同一组应用与依赖 `.slib` 和 runtime index 重链接，两个程序给出相同的预期输出和退出码。[CLI 编译与重链接用例](../../tests/fixtures/driver/linux/cli-program/fixture.toml)
- 定向 GC 专测在 debug 和 release 上运行深栈、移动后重读及 callback 重入。ABI 专测覆盖负窄整数、Boolean、HFA 溢出到栈、大结构体返回和浮点结果 shim。GNU 目标与 ARM64 宿主的登记随这些实际入口一起交付。[深栈用例](../../tests/fixtures/gc-correctness/linux/deep-frames/fixture.toml) · [native 互调用例](../../tests/fixtures/compiler-correctness/native-link/)
- 展开验证脚本现会遍历所有 Linux triple 并固定传入 `0x9b`。改为按所测目标选择 encoding、C driver 和链接模式，在对应架构上执行。通用 Linux fixture 以目标条件选择，预期输出中的机器名取自该目标。保留针对特定机器指令或重定位的专测。[展开验证脚本](../../scripts/check_linux_unwind.py) · [fixture 目标条件](../../tests/fixture_runner/README.md)

## 阶段二　musl、非图形基础库与 64 KiB 页

登记 `aarch64-unknown-linux-musl`，target profile 为 `org.scoop-lang.target-profile/linux-aarch64-musl/1`。两种 libc 共用第一阶段的机器 ABI、ELF reader、帧恢复和异常编码。Scoop sysroot 在 `lib/<triple>/` 下分别放置各自的 runtime、unwind、系统 `.slib` 与 native archive。C bridge producer 和构建记录同样使用所选目标身份。GNU 宿主通过 `--target` 选择 musl。[目标与宿主目录](../../compiler/target-data/targets.toml) · [运行库选择](../../compiler/toolchain/src/final_link/linux/runtime.rs)

| 应用目标 | 链接和运行覆盖 |
| --- | --- |
| GNU | 默认动态 PIE。依赖仅含 C 时使用该目标的 LLVM libunwind。含 C++ 的依赖使用配套 C++ driver、libstdc++ 与 libgcc_s，最终程序只链接一套 Level-I 展开实现。C++ 异常在 C++ 包装层内处理。 |
| musl | 可执行文件默认静态，显式传入 `--link-mode dynamic` 时使用该目标的 musl loader。两种模式均运行 GC 和异常用例。静态产物检查无 `PT_INTERP`／`DT_NEEDED`，动态产物在配套环境加载。依赖要求 C++ 时沿用现有的 musl 诊断。 |

ARM64 musl 的静态和动态链接都从本目标的基础库包取 LLVM compiler-rt builtins。在 Bookworm 上，系统 `musl-gcc` 使用的 glibc 版 libgcc 通过私有符号 `__getauxval` 选择原子指令实现，musl 的共享 libc 导出的是 `getauxval`。用目标 musl 头文件构建的 compiler-rt 可以为两种链接模式提供这些辅助函数。[编译器辅助库修复](https://github.com/scoop-lang/scoop/pull/247)

**基础库准备。** 按现有 Cone 依赖图扩展至全部适用的非图形库。为两种 libc 增加 OpenSSL 的 `linux-aarch64` 配方，按目标重建 Rust 文本后端、nghttp2、压缩库、PCRE2 及其余实际依赖。让 Python 准备脚本从目标目录选择配方。用各目标的头和库构建 `scoop.platform`、`scoop.os.linux`、`scoop.asyncio`、TLS 等真实消费者。将语言规范 §12.2.1 的 Linux GNU C++ 范围同步到 ARM64。§12.2.2 已能表达这两个目标的源文件条件。[依赖准备与基础库划分](../../scripts/package_toolchain.py) · [OpenSSL 配方](../../scripts/build_openssl.py) · [语言规范](../specs/SCOOP-SPEC.md)

OpenSSL、nghttp2 等脚本的 `--target` 选项取自宿主目录中的 `distribution_targets`。在两套配方接入时就将 GNU、musl 一起列入 ARM64 GNU 宿主的该字段，同批运行两个目标的依赖准备。阶段三的发行任务再消费这份清单，生成实际可安装的包。[Python 目标目录](../../scripts/target_catalog.py) · [nghttp2 构建入口](../../scripts/build_nghttp2.py)

**按系统页归还和保护内存。** Immix 普通 block 为 32 KiB，当前 full GC 要求系统页大小整除 block，遇到 64 KiB 页会终止。保留 32 KiB block、128 字节 line 和现有对象头，修改运行时规范 §3.1、§3.7 中的页归还与 stress 保护合同。[整页归还](../../runtime/src/gc/memory_return.c) · [块保护](../../runtime/src/gc/block_release.c) · [region 创建](../../runtime/src/gc/regions.c)

- 普通 full GC 在 64 KiB 页上按系统页遍历普通 region。一页覆盖相邻两个 block，仅当整页均无存活 line、包含待归还空间且不含 region owner、未被隔离保护时，才合并范围交给 `madvise`。同页任一 block 含存活对象就保留整页。沿用现有标记结果、失败重试和按实际字节计数的规则。4 KiB 和 16 KiB 页沿用 block 内归还路径。
- 在带 stress 能力的 runtime 中，64 KiB 页上的普通 region 单独保留整个 owner 首页。其余每页只启用一个 32 KiB block，另一半保留、不进入分配空闲链。这样，旧副本全部迁出且无 pin 后可以立即对整页设置 `PROT_NONE`，不会保护另一个活动 block。大对象继续按整份页对齐 mapping 处理。额外空间仅计入此 stress 布局的保留空间。

**阶段验收。** 两个 target 分别运行基础 Cone 的 release 测试，覆盖平台 I/O、编码、网络与 TLS。在允许 io_uring 的 Linux 环境运行其正常路径。外部 fixture 检查 GNU C++、musl 两种链接模式、native DSO 的线程局部存储和跨 libc 产物不相容诊断。[Linux DSO 用例](../../tests/fixtures/driver/linux/native-dso/fixture.toml)

在采用 64 KiB 页的 ARM64 内核上验证同页两块一活一死、两块全空、owner 首页及 moving stress 后旧页保护。在 4 KiB 和 16 KiB 页的内核上回归 block 内的页归还和保护。用 `getconf PAGESIZE` 记录每次实际运行的页大小。仓库 LLD 22.1.8 生成的最小静态 ELF 的 `PT_LOAD` 对齐值为 `0x10000`。链接验收使用实际 driver 的输出，在 64 KiB 页内核启动 GNU 动态、musl 静态与动态程序，并加载配套 DSO。[页归还用例](../../runtime/tests/page_discard_test.c)

## 阶段三　发行、桌面与开发工具

**发行与安装。** 在 ARM64 Linux runner 原生构建宿主命令、配套 LLVM／LLDB／Python，并按 GNU 宿主的目标清单打包资源。把 release workflow 的 x86 UAPI、musl include 路径改为所选机器的 multiarch 目录。以仓库已有的 Ubuntu 26.04.1、glibc 2.43、musl 1.2.5 为候选发行基线，在对应 ARM64 环境验证后写入发行说明。同步独立 `scoop-install` 的 Linux ARM64 宿主识别及构建矩阵。[发行基线](../../scripts/release.toml) · [发行 workflow](../../.github/workflows/release.yml) · [scoop-install](https://github.com/scoop-lang/scoop-install)

| 新增组件 | 安装用途 |
| --- | --- |
| `bin-aarch64-unknown-linux-gnu` | ARM64 GNU 宿主运行的工具，与共用的 `sources` 及 GNU 基础库构成默认安装 |
| `lib-base-aarch64-unknown-linux-gnu`、`lib-base-aarch64-unknown-linux-musl` | 各自的预编译非图形库、runtime 索引与归档、unwind 和 native 库 |
| `lib-gui-aarch64-unknown-linux-gnu` | GNU 桌面 Cones、ARM64 Skia 和所需 GTK／托盘链接资源，按需追加 |

**桌面与应用包。** 让 GTK 和托盘准备脚本接收实际 GNU 目标，从匹配的 pkg-config 目录取 headers 和库。Skia 的 Linux 配方已从目录选择 CPU，使用 `target_cpu="arm64"` 构建并验证。运行一个使用 GTK4／libadwaita、Skia 绘制的窗口，在 Wayland 和 X11 各完成首次绘制、事件派发和关闭。运行现有托盘 Cone 测试，验证图标注册与菜单回调。应用打包按所选 target 检查 ELF machine、系统 loader 与依赖，deb 使用 `arm64`，RPM 使用 `aarch64`，AppImage 选择配套 ARM64 runtime。[GTK 准备](../../scripts/prepare_gtk.py) · [托盘准备](../../scripts/prepare_tray.py) · [托盘测试](../../sysroot/src/scoop.tray/test-linux/protocol.scoop) · [Skia 构建](../../scripts/build_skia.py) · [应用打包](../../tooling/scoop-bundle/src/linux/)

**调试与最终验收。** 按实现规范 §2.21、§2.22 沿用 DWARF 5、build-id、独立 `.debug` 与 `.gnu_debuglink`。现有 LLDB 静态 TLS 补丁按 AMD64 的线程指针布局取址。为 ARM64 静态 musl 按该 ABI 的线程指针、`PT_TLS` 对齐与符号偏移实现取址。用两个线程同一个 TLS 变量的不同值验证，在 GNU 动态程序上验证源码断点、回溯及异常源码位置。[调试合同](../debugging/IMPLEMENTATION.md) · [LLDB 补丁](../../scripts/patches/lldb-scoop-source-names.patch)

发行候选从安装器默认安装起步，依次运行 GNU CLI、追加 musl 后运行静态 CLI、追加 GNU GUI 后启动窗口。用 `scoop bundle` 生成 deb、RPM 和 AppImage，核对包内架构并从部署布局启动程序。最后使用准备好的 sysroot 在 ARM64 Linux 运行以下公共验收，普通行为用 release，debug/release 组合及 GC stress 用于对应的正确性专测。Rust workspace 测试和正式 CLI fixture 各自保留。[fixture runner](../../tests/fixture_runner/README.md) · [构建准备说明](../../scripts/README.md)

```sh
export SCOOP_SYSROOT="$PWD/sysroot"
cargo test --workspace
python3 -m unittest discover -s tests/fixture_runner/tests
python3 tests/run_fixtures.py --all --target aarch64-unknown-linux-gnu
python3 tests/run_fixtures.py --all --target aarch64-unknown-linux-musl
```

届时同步 README、安装说明、CLI 编程材料与实际可用的目标示例。验收记录保留宿主、目标、libc、内核页大小、运行过的场景及剩余限制，原始构建与测试日志放在仓库外。[编程材料更新流程](../scoop-programming/UPDATE-WORKFLOW.md)
