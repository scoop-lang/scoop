# Scoop 编程 skill 与配套工具的标准更新工作流

日期：2026-11-12。适用对象：编译器、内置库、正式 CLI、语言服务、文档、示例和发行工具的维护者。配套[改进计划](PLAN.md)。

本工作流的现有命令可用于当前仓库。API 数据查询和 Agent 语言服务客户端尚属计划能力，仅在其实现后执行对应步骤。流程遵守仓库 AGENTS.md 与权威规范，不新增审批、认证或通用检查框架。

## 1. 何时执行，谁负责

改变语言/API 的可用写法、诊断、资源语义、命令行为、示例或发行布局时，变更作者在同一批工作中同步受影响的材料和实际消费者。评审关注能否用当前工具链完成说明中的操作；不必另设文档批准流程。

纯内部重构、未改变可观察行为的优化、与应用开发无关的维护修改可以不改 skill。存在公开性能或生命周期承诺时，相关变化仍需更新文档。交付记录简短说明影响范围即可，不要求所有提交都填完整表。

新增能力不能只增加一条“支持某功能”；应提供入口、正确用法、重要边界和验证。修复旧限制时，应查找并删除已失效的警告、workaround 和误导示例。项目未发布阶段直接更新调用方，不为旧 skill 保留编译器/库兼容别名或双轨接口。

## 2. 按变更类型确定同步范围

| 变更类型 | 首先核对的规范/合同 | 应检查并按需同步的材料 | 验证重点 |
| --- | --- | --- | --- |
| 语法、类型、可见性、泛型、闭包 | `SCOOP-SPEC.md`；涉及 pipeline 时检查实现规范 | skill 语言参考、相关语言主题、API 风格、编辑器语法/语言服务说明 | 合法写法和代表性编译错误、位置、组合使用 |
| 内置库签名、命名、约束、行为 | 对应库规范及 API 风格 | 声明注释、实际调用方、库/发布示例、skill 库导航、API 文档 | 正常/错误/空值行为、公共消费者、必要目标差异 |
| 资源、异步、取消、native 行为 | 对应库规范；仅实际涉及时修改 runtime/ABI | 生命周期说明、错误与收尾示例、领域参考 | 成功与失败都收尾、取消/超时、真实平台合同 |
| CLI、项目、依赖、测试运行方式 | 实现规范与相关工具设计 | `--help`、skill 命令参考、示例运行步骤、runner、发行说明 | 参数、目标/purpose、退出码、标准流、文件还原 |
| 诊断、补全、修复与导航 | 实现规范语言服务条款及现有协议 | 调试指导、真实能力清单、客户端展示 | 主因、关联位置、JSON/LSP 一致性、不完整源码 |
| 文档、搜索、示例结构 | 文档/示例输出合同 | catalog、正文、相关链接、数据导出、阅读器、skill 导航 | 可发现性、完整输出、无匹配和实际错误 |
| 打包、sysroot、安装布局 | 实现规范及发行设计 | 包内容、tooling README、随包说明、相对路径 | 安装后实际命令、配套源码/例子/skill/文档 |
| 仅内部实现变化 | 当前外部合同 | 确认上述使用方式没有变化 | 相关实现回归；无影响时无需改 skill 或全量运行示例 |

这张表用于定位维护对象，不要求每次修改所有列出的文件，也不改变缓存的普通更新规则。

## 3. 先确定事实和验证工具链

1. 阅读受影响规范，明确这是新增行为、修正规范与实现冲突，还是只改善已有行为的说明。涉及语言、runtime 或 pipeline 合同先修改对应规范及引用方，再修改实现。
2. 选定本次源码构建的配套命令和开发 sysroot，或候选发行安装根。ABI/产物格式变更时，相关 driver、compiler、linker、package manager 使用当前配套构建；按 [scripts 说明](../../scripts/README.md) 显式准备受影响目标的资源。
3. 确认版本与实际命令入口。Windows 主驱动为 `sco.exe`，其他宿主为 `scoop`；外部命令仍使用 `scoop-` 前缀。不要偶然从 PATH 选到同名 Windows 包管理器或另一份工具链。
4. 用最小程序确认关键规则。实际运行与规范冲突时定位并修正冲突，不能据一次复杂程序的候选错误就把限制写进 skill。

下面变量用于后续命令示例，替换为本次真实路径；工作目录位于仓库外。仅演示现有命令，不执行安装或修改用户 Agent 配置。

```sh
SCOOP_MAINT_BIN=/absolute/path/to/current/bin/scoop
SCOOP_MAINT_SYSROOT=/absolute/path/to/current/sysroot
SCOOP_MAINT_WORK=/absolute/path/outside-repository/scoop-programming
"$SCOOP_MAINT_BIN" --version
"$SCOOP_MAINT_BIN" --sysroot "$SCOOP_MAINT_SYSROOT" example --list
```

只修改 skill 文案或例子正文时，复用已经配套的工具链，不重新准备 LLVM/runtime/系统库。真实库输入更新则走既有准备流程，普通示例测试不承担准备职责。

## 4. 查找受影响内容并同步源材料

从变更的符号、旧写法、错误消息和主题名出发，用 `rg` 检查以下位置；已有引用方必须迁移，搜索范围随后按依赖收敛：

- `docs/specs/` 及本次功能设计：规则与交叉引用。
- `sysroot/src/<cone>/` 及实际消费者：声明注释、源码、测试和库示例。
- `tooling/scoop-programming/SKILL.md`、`references/`、`agents/openai.yaml`：使用指导与可发现的任务范围。
- `sysroot/share/scoop/examples/catalog.toml`、`topics/`、`programs/`：主题元数据、正文和完整文件。
- 相关工具的帮助/实现/测试、`tooling/README.md`、`scripts` 与发行说明：查询、验证、安装路径。

同步时遵循以下顺序：

1. 明确规范与公开声明，完善真正需要的错误、所有权、挂起性、单位及目标说明。
2. 更新实现和实际调用方，移除已被替代的写法。
3. 更新或补充有代表性的可运行例子及必要测试。
4. 更新 skill 的规则摘要、导航与调试指导；完整签名和程序保持在原来源，不复制到多个参考文件。
5. 更新帮助、主题搜索词和相关链接。新主题只有源码、正文、依赖和验证齐备后才加入 catalog；相关主题必须已存在。

skill 主文件保留任务流程和按需阅读入口。仅当新能力改变通用开发步骤时调整主流程；领域细节放进对应参考。能力已经支持后，旧的“不支持”“必须手写”说明应直接替换，不留相互矛盾的附注。

普通文档示例尽量引用现有可运行主题。少量必须内联的关键语言写法，使用相关主题/测试覆盖；不为抓取全部 Markdown 代码块另建一套语言解析器或通用测试框架。

## 5. 更新示例工具与文档

现有示例维护遵守 [scoop-example README](../../tooling/scoop-example/README.md)：

- `source` 相对安装根，`path` 是用户还原文件时使用的路径，二者不可混淆。
- 复用库示例引用原源码；新增教学程序放在 `programs/<主题 ID>/`。正文说明运行、预期结果、关键说明、常见误用，不另存整段源码副本。
- 关键词描述主题确实覆盖的内容。用用户会使用的领域词与 API 名检查检索，不能只知道主题 ID 时才找得到。
- 同步 runner 的相关行为验证，分别检查标准流、退出码与程序结果。失败时修复源码/正文/期望中错误的一方，不能一律重录实际输出作为新期望。

需要更新开发阅读器或安装其材料时，使用现有入口：

```sh
python3 scripts/build_scoop_example.py --sysroot "$SCOOP_MAINT_SYSROOT"
```

离线 API 文档由现有 `scoop doc` 或安装根的 post-install 流程生成。开发时只生成受影响的文档；发行时按已安装目标和库清单生成完整站点，遵循 [scripts 说明](../../scripts/README.md)。检查签名、公开 re-export、源码位置及相关链接，不手工修改生成的 HTML 或缓存文件。

语言服务入口为 `scoop-lsp query`，合同见[语言服务 CLI](../language-service/CLI.md)。更新查询能力时同步该工具的帮助、JSON 合同、skill 用法及真实 stdio 对照测试；不另建类型分析逻辑。B3 的独立 API 查询尚未交付时，不能把 `scoop api` 写成必需步骤；旧工具链继续保留示例、文档和库源码查询路径。

## 6. 按实际影响验证

先格式化与 lint，再测试。以下命令按修改范围选择；格式化命令会改文件，应在本次需要处理的目录/文件上运行。文档纯文字改动只需链接、命令和事实核对，不因此启动编译器全量验收。

### 编译器及 Rust 工具

编译器 Rust 修改按仓库约定执行：

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
```

随后运行相关 crate 测试，并按变更要求完成 `cargo test --workspace` 与正式 fixture 覆盖。语言错误使用代表性的 expected-error fixture，断言位置与诊断；普通库行为使用 Cone 测试。

仅修改独立示例阅读器时使用其 Cargo 项目：

```sh
cargo fmt --manifest-path tooling/scoop-example/Cargo.toml
cargo clippy --manifest-path tooling/scoop-example/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tooling/scoop-example/Cargo.toml --locked
```

### Python 与归档布局

对实际改动的 Python 文件使用仓库固定的 Ruff 版本，例如：

```sh
uv tool run --from ruff==0.16.10 ruff format tooling/scoop-example/tests/run_examples.py scripts/build_scoop_example.py
uv tool run --from ruff==0.16.10 ruff check tooling/scoop-example/tests/run_examples.py scripts/build_scoop_example.py
```

涉及发行 skill 内容/路径或打包脚本时运行相关归档测试；改变依赖或随包工具时追加对应许可测试：

```sh
python3 -m unittest discover -s scripts/tests -p test_package_toolchain.py
python3 -m unittest discover -s scripts/tests -p test_package_licenses.py
```

fixture runner 有修改时，按其 [README](../../tests/fixture_runner/README.md) 和 AGENTS.md 完成 Python 格式化、规则单测及正式 fixture 验收。

### Scoop 程序与发布示例

所有步骤使用第 3 节选定的命令和 sysroot。在改动且带测试的 Cone 根目录中，例如：

```sh
"$SCOOP_MAINT_BIN" --sysroot "$SCOOP_MAINT_SYSROOT" fmt .
"$SCOOP_MAINT_BIN" --sysroot "$SCOOP_MAINT_SYSROOT" check . --release
"$SCOOP_MAINT_BIN" --sysroot "$SCOOP_MAINT_SYSROOT" check . --tests --release
"$SCOOP_MAINT_BIN" --sysroot "$SCOOP_MAINT_SYSROOT" test . --release
```

按实际目标选择参数并运行必要的 build/run；单文件使用对应输入模式。`check` 不证明 native 链接、运行或资源收尾成功；library 不运行不存在的 main。后面的维护脚本命令回到仓库根目录执行。

用选定工具链从正式 JSON 输出还原发布主题，例如本次修改 Option 和断言主题时：

```sh
python3 tooling/scoop-example/tests/run_examples.py \
  --scoop "$SCOOP_MAINT_BIN" \
  --sysroot "$SCOOP_MAINT_SYSROOT" \
  --work-dir "$SCOOP_MAINT_WORK" \
  --topics language/option test/assertions
```

用实际受影响主题替换末行。影响多个主题的语言/库变更，或候选发行包含示例/工具链变化时，去掉 `--topics` 完成全部已发布主题验证。没有新修改或未闭合问题时，复用本批已有结果，不重复同一组检查。

普通验证保持 release、共享 Cone cache，并使用已经准备的系统库。CLI/进程测试关注真实标准流、退出状态、通信和收尾；网络使用本地可控服务；UI 在实际支持的目标运行并注明环境。GC stress 只用于 GC 正确性专项。

## 7. 发布与安装验证

1. 使用现有 `scripts/package_toolchain.py` 组装候选包，确认 skill 整目录进入 bin 组件、示例与库源码进入现有 sources 组件。`programming_skill` 继续记录普通相对路径，reference 链接保持完整。
2. 按现有发行流程将同一候选版本所需组件合并安装到测试目录，使用其中真实命令读取示例、编译相关程序并生成对应目标文档。该测试目录服务安装合同，不为每个普通 consumer 复制完整 sysroot。
3. 同一套命令、目标库、声明、示例、skill 和生成文档应描述同一能力。查询或运行失败时报告实际问题，不自动换工具链、清空缓存或补建未准备的系统库。
4. skill 打包不等于已更新用户 Agent 中复制安装的 skill。发行说明提示其按所用工具更新整个 skill 目录并保留 references；不由打包/安装脚本擅自注册技能或修改 Agent 配置。
5. 检查版本只使用现有版本/组件信息和真实调用，不增加摘要收据、生产过程审计、命中产物内容验证或来源认证。保留源码、工具和原产物，缓存验证遵循普通输入变化与复用规则。

单次 skill 文字修正不要求重新打包所有平台。完整安装验收在候选发行阶段执行；日常使用归档布局测试和受影响内容检查提供反馈。

## 8. 简短记录与完成条件

在 PR 描述或本次功能记录中按需写下：

```text
使用方式变化：发生了什么；原调用应如何更新。
同步材料：规范/声明、skill、主题、工具帮助或文档的相关位置。
验证：工具链与目标；已运行的相关检查及结果；未验证范围。
剩余事项：具体问题及所属批次；若无则省略。
```

无需单独提交逐项无影响声明、原始会话、完整终端日志或逐次 JSON 报告。原始材料留在仓库外。

本次维护完成意味着：相关新写法可运行、旧错误指导被移除、示例和 API 文档可发现、实际调用方已迁移，正式输出与测试一致。环境不足的验证明确标为未完成，不能写成已通过。

## 9. 新查询能力落地后增加的步骤

以下是[改进计划 B3/B5](PLAN.md)的配套工作，当前不执行：

- API 查询：在原有文档生成/安装更新中输出同目标机器数据；从发布的查询入口验证改动符号、重载、约束、import、相关例子和无匹配行为。API 读取不触发隐式构建或重验生成过程。
- 语言服务客户端：诊断或查询能力变化时，同步实际 capability 和帮助；用真实项目位置验证客户端与 LSP 一致，包括非 ASCII 文本、生产/测试依赖和源码更新。
- skill：只有命令及数据已进入配套工具链，才将其加入推荐查询路径；移除被新路径取代的繁琐步骤，保留有意义的源码定位入口。
- 自动检查：扩展现有文档、示例、协议与打包测试，只覆盖新增合同；不新建重复语义检查、全库内容审计或通用评估服务。

首次启用时，应将已实现部分移入本文对应常规步骤，并删除本节中的未来时表述，保持工作流可直接照做。
