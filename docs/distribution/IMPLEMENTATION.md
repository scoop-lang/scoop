# scoop-install 实施记录

实现分支均为 `codex/scoop-install`。安装器在独立公开仓库 [scoop-lang/scoop-install](https://github.com/scoop-lang/scoop-install/tree/codex/scoop-install)，官网 [scoop-lang/scoop-lang.org](https://github.com/scoop-lang/scoop-lang.org/tree/codex/scoop-install) 维护 bootstrap。各实现阶段均按提交、推送、合并 `origin/main` 的顺序完成。

| 大项 | 当前结果 |
| --- | --- |
| Scoop sources／vendor 拆分与共享下载目录 | `58985050a4`；维护脚本本机 65 项通过，11 项按平台条件跳过；追加的 MinGW 下载复用验证通过 |
| 明确的组件 manifest 与 CI 合并 | 已提交 `b98cc983d0`；三宿主七目标映射与缺失组件／发行冲突测试通过 |
| 独立安装器、更新、组件增删、代理与项目选择 | 安装器构建提交 `2f95a36`；macOS、Linux 各 10 项端到端通过，Windows 9 项通过、1 项跨宿主模拟跳过 |
| 缓存发行身份、共享外部命令、编辑器集成 | `513f212ccf`；Node 24 项通过；真实 CLI 覆盖默认、环境变量、显式参数三种缓存入口及共享命令；VS Code 扩展宿主的语义功能、运行操作和多工作区隔离通过 |
| 官网 bootstrap 与发布索引 | 官网提交 `fec15e1`、`569d6c9`；本机 7 项通过、Windows 5 项通过，其余按平台跳过；TLS 修正后 Windows 3 项 bootstrap 回归再次通过，完整官网与 Jekyll 指南构建通过 |
| 源码组件独立生成文档 | `78bc0e7f2b`；组装时把可选 vendor 文档链接转换为该 revision 的仓库 URL，4 项组件打包测试及无 vendor 的真实文档生成通过 |
| Windows 深层文档缓存 | `ad6e8a2c72`；文档输入与 warning 记录的原子发布使用 Windows 长路径；macOS、Windows 各 2 项文档回归通过 |
| R2 发布与公网安装 | 20 个工具链组件、3 个安装器、校验清单与两个 manifest 已发布；三个宿主从公网默认安装后，编译运行及 LLDB 均通过 |

安装器使用静态 liblzma，Windows 的 MSVC C runtime 也静态链接；最终 Windows 可执行文件只导入系统 DLL，macOS 可执行文件只依赖系统库。自定义 home 加入 PATH 后可直接使用管理器和代理，无需再次设置环境变量。

CLI 验收使用本地 HTTP 服务与真实可执行工具，覆盖请求范围、版本和组件选择、更新失败不激活、归档路径／文件归属、并发安装、参数与退出行为。缓存验证保持原源码、工具和产物存在，以实际调用及输出确认复用。

本次 Scoop 包版本为 `0.0.0-dev.20261111`，源码 revision 为 `ad6e8a2c72cf822700b6f1d1e524dfcab563742f`。安装器版本独立，为 `0.1.0`。

| 平台 | 目标与实际验收 | 归档 |
| --- | --- | --- |
| macOS Apple Silicon | ARM64、x86_64 的编译运行和 GUI CPU 绘图通过；官网引导安装、API 文档、LSP 源码导航和 LLDB 通过 | 7 个 |
| Linux x86_64 | GNU、musl 编译运行和 GNU GUI CPU 绘图通过；官网引导安装、API 文档、LSP 源码导航、LLDB 及真实 vendor 增删通过 | 6 个 |
| Windows x86_64 | MSVC UCRT、LLVM-MinGW UCRT／MSVCRT 的编译运行与 GUI CPU 绘图通过；官网引导安装、API 文档、LSP 源码导航和 LLDB 通过 | 11 个 |

完整安装验收首先显式选择非默认目标，核对只请求 bin、所选 base、sources 和必要 native，没有下载其他目标、GUI 或 vendor；然后增装目标与 GUI 并运行程序。LSP 的 definition 指向所选版本 `src/` 内的核心库，测试时没有安装 vendor。GUI 验收范围为 CPU 绘图、链接和像素输出。

macOS 修复版通过完整复验。Linux 首轮发现复用的 LLDB SDK 链接 Python 3.15，而打包参数选了系统 Python 3.14；改用配套的 3.15 后重打包，通过新的干净安装验证 LLDB 和 musl 程序。复用 `--debugger-prefix` 时，`--debugger-python` 应选择与该 SDK 匹配的 Python ABI。Windows 首轮真实安装发现版本目录中的文档缓存超过旧路径长度限制；修正临时文件与目标文件的 Windows 路径处理后重打三平台包，预编译库及 ABI 保持原有合同。Linux 的 musl 资源准备使用现有的编译器包装脚本提供 Linux UAPI 头；普通应用继续消费预编译资源。

公网验证发现 Windows PowerShell bootstrap 将 TLS 限制为 1.2，而发布域名当前拒绝该协议版本；改用系统默认协商后完成真实下载与安装，未改变证书验证或系统设置。

本次产物位置：

- macOS：`/Volumes/Data/home/chenxu/repos/scoop/tmp/scoop-install/dist/macos/scoop-0.0.0-dev.20261111/`。
- Linux（`nuc12`）：`/home/chenxu/repos/scoop-install-20261111/tmp/dist/linux/scoop-0.0.0-dev.20261111/`。
- Windows（`192.168.2.67`）：`E:/scoop-install-20261111/tmp/dist/windows/scoop-0.0.0-dev.20261111/`。
- 三种宿主的安装器二进制与 `SHA256SUMS`：`/Volumes/Data/home/chenxu/repos/scoop-install-src/dist/0.1.0/`。

各平台目录包含可直接用于本地安装的 `manifest.toml`。跨平台发布时，sources／vendor 采用 macOS 的 shared 片段，与三个 host 片段合并；固定 manifest 与 latest 的生产合同见 [MANIFEST.md](MANIFEST.md)。正式发行产物不覆盖，重打包使用新版本。

已使用 `.envrc` 配置将产物发布到 `scoop-releases` bucket，公开域名为 `release.scoop-lang.org`。20 个组件覆盖三个宿主、七个 target，归档位于 `2026-11-11/{macos,linux,windows}/`；安装器与校验清单位于 `installers/0.1.0/`。所有 24 个数据对象成功上传后，依次发布固定索引与 latest，总计 26 个对象。

- [固定版本索引](https://release.scoop-lang.org/versions/0.0.0-dev.20261111.toml)
- [latest 索引](https://release.scoop-lang.org/latest.toml)
- [安装器 SHA256SUMS](https://release.scoop-lang.org/installers/0.1.0/SHA256SUMS)

公网核对两个索引与本地发布结果相同，所有数据对象的可访问性和长度与清单一致。三个平台随后在独立空目录中通过官网仓库的 bootstrap 读取线上 latest，实际下载默认目标、sources 和宿主命令，运行程序及 LLDB 成功；这轮网络验收使用 `--skip-docs`，文档生成和可选目标／GUI／vendor 已由前述完整安装验收覆盖。

官网安装页与 bootstrap 已在 `codex/scoop-install` 分支更新并推送，指向 R2 的安装器固定版本；主站部署需将该分支合入官网的 `main`。本次完成仓库修改及构建，未触发主站部署。

本机临时内容统一在各仓库 `tmp/`。成品包、下载目录和可复用构建缓存保留；三个平台已完成验收的临时安装目录、传输归档和本轮临时上传 SDK 已清理。原始日志和安装产物不提交 Git。
