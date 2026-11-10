# 发行源码拆分与外部依赖下载目录设计

日期：2026-11-11。状态：源码拆包、共享下载目录、结构化发行索引和独立安装器接入已实现；验收结果见[实施记录](IMPLEMENTATION.md)。

本设计落实两项生产侧改动：将源码发行拆为 `sources`／`vendor`，以及允许打包脚本指定并复用外部依赖的下载目录。安装管理程序名称固定为 `scoop-install`，位于独立仓库；其设计见[安装程序文档](https://github.com/scoop-lang/scoop-install/blob/codex/scoop-install/docs/DESIGN.md)。

## 1. 仓库分工与当前问题

Scoop 仓库继续负责依赖维护、编译、目标资源、发行组件和 manifest。独立的 `scoop-install` 消费已发布归档，负责用户机器上的版本与组件选择。上游依赖下载目录属于本仓库维护脚本，不与安装器的组件下载、PM 源码缓存混为一个缓存。

现有实现存在以下具体问题：

- `package_toolchain.py::assemble_sources()` 同时复制自有 Cone、公开头、Rust 胶水和全部 vendor，使普通安装也下载上游维护输入。
- `package_llvm.prepare()` 优先复用仓库 vendor 的固定归档；缺失时放到 `<work-dir>/inputs`。更换工作目录会重复获取同一 LLVM 归档。
- Windows release workflow 将两种 LLVM-MinGW ZIP 下载到 `tmp/llvm-mingw`，解包后删除 ZIP，后续任务无法复用该下载文件。
- workflow 将所选 CPython 版本的 LICENSE 每次下载到工作目录。
- OpenSSL、nghttp2、PCRE2、压缩库、Skia 等主要直接消费已有 vendor 归档；Rust 文本后端消费本地 `crates.tar.gz`，不是每次从网络下载。

因此改动应接入这些真实入口，保持依赖原有构建方式和更新判断，不新增通用资源管理服务。

## 2. sources 与 vendor 的归属

| 组件 | 内容 | 默认开发安装 |
| --- | --- | --- |
| `scoop-<version>-sources.tar.xz` | Scoop 自己的 Cone、清单、`.scoop`、自有 C／C++／Rust 胶水、头文件、配套资源和示例 | 安装 |
| `scoop-<version>-vendor.tar.xz` | 上游源码、固定版本说明、许可和构建时应用到上游的补丁 | 按需 |

两个组件每个 release 各发布一份，内容供各宿主／target 使用。归档以安装根内容为顶层，保持现有正常相对路径。

具体归属：

- `src/<Cone>/` 整体保留自有代码和资源，含 Cone 内的 native 胶水；不会只按 `.scoop` 扩展名筛选。
- `native/text_backend/` 的 Scoop Rust 适配代码和自有头放入 sources；其 encoding_rs、ICU4X 等依赖源码放入 vendor。
- LLVM、Skia、OpenSSL 等上游源码及对上游应用的补丁归 vendor。现有 `llvm-sources` 合入 vendor；包括从 `scripts/patches/` 组装的 LLDB 补丁。
- 第一阶段公开 Scoop FFI 头仍随默认 sources 安装。未来允许省略 sources 时，先将应用编译必需的公开头移入必装组件。
- 二进制库所需随附许可归对应 bin／base／GUI／native 组件；vendor 另带其上游源码许可，普通安装不靠下载 vendor 获得二进制发行的必要通知。

LSP 继续从 `.slib` 获取依赖语义，从 `src/<name>/Cone.toml` 及生产源码建立导航映射。sources 保持与二进制组件一致的发行版本和逻辑路径；该操作不编译 native 或上游库，不要求 vendor。文档生成同样只消费所选库的自有源码和预编译依赖。

组装 sources 时，Markdown 中指向仓库维护文档或可选 `vendor/` 的链接改为该发行 revision 的仓库 URL；指向 sources 内源码和示例的链接继续使用本地路径。默认安装生成 API 文档不要求 vendor 中的维护说明存在。

普通 build／run／test 消费预编译系统库。安装 sources 不隐式请求重建这些库；维护者需要重建上游时，再准备 vendor 及相应工具。

## 3. 下载目录接口

给 `scripts/package_toolchain.py` 增加 `--download-dir <path>`，支持环境变量 `SCOOP_DOWNLOAD_DIR`。解析顺序为显式参数、环境变量、Scoop 仓库的 `tmp/downloads`。相对路径相对于调用 cwd；确定后将同一个绝对路径传给实际下载入口。

```sh
python3 scripts/package_toolchain.py \
  --download-dir /Volumes/Data/scoop-downloads \
  --work-dir /Volumes/Data/scoop-builds/macos \
  --output-dir /Volumes/Data/scoop-dist
```

上例以 macOS 构建为例；其他宿主继续提供自身原有必需参数。独立运行的下载／准备入口采用相同选项与默认规则；只消费显式源码树、不执行获取的底层编译脚本无需新增无用参数。

| 目录／配置 | 内容与生命周期 |
| --- | --- |
| `--download-dir` | 原始下载文件，可供不同 work-dir、Scoop 版本和适用宿主重复使用 |
| `--work-dir`／各 native `--build-dir` | 解包副本、补丁应用、生成文件、对象、SDK 构建与普通构建记录 |
| `--output-dir` | 本次发行组件归档 |
| `--llvm-prefix`、`--debugger-prefix`、`--prepared-sysroot` 等 | 已准备资源，继续按现有语义直接消费 |

下载目录不保存打过补丁的共享源码树或各目标编译结果。已有显式 `--llvm-archive`／`--llvm-source` 等输入仍优先；显式路径失败正常报错，不静默改为下载其他输入。

原归档与展开源码是不同输入。指定 LLVM 源码树或 SDK prefix 可供相应构建阶段使用；如果本次还要制作 vendor 组件，其原始归档仍按实际需要从本地或下载目录取得。

## 4. 文件布局与复用规则

文件按依赖名称、固定版本或上游 revision、实际文件名组织。例如：

```text
<download-dir>/
  llvm-project/22.1.8/llvm-project-22.1.8.src.tar.xz
  llvm-mingw/20260616/llvm-mingw-20260616-ucrt-x86_64.zip
  llvm-mingw/20260616/llvm-mingw-20260616-msvcrt-x86_64.zip
  cpython/<selected-python-version>/LICENSE
```

键来自已有固定版本、commit 和文件名，不包含 Scoop release 或构建工作目录。同一上游版本在多次打包间直接复用；平台相关的原始 SDK 归档保留上游文件名中的 CRT／架构差异。新增版本保存为另一项，不覆盖旧项。

实际入口的处理顺序：

1. 先按现有输入更新信息和结果存在情况判断构建复用；可复用时直接返回，不为填满下载目录而获取或复制输入。
2. 确实需要某个下载输入时，显式本地输入优先；自动选择先检查下载目录对应的完整文件，再使用仓库内已有的固定 vendor 输入。
3. 对自动选择命中的原始下载文件直接使用，不发 HEAD／条件请求，不读取其内容做重复摘要或解包验证，不强制复制到工作目录。
4. 本地输入均不存在且该入口已声明固定下载 URL 时，才下载所需文件。实际需要解包、组装 vendor 或编译时再读取文件内容。

查找及更新判断只使用正常配置和文件元信息；不存在显式必需输入仍按原错误路径处理。不能为了保持成功状态而忽略缺失输入，也不处理外部破坏下载缓存后的自动恢复。

首次下载写入目标文件旁的临时文件，完成后原子替换为最终文件名。网络或写入失败不产生可命中的最终文件。使用单项普通文件锁协调同机并发，取得锁后再次检查结果，使相同输入只下载一次；不同输入可以并行。临时名及锁不是发布凭据。

已有固定下载摘要可在首次下载时随数据流检查；命中后不重算。解包错误在实际解包边界报告，不另行预解包所有下载验证生产过程。

补丁来自当前 Scoop 检出的维护输入，在各工作目录的副本上应用。补丁变化进入该依赖原有更新记录，需要时重新准备／构建；同一个未变化的上游归档继续复用，不重新下载。

## 5. 实际入口的接入范围

| 入口 | 调整 |
| --- | --- |
| `package_llvm.prepare()` | 从独立 download-dir 取得固定 LLVM 归档；展开源码及 `llvm-source.json` 继续放在工作目录 |
| Windows release workflow 的 LLVM-MinGW 下载 | 使用共享下载入口，保留 ZIP；解包到本次工作目录，再将真实 prefix 传给现有 Windows 参数 |
| release workflow 的 CPython LICENSE 下载 | 按所选解释器精确版本复用下载文件，继续通过 `--python-license` 传入打包 |
| OpenSSL、nghttp2、PCRE2、压缩库、Skia 等本地 vendor 消费 | 继续使用已有输入与更新机制；本次不强制复制到 download-dir，也不为不存在的网络步骤增加下载流程 |
| Rust 文本后端 `crates.tar.gz` | 保持与 Cargo.lock 配套的本地维护归档；它不是可按单一上游版本直接下载的文件 |
| Cargo、npm、系统包管理器 | 沿用各自现有缓存与固定输入，不改造成新的 Scoop 下载服务 |

首批共享下载逻辑只有实际需要的路径解析、固定文件获取、临时写入和锁。可置于一个小的 `scripts/download_support.py`，复用 `build_support.build_lock`；不增加插件层、依赖求解器或独立缓存数据库。

Skia 的部分现有归档是维护者按固定 commit 重压缩的 xz 文件，Rust 文本归档由 `cargo vendor` 生成；不能用猜测的 URL 或另一种格式冒充这些现有输入。新增此类网络入口时需要明确实际来源和文件格式，本次继续沿用其完整本地输入。

如果将来有新的实际下载调用，使用相同目录规则接入即可。版本号、平台矩阵和依赖列表继续来自既有 release 配置及具体构建配方，不再复制一套独立事实目录。

## 6. vendor 组件如何消费下载输入

vendor 组装从本次选中的上游输入清单取得归档或源码树，再附上配套补丁与说明，写入既有 `vendor/<dependency>/` 相对布局。LLVM 原归档可以直接来自指定 download-dir。

不能直接把整个 download-dir 打进发行包：其中可能含旧版本、其他平台 SDK、临时文件和其他构建的输入。LLVM-MinGW 二进制 SDK ZIP、CPython 的独立许可下载以及 Cargo／npm 缓存不因位于下载目录就成为 vendor 源码组件；各自仍由实际组件职责选择。

只读维护输入的正常组装不是重复下载或缓存复核。制作新的 vendor 归档时读取、压缩所选源码是本次必要产物生产；不把这项工作提前到每个 target 的 native 缓存命中路径。

## 7. 发行索引及 CI

发布 manifest 分别列出 sources、vendor 和二进制组件，默认组合引用 sources。组件生产清单由本次组装直接生成；旧 `llvm-sources` 停止生成，避免目录通配把退役归档重新发布。

CI 为本次任务明确设置 download-dir，并将其作为可恢复的下载文件集合；与 LLVM／LLDB 已构建 SDK 缓存、Cargo 产物缓存分别配置。恢复出的原始文件按同一固定条目直接复用，不逐项扫描内容。更换 work-dir、构建另一个 target 或同上游版本的新 Scoop release 不应重新获取相同下载。

官网安装入口、发布索引生成和随包说明同步采用 `scoop-install`、sources／vendor 组件名。安装器仓库有自己的构建和发行流程，Scoop workflow 只提供安装器需要的工具链组件与 manifest。

## 8. 实施顺序与验收

先修改实现规范第 1、2.7 节和相关发行说明，明确组件归属、维护下载目录及实际缓存规则，再调整脚本。语言语义、`.slib` 格式、runtime ABI 和 stage 边界不随此设计改变。

实施顺序：

1. 拆出 sources／vendor 组装与许可归属，保留 Cone、胶水和 LSP 源码路径；停发独立 llvm-sources。
2. 接入 `--download-dir` 和环境变量，迁移 LLVM、MinGW ZIP 和 CPython LICENSE 的真实下载入口。
3. 调整 workflow、组件清单与 release manifest，向独立安装程序提供明确组件关系。

验收必须覆盖：

- 默认组合含 sources 而不请求 vendor，仍完成内置库源码导航、文档生成和使用预编译库的实际程序构建。
- 自有 C／Rust 胶水在 sources，上游源码／补丁在 vendor；归档不混入其他版本下载或二进制 SDK ZIP。
- 两个不同 work-dir 使用同一个 download-dir，实际只获取一次固定输入，原输入及工具均保留。
- 相同输入再次使用时没有网络请求、重复摘要或输入复制；已完成 native 构建先走原更新判断。
- 并发请求同一文件只发布一份完整结果；真实下载中断不发布最终文件，随后可以正常重试。
- 改变依赖版本只获取新项；只改补丁复用原始归档并更新相应工作副本。
- 显式缺失输入、下载失败、普通 I/O 错误得到直接诊断；路径含空格及 Windows 路径可用。
- 在实际组装与 native 消费中检查复用效果，不通过删除／隐藏源码、工具或原产物制造验收条件。

Python 实现变更后先执行仓库要求的格式化与 lint，再运行维护脚本测试及所需发布专测。原始日志、下载内容和试验归档不提交到 Git；本轮本机临时文件统一放在仓库 `tmp/`，文档保留范围与结果摘要。

## 9. 相关文件

- [发行组装](../../scripts/package_toolchain.py)、[LLVM 输入准备](../../scripts/package_llvm.py)、[构建更新与锁](../../scripts/build_support.py)。
- [Release workflow](../../.github/workflows/release.yml)、[维护脚本说明](../../scripts/README.md)、[发行许可收集](../../scripts/package_licenses.py)。
- [现有 LSP 源码关联](../../compiler/analysis/src/workspace/attached_sources.rs)、[初始发行调查](INSTALLER-RESEARCH.md)。
- [独立安装器设计](https://github.com/scoop-lang/scoop-install/blob/codex/scoop-install/docs/DESIGN.md)。
