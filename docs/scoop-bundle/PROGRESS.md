# `scoop-bundle` 实施记录

范围遵循 [设计](DESIGN.md)：首期为 app、deb、rpm、AppImage、NSIS、MSI；Flatpak/MSIX 后续单独实施。原始日志、上游源码调研和临时验证工程放在仓库外。

## 2026-11-08：公共接口

- Cone `[bundle]` 保留为工具 metadata；PM 成员协议版本 6 交付原始表、来源 span 与展开的发布 metadata。普通编译不解释打包配置。
- build JSON 提供实际 canonical target；`scoop.platform.env.executablePath()` 使用三平台原生接口返回主程序真实位置，资源定位不依赖 cwd 或 argv[0]。
- `cargo fmt --all`、完整 workspace clippy 与相关 manifest/protocol/PM 的 198 项 Rust 测试通过。
- macOS 的实际 Scoop 平台测试运行 5 项，全部通过；原生路径探针覆盖直接启动、符号链接、其他 cwd 和不同 argv[0]。
- 实际 Scoop CLI 构建验证 JSON target，并确认只改 bundle 显示名称后没有编译子进程调用；源码、编译器和输出始终保留。
- 本轮临时验证工程已清理；Linux/Windows 原生实现等待各平台阶段验证。打包器后端尚未交付，真实 Developer ID 签名尚未验收。

## 2026-11-08：公共打包器与 macOS

- 独立 Rust 命令完成内嵌/引用/显式配置、PM/build 交接、已有程序入口、目标条件、统一文件清单、更新复用与按格式暂存发布。target facts 直接使用仓库同一静态目录。
- macOS 完成 Info.plist、PNG/ICNS 图标、framework 链接保留、实际传递 dylib 部署、加载路径修正和由内到外签名。全部修改只发生在副本。
- 独立 Cargo 格式化、clippy 与 6 项公共测试、1 项实际 macOS CLI 集成测试通过。集成覆盖传递原生依赖、空格/中文路径、资源更新、目录新增文件、无变化不再调用签名工具，以及签名失败保留旧包。
- 通过开发 sysroot 的真实 `scoop bundle` 运行资源示例；从 `/tmp` 启动读取资源成功。本机 Developer ID 专项验证通过，签名包含可信时间戳及 hardened runtime；该记录不代表完成公证。
- 维护构建入口和完整工具链发行已接入新命令，依赖许可收集覆盖其独立 Cargo workspace。定向发行脚本测试通过，其他宿主专属检查留待相应平台。
- 本轮临时 C/framework 工程与签名配置已清理；签名样包和必要构建缓存保留在仓库外供后续验收。Linux/Windows 格式继续实施。

## 2026-11-08：Linux deb、rpm 与 AppImage

- 三种格式共用固定目录、desktop/图标、文件映射和 ELF 依赖部署；deb 调用 dpkg-deb，rpm 使用 rpm 0.30.2，AppImage 显式提供 appimagetool 与 type-2 runtime。因 ELF 部署和 GUI 验收共用，AppImage 与 Linux 大项一起完成。
- 部署保留原有 SONAME，必要时生成相对链接，只修改副本的 RUNPATH/依赖路径。实际 GUI 验收发现并修复了改变 SONAME 导致 GLib 重复加载的问题；定向测试覆盖重命名私有库后链接依赖与 dlopen 使用同一实例。
- 本地与 Linux 格式化、clippy 通过；8 项公共/格式单测与实际 Linux CLI 集成测试通过。集成覆盖传递依赖、空格/中文路径、目录资源、三个格式复用、deb/rpm 安装/升级/卸载与普通 FUSE 启动。
- nuc12 独立检出中的真实 Scoop 平台测试 5 项通过。资源和窗口两个 Scoop 示例均完成三格式生成、从 `/tmp` 读取资源；窗口示例在 Xvfb/X11 与 D-Bus 会话中显示窗口并正常退出。运行时未使用开发工具链的 LD_LIBRARY_PATH。
- 验收宿主为 Ubuntu 26.04.1 x86_64、glibc 2.43、GTK 4.22.4、libadwaita 1.9.1；RPM 安装器为 6.0.1，AppImage 工具为 2026-11-06 的 continuous build 370。这不是较早发行版兼容性声明，发布者须在选定的最低系统基线上构建。
- 安装验收使用临时目录和独立包数据库，完成后自动清理。GUI 包依赖由宿主提供，临时数据库安装绕过依赖求解；RPM 包并未在 Fedora 系统包数据库上验收。样包、日志与后续发行仍需的缓存/工具保留在仓库外。

## 2026-11-08：Windows NSIS 与 MSI

- 完成两种格式的目录、图标、资源与 PE 依赖部署。导入、延迟加载和导出转发依赖均递归处理；VC runtime 显式选择系统提供或随应用部署，API-set/UCRT 保持系统依赖。
- NSIS 支持同格式升级和卸载；MSI 使用稳定 UpgradeCode/component 身份，支持安装、修复、升级与卸载。切换格式或安装范围须先卸载，用户新建的数据文件保留。
- Windows 11 Pro 10.0.26300 上通过格式化、clippy、公共/格式单测及真实安装生命周期测试。使用 NSIS 3.13、WiX 5.0.2 与同版本 UI 扩展；测试包含传递/延迟/转发 DLL、Unicode metadata/路径、特殊文件名、资源更新、缓存复用与卸载后保留用户文件。
- 实际打包时发现并修复了 NSIS 预处理字符串展开、WiX 路径前缀、MSI 摘要代码页及 CAB 中间目录问题。WiX 5 的 TEMP 仍需由系统 ANSI 代码页表示；完整产品 metadata、输入与产物支持 Unicode。未代为接受 WiX 6/7 的 OSMF 协议。
- 当前 Windows 工具链上的 `scoop.platform` 环境测试 8 项通过。真实 Scoop 资源与窗口示例均完成 NSIS/MSI 生成、安装、从其他 cwd 读取资源、卸载及复用；窗口包在当前用户交互桌面中实际产生绘制事件并正常退出。SSH 非交互会话不能提供该窗口库要求的 DPI 环境，GUI 验收使用临时桌面任务。
- GUI 示例通过正常 RC 输入嵌入 Per-Monitor V2 manifest，并选择 GUI subsystem。打包器不修改主程序的 PE 资源。PM 增加独立的诊断格式选项，使打包器能读取 JSON 成员数据并显示可读的清单错误；原 JSON 诊断默认行为保留。
- Authenticode 的 payload、NSIS 卸载器和最终安装器签名顺序已经实现，并提供可选真实证书测试入口。本轮没有可用的受信任代码签名证书，尚未完成该专项的正向验收；首期 Windows 样包为未签名包。

## 2026-11-08：macOS 多架构输入与窗口示例

- 多架构主程序、framework 和传递 dylib 在部署副本中先用 `lipo` 提取所选架构，再修正加载路径并签名。其他架构的依赖不进入本次包，源文件完整保留。
- 格式化、clippy 与真实 macOS 集成测试通过。测试实际生成 arm64/x86_64 双架构文件，检查三类部署文件均只保留目标架构，同时覆盖运行、签名、更新复用及失败保留旧包。
- macOS 27.2 上通过 LaunchServices 启动真实 Scoop `.app`，完成窗口创建、资源读取、两秒事件循环和正常退出，并通过 `codesign --verify --deep --strict`。
- 当前桌面将测试窗口报告为可见但遮挡，绘制事件可能延后。示例改为窗口创建成功后打印资源，避免把正常遮挡误判为打包失败；显式请求焦点与重绘，仍保留真实窗口和事件循环。
- 两种 Darwin 架构及 Linux GNU/musl 系统库已完成发行准备。musl 使用维护文档约定的独立 UAPI 头目录和编译器包装脚本，成功构建 liburing；没有把 glibc 头加入 musl 搜索路径。

## 2026-11-08：完整工具链发行与包内验收

- 发行版本为 `0.0.0-bundle.20261108`，实现基线为 `ca56c85d40f51f05fbf6377e91e95ffde45d0dae`。使用 `scripts/package_toolchain.py` 和本轮重新准备的目标系统库组装；匹配的 LLVM/LLDB SDK、Python 与既有构建缓存按维护入口复用。
- macOS 生成 8 个组件归档，覆盖 arm64 宿主工具及 arm64/x86_64 目标库。合并安装到带空格的独立目录后，五个主命令版本、bundle/example 分发、随包 LLDB/Python 与 formatter 检查通过。
- macOS 包内工具完成 arm64 资源/窗口应用的构建和 app 打包；资源读取、LaunchServices 启动、签名验证与不变输入复用通过。x86_64 资源应用另外完成交叉构建、签名及 Rosetta 运行。
- Linux 生成 7 个组件归档，覆盖 x86_64 GNU 宿主/目标及 musl 基础库。独立安装后的五个主命令、bundle/example、随包 LLDB/Python 与 formatter 检查通过。
- Linux 包内工具为资源/窗口示例各生成 deb、rpm、AppImage，并完成安装、实际运行、资源读取、卸载和复用；musl 资源应用另完成 AppImage 构建与普通 FUSE 启动。运行时清除开发工具链的动态库环境变量，GUI 与安装数据库范围沿用上述 Linux 验收条件。
- Windows 复用安装资源时漏带了维护布局下的 `rust-std` 许可目录，首次目标库组装按普通 I/O 失败。已从匹配的 Rust 1.99.1 补齐三个目标的许可，并在维护说明中明确安装布局与准备布局的区别；后续沿用原组装函数，保留本轮已经成功的主工具、LLVM 与源码归档。
- Windows 生成 10 个组件归档，覆盖 MSVC UCRT、MinGW MSVCRT、MinGW UCRT 三种目标。九个运行组件合并安装到带空格的独立目录后，五个主命令版本、bundle/example 分发、随包 LLDB/Python 3.14.9 与 formatter 检查通过；运行使用安装目录和系统目录组成的 PATH。
- Windows 包内工具为 MSVC 资源示例生成 NSIS/MSI，并为两个 MinGW 目标各生成 NSIS；三种目标都完成实际安装、从其他 cwd 读取资源、卸载和不变输入复用。
- Windows 包内工具生成的 MSVC 窗口示例在交互桌面完成 NSIS/MSI 安装、窗口创建、资源读取、正常退出、卸载和复用。临时桌面任务已删除，测试安装均已卸载。
- 三宿主共生成 25 个完整工具链组件归档；包内工具另外生成并运行 macOS 3 个 app、Linux 7 个应用包、Windows 6 个安装器。最终产物、日志和必要构建缓存保留在各宿主仓库外的工作目录；临时恢复脚本和已结束的验证工程已清理。本轮 Windows 安装器未签名，真实证书专项限制见上述 Windows 记录。
