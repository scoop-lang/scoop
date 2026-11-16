# 已有诊断标识的归并方向

这 **96 个旧标识** 已全部列出。表中保留旧标识到规则编号的归并方向；当前实现已经在构造点和 typed error 边界使用新编号。一个旧阶段码可对应多个新规则，必须依据 typed error/调用边界分流；不能按旧字符串做一对一重命名。

项目尚未发布，本次直接更新编号与消费者，不保留旧编号的兼容 alias 或双轨报告。外部 generator/LLVM/系统 linker 自身的编号保留来源，不冒充 Scoop E 编号。

| 旧标识 | 候选归属 | 判定说明 |
| --- | --- | --- |
| `SCOOPC_BUILD_FAILED` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOPC_CAPABILITY_ODR_UNAVAILABLE` | [E2411](RULES.md#e2411) | 保留所需 capability 作为参数。 |
| `SCOOPC_CHILD_REQUEST_INVALID` | [E2502](RULES.md#e2502) / [ICE2501](RULES.md#ice2501) | 外部首次消费边界用 E2502；当前编译器产出或已接受且未变的数据在内部失配用 ICE2501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SCOOPC_COMPILER_WARNING` | W09/W18/W20：按规则分流 | 应按具体 warning 分流到相应的 W 规则，本行代表链接不表示所有警告改为 W1801。 |
| `SCOOPC_HIR_ERROR` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOPC_PACKAGE_ERROR` | [E2007](RULES.md#e2007) | package 必须等于 group 或其子 package。 |
| `SCOOPC_PARSE_ERROR` | E01/E02 及实际声明规则 | 这是现有的粗粒度阶段码；需按词法、语法或实际声明规则分到目录各项，E0201 仅用于 expected/found。 |
| `SCOOP_BUILD_FAILED` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_CACHE_IO` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_CACHE_LOCK` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_CACHE_PUBLISH` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_CHECK` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_CHILD_EXIT` | [E2203](RULES.md#e2203) | 若响应含原编译诊断则透传；退出状态与响应矛盾用协议错误。 |
| `SCOOP_CHILD_PROTOCOL` | [E2502](RULES.md#e2502) / [ICE2501](RULES.md#ice2501) | 外部首次消费边界用 E2502；当前编译器产出或已接受且未变的数据在内部失配用 ICE2501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SCOOP_CHILD_RESPONSE_MISMATCH` | [E2502](RULES.md#e2502) / [ICE2501](RULES.md#ice2501) | 外部首次消费边界用 E2502；当前编译器产出或已接受且未变的数据在内部失配用 ICE2501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SCOOP_CHILD_TOOL_MISMATCH` | [E2501](RULES.md#e2501) | scoopc/package/LSP 协议版本或协商能力不匹配。 |
| `SCOOP_CHILD_TRANSPORT` | [E2204](RULES.md#e2204) | 管道、frame 读写及 wait 失败；失败内容保留系统原因。 |
| `SCOOP_CLEAN_IO` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_CLI_ARGS` | [E2001](RULES.md#e2001) | CLI mode/operand/selection 与当前操作不兼容；具体选项和值作原因。 |
| `SCOOP_CLI_CONFIG` | [E2001](RULES.md#e2001) | CLI mode/operand/selection 与当前操作不兼容；具体选项和值作原因。 |
| `SCOOP_COMMAND_START` | [E2202](RULES.md#e2202) | compiler、generator、Git、native 工具等启动失败。 |
| `SCOOP_DOCUMENTATION` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_DOC_RESOURCE` | [W1813](RULES.md#w1813) | README 本地资源不存在的既有 warning；实际读取失败使用 E2201，资源配置错误使用 E1801。 |
| `SCOOP_FMT_INPUT` | [E2002](RULES.md#e2002) | Cone 目录/Cone.toml/.scoop/regular file 要求不满足；普通读取失败另列 I/O。 |
| `SCOOP_FMT_IO` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_FMT_PARSE` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_GRAPH_ALLOCATION` | [E2205](RULES.md#e2205) | 真实 allocation/索引/固定表示位宽不足；不引入 CPU/内存预算或计费。 |
| `SCOOP_GRAPH_CYCLE` | [E2105](RULES.md#e2105) | 含直接自依赖、回到当前 Cone 的边和间接环。 |
| `SCOOP_GRAPH_MISSING_CORE` | [E2101](RULES.md#e2101) | 显式依赖、sysroot artifact、已锁包记录未找到。 |
| `SCOOP_GRAPH_MULTIPLE_VERSIONS` | [E2104](RULES.md#e2104) | 同一 package 只能选一个精确版本。 |
| `SCOOP_GRAPH_NON_LIBRARY_DEPENDENCY` | [E2103](RULES.md#e2103) | 依赖必须是 manifest library，不能是 executable/native-app/single-file 节点。 |
| `SCOOP_GRAPH_RESERVED_IDENTITY` | [E2011](RULES.md#e2011) | 若内部已解析图的身份键不一致，用内部驱动/身份码。 |
| `SCOOP_GRAPH_SINGLE_FILE_DEPENDENCY` | [E2103](RULES.md#e2103) | 依赖必须是 manifest library，不能是 executable/native-app/single-file 节点。 |
| `SCOOP_GRAPH_UNREACHABLE_NODE` | [ICE2201](RULES.md#ice2201) | 按实际内部合同进一步分流 ICE04/05/22/25；用户提供非法输出路径应使用 E20。 |
| `SCOOP_HIR_CROSS_CONE_DISPATCH_REQUIRED` | [E2411](RULES.md#e2411) | 保留所需 capability 作为参数。 |
| `SCOOP_HIR_CROSS_CONE_GENERIC_REQUIRED` | [E2411](RULES.md#e2411) | 保留所需 capability 作为参数。 |
| `SCOOP_HIR_CROSS_CONE_LAYOUT_REQUIRED` | [E2411](RULES.md#e2411) | 保留所需 capability 作为参数。 |
| `SCOOP_HIR_CROSS_CONE_NATIVE_REQUIRED` | [E2411](RULES.md#e2411) | 保留所需 capability 作为参数。 |
| `SCOOP_INTERNAL_ERROR` | [ICE2201](RULES.md#ice2201) | 按实际内部合同进一步分流 ICE04/05/22/25；用户提供非法输出路径应使用 E20。 |
| `SCOOP_LINK_FAILED` | [E2304](RULES.md#e2304) | 只承载系统 linker 失败；前置 reader/ABI/native 错误保留更具体主码。 |
| `SCOOP_LINK_TOOLCHAIN` | [E2303](RULES.md#e2303) | 按缺工具、目标无支持、ABI/profile 不符、I/O 等实际原因细分。 |
| `SCOOP_LOCATOR_AMBIGUOUS_ARTIFACT` | [E2106](RULES.md#e2106) | 同一身份的 source/artifact/locator/fingerprint 不唯一。 |
| `SCOOP_LOCATOR_CONFLICTING_REPRESENTATION` | [E2106](RULES.md#e2106) | 同一身份的 source/artifact/locator/fingerprint 不唯一。 |
| `SCOOP_LOCATOR_CONFLICTING_SOURCE` | [E2106](RULES.md#e2106) | 同一身份的 source/artifact/locator/fingerprint 不唯一。 |
| `SCOOP_LOCATOR_COORDINATE_MISMATCH` | [E2102](RULES.md#e2102) | source/prebuilt/registry 选中的坐标不符合请求。 |
| `SCOOP_LOCATOR_NOT_FOUND` | [E2101](RULES.md#e2101) | 显式依赖、sysroot artifact、已锁包记录未找到。 |
| `SCOOP_LOCATOR_WRONG_TYPE` | [E2103](RULES.md#e2103) | 依赖必须是 manifest library，不能是 executable/native-app/single-file 节点。 |
| `SCOOP_NEW_INPUT` | [E2001](RULES.md#e2001) | CLI mode/operand/selection 与当前操作不兼容；具体选项和值作原因。 |
| `SCOOP_NEW_IO` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_OUTPUT_LAYOUT` | [ICE2201](RULES.md#ice2201) | 按实际内部合同进一步分流 ICE04/05/22/25；用户提供非法输出路径应使用 E20。 |
| `SCOOP_OUTPUT_WRITE` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_PACKAGE_PREPARE` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_PREBUILT_STALE_DEPENDENCY` | [E2108](RULES.md#e2108) | SLIB 自身摘要字段损坏/不一致也可能是产物格式或 identity 错误，须保留实际检查对象。 |
| `SCOOP_PREBUILT_SUMMARY_MISMATCH` | [E2107](RULES.md#e2107) | direct/support 集合缺失、多余或边关系不同；编译器自己构造的图违约使用 ICE，不归咎源码。 |
| `SCOOP_PREBUILT_VIEW_INVALID` | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 外部首次消费边界用 E2406；当前编译器产出或已接受且未变的数据在内部失配用 ICE0501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SCOOP_PROCESS_INIT` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SCOOP_PROGRAM_START` | [E2202](RULES.md#e2202) | compiler、generator、Git、native 工具等启动失败。 |
| `SCOOP_PROJECT` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_RUNTIME_BUILD` | 仅转发/汇总/包装 | 阶段/命令包装。按底层 typed error 选择 E/W/ICE，不能整体换成一个总失败码。 |
| `SCOOP_RUN_LIBRARY` | [E2001](RULES.md#e2001) | CLI mode/operand/selection 与当前操作不兼容；具体选项和值作原因。 |
| `SCOOP_RUN_NATIVE_APP` | [E2001](RULES.md#e2001) | CLI mode/operand/selection 与当前操作不兼容；具体选项和值作原因。 |
| `SCOOP_TEST_RUN` | [E2508](RULES.md#e2508) | 启动/协议失败分别使用 E22/E25，已执行的用例失败使用 test-run。 |
| `SLIB_ALLOCATION` | [E2205](RULES.md#e2205) | 真实 allocation/索引/固定表示位宽不足；不引入 CPU/内存预算或计费。 |
| `SLIB_BRIDGE_MISMATCH` | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 根据 bridge 的 HIR/MIR/LIR 合同可细分 E2406/E2407/E2408；当前自产 bridge 使用对应 ICE。 |
| `SLIB_CAPABILITY_INVALID` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_CAPABILITY_NATIVE_BOUNDARY_CLOSURE_REQUIRED` | [E2411](RULES.md#e2411) | source-only/layout/dispatch/generic/native/Strong profile 能力不足；不是“尚未实现”的通用占位码。 |
| `SLIB_CAPABILITY_UNSUPPORTED` | [E2411](RULES.md#e2411) | source-only/layout/dispatch/generic/native/Strong profile 能力不足；不是“尚未实现”的通用占位码。 |
| `SLIB_COMPAT_MANGLING` | [E2402](RULES.md#e2402) | 已有产物版本与当前 reader 不兼容。 |
| `SLIB_COMPAT_PROFILE` | [E2404](RULES.md#e2404) | 目标机器、完整 target、profile 或固定 ABI 版本不符合本次消费要求。 |
| `SLIB_COMPAT_SCHEMA` | [E2402](RULES.md#e2402) | 已有产物版本与当前 reader 不兼容。 |
| `SLIB_CONTAINER_BAD_MAGIC` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_CONTAINER_INVALID_LENGTH` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_CONTAINER_NON_CANONICAL` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_CONTAINER_TRUNCATED` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_DIRECTORY_MISMATCH` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_DIRECTORY_NON_CANONICAL_ORDER` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| `SLIB_FINGERPRINT_MISMATCH` | [E2108](RULES.md#e2108) | SLIB 自身摘要字段损坏/不一致也可能是产物格式或 identity 错误，须保留实际检查对象。 |
| `SLIB_IDENTITY_CYCLE` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_IDENTITY_DUPLICATE` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_IDENTITY_INVALID` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_IDENTITY_MISMATCH` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_IDENTITY_MISSING` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_INPUT_IO` | [E2201](RULES.md#e2201) | 仅实际系统 I/O/初始化失败；锁中毒或自产状态错误归 ICE。 |
| `SLIB_REFERENCE_FUTURE_LAYER` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_REFERENCE_INVALID` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_REFERENCE_MISSING` | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| `SLIB_SESSION_CONFLICT` | [ICE0401](RULES.md#ice0401) | 若源于两个外部 artifact 声明冲突则用 dependency-ambiguous/artifact-odr；内部 session 混用才是 ICE。 |
| `SLIB_WIRE_INDEFINITE_LENGTH` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_INTEGER_OUT_OF_RANGE` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_INVALID_FIELD` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_INVALID_LENGTH` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_NON_CANONICAL` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_TRAILING_DATA` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_UNEXPECTED_END` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_UNKNOWN_TAG` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
| `SLIB_WIRE_WRONG_TYPE` | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |
