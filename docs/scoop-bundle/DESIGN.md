# `scoop-bundle` 应用打包设计

状态：首期六种格式已实现；阶段、验证范围与限制见 [PROGRESS.md](PROGRESS.md)。日期：2026-11-08。

用户决定：通过外部子命令为 Scoop 可执行程序生成应用包和安装包；各种格式都能附加文件、目录和资源，macOS 支持附加 framework 及 codesign。首期交付 `.app`、deb、rpm、AppImage、NSIS、MSI；**Flatpak 和 MSIX 放到后续阶段，不作为首期完成条件。**

依据：[语言规范 §12.2](../specs/SCOOP-SPEC.md#122-cone-identity与conetoml)、[运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)、[实现规范 §2.7～2.8](../specs/SCOOP-IMPL-SPEC.md#27-scoop-与-single-cone-scoopc)、[包管理规范](../specs/SCOOP-PACKAGE-SPEC.md)与[路线图](../ROADMAP.md)。首期 CLI、配置和接口已实现，并按第 11 节同步对应权威规范；实际工具版本、运行环境与专项验证限制以实施记录为准。

## 1. 首期目标与职责

`scoop bundle` 将已经链接完成的程序、应用 metadata、实际运行依赖与附加文件组装为本地交付产物。项目模式可先调用现有 `scoop build`，也支持显式传入已构建程序。

| 宿主 | 首期格式 | 必须完成的能力 |
| --- | --- | --- |
| macOS | `app` | `.app` 布局、Info.plist、图标、资源、framework/dylib 部署、codesign |
| Linux | `deb` | Debian metadata、文件布局、系统依赖声明、桌面入口、安装/升级/卸载 |
| Linux | `rpm` | RPM metadata、文件清单、系统依赖声明、桌面入口、安装/升级/卸载 |
| Linux | `appimage` | AppDir、AppRun、桌面入口、私有依赖及资源、可搬迁运行 |
| Windows | `nsis` | exe 安装器、安装范围、快捷方式、附加文件、升级/卸载、可选签名 |
| Windows | `msi` | WiX/MSI、稳定升级身份、文件 component、安装/修复/升级/卸载、可选签名 |

一次调用处理一个应用，可以生成该宿主的多种格式。首期主入口为 executable；`native-app` 动态库可以作为应用的附加原生库，但不将一个 DLL 或 `.slib` 自动转换为应用入口。

应用包所需内容由配置和实际运行依赖决定。语言依赖求解、源码编译、runtime 链接仍属于现有 PM、driver 和 linker；打包器不读取 HIR/MIR/LIR，不重新检查语言语义或 GC 合同。源码 registry 的 `scoop pm package`、工具链/sysroot 的发行组装继续保持各自职责。

首期不加入自动更新服务、商店上传、通用安装脚本 DSL 或打包插件系统。macOS 公证/装订、DMG/PKG、Universal 合并等扩展单独列在第 12 节。

## 2. 工具组织与现有接口

实际宿主程序安装为 `<SYSROOT>/commands/scoop-bundle`，Windows 为 `scoop-bundle.exe`。用户入口为 macOS/Linux 的 `scoop bundle` 与 Windows 的 `sco bundle`，复用已有 argv、cwd、环境、标准流和退出状态转发。

工具按 [scoop-example](../../tooling/scoop-example/Cargo.toml) 的方式放在独立的 `tooling/scoop-bundle/` Rust package 中，拥有自身 Cargo.lock 和维护构建入口，不依赖 LLVM 或编译 stage 实现。内部按配置、构建调用、文件部署、各平台后端划分普通模块；六种固定格式不需要动态插件接口。

| 现有能力 | 复用方式与必要补充 |
| --- | --- |
| 外部子命令分发 | 直接复用 `commands/` 查找；不将 bundle 改成 driver 内置命令 |
| `scoop pm members --json` | 复用根发现、成员选择和 workspace 继承；补充已展开的发布 metadata、所选成员的 bundle 配置及声明位置 |
| `scoop build --message-format json` | 消费成功记录中的 `output`、`root.coordinate`、`profile`；补充实际 canonical `target`，不猜测输出路径 |
| `[windows].subsystem` / `[windows].rc` | 继续控制 exe 的 subsystem 与内嵌 PE 资源；安装包 metadata 不覆盖这些编译输入 |
| 工具链发行脚本 | 构建并安装外部命令，将其依赖许可纳入现有收集过程 |

PM 识别 `[bundle]` 工具表或外部配置引用，保留原始 TOML 值及诊断来源；字段含义只由 bundle 解析器解释。普通 build/check 不读取外部 bundle 文件、资源、证书或平台打包工具。bundle 表不进入 `ConeManifestSemantic`、`.slib`、语言身份或编译请求。

成员接口提供已展开的名称、版本、description、license、repository，打包器不再解释 workspace 继承或依赖规则。调用时使用 PM 的 `--diagnostic-format human`，成功 stdout 仍为成员 JSON，失败 stderr 可直接展示清单位置及原因，不把 CBOR 字节数组打印给用户。原始 manifest 文本仍只服务诊断。上述两处 JSON 扩展按各自现有格式规则同步生产者与消费者，不另建构建收据或完整编译协议。

项目模式的主流程为：

```text
选择根与配置 → 调用 scoop build → 取得最终程序
→ 判断打包输入是否更新 → 部署文件与动态依赖
→ 生成平台 metadata / 安装包 → 按配置签名 → 发布结果
```

## 3. 命令、输入和输出

```sh
# Build one Cone and create a macOS app.
scoop bundle . --format app

# Select one workspace application and generate Linux packages.
scoop bundle -p com.example:editor --format deb --format rpm --format appimage

# Package an existing binary without invoking build or PM.
scoop bundle --executable ./dist/editor --target aarch64-apple-darwin \
  --config packaging/Bundle.toml --format app

# Windows keeps the existing host command name.
sco bundle . --format nsis --format msi
```

| 参数 | 首期规则 |
| --- | --- |
| `[root-input]`、`-p group:name` | 项目模式按 PM 的本地选择规则得到恰好一个 executable Cone；多根要求明确选择，不隐式挑第一个 |
| `--executable path` | 与 root-input、`-p` 互斥；要求 `--config` 和 `--target`，metadata 必须自足，不向上寻找 Cone 或调用编译器 |
| `--config path` | 显式选择本次完整 bundle 配置；覆盖内嵌/引用的选择，不与它们深度合并 |
| `--format name` | 可重复；省略时取当前平台的 `formats`；无配置时默认 macOS `app`、Linux `deb`、Windows `nsis` |
| `--profile debug\|release` | 项目模式默认 release，传给 build；已有程序模式不接收 profile，由输出目录的 `prebuilt` 分区区分 |
| `--target` | 项目模式传给 build，省略时以成功结果中的实际 target 为准；已有程序模式必需 |
| `--target-dir` | 项目模式复用 workspace target 根并传给 build；已有程序模式默认 cwd 下的 target |
| `--output-dir` | 替换默认的 bundle 产物根，不改变 build 输出 |
| `--scoop`、`--package-manager`、`--sysroot` | 项目模式可显式选择配套工具与 sysroot；已有程序模式不需要这些参数 |
| `--locked`、`--offline`、`--frozen`、`-j` | 项目模式按已有 build 合同传递；不自行解释 lock 或获取依赖 |
| `--force` | 忽略 bundle 结果复用，重新打包/签名；仍允许 build 复用编译缓存 |
| `--message-format human\|json` | 默认 human；JSON 输出本次格式、target、主程序、最终路径、生成/复用状态及错误 |

特殊 native 构建选项可先通过正常 build 生成程序，再使用 `--executable`；首期不增加任意 shell hook 或另一套构建参数 DSL。

项目模式默认使用 bundle 所在安装根的 `bin/scoop`（Windows `bin/sco.exe`）及配套 PM。外部转发只传递原环境，不能假设父进程的 `--sysroot` 被加入环境；bundle 根据实际安装位置确定默认根，自己收到的显式参数优先，调用子程序时传入最终选择。

现有 build JSON 按行写入 stderr。bundle 消费匹配所选 Cone 的成功记录，并要求子进程成功退出；编译失败时显示原诊断，不继续打包磁盘上旧的可执行文件。bundle 自身的 JSON 结果写 stdout，进度和外部工具日志写 stderr；子进程参数直接按 argv 传递。

默认产物目录为：

```text
target/bundle/<canonical-target>/<profile-or-prebuilt>/<identifier>/<format>/
```

`--output-dir` 替换 `target/bundle` 部分。目录保留完整 target，Windows 的不同 CRT target 不因都叫 x64 而覆盖。包文件名使用该格式的名称、版本和架构规则；`.app` 用显示名称命名。主程序安装名称由 `bundle.name` 决定，Windows 增加 `.exe`。

首期在对应 OS 上打包和签名，不承诺跨 OS 运行 SDK 工具。目标架构遵守现有 Scoop 支持范围及后端能力；macOS 分别生成 arm64/x86_64 包，Windows 为现有 x64 target，Linux 首先验收 GNU x86_64 GUI 和 CLI。musl 的静态 CLI 可按同一布局单独验收，不宣称其具备 GUI 库。

## 4. Metadata 配置

### 4.1 两种承载方式

内嵌配置示例：

```toml
schema = 1

[cone]
group = "com.example"
name = "editor"
version = "1.2.3"
kind = "executable"
description = "A desktop text editor"
license = "MIT"

[bundle]
schema = 1
identifier = "com.example.editor"
display-name = "Scoop Editor"
icons = ["packaging/icon.png"]

[[bundle.files]]
from = "assets"
to = "assets"
kind = "resource"

[[bundle.files]]
from = "vendor/Foo.framework"
to = "Foo.framework"
kind = "framework"
when = { os = "darwin" }

[bundle.macos]
formats = ["app"]

[bundle.linux]
formats = ["deb", "rpm", "appimage"]

[bundle.windows]
formats = ["nsis", "msi"]
```

也可在 Cone.toml 中只保存引用：

```toml
[bundle]
config = "packaging/Bundle.toml"
```

外部文件只包含 `[bundle]` 及其子表，内容结构与内嵌形式相同，包含 `bundle.schema = 1`。引用形式的表只允许 `config`；外部文件不能再次引用配置。独立文件模式的 `name`、`version`、`identifier` 等所需字段必须显式提供。

选择顺序为显式 `--config`、Cone 的 `bundle.config`、Cone 的内嵌表；显式配置仍可在项目模式继承所选 Cone 的公共 metadata。没有配置时报告缺少 bundle 配置，不在 cwd、父目录或用户目录搜索同名文件。

相对路径以声明所在文件的目录为基准；命令行路径以 cwd 为基准。外部配置位于 `packaging/` 时，项目 assets 可以写 `../assets`。不执行环境变量插值、shell 展开或远程资源下载。版本等标量的 CLI 覆盖只限实际公开的参数，首期不提供通用 key/value 修改器。

### 4.2 公共字段

| 字段 | 含义与默认值 |
| --- | --- |
| `schema` | bundle 配置版本，首期为 1，与 Cone manifest schema 分别解释 |
| `identifier` | 必填、稳定的反向域名应用标识；用于 Bundle ID、资源目录和安装身份，不替代 ConeIdentity |
| `name` | 安装后的程序名，默认 Cone name；须能作为单个文件名，并满足所选格式的包名规则 |
| `display-name` | 用户可见名称，默认 name，允许空格和 Unicode |
| `version` | 精确 SemVer，默认 Cone version；与平台包版本分开保留 |
| `description`、`license`、`repository` | 默认所选 Cone 对应 metadata；各格式对必填项再作具体检查 |
| `homepage`、`publisher` | 可选应用网址与发布者；需要 Manufacturer 的 MSI 必须有 publisher |
| `icons` | 本地 PNG、ICNS、ICO 文件；后端按目标格式选择或转换尺寸，缺少该格式必需图标时明确诊断 |
| `files` | 第 5 节的文件/目录映射 |
| `library-search-paths` | 查找实际运行依赖的显式本地目录，不等同编译器 library-path |
| `macos`、`linux`、`windows` | 平台 metadata、格式选择、原生依赖策略与签名配置 |

`identifier`、安装范围和各格式升级标识应在应用更新中保持稳定。它们是 OS 的应用/包身份，与语言实体 ID 或产物来源认证没有关系。

仅检查所选格式要求的必填字段和工具；全部配置仍须有正确的 TOML 类型、字段名与合法目标条件。未选中平台的文件不检查存在性，未选中格式不发现其打包工具。首期不公开 Flatpak/MSIX 字段和 format 值。

### 4.3 版本映射

稳定的 `major.minor.patch` 可映射到各格式的默认版本。macOS 的 `short-version`/`build-version`、deb 的 `version`/`revision`、rpm 的 `version`/`release`、MSI 的 `product-version` 允许分别显式声明，并按原生格式检查。

pre-release/build metadata 无法无损映射到某格式时，要求提供该格式的版本，不能静默去掉后缀、截断数值或改变升级顺序。MSI 使用其三段数值及范围限制，macOS 使用对应 bundle 字段的规则；NSIS 的 `file-version` 为四段数字版本，稳定 SemVer 默认补零为第四段，用户可见版本仍保留原 SemVer。首期不实现一套跨所有包管理器的版本比较器。

## 5. 附加文件与安装布局

每项 `files` 包含必需的 `from`、`to`、`kind`，以及可选 `when`。`when` 复用现有目标条件的字段和值，包括 `os = "darwin"`；按程序 target 选择，不能按打包进程的 ISA 错选原生库。

| kind | `.app` 中的目标基准 | Linux payload/AppDir 中的目标基准 | Windows 安装目录中的目标基准 |
| --- | --- | --- | --- |
| `resource` | `Contents/Resources/` | `usr/share/<identifier>/` | `resources/` |
| `executable` | `Contents/MacOS/` | `usr/lib/<identifier>/bin/` | 安装根 |
| `library` | `Contents/Frameworks/` | `usr/lib/<identifier>/lib/` | 安装根 |
| `framework` | `Contents/Frameworks/` | 不适用 | 不适用 |
| `file` | `.app` 根 | payload/AppDir 根 | 安装根 |

主程序与辅助 executable 使用同一可执行目录：Linux 为 `usr/lib/<identifier>/bin/<name>`，deb/rpm 另在 `usr/bin/<name>` 放置相对链接；AppImage 的 AppRun 直接启动内部主程序。这使 Linux 三种格式共用资源与私有库相对布局。

具体复制规则：

- `from` 可为文件或目录，支持普通绝对路径和相对路径；首期不解释 glob。目录内容复制到 `to` 指定的目录，不额外附加源目录名。示例将 assets 的内容放入逻辑资源目录下的 assets。
- `to` 是非空、相对于上表基准的路径；不允许绝对路径或词法越出该基准。它只决定包内布局，不限制输入必须来自 Cone 根。
- 文件、目录和 symlink 按格式需要保留结构与权限，framework 必须保留 `Versions/Current` 等链接。复制目录不递归跟随 symlink；不审计输入 owner、inode 或来源。目标格式无法表达选中链接时报告具体错误，不能静默遗漏。
- 主程序、生成的 Info.plist/desktop/安装 metadata 与附加文件共享一个目标清单。重复目标、文件/目录冲突和目标平台上的名称冲突在写入前报告，不用声明顺序隐式覆盖。
- `file` 用于普通格式特有文件；原生代码仍按实际类型进入依赖处理和签名。安装布局必须满足该平台规则，不能靠放进 Resources 跳过代码处理。
- 许可证文本、字体、图片、模型、翻译等都使用同一文件映射，不为每种资源建立注册表。

## 6. 运行依赖与资源访问

### 6.1 依赖部署

已有 Scoop runtime 与语言依赖已经正常链接进最终程序；bundle 处理的是机器程序实际加载的原生动态依赖和显式文件。初次生成或输入变化时，使用成熟机器格式 reader/平台工具读取架构、依赖名称与加载路径，不重新消费 `.slib`。

显式映射的 library/framework 优先提供私有依赖；其余私有依赖从 `library-search-paths` 和程序实际声明的加载路径定位。按实际依赖递归收集，遇到同名不同文件、错误架构、缺少必需库时明确诊断。系统库按平台部署规则保留为系统依赖；不把整个 sysroot、开发 SDK 或 PATH 目录复制进应用。

平台策略分别为：

- **macOS**：系统 framework 和系统 dylib 由系统提供；选定第三方 framework/dylib 复制到 `Contents/Frameworks`。在副本上调整 `@rpath`、`@loader_path`、`@executable_path` 及 install name，覆盖内部库之间的引用，再进行签名。`.tbd` 只是链接输入，不能当作运行库打包。
- **deb/rpm**：配置列出的私有库随包部署，其他依赖由显式 `depends`/`requires` 声明。ELF SONAME 不是发行版包名，不自行猜测 GTK 等依赖的跨发行版映射。保留系统搜索能力，将私有 RUNPATH 改为实际相对位置。
- **AppImage**：根据明确的最低系统基线部署非基线依赖。使用 linuxdeploy/appimagetool 等成熟能力处理库和模块，并保留显式文件声明以覆盖 `dlopen`、图像 loader、schema 等无法由普通 ELF imports 推导的输入。
- **Windows**：按 PE imports 部署实际非系统 DLL及其传递依赖，系统 DLL/API-set 由 Windows 提供。MSVC、LLVM-MinGW 的 CRT/运行库按各自目标选择；不从开发机偶然可见的同名 DLL 替换已选依赖。是否分发 VC runtime 必须显式配置。

当前 Linux GUI 使用 GTK 4.18 以上和 libadwaita 1.4 以上，且依赖对应 SONAME；包的发行版/runtime 基线必须满足真实二进制要求。AppImage 不能降低已构建程序的 glibc 符号版本要求。现有 linker 可能留下开发目录的 rpath，bundle 只在本次暂存副本上调整。

### 6.2 资源定位

资源路径不能依赖 cwd、PWD、`argv[0]` 或安装时的开发机绝对路径。首期固定上表布局，配套示例从实际 executable 路径定位资源：macOS 为其目录的 `../Resources`；Linux 为 `../../../share/<identifier>`；Windows 为其目录的 `resources`。

为完成这一真实用例，在 `scoop.platform.env` 提供最小的 `executablePath(): Path`：通过各 OS 原生接口取得实际主程序绝对路径，Linux/macOS 解析 `/usr/bin` 启动链接到真实程序的位置，Windows 保留原生路径表示；失败沿既有 PlatformException 报告。其含义是进程主程序，不是调用者所在动态库，不读取 bundle metadata、不改变 runtime ABI。

实施该 API 前补充平台库规范。应用可按所属平台的固定布局组合 Path；首期不增加资源查找 DSL、隐式环境变量注入或运行期资源清单。安装资源按只读内容使用，用户配置与可写数据使用用户目录。

## 7. macOS `.app` 与 codesign

```text
Scoop Editor.app/
  Contents/
    Info.plist
    MacOS/editor
    Resources/assets/...
    Resources/<icon>.icns
    Frameworks/Foo.framework/...
```

Info.plist 至少生成 CFBundleIdentifier、CFBundleExecutable、CFBundleName、CFBundleDisplayName、CFBundlePackageType、版本、图标和最低系统版本。`bundle.macos.minimum-system-version` 默认取实际主程序与附加代码的最高部署要求，显式值不得更低。可用 `bundle.macos.info-plist` 提供额外 plist 键；身份、入口等生成字段发生不一致时报告冲突。

一次打包对应一个明确 target。主程序、framework 或 dylib 为多架构 Mach-O 时，只根据所选 slice 收集依赖和最低系统版本，部署后先使用系统 `lipo` 提取该架构，再修改加载路径和签名；源文件保留全部架构。首期不合并 Universal 应用，也不将尚未部署依赖的其他 slice 留在包中。

```toml
[bundle.macos]
formats = ["app"]
short-version = "1.2.3"
build-version = "123"
info-plist = "packaging/Info.extra.plist"

[bundle.macos.signing]
mode = "identity"
identity = "Developer ID Application: Example Company (TEAMID)"
entitlements = "packaging/App.entitlements"
hardened-runtime = true
timestamp = true
```

签名模式为 `ad-hoc`、`identity`、`none`。默认 ad-hoc 便于本地运行，identity 模式必须显式选择证书；none 只生成供后续处理的未签包，不宣称它满足本地执行或分发要求。identity 模式默认开启 hardened runtime 和时间戳；ad-hoc 不请求可信时间戳。需要 identity 却找不到证书或签名失败时直接失败，不能降级成 ad-hoc。

所有文件部署、加载路径修改和可选符号处理完成后，先签署最内层原生代码，再签 framework/辅助 bundle，最后签外层 app。应用 entitlements 只应用于主应用；辅助程序需要独立 entitlements 时，在对应 executable 文件项的 `macos.entitlements` 声明，不把主应用权限复制到每个 dylib。

新生成结果使用 codesign 验证签名与嵌套代码，成功后发布。已有签名的第三方代码在需要重写或统一签名时按同一流程处理，不修改源文件。证书引用和 entitlements 路径可进配置，私钥/密码不写入 metadata、日志或更新记录。

Developer ID 签名与公证是独立操作；首期完成本节 codesign，notarytool/stapler 在后续扩展中实现。

## 8. Linux 三种格式

```toml
[bundle.linux]
formats = ["deb", "rpm", "appimage"]
categories = ["Utility", "TextEditor"]
terminal = false

[bundle.linux.deb]
maintainer = "Example Team <dev@example.com>"
section = "utils"
revision = "1"
depends = ["libgtk-4-1 (>= 4.18)", "libadwaita-1-0 (>= 1.4)"]

[bundle.linux.rpm]
release = "1"
requires = ["gtk4 >= 4.18", "libadwaita >= 1.4"]
```

示例依赖名只说明字段结构，实施验收按所选发行版的实际包数据库确认，不将其固化为所有 Linux 目标的通用规则。

三种格式共用主程序、资源、私有库、图标和 desktop metadata。desktop 的文件名以 identifier 为基准；deb/rpm 的 Exec 指向安装后的程序入口，AppImage 使用 AppDir 中的对应入口。生成器负责空格、Unicode 和 desktop Exec 的转义，不能把未经转义的配置拼入 shell。

deb 生成 control、payload、权限和普通包升级信息；rpm 生成对应 header、文件清单和依赖。架构名称从程序目标转换，例如 x86_64 对应 Debian amd64、RPM x86_64。license、description 等按格式要求提供。首期采用成熟后端和标准安装行为，不开放任意 post-install shell 脚本。

AppImage 使用第 5 节的 AppDir 布局，AppRun 保留用户参数、环境、退出码和信号行为，并执行真实主程序。显式确定最低发行系统和依赖基线；配置中的 `bundle.linux.appimage.system-libraries` 仅列出由目标基线提供的额外 SONAME，核心系统库遵循后端的正常基线规则。新增未打包依赖必须在该基线上实际验证。

GUI 依赖的模块与数据用真实窗口和资源读取验收。`--appimage-extract-and-run` 可用于无 FUSE 的环境，但不能以此代替支持环境中的普通 AppImage 启动验证。AppImage 不进行系统安装/卸载，不附加常驻桌面集成服务。

## 9. Windows NSIS/MSI

```toml
[bundle.windows]
formats = ["nsis", "msi"]
install-mode = "per-user"
# Required when PE imports use the VC runtime: system or app-local.
vc-runtime = "system"

[bundle.windows.nsis]
start-menu-shortcut = true
desktop-shortcut = false
# file-version = "1.2.3.0"

[bundle.windows.msi]
upgrade-code = "58723036-0bd5-4df5-9d42-8df91d07f606"
product-version = "1.2.3"
start-menu-shortcut = true
desktop-shortcut = false

[bundle.windows.signing]
certificate-thumbprint = "0123456789ABCDEF0123456789ABCDEF01234567"
timestamp-url = "https://timestamp.example.com"
# certificate-store = "current-user" # or local-machine
```

上述 UUID、证书和时间戳地址仅为格式示例；应用应使用自己的稳定 UpgradeCode 和实际签名配置。

两种格式使用相同的 payload 和公共 publisher/display-name/version；默认 per-user，安装到当前用户的 `LocalAppData/Programs/<identifier>`，per-machine 默认使用 `Program Files/<identifier>` 并按 Windows 正常权限流程安装。安装器允许用户选择目录，资源和 DLL 继续相对程序定位。NSIS 的卸载登记以稳定应用标识区分产品，安装器支持标准静默参数和版本升级。

PE 主程序与附加代码验证目标架构；普通 imports、delay imports 和导出转发引用中的私有 DLL 一并处理。自动部署的 DLL 与使用它的程序位于同一加载目录，不从 PATH 随机选择运行库。VC runtime 的 `system` 表示由部署环境提供；`app-local` 从显式文件或 library-search-paths 中取得可分发 DLL。Windows 自带 UCRT/API-set 继续由系统提供，应用自行动态加载的插件仍须显式声明。

首期升级针对同一种格式和安装范围；NSIS/MSI 互相切换或改变安装范围时，要求先正常卸载已有安装，再安装新包，不覆盖另一安装器拥有的同目录文件。两种格式共用应用 metadata，但不伪造对方的安装记录。

MSI 通过 WiX 定义产品、目录、component、快捷方式和升级。UpgradeCode 是必填的稳定 UUID，不随版本变化；ProductCode 和 PackageCode 按 WiX/MSI 规则产生，component 身份按稳定安装路径维护。支持正常安装、修复、升级和卸载；默认拒绝降级。用户数据不放在 payload 中，卸载只处理安装器拥有的内容。

MSI 使用 WiX CLI 与同版本 UI 扩展提供目录选择；扩展 DLL 通过 `bundle.tools.wix-ui-extension` 指定，或从 PATH 中选择 `WixToolset.UI.wixext.dll`。首期验收选择 MS-RL 许可的 WiX 5.0.2；WiX 6/7 引入的 OSMF 条款由使用者自行处理，打包器不代为接受协议。安装不要求接受工具生成的示例许可协议。NSIS/MSI 使用普通应用注册表值记录安装范围、目录及安装器类型，阻止未卸载就切换格式/范围；这些值只服务安装和升级，不用于证明来源。

MSI 数据库使用 UTF-8 保存产品名称、发布者和文件名。Windows Installer 摘要只允许 ANSI 代码页，因此摘要使用 ASCII 应用标识、版本和生成工具名称；安装界面与程序列表继续显示完整的产品 metadata。

WiX 5 的原生 CAB 接口使用系统 ANSI 代码页表示压缩输出目录。后端把 WiX 中间文件放在独立的系统临时目录，输入文件和最终安装包仍允许 Unicode 路径；使用 WiX 5 时，`TEMP` 须能由系统 ANSI 代码页表示。临时目录随本次调用清理，不进入更新缓存。

可选 Authenticode 签名使用 Windows 证书存储引用和时间戳配置：先签署本次需要签名的 exe/DLL payload，再生成安装器并签署最终 NSIS/MSI。后端负责 NSIS uninstaller 等内部可执行文件的正确签名顺序，不能只签外层安装包后宣称内部程序已签名。显式要求签名时失败不发布新包。

`[windows].rc` 仍是编译输入。本期 bundle 中的图标/版本用于安装包，不自动重写主程序的 PE 资源或生成 RC；需要 exe 内嵌信息的项目通过已有 RC 配置提供。安装包资源变化仅重打包，RC 及其依赖变化由正常 build 更新。

## 10. 更新判断、失败与诊断

项目模式先让正常 build 按自身规则取得最新程序，随后判断 bundle 结果是否可复用。打包更新记录只保存配置选择、实际输入路径/文件状态、目录成员、工具选择、签名选项和所需输出位置，用于判断更新；不证明产物来源或重放生产过程。

- 相同输入且所需输出根存在时直接返回结果。检查必须发生在读取程序/资源内容、复制暂存文件、动态依赖扫描、工具版本探测或签名之前。
- 更新输入覆盖主程序、所选配置、所选附加文件/目录成员、此前实际解析的私有库、图标、plist、entitlements 及工具文件状态。目录增删和依赖搜索候选变化也使相关记录更新；未选平台的资源不进入本次输入。
- 不读取命中 app/安装包的内容、哈希或签名，不恢复外部修改导致的缓存损坏。缓存命中不查询钥匙串或时间戳服务；同一证书引用完成轮换后，用户可用 `--force` 重新签名。
- 实际需要生成时在独立暂存目录处理，绝不修改 build 输出、源 framework 或共享依赖。按格式独立发布，失败不覆盖该格式原有成功结果，也不把旧包报告成本次生成成功。
- 多格式调用中，已成功发布的格式保留；后续格式失败时整体返回非零并逐项报告。并发写同一目标使用普通文件锁，成功后更新记录，不建立跨格式事务或任务服务。
- 新产物的格式和签名验证只在必要边界执行。原生工具的实际错误、退出码和相关文件路径直接保留，不增加来源授权、签名收据链或资源计费模型。

配置诊断定位到具体表/字段和来源文件；文件复制错误包含 from/to；原生库错误包含发起引用的程序/库及缺失名称；外部工具失败包含步骤、退出状态及必要 stderr。任何生成失败都不安装软件或自动修改用户系统配置。

## 11. 实施顺序与验证

### 11.1 依赖选择和规范同步

已核查 `tauri-bundler` 2.10.4 的 library API 与源码。其格式覆盖足够，但 Linux 默认把主程序和资源分别部署到 `usr/bin` 与 `usr/lib/<product>`，RPM 的自定义文件遍历不能直接表达本设计的链接和目录清单，Windows 后端还包含工具下载和特定 WiX 版本假设。为这些行为建立第二套布局或修改上游实现，成本高于直接调用格式工具，因此首期不依赖该 crate。

固定目录、文件映射和实际依赖部署由本工具负责；机器格式读取用 `goblin`，plist 用 `plist`，图标转换用 `image`，RPM 用成熟的 `rpm` crate。deb 调用 `dpkg-deb`，AppImage 调用 `appimagetool`，Windows 分别生成 NSIS/WiX 输入后调用 `makensis`/`wix`；macOS 使用 `install_name_tool`、`codesign` 与 `iconutil`。这些模块直接服务六种实际格式，不另建插件层。

2026-11-08 确认上述 Rust 库近期仍有维护，许可为 MIT 或 MIT/Apache-2.0；独立工具声明 Rust 1.90 并锁定 Cargo.lock，不调整主 compiler workspace 的最低版本。实际运行依赖和格式工具版本记录在平台验收中。

只为所选格式发现工具，允许在 `bundle.tools` 中按工具名称显式给出路径。普通打包不静默安装 SDK 或下载执行辅助工具；维护环境显式准备系统工具，并通过配置选择其位置。AppImage 的维护工具需要单独准备 `appimagetool` 与 type-2 runtime（`bundle.tools.appimage-runtime`），调用时总是显式传入 runtime 文件，避免上游默认的在线获取行为。AppImage 自动部署实际 ELF 依赖；无法由 imports 推导的模块和数据继续使用显式文件映射。

实施前同步：

| 文档/接口 | 修改内容 |
| --- | --- |
| 语言规范 §12.2、包管理规范 manifest/成员接口 | 允许 bundle 工具 metadata 与外部引用，说明其不参与语言编译语义 |
| 实现规范 §2.7 | 外部命令发行、build JSON 的 target 与独立 bundle 消费；明确应用打包与工具链发行职责 |
| 平台库规范 | 第 6.2 节实际 executable 路径 API、错误和平台语义 |
| 成员/构建 JSON 及相应消费者 | 增加上述实际消费字段，按现有协议版本规则同步；不改变 `.slib` 或 runtime ABI |
| 发行维护脚本与 README | 独立工具构建、安装、所需后端工具及许可说明 |

### 11.2 首期阶段

| 阶段 | 交付与完成条件 |
| --- | --- |
| A：公共配置与 macOS | 两种配置、项目/已有程序入口、成员/构建交接、文件映射、资源定位、app/framework 部署、ad-hoc 与 identity 签名；真实 app 从非项目 cwd 启动并读取资源 |
| B：Linux deb/rpm | 共用布局、系统/私有依赖、desktop/图标、安装/升级/卸载；在选定发行版实际运行 GUI 与 CLI |
| C：Windows NSIS/MSI | 安装范围、附加 DLL/资源、快捷方式、签名及升级/修复/卸载；验证实际目标 CRT 依赖 |
| D：AppImage 与首期收尾 | 依赖与最低系统基线、可搬迁运行；六种格式的更新复用、失败不发布、工具分发与使用文档 |

每阶段先完成对应规范、格式化/lint，再运行必要测试。仅已完成的格式进入实现的可用列表；未实现阶段不放置空后端或返回成功的占位分支。阶段 D 完成即可验收首期，不等待 Flatpak/MSIX。

### 11.3 验收范围

| 层次 | 代表性验证 |
| --- | --- |
| 配置单测 | 内嵌/外部等价、选择互斥、workspace metadata、相对路径、字段诊断、版本与目标条件 |
| 文件与部署单测 | 目录复制、framework 链接、权限、名称冲突、生成文件冲突、依赖缺失与错误架构 |
| CLI fixture | 真正调用外部命令；构建失败停止；已有程序模式不调用编译器；空格/Unicode 路径与 argv |
| 平台产物验收 | app 内容及签名；deb/rpm 查询/安装；NSIS/MSI 安装/升级/卸载，MSI 修复；AppImage 普通启动 |
| 功能组合 | 主程序读取文件/目录资源，加载附加原生库；framework 传递依赖；Linux GUI 的模块和数据；从不同 cwd 启动 |
| 更新与失败 | 无变化直接复用；资源变化只重打包；源码/RC 变化正常构建；输入缺失、工具/签名失败不覆盖旧成功结果；同输出并发协调 |

普通资源读取、Path 行为放所属 Cone 的 scoop.test；必须观察包内容、命令/签名、安装和外部进程的行为使用 fixture 或定向平台验收。普通用例使用 release，不把 GC stress、全 IR 快照或全量工具链重建加入打包验收。

测试保留源码、工具和原构建产物，以真实调用和输出验证复用及部署，不通过移走输入要求后续步骤成功。需要安装系统包的验收使用专用 VM/测试环境，不在普通单测中修改开发机。macOS ad-hoc 作为常规验证，Developer ID 使用配置真实签名身份的专项环境；不能将没有证书的跳过记录为签名已通过。

原始日志和逐次报告放仓库外；后续验收文档只记录覆盖平台、工具版本、结果概要、失败闭合及限制。

## 12. 后续阶段

### 12.1 Flatpak

在首期完成后单独设计和实施：app-id/branch、runtime 与版本、SDK、`/app` 布局、manifest、桌面集成、sandbox 权限与 portal 需求，以及导出仓库/单文件 `.flatpak`。先确定满足 Scoop GUI 的 GNOME/其他 runtime 基线，再验证实际加载、文件访问和窗口行为。

单文件 `.flatpak` 不等于包含全部 runtime 的独立可执行文件；导出/安装流程必须明确运行时获取方式。该阶段复用公共 metadata、文件声明和程序选择，不反向要求首期增加 runtime/sandbox 的占位配置。Flathub 上架属于后续明确的发布需求。

### 12.2 MSIX

在首期完成后单独设计和实施：Identity Name、Publisher、四段版本、目标架构/最低 Windows 版本、桌面应用入口、Assets/capabilities、MakeAppx 产物及 SignTool 签名；验证证书 Subject 与 Publisher、稳定更新身份、安装目录和应用数据目录的行为。

该阶段复用 Windows payload、公共 metadata 和资源输入，但依据 MSIX 规则生成新包，不能把 MSI 压缩后视为 MSIX。Microsoft Store 上架和发布凭据不进入首期。

### 12.3 其他独立扩展

macOS `notarytool`/`stapler`、DMG/PKG、Universal app，以及文件/URL 关联等更深入桌面集成，在有具体应用需求时追加相应字段、实现和验收。它们不改变本设计首期六种格式的完成标准。

## 13. 调研依据

以下是 2026-11-08 调研使用的上游资料；库和工具版本在实施时重新确认。

- [tauri-bundler 支持的格式](https://docs.rs/tauri-bundler/2.10.4/tauri_bundler/bundle/enum.PackageType.html)、[SettingsBuilder](https://docs.rs/tauri-bundler/2.10.4/tauri_bundler/bundle/struct.SettingsBuilder.html)、[发布 metadata](https://crates.io/api/v1/crates/tauri-bundler)。
- [Apple 嵌套代码与签名规则](https://developer.apple.com/library/archive/technotes/tn2206/_index.html)、[公证流程](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)。
- [AppImage 部署与兼容性建议](https://docs.appimage.org/reference/best-practices.html)。
- [WiX 命令行工具](https://docs.firegiant.com/wix/tools/wixexe/)。
- 后续阶段：[Flatpak 构建](https://docs.flatpak.org/en/latest/first-build.html)、[单文件 bundle](https://docs.flatpak.org/en/latest/single-file-bundles.html)、[MSIX 手工打包](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion)、[签名证书](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing)。
