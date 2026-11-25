# 原生调试支持范围

Windows／PDB 已与主线共享同一套源码记录和调试器交付流程，目标与工具链入口见 [Windows 支持范围](../windows-targets/README.md)。

基础源码调试、Throwable 历史栈与 trap 栈打印、调试器打包和 VS Code 集成已完成。编译器与工具的具体能力、维护入口和平台限制如下；完整 Scoop 表达式求值及协程逻辑栈继续作为独立工作。

## 编译接口与产物

`scoop build/run/test` 和 `scoopc` 接受 `--debug-info none|line-tables|full`，与 O0／O2 独立。debug 默认 full，release 默认 none；release + full 保持优化并提供仍可定位的变量。仅消费产物的 `scoop link`／`scoop-link` 使用 `--emit-debug-file`，不能补造输入中没有的调试数据。

| 目标 | 产物及处理 |
| --- | --- |
| Darwin/AArch64 | DWARF 5、相邻 dSYM；dsymutil 完成后 strip，再按最终字节签名 |
| Linux/amd64 GNU、musl | DWARF 5、相邻 `.debug`、`.gnu_debuglink`；动态及静态程序均保留 SHA-1 build-id |
| Windows/x86_64 MSVC、LLVM-MinGW | CodeView、相邻 `.pdb`；PE 中保存 PDB basename，GC／SEH 数据独立保留 |

伴随文件参与正常结果存在判断和发布，最终 basename 进入调试结果更新键。同 basename 换目录可复用，换 basename 重新生成最终调试文件；命中不启动调试工具、不重读产物内容。旧 `--emit-symbol-map`、map 字段和性能 map parser 已移除。性能工具使用 LLVM JSON、标准调试文件与精确 linkage identity，支持 inline 链、Darwin slide 和 Linux load bias。

合并 compiler intrinsics 后，machine protocol 为 22，runtime ABI contract 为 30，metadata ABI 保持 9；当前 HIR cross-cone-interface 为 87。contract 30 为程序启动入口增加栈快照开关；编译器、runtime 和机器产物须沿正常 ABI 边界配套重建。该开关不再改变 Throwable 前缀或模板编码。

## 来源共享与 `.slib`

`scoop-source` 集中保存共享的文件、位置、作用域、声明与内联调用链；HIR／MIR／LIR 节点保留共享引用和本阶段实际变换关系。MIR／LIR 没有新增完整的持久调试快照。跨 Cone 所需的局部名称进入原 HIR 模板局部表，已有来源与定义／求值语义继续复用。LIR 的具体类型、字段偏移和变量存储关联仅服务当前原生 DWARF 发射。

低成本体积优化已实施：现有 Zstandard 成员压缩统一由 `-3` 改为 `3`，reader 和容器格式保持兼容。对已有 25 个 Darwin 库离线重编码，成员合计减少约 106.82 MiB，约占 849.83 MiB 全样本的 12.6%；这是现有成员的重编码估算，不是全量重建归档的大小。来源 wire 全面索引化、LIR／manifest 身份去重、对象／DWARF 合并等较大方案及测量方法见[体积调查](../slib-storage/FOLLOW_UP.md)。

## DWARF 与移动 GC

函数同时保留源码名和精确 linkage name；line-tables 也保留 linkage，full 另提供参数、局部、全局／TLS、作用域和实际类型布局。生成操作显式没有用户位置，生成 helper 标为 artificial。内联链、模板局部名称与闭包实际读取贯通到最终记录。generated-C 桥保留展开信息和物理帧，不伪造用户源码行。

变量只观察程序已有的 SSA 值或存储，不增加调试专用的读取、栈槽、GC root 或额外保活。已有 canonical root 先通过 declare 交给 SROA 维护真实分片，GC lowering 后统一为可结束位置区间的 value 记录。safepoint 后使用该边实际恢复的根和支配当前观察的真实 relocation；不能确认的位置显示 unavailable。调用结果在旧观察失效后建立位置，normal／unwind 边分别处理。

定向后端测试比较 none／full 的 root、slot、statepoint 和 relocation 数量；真实 DAP 用例进一步检查 GC 后对象地址已改变且字段仍正确、新建对象在存活时可展开、已失效变量不可用、调用者帧不显示搬迁前地址。

## Throwable 与 trap

Throwable 是可继承的 intrinsic FixedObject，保留公共零参数构造；24-byte 前缀中 offset 16 保存私有快照引用，普通派生字段从完整前缀后开始，GC 扫描继承该隐藏根。

`--enable-stack-trace[=true|false]` 是程序级链接选项，源码构建 debug 默认开启、release 默认关闭；`scoop link`／`scoop-link` 默认关闭，可显式开启。它独立于 DWARF 和符号保留，不增加 runtime 变体或 `.slib` 数据。关闭时准备入口直接返回原对象，快照槽保持空；`printStackTrace()`、未捕获报告和 trap 在类型／原因之后输出 `  <stack trace unavailable: disabled>`。链接更新键使用生效后的布尔值，切换只重新链接。

开启时，首次 throw 的 Managed 准备采集物理地址、分配普通 immutable byte array 并一次发布，返回潜在搬迁后的同一异常对象；重抛和延迟打印保留原栈。同对象并发采集只发布一份完整快照。大栈缓冲仅位于不可内联的采集慢路径，关闭及已有快照的快路径无需该缓冲或 stack probe。`printStackTrace()`、未捕获报告及语言 trap 使用稳定数据输出；语言 trap 保持退出码 1，内部 compiler fatal 保持 abort 语义。

运行期符号化使用固定 libbacktrace 版本及源码名／DWARF 5 完整路径修补，不启动外部 LLVM 或 LLDB 进程。快照保存原始 PC、查询 PC、模块 UUID／build-id、load bias 和路径。没有调试信息时保留模块与地址；检测到 loader 变化时保守降级，避免用新模块误解释旧地址。

## LLDB、VS Code 与发行脚本

固定 LLVM 22.1.8 的 Python-enabled LLDB 构建保留源码显示名及精确身份，支持按两种名称命中同一断点；同源文件的独立编译单元全局量加入所选栈帧，TLS 使用该线程上下文。Linux/amd64 静态 TLS 从 PT_TLS 和线程指针定位，不依赖动态 link map。

VS Code 扩展 0.3.0 注册 Scoop debug type 和源码断点。构建／launch 从 JSON executable 记录取得真实路径；支持 attach、参数、cwd、环境、终端、sourceMap，以及 `scoop test --no-run` 与测试过滤。只读 formatter 显示 UTF-8／内嵌 NUL String、分页数组、tagged／niche enum、class／interface 和 raw 字段，不执行 getter／toString 或写入 managed 对象。使用方式见[扩展说明](../../tooling/vscode-scoop/README.md#原生调试)。

完整打包入口调用 LLVM／Clang SDK 与 LLDB 构建，并收集 lldb、lldb-dap、LLDB library、argdumper、debugserver／lldb-server、匹配 Python、标准库／原生模块、Clang resources、formatter 和实际非系统动态依赖。依赖改为安装根内相对加载；Darwin 最后签名并保留 debugserver entitlements。源码兼容修补随 llvm-sources 分发。维护与复用方法见[脚本说明](../../scripts/README.md#调试器发行资源)。

发行包公开 `bin` 中的 LLVM／LLDB 命令统一带 `scoop-` 前缀，例如 `scoop-lldb`、`scoop-lldb-dap` 和 `scoop-llvm-symbolizer`，Windows 附加 `.exe`。编译器相邻工具查找、VS Code 默认 adapter 与性能分析脚本同步使用发行名称；私有 helper 和显式外部 LLVM 保留上游命名。

## 平台边界与维护入口

Windows 的 CodeView／PDB 可以表示源码位置、参数、全局、TLS 和可定位的聚合成员；部分纯计算值与聚合分片显示 unavailable，断点顺序随真实指令布局变化。调试观察使用程序已有存储和 GC 位置，不额外延长对象生命期。

协议、静态值和进程内调试的维护入口见[调试验收说明](../../tooling/vscode-scoop/test/debugger/README.md)，栈诊断用例位于 `tests/fixtures/compiler-correctness/debugging/`。历史验收中的平台范围、macOS 主机 launch 限制、栈快照性能测量和逐轮回归已移到相邻归档 `../scoop-doc-archive/docs/debugging/IMPLEMENTATION.md`，也可从 [Git 历史](https://github.com/scoop-lang/scoop/blob/f93c4b46082def07aa51103567e2d6a2ea0978fc/docs/debugging/IMPLEMENTATION.md)查阅。

## 后续范围

协程已有 owner、state、completion 和 saved value identity，但缺少可消费的状态到挂起行映射、当前 frame 参数和 adapter／终端 continuation 识别。完整逻辑栈还需验证线程迁移、合成帧选择及 saved locals，不能只靠重命名物理 helper 或打印任务树完成。当前保留物理栈，后续按[主设计 10.4](DESIGN.md#104-异常协程和数据断点) 独立实施。

完整 Scoop 求值／条件表达式、managed 写入、Scoop 异常筛选及 GC 感知数据断点均不属于当前交付。复杂 `.slib` 优化保留在体积调查中。
