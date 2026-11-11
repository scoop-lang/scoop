---

name: scoop-programming
description: 设计、编写、修改和调试 Scoop 编程语言的 .scoop 程序与 Cone.toml 项目，通过配套示例、API 声明及 CLI 完成实现和验证。适用于库、数据处理、命令行、异步网络与 UI 开发，不用于 Windows Scoop 包管理器。
---

# Scoop 程序开发

Scoop 是静态类型、编译到原生代码的语言，语法借鉴 Kotlin，值类型、可空性、闭包和库 API 有独立规则。使用当前工具链提供的示例和声明确定写法，再通过实际编译与运行验证。

## 确认工具链和项目

先查看项目已有的 `Cone.toml`、源码、测试及项目约定，保留既有组织方式。优先使用用户指定的主驱动路径；未指定时在 Windows 上从 PATH 查找 `sco.exe`，macOS／Linux 查找 `scoop`，常见安装目录是 `~/.scoop/bin/`。Windows 的 `scoop` 命令可能属于同名包管理器，不将其当作本语言驱动。调用时保留所选入口名：multicall 安装器通过 `scoop`/`sco` 分发，不能解析符号链接后改为调用 `scoop-install`。

```sh
scoop --version
scoop example --help
```

本文及所附参考资料命令中的 `scoop` 均指这份可执行文件；Windows 上替换为 `sco`，例如 `sco --version`、`sco example --help`。其他工具名保持不变，包括 `scoopc.exe`、`scoop-pm.exe`、`scoop-lsp.exe` 和 `scoop-example.exe`。`<SYSROOT>` 指所选安装根：显式 `--sysroot` 优先，其次为 `SCOOP_SYSROOT`，否则使用实际可执行文件所属安装根。示例、库声明和编译运行使用同一套资源。指定安装根查询示例时，参数放在外部子命令前：

```sh
scoop --sysroot /path/to/scoop example language/option
scoop --sysroot /path/to/scoop check main.scoop
```

`scoop example` 是独立阅读器；它从自身所在的安装根读取材料。若命令缺失，检查选中的工具链，并从该安装根的 `src/`、`share/scoop/examples/` 和离线文档查找现有材料。不要把查询失败误报为语言不支持，也不要为应用开发自动重建编译器或系统库。

## 确定边界，再验证最不确定的组合

先明确程序的输入、输出、失败行为和需要保存的状态。设计数据模型、跨文件或跨 Cone 接口时阅读[设计与复审](references/design.md)；按该文档中的领域入口选择相关材料，不为简单脚本建立完整分层。

从一个能编译运行的实际功能开始。对尚未确认的 Option/泛型、receiver、挂起回调或库调用，先用最小源码核实声明与行为，再扩展到其他文件。已经确认的写法直接复用；一次完成一批有内聚关系的修改再验证，不逐行检查，也不把未确认的写法复制到整个项目后才编译。

## 按问题查询示例与 API

按实际问题选择主题，不必读取全部材料：

```sh
scoop example --list
scoop example --search "JSON 序列化"
scoop example json/typed
scoop example http/get --format source --file client.scoop
scoop example start/cone --format json
```

- `--list` 查看已发布主题，分类名（如 `language`）查看该分类；搜索只匹配主题元数据，不搜索全部 API 或源码。多个空白分隔的词必须同时命中同一主题；没有结果时缩短关键词或查看列表。
- 默认 Markdown 包含完整文件、运行条件、步骤、预期结果、关键规则和常见误用。先读这些说明，再复用源码。
- `--format source` 输出原文；`--file` 选择**显示路径**，不指定输出文件。多文件主题必须选择文件，不能只取客户端而遗漏所需服务或 manifest。
- `--format json` 的主题结果为 `schema: 1`、`kind: "topic"`。按 `files[].path` 在工作目录保存 `files[].text`，保留子目录关系；`files[].source` 是安装根内的材料来源，不是项目输出路径。一起阅读 `requirements`、`body_markdown` 和 `related`。
- 阅读器不会创建项目或执行程序。保存或合并文件后，仍需运行对应命令验证。内容错误、无匹配和 I/O 失败退出 1，用法错误退出 2；错误在 stderr，不能当作正常 JSON 主题继续处理。

常用入口（具体可用主题以本机列表为准）：

| 要解决的问题                         | 主题                                                                                               |
| ------------------------------------ | -------------------------------------------------------------------------------------------------- |
| 单文件、依赖、多文件项目             | `start/hello`、`start/single-file`、`start/cone`                                                   |
| 值、Option、模式匹配、闭包、公开 API | `language/overview`、`language/option`、`language/enum`、`language/lambdas`、`language/visibility` |
| Unicode 文本与集合                   | `text/strings`、`collections/list`、`collections/map`                                              |
| 文件、进程、环境与 JSON              | `io/files`、`io/resources`、`io/processes`、`io/environment`、`json/typed`                         |
| 计算与展示、终端与界面状态          | `apps/file-report`、`terminal/output`、`ui/state`                                                 |
| 异步任务、HTTP 与命令行参数          | `async/tasks`、`http/get`、`cli/options`                                                           |
| Cone 测试与异常断言                  | `test/assertions`                                                                                  |

## 写出符合 Scoop 规则的程序

首次编写或移植 Scoop 代码前，阅读[语言差异](references/language.md)，尤其是 `Option`、f-string、值类型更新、显式可见性和闭包捕获。不要直接套用 Kotlin 的 `null`、`data class`、`copy`、协程库或 Java API。

需要库能力时，按[库与 API 查找](references/libraries.md)确定所属 Cone、import、实际签名及资源生命周期。只读取相关声明和调用示例；示例未命中时直接定位库，不反复扩大教程关键词或通读全部源码。改写 Option/Result 分支前先查已有组合操作，注意返回形状和 fallback 的求值时机。

需要当前位置的类型、成员或调用签名时，检查所选工具链的 `scoop-lsp query --help`，按[语言服务查询](references/projects-and-tools.md#语言服务查询)使用 hover、completion、signature 等入口。它按当前 Cone 和依赖回答；按名字发现尚未使用的库 API 仍查文档和声明。较旧工具链没有 query 时沿用这些已有路径。

新建多文件项目优先使用 `scoop new my-app --group example.app`，目标目录必须尚不存在；默认带测试。小型独立程序可以使用单文件模式。具体依赖、package、源码选择、测试和工具选项见[项目与命令](references/projects-and-tools.md)。特别注意：
- 传项目目录或 `Cone.toml` 才会编译整个 Cone；传 `src/main.scoop` 会选择单文件模式。
- 依赖声明和 import 分别负责可用库与可用名称；单文件 `//! dep:` 注释在 manifest 模式中不起作用。
- 复用系统库的现有能力；按业务需要处理缺失值、错误、取消和资源关闭。

## 验证修改

先格式化，再检查语义，最后按交付内容运行测试和程序。例如在带测试的可执行 Cone 根目录：

```sh
scoop fmt .
scoop check . --release
scoop check . --tests --release
scoop test . --release
scoop run . --release -- Ada
```

上例最后的 `Ada` 是程序参数，按实际程序修改。单文件用 `scoop fmt main.scoop`、`scoop check main.scoop --release`、`scoop run main.scoop --release`；library 用 `scoop build . --release` 和所属 Cone 测试验证。`scoop check` 完成前端检查，不证明后端、native 链接或运行成功；`--tests` 也不验证生产 main。

普通行为测试使用 `scoop.test`，从 `test/assertions` 取得准确的容器、注解和断言写法。根据需求准备输入和期望，覆盖本次实际涉及的零值/空值/缺失/失败差别，以及资源收尾或副作用；不要只把实现当前的输出抄成期望。精确比较使用 assertEquals 等能保留期望与实际值的断言。性能测量使用 release 构建，普通功能验证保持 GC stress 关闭。

需要机器诊断时使用 `scoop check . --message-format json` 等命令并读取 stderr。诊断与预期不符时按[定位错误](references/diagnostics.md)检查内层表达式、实际参数和完整重载，再判断是否存在编译器问题；不要仅凭候选失败消息弃用有效 API。验证 CLI 标准流时先 build，再直接运行程序，避免把编译进度当成程序 stderr。

修复编译错误后按[设计与复审](references/design.md#完成一批修改后复审)检查数据和错误是否被丢失、是否重复 I/O、是否用多余包装或弱化断言换取通过。验证通过后继续交付；仅在新修改、失败或未解决的问题需要时扩大测试范围。

交付时说明修改的文件、运行方法、已执行检查与结果。工具链、目标资源或运行环境缺失时明确区分“未验证”和“验证通过”，保留可复现命令及实际阻碍。
