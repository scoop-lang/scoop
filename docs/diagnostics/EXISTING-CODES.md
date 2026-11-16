# 已有诊断与能力标识

共 **96** 个字符串标识：SCOOP 55 个、SCOOPC 7 个、SLIB 34 个。其中 5 个是能力诊断正文中的标识，34 个属于产物诊断记录，其余 57 个是结构化诊断代码或其声明。这里只列定义/使用，未将未触发的定义认定为独立用户错误，也不新增编号。环境变量、工具版本与测试标记不计入。

见 [盘点总表](INVENTORY.md)。

| 序号 | 已有标识 | 含义 | 承载位置 | 源码位置 |
| --- | --- | --- | --- | --- |
| 1 | ` SCOOPC_BUILD_FAILED ` | scoopc 构建失败兜底 | 结构化诊断代码/声明 | [driver/src/child_protocol.rs:16](../../compiler/driver/src/child_protocol.rs#L16)；[driver/src/request/report/protocol.rs:228](../../compiler/driver/src/request/report/protocol.rs#L228) |
| 2 | ` SCOOPC_CAPABILITY_ODR_UNAVAILABLE ` | 当前产物不具备所需 ODR 能力 | 正文能力标识 | [hir/src/foundation/strong_profile.rs:86](../../compiler/hir/src/foundation/strong_profile.rs#L86)；[lir-lower/src/capability.rs:25](../../compiler/lir-lower/src/capability.rs#L25)；[lir/src/foundation/cone.rs:468](../../compiler/lir/src/foundation/cone.rs#L468)；[mir/src/foundation/strong_profile.rs:90](../../compiler/mir/src/foundation/strong_profile.rs#L90) |
| 3 | ` SCOOPC_CHILD_REQUEST_INVALID ` | 子编译请求无效 | 结构化诊断代码/声明 | [driver/src/child_protocol.rs:15](../../compiler/driver/src/child_protocol.rs#L15) |
| 4 | ` SCOOPC_COMPILER_WARNING ` | 编译器警告统一码 | 结构化诊断代码/声明 | [driver/src/request/report/protocol.rs:98](../../compiler/driver/src/request/report/protocol.rs#L98)；[scoop/src/cli/format/diagnostics.rs:34](../../compiler/scoop/src/cli/format/diagnostics.rs#L34) |
| 5 | ` SCOOPC_HIR_ERROR ` | HIR/语义诊断统一码 | 结构化诊断代码/声明 | [driver/src/request/report/protocol.rs:110](../../compiler/driver/src/request/report/protocol.rs#L110)；[driver/src/request/report/protocol.rs:163](../../compiler/driver/src/request/report/protocol.rs#L163) |
| 6 | ` SCOOPC_PACKAGE_ERROR ` | 源码 package 规则错误 | 结构化诊断代码/声明 | [driver/src/request/report/protocol.rs:144](../../compiler/driver/src/request/report/protocol.rs#L144) |
| 7 | ` SCOOPC_PARSE_ERROR ` | 词法/语法诊断统一码 | 结构化诊断代码/声明 | [driver/src/request/report/protocol.rs:152](../../compiler/driver/src/request/report/protocol.rs#L152)；[scoop/src/cli/format/diagnostics.rs:32](../../compiler/scoop/src/cli/format/diagnostics.rs#L32) |
| 8 | ` SCOOP_BUILD_FAILED ` | 主构建失败兜底 | 结构化诊断代码/声明 | [scoop/src/build/error.rs:72](../../compiler/scoop/src/build/error.rs#L72) |
| 9 | ` SCOOP_CACHE_IO ` | 缓存 I/O 错误 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:96](../../compiler/scoop/src/diagnostic.rs#L96) |
| 10 | ` SCOOP_CACHE_LOCK ` | 缓存锁失败 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:97](../../compiler/scoop/src/diagnostic.rs#L97) |
| 11 | ` SCOOP_CACHE_PUBLISH ` | 缓存发布失败 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:98](../../compiler/scoop/src/diagnostic.rs#L98) |
| 12 | ` SCOOP_CHECK ` | check 命令失败 | 结构化诊断代码/声明 | [scoop/src/check.rs:41](../../compiler/scoop/src/check.rs#L41) |
| 13 | ` SCOOP_CHILD_EXIT ` | 子进程退出异常 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:103](../../compiler/scoop/src/diagnostic.rs#L103) |
| 14 | ` SCOOP_CHILD_PROTOCOL ` | 子进程协议错误 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:102](../../compiler/scoop/src/diagnostic.rs#L102) |
| 15 | ` SCOOP_CHILD_RESPONSE_MISMATCH ` | 子进程响应不匹配 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:104](../../compiler/scoop/src/diagnostic.rs#L104) |
| 16 | ` SCOOP_CHILD_TOOL_MISMATCH ` | 配套子工具不匹配 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:100](../../compiler/scoop/src/diagnostic.rs#L100) |
| 17 | ` SCOOP_CHILD_TRANSPORT ` | 子进程通信失败 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:101](../../compiler/scoop/src/diagnostic.rs#L101) |
| 18 | ` SCOOP_CLEAN_IO ` | 清理输出 I/O 失败 | 结构化诊断代码/声明 | [scoop/src/cli/clean.rs:58](../../compiler/scoop/src/cli/clean.rs#L58) |
| 19 | ` SCOOP_CLI_ARGS ` | 命令行参数错误 | 结构化诊断代码/声明 | [scoop/src/cli.rs:45](../../compiler/scoop/src/cli.rs#L45) |
| 20 | ` SCOOP_CLI_CONFIG ` | 命令配置错误 | 结构化诊断代码/声明 | [scoop/src/cli/commands.rs:234](../../compiler/scoop/src/cli/commands.rs#L234)；[scoop/src/cli/native.rs:51](../../compiler/scoop/src/cli/native.rs#L51) |
| 21 | ` SCOOP_COMMAND_START ` | 外部命令启动失败 | 结构化诊断代码/声明 | [scoop/src/cli/external.rs:40](../../compiler/scoop/src/cli/external.rs#L40) |
| 22 | ` SCOOP_DOCUMENTATION ` | 文档构建失败 | 结构化诊断代码/声明 | [scoop/src/document.rs:132](../../compiler/scoop/src/document.rs#L132) |
| 23 | ` SCOOP_DOC_RESOURCE ` | 文档资源读取失败 | 结构化诊断代码/声明 | [scoop/src/document/overview.rs:88](../../compiler/scoop/src/document/overview.rs#L88) |
| 24 | ` SCOOP_FMT_INPUT ` | 格式化输入错误 | 结构化诊断代码/声明 | [scoop/src/cli/format.rs:115](../../compiler/scoop/src/cli/format.rs#L115)；[scoop/src/cli/format.rs:138](../../compiler/scoop/src/cli/format.rs#L138) |
| 25 | ` SCOOP_FMT_IO ` | 格式化 I/O 错误 | 结构化诊断代码/声明 | [scoop/src/cli/format.rs:146](../../compiler/scoop/src/cli/format.rs#L146)；[scoop/src/cli/format.rs:163](../../compiler/scoop/src/cli/format.rs#L163) |
| 26 | ` SCOOP_FMT_PARSE ` | 格式化前解析失败 | 结构化诊断代码/声明 | [scoop/src/cli/format.rs:148](../../compiler/scoop/src/cli/format.rs#L148) |
| 27 | ` SCOOP_GRAPH_ALLOCATION ` | 依赖图分配失败 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:90](../../compiler/scoop/src/diagnostic.rs#L90) |
| 28 | ` SCOOP_GRAPH_CYCLE ` | 依赖图有环 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:88](../../compiler/scoop/src/diagnostic.rs#L88) |
| 29 | ` SCOOP_GRAPH_MISSING_CORE ` | 依赖图缺少 core | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:87](../../compiler/scoop/src/diagnostic.rs#L87) |
| 30 | ` SCOOP_GRAPH_MULTIPLE_VERSIONS ` | 依赖图包含同一坐标的多个版本 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:84](../../compiler/scoop/src/diagnostic.rs#L84) |
| 31 | ` SCOOP_GRAPH_NON_LIBRARY_DEPENDENCY ` | 依赖不是库 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:85](../../compiler/scoop/src/diagnostic.rs#L85) |
| 32 | ` SCOOP_GRAPH_RESERVED_IDENTITY ` | 保留 Cone 身份不合法 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:83](../../compiler/scoop/src/diagnostic.rs#L83) |
| 33 | ` SCOOP_GRAPH_SINGLE_FILE_DEPENDENCY ` | 单文件依赖规则错误 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:86](../../compiler/scoop/src/diagnostic.rs#L86) |
| 34 | ` SCOOP_GRAPH_UNREACHABLE_NODE ` | 依赖图包含不可达节点 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:89](../../compiler/scoop/src/diagnostic.rs#L89) |
| 35 | ` SCOOP_HIR_CROSS_CONE_DISPATCH_REQUIRED ` | 需要跨 Cone 成员/分派能力 | 正文能力标识 | [hir-lower/src/imported_capabilities.rs:20](../../compiler/hir-lower/src/imported_capabilities.rs#L20) |
| 36 | ` SCOOP_HIR_CROSS_CONE_GENERIC_REQUIRED ` | 需要跨 Cone 泛型/ODR 能力 | 正文能力标识 | [hir-lower/src/imported_capabilities.rs:21](../../compiler/hir-lower/src/imported_capabilities.rs#L21) |
| 37 | ` SCOOP_HIR_CROSS_CONE_LAYOUT_REQUIRED ` | 需要跨 Cone 布局/ABI 能力 | 正文能力标识 | [hir-lower/src/imported_capabilities.rs:19](../../compiler/hir-lower/src/imported_capabilities.rs#L19) |
| 38 | ` SCOOP_HIR_CROSS_CONE_NATIVE_REQUIRED ` | 需要跨 Cone native closure 能力 | 正文能力标识 | [hir-lower/src/imported_capabilities.rs:22](../../compiler/hir-lower/src/imported_capabilities.rs#L22) |
| 39 | ` SCOOP_INTERNAL_ERROR ` | 内部错误兜底 | 结构化诊断代码/声明 | [scoop/src/artifact_link.rs:145](../../compiler/scoop/src/artifact_link.rs#L145) |
| 40 | ` SCOOP_LINK_FAILED ` | 最终链接失败 | 结构化诊断代码/声明 | [scoop/src/artifact_link.rs:129](../../compiler/scoop/src/artifact_link.rs#L129)；[scoop/src/build/executable.rs:138](../../compiler/scoop/src/build/executable.rs#L138) |
| 41 | ` SCOOP_LINK_TOOLCHAIN ` | 链接工具链错误 | 结构化诊断代码/声明 | [scoop/src/build/executable.rs:64](../../compiler/scoop/src/build/executable.rs#L64)；[scoop/src/build/executable.rs:82](../../compiler/scoop/src/build/executable.rs#L82) |
| 42 | ` SCOOP_LOCATOR_AMBIGUOUS_ARTIFACT ` | 产物选择有歧义 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:81](../../compiler/scoop/src/diagnostic.rs#L81) |
| 43 | ` SCOOP_LOCATOR_CONFLICTING_REPRESENTATION ` | 同一节点的来源表示冲突 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:80](../../compiler/scoop/src/diagnostic.rs#L80) |
| 44 | ` SCOOP_LOCATOR_CONFLICTING_SOURCE ` | 来源定位冲突 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:78](../../compiler/scoop/src/diagnostic.rs#L78) |
| 45 | ` SCOOP_LOCATOR_COORDINATE_MISMATCH ` | 依赖坐标不符 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:77](../../compiler/scoop/src/diagnostic.rs#L77) |
| 46 | ` SCOOP_LOCATOR_NOT_FOUND ` | 未找到依赖产物 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:75](../../compiler/scoop/src/diagnostic.rs#L75) |
| 47 | ` SCOOP_LOCATOR_WRONG_TYPE ` | 依赖产物类型不符 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:76](../../compiler/scoop/src/diagnostic.rs#L76) |
| 48 | ` SCOOP_NEW_INPUT ` | 新项目输入错误 | 结构化诊断代码/声明 | [scoop/src/cli/new.rs:105](../../compiler/scoop/src/cli/new.rs#L105) |
| 49 | ` SCOOP_NEW_IO ` | 新项目写入失败 | 结构化诊断代码/声明 | [scoop/src/cli/new.rs:109](../../compiler/scoop/src/cli/new.rs#L109) |
| 50 | ` SCOOP_OUTPUT_LAYOUT ` | 输出目录/布局错误 | 结构化诊断代码/声明 | [scoop/src/build.rs:279](../../compiler/scoop/src/build.rs#L279)；[scoop/src/build.rs:289](../../compiler/scoop/src/build.rs#L289) |
| 51 | ` SCOOP_OUTPUT_WRITE ` | 输出写入失败 | 结构化诊断代码/声明 | [scoop/src/build.rs:348](../../compiler/scoop/src/build.rs#L348) |
| 52 | ` SCOOP_PACKAGE_PREPARE ` | 包管理准备失败 | 结构化诊断代码/声明 | [pm/src/error.rs:79](../../compiler/pm/src/error.rs#L79) |
| 53 | ` SCOOP_PREBUILT_STALE_DEPENDENCY ` | 预编译依赖过期 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:94](../../compiler/scoop/src/diagnostic.rs#L94) |
| 54 | ` SCOOP_PREBUILT_SUMMARY_MISMATCH ` | 预编译摘要不一致 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:92](../../compiler/scoop/src/diagnostic.rs#L92) |
| 55 | ` SCOOP_PREBUILT_VIEW_INVALID ` | 预编译视图无效 | 结构化诊断代码/声明 | [scoop/src/diagnostic.rs:93](../../compiler/scoop/src/diagnostic.rs#L93) |
| 56 | ` SCOOP_PROCESS_INIT ` | 进程信号处理初始化失败 | 结构化诊断代码/声明 | [scoop/src/cli.rs:58](../../compiler/scoop/src/cli.rs#L58) |
| 57 | ` SCOOP_PROGRAM_START ` | 生成程序启动失败 | 结构化诊断代码/声明 | [scoop/src/cli/run.rs:41](../../compiler/scoop/src/cli/run.rs#L41) |
| 58 | ` SCOOP_PROJECT ` | LSP 项目配置诊断 | 结构化诊断代码/声明 | [lsp/src/server/workspace.rs:60](../../compiler/lsp/src/server/workspace.rs#L60) |
| 59 | ` SCOOP_RUNTIME_BUILD ` | runtime 构建失败 | 结构化诊断代码/声明 | [scoop/src/build.rs:192](../../compiler/scoop/src/build.rs#L192) |
| 60 | ` SCOOP_RUN_LIBRARY ` | 不能把库作为程序运行 | 结构化诊断代码/声明 | [scoop/src/build.rs:64](../../compiler/scoop/src/build.rs#L64) |
| 61 | ` SCOOP_RUN_NATIVE_APP ` | native-app 运行方式不合法 | 结构化诊断代码/声明 | [scoop/src/build.rs:71](../../compiler/scoop/src/build.rs#L71) |
| 62 | ` SCOOP_TEST_RUN ` | 测试进程/结果错误 | 结构化诊断代码/声明 | [scoop/src/cli/test_run.rs:117](../../compiler/scoop/src/cli/test_run.rs#L117) |
| 63 | ` SLIB_ALLOCATION ` | 内存分配失败 | 产物诊断标识 | [slib/src/diagnostic.rs:73](../../compiler/slib/src/diagnostic.rs#L73) |
| 64 | ` SLIB_BRIDGE_MISMATCH ` | 跨阶段 bridge 不匹配 | 产物诊断标识 | [slib/src/diagnostic.rs:92](../../compiler/slib/src/diagnostic.rs#L92) |
| 65 | ` SLIB_CAPABILITY_INVALID ` | 产物能力记录无效 | 产物诊断标识 | [slib/src/diagnostic.rs:76](../../compiler/slib/src/diagnostic.rs#L76) |
| 66 | ` SLIB_CAPABILITY_NATIVE_BOUNDARY_CLOSURE_REQUIRED ` | 需要 native boundary 闭包能力 | 产物诊断标识 | [slib/src/diagnostic.rs:79](../../compiler/slib/src/diagnostic.rs#L79) |
| 67 | ` SLIB_CAPABILITY_UNSUPPORTED ` | 产物能力不支持 | 产物诊断标识 | [slib/src/diagnostic.rs:77](../../compiler/slib/src/diagnostic.rs#L77) |
| 68 | ` SLIB_COMPAT_MANGLING ` | 符号编码规则不相容 | 产物诊断标识 | [slib/src/diagnostic.rs:82](../../compiler/slib/src/diagnostic.rs#L82) |
| 69 | ` SLIB_COMPAT_PROFILE ` | 目标/backend/profile 不相容 | 产物诊断标识 | [slib/src/diagnostic.rs:83](../../compiler/slib/src/diagnostic.rs#L83) |
| 70 | ` SLIB_COMPAT_SCHEMA ` | schema 不相容 | 产物诊断标识 | [slib/src/diagnostic.rs:81](../../compiler/slib/src/diagnostic.rs#L81) |
| 71 | ` SLIB_CONTAINER_BAD_MAGIC ` | 归档 magic 错误 | 产物诊断标识 | [slib/src/diagnostic.rs:59](../../compiler/slib/src/diagnostic.rs#L59) |
| 72 | ` SLIB_CONTAINER_INVALID_LENGTH ` | 归档长度无效 | 产物诊断标识 | [slib/src/diagnostic.rs:62](../../compiler/slib/src/diagnostic.rs#L62) |
| 73 | ` SLIB_CONTAINER_NON_CANONICAL ` | 归档非规范编码 | 产物诊断标识 | [slib/src/diagnostic.rs:61](../../compiler/slib/src/diagnostic.rs#L61) |
| 74 | ` SLIB_CONTAINER_TRUNCATED ` | 归档截断 | 产物诊断标识 | [slib/src/diagnostic.rs:60](../../compiler/slib/src/diagnostic.rs#L60) |
| 75 | ` SLIB_DIRECTORY_MISMATCH ` | 产物目录不匹配 | 产物诊断标识 | [slib/src/diagnostic.rs:74](../../compiler/slib/src/diagnostic.rs#L74) |
| 76 | ` SLIB_DIRECTORY_NON_CANONICAL_ORDER ` | 产物目录顺序不规范 | 产物诊断标识 | [slib/src/diagnostic.rs:75](../../compiler/slib/src/diagnostic.rs#L75) |
| 77 | ` SLIB_FINGERPRINT_MISMATCH ` | 语义指纹不匹配 | 产物诊断标识 | [slib/src/diagnostic.rs:93](../../compiler/slib/src/diagnostic.rs#L93) |
| 78 | ` SLIB_IDENTITY_CYCLE ` | 实体身份引用有环 | 产物诊断标识 | [slib/src/diagnostic.rs:87](../../compiler/slib/src/diagnostic.rs#L87) |
| 79 | ` SLIB_IDENTITY_DUPLICATE ` | 实体身份重复 | 产物诊断标识 | [slib/src/diagnostic.rs:85](../../compiler/slib/src/diagnostic.rs#L85) |
| 80 | ` SLIB_IDENTITY_INVALID ` | 实体身份无效 | 产物诊断标识 | [slib/src/diagnostic.rs:88](../../compiler/slib/src/diagnostic.rs#L88) |
| 81 | ` SLIB_IDENTITY_MISMATCH ` | 实体身份不匹配 | 产物诊断标识 | [slib/src/diagnostic.rs:84](../../compiler/slib/src/diagnostic.rs#L84) |
| 82 | ` SLIB_IDENTITY_MISSING ` | 实体身份缺失 | 产物诊断标识 | [slib/src/diagnostic.rs:86](../../compiler/slib/src/diagnostic.rs#L86) |
| 83 | ` SLIB_INPUT_IO ` | 产物输入 I/O | 产物诊断标识 | [slib/src/diagnostic.rs:58](../../compiler/slib/src/diagnostic.rs#L58) |
| 84 | ` SLIB_REFERENCE_FUTURE_LAYER ` | 引用了后续层 | 产物诊断标识 | [slib/src/diagnostic.rs:90](../../compiler/slib/src/diagnostic.rs#L90) |
| 85 | ` SLIB_REFERENCE_INVALID ` | 引用无效 | 产物诊断标识 | [slib/src/diagnostic.rs:91](../../compiler/slib/src/diagnostic.rs#L91) |
| 86 | ` SLIB_REFERENCE_MISSING ` | 引用缺失 | 产物诊断标识 | [slib/src/diagnostic.rs:89](../../compiler/slib/src/diagnostic.rs#L89) |
| 87 | ` SLIB_SESSION_CONFLICT ` | 导入会话冲突 | 产物诊断标识 | [slib/src/diagnostic.rs:94](../../compiler/slib/src/diagnostic.rs#L94) |
| 88 | ` SLIB_WIRE_INDEFINITE_LENGTH ` | 不允许的无限定长度编码 | 产物诊断标识 | [slib/src/diagnostic.rs:65](../../compiler/slib/src/diagnostic.rs#L65) |
| 89 | ` SLIB_WIRE_INTEGER_OUT_OF_RANGE ` | wire 整数越界 | 产物诊断标识 | [slib/src/diagnostic.rs:71](../../compiler/slib/src/diagnostic.rs#L71) |
| 90 | ` SLIB_WIRE_INVALID_FIELD ` | wire 字段错误 | 产物诊断标识 | [slib/src/diagnostic.rs:68](../../compiler/slib/src/diagnostic.rs#L68) |
| 91 | ` SLIB_WIRE_INVALID_LENGTH ` | wire 长度错误 | 产物诊断标识 | [slib/src/diagnostic.rs:70](../../compiler/slib/src/diagnostic.rs#L70) |
| 92 | ` SLIB_WIRE_NON_CANONICAL ` | wire 非规范编码 | 产物诊断标识 | [slib/src/diagnostic.rs:66](../../compiler/slib/src/diagnostic.rs#L66) |
| 93 | ` SLIB_WIRE_TRAILING_DATA ` | wire 尾随数据 | 产物诊断标识 | [slib/src/diagnostic.rs:67](../../compiler/slib/src/diagnostic.rs#L67) |
| 94 | ` SLIB_WIRE_UNEXPECTED_END ` | wire 数据提前结束 | 产物诊断标识 | [slib/src/diagnostic.rs:63](../../compiler/slib/src/diagnostic.rs#L63) |
| 95 | ` SLIB_WIRE_UNKNOWN_TAG ` | wire 未知 tag | 产物诊断标识 | [slib/src/diagnostic.rs:69](../../compiler/slib/src/diagnostic.rs#L69) |
| 96 | ` SLIB_WIRE_WRONG_TYPE ` | wire 类型错误 | 产物诊断标识 | [slib/src/diagnostic.rs:64](../../compiler/slib/src/diagnostic.rs#L64) |
