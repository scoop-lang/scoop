# Scoop 工具链安装与版本管理调研

首次调研：2026-11-10。更新：2026-11-11。本文件保留首次调查时的包结构、体积和设计建议；当前已实现功能与验收结果见[实施记录](IMPLEMENTATION.md)。发行合同以独立仓库的 [scoop-install 设计](https://github.com/scoop-lang/scoop-install/blob/codex/scoop-install/docs/DESIGN.md)和本仓库的[发行包与下载目录设计](PACKAGING-DESIGN.md)为准。

安装程序定名为 `scoop-install`，在独立 Git 仓库维护，管理 Scoop 发行版本、所选 target 与可选组件。现有二进制组件及安装根布局可以复用；源码发行拆为 `sources` 与 `vendor`，默认保留前者供 LSP 和源码阅读使用，后者按需安装。主要新增工作是发布索引、版本目录、命令代理与安装状态；生产侧增加独立的外部依赖下载目录。真正支持项目间切换版本时，还需要接入构建缓存隔离和编辑器的工具选择。

## 1. 已核实的发布现状

本次检查了语言、运行时、实现规范和路线图，发行组装、post-install、官网两种安装脚本与索引发布脚本，以及真实归档。以下发布事实与体积统计均为 2026-11-10 的调查结果；当时线上入口 [latest.txt](https://release.scoop-lang.org/latest.txt) 指向：

- release：`2026-11-10`。
- version：`0.0.0-dev.20261110`。
- 三个宿主 bin 包中的 revision 均为 `d5e40a4e1aef0ec178d0c55ea8328540e95fc0ef`。

本地参考包位于 `/Volumes/N/homes/chenxu/scoop-dist/2026-11-10/`。索引中 21 个安装归档的线上 Content-Length 均与同名本地文件大小相符；这只用于确认本次体积统计的对象，不代表逐字节内容比对。另读取了三个 bin 的 VERSION，并检查了 macOS bin、sources、base 归档的成员与目录组成。当时工作区 HEAD 为 `c283ee875c`，与发布包 revision 有区别；下面分别区分发布事实与拟议改动。

### 组件与宿主

归档统一命名为 `scoop-<version>-<component>.tar.xz`。

| 组件 | 内容与安装关系 |
| --- | --- |
| `bin-<host>` | Scoop 主命令、编译器、linker、LSP、PM、外部命令、LLVM／LLDB／Python 运行资源、VSIX、安装说明；每宿主一份 |
| `lib-base-<target>` | 非 GUI 系统 `.slib`、native 链接库、runtime 及索引；每目标一份 |
| `lib-gui-<target>` | GUI `.slib`、Skia 等链接输入；依赖同版本、同目标的 base |
| `native-<target>` | 两种 MinGW CRT 的 SDK 数据；需要 Windows bin 提供公共工具与头文件 |
| `sources` | 内置库源码、示例、公开 FFI 头，以及大量第三方维护输入 |
| `llvm-sources` | LLVM 维护源码；现行安装索引已排除 |

普通组件直接包含安装根内容，没有额外版本目录；合并解压后是 `bin/`、`commands/`、`lib/<target>/`、`include/`、`src/`、`libexec/`、`share/` 等目录。系统库已经预编译，post-install 只生成离线 API 文档。

| 实际发布的宿主 bin | 默认目标 | 可追加的发行目标 |
| --- | --- | --- |
| `aarch64-apple-darwin` | `aarch64-apple-darwin` | `x86_64-apple-darwin` |
| `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-musl` |
| `x86_64-pc-windows-gnullvm` | `x86_64-pc-windows-msvc-ucrt` | `x86_64-w64-windows-llvm-mingw-ucrt`、`x86_64-w64-windows-llvm-mingw-msvcrt` |

共七种 target，musl 没有 GUI 包。Windows 的宿主 triple 与默认应用 target 不同，安装器必须使用明确映射。目标目录中还登记了 Intel macOS、Linux musl 宿主的事实，但当前线上没有它们的 bin 包；可安装列表应以实际 release 为准。

`sources` 在逻辑设计上是全局组件，当前 R2 目录却在三个平台下各放了一份，大小也略有不同。新索引可以显式引用选定的源码归档；后续由发布流程只产出／发布一份，客户端无需按同名或摘要自行推断共享关系。

### 现有安装脚本的行为与体积

官网 [install.sh](https://scoop-lang.org/install.sh) 和 [install.ps1](https://scoop-lang.org/install.ps1) 都只按平台筛选索引，然后下载该平台的全部条目。`--release`／`--version` 选择日期目录；传到 post-install 的 `--target` 只选择生成文档的目标，不限制下载。

两种脚本均将版本平铺安装到 `~/.scoop`，更新前删除旧发行目录，没有多版本并存、已安装组件查询或单独 target 增删。仓库说明描述了手动选择组件的能力，但当前自动脚本没有实现这个选择界面。

以下均为压缩归档下载量，单位 MiB，不含本地生成文档；“默认目标”组合保留现有 sources，不安装 GUI 和其他目标：

| 平台 | 当前脚本全部下载 | bin + sources + 默认目标 base | 减少 |
| --- | ---: | ---: | ---: |
| macOS | 1,152.91 | 455.54 | 60.5% |
| Linux | 933.45 | 486.70 | 47.9% |
| Windows | 1,694.85 | 480.91 | 71.6% |

Windows 此处是默认 MSVC 目标；选择任一 MinGW 目标时另加对应约 4.6 MiB 的 native 包。仅增加下载选择就能得到上述收益；这些数字仍包含拆分前的完整 sources，尚未计入省去 vendor 的进一步节省。

## 2. 从 rustup 借用的模型

[rustup toolchain](https://rust-lang.github.io/rustup/concepts/toolchains.html) 表示一套编译器安装，可选择版本／渠道、日期和宿主；[component](https://rust-lang.github.io/rustup/concepts/components.html) 是其中的安装单元；target 的库资源也是组件。其 [profile](https://rust-lang.github.io/rustup/concepts/profiles.html) 只是组件预设，[override](https://rust-lang.github.io/rustup/overrides.html) 决定命令实际使用哪套工具链。

Scoop 可以采用相同分层：

- **toolchain = 发行版本 + 宿主**。例如当前版本的 Windows GNU-LLVM 宿主工具。
- **target = 生成程序的完整目标**。例如 MSVC UCRT 或 MinGW UCRT；两者共用同一套 Windows 宿主命令。
- **component = 实际下载／增删的资源包**。base、GUI、native、sources、vendor 等。
- **profile = 默认组件组合**。它与 `scoop build --profile debug/release` 无关。

当前只需要支持已发布的精确开发版本及 `latest` 别名。日期 release 可作为版本查找入口；没有实际渠道时不必预设 stable／beta／nightly。`latest` 解析到固定发行版本，项目固定版本保持不变。

`target add` 只增加资源，不改变程序的编译目标；编译时继续使用 `scoop build --target <target>`。这一点也符合 [rustup 的交叉编译模型](https://rust-lang.github.io/rustup/cross-compilation.html)。

## 3. 建议的安装器、目录和命令

### 独立安装管理程序

安装程序放在独立仓库，本机为 `/Volumes/Data/home/chenxu/repos/scoop-install-src`，不加入 Scoop 仓库的 `tooling/` 或 Cargo workspace。它是一个不依赖 LLVM、编译 stage 或 PM 实现的 Rust 程序。POSIX shell／PowerShell 入口只负责选宿主、下载并启动管理程序；下载选择、解包、更新和安装状态由同一份 Rust 实现处理。

HTTP、TOML、tar、临时目录、文件锁等优先沿用仓库已有成熟依赖的选型，例如 `ureq`、`serde`／`toml`、`tar`、`tempfile`、`fs4`；XZ 解码库在实施时确认维护状态和各宿主构建条件。安装管理程序自身应能独立启动，无需先安装 Scoop、LLVM 或 LLDB 的 Python。

简单扩展两种脚本也能先解决选择下载，但版本代理、项目固定、Windows 更新和组件增删会持续扩大双份实现。独立管理程序更适合作为最终实现；无需把 rustup 整体移植或引入通用包依赖求解器。

### 安装布局

```text
~/.scoop/
  bin/
    scoop-install
    scoop                 # Windows: sco.exe
    scoopc
    scoop-link
    scoop-lsp
    scoop-pm
    ...
  settings.toml
  toolchains/
    0.0.0-dev.20261110-aarch64-apple-darwin/
      bin/
      commands/
      lib/<target>/
      include/
      src/
      libexec/
      share/
      VERSION
      ...
  packages/
```

顶层 `bin` 是命令代理，各 toolchain 内部完整复用现有安装根布局。当前 `configured_sysroot_root()` 从真实执行文件位置定位根，配套 `scoopc` 等也从真实文件的相邻路径选择，因此代理只要启动选定版本中的实际命令，大部分资源发现逻辑即可沿用。

建议的选择优先级是命令行 `+<version>`、环境变量 `SCOOP_TOOLCHAIN`、最近的项目 `scoop-toolchain.toml`、全局默认。第一版可先实现 `default` 和 `run`，再接项目文件及 `+version`。

代理应将一次选择传给子进程，保持配套工具一致，并保留参数、标准输入输出、退出码和信号行为。现有显式 `--sysroot`／`SCOOP_SYSROOT` 覆盖规则继续有效；不在用户 shell 中写入某一具体版本的全局 `SCOOP_SYSROOT`。Windows 使用原生 `.exe` 代理，保持主命令 `sco.exe`。

下面是调研时拟议的命令接口；已实现接口见 [scoop-install 使用说明](https://github.com/scoop-lang/scoop-install/blob/codex/scoop-install/README.md)：

```sh
scoop-install toolchain install 0.0.0-dev.20261110
scoop-install toolchain install 0.0.0-dev.20261110 --target x86_64-apple-darwin
scoop-install target add x86_64-apple-darwin --toolchain 0.0.0-dev.20261110
scoop-install component add gui --target x86_64-apple-darwin --toolchain 0.0.0-dev.20261110
scoop-install default 0.0.0-dev.20261110
scoop-install run 0.0.0-dev.20261110 scoop --version
scoop-install toolchain list
scoop-install target list --installed
```

示例中的 Darwin target 适用于当前 macOS 宿主。Windows 选择 MinGW 时，安装器自动补上同目标 native 包，用户无需知道归档间的依赖细节。

不指定 target 时只装宿主默认目标；显式 target 列表取代默认列表，从而可以只装 MinGW 或只装 Darwin x86_64 库。向已有工具链追加 target 时，从该工具链固定版本的索引取包，不从 latest 取包。

默认开发组合包含该版本的 `sources`，提供内置 Cone 源码导航；`vendor` 独立按需安装，不是 sources 或 LSP 的依赖。两种源码组件各按 release 全局发布一份，跨宿主与 target 共用其安装内容；实际源码选择仍遵守所选 target 的 Cone 清单。

## 4. 发布索引需要补充什么

现行 `format 1` 只有 release、version、平台和路径。它不能直接描述组件角色、宿主默认目标、依赖、下载大小或已发布目标集合。bin 内的 `share/scoop/toolchain.toml` 又必须下载解包后才能读到，而且记录的是发行提供的目标，不是用户实际安装的目标。

建议让发行组装输出结构化 TOML／JSON，发布侧合并为每个 release 的 manifest；官网和 GitHub 发布流程使用同一份组件事实。最小字段包括：

| 数据 | 用途 |
| --- | --- |
| schema、release、version、source revision | 选择固定发行版本，显示版本信息 |
| host、default target、该 host 提供的 targets | 选择可运行宿主及真实发行组合 |
| component id、角色、host 或 target | 形成安装计划，无需从文件名拆语义 |
| URL／相对路径、压缩字节数、SHA-256 | 下载、展示体积、下载完成时检查传输结果 |
| 必需组件引用 | 表达 GUI → base、MinGW target → native 等有限关系 |
| 已有 OS／libc／SDK 基线 | 在选择时展示该发行的外部要求 |

索引生成时直接复用 `targets.toml`、发行参数和本次成功的组件清单，避免重新扫描整个 bucket 猜测可安装组件。固定 release 的包上传齐全后发布 manifest，最后更新 latest；同一固定版本对应稳定的一组包，重打包采用新的 release/build 标识。

下载摘要只在下载边界使用，安装清单只记录版本、组件和正常卸载所需的文件归属。不增加编译器启动巡检、产物认证、逐文件内容证明或来源审计；普通构建继续使用既有格式／目标／ABI 检查和缓存规则。

## 5. 安装和更新的行为

新版本先在临时目录完整解包，成功后将目录发布到 `toolchains/<id>`，最后更新默认选择。旧版本保留，安装中断不会先删掉当前可用版本。更新沿用用户已选 targets／components，不自动扩展成该宿主所有包；新版本缺少所选目标时，先明确报告，保持现有选择。

追加组件使用同一工具链的安装锁，先准备所需归档，成功后提交新增文件和安装记录。归档目录可合并，文件归属需要明确；卸载 GUI、native 或整个 target 时只处理所属文件及相应生成文档。现有许可证目录已经多数按组件分组，可沿用。Windows bin 使用归档内硬链接共享 DLL，解包实现必须保持其正常加载路径；不能只检查普通文件的解压。

现有平铺安装需要一次迁移，重点保留 PM 的 `packages/` 和用户自定义外部命令。新安装器不能沿用旧脚本先删除顶层 bin／lib 等目录的更新方式。管理程序自身的 Windows 替换／自更新可单独实施，不必成为第一阶段按 target 安装的前置能力。

post-install 接收安装计划中的实际目标；只安装非默认目标时不能无参数调用它。第一阶段可保留默认生成文档，同时提供跳过文档的安装选项用于 CI。文档失败明确报告，可单独重试；未完成必需安装步骤时不切换默认版本。

## 6. 需要提前处理的集成点

### sources／vendor 拆分（已确定）

2026-11-11 确定按以下职责拆分源码发行：

| 组件 | 内容 | 使用场景 |
| --- | --- | --- |
| `sources` | Scoop 自己的 Cone 代码，包括 `Cone.toml`、`.scoop`、自有 C／C++／Rust 胶水、头文件、配套资源和示例 | 默认开发安装、LSP 源码关联、文档生成、源码阅读与维护 |
| `vendor` | 上游源码归档或源码树、固定版本说明、许可，以及构建时应用到上游的补丁 | 显式重建或维护第三方依赖时安装 |

`sources` 保留现有 `src/<Cone>/` 结构，包括 Cone 内的 `native/` 胶水。独立放置的自有胶水同样归 sources，例如 `native/text_backend/` 中的 Scoop 文本后端 Rust 适配代码；它依赖的 encoding_rs、ICU4X 等上游源码归 vendor。归属不按实现语言区分，修改上游源码的构建补丁归 vendor。

按同一边界，LLVM 上游源码与 LLVM／LLDB 构建补丁也归 vendor，实施时将现有独立 `llvm-sources` 内容合入 vendor。Scoop 自有 glue 不因调用 LLVM、Skia 或 OpenSSL 而移入 vendor。归档仍以安装根内容为顶层，保持正常源码与补丁的相对布局。

现有 LSP 关联路径支持此划分：对已选的 `lib/<target>/<name>.slib`，从同一 sysroot 的 `src/<name>/Cone.toml` 取得生产源码并建立 SourceIdentity 映射。依赖语义继续来自 `.slib`，sources 提供对应源码及导航位置；关联操作不构建 native 胶水或上游库，因此不要求安装 vendor。拆分保留 Cone 清单、逻辑源码路径和与二进制组件一致的发行版本即可沿用这条路径，见[现有实现](../../compiler/analysis/src/workspace/attached_sources.rs)。

拆分前约 202 MiB 的 sources 同时装了源码和维护材料。2026-11-10 macOS 归档的未压缩逻辑大小为：

- `src/` 约 16.52 MiB。
- `vendor/skia/` 约 129.51 MiB，`vendor/openssl/` 约 50.82 MiB，`vendor/text_backend/` 约 11.33 MiB。
- `include/` 约 0.03 MiB，包含 `scoop_app.h`、`scoop_rt.h`、`scoop_runtime_metadata_v1.h`。

`assemble_sources()` 当前把公开 FFI 头放进 sources；native 编译会使用所选 sysroot 的 `include/`。第一阶段将这些自有头留在默认安装的 sources，拆出 vendor 不会移走它们；同时把已发行 native 库需要随附的许可归入对应二进制组件，使其完整安装不依赖 vendor。

默认组合因此是 bin + 所选目标 base + sources + 必要 native，GUI 按目标追加，vendor 显式选择。若再提供不装 sources 的 minimal，应先把应用 native 编译必需的公开头移入必装组件，再允许省略源码和文档；这不影响默认开发组合继续安装 sources。重建第三方后端仍需 vendor 和相应维护工具。具体压缩节省需要重新打包测量，不能把上述未压缩数字当成下载量。

### 多版本缓存

现有发行说明要求升级后执行 `scoop clean`，显式放到项目 target 外的缓存也要清理；项目缓存不监视编译器版本。当前默认构建缓存路径按 target／profile 划分。

因此“多个目录 + 改 PATH”还不足以承诺透明版本切换。建议把所选工具链的明确发行身份接入现有缓存配置或命名空间，覆盖默认与显式共享缓存；该值来自发行／选择配置，不通过读取、哈希或巡视工具文件取得。接入前仍须清理旧缓存，并明确此限制。实施涉及实现规范 2.7，应先修订相关合同。

### VS Code、LSP 与调试

扩展已经优先查找 `~/.scoop/bin/scoop-lsp` 及其他公开工具，因此可以保留顶层代理入口。项目固定版本还要求客户端按工作区目录启动工具，并在版本配置改变后重启相应语言服务；不能假设正在运行的 LSP 会随全局 default 自动换版本。多根工作区使用不同版本时的服务选择也需明确，宜在项目级版本管理阶段一并处理。

### target 资源与外部 SDK

安装 target 不等于获得任意跨 OS 编译环境。第一阶段支持现有发行宿主／目标矩阵：

- Darwin 仍需要对应的 Xcode／Command Line Tools 和 macOS SDK。
- Linux GNU 仍有系统 libc／开发文件和 linker 要求；musl 需要 musl-gcc 与匹配开发文件，当前没有 musl GUI／C++ 组合。
- Windows MSVC UCRT 仍需要 Build Tools／Windows SDK，或显式 native sysroot；MinGW 使用随包公共资源与所选 native 包。
- Linux GUI 仍需要声明版本的 GTK／libadwaita 等桌面运行库。

将 Windows target 数据安装到其他宿主不会自动提供可运行的 Windows 工具。既有跨宿主编译继续使用该宿主可运行的工具与显式外部 SDK；不把扩大交叉工具链分发矩阵作为本次安装器的附带工作。

## 7. 实施顺序与验收

建议按三个可交付阶段推进：

1. **版本与 target 按需安装**：先修订发行／安装合同，完成 sources／vendor 拆分；生成明确的 release manifest；实现独立管理程序、版本目录、default／run、安装查询与 target／GUI 增删，按需处理文档。沿用当前二进制组件，默认安装 sources，vendor 按需。已有项目切换版本时暂遵守 clean 要求。
2. **项目版本切换完整闭环**：接入工具链缓存命名空间、`+version`、项目配置及编辑器选择，验证配套编译器、LSP、PM、debugger 一致使用选定版本。
3. **提供不带源码的精简组合**：将应用 native 编译必需的公开头移入必装组件，提供可省略 sources 的 minimal；default 继续带 sources。再决定是否需要管理程序自更新与本地工具链 link。

涉及的主要位置：

| 位置 | 工作 |
| --- | --- |
| `docs/specs/SCOOP-IMPL-SPEC.md` 第 1、2.7 节，发行设计与说明 | 先明确管理程序职责、版本布局、选择规则；缓存与源码可选化随所属阶段同步 |
| `scripts/package_toolchain.py`、`scripts/package_licenses.py`、`.github/workflows/release.yml` | 分别组装 sources／vendor 并调整许可归属；复用外部依赖下载目录；向独立安装器提供完整组件 manifest |
| 独立的 `scoop-install` 仓库 | 新增安装、状态、版本选择、代理实现及安装程序自身的发行 |
| 官网仓库 `tools/publish_release_index.sh`、`install.sh`、`install.ps1` | 发布结构化索引，改为轻量 bootstrap |
| `compiler/scoop/src/cli/paths.rs` 等现有缓存入口 | 工具链缓存配置／命名空间，不引入工具文件巡检 |
| `tooling/vscode-scoop/` | 工作区版本选择与服务生命周期 |

验收围绕真实安装和命令行为：选定目标只请求相应归档；默认组合安装 sources、不请求 vendor，并保留内置库的 LSP 源码导航和文档生成；自有胶水进入 sources，上游源码和构建补丁进入 vendor；MinGW 自动补 native；追加 GUI 保留已有目标；两个版本的代理选择和缓存互不误用；重复安装复用已有组件；下载／解包失败不切换到半成品；三宿主的空格路径、Windows 硬链接、退出码和配套工具可用。源码可选化时增加 native 消费和有／无源码导航的定向覆盖。

这些属于发布与 CLI 专测。测试保持源码、工具和原产物存在，不通过删除或隐藏它们验证安装隔离；普通库 API 测试仍留在既有 Cone 测试中。

## 8. 主要依据

- [现行发行组件设计](DESIGN.md)、[发行说明](../../scripts/distribution/README.md)、[维护脚本说明](../../scripts/README.md)。
- [实现规范](../specs/SCOOP-IMPL-SPEC.md)、[语言规范](../specs/SCOOP-SPEC.md)、[运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)、[路线图](../ROADMAP.md)。
- [发行组装](../../scripts/package_toolchain.py)、[目标目录](../../compiler/target-data/targets.toml)、[资源路径发现](../../compiler/toolchain/src/paths.rs)、[构建缓存位置](../../compiler/scoop/src/cli/paths.rs)。
- 官网仓库：`/Volumes/Data/home/chenxu/repos/scoop-lang.org`；读取了安装脚本和 `tools/publish_release_index.sh`，未执行安装或发布。
- [线上 release 索引](https://release.scoop-lang.org/2026-11-10/index.txt)、[shell 安装入口](https://scoop-lang.org/install.sh)、[PowerShell 安装入口](https://scoop-lang.org/install.ps1)。
- rustup 官方文档：[Toolchains](https://rust-lang.github.io/rustup/concepts/toolchains.html)、[Components](https://rust-lang.github.io/rustup/concepts/components.html)、[Profiles](https://rust-lang.github.io/rustup/concepts/profiles.html)、[Overrides](https://rust-lang.github.io/rustup/overrides.html)、[Cross-compilation](https://rust-lang.github.io/rustup/cross-compilation.html)。
