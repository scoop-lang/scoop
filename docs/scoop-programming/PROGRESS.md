# Scoop AI 编程支持实施进展

对应[改进计划](PLAN.md)与[标准更新工作流](UPDATE-WORKFLOW.md)。分支：`codex/scoop-ai-programming`。日期：2026-11-12。

## 批次状态

| 批次 | 状态 | 结果 |
| --- | --- | --- |
| B0 | 完成 | 修正挂起 lambda、相等断言、字符串计数和事件循环指导；CLI 示例分离标准流；维护工作流进入仓库约定 |
| B1 | 材料完成，待效果验收 | 通用设计、调试和复审流程已进入 skill；与 B2 示例一同进行 B7 对照 |
| B2 | 完成，本机验证通过 | 31 个完整主题；进程、环境、终端、报告和 UI；Cone 行为测试及真实进程/原生宿主验证 |
| B3 | 完成，本机验证通过 | 文档同步发布 API 数据；`scoop api search/show` 读取真实签名、约束、导入、文档和来源 |
| B4 | 完成，相关回归通过 | 保留实参根因、真实 import 提示及版本化编辑；完整前端测试有 42 项已独立复现的既有失败 |
| B5 | 完成，本机验证通过 | `scoop-lsp query` 复用 stdio LSP；含 B4 新增 code-actions，共 12 类查询，支持 JSON、位置编码和正文覆盖 |
| B6 | 完成，六个目标运行通过 | 覆盖整个通信和最终等待的期限，保留部分输出，完成直接 child、端点和 worker 收尾 |
| B7 | 进行中 | K3 新旧材料数据样本均通过七组独立输入；K3.1 Pro 库样本通过独立消费者；进程、网络及 UI 样本继续进行 |

## B0 验证

- 宿主 macOS/AArch64，复用 `scoop 0.0.0-dev.20261111 (ad6e8a2c72cf822700b6f1d1e524dfcab563742f)` 与已有预编译库，示例和库源码取本分支。未重建编译器或未变化的系统库。
- `language/lambdas`、`async/tasks`、`cli/options`、`test/assertions`、`language/compiler-inputs` 五个受影响主题通过；失败项修复后只复验相关主题。已定型普通函数转为挂起函数的独立负例仍被拒绝，临时探针已清理。
- 异步示例实际执行带 receiver 的推断挂起回调及显式挂起回调。Option 相等断言保持原有测试入口。
- CLI 验证先构建一次，再直接运行程序，分别检查正常输出、帮助、业务错误及解码错误；不把编译进度误当成应用 stderr。
- 修复验证时发现的实际启动问题：runner 保留 `--scoop` 的可执行入口名，避免将 multicall 安装器的 `scoop` 链接解析成 `scoop-install`。
- Scoop 格式化、Ruff 0.16.10 格式化/lint、skill 结构检查及四个归档布局测试通过。未修改 Rust，因此未运行编译器全量测试。

原始执行日志与 AI 会话产物保存在仓库外。共享开发资源、示例缓存和基线样本仍供后续批次复用；各批清理不再使用的临时探针。

## B1 材料与验证

- 主 skill 引入最小功能验证和按需设计/诊断参考；新增 design 与 diagnostics 两份参考，覆盖值/状态、Cone 边界、Option/Result、错误/资源、异步和 UI 的实际判断。
- 修正库导航中的类型归属、进程/环境入口及 LSP 缺少 lock 时的实际解析行为，更新界面元数据。保留精确断言与编译后的设计复审，明确直接运行 CLI 才能观察应用自身的标准流。
- 核对所引用的公开声明和语言服务实现，skill 结构、相对链接和差异检查通过。本批只修改材料，没有重复编译器或完整示例测试。
- Kimi 用户目录中的 skill 同步为当前内容；已有基线进程使用独立的原版快照，新评测使用新进程。效果结论等待多领域任务及独立行为验证，不以文件数量或文档长度计为通过。

## B2 跨领域示例

- 发布主题从 26 增至 31，新增 io/processes、io/environment、terminal/output、apps/file-report、ui/state，扩展 Option/Result、惰性回调断言、公开库消费者、异步取消与 HTTP 失败路径。材料只有一份源码，检索词覆盖实际 API 与领域词，无结果提示转向文档/源码和语言服务。
- 普通计算与组合行为放在 Cone 测试中；runner 从公开 JSON 还原完整文件，单独构建后观察 CLI 标准流、退出码及资源收尾。进程示例交换 stdin/stdout/stderr 并区分空成功、非零退出、创建失败；环境变量覆盖缺失、空文本与有值；HTTP 使用本机正常路由、404 与连接失败。
- macOS/AArch64 上，30 个非 UI 主题通过；UI 修正后单独通过模型和原生宿主共三项测试及应用构建。终端以真实 POSIX PTY 验证窄屏、颜色与 NO_COLOR，并检查重定向无 ANSI。Windows PTY 尚未验证，runner 明确报告该前提，不能计为通过。
- UI 真实点击验证连续加值、重排保值、删除和快捷键新增；视觉检查发现并修正未设置主题背景造成的文字对比度问题。隐藏窗口测试另行验证组件挂载、更新、关闭和宿主收尾。
- 主干合并带来 HIR 接口升级，已集中重建当前编译器和系统库，随后共享同一套资源与 Cone cache。首次执行补齐开发链接器路径后复用已编译结果；没有为各主题分别准备 sysroot。LSP 正式 CLI fixture 在新工具链下再次通过。
- 独立阅读器 9 项测试、Clippy（warnings 为错误）、Rust/Scoop/Ruff 格式检查与 Python lint 通过；完整材料的文件、相关主题及输出格式一致性由阅读器测试检查。后续 AI 效果对照仍记在 B7，不以示例数量证明质量改善。

## B3 API 数据与查询

- 文档生成器从现有可见声明与公开绑定投影 schema 1，按实际目标发布索引和各 Cone 数据。API 文件参与已有文档发布及增量更新；没有建立另一套语义检查或缓存。
- 独立 `scoop-api` 阅读器随 commands 安装，由 `scoop api search/show` 分发。支持目标、Cone、接收者、分页和 text/JSON；所有重载及 re-export 路径保留，名称特殊字符使用可复制的反引号导入路径，实例成员保留所属类型。正常空结果退出 0，读取/格式错误与用法错误分别退出 1/2，错误写 stderr。
- 签名、约束、属性访问器可见性、文档标签、源码范围和示例引用来自实际材料。未发布详情的绑定仍可发现，并明确缺失部分；不把文档可见性当作当前项目调用已经合法。
- 3 项阅读器 Rust 测试、25 项文档测试、6 组正式文档 CLI fixture 通过，覆盖真实 std/test 重载、公开别名、Unicode、隐藏声明、属性访问器、来源、错误流和增量移除旧声明。新增文件导致的增量文件数期望同步为 13；实例成员筛选在真实 Result 属性与 ChildProcess 方法上验证。
- macOS/AArch64 安装站点实际发布 44 个内置 Cone，安装后的驱动查询 Option、assertEquals、ChildProcess、NO_COLOR 和空结果通过。Ruff、Rust/Scoop 格式化、目标 crate Clippy、84 项 fixture 规则测试（4 项平台跳过）、4 项打包布局测试、9 项示例阅读器测试和 skill 结构/引用检查通过。当前 Rust 1.99 对未修改依赖仍有已有 Clippy warnings；没有据此声称其他平台运行已验收。
- skill、独立命令说明、安装脚本与维护工作流同步；Kimi skill 与开发安装材料已更新，后续评测使用新进程。查询不隐式生成文档或准备工具链。

## B4 根因诊断与导入修复

- 当前源码与导入候选共用参数推断结果，保留已经确定的独立实参错误。实际 std/test 的 F08 原例、交换参数、嵌套 Option 和安全调用均指向错误实参；好例继续调用原有 assertEquals，真正独立的名义约束错误仍被报告。内部根因分类只服务前端传播，不进入 wire、产物或 runtime。
- import 候选复用当前 Cone、直接依赖、公开转导出和真实 receiver 约束。编译 JSON 的 note.importPath 与 LSP 的 data.imports 保留同一组标识符路径和实际来源；歧义、不可见、传递依赖及已有冲突绑定不生成错误导入。当前提示枚举包级公开绑定，完整 API 发现仍由 scoop api 负责。
- LSP 提供语法/import quickfix，并核对正文及分析版本；CLI 新增 code-actions，直接返回版本化 WorkspaceEdit，调用方确认正文未变后应用。实际测试包含中文引用标识符、CRLF、当前源码/跨 Cone/标准库扩展、每个合法导入选项应用后通过检查，以及正文或其他源码变化后的 ContentModified。没有自动包装 Some 或自动选择同名 API。
- compiler worker 协议升至 31，附注新增 import 路径字段；重建配套命令，复用既有目标系统库。正式诊断/import fixture、原 CLI 查询 fixture 及原声明诊断 fixture 通过，比较主位置、原因和关联源码范围。编辑态在坏 import 后继续分析所产生的明确依赖错误单独断言，不掩盖严格编译在文件头停止的区别。
- 七项新增前端回归通过。完整 HIR 测试为 2122 通过、42 失败；在改动前 d11d78d7c3 的独立源码与构建目录复现完全相同的 42 项失败，集中于已有 core/MIR 测试输入和形状期望。本批没有将该全量结果描述为全绿，也未扩展修改无关测试。source/protocol/analysis/LSP 测试全部通过；Rust/Scoop/Ruff 格式化、相关 all-targets Clippy、84 项 fixture 规则（4 平台跳过）及 skill 结构/相对引用检查通过，Clippy 仍有未修改依赖的已有警告。
- 独立命令文档、skill 的查询与诊断步骤、工具导航和更新工作流同步；Kimi 与开发安装根的材料按同一版本更新，新评测使用新进程。本批实走了诊断变更的维护流程，库 API 变更流程将在 B6 验证。

## B5 语言服务命令行

- 按用户后续要求优先实施。`scoop-lsp query` 启动配套 `--stdio` 服务器，直接使用原有 workspace、PM、前端和查询；提供 hover、signature、completion、定义/类型定义/实现、引用、文档/工作区符号、诊断和 capability。没有新增语义数据库或编译流程。
- 支持项目、目标、production/test purpose、依赖路径、三种位置编码及 stdin 正文。结果明确区分 ok/empty/partial，保留原 LSP 数据与当前版本诊断；正常与失败路径均回收本次服务器。原 stdio 启动方式保持可用。
- 13 项 Rust CLI/生命周期/同步/语法回归通过。正式 `lsp-command-line-queries` fixture 消费已有系统库，通过 UTF-8/16/32 与原始 stdio 对照、CRLF/非 ASCII、跨库定义、扩展补全、嵌套泛型、错误源码和测试用途检查；没有应用机器产物生成。
- 测试用途的 lock 记录缺失时会返回实际项目诊断；显式 PM resolve 后查询恢复。fixture 支持复用已有目录和 lock，不以清理缓存换取通过。
- Rust/Scoop/Ruff 格式检查、目标 crate 的 all-targets Clippy、skill 结构与链接检查通过；当前 Rust 1.99 在未修改代码中报告已有 lint warnings，新增 CLI 无警告。本批在 macOS/AArch64 验证，尚未据此声称其他宿主已验收。
- 规范、独立命令说明、工具 README、skill 查询路径和维护工作流同步更新；旧工具链仍可使用文档与源码路径。对照评测和跨平台复验在后续批次记录。

## B6 进程通信期限

- 平台规范 §11.3、公开注释和实现同步增加 `ChildProcess.communicate(timeout: Duration, input)` 与 `ProcessTimeoutException`。期限使用同一单调起点，涵盖输入、两路输出和最终退出观察；负值不消费端点，零值不等待管道，长时长不会在毫秒转换时溢出。超时保存已收到的部分字节，原始 I/O 和清理错误仍按既有规则保留。
- POSIX 复用 nonblocking/poll 驱动并在中断后重算剩余时间。Windows 每个实际方向使用独立 worker，停止后取消同步 I/O、等待退出并关闭真实 thread HANDLE；取消与提交竞争不会把 ERROR_NOT_FOUND 误当作完成。不管理后代进程，后代持有 pipe 也不令有限通信无限等待；没有新增全局 runtime 协议。
- 正式 `process-communication-deadline` fixture 覆盖空成功、非零退出、启动失败、大量双向数据、参数及时间边界、分阶段共享期限、阻塞输入、部分输出、已退出 child 的后代持有 pipe、先发生的 BrokenPipe，以及重复超时后句柄和线程数恢复。资源检查没有用 GC 收集掩盖显式收尾。验证使用真实子进程，并在独立外部 watchdog 下观察结果。
- macOS/AArch64、Linux/x86_64 GNU 与 musl、Windows/x86_64 MSVC UCRT 与 LLVM-MinGW UCRT/MSVCRT 六个目标均实际通过。原 process-native 回归在 macOS、Linux GNU/musl 通过；本机平台 Cone 的 16 项相关行为测试通过。远端复用配套的 d5e40a4e1aef 安装命令，以本分支 core/std/test/platform 源码构建独立目标库；未修改其他任务的 checkout。没有据此声称其他 CPU 架构已运行验收。
- 平台复验修正了测试辅助代码的 SDK 头文件大小写、C 声明、Windows 标准输出字节写入，以及复用 work 目录时残留同步文件的问题；Linux 新 glibc 下的旧 native probe 将 atomic_load 移至 assert 外，保持原断言含义。这些修正没有放宽被测通信合同。
- 本机显式准备目标系统库，重新生成 44 个 Cone 的 API 站点；实际查询已显示 communicate 两个重载及超时异常。io/processes 从正式 JSON 还原后构建、运行通过，正文、关键词、runner 与 skill 库导航同步。Kimi 和开发安装根的 skill 已替换，后续样本使用新进程。
- 实走库 API 更新流程时补齐 runtime_builder 独立 Cargo.lock 中的文档工具依赖，保留 --locked；重试复用了四个未变化的 runtime 及 headers。受影响 Scoop/Python 格式化、Ruff、C 严格编译、skill 结构与链接检查通过。本批没有修改 Rust 源码，没有重复编译器全量测试。

## B7 当前样本

- Kimi K3 使用原版 skill 快照与配置中的 max 思考强度完成 TSV 统计任务，用时约 801 秒；完成 85 次工具调用（含读取、检索、写入与验证），产生 3 个生产与 2 个测试 Scoop 文件。
- 维护者另行验证正常统计、空输入、负数、后段非法记录、CRLF/排序、缺文件与缺参数，七组均通过；统计与渲染已分开。该基线说明不能预设旧 skill 一定产生坏结构，后续按具体缺陷、查询路径和反馈成本比较。
- 更新材料后的 K3/max 样本用时约 526 秒、52 次工具调用，同样通过七组独立输入，并实际使用 API 查询；该轮没有使用 LSP。新旧工具链也有差异，且各配置只有一次样本，时间和调用数只用于描述观察，不能归因于 skill 或推断总体提升。
- K3.1 Pro/high 修改已有库与消费者的样本约 592 秒完成。独立消费者验证输入对象和集合的后续修改不影响快照、返回集合不可作为 MutableList 修改、无 equals 的类型仍可泛型查找、首次匹配停止、空值、重复 id、负数和 Long 溢出，三组组合测试通过。公共 API 的行为注释仍不充分，评测记录保留此缺口。
- OpenCode/DeepSeek V4.1 Flash 1017 的小型 LSP 使用样本仅作工具可用性参考，不纳入 Kimi 同模型对照。
