# 首次发布前代码审查

这次审查从源码进入解析器开始，沿编译、产物写入和原生链接追踪数据，也检查了系统库、运行时的 C 接口，以及开发者日常使用的命令和编辑器。以下记录影响实现与维护的决定。公开合同仍以[语言规范](specs/SCOOP-SPEC.md)、[运行时规范](specs/SCOOP-RUNTIME-SPEC.md)和[编译器与产物规范](specs/SCOOP-IMPL-SPEC.md)为准。

## 编译器与链接器

- **解析器的嵌套状态。** 以前每次解析块或恢复某些语法错误，都要从记号序列开头重新计算括号深度。解析器现在为每个记号记录读到它之前的深度，检查点回退后也能直接取得该位置的状态。[解析器实现](../compiler/parser/src/parser/nesting.rs)
- **源码和语法树的所有权。** 同一份源码由编译器与编辑器快照共享，行索引随文本按需建立。已解析的 Cone 在驱动中持有 AST，HIR lowering 借用它，省去了整个 Cone 的语法树副本。[源码表示](../compiler/source/src/text.rs) · [HIR 入口](../compiler/hir-lower/src/pipeline/run.rs)
- **HIR 的源码位置。** HIR lowering 和输出元数据继续共享 AST 输入的 `SourceText`，移除了两次全文复制。源码位置 intrinsic、调试坐标和文档行号共用 LF 行索引，Unicode 列规则保持一致。[源码位置](../compiler/hir-lower/src/concretize/body/source_location.rs) · [调试坐标](../compiler/hir/src/concrete/debug.rs)
- **MIR 的常量折叠。** 语句和终结符共用表达式折叠规则，每次读取一次数据流事实。优化结果直接由已改写的可达块组装，保留基本块身份，删除第二次函数体复制和无用状态副本。[常量折叠](../compiler/mir-lower/src/optimization/simplify.rs)
- **LIR 与 codegen 的指令结构。** 操作数和定义槽位的遍历归入 LIR 数据结构模块；GC 活跃性、局部存储和符号引用收集共用规则，不再为每条指令创建临时列表。原有针对间接结果与调用存储的用例随规则归属调整，新增指令无需维护两份平行匹配表。[结构遍历](../compiler/lir/src/dataflow.rs)
- **对象布局的复用与诊断。** 描述符、字段访问、布局产物和调试信息共享按需计算的类布局，字段读写不再重算整条继承链。继承排序复用已知深度；扫描直接借用字段类型。缺失基类、字段越界和操作数类型不匹配沿诊断返回，并标明相关类或字段。[类布局](../compiler/lir-lower/src/metadata/layouts/classes.rs) · [存储诊断](../compiler/lir-lower/src/error/storage.rs)
- **跨 Cone 的布局与 ABI。** 选择依赖时完成提供方检查和物理导入排序。写入产物时直接移交这两份数据，省掉记录复制、键编码和第二次排序。本 Cone 的导出此时才齐备，因此在这里计算最终的语义依赖，并检查这些依赖能够解释实际导入。[LIR 依赖选择](../compiler/lir/src/layout_abi/strong_production_selection.rs) · [驱动中的产物生成](../compiler/driver/src/request/preflight/production/layout.rs)
- **错误来源和符号冲突。** 命令行输出失败保留 I/O 原因，导入常驻对象的身份计算错误沿编译诊断返回。链接器共用一条符号注册路径，生成的映像符号与对象符号发生冲突时，无论读取顺序都报 E2412。缺少映像符号所有者时报 E2409，并显示 Cone 和符号名。[编译器命令入口](../compiler/driver/src/main.rs) · [链接器符号处理](../compiler/linker/src/program.rs)

## 系统库与运行时

- **JSON 对象。** 解析、构造、编码和解码共用插入有序的哈希映射。过去解码含 n 个字段的对象时，每次取字段都线性扫描，全对象读取需要 O(n²) 次字段比较。现在每个字段按名查找，同时保留输入顺序、重复键的报错路径，以及缺失字段与显式 null 的区别。公开 builder 发布独立快照，保留 builder 的调用者可以继续添加字段。[JSON 规范](specs/SCOOP-JSON-SPEC.md) · [字段解码](../sysroot/src/scoop.json/src/decoding_containers.scoop)
- **原生接口与工具链。** C 头文件在声明旁说明注册根槽位、句柄和回调的有效期，以及运行时可移动对象时如何访问其数据。正式构建说明给出配套 LLVM 22.1.8 的准备和加载方式，覆盖异常入口与 Windows statepoint 返回地址所依赖的补丁。[C 接口](../runtime/include/scoop_rt.h) · [LLVM 构建说明](../scripts/README.md)
- **UTF-8 字符游标。** 字符读取入口拒绝指向续字节的游标，避免将尾部续字节误作字符首字节后读出有效数据范围。内容合法性继续由 String 构造保证，读取只检查当前游标。[字符读取](../runtime/src/strings.c)

## 开发工具与 fixture

- **测试命令。** 列举及执行只筛选一次编译产物里的声明，运行使用原始用例索引。终端与 JSON 各自呈现同一份带类型的结果，终端可以直接写出失败用例的原始字节。需要独占原生主循环的 Cone 可以显式选择逐用例进程，整组共用一份可执行文件。[用例选择](../compiler/scoop/src/cli/test_selection.rs) · [结果与声明的对应关系](../compiler/scoop/src/cli/presentation/test_report/summary.rs) · [包规范](specs/SCOOP-PACKAGE-SPEC.md)
- **编辑器。** 一条通知内的连续编辑复用已独占的字符串缓冲，按编辑顺序换算 Unicode 列。后续编辑范围无效时，分析器仍保有原文与原版本。Tree-sitter 同步修正了换行成员访问和泛型调用的识别。[文档同步](../compiler/lsp/src/documents.rs) · [Tree-sitter 表达式语法](../tooling/tree-sitter-scoop/grammar/expressions.js)
- **fixture 的断言。** 去虚化、内联和小值 ABI 用例删除了 36 份完整 MIR/LIR 快照，合计 1,952,163 字节。它们现在直接定位被测调用点和按架构区分的签名，同时保留原生调用方对真实参数传递的约束。发现 fixture 时先读固定长度的文件前缀，遇到标记才继续解析声明，格式错误会显示载体路径。[去虚化用例](../tests/fixtures/compiler-correctness/devirtualization) · [小值 ABI 用例](../tests/fixtures/compiler-correctness/small-values) · [fixture 发现逻辑](../tests/fixture_runner/discovery.py)
- **当前说明与历史记录。** Windows 与原生调试的旧分支进度、环境版本和逐轮验收移入相邻归档；仓库保留当前支持范围、构建入口和实际限制，并修正异步 I/O 等规范中的过时实施状态。[Windows 维护入口](windows-targets/README.md) · [调试支持范围](debugging/IMPLEMENTATION.md)
