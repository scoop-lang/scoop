# 发行组件精简验收

日期：2026-11-09。对应[设计](DESIGN.md)。

本轮归档版本为 `0.0.0-distcleanup.20261109`，编译器构建基线为 `9b2631cb45d0c9b765a4459af8778addaab8faa9`，LLVM／LLDB 为 22.1.8。收尾时按用户要求将最新 `origin/main` 合入工作分支；该次合并不触发重打包，归档继续对应上述 revision。

## 组成与实际使用

- 每个宿主只生成一个 bin，包含 Scoop、外部命令、LLVM／LLDB、Python 和实际动态依赖；不再生成独立的 LLVM 宿主包。LLVM SDK include、CMake exports、llvm-config 和 llc 不在 bin 中。
- base／GUI 归档不含第三方开发头、executable、DLL 或完整 target toolchain。保留预编译 `.slib`、链接输入、四种 runtime 的对象与索引及许可证；Linux GUI 的必要共享库继续提供。
- Windows 的公共 MinGW／libc++ 头随 bin 提供；MSVCRT 与 UCRT 各有一个 native 包，包含配置头、目标链接库、启动对象和 compiler-rt，没有完整 Clang、Python 或其他架构资源。Microsoft SDK／MSVC 继续使用系统开发环境。
- 归档入口按 `--jobs` 并行压缩独立组件，全部成功后才写本次 `packages-<host>.list`。发布流程和官网索引只选择新组件集合。

## 覆盖范围

| 平台 | 环境与目标 | 实际验证 |
| --- | --- | --- |
| macOS | macOS 27.2／AArch64；AArch64 与通过 Rosetta 运行的 x86_64 目标 | 中文及空格路径、五个命令及两个外部命令、base 编译／运行／测试、GUI raster 与 AppKit 隐藏窗口生命周期、C/C++、复制安装目录、两阶段 API 文档、dSYM／DAP／String formatter、正常异常展开和源码栈诊断 |
| Linux | Ubuntu 26.04.1／x86_64，glibc 2.43；GNU 与 musl | 同样的安装路径、命令、base 和目录复制检查；GNU C/C++、musl C；GNU Skia raster 及 Xvfb／D-Bus 下的 GTK 窗口生命周期；两阶段 API 文档、GNU `.debug`／DAP／formatter；GNU 与 musl 的异常展开和源码栈诊断 |
| Windows | Windows 11 Pro 10.0.26300／x64；MSVC UCRT、MinGW MSVCRT、MinGW UCRT | 最终包组件及硬链接解包、宿主命令与 LLDB／Python、三 CRT 的 base 编译／运行／测试／异常展开、base-only API 文档、MSVC Skia raster；候选安装的 native 与调试覆盖见下文 |

实际 loader 输出确认 macOS 与 Linux 的 scoopc 使用安装目录内的 LLVM 动态库。LLDB 的 Python 路径也来自各自安装根，ssl、sqlite3 和 lldb 模块可导入。

Windows 拆分布局已在真实编译中检查：宿主 executable 与 `--sysroot` 分居两个目录；公共头更新使两个 MinGW 目标更新，单一目标配置头更新只影响该 CRT；输入不变时，最终程序与 native 工具探测记录复用。测试修改后恢复原输入，临时 SDK 与缓存已清理。

macOS／Linux 的 post-install 分别在仅安装 base、以及追加 GUI 后运行默认目标；Windows 最终包完成 base-only 文档生成。检查文档入口及库产物复用。实际 DAP 断点与 formatter 使用各宿主的默认目标，其中 Windows 使用候选安装；各目标的编译运行、native 链接及源码栈诊断另行覆盖。

Windows 窗口示例成功链接，但在当前 SSH 会话中创建应用返回 Win32 错误 5（ERROR_ACCESS_DENIED）。按本轮收尾要求停止调查，窗口生命周期没有记为通过。Windows 最终包的两个 MinGW GUI consumer、GUI 文档叠加、整目录复制及 DAP 复验未继续执行；此前候选安装已通过的三 CRT 原生专测、MSVC PDB／DAP／formatter 和三 CRT 栈诊断结果保留。本记录区分候选安装验证与最终归档验证，不将未执行项列为通过。

普通验收使用 release；调试使用显式 full 或 line-tables。必要的 Windows PDB 等既有编译器正确性 fixture 保留自身模式。未为本次包布局验收开启 GC stress。Linux musl C++ 不是已支持能力，对应 fixture 按目标声明跳过。Linux 当前维护 LLVM 未启用 zlib，LLDB 对部分系统库的压缩 DWARF 给出提示；Scoop 自身的源码断点、独立调试文件和 formatter 已通过。

## 自动检查与问题闭合

相关 Python 打包、依赖收集、组件筛选、post-install 和 fixture runner 检查均通过；Rust 已完成格式化、workspace／all-targets Clippy、47 项协议测试及定向 native 更新记录测试。官网仓库的安装与组件索引测试共 20 项通过，后续文案修改又复验安装测试；Windows 指定目标的文档命令已统一为脚本实际接受的 `--target`。

实际消费中修复了 Darwin 依赖重写时已有相对 rpath 的保留、Windows 显式 libc++ 头目录下的多余编译参数，以及 fixture 辅助 C／C++／DLL 工具仍依赖旧 target/bin、GCC runtime 和 target unwind 头的假设。候选安装中，三种 Windows CRT 的六项专测均通过，覆盖原生静态／DLL 数据、重叠导入库、PDB、资源文件、子系统选择和 Unicode 参数。手动安装说明中残留的独立 LLVM 解包命令也已移除。源码选择补充过滤 Finder 元数据；macOS 与 Linux 的源码文件和许可证内容一致，前者直接复用后者的公共 sources 归档。

## 体积

与 `/Volumes/N/homes/chenxu/scoop-dist/2026-11-08` 比较相同宿主和目标组合，不计两边的 sources／llvm-sources：macOS 只计 AArch64，Linux 只计 GNU，Windows 计三个 CRT，且新 Windows 组合包含两个 native 包。单位为 MiB（1,048,576 字节）；文件数据量按 tar 中实际保存的普通文件计，不重复计算硬链接别名。

| 组合 | 参考包压缩合计 | 新包压缩合计 | 参考文件数据量 | 新文件数据量 |
| --- | ---: | ---: | ---: | ---: |
| macOS AArch64 | 504.03 | 510.59 | 962.75 | 886.18 |
| Linux GNU | 547.29 | 547.22 | 1,172.46 | 1,074.78 |
| Windows 三 CRT | 1,715.99 | 1,488.89 | 3,945.03 | 2,485.12 |

Windows 三 CRT 组合的压缩量减少 13.2%，文件数据量减少 37.0%。Windows bin 中的 314 个硬链接别名共享约 557.7 MiB 的内容；两种 MinGW 目标各自的 native 资源约 62.9／62.3 MiB，公共头文件约 103.4 MiB 只随 bin 提供一次。

参考包来自不同 revision，重新生成的 `.slib` 大小也有变化：相同目标的 GUI `.slib` 在 macOS 与 Linux 分别增加 21.09／21.14 MiB。因此总压缩差异不能全部归因于包布局；macOS 的压缩合计略有增加，文件数据量则下降约 8%。本轮验证的是必要文件归属和真实消费；没有通过裁掉已支持功能或改变 `.slib` 格式追求压缩比例。

原始日志、逐包统计和归档保留在仓库外的本轮工作目录。仓库只保留本记录和设计，不提交解包目录或原始报告。
