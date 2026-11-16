# 原清单到规则编号的完整对照

本表覆盖初稿 **5,960 条错误相关文案**。F/B/A/D/T 分别指前端、后端、产物、构建和工具清单中的原行号；它们不是正式诊断码。区间包含两端，点击起点可回到原英文文案和全部源码位置。

`主/因` 表示同一规则可在直接触发处作主诊断，也可作为候选拒绝原因；`因` 表示该文案是明确的解释片段。`按边界` 必须按下表选择，斜线不表示同时报出两个主错误。本表保留初稿的逐条归并证据；接入后的边界校正见文末，以规则目录与实际 typed error 构造点为准。

## 共用校验器的判定边界

| 边界组 | 候选编号 | 判定条件 |
| --- | --- | --- |
| 格式校验的读写边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 外部首次消费边界用 E2401；当前编译器产出或已接受且未变的数据在内部失配用 ICE2401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| 实体/源码引用校验的边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 外部首次消费边界用 E2405；当前编译器产出或已接受且未变的数据在内部失配用 ICE0401。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 若子错误来自源码名称/manifest 字段解析，保留其 E01/E20 主规则，不改报产物错误。 |
| 声明接口校验/投影的边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 外部首次消费边界用 E2406；当前编译器产出或已接受且未变的数据在内部失配用 ICE0501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| 可执行模板/MIR 校验的边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 外部首次消费边界用 E2407；当前编译器产出或已接受且未变的数据在内部失配用 ICE0701。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| 布局/ABI 校验的边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 外部首次消费边界用 E2408；当前编译器产出或已接受且未变的数据在内部失配用 ICE1201。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| native 记录读写的边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | 外部首次消费边界用 E2409；当前编译器产出或已接受且未变的数据在内部失配用 ICE2302。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| target 合同的消费/内部边界 | [E2404](RULES.md#e2404) / [ICE2201](RULES.md#ice2201) | 外部首次消费边界用 E2404；当前编译器产出或已接受且未变的数据在内部失配用 ICE2201。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| 工具协议消息的收发边界 | [E2502](RULES.md#e2502) / [ICE2501](RULES.md#ice2501) | 外部首次消费边界用 E2502；当前编译器产出或已接受且未变的数据在内部失配用 ICE2501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| 查询输入与内部查询状态边界 | [E2505](RULES.md#e2505) / [ICE2502](RULES.md#ice2502) | 外部首次消费边界用 E2505；当前编译器产出或已接受且未变的数据在内部失配用 ICE2502。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| core 协议消费与派生内部状态 | [E1404](RULES.md#e1404) / [ICE0501](RULES.md#ice0501) | 外部首次消费边界用 E1404；当前编译器产出或已接受且未变的数据在内部失配用 ICE0501。具体字段/变体仅作为原因；I/O、资源与已有子诊断继续透传。 |
| wire 作为产物/协议子错误的边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | 在外部 .slib 中作为产物格式原因，在外部工具消息中作为协议 payload 原因；自身编码产生非法值则为 ICE。不能仅凭 WireError 类型确定主码。 |

**wrapper 优先保留下游码。** 跨 Cone 选择/实例化返回的错误若已包含源码诊断、I/O、资源或能力错误，应直接透传对应码；只有真正违反本表所述数据合同的 leaf reason 才使用边界组的主码。第一处外部数据解码可以延迟到实际使用时，不能因发生在 HIR/MIR 阶段就自动判成 ICE。

## 不新分配错误码的项目

| 处置 | 含义 | 原条目数量 |
| --- | --- | --- |
| 运行时异常文字 | 这些字面量被嵌入生成的 Scoop 代码，属于应用运行时错误，不是本次编译诊断。 | 3 |
| 上下文标签 | 传给共用检查器的上下文名称，不是单独发出的错误。 | 1 |
| 仅转发/汇总/包装 | 保留下游规则编号与定位；外层阶段、声明名、数量和操作名作为上下文，不新增错误码。 | 59 |
| 依赖已失败或输入不完整的后继状态 | 不是新根因。保留已有根错误的关联信息；编辑器因不完整输入不能回答查询时走查询状态，不伪造语言错误。 | 8 |
| 取消/中断状态 | 用户或协议取消请求属于完成状态，不建议分配编译错误码；CLI 可返回相应退出状态。 | 3 |
| 应用测试失败的打印片段 | 测试报告中的运行时文字，占位位置行，不是编译器诊断模板。 | 1 |
| 位置/候选链的排版片段 | 只是输出格式，不独立编号；语义内容分别归相关位置或候选原因。 | 11 |
| warning 被当前策略视为失败 | 不额外发一个遮盖原 warning 的主错误；保留 W 编号，渲染/退出状态注明 warnings denied。 | 1 |
| 名义类型不变性的解释 | 解释 type-mismatch 的第一个不同类型实参；拆成附注和显式转换建议，不新增主错误。 | 1 |
| 离线求解范围说明 | 说明仅检查本地元数据/Git objects；附在求解失败后，不是另一错误。 | 1 |
| API 数据失败的恢复建议 | 底层错误继续保留；scoop doc / scoop-install docs 建议归 H。 | 1 |

## F · FRONTEND.md

| 原条目 | 条数 | 角色 | 候选编号/处置 | 归属 |
| --- | --- | --- | --- | --- |
| [F0001](FRONTEND.md#f0001) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0002](FRONTEND.md#f0002) | 1 | 主/因 | [E0608](RULES.md#e0608) | 类型别名循环 |
| [F0003](FRONTEND.md#f0003) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0004](FRONTEND.md#f0004)–F0007 | 4 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0008](FRONTEND.md#f0008)–F0010 | 3 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0011](FRONTEND.md#f0011) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0012](FRONTEND.md#f0012) | 1 | 主/因 | [E1302](RULES.md#e1302) | 非 repeatable 注解被重复使用 |
| [F0013](FRONTEND.md#f0013) | 1 | 主/因 | [E1208](RULES.md#e1208) | raw/extern 存储方式不合法 |
| [F0014](FRONTEND.md#f0014) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F0015](FRONTEND.md#f0015)–F0016 | 2 | 主/因 | [E1203](RULES.md#e1203) | Extern 声明形态不合法 |
| [F0017](FRONTEND.md#f0017)–F0020 | 4 | 主/因 | [E1208](RULES.md#e1208) | raw/extern 存储方式不合法 |
| [F0021](FRONTEND.md#f0021) | 1 | 主/因 | [E1212](RULES.md#e1212) | raw 初始化不是合格常量 |
| [F0022](FRONTEND.md#f0022)–F0023 | 2 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F0024](FRONTEND.md#f0024) | 1 | 主/因 | [E1303](RULES.md#e1303) | 注解组合冲突 |
| [F0025](FRONTEND.md#f0025)–F0026 | 2 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F0027](FRONTEND.md#f0027) | 1 | 主/因 | [E1401](RULES.md#e1401) | 未知 intrinsic 标识 |
| [F0028](FRONTEND.md#f0028) | 1 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0029](FRONTEND.md#f0029) | 1 | 主/因 | [E1402](RULES.md#e1402) | intrinsic 声明位置不合法 |
| [F0030](FRONTEND.md#f0030)–F0033 | 4 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F0034](FRONTEND.md#f0034)–F0035 | 2 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0036](FRONTEND.md#f0036) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F0037](FRONTEND.md#f0037) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0038](FRONTEND.md#f0038) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F0039](FRONTEND.md#f0039) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0040](FRONTEND.md#f0040) | 1 | 主/因 | [E0703](RULES.md#e0703) | 形参被重复传入 |
| [F0041](FRONTEND.md#f0041) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0042](FRONTEND.md#f0042) | 1 | 主/因 | [E1304](RULES.md#e1304) | 注解值不是允许的常量形式 |
| [F0043](FRONTEND.md#f0043) | 1 | 主/因 | [E1207](RULES.md#e1207) | CLayout 对齐值无效 |
| [F0044](FRONTEND.md#f0044) | 1 | 主/因 | [E1118](RULES.md#e1118) | 效果注解组合不合法 |
| [F0045](FRONTEND.md#f0045) | 1 | 主/因 | [E1302](RULES.md#e1302) | 非 repeatable 注解被重复使用 |
| [F0046](FRONTEND.md#f0046) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F0047](FRONTEND.md#f0047) | 1 | 主/因 | [E0703](RULES.md#e0703) | 形参被重复传入 |
| [F0048](FRONTEND.md#f0048) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F0049](FRONTEND.md#f0049) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0050](FRONTEND.md#f0050) | 1 | 主/因 | [E0703](RULES.md#e0703) | 形参被重复传入 |
| [F0051](FRONTEND.md#f0051) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0052](FRONTEND.md#f0052)–F0053 | 2 | 主/因 | [E1206](RULES.md#e1206) | native 库或符号名称不合法 |
| [F0054](FRONTEND.md#f0054) | 1 | 主/因 | [E1204](RULES.md#e1204) | 不支持的 native ABI/调用约定 |
| [F0055](FRONTEND.md#f0055) | 1 | 主/因 | [E1401](RULES.md#e1401) | 未知 intrinsic 标识 |
| [F0056](FRONTEND.md#f0056) | 1 | 主/因 | [E1402](RULES.md#e1402) | intrinsic 声明位置不合法 |
| [F0057](FRONTEND.md#f0057) | 1 | 主/因 | [E1204](RULES.md#e1204) | 不支持的 native ABI/调用约定 |
| [F0058](FRONTEND.md#f0058) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F0059](FRONTEND.md#f0059) | 1 | 主/因 | [E1118](RULES.md#e1118) | 效果注解组合不合法 |
| [F0060](FRONTEND.md#f0060)–F0067 | 8 | 主/因 | [E1203](RULES.md#e1203) | Extern 声明形态不合法 |
| [F0068](FRONTEND.md#f0068)–F0070 | 3 | 主/因 | [E1205](RULES.md#e1205) | 调用约定注解使用条件不符 |
| [F0071](FRONTEND.md#f0071)–F0072 | 2 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0073](FRONTEND.md#f0073) | 1 | 主/因 | [E1303](RULES.md#e1303) | 注解组合冲突 |
| [F0074](FRONTEND.md#f0074) | 1 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0075](FRONTEND.md#f0075) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0076](FRONTEND.md#f0076) | 1 | 主/因 | [E0607](RULES.md#e0607) | 缺少足够的类型推断信息 |
| [F0077](FRONTEND.md#f0077) | 1 | 主/因 | [E1102](RULES.md#e1102) | unsafe 函数不能存入普通托管函数值 |
| [F0078](FRONTEND.md#f0078) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0079](FRONTEND.md#f0079) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F0080](FRONTEND.md#f0080) | 1 | 主/因 | [E0703](RULES.md#e0703) | 形参被重复传入 |
| [F0081](FRONTEND.md#f0081) | 1 | 主/因 | [E0704](RULES.md#e0704) | 实参顺序不满足映射规则 |
| [F0082](FRONTEND.md#f0082) | 1 | 主/因 | [E0705](RULES.md#e0705) | 向仅命名形参传位置实参 |
| [F0083](FRONTEND.md#f0083) | 1 | 主/因 | [E0706](RULES.md#e0706) | 向仅位置形参传命名实参 |
| [F0084](FRONTEND.md#f0084) | 1 | 主/因 | [E0707](RULES.md#e0707) | 非 vararg 位置使用 spread |
| [F0085](FRONTEND.md#f0085) | 1 | 主/因 | [E0708](RULES.md#e0708) | 缺少必填实参 |
| [F0086](FRONTEND.md#f0086) | 1 | 主/因 | [E0709](RULES.md#e0709) | 混合互斥的 vararg 输入形式 |
| [F0087](FRONTEND.md#f0087) | 1 | 主/因 | [E0710](RULES.md#e0710) | 尾随 lambda 不能映射到 vararg |
| [F0088](FRONTEND.md#f0088) | 1 | 主/因 | [E0732](RULES.md#e0732) | 变体构造形式不合法 |
| [F0089](FRONTEND.md#f0089) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0090](FRONTEND.md#f0090) | 1 | 抑制/附注 | 依赖已失败或输入不完整的后继状态 | 依赖已失败或输入不完整的后继状态 |
| [F0091](FRONTEND.md#f0091) | 1 | 主/因 | [E0602](RULES.md#e0602) | 不满足 kind bound |
| [F0092](FRONTEND.md#f0092) | 1 | 主/因 | [E0604](RULES.md#e0604) | 违反负接口约束 |
| [F0093](FRONTEND.md#f0093)–F0094 | 2 | 主/因 | [E0603](RULES.md#e0603) | 不满足名义上界 |
| [F0095](FRONTEND.md#f0095) | 1 | 主/因 | [E0605](RULES.md#e0605) | 类型推断约束冲突 |
| [F0096](FRONTEND.md#f0096) | 1 | 主/因 | [E0606](RULES.md#e0606) | 类型推断存在多个解 |
| [F0097](FRONTEND.md#f0097) | 1 | 主/因 | [E0607](RULES.md#e0607) | 缺少足够的类型推断信息 |
| [F0098](FRONTEND.md#f0098) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0099](FRONTEND.md#f0099) | 1 | 主/因 | [E0502](RULES.md#e0502) | 表达式不是可调用类型 |
| [F0100](FRONTEND.md#f0100) | 1 | 主/因 | [E0504](RULES.md#e0504) | 普通与 suspend 函数类型不相容 |
| [F0101](FRONTEND.md#f0101) | 1 | 主/因 | [E0503](RULES.md#e0503) | 函数类型的参数形状不符 |
| [F0102](FRONTEND.md#f0102)–F0105 | 4 | 内部 | [ICE0601](RULES.md#ice0601) | 类型求解器状态不一致 |
| [F0106](FRONTEND.md#f0106) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0107](FRONTEND.md#f0107) | 1 | 主/因 | [E0712](RULES.md#e0712) | 可调用候选无法唯一选择 |
| [F0108](FRONTEND.md#f0108) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0109](FRONTEND.md#f0109) | 1 | 主/因 | [E0604](RULES.md#e0604) | 违反负接口约束 |
| [F0110](FRONTEND.md#f0110) | 1 | 主/因 | [E0727](RULES.md#e0727) | operator/infix 不允许 vararg |
| [F0111](FRONTEND.md#f0111)–F0112 | 2 | 主/因 | [E0725](RULES.md#e0725) | operator/infix 声明缺少 receiver |
| [F0113](FRONTEND.md#f0113) | 1 | 主/因 | [E0726](RULES.md#e0726) | operator/infix 参数数量不符 |
| [F0114](FRONTEND.md#f0114) | 1 | 主/因 | [E0727](RULES.md#e0727) | operator/infix 不允许 vararg |
| [F0115](FRONTEND.md#f0115)–F0117 | 3 | 主/因 | [E0728](RULES.md#e0728) | operator 角色名称无效 |
| [F0118](FRONTEND.md#f0118) | 1 | 主/因 | [E0729](RULES.md#e0729) | operator 声明形状不符 |
| [F0119](FRONTEND.md#f0119) | 1 | 主/因 | [E0726](RULES.md#e0726) | operator/infix 参数数量不符 |
| [F0120](FRONTEND.md#f0120)–F0122 | 3 | 主/因 | [E0729](RULES.md#e0729) | operator 声明形状不符 |
| [F0123](FRONTEND.md#f0123) | 1 | 主/因 | [E0726](RULES.md#e0726) | operator/infix 参数数量不符 |
| [F0124](FRONTEND.md#f0124) | 1 | 主/因 | [E0727](RULES.md#e0727) | operator/infix 不允许 vararg |
| [F0125](FRONTEND.md#f0125)–F0126 | 2 | 主/因 | [E0726](RULES.md#e0726) | operator/infix 参数数量不符 |
| [F0127](FRONTEND.md#f0127) | 1 | 主/因 | [E0727](RULES.md#e0727) | operator/infix 不允许 vararg |
| [F0128](FRONTEND.md#f0128)–F0132 | 5 | 主/因 | [E0729](RULES.md#e0729) | operator 声明形状不符 |
| [F0133](FRONTEND.md#f0133) | 1 | 主/因 | [E1001](RULES.md#e1001) | 构造器委托路径不合法 |
| [F0134](FRONTEND.md#f0134) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0135](FRONTEND.md#f0135) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0136](FRONTEND.md#f0136)–F0140 | 5 | 主/因 | [E1001](RULES.md#e1001) | 构造器委托路径不合法 |
| [F0141](FRONTEND.md#f0141) | 1 | 排除 | 上下文标签 | 上下文标签 |
| [F0142](FRONTEND.md#f0142)–F0143 | 2 | 主/因 | [E0302](RULES.md#e0302) | 重复可调用签名 |
| [F0144](FRONTEND.md#f0144) | 1 | 主/因 | [E1002](RULES.md#e1002) | 构造器委托成环 |
| [F0145](FRONTEND.md#f0145) | 1 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0146](FRONTEND.md#f0146)–F0148 | 3 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0149](FRONTEND.md#f0149) | 1 | 主/因 | [E1003](RULES.md#e1003) | 不能显式继承根类型 |
| [F0150](FRONTEND.md#f0150)–F0151 | 2 | 主/因 | [E1004](RULES.md#e1004) | 父类型种类不合法 |
| [F0152](FRONTEND.md#f0152) | 1 | 主/因 | [E1005](RULES.md#e1005) | 不能继承 final 类型 |
| [F0153](FRONTEND.md#f0153) | 1 | 主/因 | [E1006](RULES.md#e1006) | 接口继承项不接受构造实参 |
| [F0154](FRONTEND.md#f0154) | 1 | 主/因 | [E1004](RULES.md#e1004) | 父类型种类不合法 |
| [F0155](FRONTEND.md#f0155) | 1 | 主/因 | [E1007](RULES.md#e1007) | 接口委托声明位置不合法 |
| [F0156](FRONTEND.md#f0156) | 1 | 主/因 | [E1006](RULES.md#e1006) | 接口继承项不接受构造实参 |
| [F0157](FRONTEND.md#f0157) | 1 | 主/因 | [E1004](RULES.md#e1004) | 父类型种类不合法 |
| [F0158](FRONTEND.md#f0158) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0159](FRONTEND.md#f0159) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0160](FRONTEND.md#f0160) | 1 | 主/因 | [E0715](RULES.md#e0715) | 构造器缺少外部实例或捕获值 |
| [F0161](FRONTEND.md#f0161) | 1 | 主/因 | [E1008](RULES.md#e1008) | 具体类型缺少抽象成员实现 |
| [F0162](FRONTEND.md#f0162)–F0163 | 2 | 主/因 | [E1009](RULES.md#e1009) | 继承图成环 |
| [F0164](FRONTEND.md#f0164)–F0165 | 2 | 主/因 | [E1008](RULES.md#e1008) | 具体类型缺少抽象成员实现 |
| [F0166](FRONTEND.md#f0166) | 1 | 主/因 | [E1313](RULES.md#e1313) | 自动派生有多个字段 codec 候选 |
| [F0167](FRONTEND.md#f0167) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0168](FRONTEND.md#f0168) | 1 | 主/因 | [E1312](RULES.md#e1312) | 自动派生找不到字段 codec |
| [F0169](FRONTEND.md#f0169)–F0170 | 2 | 主/因 | [E1013](RULES.md#e1013) | override 签名合同不符 |
| [F0171](FRONTEND.md#f0171) | 1 | 按边界 | [E1404](RULES.md#e1404) / [ICE0501](RULES.md#ice0501) | core 协议消费与派生内部状态 |
| [F0172](FRONTEND.md#f0172)–F0174 | 3 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0175](FRONTEND.md#f0175)–F0177 | 3 | 主/因 | [E1311](RULES.md#e1311) | 字段无法按默认规则自动派生 |
| [F0178](FRONTEND.md#f0178)–F0180 | 3 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0181](FRONTEND.md#f0181) | 1 | 主/因 | [E1314](RULES.md#e1314) | 派生序列化名称重复 |
| [F0182](FRONTEND.md#f0182) | 1 | 主/因 | [E1315](RULES.md#e1315) | 省略解码参数却没有默认值 |
| [F0183](FRONTEND.md#f0183) | 1 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0184](FRONTEND.md#f0184) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0185](FRONTEND.md#f0185) | 1 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0186](FRONTEND.md#f0186) | 1 | 排除 | 运行时异常文字 | 运行时异常文字 |
| [F0187](FRONTEND.md#f0187) | 1 | 主/因 | [E1314](RULES.md#e1314) | 派生序列化名称重复 |
| [F0188](FRONTEND.md#f0188) | 1 | 排除 | 运行时异常文字 | 运行时异常文字 |
| [F0189](FRONTEND.md#f0189) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0190](FRONTEND.md#f0190) | 1 | 主/因 | [E1010](RULES.md#e1010) | 继承实现冲突 |
| [F0191](FRONTEND.md#f0191) | 1 | 按边界 | [E1404](RULES.md#e1404) / [ICE0501](RULES.md#ice0501) | core 协议消费与派生内部状态 |
| [F0192](FRONTEND.md#f0192)–F0193 | 2 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0194](FRONTEND.md#f0194) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0195](FRONTEND.md#f0195)–F0198 | 4 | 主/因 | [E1311](RULES.md#e1311) | 字段无法按默认规则自动派生 |
| [F0199](FRONTEND.md#f0199) | 1 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0200](FRONTEND.md#f0200) | 1 | 主/因 | [E1314](RULES.md#e1314) | 派生序列化名称重复 |
| [F0201](FRONTEND.md#f0201)–F0202 | 2 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0203](FRONTEND.md#f0203) | 1 | 主/因 | [E1016](RULES.md#e1016) | 继承成员访问域合同不符 |
| [F0204](FRONTEND.md#f0204) | 1 | 主/因 | [E1013](RULES.md#e1013) | override 签名合同不符 |
| [F0205](FRONTEND.md#f0205) | 1 | 按边界 | [E1404](RULES.md#e1404) / [ICE0501](RULES.md#ice0501) | core 协议消费与派生内部状态 |
| [F0206](FRONTEND.md#f0206)–F0207 | 2 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0208](FRONTEND.md#f0208) | 1 | 主/因 | [E1311](RULES.md#e1311) | 字段无法按默认规则自动派生 |
| [F0209](FRONTEND.md#f0209) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0210](FRONTEND.md#f0210) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0211](FRONTEND.md#f0211) | 1 | 排除 | 运行时异常文字 | 运行时异常文字 |
| [F0212](FRONTEND.md#f0212) | 1 | 主/因 | [E1310](RULES.md#e1310) | 类型不满足自动派生的形状要求 |
| [F0213](FRONTEND.md#f0213) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0214](FRONTEND.md#f0214) | 1 | 主/因 | [E1507](RULES.md#e1507) | 值类型属性形态不合法 |
| [F0215](FRONTEND.md#f0215) | 1 | 主/因 | [E1010](RULES.md#e1010) | 继承实现冲突 |
| [F0216](FRONTEND.md#f0216) | 1 | 主/因 | [E1008](RULES.md#e1008) | 具体类型缺少抽象成员实现 |
| [F0217](FRONTEND.md#f0217) | 1 | 主/因 | [E1010](RULES.md#e1010) | 继承实现冲突 |
| [F0218](FRONTEND.md#f0218)–F0219 | 2 | 主/因 | [E1011](RULES.md#e1011) | override/重列接口条件不覆盖原合同 |
| [F0220](FRONTEND.md#f0220) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0221](FRONTEND.md#f0221) | 1 | 主/因 | [E1012](RULES.md#e1012) | override 没有匹配目标 |
| [F0222](FRONTEND.md#f0222) | 1 | 主/因 | [E1013](RULES.md#e1013) | override 签名合同不符 |
| [F0223](FRONTEND.md#f0223)–F0224 | 2 | 主/因 | [E1014](RULES.md#e1014) | 不能覆盖 final 成员 |
| [F0225](FRONTEND.md#f0225) | 1 | 主/因 | [E1015](RULES.md#e1015) | 覆盖成员缺少 override |
| [F0226](FRONTEND.md#f0226) | 1 | 主/因 | [E1012](RULES.md#e1012) | override 没有匹配目标 |
| [F0227](FRONTEND.md#f0227) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F0228](FRONTEND.md#f0228)–F0230 | 3 | 主/因 | [E1016](RULES.md#e1016) | 继承成员访问域合同不符 |
| [F0231](FRONTEND.md#f0231) | 1 | 主/因 | [E1011](RULES.md#e1011) | override/重列接口条件不覆盖原合同 |
| [F0232](FRONTEND.md#f0232) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F0233](FRONTEND.md#f0233) | 1 | 主/因 | [E1016](RULES.md#e1016) | 继承成员访问域合同不符 |
| [F0234](FRONTEND.md#f0234) | 1 | 主/因 | [E1012](RULES.md#e1012) | override 没有匹配目标 |
| [F0235](FRONTEND.md#f0235)–F0236 | 2 | 主/因 | [E1013](RULES.md#e1013) | override 签名合同不符 |
| [F0237](FRONTEND.md#f0237) | 1 | 主/因 | [E1014](RULES.md#e1014) | 不能覆盖 final 成员 |
| [F0238](FRONTEND.md#f0238) | 1 | 主/因 | [E1015](RULES.md#e1015) | 覆盖成员缺少 override |
| [F0239](FRONTEND.md#f0239) | 1 | 主/因 | [E1016](RULES.md#e1016) | 继承成员访问域合同不符 |
| [F0240](FRONTEND.md#f0240)–F0241 | 2 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0242](FRONTEND.md#f0242) | 1 | 主/因 | [E0810](RULES.md#e0810) | 读取尚未初始化的值 |
| [F0243](FRONTEND.md#f0243) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0244](FRONTEND.md#f0244) | 1 | 主/因 | [E0411](RULES.md#e0411) | protected 接收者不合法 |
| [F0245](FRONTEND.md#f0245) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0246](FRONTEND.md#f0246)–F0248 | 3 | 主/因 | [E1017](RULES.md#e1017) | 泛型方法声明/覆盖限制 |
| [F0249](FRONTEND.md#f0249) | 1 | 主/因 | [E0308](RULES.md#e0308) | 声明缺少必须的函数体 |
| [F0250](FRONTEND.md#f0250)–F0251 | 2 | 主/因 | [E1018](RULES.md#e1018) | abstract/open 成员所属类型不允许 |
| [F0252](FRONTEND.md#f0252) | 1 | 主/因 | [E0308](RULES.md#e0308) | 声明缺少必须的函数体 |
| [F0253](FRONTEND.md#f0253) | 1 | 主/因 | [E0309](RULES.md#e0309) | 抽象声明不允许函数体 |
| [F0254](FRONTEND.md#f0254)–F0255 | 2 | 主/因 | [E1018](RULES.md#e1018) | abstract/open 成员所属类型不允许 |
| [F0256](FRONTEND.md#f0256)–F0257 | 2 | 主/因 | [E1019](RULES.md#e1019) | sealed 派生/构造访问范围不符 |
| [F0258](FRONTEND.md#f0258) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [F0259](FRONTEND.md#f0259) | 1 | 主/因 | [E1116](RULES.md#e1116) | 重复 context key |
| [F0260](FRONTEND.md#f0260) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [F0261](FRONTEND.md#f0261) | 1 | 主/因 | [E0610](RULES.md#e0610) | 递归调用改变完整类型实参映射 |
| [F0262](FRONTEND.md#f0262) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [F0263](FRONTEND.md#f0263) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0264](FRONTEND.md#f0264) | 1 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0265](FRONTEND.md#f0265) | 1 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0266](FRONTEND.md#f0266) | 1 | 主/因 | [E1004](RULES.md#e1004) | 父类型种类不合法 |
| [F0267](FRONTEND.md#f0267) | 1 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F0268](FRONTEND.md#f0268)–F0269 | 2 | 主/因 | [E1022](RULES.md#e1022) | 条件接口实现重叠 |
| [F0270](FRONTEND.md#f0270) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0271](FRONTEND.md#f0271) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0272](FRONTEND.md#f0272) | 1 | 主/因 | [E1023](RULES.md#e1023) | 条件接口证明成环 |
| [F0273](FRONTEND.md#f0273) | 1 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0274](FRONTEND.md#f0274) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0275](FRONTEND.md#f0275) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0276](FRONTEND.md#f0276) | 1 | 主/因 | [E0711](RULES.md#e0711) | 没有适用可调用候选 |
| [F0277](FRONTEND.md#f0277) | 1 | 主/因 | [E0712](RULES.md#e0712) | 可调用候选无法唯一选择 |
| [F0278](FRONTEND.md#f0278) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0279](FRONTEND.md#f0279) | 1 | 主/因 | [E0715](RULES.md#e0715) | 构造器缺少外部实例或捕获值 |
| [F0280](FRONTEND.md#f0280) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0281](FRONTEND.md#f0281)–F0285 | 5 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0286](FRONTEND.md#f0286) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0287](FRONTEND.md#f0287)–F0291 | 5 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0292](FRONTEND.md#f0292) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0293](FRONTEND.md#f0293)–F0295 | 3 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0296](FRONTEND.md#f0296) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0297](FRONTEND.md#f0297)–F0302 | 6 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0303](FRONTEND.md#f0303) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0304](FRONTEND.md#f0304) | 1 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0305](FRONTEND.md#f0305) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0306](FRONTEND.md#f0306)–F0307 | 2 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0308](FRONTEND.md#f0308) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0309](FRONTEND.md#f0309) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0310](FRONTEND.md#f0310)–F0311 | 2 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0312](FRONTEND.md#f0312) | 1 | 主/因 | [E1406](RULES.md#e1406) | intrinsic 语义身份重复绑定 |
| [F0313](FRONTEND.md#f0313)–F0315 | 3 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0316](FRONTEND.md#f0316) | 1 | 主/因 | [E1406](RULES.md#e1406) | intrinsic 语义身份重复绑定 |
| [F0317](FRONTEND.md#f0317) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0318](FRONTEND.md#f0318)–F0321 | 4 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0322](FRONTEND.md#f0322) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0323](FRONTEND.md#f0323)–F0326 | 4 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0327](FRONTEND.md#f0327) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0328](FRONTEND.md#f0328)–F0334 | 7 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0335](FRONTEND.md#f0335) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0336](FRONTEND.md#f0336)–F0342 | 7 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0343](FRONTEND.md#f0343) | 1 | 内部 | [ICE0301](RULES.md#ice0301) | 阶段输入混用了不同上下文 |
| [F0344](FRONTEND.md#f0344)–F0345 | 2 | 主/因 | [E2109](RULES.md#e2109) | 测试依赖边使用不合法 |
| [F0346](FRONTEND.md#f0346) | 1 | 内部 | [ICE0301](RULES.md#ice0301) | 阶段输入混用了不同上下文 |
| [F0347](FRONTEND.md#f0347) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0348](FRONTEND.md#f0348) | 1 | 内部 | [ICE0301](RULES.md#ice0301) | 阶段输入混用了不同上下文 |
| [F0349](FRONTEND.md#f0349) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [F0350](FRONTEND.md#f0350) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0351](FRONTEND.md#f0351) | 1 | 抑制/附注 | 依赖已失败或输入不完整的后继状态 | 依赖已失败或输入不完整的后继状态 |
| [F0352](FRONTEND.md#f0352) | 1 | 主/因 | [E0609](RULES.md#e0609) | 声明类型推断循环 |
| [F0353](FRONTEND.md#f0353) | 1 | 主/因 | [E1013](RULES.md#e1013) | override 签名合同不符 |
| [F0354](FRONTEND.md#f0354) | 1 | 主/因 | [E1508](RULES.md#e1508) | 属性缺少可推断类型的来源 |
| [F0355](FRONTEND.md#f0355) | 1 | 主/因 | [E0609](RULES.md#e0609) | 声明类型推断循环 |
| [F0356](FRONTEND.md#f0356) | 1 | 抑制/附注 | 依赖已失败或输入不完整的后继状态 | 依赖已失败或输入不完整的后继状态 |
| [F0357](FRONTEND.md#f0357) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0358](FRONTEND.md#f0358)–F0359 | 2 | 主/因 | [E0302](RULES.md#e0302) | 重复可调用签名 |
| [F0360](FRONTEND.md#f0360) | 1 | 主/因 | [E1020](RULES.md#e1020) | inner class 所在位置不合法 |
| [F0361](FRONTEND.md#f0361) | 1 | 主/因 | [E1402](RULES.md#e1402) | intrinsic 声明位置不合法 |
| [F0362](FRONTEND.md#f0362) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0363](FRONTEND.md#f0363)–F0370 | 8 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0371](FRONTEND.md#f0371) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0372](FRONTEND.md#f0372)–F0373 | 2 | 主/因 | [E1004](RULES.md#e1004) | 父类型种类不合法 |
| [F0374](FRONTEND.md#f0374) | 1 | 主/因 | [E1523](RULES.md#e1523) | enum 不允许声明指定自有运算符 |
| [F0375](FRONTEND.md#f0375) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0376](FRONTEND.md#f0376) | 1 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F0377](FRONTEND.md#f0377) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0378](FRONTEND.md#f0378)–F0379 | 2 | 主/因 | [E1021](RULES.md#e1021) | 重复或冲突的 companion |
| [F0380](FRONTEND.md#f0380) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0381](FRONTEND.md#f0381) | 1 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F0382](FRONTEND.md#f0382) | 1 | 主/因 | [E1402](RULES.md#e1402) | intrinsic 声明位置不合法 |
| [F0383](FRONTEND.md#f0383) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0384](FRONTEND.md#f0384) | 1 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F0385](FRONTEND.md#f0385)–F0386 | 2 | 主/因 | [E0720](RULES.md#e0720) | 默认表达式访问域不合法 |
| [F0387](FRONTEND.md#f0387) | 1 | 主/因 | [E0721](RULES.md#e0721) | 继承的默认参数冲突 |
| [F0388](FRONTEND.md#f0388) | 1 | 主/因 | [E1113](RULES.md#e1113) | release 中不允许该操作 |
| [F0389](FRONTEND.md#f0389) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0390](FRONTEND.md#f0390) | 1 | 抑制/附注 | 依赖已失败或输入不完整的后继状态 | 依赖已失败或输入不完整的后继状态 |
| [F0391](FRONTEND.md#f0391) | 1 | 主/因 | [E0722](RULES.md#e0722) | 默认参数展开循环 |
| [F0392](FRONTEND.md#f0392) | 1 | 主/因 | [E0723](RULES.md#e0723) | override 不能新增默认参数 |
| [F0393](FRONTEND.md#f0393) | 1 | 主/因 | [E0509](RULES.md#e0509) | 值布局无限递归 |
| [F0394](FRONTEND.md#f0394) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [F0395](FRONTEND.md#f0395) | 1 | 主/因 | [E2004](RULES.md#e2004) | 没有选择到源码 |
| [F0396](FRONTEND.md#f0396) | 1 | 内部 | [ICE0301](RULES.md#ice0301) | 阶段输入混用了不同上下文 |
| [F0397](FRONTEND.md#f0397)–F0398 | 2 | 抑制/附注 | 依赖已失败或输入不完整的后继状态 | 依赖已失败或输入不完整的后继状态 |
| [F0399](FRONTEND.md#f0399) | 1 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0400](FRONTEND.md#f0400) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0401](FRONTEND.md#f0401)–F0405 | 5 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0406](FRONTEND.md#f0406)–F0409 | 4 | 主/因 | [E1108](RULES.md#e1108) | safe 签名暴露 InteriorMutable |
| [F0410](FRONTEND.md#f0410)–F0413 | 4 | 主/因 | [E1104](RULES.md#e1104) | NoGC 中的调用可能分配/转换/抛出 |
| [F0414](FRONTEND.md#f0414) | 1 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0415](FRONTEND.md#f0415) | 1 | 主/因 | [E1104](RULES.md#e1104) | NoGC 中的调用可能分配/转换/抛出 |
| [F0416](FRONTEND.md#f0416)–F0417 | 2 | 主/因 | [E1213](RULES.md#e1213) | Ptr/addressOf 要求 GC-free 值 |
| [F0418](FRONTEND.md#f0418) | 1 | 主/因 | [E0610](RULES.md#e0610) | 递归调用改变完整类型实参映射 |
| [F0419](FRONTEND.md#f0419) | 1 | 主/因 | [E1107](RULES.md#e1107) | 泛型约束不能证明 GC-free |
| [F0420](FRONTEND.md#f0420) | 1 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0421](FRONTEND.md#f0421)–F0423 | 3 | 主/因 | [E1114](RULES.md#e1114) | release 泛型值/调用合同不成立 |
| [F0424](FRONTEND.md#f0424) | 1 | 主/因 | [E1101](RULES.md#e1101) | 操作要求 unsafe 上下文 |
| [F0425](FRONTEND.md#f0425) | 1 | 主/因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0426](FRONTEND.md#f0426) | 1 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0427](FRONTEND.md#f0427) | 1 | 主/因 | [E1104](RULES.md#e1104) | NoGC 中的调用可能分配/转换/抛出 |
| [F0428](FRONTEND.md#f0428)–F0443 | 16 | 主/因 | [E1105](RULES.md#e1105) | NoGC 中的操作要求 GC 或可抛出 |
| [F0444](FRONTEND.md#f0444) | 1 | 主/因 | [E1104](RULES.md#e1104) | NoGC 中的调用可能分配/转换/抛出 |
| [F0445](FRONTEND.md#f0445)–F0449 | 5 | 主/因 | [E1105](RULES.md#e1105) | NoGC 中的操作要求 GC 或可抛出 |
| [F0450](FRONTEND.md#f0450)–F0453 | 4 | 主/因 | [E1104](RULES.md#e1104) | NoGC 中的调用可能分配/转换/抛出 |
| [F0454](FRONTEND.md#f0454)–F0456 | 3 | 主/因 | [E1105](RULES.md#e1105) | NoGC 中的操作要求 GC 或可抛出 |
| [F0457](FRONTEND.md#f0457) | 1 | 主/因 | [E1106](RULES.md#e1106) | NoGC 中的值类型不是 GC-free |
| [F0458](FRONTEND.md#f0458) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0459](FRONTEND.md#f0459)–F0460 | 2 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0461](FRONTEND.md#f0461) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F0462](FRONTEND.md#f0462) | 1 | 主/因 | [E0204](RULES.md#e0204) | 语法列表不允许为空 |
| [F0463](FRONTEND.md#f0463) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0464](FRONTEND.md#f0464) | 1 | 主/因 | [E0607](RULES.md#e0607) | 缺少足够的类型推断信息 |
| [F0465](FRONTEND.md#f0465) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0466](FRONTEND.md#f0466) | 1 | 主/因 | [E1223](RULES.md#e1223) | 带内存序的原子方法不能直接取引用 |
| [F0467](FRONTEND.md#f0467)–F0469 | 3 | 主/因 | [E1222](RULES.md#e1222) | 原子操作不允许该内存序 |
| [F0470](FRONTEND.md#f0470) | 1 | 主/因 | [E1221](RULES.md#e1221) | 原子内存序不是编译期常量 |
| [F0471](FRONTEND.md#f0471) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0472](FRONTEND.md#f0472) | 1 | 主/因 | [E0802](RULES.md#e0802) | 匿名函数混用有值/无值 return |
| [F0473](FRONTEND.md#f0473) | 1 | 主/因 | [E0801](RULES.md#e0801) | 返回值路径不完整 |
| [F0474](FRONTEND.md#f0474) | 1 | 主/因 | [E0504](RULES.md#e0504) | 普通与 suspend 函数类型不相容 |
| [F0475](FRONTEND.md#f0475) | 1 | 主/因 | [E0503](RULES.md#e0503) | 函数类型的参数形状不符 |
| [F0476](FRONTEND.md#f0476)–F0478 | 3 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0479](FRONTEND.md#f0479)–F0480 | 2 | 主/因 | [E0503](RULES.md#e0503) | 函数类型的参数形状不符 |
| [F0481](FRONTEND.md#f0481) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0482](FRONTEND.md#f0482) | 1 | 主/因 | [E0607](RULES.md#e0607) | 缺少足够的类型推断信息 |
| [F0483](FRONTEND.md#f0483) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0484](FRONTEND.md#f0484) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0485](FRONTEND.md#f0485) | 1 | 主/因 | [E0713](RULES.md#e0713) | 该声明形式不能成为函数引用 |
| [F0486](FRONTEND.md#f0486) | 1 | 主/因 | [E1217](RULES.md#e1217) | native 函数地址目标不合格 |
| [F0487](FRONTEND.md#f0487) | 1 | 主/因 | [E0713](RULES.md#e0713) | 该声明形式不能成为函数引用 |
| [F0488](FRONTEND.md#f0488) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F0489](FRONTEND.md#f0489) | 1 | 主/因 | [E0714](RULES.md#e0714) | 不能经接口值使用泛型成员 |
| [F0490](FRONTEND.md#f0490) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0491](FRONTEND.md#f0491) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0492](FRONTEND.md#f0492) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0493](FRONTEND.md#f0493)–F0496 | 4 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0497](FRONTEND.md#f0497) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0498](FRONTEND.md#f0498)–F0499 | 2 | 主/因 | [E0715](RULES.md#e0715) | 构造器缺少外部实例或捕获值 |
| [F0500](FRONTEND.md#f0500) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0501](FRONTEND.md#f0501) | 1 | 主/因 | [E0713](RULES.md#e0713) | 该声明形式不能成为函数引用 |
| [F0502](FRONTEND.md#f0502) | 1 | 主/因 | [E1217](RULES.md#e1217) | native 函数地址目标不合格 |
| [F0503](FRONTEND.md#f0503) | 1 | 主/因 | [E0712](RULES.md#e0712) | 可调用候选无法唯一选择 |
| [F0504](FRONTEND.md#f0504) | 1 | 主/因 | [E1217](RULES.md#e1217) | native 函数地址目标不合格 |
| [F0505](FRONTEND.md#f0505) | 1 | 主/因 | [E0711](RULES.md#e0711) | 没有适用可调用候选 |
| [F0506](FRONTEND.md#f0506)–F0511 | 6 | 因 | [E1217](RULES.md#e1217) | native 函数地址目标不合格 |
| [F0512](FRONTEND.md#f0512) | 1 | 主/因 | [E0713](RULES.md#e0713) | 该声明形式不能成为函数引用 |
| [F0513](FRONTEND.md#f0513) | 1 | 主/因 | [E0711](RULES.md#e0711) | 没有适用可调用候选 |
| [F0514](FRONTEND.md#f0514) | 1 | 主/因 | [E0712](RULES.md#e0712) | 可调用候选无法唯一选择 |
| [F0515](FRONTEND.md#f0515) | 1 | 主/因 | [E0713](RULES.md#e0713) | 该声明形式不能成为函数引用 |
| [F0516](FRONTEND.md#f0516)–F0517 | 2 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0518](FRONTEND.md#f0518) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0519](FRONTEND.md#f0519) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0520](FRONTEND.md#f0520) | 1 | 主/因 | [E0717](RULES.md#e0717) | 调用目标未声明 infix 角色 |
| [F0521](FRONTEND.md#f0521) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0522](FRONTEND.md#f0522) | 1 | 主/因 | [E0502](RULES.md#e0502) | 表达式不是可调用类型 |
| [F0523](FRONTEND.md#f0523) | 1 | 主/因 | [E0706](RULES.md#e0706) | 向仅位置形参传命名实参 |
| [F0524](FRONTEND.md#f0524) | 1 | 主/因 | [E0707](RULES.md#e0707) | 非 vararg 位置使用 spread |
| [F0525](FRONTEND.md#f0525) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0526](FRONTEND.md#f0526) | 1 | 主/因 | [E1103](RULES.md#e1103) | 当前位置不能调用 suspend 函数 |
| [F0527](FRONTEND.md#f0527)–F0528 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0529](FRONTEND.md#f0529) | 1 | 主/因 | [E1216](RULES.md#e1216) | native 函数指针类型参数不合法 |
| [F0530](FRONTEND.md#f0530) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0531](FRONTEND.md#f0531) | 1 | 主/因 | [E1218](RULES.md#e1218) | 回调 mode 不是允许的常量 |
| [F0532](FRONTEND.md#f0532)–F0533 | 2 | 主/因 | [E1216](RULES.md#e1216) | native 函数指针类型参数不合法 |
| [F0534](FRONTEND.md#f0534)–F0536 | 3 | 主/因 | [E1219](RULES.md#e1219) | 回调 context 参数位置或类型无效 |
| [F0537](FRONTEND.md#f0537) | 1 | 主/因 | [E1502](RULES.md#e1502) | 不能捕获可变局部变量 |
| [F0538](FRONTEND.md#f0538)–F0540 | 3 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0541](FRONTEND.md#f0541)–F0543 | 3 | 主/因 | [E1601](RULES.md#e1601) | 编译期输入参数不是要求的常量 |
| [F0544](FRONTEND.md#f0544)–F0545 | 2 | 主/因 | [E1604](RULES.md#e1604) | 当前入口未提供编译期输入能力 |
| [F0546](FRONTEND.md#f0546) | 1 | 主/因 | [E1602](RULES.md#e1602) | includeStr 输入不是 UTF-8 |
| [F0547](FRONTEND.md#f0547) | 1 | 主/因 | [E1601](RULES.md#e1601) | 编译期输入参数不是要求的常量 |
| [F0548](FRONTEND.md#f0548) | 1 | 主/因 | [E1604](RULES.md#e1604) | 当前入口未提供编译期输入能力 |
| [F0549](FRONTEND.md#f0549)–F0550 | 2 | 主/因 | [E1603](RULES.md#e1603) | includeDir pattern 无效 |
| [F0551](FRONTEND.md#f0551)–F0552 | 2 | 主/因 | [E1601](RULES.md#e1601) | 编译期输入参数不是要求的常量 |
| [F0553](FRONTEND.md#f0553)–F0554 | 2 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0555](FRONTEND.md#f0555)–F0556 | 2 | 主/因 | [E0732](RULES.md#e0732) | 变体构造形式不合法 |
| [F0557](FRONTEND.md#f0557)–F0560 | 4 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0561](FRONTEND.md#f0561) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0562](FRONTEND.md#f0562) | 1 | 主/因 | [E0607](RULES.md#e0607) | 缺少足够的类型推断信息 |
| [F0563](FRONTEND.md#f0563) | 1 | 主/因 | [E1213](RULES.md#e1213) | Ptr/addressOf 要求 GC-free 值 |
| [F0564](FRONTEND.md#f0564) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F0565](FRONTEND.md#f0565) | 1 | 主/因 | [E0703](RULES.md#e0703) | 形参被重复传入 |
| [F0566](FRONTEND.md#f0566) | 1 | 主/因 | [E0707](RULES.md#e0707) | 非 vararg 位置使用 spread |
| [F0567](FRONTEND.md#f0567) | 1 | 主/因 | [E0708](RULES.md#e0708) | 缺少必填实参 |
| [F0568](FRONTEND.md#f0568) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0569](FRONTEND.md#f0569) | 1 | 主/因 | [E1215](RULES.md#e1215) | 非空 Ptr 的原始地址为零 |
| [F0570](FRONTEND.md#f0570) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0571](FRONTEND.md#f0571) | 1 | 主/因 | [E0607](RULES.md#e0607) | 缺少足够的类型推断信息 |
| [F0572](FRONTEND.md#f0572) | 1 | 主/因 | [E0732](RULES.md#e0732) | 变体构造形式不合法 |
| [F0573](FRONTEND.md#f0573) | 1 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0574](FRONTEND.md#f0574) | 1 | 主/因 | [E1115](RULES.md#e1115) | context key/binding 类型不合法 |
| [F0575](FRONTEND.md#f0575) | 1 | 主/因 | [E1116](RULES.md#e1116) | 重复 context key |
| [F0576](FRONTEND.md#f0576) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0577](FRONTEND.md#f0577) | 1 | 主/因 | [E1118](RULES.md#e1118) | 效果注解组合不合法 |
| [F0578](FRONTEND.md#f0578) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0579](FRONTEND.md#f0579) | 1 | 主/因 | [E1115](RULES.md#e1115) | context key/binding 类型不合法 |
| [F0580](FRONTEND.md#f0580) | 1 | 主/因 | [E1504](RULES.md#e1504) | copy update 目标不是精确 struct 值 |
| [F0581](FRONTEND.md#f0581) | 1 | 主/因 | [E1505](RULES.md#e1505) | copy update 重复字段 |
| [F0582](FRONTEND.md#f0582) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0583](FRONTEND.md#f0583) | 1 | 主/因 | [E0505](RULES.md#e0505) | Option 操作数类型不符 |
| [F0584](FRONTEND.md#f0584)–F0585 | 2 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0586](FRONTEND.md#f0586) | 1 | 主/因 | [E0732](RULES.md#e0732) | 变体构造形式不合法 |
| [F0587](FRONTEND.md#f0587)–F0588 | 2 | 主/因 | [E0505](RULES.md#e0505) | Option 操作数类型不符 |
| [F0589](FRONTEND.md#f0589) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0590](FRONTEND.md#f0590)–F0594 | 5 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0595](FRONTEND.md#f0595) | 1 | 主/因 | [E0107](RULES.md#e0107) | 数值字面量超出表示范围 |
| [F0596](FRONTEND.md#f0596) | 1 | 因 | [E0107](RULES.md#e0107) | 数值字面量超出表示范围 |
| [F0597](FRONTEND.md#f0597) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0598](FRONTEND.md#f0598)–F0599 | 2 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0600](FRONTEND.md#f0600) | 1 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0601](FRONTEND.md#f0601) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F0602](FRONTEND.md#f0602) | 1 | 主/因 | [E1112](RULES.md#e1112) | release 不能写 owner field 或取其地址 |
| [F0603](FRONTEND.md#f0603) | 1 | 主/因 | [E1214](RULES.md#e1214) | addressOf 实参不可取址 |
| [F0604](FRONTEND.md#f0604) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0605](FRONTEND.md#f0605) | 1 | 主/因 | [E1213](RULES.md#e1213) | Ptr/addressOf 要求 GC-free 值 |
| [F0606](FRONTEND.md#f0606) | 1 | 主/因 | [E0511](RULES.md#e0511) | 布局查询要求值类型 |
| [F0607](FRONTEND.md#f0607) | 1 | 主/因 | [E0711](RULES.md#e0711) | 没有适用可调用候选 |
| [F0608](FRONTEND.md#f0608) | 1 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0609](FRONTEND.md#f0609)–F0612 | 4 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0613](FRONTEND.md#f0613) | 1 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [F0614](FRONTEND.md#f0614) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0615](FRONTEND.md#f0615) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0616](FRONTEND.md#f0616) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0617](FRONTEND.md#f0617)–F0618 | 2 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0619](FRONTEND.md#f0619) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0620](FRONTEND.md#f0620)–F0621 | 2 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [F0622](FRONTEND.md#f0622)–F0623 | 2 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [F0624](FRONTEND.md#f0624) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0625](FRONTEND.md#f0625) | 1 | 主/因 | [E0716](RULES.md#e0716) | 缺少所需接收者 |
| [F0626](FRONTEND.md#f0626) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0627](FRONTEND.md#f0627)–F0633 | 7 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0634](FRONTEND.md#f0634) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0635](FRONTEND.md#f0635) | 1 | 主/因 | [E1501](RULES.md#e1501) | 不能写入不可变值 |
| [F0636](FRONTEND.md#f0636) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0637](FRONTEND.md#f0637) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0638](FRONTEND.md#f0638) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0639](FRONTEND.md#f0639) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0640](FRONTEND.md#f0640) | 1 | 主/因 | [E1025](RULES.md#e1025) | super 目标或形式不合法 |
| [F0641](FRONTEND.md#f0641)–F0643 | 3 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0644](FRONTEND.md#f0644) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0645](FRONTEND.md#f0645) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0646](FRONTEND.md#f0646)–F0647 | 2 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0648](FRONTEND.md#f0648) | 1 | 主/因 | [E0603](RULES.md#e0603) | 不满足名义上界 |
| [F0649](FRONTEND.md#f0649)–F0651 | 3 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0652](FRONTEND.md#f0652) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0653](FRONTEND.md#f0653) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0654](FRONTEND.md#f0654)–F0657 | 4 | 主/因 | [E0716](RULES.md#e0716) | 缺少所需接收者 |
| [F0658](FRONTEND.md#f0658) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0659](FRONTEND.md#f0659) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0660](FRONTEND.md#f0660) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F0661](FRONTEND.md#f0661) | 1 | 主/因 | [E0707](RULES.md#e0707) | 非 vararg 位置使用 spread |
| [F0662](FRONTEND.md#f0662)–F0663 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0664](FRONTEND.md#f0664) | 1 | 主/因 | [E0714](RULES.md#e0714) | 不能经接口值使用泛型成员 |
| [F0665](FRONTEND.md#f0665) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0666](FRONTEND.md#f0666) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0667](FRONTEND.md#f0667)–F0668 | 2 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0669](FRONTEND.md#f0669) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0670](FRONTEND.md#f0670)–F0675 | 6 | 主/因 | [E1025](RULES.md#e1025) | super 目标或形式不合法 |
| [F0676](FRONTEND.md#f0676) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0677](FRONTEND.md#f0677)–F0679 | 3 | 主/因 | [E0510](RULES.md#e0510) | 类型不可直接构造 |
| [F0680](FRONTEND.md#f0680) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0681](FRONTEND.md#f0681)–F0682 | 2 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0683](FRONTEND.md#f0683) | 1 | 主/因 | [E1025](RULES.md#e1025) | super 目标或形式不合法 |
| [F0684](FRONTEND.md#f0684) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0685](FRONTEND.md#f0685) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0686](FRONTEND.md#f0686)–F0687 | 2 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0688](FRONTEND.md#f0688)–F0690 | 3 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0691](FRONTEND.md#f0691) | 1 | 主/因 | [E1025](RULES.md#e1025) | super 目标或形式不合法 |
| [F0692](FRONTEND.md#f0692)–F0693 | 2 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0694](FRONTEND.md#f0694) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0695](FRONTEND.md#f0695) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0696](FRONTEND.md#f0696)–F0716 | 21 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0717](FRONTEND.md#f0717) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0718](FRONTEND.md#f0718)–F0739 | 22 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0740](FRONTEND.md#f0740)–F0741 | 2 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0742](FRONTEND.md#f0742)–F0745 | 4 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0746](FRONTEND.md#f0746) | 1 | 主/因 | [E1214](RULES.md#e1214) | addressOf 实参不可取址 |
| [F0747](FRONTEND.md#f0747)–F0748 | 2 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0749](FRONTEND.md#f0749) | 1 | 主/因 | [E0603](RULES.md#e0603) | 不满足名义上界 |
| [F0750](FRONTEND.md#f0750)–F0753 | 4 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0754](FRONTEND.md#f0754) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0755](FRONTEND.md#f0755) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F0756](FRONTEND.md#f0756) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0757](FRONTEND.md#f0757)–F0760 | 4 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0761](FRONTEND.md#f0761) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0762](FRONTEND.md#f0762) | 1 | 主/因 | [E0711](RULES.md#e0711) | 没有适用可调用候选 |
| [F0763](FRONTEND.md#f0763) | 1 | 主/因 | [E1405](RULES.md#e1405) | annotationKey 参数不是声明查询 |
| [F0764](FRONTEND.md#f0764) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F0765](FRONTEND.md#f0765)–F0767 | 3 | 主/因 | [E1405](RULES.md#e1405) | annotationKey 参数不是声明查询 |
| [F0768](FRONTEND.md#f0768)–F0769 | 2 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0770](FRONTEND.md#f0770) | 1 | 主/因 | [E0620](RULES.md#e0620) | member requirement 递归 |
| [F0771](FRONTEND.md#f0771)–F0772 | 2 | 主/因 | [E0621](RULES.md#e0621) | 精确成员 requirement 不满足 |
| [F0773](FRONTEND.md#f0773) | 1 | 主/因 | [E0622](RULES.md#e0622) | 成员 requirement 有歧义 |
| [F0774](FRONTEND.md#f0774)–F0775 | 2 | 主/因 | [E0621](RULES.md#e0621) | 精确成员 requirement 不满足 |
| [F0776](FRONTEND.md#f0776) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0777](FRONTEND.md#f0777) | 1 | 主/因 | [E0622](RULES.md#e0622) | 成员 requirement 有歧义 |
| [F0778](FRONTEND.md#f0778) | 1 | 主/因 | [E0715](RULES.md#e0715) | 构造器缺少外部实例或捕获值 |
| [F0779](FRONTEND.md#f0779) | 1 | 主/因 | [E0621](RULES.md#e0621) | 精确成员 requirement 不满足 |
| [F0780](FRONTEND.md#f0780) | 1 | 主/因 | [E0715](RULES.md#e0715) | 构造器缺少外部实例或捕获值 |
| [F0781](FRONTEND.md#f0781)–F0782 | 2 | 主/因 | [E0621](RULES.md#e0621) | 精确成员 requirement 不满足 |
| [F0783](FRONTEND.md#f0783) | 1 | 因 | [E0621](RULES.md#e0621) | 精确成员 requirement 不满足 |
| [F0784](FRONTEND.md#f0784)–F0786 | 3 | 主/因 | [E0621](RULES.md#e0621) | 精确成员 requirement 不满足 |
| [F0787](FRONTEND.md#f0787) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0788](FRONTEND.md#f0788) | 1 | 主/因 | [E0716](RULES.md#e0716) | 缺少所需接收者 |
| [F0789](FRONTEND.md#f0789) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0790](FRONTEND.md#f0790) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F0791](FRONTEND.md#f0791) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0792](FRONTEND.md#f0792) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0793](FRONTEND.md#f0793) | 1 | 主/因 | [E0508](RULES.md#e0508) | 值类型不能使用引用相等 |
| [F0794](FRONTEND.md#f0794) | 1 | 主/因 | [E1316](RULES.md#e1316) | 字段无法派生相等比较 |
| [F0795](FRONTEND.md#f0795) | 1 | 主/因 | [E0711](RULES.md#e0711) | 没有适用可调用候选 |
| [F0796](FRONTEND.md#f0796)–F0797 | 2 | 主/因 | [E1316](RULES.md#e1316) | 字段无法派生相等比较 |
| [F0798](FRONTEND.md#f0798) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0799](FRONTEND.md#f0799)–F0801 | 3 | 主/因 | [E0716](RULES.md#e0716) | 缺少所需接收者 |
| [F0802](FRONTEND.md#f0802) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0803](FRONTEND.md#f0803)–F0804 | 2 | 主/因 | [E0107](RULES.md#e0107) | 数值字面量超出表示范围 |
| [F0805](FRONTEND.md#f0805) | 1 | 主/因 | [E0506](RULES.md#e0506) | 不可能成功的类型检查 |
| [F0806](FRONTEND.md#f0806) | 1 | 主/因 | [E0507](RULES.md#e0507) | 不可能成功的类型转换 |
| [F0807](FRONTEND.md#f0807) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0808](FRONTEND.md#f0808)–F0811 | 4 | 主/因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0812](FRONTEND.md#f0812)–F0813 | 2 | 主/因 | [E1202](RULES.md#e1202) | extern 符号冲突 |
| [F0814](FRONTEND.md#f0814)–F0816 | 3 | 主/因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0817](FRONTEND.md#f0817) | 1 | 主/因 | [E1202](RULES.md#e1202) | extern 符号冲突 |
| [F0818](FRONTEND.md#f0818)–F0822 | 5 | 主/因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0823](FRONTEND.md#f0823)–F0829 | 7 | 因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0830](FRONTEND.md#f0830) | 1 | 主/因 | [E0509](RULES.md#e0509) | 值布局无限递归 |
| [F0831](FRONTEND.md#f0831)–F0832 | 2 | 因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0833](FRONTEND.md#f0833)–F0835 | 3 | 主/因 | [E1220](RULES.md#e1220) | captureErrno 合同不满足 |
| [F0836](FRONTEND.md#f0836)–F0837 | 2 | 主/因 | [E2511](RULES.md#e2511) | 外部生成器结果违反协议 |
| [F0838](FRONTEND.md#f0838) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F0839](FRONTEND.md#f0839)–F0847 | 9 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F0848](FRONTEND.md#f0848) | 1 | 主/因 | [E1517](RULES.md#e1517) | const 依赖成环 |
| [F0849](FRONTEND.md#f0849)–F0850 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0851](FRONTEND.md#f0851) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0852](FRONTEND.md#f0852) | 1 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0853](FRONTEND.md#f0853) | 1 | 主/因 | [E1520](RULES.md#e1520) | tuple 常量元素数量不符 |
| [F0854](FRONTEND.md#f0854)–F0856 | 3 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0857](FRONTEND.md#f0857) | 1 | 主/因 | [E1519](RULES.md#e1519) | 常量求值除零 |
| [F0858](FRONTEND.md#f0858)–F0859 | 2 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0860](FRONTEND.md#f0860) | 1 | 主/因 | [E1521](RULES.md#e1521) | integral enum 值声明不合法 |
| [F0861](FRONTEND.md#f0861) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0862](FRONTEND.md#f0862) | 1 | 主/因 | [E1522](RULES.md#e1522) | integral enum 值重复 |
| [F0863](FRONTEND.md#f0863)–F0864 | 2 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0865](FRONTEND.md#f0865) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0866](FRONTEND.md#f0866) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F0867](FRONTEND.md#f0867) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F0868](FRONTEND.md#f0868) | 1 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0869](FRONTEND.md#f0869) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0870](FRONTEND.md#f0870) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0871](FRONTEND.md#f0871) | 1 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0872](FRONTEND.md#f0872) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0873](FRONTEND.md#f0873) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0874](FRONTEND.md#f0874) | 1 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0875](FRONTEND.md#f0875)–F0876 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0877](FRONTEND.md#f0877)–F0878 | 2 | 主/因 | [E1518](RULES.md#e1518) | 表达式不是允许的编译期常量 |
| [F0879](FRONTEND.md#f0879) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0880](FRONTEND.md#f0880)–F0882 | 3 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F0883](FRONTEND.md#f0883) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F0884](FRONTEND.md#f0884) | 1 | 主/因 | [E1515](RULES.md#e1515) | 该属性位置不允许存储或委托 |
| [F0885](FRONTEND.md#f0885) | 1 | 主/因 | [E0624](RULES.md#e0624) | 扩展属性类型参数不可从 receiver 推断 |
| [F0886](FRONTEND.md#f0886) | 1 | 主/因 | [E0302](RULES.md#e0302) | 重复可调用签名 |
| [F0887](FRONTEND.md#f0887) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F0888](FRONTEND.md#f0888) | 1 | 主/因 | [E1524](RULES.md#e1524) | 运行时初始化依赖循环 |
| [F0889](FRONTEND.md#f0889)–F0891 | 3 | 主/因 | [E1209](RULES.md#e1209) | raw/extern 属性形态不合法 |
| [F0892](FRONTEND.md#f0892) | 1 | 主/因 | [E1210](RULES.md#e1210) | raw 类型没有合法全零初始映像 |
| [F0893](FRONTEND.md#f0893) | 1 | 主/因 | [E1201](RULES.md#e1201) | 类型不能用于选定 FFI ABI |
| [F0894](FRONTEND.md#f0894) | 1 | 主/因 | [E1211](RULES.md#e1211) | raw 存储类型不是具体 GC-free 值 |
| [F0895](FRONTEND.md#f0895) | 1 | 主/因 | [E1212](RULES.md#e1212) | raw 初始化不是合格常量 |
| [F0896](FRONTEND.md#f0896) | 1 | 主/因 | [E1509](RULES.md#e1509) | 无初始化值的属性不符合 Option 简写 |
| [F0897](FRONTEND.md#f0897) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [F0898](FRONTEND.md#f0898) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0899](FRONTEND.md#f0899)–F0920 | 22 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0921](FRONTEND.md#f0921) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F0922](FRONTEND.md#f0922)–F0924 | 3 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0925](FRONTEND.md#f0925) | 1 | 主/因 | [E0602](RULES.md#e0602) | 不满足 kind bound |
| [F0926](FRONTEND.md#f0926) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0927](FRONTEND.md#f0927) | 1 | 主/因 | [E0603](RULES.md#e0603) | 不满足名义上界 |
| [F0928](FRONTEND.md#f0928) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0929](FRONTEND.md#f0929) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F0930](FRONTEND.md#f0930)–F0931 | 2 | 主/因 | [E1216](RULES.md#e1216) | native 函数指针类型参数不合法 |
| [F0932](FRONTEND.md#f0932)–F0942 | 11 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0943](FRONTEND.md#f0943)–F0950 | 8 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0951](FRONTEND.md#f0951)–F0955 | 5 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0956](FRONTEND.md#f0956)–F0959 | 4 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0960](FRONTEND.md#f0960) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0961](FRONTEND.md#f0961)–F0966 | 6 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0967](FRONTEND.md#f0967) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0968](FRONTEND.md#f0968)–F0970 | 3 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0971](FRONTEND.md#f0971) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0972](FRONTEND.md#f0972)–F0973 | 2 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [F0974](FRONTEND.md#f0974) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0975](FRONTEND.md#f0975)–F0979 | 5 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F0980](FRONTEND.md#f0980) | 1 | 主/因 | [E0407](RULES.md#e0407) | 公开导入规则不满足 |
| [F0981](FRONTEND.md#f0981)–F0982 | 2 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0983](FRONTEND.md#f0983)–F0985 | 3 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F0986](FRONTEND.md#f0986)–F0987 | 2 | 抑制/附注 | 依赖已失败或输入不完整的后继状态 | 依赖已失败或输入不完整的后继状态 |
| [F0988](FRONTEND.md#f0988) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F0989](FRONTEND.md#f0989) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F0990](FRONTEND.md#f0990)–F0991 | 2 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F0992](FRONTEND.md#f0992) | 1 | 主/因 | [E0716](RULES.md#e0716) | 缺少所需接收者 |
| [F0993](FRONTEND.md#f0993) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [F0994](FRONTEND.md#f0994) | 1 | 主/因 | [E0407](RULES.md#e0407) | 公开导入规则不满足 |
| [F0995](FRONTEND.md#f0995)–F0996 | 2 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F0997](FRONTEND.md#f0997) | 1 | 主/因 | [E0407](RULES.md#e0407) | 公开导入规则不满足 |
| [F0998](FRONTEND.md#f0998)–F0999 | 2 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1000](FRONTEND.md#f1000) | 1 | 主/因 | [E0409](RULES.md#e0409) | 公开绑定冲突 |
| [F1001](FRONTEND.md#f1001)–F1003 | 3 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1004](FRONTEND.md#f1004) | 1 | 主/因 | [E0407](RULES.md#e0407) | 公开导入规则不满足 |
| [F1005](FRONTEND.md#f1005) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F1006](FRONTEND.md#f1006) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F1007](FRONTEND.md#f1007)–F1008 | 2 | 主/因 | [E0406](RULES.md#e0406) | 导入目标不合法或不存在 |
| [F1009](FRONTEND.md#f1009) | 1 | 主/因 | [E0404](RULES.md#e0404) | 名称查找有歧义 |
| [F1010](FRONTEND.md#f1010)–F1011 | 2 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [F1012](FRONTEND.md#f1012) | 1 | 主/因 | [E1503](RULES.md#e1503) | 该局部类型不能捕获运行时值 |
| [F1013](FRONTEND.md#f1013)–F1014 | 2 | 主/因 | [E0410](RULES.md#e0410) | 签名泄露不可访问或局部类型 |
| [F1015](FRONTEND.md#f1015) | 1 | 主/因 | [E1103](RULES.md#e1103) | 当前位置不能调用 suspend 函数 |
| [F1016](FRONTEND.md#f1016)–F1017 | 2 | 主/因 | [E1101](RULES.md#e1101) | 操作要求 unsafe 上下文 |
| [F1018](FRONTEND.md#f1018) | 1 | 主/因 | [E1701](RULES.md#e1701) | 存在多个合法入口 |
| [F1019](FRONTEND.md#f1019)–F1021 | 3 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1022](FRONTEND.md#f1022)–F1023 | 2 | 主/因 | [E1702](RULES.md#e1702) | 没有合法入口 |
| [F1024](FRONTEND.md#f1024)–F1026 | 3 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1027](FRONTEND.md#f1027) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F1028](FRONTEND.md#f1028) | 1 | 主/因 | [E0901](RULES.md#e0901) | 裸名称不能作为当前匹配模式 |
| [F1029](FRONTEND.md#f1029) | 1 | 主/因 | [E0902](RULES.md#e0902) | 不可失败的绑定位置使用可失败模式 |
| [F1030](FRONTEND.md#f1030) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F1031](FRONTEND.md#f1031)–F1032 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1033](FRONTEND.md#f1033) | 1 | 主/因 | [E0903](RULES.md#e0903) | 模式 rest 位置或次数不合法 |
| [F1034](FRONTEND.md#f1034) | 1 | 主/因 | [E0904](RULES.md#e0904) | when 未穷尽 |
| [F1035](FRONTEND.md#f1035)–F1037 | 3 | 主/因 | [E0905](RULES.md#e0905) | 该类型不能以此形式解构 |
| [F1038](FRONTEND.md#f1038) | 1 | 主/因 | [E0906](RULES.md#e0906) | 模式字段数量不符 |
| [F1039](FRONTEND.md#f1039) | 1 | 主/因 | [E0907](RULES.md#e0907) | 字段模式遗漏字段 |
| [F1040](FRONTEND.md#f1040) | 1 | 主/因 | [E0908](RULES.md#e0908) | 模式字段名称形式不合法 |
| [F1041](FRONTEND.md#f1041) | 1 | 主/因 | [E0909](RULES.md#e0909) | 模式重复列举字段 |
| [F1042](FRONTEND.md#f1042) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F1043](FRONTEND.md#f1043) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1044](FRONTEND.md#f1044) | 1 | 主/因 | [E0906](RULES.md#e0906) | 模式字段数量不符 |
| [F1045](FRONTEND.md#f1045)–F1046 | 2 | 主/因 | [E0910](RULES.md#e0910) | 模式形式与变体字段形式不符 |
| [F1047](FRONTEND.md#f1047)–F1071 | 25 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1072](FRONTEND.md#f1072) | 1 | 主/因 | [E0409](RULES.md#e0409) | 公开绑定冲突 |
| [F1073](FRONTEND.md#f1073)–F1080 | 8 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1081](FRONTEND.md#f1081)–F1082 | 2 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [F1083](FRONTEND.md#f1083)–F1107 | 25 | 内部 | [ICE0401](RULES.md#ice0401) | 编译器实体身份或源码定位不一致 |
| [F1108](FRONTEND.md#f1108) | 1 | 主/因 | [E2004](RULES.md#e2004) | 没有选择到源码 |
| [F1109](FRONTEND.md#f1109)–F1111 | 3 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F1112](FRONTEND.md#f1112) | 1 | 主/因 | [E1514](RULES.md#e1514) | field 使用位置不合法 |
| [F1113](FRONTEND.md#f1113) | 1 | 主/因 | [E1512](RULES.md#e1512) | accessor 可见性不合法 |
| [F1114](FRONTEND.md#f1114) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1115](FRONTEND.md#f1115) | 1 | 主/因 | [E1016](RULES.md#e1016) | 继承成员访问域合同不符 |
| [F1116](FRONTEND.md#f1116) | 1 | 主/因 | [E0307](RULES.md#e0307) | 扩展声明位置不合法 |
| [F1117](FRONTEND.md#f1117)–F1119 | 3 | 主/因 | [E1507](RULES.md#e1507) | 值类型属性形态不合法 |
| [F1120](FRONTEND.md#f1120) | 1 | 主/因 | [E1510](RULES.md#e1510) | computed 属性缺少 accessor body |
| [F1121](FRONTEND.md#f1121) | 1 | 主/因 | [E1511](RULES.md#e1511) | 不可变属性声明了 setter |
| [F1122](FRONTEND.md#f1122) | 1 | 主/因 | [E1510](RULES.md#e1510) | computed 属性缺少 accessor body |
| [F1123](FRONTEND.md#f1123)–F1124 | 2 | 主/因 | [E1018](RULES.md#e1018) | abstract/open 成员所属类型不允许 |
| [F1125](FRONTEND.md#f1125) | 1 | 主/因 | [E0309](RULES.md#e0309) | 抽象声明不允许函数体 |
| [F1126](FRONTEND.md#f1126) | 1 | 主/因 | [E1203](RULES.md#e1203) | Extern 声明形态不合法 |
| [F1127](FRONTEND.md#f1127) | 1 | 主/因 | [E1516](RULES.md#e1516) | 实例属性不能是 const |
| [F1128](FRONTEND.md#f1128)–F1129 | 2 | 主/因 | [E1018](RULES.md#e1018) | abstract/open 成员所属类型不允许 |
| [F1130](FRONTEND.md#f1130) | 1 | 主/因 | [E1515](RULES.md#e1515) | 该属性位置不允许存储或委托 |
| [F1131](FRONTEND.md#f1131) | 1 | 主/因 | [E1510](RULES.md#e1510) | computed 属性缺少 accessor body |
| [F1132](FRONTEND.md#f1132) | 1 | 主/因 | [E1525](RULES.md#e1525) | 缺少合格的属性委托 operator |
| [F1133](FRONTEND.md#f1133) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1134](FRONTEND.md#f1134)–F1136 | 3 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F1137](FRONTEND.md#f1137) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F1138](FRONTEND.md#f1138) | 1 | 主/因 | [E1110](RULES.md#e1110) | 同一类重复 release |
| [F1139](FRONTEND.md#f1139)–F1141 | 3 | 主/因 | [E1109](RULES.md#e1109) | release 所属类型不合法 |
| [F1142](FRONTEND.md#f1142) | 1 | 主/因 | [E1111](RULES.md#e1111) | release 读取非本类普通 backing field |
| [F1143](FRONTEND.md#f1143) | 1 | 主/因 | [E1112](RULES.md#e1112) | release 不能写 owner field 或取其地址 |
| [F1144](FRONTEND.md#f1144) | 1 | 主/因 | [E1113](RULES.md#e1113) | release 中不允许该操作 |
| [F1145](FRONTEND.md#f1145) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [F1146](FRONTEND.md#f1146) | 1 | 按边界 | [E2505](RULES.md#e2505) / [ICE2502](RULES.md#ice2502) | 查询输入与内部查询状态边界 |
| [F1147](FRONTEND.md#f1147) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [F1148](FRONTEND.md#f1148) | 1 | 按边界 | [E2505](RULES.md#e2505) / [ICE2502](RULES.md#ice2502) | 查询输入与内部查询状态边界 |
| [F1149](FRONTEND.md#f1149)–F1155 | 7 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [F1156](FRONTEND.md#f1156)–F1160 | 5 | 主/因 | [E2503](RULES.md#e2503) | 查询请求或编辑范围无效 |
| [F1161](FRONTEND.md#f1161)–F1162 | 2 | 主/因 | [E1403](RULES.md#e1403) | core/intrinsic 声明合同不符 |
| [F1163](FRONTEND.md#f1163)–F1164 | 2 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1165](FRONTEND.md#f1165)–F1166 | 2 | 主/因 | [E1521](RULES.md#e1521) | integral enum 值声明不合法 |
| [F1167](FRONTEND.md#f1167)–F1168 | 2 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1169](FRONTEND.md#f1169) | 1 | 主/因 | [E0724](RULES.md#e0724) | 该字段形式不允许默认值 |
| [F1170](FRONTEND.md#f1170) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1171](FRONTEND.md#f1171) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [F1172](FRONTEND.md#f1172)–F1173 | 2 | 主/因 | [E1501](RULES.md#e1501) | 不能写入不可变值 |
| [F1174](FRONTEND.md#f1174) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1175](FRONTEND.md#f1175)–F1177 | 3 | 主/因 | [E1501](RULES.md#e1501) | 不能写入不可变值 |
| [F1178](FRONTEND.md#f1178) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F1179](FRONTEND.md#f1179)–F1180 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1181](FRONTEND.md#f1181) | 1 | 主/因 | [E0807](RULES.md#e0807) | 值 if 缺少 else |
| [F1182](FRONTEND.md#f1182) | 1 | 主/因 | [E0801](RULES.md#e0801) | 返回值路径不完整 |
| [F1183](FRONTEND.md#f1183) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1184](FRONTEND.md#f1184) | 1 | 主/因 | [E0730](RULES.md#e0730) | 复合赋值的两条路径都适用 |
| [F1185](FRONTEND.md#f1185) | 1 | 主/因 | [E1506](RULES.md#e1506) | 写入目标不可赋值 |
| [F1186](FRONTEND.md#f1186) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1187](FRONTEND.md#f1187) | 1 | 主/因 | [E0731](RULES.md#e0731) | 复合赋值没有适用运算符 |
| [F1188](FRONTEND.md#f1188)–F1189 | 2 | 主/因 | [E0808](RULES.md#e0808) | 异常类型不满足 Throwable |
| [F1190](FRONTEND.md#f1190) | 1 | 主/因 | [E0809](RULES.md#e0809) | catch 已被前面的分支覆盖 |
| [F1191](FRONTEND.md#f1191) | 1 | 主/因 | [E0810](RULES.md#e0810) | 读取尚未初始化的值 |
| [F1192](FRONTEND.md#f1192)–F1193 | 2 | 主/因 | [E0811](RULES.md#e0811) | iterator 协议不满足 |
| [F1194](FRONTEND.md#f1194) | 1 | 主/因 | [E0804](RULES.md#e0804) | return 不允许用于当前上下文 |
| [F1195](FRONTEND.md#f1195) | 1 | 主/因 | [E0803](RULES.md#e0803) | return 缺少返回值 |
| [F1196](FRONTEND.md#f1196) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1197](FRONTEND.md#f1197)–F1199 | 3 | 主/因 | [E0804](RULES.md#e0804) | return 不允许用于当前上下文 |
| [F1200](FRONTEND.md#f1200)–F1201 | 2 | 主/因 | [E0805](RULES.md#e0805) | return 目标不合法 |
| [F1202](FRONTEND.md#f1202) | 1 | 主/因 | [E0804](RULES.md#e0804) | return 不允许用于当前上下文 |
| [F1203](FRONTEND.md#f1203) | 1 | 主/因 | [E0801](RULES.md#e0801) | 返回值路径不完整 |
| [F1204](FRONTEND.md#f1204) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1205](FRONTEND.md#f1205) | 1 | 主/因 | [E0308](RULES.md#e0308) | 声明缺少必须的函数体 |
| [F1206](FRONTEND.md#f1206) | 1 | 主/因 | [E0302](RULES.md#e0302) | 重复可调用签名 |
| [F1207](FRONTEND.md#f1207)–F1208 | 2 | 主/因 | [E0806](RULES.md#e0806) | 循环跳转没有合法目标 |
| [F1209](FRONTEND.md#f1209) | 1 | 主/因 | [E1506](RULES.md#e1506) | 写入目标不可赋值 |
| [F1210](FRONTEND.md#f1210) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1211](FRONTEND.md#f1211) | 1 | 主/因 | [E1506](RULES.md#e1506) | 写入目标不可赋值 |
| [F1212](FRONTEND.md#f1212) | 1 | 主/因 | [E0812](RULES.md#e0812) | smart cast 在所有路径上不成立 |
| [F1213](FRONTEND.md#f1213) | 1 | 主/因 | [E0813](RULES.md#e0813) | 表达式不能作为该语句 |
| [F1214](FRONTEND.md#f1214)–F1215 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1216](FRONTEND.md#f1216) | 1 | 主/因 | [E0904](RULES.md#e0904) | when 未穷尽 |
| [F1217](FRONTEND.md#f1217) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1218](FRONTEND.md#f1218) | 1 | 主/因 | [E0911](RULES.md#e0911) | case subject 类型不允许 |
| [F1219](FRONTEND.md#f1219)–F1220 | 2 | 主/因 | [E0912](RULES.md#e0912) | 模式或简写条件缺少 subject |
| [F1221](FRONTEND.md#f1221) | 1 | 主/因 | [E0904](RULES.md#e0904) | when 未穷尽 |
| [F1222](FRONTEND.md#f1222) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F1223](FRONTEND.md#f1223) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1224](FRONTEND.md#f1224) | 1 | 主/因 | [E1703](RULES.md#e1703) | 测试注解出现在非测试源码 |
| [F1225](FRONTEND.md#f1225) | 1 | 主/因 | [E1704](RULES.md#e1704) | 测试容器不符合规定形状 |
| [F1226](FRONTEND.md#f1226)–F1227 | 2 | 主/因 | [E1705](RULES.md#e1705) | 测试注解组合不合法 |
| [F1228](FRONTEND.md#f1228) | 1 | 主/因 | [E1706](RULES.md#e1706) | 测试方法不在直接 Test class 内 |
| [F1229](FRONTEND.md#f1229) | 1 | 主/因 | [E1707](RULES.md#e1707) | 测试/生命周期方法签名不合格 |
| [F1230](FRONTEND.md#f1230) | 1 | 主/因 | [E1708](RULES.md#e1708) | 重复测试生命周期方法 |
| [F1231](FRONTEND.md#f1231)–F1239 | 9 | 因 | [E1707](RULES.md#e1707) | 测试/生命周期方法签名不合格 |
| [F1240](FRONTEND.md#f1240) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F1241](FRONTEND.md#f1241)–F1246 | 6 | 按边界 | [E1404](RULES.md#e1404) / [ICE0501](RULES.md#ice0501) | core 协议消费与派生内部状态 |
| [F1247](FRONTEND.md#f1247)–F1248 | 2 | 主/因 | [E0617](RULES.md#e0617) | 缺少可用的 companion 条件 |
| [F1249](FRONTEND.md#f1249) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F1250](FRONTEND.md#f1250) | 1 | 主/因 | [E0618](RULES.md#e0618) | 依赖类型投影形式不合法 |
| [F1251](FRONTEND.md#f1251) | 1 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F1252](FRONTEND.md#f1252) | 1 | 主/因 | [E0612](RULES.md#e0612) | 约束主体不合法 |
| [F1253](FRONTEND.md#f1253) | 1 | 主/因 | [E0611](RULES.md#e0611) | 约束形式不适用于声明 |
| [F1254](FRONTEND.md#f1254) | 1 | 主/因 | [E0613](RULES.md#e0613) | 上界类型种类错误 |
| [F1255](FRONTEND.md#f1255) | 1 | 主/因 | [E0611](RULES.md#e0611) | 约束形式不适用于声明 |
| [F1256](FRONTEND.md#f1256) | 1 | 主/因 | [E0614](RULES.md#e0614) | 泛型约束相互冲突 |
| [F1257](FRONTEND.md#f1257) | 1 | 主/因 | [E0615](RULES.md#e0615) | 重复约束 |
| [F1258](FRONTEND.md#f1258) | 1 | 主/因 | [E0613](RULES.md#e0613) | 上界类型种类错误 |
| [F1259](FRONTEND.md#f1259)–F1260 | 2 | 主/因 | [E0614](RULES.md#e0614) | 泛型约束相互冲突 |
| [F1261](FRONTEND.md#f1261) | 1 | 主/因 | [E0615](RULES.md#e0615) | 重复约束 |
| [F1262](FRONTEND.md#f1262) | 1 | 主/因 | [E0613](RULES.md#e0613) | 上界类型种类错误 |
| [F1263](FRONTEND.md#f1263) | 1 | 主/因 | [E0615](RULES.md#e0615) | 重复约束 |
| [F1264](FRONTEND.md#f1264) | 1 | 主/因 | [E0614](RULES.md#e0614) | 泛型约束相互冲突 |
| [F1265](FRONTEND.md#f1265) | 1 | 主/因 | [E0611](RULES.md#e0611) | 约束形式不适用于声明 |
| [F1266](FRONTEND.md#f1266) | 1 | 主/因 | [E0615](RULES.md#e0615) | 重复约束 |
| [F1267](FRONTEND.md#f1267) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1268](FRONTEND.md#f1268) | 1 | 主/因 | [E0612](RULES.md#e0612) | 约束主体不合法 |
| [F1269](FRONTEND.md#f1269)–F1270 | 2 | 主/因 | [E0623](RULES.md#e0623) | member requirement 声明形态不合法 |
| [F1271](FRONTEND.md#f1271)–F1273 | 3 | 主/因 | [E0612](RULES.md#e0612) | 约束主体不合法 |
| [F1274](FRONTEND.md#f1274)–F1276 | 3 | 主/因 | [E1118](RULES.md#e1118) | 效果注解组合不合法 |
| [F1277](FRONTEND.md#f1277) | 1 | 主/因 | [E0612](RULES.md#e0612) | 约束主体不合法 |
| [F1278](FRONTEND.md#f1278) | 1 | 附注/建议 | 名义类型不变性的解释 | 名义类型不变性的解释 |
| [F1279](FRONTEND.md#f1279) | 1 | 主/因 | [E0602](RULES.md#e0602) | 不满足 kind bound |
| [F1280](FRONTEND.md#f1280) | 1 | 主/因 | [E0603](RULES.md#e0603) | 不满足名义上界 |
| [F1281](FRONTEND.md#f1281) | 1 | 主/因 | [E0509](RULES.md#e0509) | 值布局无限递归 |
| [F1282](FRONTEND.md#f1282) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F1283](FRONTEND.md#f1283)–F1284 | 2 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F1285](FRONTEND.md#f1285) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F1286](FRONTEND.md#f1286) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F1287](FRONTEND.md#f1287)–F1288 | 2 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F1289](FRONTEND.md#f1289) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [F1290](FRONTEND.md#f1290) | 1 | 主/因 | [E0403](RULES.md#e0403) | 声明不可访问 |
| [F1291](FRONTEND.md#f1291) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F1292](FRONTEND.md#f1292) | 1 | 主/因 | [E0402](RULES.md#e0402) | 成员不存在 |
| [F1293](FRONTEND.md#f1293)–F1297 | 5 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F1298](FRONTEND.md#f1298)–F1299 | 2 | 主/因 | [E0619](RULES.md#e0619) | 静态声明不能使用外层类型参数 |
| [F1300](FRONTEND.md#f1300)–F1308 | 9 | 主/因 | [E0601](RULES.md#e0601) | 类型实参数量不符 |
| [F1309](FRONTEND.md#f1309) | 1 | 主/因 | [E1404](RULES.md#e1404) | 缺少所需 core 协议声明 |
| [F1310](FRONTEND.md#f1310) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1311](FRONTEND.md#f1311) | 1 | 主/因 | [E1304](RULES.md#e1304) | 注解值不是允许的常量形式 |
| [F1312](FRONTEND.md#f1312) | 1 | 主/因 | [E1303](RULES.md#e1303) | 注解组合冲突 |
| [F1313](FRONTEND.md#f1313) | 1 | 主/因 | [E0405](RULES.md#e0405) | 名称种类不适用于当前位置 |
| [F1314](FRONTEND.md#f1314) | 1 | 主/因 | [E0401](RULES.md#e0401) | 名称未解析 |
| [F1315](FRONTEND.md#f1315) | 1 | 主/因 | [E0702](RULES.md#e0702) | 未知命名实参 |
| [F1316](FRONTEND.md#f1316) | 1 | 主/因 | [E0701](RULES.md#e0701) | 调用实参数量不符 |
| [F1317](FRONTEND.md#f1317) | 1 | 主/因 | [E0703](RULES.md#e0703) | 形参被重复传入 |
| [F1318](FRONTEND.md#f1318) | 1 | 主/因 | [E0708](RULES.md#e0708) | 缺少必填实参 |
| [F1319](FRONTEND.md#f1319) | 1 | 主/因 | [E1304](RULES.md#e1304) | 注解值不是允许的常量形式 |
| [F1320](FRONTEND.md#f1320)–F1321 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1322](FRONTEND.md#f1322) | 1 | 主/因 | [E1304](RULES.md#e1304) | 注解值不是允许的常量形式 |
| [F1323](FRONTEND.md#f1323) | 1 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1324](FRONTEND.md#f1324) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F1325](FRONTEND.md#f1325) | 1 | 主/因 | [E1302](RULES.md#e1302) | 非 repeatable 注解被重复使用 |
| [F1326](FRONTEND.md#f1326)–F1328 | 3 | 主/因 | [E1308](RULES.md#e1308) | Target 注解的目标集合不合法 |
| [F1329](FRONTEND.md#f1329) | 1 | 主/因 | [E1306](RULES.md#e1306) | 注解参数、默认值或类型注解循环 |
| [F1330](FRONTEND.md#f1330) | 1 | 主/因 | [E0301](RULES.md#e0301) | 重复声明 |
| [F1331](FRONTEND.md#f1331) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F1332](FRONTEND.md#f1332) | 1 | 主/因 | [E1303](RULES.md#e1303) | 注解组合冲突 |
| [F1333](FRONTEND.md#f1333)–F1334 | 2 | 主/因 | [E1309](RULES.md#e1309) | Serialize 选项不适用于该目标 |
| [F1335](FRONTEND.md#f1335)–F1336 | 2 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F1337](FRONTEND.md#f1337) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F1338](FRONTEND.md#f1338) | 1 | 主/因 | [E1306](RULES.md#e1306) | 注解参数、默认值或类型注解循环 |
| [F1339](FRONTEND.md#f1339)–F1340 | 2 | 主/因 | [E1305](RULES.md#e1305) | 注解参数类型不合法 |
| [F1341](FRONTEND.md#f1341) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [F1342](FRONTEND.md#f1342) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F1343](FRONTEND.md#f1343)–F1344 | 2 | 主/因 | [E0501](RULES.md#e0501) | 类型不匹配 |
| [F1345](FRONTEND.md#f1345) | 1 | 主/因 | [E1304](RULES.md#e1304) | 注解值不是允许的常量形式 |
| [F1346](FRONTEND.md#f1346)–F1348 | 3 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1349](FRONTEND.md#f1349) | 1 | 主/因 | [E0410](RULES.md#e0410) | 签名泄露不可访问或局部类型 |
| [F1350](FRONTEND.md#f1350) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F1351](FRONTEND.md#f1351) | 1 | 主/因 | [E0704](RULES.md#e0704) | 实参顺序不满足映射规则 |
| [F1352](FRONTEND.md#f1352) | 1 | 主/因 | [E0310](RULES.md#e0310) | 声明类别不允许类型参数 |
| [F1353](FRONTEND.md#f1353) | 1 | 主/因 | [E1307](RULES.md#e1307) | 注解声明形状不合法 |
| [F1354](FRONTEND.md#f1354)–F1355 | 2 | 主/因 | [E1304](RULES.md#e1304) | 注解值不是允许的常量形式 |
| [F1356](FRONTEND.md#f1356) | 1 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1357](FRONTEND.md#f1357) | 1 | 主/因 | [E0719](RULES.md#e0719) | 一个参数列表含多个 vararg |
| [F1358](FRONTEND.md#f1358) | 1 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1359](FRONTEND.md#f1359) | 1 | 主/因 | [E0204](RULES.md#e0204) | 语法列表不允许为空 |
| [F1360](FRONTEND.md#f1360)–F1361 | 2 | 主/因 | [E1117](RULES.md#e1117) | context 声明/绑定形状不合法 |
| [F1362](FRONTEND.md#f1362) | 1 | 主/因 | [E1507](RULES.md#e1507) | 值类型属性形态不合法 |
| [F1363](FRONTEND.md#f1363) | 1 | 主/因 | [E0724](RULES.md#e0724) | 该字段形式不允许默认值 |
| [F1364](FRONTEND.md#f1364)–F1365 | 2 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1366](FRONTEND.md#f1366) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1367](FRONTEND.md#f1367)–F1369 | 3 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1370](FRONTEND.md#f1370) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1371](FRONTEND.md#f1371) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1372](FRONTEND.md#f1372) | 1 | 主/因 | [E0307](RULES.md#e0307) | 扩展声明位置不合法 |
| [F1373](FRONTEND.md#f1373) | 1 | 主/因 | [E0309](RULES.md#e0309) | 抽象声明不允许函数体 |
| [F1374](FRONTEND.md#f1374) | 1 | 主/因 | [E0616](RULES.md#e0616) | 名义泛型不允许变型投影 |
| [F1375](FRONTEND.md#f1375) | 1 | 主/因 | [E0618](RULES.md#e0618) | 依赖类型投影形式不合法 |
| [F1376](FRONTEND.md#f1376) | 1 | 主/因 | [E0204](RULES.md#e0204) | 语法列表不允许为空 |
| [F1377](FRONTEND.md#f1377) | 1 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1378](FRONTEND.md#f1378) | 1 | 主/因 | [E0623](RULES.md#e0623) | member requirement 声明形态不合法 |
| [F1379](FRONTEND.md#f1379) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1380](FRONTEND.md#f1380)–F1381 | 2 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1382](FRONTEND.md#f1382) | 1 | 主/因 | [E1004](RULES.md#e1004) | 父类型种类不合法 |
| [F1383](FRONTEND.md#f1383) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1384](FRONTEND.md#f1384) | 1 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1385](FRONTEND.md#f1385)–F1387 | 3 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1388](FRONTEND.md#f1388) | 1 | 主/因 | [E0310](RULES.md#e0310) | 声明类别不允许类型参数 |
| [F1389](FRONTEND.md#f1389) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1390](FRONTEND.md#f1390) | 1 | 主/因 | [E1001](RULES.md#e1001) | 构造器委托路径不合法 |
| [F1391](FRONTEND.md#f1391) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1392](FRONTEND.md#f1392) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1393](FRONTEND.md#f1393) | 1 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1394](FRONTEND.md#f1394) | 1 | 主/因 | [E1507](RULES.md#e1507) | 值类型属性形态不合法 |
| [F1395](FRONTEND.md#f1395) | 1 | 主/因 | [E1015](RULES.md#e1015) | 覆盖成员缺少 override |
| [F1396](FRONTEND.md#f1396) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1397](FRONTEND.md#f1397)–F1400 | 4 | 主/因 | [E1024](RULES.md#e1024) | 条件接口实现的语法/成员限制不满足 |
| [F1401](FRONTEND.md#f1401) | 1 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1402](FRONTEND.md#f1402) | 1 | 主/因 | [E0310](RULES.md#e0310) | 声明类别不允许类型参数 |
| [F1403](FRONTEND.md#f1403) | 1 | 主/因 | [E1024](RULES.md#e1024) | 条件接口实现的语法/成员限制不满足 |
| [F1404](FRONTEND.md#f1404)–F1406 | 3 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1407](FRONTEND.md#f1407)–F1408 | 2 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1409](FRONTEND.md#f1409)–F1410 | 2 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1411](FRONTEND.md#f1411) | 1 | 主/因 | [E1301](RULES.md#e1301) | 注解目标或 use-site 不合法 |
| [F1412](FRONTEND.md#f1412)–F1414 | 3 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1415](FRONTEND.md#f1415) | 1 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1416](FRONTEND.md#f1416) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1417](FRONTEND.md#f1417)–F1420 | 4 | 主/因 | [E0311](RULES.md#e0311) | 成员或声明形态不适用于所属类型 |
| [F1421](FRONTEND.md#f1421) | 1 | 主/因 | [E1117](RULES.md#e1117) | context 声明/绑定形状不合法 |
| [F1422](FRONTEND.md#f1422)–F1426 | 5 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1427](FRONTEND.md#f1427) | 1 | 主/因 | [E0306](RULES.md#e0306) | 不支持 lateinit 声明 |
| [F1428](FRONTEND.md#f1428)–F1430 | 3 | 主/因 | [E0303](RULES.md#e0303) | 重复修饰符 |
| [F1431](FRONTEND.md#f1431) | 1 | 主/因 | [E0304](RULES.md#e0304) | 互斥修饰符组合 |
| [F1432](FRONTEND.md#f1432)–F1433 | 2 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1434](FRONTEND.md#f1434) | 1 | 主/因 | [E0304](RULES.md#e0304) | 互斥修饰符组合 |
| [F1435](FRONTEND.md#f1435)–F1436 | 2 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1437](FRONTEND.md#f1437) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1438](FRONTEND.md#f1438) | 1 | 主/因 | [E1109](RULES.md#e1109) | release 所属类型不合法 |
| [F1439](FRONTEND.md#f1439)–F1441 | 3 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1442](FRONTEND.md#f1442) | 1 | 主/因 | [E0309](RULES.md#e0309) | 抽象声明不允许函数体 |
| [F1443](FRONTEND.md#f1443) | 1 | 主/因 | [E1515](RULES.md#e1515) | 该属性位置不允许存储或委托 |
| [F1444](FRONTEND.md#f1444) | 1 | 主/因 | [E1513](RULES.md#e1513) | 重复 accessor |
| [F1445](FRONTEND.md#f1445) | 1 | 主/因 | [E1512](RULES.md#e1512) | accessor 可见性不合法 |
| [F1446](FRONTEND.md#f1446) | 1 | 主/因 | [E1513](RULES.md#e1513) | 重复 accessor |
| [F1447](FRONTEND.md#f1447)–F1448 | 2 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1449](FRONTEND.md#f1449) | 1 | 主/因 | [E1117](RULES.md#e1117) | context 声明/绑定形状不合法 |
| [F1450](FRONTEND.md#f1450) | 1 | 主/因 | [E0204](RULES.md#e0204) | 语法列表不允许为空 |
| [F1451](FRONTEND.md#f1451)–F1454 | 4 | 主/因 | [E0205](RULES.md#e0205) | copy update 字段形式不合法 |
| [F1455](FRONTEND.md#f1455) | 1 | 主/因 | [E0206](RULES.md#e0206) | 数组切片语法不受支持 |
| [F1456](FRONTEND.md#f1456)–F1457 | 2 | 主/因 | [E1506](RULES.md#e1506) | 写入目标不可赋值 |
| [F1458](FRONTEND.md#f1458) | 1 | 主/因 | [E1504](RULES.md#e1504) | copy update 目标不是精确 struct 值 |
| [F1459](FRONTEND.md#f1459) | 1 | 主/因 | [E1025](RULES.md#e1025) | super 目标或形式不合法 |
| [F1460](FRONTEND.md#f1460) | 1 | 主/因 | [E0713](RULES.md#e0713) | 该声明形式不能成为函数引用 |
| [F1461](FRONTEND.md#f1461)–F1463 | 3 | 主/因 | [E1025](RULES.md#e1025) | super 目标或形式不合法 |
| [F1464](FRONTEND.md#f1464) | 1 | 主/因 | [E0207](RULES.md#e0207) | 类型占位符使用位置不合法 |
| [F1465](FRONTEND.md#f1465) | 1 | 主/因 | [E0718](RULES.md#e0718) | 同一调用有多个尾随 lambda |
| [F1466](FRONTEND.md#f1466)–F1467 | 2 | 主/因 | [E0408](RULES.md#e0408) | 星号导入形式不合法 |
| [F1468](FRONTEND.md#f1468) | 1 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1469](FRONTEND.md#f1469) | 1 | 主/因 | [E0102](RULES.md#e0102) | 未闭合的词法结构 |
| [F1470](FRONTEND.md#f1470) | 1 | 主/因 | [E0101](RULES.md#e0101) | 无法识别的字符 |
| [F1471](FRONTEND.md#f1471) | 1 | 主/因 | [E0102](RULES.md#e0102) | 未闭合的词法结构 |
| [F1472](FRONTEND.md#f1472) | 1 | 主/因 | [E0106](RULES.md#e0106) | 数值字面量后缀无效 |
| [F1473](FRONTEND.md#f1473) | 1 | 主/因 | [E0104](RULES.md#e0104) | 数字分隔符位置无效 |
| [F1474](FRONTEND.md#f1474) | 1 | 主/因 | [E0105](RULES.md#e0105) | 数字组成不完整或进制不符 |
| [F1475](FRONTEND.md#f1475) | 1 | 主/因 | [E0103](RULES.md#e0103) | 无效的转义标识符 |
| [F1476](FRONTEND.md#f1476) | 1 | 主/因 | [E0102](RULES.md#e0102) | 未闭合的词法结构 |
| [F1477](FRONTEND.md#f1477) | 1 | 主/因 | [E0103](RULES.md#e0103) | 无效的转义标识符 |
| [F1478](FRONTEND.md#f1478) | 1 | 主/因 | [E0104](RULES.md#e0104) | 数字分隔符位置无效 |
| [F1479](FRONTEND.md#f1479)–F1480 | 2 | 主/因 | [E0105](RULES.md#e0105) | 数字组成不完整或进制不符 |
| [F1481](FRONTEND.md#f1481) | 1 | 主/因 | [E0106](RULES.md#e0106) | 数值字面量后缀无效 |
| [F1482](FRONTEND.md#f1482) | 1 | 主/因 | [E0107](RULES.md#e0107) | 数值字面量超出表示范围 |
| [F1483](FRONTEND.md#f1483) | 1 | 主/因 | [E0102](RULES.md#e0102) | 未闭合的词法结构 |
| [F1484](FRONTEND.md#f1484) | 1 | 主/因 | [E0110](RULES.md#e0110) | 插值缺少花括号 |
| [F1485](FRONTEND.md#f1485) | 1 | 主/因 | [E0102](RULES.md#e0102) | 未闭合的词法结构 |
| [F1486](FRONTEND.md#f1486) | 1 | 主/因 | [E0108](RULES.md#e0108) | 字符字面量不是一个 Unicode 标量 |
| [F1487](FRONTEND.md#f1487)–F1488 | 2 | 主/因 | [E0102](RULES.md#e0102) | 未闭合的词法结构 |
| [F1489](FRONTEND.md#f1489)–F1491 | 3 | 主/因 | [E0109](RULES.md#e0109) | 无效转义 |
| [F1492](FRONTEND.md#f1492) | 1 | 主/因 | [E0202](RULES.md#e0202) | 重复 package 声明 |
| [F1493](FRONTEND.md#f1493)–F1494 | 2 | 主/因 | [E0203](RULES.md#e0203) | 文件头顺序不合法 |
| [F1495](FRONTEND.md#f1495) | 1 | 主/因 | [E0305](RULES.md#e0305) | 修饰符不适用于该声明 |
| [F1496](FRONTEND.md#f1496)–F1497 | 2 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1498](FRONTEND.md#f1498) | 1 | 主/因 | [E0908](RULES.md#e0908) | 模式字段名称形式不合法 |
| [F1499](FRONTEND.md#f1499)–F1500 | 2 | 主/因 | [E0903](RULES.md#e0903) | 模式 rest 位置或次数不合法 |
| [F1501](FRONTEND.md#f1501) | 1 | 主/因 | [E2005](RULES.md#e2005) | 重复选择同一源码 |
| [F1502](FRONTEND.md#f1502) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [F1503](FRONTEND.md#f1503)–F1506 | 4 | 主/因 | [E1118](RULES.md#e1118) | 效果注解组合不合法 |
| [F1507](FRONTEND.md#f1507) | 1 | 主/因 | [E1526](RULES.md#e1526) | 局部委托属性不是简单名称 |
| [F1508](FRONTEND.md#f1508) | 1 | 主/因 | [E0814](RULES.md#e0814) | catch 缺少类型注解 |
| [F1509](FRONTEND.md#f1509) | 1 | 主/因 | [E0912](RULES.md#e0912) | 模式或简写条件缺少 subject |
| [F1510](FRONTEND.md#f1510) | 1 | 主/因 | [E0913](RULES.md#e0913) | 多条件分支不能同时带 guard |
| [F1511](FRONTEND.md#f1511) | 1 | 主/因 | [E0912](RULES.md#e0912) | 模式或简写条件缺少 subject |
| [F1512](FRONTEND.md#f1512) | 1 | 主/因 | [E0914](RULES.md#e0914) | case arm 只能包含一个模式 |
| [F1513](FRONTEND.md#f1513) | 1 | 主/因 | [E0915](RULES.md#e0915) | 匹配模式缺少 case |
| [F1514](FRONTEND.md#f1514)–F1515 | 2 | 主/因 | [E0912](RULES.md#e0912) | 模式或简写条件缺少 subject |
| [F1516](FRONTEND.md#f1516)–F1517 | 2 | 主/因 | [E0916](RULES.md#e0916) | when subject 绑定形态不合法 |
| [F1518](FRONTEND.md#f1518)–F1519 | 2 | 主/因 | [E0201](RULES.md#e0201) | 缺少预期的语法项 |
| [F1520](FRONTEND.md#f1520) | 1 | 主/因 | [E0208](RULES.md#e0208) | suspend 缺少函数类型 |
| [F1521](FRONTEND.md#f1521) | 1 | 主/因 | [E0207](RULES.md#e0207) | 类型占位符使用位置不合法 |
| [F1522](FRONTEND.md#f1522)–F1524 | 3 | 主/因 | [E0616](RULES.md#e0616) | 名义泛型不允许变型投影 |

## B · BACKEND.md

| 原条目 | 条数 | 角色 | 候选编号/处置 | 归属 |
| --- | --- | --- | --- | --- |
| [B0001](BACKEND.md#b0001) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [B0002](BACKEND.md#b0002) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [B0003](BACKEND.md#b0003) | 1 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0004](BACKEND.md#b0004)–B0012 | 9 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0013](BACKEND.md#b0013)–B0021 | 9 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0022](BACKEND.md#b0022)–B0025 | 4 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [B0026](BACKEND.md#b0026) | 1 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0027](BACKEND.md#b0027)–B0069 | 43 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0070](BACKEND.md#b0070)–B0075 | 6 | 内部 | [ICE1101](RULES.md#ice1101) | GC/root/statepoint 合同不一致 |
| [B0076](BACKEND.md#b0076)–B0107 | 32 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0108](BACKEND.md#b0108)–B0109 | 2 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0110](BACKEND.md#b0110)–B0143 | 34 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0144](BACKEND.md#b0144)–B0160 | 17 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0161](BACKEND.md#b0161)–B0175 | 15 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0176](BACKEND.md#b0176)–B0281 | 106 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0282](BACKEND.md#b0282)–B0367 | 86 | 内部 | [ICE1101](RULES.md#ice1101) | GC/root/statepoint 合同不一致 |
| [B0368](BACKEND.md#b0368)–B0571 | 204 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0572](BACKEND.md#b0572)–B0611 | 40 | 内部 | [ICE1101](RULES.md#ice1101) | GC/root/statepoint 合同不一致 |
| [B0612](BACKEND.md#b0612) | 1 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0613](BACKEND.md#b0613)–B0619 | 7 | 内部 | [ICE1101](RULES.md#ice1101) | GC/root/statepoint 合同不一致 |
| [B0620](BACKEND.md#b0620)–B0642 | 23 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0643](BACKEND.md#b0643)–B0678 | 36 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0679](BACKEND.md#b0679)–B0703 | 25 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [B0704](BACKEND.md#b0704)–B0715 | 12 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0716](BACKEND.md#b0716)–B0770 | 55 | 内部 | [ICE1101](RULES.md#ice1101) | GC/root/statepoint 合同不一致 |
| [B0771](BACKEND.md#b0771) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [B0772](BACKEND.md#b0772) | 1 | 按边界 | [E2404](RULES.md#e2404) / [ICE2201](RULES.md#ice2201) | target 合同的消费/内部边界 |
| [B0773](BACKEND.md#b0773)–B0774 | 2 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [B0775](BACKEND.md#b0775) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [B0776](BACKEND.md#b0776)–B0777 | 2 | 按边界 | [E2404](RULES.md#e2404) / [ICE2201](RULES.md#ice2201) | target 合同的消费/内部边界 |
| [B0778](BACKEND.md#b0778)–B0781 | 4 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [B0782](BACKEND.md#b0782)–B0793 | 12 | 内部 | [ICE2301](RULES.md#ice2301) | LLVM 生成不变量失败 |
| [B0794](BACKEND.md#b0794)–B0795 | 2 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [B0796](BACKEND.md#b0796) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [B0797](BACKEND.md#b0797)–B0850 | 54 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [B0851](BACKEND.md#b0851)–B0865 | 15 | 内部 | [ICE1101](RULES.md#ice1101) | GC/root/statepoint 合同不一致 |
| [B0866](BACKEND.md#b0866) | 1 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [B0867](BACKEND.md#b0867) | 1 | 内部 | [ICE0701](RULES.md#ice0701) | MIR 变换或调用桥接不一致 |
| [B0868](BACKEND.md#b0868) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [B0869](BACKEND.md#b0869)–B0880 | 12 | 内部 | [ICE0701](RULES.md#ice0701) | MIR 变换或调用桥接不一致 |

## A · ARTIFACTS.md

| 原条目 | 条数 | 角色 | 候选编号/处置 | 归属 |
| --- | --- | --- | --- | --- |
| [A0001](ARTIFACTS.md#a0001)–A0007 | 7 | 内部 | [ICE0301](RULES.md#ice0301) | 阶段输入混用了不同上下文 |
| [A0008](ARTIFACTS.md#a0008)–A0009 | 2 | 主/因 | [E1607](RULES.md#e1607) | 编译期环境变量名称不合法 |
| [A0010](ARTIFACTS.md#a0010) | 1 | 主/因 | [E1605](RULES.md#e1605) | 编译期输入路径不合法 |
| [A0011](ARTIFACTS.md#a0011)–A0049 | 39 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [A0050](ARTIFACTS.md#a0050)–A0178 | 129 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A0179](ARTIFACTS.md#a0179)–A0470 | 292 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [A0471](ARTIFACTS.md#a0471)–A0551 | 81 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A0552](ARTIFACTS.md#a0552)–A0580 | 29 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [A0581](ARTIFACTS.md#a0581) | 1 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | wire 作为产物/协议子错误的边界 |
| [A0582](ARTIFACTS.md#a0582)–A0583 | 2 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [A0584](ARTIFACTS.md#a0584)–A0960 | 377 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A0961](ARTIFACTS.md#a0961)–A0965 | 5 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A0966](ARTIFACTS.md#a0966)–A0973 | 8 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A0974](ARTIFACTS.md#a0974)–A1001 | 28 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | wire 作为产物/协议子错误的边界 |
| [A1002](ARTIFACTS.md#a1002) | 1 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [A1003](ARTIFACTS.md#a1003)–A1004 | 2 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A1005](ARTIFACTS.md#a1005) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1006](ARTIFACTS.md#a1006)–A1021 | 16 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A1022](ARTIFACTS.md#a1022)–A1062 | 41 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [A1063](ARTIFACTS.md#a1063)–A1183 | 121 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1184](ARTIFACTS.md#a1184) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1185](ARTIFACTS.md#a1185)–A1257 | 73 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1258](ARTIFACTS.md#a1258)–A1600 | 343 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A1601](ARTIFACTS.md#a1601)–A1604 | 4 | 内部 | [ICE0501](RULES.md#ice0501) | HIR 完备性或已解析语义关系被破坏 |
| [A1605](ARTIFACTS.md#a1605)–A1642 | 38 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A1643](ARTIFACTS.md#a1643) | 1 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1644](ARTIFACTS.md#a1644) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1645](ARTIFACTS.md#a1645)–A1655 | 11 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1656](ARTIFACTS.md#a1656) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1657](ARTIFACTS.md#a1657)–A1661 | 5 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1662](ARTIFACTS.md#a1662)–A1665 | 4 | 排版 | 位置/候选链的排版片段 | 位置/候选链的排版片段 |
| [A1666](ARTIFACTS.md#a1666) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [A1667](ARTIFACTS.md#a1667)–A1696 | 30 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1697](ARTIFACTS.md#a1697) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1698](ARTIFACTS.md#a1698)–A1706 | 9 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1707](ARTIFACTS.md#a1707)–A1708 | 2 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1709](ARTIFACTS.md#a1709)–A1721 | 13 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1722](ARTIFACTS.md#a1722) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1723](ARTIFACTS.md#a1723)–A1728 | 6 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1729](ARTIFACTS.md#a1729) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1730](ARTIFACTS.md#a1730)–A1734 | 5 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1735](ARTIFACTS.md#a1735) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1736](ARTIFACTS.md#a1736)–A1748 | 13 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1749](ARTIFACTS.md#a1749) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1750](ARTIFACTS.md#a1750)–A1781 | 32 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1782](ARTIFACTS.md#a1782) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1783](ARTIFACTS.md#a1783)–A1786 | 4 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1787](ARTIFACTS.md#a1787) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1788](ARTIFACTS.md#a1788)–A1803 | 16 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1804](ARTIFACTS.md#a1804) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1805](ARTIFACTS.md#a1805)–A1806 | 2 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1807](ARTIFACTS.md#a1807) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1808](ARTIFACTS.md#a1808)–A1815 | 8 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1816](ARTIFACTS.md#a1816) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1817](ARTIFACTS.md#a1817)–A1831 | 15 | 按边界 | [E2405](RULES.md#e2405) / [ICE0401](RULES.md#ice0401) | 实体/源码引用校验的边界 |
| [A1832](ARTIFACTS.md#a1832)–A1833 | 2 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A1834](ARTIFACTS.md#a1834)–A1835 | 2 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1836](ARTIFACTS.md#a1836) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [A1837](ARTIFACTS.md#a1837) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [A1838](ARTIFACTS.md#a1838) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1839](ARTIFACTS.md#a1839)–A1845 | 7 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1846](ARTIFACTS.md#a1846) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1847](ARTIFACTS.md#a1847)–A1849 | 3 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1850](ARTIFACTS.md#a1850)–A1851 | 2 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1852](ARTIFACTS.md#a1852) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1853](ARTIFACTS.md#a1853) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1854](ARTIFACTS.md#a1854)–A1876 | 23 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1877](ARTIFACTS.md#a1877)–A1878 | 2 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [A1879](ARTIFACTS.md#a1879) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [A1880](ARTIFACTS.md#a1880) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [A1881](ARTIFACTS.md#a1881) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [A1882](ARTIFACTS.md#a1882) | 1 | 主/因 | [E2304](RULES.md#e2304) | 最终链接失败 |
| [A1883](ARTIFACTS.md#a1883) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [A1884](ARTIFACTS.md#a1884) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [A1885](ARTIFACTS.md#a1885) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [A1886](ARTIFACTS.md#a1886) | 1 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [A1887](ARTIFACTS.md#a1887) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [A1888](ARTIFACTS.md#a1888) | 1 | 主/因 | [E2304](RULES.md#e2304) | 最终链接失败 |
| [A1889](ARTIFACTS.md#a1889)–A1890 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [A1891](ARTIFACTS.md#a1891) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [A1892](ARTIFACTS.md#a1892) | 1 | 主/因 | [E2304](RULES.md#e2304) | 最终链接失败 |
| [A1893](ARTIFACTS.md#a1893) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [A1894](ARTIFACTS.md#a1894)–A1903 | 10 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1904](ARTIFACTS.md#a1904)–A1906 | 3 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [A1907](ARTIFACTS.md#a1907) | 1 | 主/因 | [E2307](RULES.md#e2307) | runtime 配置或 ABI 不相容 |
| [A1908](ARTIFACTS.md#a1908)–A1910 | 3 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1911](ARTIFACTS.md#a1911)–A1912 | 2 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1913](ARTIFACTS.md#a1913) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1914](ARTIFACTS.md#a1914)–A1915 | 2 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1916](ARTIFACTS.md#a1916) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1917](ARTIFACTS.md#a1917) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1918](ARTIFACTS.md#a1918) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1919](ARTIFACTS.md#a1919) | 1 | 主/因 | [E2308](RULES.md#e2308) | native 符号未解析 |
| [A1920](ARTIFACTS.md#a1920)–A1922 | 3 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1923](ARTIFACTS.md#a1923) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1924](ARTIFACTS.md#a1924) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1925](ARTIFACTS.md#a1925)–A1928 | 4 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1929](ARTIFACTS.md#a1929) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1930](ARTIFACTS.md#a1930)–A1943 | 14 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1944](ARTIFACTS.md#a1944) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [A1945](ARTIFACTS.md#a1945) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [A1946](ARTIFACTS.md#a1946) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1947](ARTIFACTS.md#a1947) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1948](ARTIFACTS.md#a1948) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1949](ARTIFACTS.md#a1949)–A1950 | 2 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1951](ARTIFACTS.md#a1951) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1952](ARTIFACTS.md#a1952) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [A1953](ARTIFACTS.md#a1953) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [A1954](ARTIFACTS.md#a1954) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [A1955](ARTIFACTS.md#a1955) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [A1956](ARTIFACTS.md#a1956)–A1963 | 8 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1964](ARTIFACTS.md#a1964) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [A1965](ARTIFACTS.md#a1965) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1966](ARTIFACTS.md#a1966) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1967](ARTIFACTS.md#a1967)–A1969 | 3 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1970](ARTIFACTS.md#a1970) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1971](ARTIFACTS.md#a1971) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [A1972](ARTIFACTS.md#a1972) | 1 | 主/因 | [E2305](RULES.md#e2305) | native 链接输入无效 |
| [A1973](ARTIFACTS.md#a1973)–A1974 | 2 | 按边界 | [E2404](RULES.md#e2404) / [ICE2201](RULES.md#ice2201) | target 合同的消费/内部边界 |
| [A1975](ARTIFACTS.md#a1975) | 1 | 按边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | native 记录读写的边界 |
| [A1976](ARTIFACTS.md#a1976) | 1 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A1977](ARTIFACTS.md#a1977) | 1 | 主/因 | [E2412](RULES.md#e2412) | 跨产物定义或链接合同冲突 |
| [A1978](ARTIFACTS.md#a1978) | 1 | 按边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | native 记录读写的边界 |
| [A1979](ARTIFACTS.md#a1979)–A1980 | 2 | 主/因 | [E2412](RULES.md#e2412) | 跨产物定义或链接合同冲突 |
| [A1981](ARTIFACTS.md#a1981)–A1982 | 2 | 按边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | native 记录读写的边界 |
| [A1983](ARTIFACTS.md#a1983) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A1984](ARTIFACTS.md#a1984)–A1991 | 8 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1992](ARTIFACTS.md#a1992)–A1994 | 3 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A1995](ARTIFACTS.md#a1995) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A1996](ARTIFACTS.md#a1996)–A1998 | 3 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [A1999](ARTIFACTS.md#a1999)–A2000 | 2 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A2001](ARTIFACTS.md#a2001) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A2002](ARTIFACTS.md#a2002) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A2003](ARTIFACTS.md#a2003)–A2004 | 2 | 主/因 | [E2308](RULES.md#e2308) | native 符号未解析 |
| [A2005](ARTIFACTS.md#a2005) | 1 | 主/因 | [E2306](RULES.md#e2306) | native 符号/namespace/ODR 冲突 |
| [A2006](ARTIFACTS.md#a2006)–A2009 | 4 | 按边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | native 记录读写的边界 |
| [A2010](ARTIFACTS.md#a2010) | 1 | 主/因 | [E2412](RULES.md#e2412) | 跨产物定义或链接合同冲突 |
| [A2011](ARTIFACTS.md#a2011)–A2014 | 4 | 按边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | native 记录读写的边界 |
| [A2015](ARTIFACTS.md#a2015)–A2026 | 12 | 主/因 | [E2307](RULES.md#e2307) | runtime 配置或 ABI 不相容 |
| [A2027](ARTIFACTS.md#a2027) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [A2028](ARTIFACTS.md#a2028)–A2030 | 3 | 主/因 | [E2307](RULES.md#e2307) | runtime 配置或 ABI 不相容 |
| [A2031](ARTIFACTS.md#a2031) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [A2032](ARTIFACTS.md#a2032) | 1 | 内部 | [ICE2302](RULES.md#ice2302) | 编译器自产 object 的结构不一致 |
| [A2033](ARTIFACTS.md#a2033)–A2041 | 9 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [A2042](ARTIFACTS.md#a2042)–A2047 | 6 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2048](ARTIFACTS.md#a2048)–A2050 | 3 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [A2051](ARTIFACTS.md#a2051)–A2062 | 12 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2063](ARTIFACTS.md#a2063) | 1 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [A2064](ARTIFACTS.md#a2064)–A2166 | 103 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2167](ARTIFACTS.md#a2167)–A2173 | 7 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [A2174](ARTIFACTS.md#a2174)–A2227 | 54 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2228](ARTIFACTS.md#a2228)–A2234 | 7 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [A2235](ARTIFACTS.md#a2235)–A2236 | 2 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2237](ARTIFACTS.md#a2237)–A2239 | 3 | 内部 | [ICE1201](RULES.md#ice1201) | LIR 布局或调用 ABI 变换不一致 |
| [A2240](ARTIFACTS.md#a2240)–A2243 | 4 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2244](ARTIFACTS.md#a2244)–A2267 | 24 | 内部 | [ICE0701](RULES.md#ice0701) | MIR 变换或调用桥接不一致 |
| [A2268](ARTIFACTS.md#a2268)–A2295 | 28 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [A2296](ARTIFACTS.md#a2296)–A2301 | 6 | 内部 | [ICE0701](RULES.md#ice0701) | MIR 变换或调用桥接不一致 |
| [A2302](ARTIFACTS.md#a2302)–A2316 | 15 | 按边界 | [E2407](RULES.md#e2407) / [ICE0701](RULES.md#ice0701) | 可执行模板/MIR 校验的边界 |
| [A2317](ARTIFACTS.md#a2317)–A2361 | 45 | 内部 | [ICE0701](RULES.md#ice0701) | MIR 变换或调用桥接不一致 |
| [A2362](ARTIFACTS.md#a2362)–A2366 | 5 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2367](ARTIFACTS.md#a2367) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2368](ARTIFACTS.md#a2368) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2369](ARTIFACTS.md#a2369) | 1 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A2370](ARTIFACTS.md#a2370) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [A2371](ARTIFACTS.md#a2371) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A2372](ARTIFACTS.md#a2372) | 1 | 主/因 | [E2410](RULES.md#e2410) | 产物压缩或解压失败 |
| [A2373](ARTIFACTS.md#a2373) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [A2374](ARTIFACTS.md#a2374)–A2378 | 5 | 主/因 | [E2410](RULES.md#e2410) | 产物压缩或解压失败 |
| [A2379](ARTIFACTS.md#a2379)–A2380 | 2 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A2381](ARTIFACTS.md#a2381) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2382](ARTIFACTS.md#a2382)–A2413 | 32 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A2414](ARTIFACTS.md#a2414)–A2417 | 4 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2418](ARTIFACTS.md#a2418)–A2445 | 28 | 按边界 | [E2408](RULES.md#e2408) / [ICE1201](RULES.md#ice1201) | 布局/ABI 校验的边界 |
| [A2446](ARTIFACTS.md#a2446)–A2582 | 137 | 按边界 | [E2409](RULES.md#e2409) / [ICE2302](RULES.md#ice2302) | native 记录读写的边界 |
| [A2583](ARTIFACTS.md#a2583) | 1 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2584](ARTIFACTS.md#a2584) | 1 | 主/因 | [E2402](RULES.md#e2402) | 产物 schema/协议版本不支持 |
| [A2585](ARTIFACTS.md#a2585)–A2588 | 4 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2589](ARTIFACTS.md#a2589) | 1 | 主/因 | [E2404](RULES.md#e2404) | 产物 target/profile/ABI 不匹配 |
| [A2590](ARTIFACTS.md#a2590) | 1 | 主/因 | [E2402](RULES.md#e2402) | 产物 schema/协议版本不支持 |
| [A2591](ARTIFACTS.md#a2591)–A2592 | 2 | 主/因 | [E2404](RULES.md#e2404) | 产物 target/profile/ABI 不匹配 |
| [A2593](ARTIFACTS.md#a2593)–A2595 | 3 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2596](ARTIFACTS.md#a2596)–A2597 | 2 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A2598](ARTIFACTS.md#a2598) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2599](ARTIFACTS.md#a2599)–A2608 | 10 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2609](ARTIFACTS.md#a2609) | 1 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A2610](ARTIFACTS.md#a2610) | 1 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2611](ARTIFACTS.md#a2611) | 1 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A2612](ARTIFACTS.md#a2612)–A2616 | 5 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2617](ARTIFACTS.md#a2617) | 1 | 主/因 | [E2402](RULES.md#e2402) | 产物 schema/协议版本不支持 |
| [A2618](ARTIFACTS.md#a2618)–A2625 | 8 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2626](ARTIFACTS.md#a2626) | 1 | 主/因 | [E2402](RULES.md#e2402) | 产物 schema/协议版本不支持 |
| [A2627](ARTIFACTS.md#a2627)–A2628 | 2 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2629](ARTIFACTS.md#a2629) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2630](ARTIFACTS.md#a2630) | 1 | 主/因 | [E2404](RULES.md#e2404) | 产物 target/profile/ABI 不匹配 |
| [A2631](ARTIFACTS.md#a2631) | 1 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A2632](ARTIFACTS.md#a2632) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [A2633](ARTIFACTS.md#a2633) | 1 | 主/因 | [E2403](RULES.md#e2403) | 产物缺少必需成员或节 |
| [A2634](ARTIFACTS.md#a2634) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [A2635](ARTIFACTS.md#a2635)–A2636 | 2 | 主/因 | [E2412](RULES.md#e2412) | 跨产物定义或链接合同冲突 |
| [A2637](ARTIFACTS.md#a2637) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [A2638](ARTIFACTS.md#a2638) | 1 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [A2639](ARTIFACTS.md#a2639) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2640](ARTIFACTS.md#a2640) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A2641](ARTIFACTS.md#a2641) | 1 | 主/因 | [E2411](RULES.md#e2411) | 产物不提供当前操作所需能力 |
| [A2642](ARTIFACTS.md#a2642) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A2643](ARTIFACTS.md#a2643)–A2644 | 2 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2645](ARTIFACTS.md#a2645) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2646](ARTIFACTS.md#a2646) | 1 | 按边界 | [E2406](RULES.md#e2406) / [ICE0501](RULES.md#ice0501) | 声明接口校验/投影的边界 |
| [A2647](ARTIFACTS.md#a2647)–A2649 | 3 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) | 格式校验的读写边界 |
| [A2650](ARTIFACTS.md#a2650) | 1 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2651](ARTIFACTS.md#a2651)–A2652 | 2 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | wire 作为产物/协议子错误的边界 |
| [A2653](ARTIFACTS.md#a2653)–A2654 | 2 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2655](ARTIFACTS.md#a2655)–A2656 | 2 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2657](ARTIFACTS.md#a2657) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2658](ARTIFACTS.md#a2658)–A2660 | 3 | 内部 | [ICE2401](RULES.md#ice2401) | 编译器自产元数据或产物投影不一致 |
| [A2661](ARTIFACTS.md#a2661)–A2662 | 2 | 排版 | 位置/候选链的排版片段 | 位置/候选链的排版片段 |
| [A2663](ARTIFACTS.md#a2663)–A2675 | 13 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | wire 作为产物/协议子错误的边界 |
| [A2676](ARTIFACTS.md#a2676)–A2678 | 3 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [A2679](ARTIFACTS.md#a2679) | 1 | 排版 | 位置/候选链的排版片段 | 位置/候选链的排版片段 |
| [A2680](ARTIFACTS.md#a2680)–A2681 | 2 | 按边界 | [E2401](RULES.md#e2401) / [ICE2401](RULES.md#ice2401) / [E2502](RULES.md#e2502) | wire 作为产物/协议子错误的边界 |
| [A2682](ARTIFACTS.md#a2682) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |

## D · BUILD.md

| 原条目 | 条数 | 角色 | 候选编号/处置 | 归属 |
| --- | --- | --- | --- | --- |
| [D0001](BUILD.md#d0001)–D0005 | 5 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0006](BUILD.md#d0006)–D0007 | 2 | 主/因 | [E1801](RULES.md#e1801) | 文档资源或站点配置无效 |
| [D0008](BUILD.md#d0008) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0009](BUILD.md#d0009) | 1 | 主/因 | [E2204](RULES.md#e2204) | 子进程通信或等待失败 |
| [D0010](BUILD.md#d0010) | 1 | 主/因 | [E2501](RULES.md#e2501) | 协议版本或配套工具不相容 |
| [D0011](BUILD.md#d0011) | 1 | 主/因 | [E2204](RULES.md#e2204) | 子进程通信或等待失败 |
| [D0012](BUILD.md#d0012) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0013](BUILD.md#d0013) | 1 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0014](BUILD.md#d0014) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0015](BUILD.md#d0015) | 1 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0016](BUILD.md#d0016) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0017](BUILD.md#d0017)–D0019 | 3 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0020](BUILD.md#d0020)–D0024 | 5 | 主/因 | [E1317](RULES.md#e1317) | GenerateWith 配置不合法 |
| [D0025](BUILD.md#d0025) | 1 | 主/因 | [E2509](RULES.md#e2509) | 生成器程序未注册 |
| [D0026](BUILD.md#d0026)–D0030 | 5 | 主/因 | [E2511](RULES.md#e2511) | 外部生成器结果违反协议 |
| [D0031](BUILD.md#d0031) | 1 | 主/因 | [E2510](RULES.md#e2510) | 外部生成器报告失败 |
| [D0032](BUILD.md#d0032) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0033](BUILD.md#d0033) | 1 | 主/因 | [E2511](RULES.md#e2511) | 外部生成器结果违反协议 |
| [D0034](BUILD.md#d0034) | 1 | 主/因 | [E2510](RULES.md#e2510) | 外部生成器报告失败 |
| [D0035](BUILD.md#d0035)–D0038 | 4 | 主/因 | [E2511](RULES.md#e2511) | 外部生成器结果违反协议 |
| [D0039](BUILD.md#d0039) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0040](BUILD.md#d0040) | 1 | 主/因 | [E1317](RULES.md#e1317) | GenerateWith 配置不合法 |
| [D0041](BUILD.md#d0041)–D0042 | 2 | 主/因 | [E1318](RULES.md#e1318) | 生成器重复绑定同一目标 |
| [D0043](BUILD.md#d0043) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0044](BUILD.md#d0044)–D0045 | 2 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0046](BUILD.md#d0046) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0047](BUILD.md#d0047) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0048](BUILD.md#d0048)–D0050 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0051](BUILD.md#d0051) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0052](BUILD.md#d0052) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0053](BUILD.md#d0053)–D0059 | 7 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0060](BUILD.md#d0060)–D0063 | 4 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0064](BUILD.md#d0064)–D0065 | 2 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [D0066](BUILD.md#d0066) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0067](BUILD.md#d0067) | 1 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [D0068](BUILD.md#d0068)–D0070 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0071](BUILD.md#d0071)–D0074 | 4 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0075](BUILD.md#d0075) | 1 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [D0076](BUILD.md#d0076) | 1 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [D0077](BUILD.md#d0077) | 1 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [D0078](BUILD.md#d0078) | 1 | 主/因 | [E2104](RULES.md#e2104) | 依赖闭包包含冲突版本 |
| [D0079](BUILD.md#d0079) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0080](BUILD.md#d0080) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0081](BUILD.md#d0081)–D0082 | 2 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [D0083](BUILD.md#d0083) | 1 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [D0084](BUILD.md#d0084) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0085](BUILD.md#d0085) | 1 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [D0086](BUILD.md#d0086)–D0087 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0088](BUILD.md#d0088) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0089](BUILD.md#d0089)–D0090 | 2 | 主/因 | [E2007](RULES.md#e2007) | 源码 package 不属于当前 Cone group |
| [D0091](BUILD.md#d0091) | 1 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0092](BUILD.md#d0092) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0093](BUILD.md#d0093) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0094](BUILD.md#d0094) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0095](BUILD.md#d0095) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0096](BUILD.md#d0096) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0097](BUILD.md#d0097) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0098](BUILD.md#d0098)–D0100 | 3 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0101](BUILD.md#d0101) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0102](BUILD.md#d0102) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0103](BUILD.md#d0103) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0104](BUILD.md#d0104) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0105](BUILD.md#d0105)–D0109 | 5 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0110](BUILD.md#d0110)–D0112 | 3 | 主/因 | [E2512](RULES.md#e2512) | 生成器查询引用不合法 |
| [D0113](BUILD.md#d0113)–D0116 | 4 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0117](BUILD.md#d0117) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0118](BUILD.md#d0118)–D0120 | 3 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0121](BUILD.md#d0121)–D0122 | 2 | 主/因 | [E2006](RULES.md#e2006) | 源码发现遇到目录循环 |
| [D0123](BUILD.md#d0123) | 1 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0124](BUILD.md#d0124) | 1 | 主/因 | [E2005](RULES.md#e2005) | 重复选择同一源码 |
| [D0125](BUILD.md#d0125) | 1 | 主/因 | [E2003](RULES.md#e2003) | 源码或清单不是 UTF-8 |
| [D0126](BUILD.md#d0126) | 1 | 主/因 | [E2004](RULES.md#e2004) | 没有选择到源码 |
| [D0127](BUILD.md#d0127) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0128](BUILD.md#d0128) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0129](BUILD.md#d0129) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0130](BUILD.md#d0130) | 1 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0131](BUILD.md#d0131) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0132](BUILD.md#d0132)–D0133 | 2 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0134](BUILD.md#d0134)–D0142 | 9 | 主/因 | [E2017](RULES.md#e2017) | native 输入或选项无效 |
| [D0143](BUILD.md#d0143)–D0144 | 2 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0145](BUILD.md#d0145)–D0149 | 5 | 主/因 | [E2017](RULES.md#e2017) | native 输入或选项无效 |
| [D0150](BUILD.md#d0150) | 1 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0151](BUILD.md#d0151)–D0162 | 12 | 主/因 | [E2014](RULES.md#e2014) | 依赖来源声明不合法 |
| [D0163](BUILD.md#d0163) | 1 | 主/因 | [E2010](RULES.md#e2010) | 坐标或依赖 key 格式无效 |
| [D0164](BUILD.md#d0164) | 1 | 主/因 | [E2008](RULES.md#e2008) | 清单结构或字段值无效 |
| [D0165](BUILD.md#d0165)–D0167 | 3 | 主/因 | [E2009](RULES.md#e2009) | 清单配置不适用于此 Cone/workspace |
| [D0168](BUILD.md#d0168) | 1 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0169](BUILD.md#d0169)–D0170 | 2 | 主/因 | [E2008](RULES.md#e2008) | 清单结构或字段值无效 |
| [D0171](BUILD.md#d0171) | 1 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0172](BUILD.md#d0172)–D0173 | 2 | 主/因 | [E2016](RULES.md#e2016) | registry/archive/Git 地址无效 |
| [D0174](BUILD.md#d0174)–D0176 | 3 | 主/因 | [E2015](RULES.md#e2015) | 版本需求语法无效 |
| [D0177](BUILD.md#d0177)–D0181 | 5 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0182](BUILD.md#d0182) | 1 | 主/因 | [E2003](RULES.md#e2003) | 源码或清单不是 UTF-8 |
| [D0183](BUILD.md#d0183) | 1 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0184](BUILD.md#d0184)–D0187 | 4 | 主/因 | [E2013](RULES.md#e2013) | target predicate 无效 |
| [D0188](BUILD.md#d0188)–D0190 | 3 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0191](BUILD.md#d0191)–D0192 | 2 | 主/因 | [E2010](RULES.md#e2010) | 坐标或依赖 key 格式无效 |
| [D0193](BUILD.md#d0193)–D0194 | 2 | 主/因 | [E2008](RULES.md#e2008) | 清单结构或字段值无效 |
| [D0195](BUILD.md#d0195)–D0196 | 2 | 主/因 | [E2011](RULES.md#e2011) | 保留坐标或 core kind 不合法 |
| [D0197](BUILD.md#d0197) | 1 | 主/因 | [E2008](RULES.md#e2008) | 清单结构或字段值无效 |
| [D0198](BUILD.md#d0198) | 1 | 主/因 | [E2010](RULES.md#e2010) | 坐标或依赖 key 格式无效 |
| [D0199](BUILD.md#d0199)–D0200 | 2 | 主/因 | [E2014](RULES.md#e2014) | 依赖来源声明不合法 |
| [D0201](BUILD.md#d0201) | 1 | 主/因 | [E2012](RULES.md#e2012) | 清单路径格式无效 |
| [D0202](BUILD.md#d0202) | 1 | 主/因 | [E2014](RULES.md#e2014) | 依赖来源声明不合法 |
| [D0203](BUILD.md#d0203) | 1 | 主/因 | [E2009](RULES.md#e2009) | 清单配置不适用于此 Cone/workspace |
| [D0204](BUILD.md#d0204) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0205](BUILD.md#d0205)–D0207 | 3 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0208](BUILD.md#d0208) | 1 | 主/因 | [E2014](RULES.md#e2014) | 依赖来源声明不合法 |
| [D0209](BUILD.md#d0209)–D0210 | 2 | 主/因 | [E2010](RULES.md#e2010) | 坐标或依赖 key 格式无效 |
| [D0211](BUILD.md#d0211)–D0212 | 2 | 主/因 | [E2009](RULES.md#e2009) | 清单配置不适用于此 Cone/workspace |
| [D0213](BUILD.md#d0213) | 1 | 状态 | 取消/中断状态 | 取消/中断状态 |
| [D0214](BUILD.md#d0214)–D0220 | 7 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0221](BUILD.md#d0221)–D0222 | 2 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0223](BUILD.md#d0223) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0224](BUILD.md#d0224) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0225](BUILD.md#d0225) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0226](BUILD.md#d0226) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0227](BUILD.md#d0227)–D0229 | 3 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0230](BUILD.md#d0230) | 1 | 主/因 | [E2501](RULES.md#e2501) | 协议版本或配套工具不相容 |
| [D0231](BUILD.md#d0231)–D0250 | 20 | 按边界 | [E2502](RULES.md#e2502) / [ICE2501](RULES.md#ice2501) | 工具协议消息的收发边界 |
| [D0251](BUILD.md#d0251)–D0252 | 2 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0253](BUILD.md#d0253)–D0264 | 12 | 按边界 | [E2502](RULES.md#e2502) / [ICE2501](RULES.md#ice2501) | 工具协议消息的收发边界 |
| [D0265](BUILD.md#d0265) | 1 | 主/因 | [E2501](RULES.md#e2501) | 协议版本或配套工具不相容 |
| [D0266](BUILD.md#d0266)–D0267 | 2 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0268](BUILD.md#d0268) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0269](BUILD.md#d0269)–D0272 | 4 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0273](BUILD.md#d0273) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0274](BUILD.md#d0274)–D0277 | 4 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0278](BUILD.md#d0278) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0279](BUILD.md#d0279) | 1 | 主/因 | [E2108](RULES.md#e2108) | 预编译依赖与当前语义版本不相容 |
| [D0280](BUILD.md#d0280) | 1 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [D0281](BUILD.md#d0281)–D0282 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0283](BUILD.md#d0283)–D0284 | 2 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [D0285](BUILD.md#d0285)–D0286 | 2 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [D0287](BUILD.md#d0287)–D0288 | 2 | 主/因 | [E2404](RULES.md#e2404) | 产物 target/profile/ABI 不匹配 |
| [D0289](BUILD.md#d0289) | 1 | 主/因 | [E2108](RULES.md#e2108) | 预编译依赖与当前语义版本不相容 |
| [D0290](BUILD.md#d0290)–D0292 | 3 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0293](BUILD.md#d0293) | 1 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [D0294](BUILD.md#d0294) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0295](BUILD.md#d0295) | 1 | 主/因 | [E2108](RULES.md#e2108) | 预编译依赖与当前语义版本不相容 |
| [D0296](BUILD.md#d0296) | 1 | 主/因 | [E2104](RULES.md#e2104) | 依赖闭包包含冲突版本 |
| [D0297](BUILD.md#d0297)–D0299 | 3 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0300](BUILD.md#d0300) | 1 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [D0301](BUILD.md#d0301) | 1 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [D0302](BUILD.md#d0302) | 1 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [D0303](BUILD.md#d0303) | 1 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [D0304](BUILD.md#d0304)–D0305 | 2 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0306](BUILD.md#d0306)–D0308 | 3 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0309](BUILD.md#d0309)–D0311 | 3 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0312](BUILD.md#d0312) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0313](BUILD.md#d0313) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0314](BUILD.md#d0314) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0315](BUILD.md#d0315) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0316](BUILD.md#d0316) | 1 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [D0317](BUILD.md#d0317) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0318](BUILD.md#d0318) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0319](BUILD.md#d0319) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0320](BUILD.md#d0320)–D0323 | 4 | 主/因 | [E2204](RULES.md#e2204) | 子进程通信或等待失败 |
| [D0324](BUILD.md#d0324) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0325](BUILD.md#d0325)–D0326 | 2 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0327](BUILD.md#d0327) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0328](BUILD.md#d0328) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0329](BUILD.md#d0329) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0330](BUILD.md#d0330)–D0332 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0333](BUILD.md#d0333)–D0335 | 3 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0336](BUILD.md#d0336)–D0337 | 2 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0338](BUILD.md#d0338)–D0339 | 2 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0340](BUILD.md#d0340) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0341](BUILD.md#d0341) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0342](BUILD.md#d0342)–D0343 | 2 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0344](BUILD.md#d0344) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0345](BUILD.md#d0345)–D0347 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0348](BUILD.md#d0348) | 1 | 主/因 | [E2007](RULES.md#e2007) | 源码 package 不属于当前 Cone group |
| [D0349](BUILD.md#d0349) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0350](BUILD.md#d0350) | 1 | 排除 | 应用测试失败的打印片段 | 应用测试失败的打印片段 |
| [D0351](BUILD.md#d0351) | 1 | 主/因 | [E2508](RULES.md#e2508) | 测试进程报告用例失败 |
| [D0352](BUILD.md#d0352) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0353](BUILD.md#d0353)–D0354 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0355](BUILD.md#d0355)–D0358 | 4 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [D0359](BUILD.md#d0359) | 1 | 主/因 | [E2011](RULES.md#e2011) | 保留坐标或 core kind 不合法 |
| [D0360](BUILD.md#d0360) | 1 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [D0361](BUILD.md#d0361)–D0362 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0363](BUILD.md#d0363) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0364](BUILD.md#d0364)–D0365 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0366](BUILD.md#d0366) | 1 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [D0367](BUILD.md#d0367) | 1 | 策略 | warning 被当前策略视为失败 | warning 被当前策略视为失败 |
| [D0368](BUILD.md#d0368) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0369](BUILD.md#d0369)–D0371 | 3 | 主/因 | [E1801](RULES.md#e1801) | 文档资源或站点配置无效 |
| [D0372](BUILD.md#d0372) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0373](BUILD.md#d0373) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0374](BUILD.md#d0374) | 1 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0375](BUILD.md#d0375) | 1 | 主/因 | [E1801](RULES.md#e1801) | 文档资源或站点配置无效 |
| [D0376](BUILD.md#d0376) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0377](BUILD.md#d0377) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0378](BUILD.md#d0378) | 1 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [D0379](BUILD.md#d0379)–D0381 | 3 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0382](BUILD.md#d0382) | 1 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [D0383](BUILD.md#d0383) | 1 | 主/因 | [E2104](RULES.md#e2104) | 依赖闭包包含冲突版本 |
| [D0384](BUILD.md#d0384)–D0390 | 7 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0391](BUILD.md#d0391) | 1 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [D0392](BUILD.md#d0392) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0393](BUILD.md#d0393)–D0396 | 4 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0397](BUILD.md#d0397) | 1 | 主/因 | [E2107](RULES.md#e2107) | 依赖集合与声明/构建图不一致 |
| [D0398](BUILD.md#d0398) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0399](BUILD.md#d0399) | 1 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [D0400](BUILD.md#d0400)–D0401 | 2 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [D0402](BUILD.md#d0402) | 1 | 主/因 | [E2011](RULES.md#e2011) | 保留坐标或 core kind 不合法 |
| [D0403](BUILD.md#d0403) | 1 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [D0404](BUILD.md#d0404)–D0405 | 2 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [D0406](BUILD.md#d0406) | 1 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [D0407](BUILD.md#d0407)–D0408 | 2 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [D0409](BUILD.md#d0409) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0410](BUILD.md#d0410) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0411](BUILD.md#d0411)–D0413 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0414](BUILD.md#d0414) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0415](BUILD.md#d0415) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0416](BUILD.md#d0416) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0417](BUILD.md#d0417)–D0418 | 2 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0419](BUILD.md#d0419)–D0420 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0421](BUILD.md#d0421) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0422](BUILD.md#d0422)–D0426 | 5 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0427](BUILD.md#d0427)–D0446 | 20 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0447](BUILD.md#d0447) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0448](BUILD.md#d0448) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0449](BUILD.md#d0449) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [D0450](BUILD.md#d0450) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0451](BUILD.md#d0451) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0452](BUILD.md#d0452)–D0453 | 2 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0454](BUILD.md#d0454) | 1 | 主/因 | [E2204](RULES.md#e2204) | 子进程通信或等待失败 |
| [D0455](BUILD.md#d0455) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0456](BUILD.md#d0456)–D0459 | 4 | 主/因 | [E2503](RULES.md#e2503) | 查询请求或编辑范围无效 |
| [D0460](BUILD.md#d0460) | 1 | 主/因 | [E2205](RULES.md#e2205) | 主机资源或表示容量不足 |
| [D0461](BUILD.md#d0461) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0462](BUILD.md#d0462) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0463](BUILD.md#d0463)–D0470 | 8 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0471](BUILD.md#d0471)–D0472 | 2 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0473](BUILD.md#d0473)–D0475 | 3 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0476](BUILD.md#d0476) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0477](BUILD.md#d0477) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0478](BUILD.md#d0478) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0479](BUILD.md#d0479)–D0494 | 16 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0495](BUILD.md#d0495) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0496](BUILD.md#d0496) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0497](BUILD.md#d0497)–D0498 | 2 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0499](BUILD.md#d0499) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0500](BUILD.md#d0500) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0501](BUILD.md#d0501)–D0513 | 13 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0514](BUILD.md#d0514) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [D0515](BUILD.md#d0515) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0516](BUILD.md#d0516) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0517](BUILD.md#d0517) | 1 | 主/因 | [E1606](RULES.md#e1606) | 编译期环境变量不是 Unicode |
| [D0518](BUILD.md#d0518)–D0526 | 9 | 主/因 | [E1605](RULES.md#e1605) | 编译期输入路径不合法 |
| [D0527](BUILD.md#d0527) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0528](BUILD.md#d0528)–D0529 | 2 | 主/因 | [E1603](RULES.md#e1603) | includeDir pattern 无效 |
| [D0530](BUILD.md#d0530) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0531](BUILD.md#d0531) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0532](BUILD.md#d0532)–D0533 | 2 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0534](BUILD.md#d0534)–D0535 | 2 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0536](BUILD.md#d0536) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0537](BUILD.md#d0537)–D0538 | 2 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0539](BUILD.md#d0539) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0540](BUILD.md#d0540) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0541](BUILD.md#d0541) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0542](BUILD.md#d0542)–D0544 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0545](BUILD.md#d0545) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0546](BUILD.md#d0546)–D0547 | 2 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0548](BUILD.md#d0548)–D0549 | 2 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0550](BUILD.md#d0550) | 1 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0551](BUILD.md#d0551) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0552](BUILD.md#d0552) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0553](BUILD.md#d0553) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0554](BUILD.md#d0554)–D0556 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0557](BUILD.md#d0557)–D0558 | 2 | 内部 | [ICE2201](RULES.md#ice2201) | 驱动、调度或缓存操作状态不一致 |
| [D0559](BUILD.md#d0559) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0560](BUILD.md#d0560) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0561](BUILD.md#d0561)–D0562 | 2 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0563](BUILD.md#d0563)–D0568 | 6 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0569](BUILD.md#d0569) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0570](BUILD.md#d0570)–D0574 | 5 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [D0575](BUILD.md#d0575) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0576](BUILD.md#d0576) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0577](BUILD.md#d0577)–D0578 | 2 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0579](BUILD.md#d0579) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0580](BUILD.md#d0580) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0581](BUILD.md#d0581)–D0584 | 4 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0585](BUILD.md#d0585)–D0586 | 2 | 主/因 | [E2017](RULES.md#e2017) | native 输入或选项无效 |
| [D0587](BUILD.md#d0587) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0588](BUILD.md#d0588) | 1 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [D0589](BUILD.md#d0589)–D0590 | 2 | 主/因 | [E2017](RULES.md#e2017) | native 输入或选项无效 |
| [D0591](BUILD.md#d0591)–D0592 | 2 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0593](BUILD.md#d0593) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0594](BUILD.md#d0594) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0595](BUILD.md#d0595)–D0598 | 4 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0599](BUILD.md#d0599)–D0600 | 2 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0601](BUILD.md#d0601) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0602](BUILD.md#d0602)–D0606 | 5 | 主/因 | [E2307](RULES.md#e2307) | runtime 配置或 ABI 不相容 |
| [D0607](BUILD.md#d0607)–D0611 | 5 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0612](BUILD.md#d0612) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0613](BUILD.md#d0613)–D0622 | 10 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0623](BUILD.md#d0623) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [D0624](BUILD.md#d0624)–D0625 | 2 | 主/因 | [E2307](RULES.md#e2307) | runtime 配置或 ABI 不相容 |
| [D0626](BUILD.md#d0626) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0627](BUILD.md#d0627) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0628](BUILD.md#d0628) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0629](BUILD.md#d0629) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0630](BUILD.md#d0630)–D0632 | 3 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0633](BUILD.md#d0633) | 1 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [D0634](BUILD.md#d0634)–D0639 | 6 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0640](BUILD.md#d0640) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0641](BUILD.md#d0641)–D0642 | 2 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [D0643](BUILD.md#d0643) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [D0644](BUILD.md#d0644) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [D0645](BUILD.md#d0645) | 1 | 透传 | 仅转发/汇总/包装 | 仅转发/汇总/包装 |
| [D0646](BUILD.md#d0646) | 1 | 主/因 | [E2206](RULES.md#e2206) | 准备期间输入发生变化 |
| [D0647](BUILD.md#d0647)–D0648 | 2 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |

## T · TOOLS.md

| 原条目 | 条数 | 角色 | 候选编号/处置 | 归属 |
| --- | --- | --- | --- | --- |
| [T0001](TOOLS.md#t0001) | 1 | 主/因 | [E2504](RULES.md#e2504) | 文档/分析版本或会话状态不符 |
| [T0002](TOOLS.md#t0002) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0003](TOOLS.md#t0003) | 1 | 主/因 | [E2504](RULES.md#e2504) | 文档/分析版本或会话状态不符 |
| [T0004](TOOLS.md#t0004)–T0005 | 2 | 主/因 | [E2503](RULES.md#e2503) | 查询请求或编辑范围无效 |
| [T0006](TOOLS.md#t0006) | 1 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [T0007](TOOLS.md#t0007) | 1 | 主/因 | [E2406](RULES.md#e2406) | 产物声明接口合同无效 |
| [T0008](TOOLS.md#t0008) | 1 | 状态 | 取消/中断状态 | 取消/中断状态 |
| [T0009](TOOLS.md#t0009) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0010](TOOLS.md#t0010) | 1 | 主/因 | [E2506](RULES.md#e2506) | 重命名目标名称不合法 |
| [T0011](TOOLS.md#t0011) | 1 | 主/因 | [E2507](RULES.md#e2507) | 重命名会改变绑定或不可完整编辑 |
| [T0012](TOOLS.md#t0012)–T0013 | 2 | 内部 | [ICE2502](RULES.md#ice2502) | 查询实现内部关系缺失 |
| [T0014](TOOLS.md#t0014)–T0017 | 4 | 主/因 | [E2507](RULES.md#e2507) | 重命名会改变绑定或不可完整编辑 |
| [T0018](TOOLS.md#t0018)–T0024 | 7 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0025](TOOLS.md#t0025) | 1 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [T0026](TOOLS.md#t0026) | 1 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [T0027](TOOLS.md#t0027) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0028](TOOLS.md#t0028) | 1 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [T0029](TOOLS.md#t0029) | 1 | 主/因 | [E2105](RULES.md#e2105) | 依赖图存在环 |
| [T0030](TOOLS.md#t0030) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0031](TOOLS.md#t0031) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [T0032](TOOLS.md#t0032) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [T0033](TOOLS.md#t0033) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0034](TOOLS.md#t0034) | 1 | 主/因 | [E2302](RULES.md#e2302) | 找不到所需工具或 sysroot 组件 |
| [T0035](TOOLS.md#t0035)–T0036 | 2 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [T0037](TOOLS.md#t0037)–T0038 | 2 | 主/因 | [E2301](RULES.md#e2301) | 不支持或不认识目标配置 |
| [T0039](TOOLS.md#t0039) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [T0040](TOOLS.md#t0040)–T0041 | 2 | 主/因 | [E1801](RULES.md#e1801) | 文档资源或站点配置无效 |
| [T0042](TOOLS.md#t0042) | 1 | 透传/建议 | API 数据失败的恢复建议 | API 数据失败的恢复建议 |
| [T0043](TOOLS.md#t0043)–T0046 | 4 | 主/因 | [E1801](RULES.md#e1801) | 文档资源或站点配置无效 |
| [T0047](TOOLS.md#t0047)–T0048 | 2 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0049](TOOLS.md#t0049) | 1 | 主/因 | [E2204](RULES.md#e2204) | 子进程通信或等待失败 |
| [T0050](TOOLS.md#t0050) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [T0051](TOOLS.md#t0051) | 1 | 内部 | [ICE2502](RULES.md#ice2502) | 查询实现内部关系缺失 |
| [T0052](TOOLS.md#t0052) | 1 | 主/因 | [E2501](RULES.md#e2501) | 协议版本或配套工具不相容 |
| [T0053](TOOLS.md#t0053) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [T0054](TOOLS.md#t0054)–T0059 | 6 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [T0060](TOOLS.md#t0060)–T0062 | 3 | 主/因 | [E2504](RULES.md#e2504) | 文档/分析版本或会话状态不符 |
| [T0063](TOOLS.md#t0063) | 1 | 主/因 | [E2503](RULES.md#e2503) | 查询请求或编辑范围无效 |
| [T0064](TOOLS.md#t0064) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0065](TOOLS.md#t0065) | 1 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [T0066](TOOLS.md#t0066)–T0067 | 2 | 主/因 | [E2503](RULES.md#e2503) | 查询请求或编辑范围无效 |
| [T0068](TOOLS.md#t0068) | 1 | 状态 | 取消/中断状态 | 取消/中断状态 |
| [T0069](TOOLS.md#t0069) | 1 | 主/因 | [E2503](RULES.md#e2503) | 查询请求或编辑范围无效 |
| [T0070](TOOLS.md#t0070) | 1 | 主/因 | [E2504](RULES.md#e2504) | 文档/分析版本或会话状态不符 |
| [T0071](TOOLS.md#t0071) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0072](TOOLS.md#t0072) | 1 | 主/因 | [E2504](RULES.md#e2504) | 文档/分析版本或会话状态不符 |
| [T0073](TOOLS.md#t0073) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0074](TOOLS.md#t0074) | 1 | 主/因 | [E2501](RULES.md#e2501) | 协议版本或配套工具不相容 |
| [T0075](TOOLS.md#t0075)–T0079 | 5 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0080](TOOLS.md#t0080) | 1 | 主/因 | [E2501](RULES.md#e2501) | 协议版本或配套工具不相容 |
| [T0081](TOOLS.md#t0081)–T0085 | 5 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0086](TOOLS.md#t0086)–T0089 | 4 | 主/因 | [E2504](RULES.md#e2504) | 文档/分析版本或会话状态不符 |
| [T0090](TOOLS.md#t0090) | 1 | 内部 | [ICE2502](RULES.md#ice2502) | 查询实现内部关系缺失 |
| [T0091](TOOLS.md#t0091) | 1 | 主/因 | [E2505](RULES.md#e2505) | 查询所需语义/源码输入不可用 |
| [T0092](TOOLS.md#t0092) | 1 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [T0093](TOOLS.md#t0093) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [T0094](TOOLS.md#t0094) | 1 | 主/因 | [E2206](RULES.md#e2206) | 准备期间输入发生变化 |
| [T0095](TOOLS.md#t0095) | 1 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0096](TOOLS.md#t0096) | 1 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0097](TOOLS.md#t0097) | 1 | 主/因 | [E2108](RULES.md#e2108) | 预编译依赖与当前语义版本不相容 |
| [T0098](TOOLS.md#t0098)–T0099 | 2 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [T0100](TOOLS.md#t0100) | 1 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [T0101](TOOLS.md#t0101) | 1 | 主/因 | [E2106](RULES.md#e2106) | 依赖有冲突的候选或来源 |
| [T0102](TOOLS.md#t0102) | 1 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [T0103](TOOLS.md#t0103) | 1 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0104](TOOLS.md#t0104) | 1 | 主/因 | [E2115](RULES.md#e2115) | Git selector 无法确定提交 |
| [T0105](TOOLS.md#t0105) | 1 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0106](TOOLS.md#t0106) | 1 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0107](TOOLS.md#t0107) | 1 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0108](TOOLS.md#t0108) | 1 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0109](TOOLS.md#t0109)–T0110 | 2 | 主/因 | [E2014](RULES.md#e2014) | 依赖来源声明不合法 |
| [T0111](TOOLS.md#t0111) | 1 | 主/因 | [E2116](RULES.md#e2116) | 没有可用 registry 配置 |
| [T0112](TOOLS.md#t0112) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0113](TOOLS.md#t0113) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [T0114](TOOLS.md#t0114) | 1 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [T0115](TOOLS.md#t0115)–T0116 | 2 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0117](TOOLS.md#t0117) | 1 | 主/因 | [E2114](RULES.md#e2114) | 选中的包不在当前图中 |
| [T0118](TOOLS.md#t0118)–T0120 | 3 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [T0121](TOOLS.md#t0121) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0122](TOOLS.md#t0122) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [T0123](TOOLS.md#t0123) | 1 | 主/因 | [E2008](RULES.md#e2008) | 清单结构或字段值无效 |
| [T0124](TOOLS.md#t0124) | 1 | 主/因 | [E2014](RULES.md#e2014) | 依赖来源声明不合法 |
| [T0125](TOOLS.md#t0125) | 1 | 排版 | 位置/候选链的排版片段 | 位置/候选链的排版片段 |
| [T0126](TOOLS.md#t0126) | 1 | 主/因 | [E2113](RULES.md#e2113) | Cone.lock 内容无效 |
| [T0127](TOOLS.md#t0127)–T0128 | 2 | 主/因 | [E2020](RULES.md#e2020) | 输出路径不合法或与输入冲突 |
| [T0129](TOOLS.md#t0129)–T0148 | 20 | 主/因 | [E2113](RULES.md#e2113) | Cone.lock 内容无效 |
| [T0149](TOOLS.md#t0149) | 1 | 内部 | [ICE2501](RULES.md#ice2501) | 自产协议消息或诊断位置无效 |
| [T0150](TOOLS.md#t0150) | 1 | 主/因 | [E2206](RULES.md#e2206) | 准备期间输入发生变化 |
| [T0151](TOOLS.md#t0151) | 1 | 主/因 | [E2001](RULES.md#e2001) | 命令参数或操作组合不合法 |
| [T0152](TOOLS.md#t0152) | 1 | 主/因 | [E2018](RULES.md#e2018) | workspace 成员选择无效 |
| [T0153](TOOLS.md#t0153)–T0154 | 2 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0155](TOOLS.md#t0155) | 1 | 附注 | 离线求解范围说明 | 离线求解范围说明 |
| [T0156](TOOLS.md#t0156) | 1 | 主/因 | [E2111](RULES.md#e2111) | 锁定结果不能满足当前请求 |
| [T0157](TOOLS.md#t0157) | 1 | 主/因 | [E2114](RULES.md#e2114) | 选中的包不在当前图中 |
| [T0158](TOOLS.md#t0158) | 1 | 主/因 | [E2206](RULES.md#e2206) | 准备期间输入发生变化 |
| [T0159](TOOLS.md#t0159) | 1 | 主/因 | [E2102](RULES.md#e2102) | 依赖实际坐标与要求不符 |
| [T0160](TOOLS.md#t0160) | 1 | 主/因 | [E2110](RULES.md#e2110) | 没有满足要求的依赖解 |
| [T0161](TOOLS.md#t0161)–T0162 | 2 | 排版 | 位置/候选链的排版片段 | 位置/候选链的排版片段 |
| [T0163](TOOLS.md#t0163) | 1 | 主/因 | [E2103](RULES.md#e2103) | 依赖不是可消费的库 |
| [T0164](TOOLS.md#t0164)–T0166 | 3 | 因 | [E2110](RULES.md#e2110) | 没有满足要求的依赖解 |
| [T0167](TOOLS.md#t0167) | 1 | 排版 | 位置/候选链的排版片段 | 位置/候选链的排版片段 |
| [T0168](TOOLS.md#t0168)–T0169 | 2 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0170](TOOLS.md#t0170) | 1 | 主/因 | [E2113](RULES.md#e2113) | Cone.lock 内容无效 |
| [T0171](TOOLS.md#t0171) | 1 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0172](TOOLS.md#t0172)–T0173 | 2 | 主/因 | [E2016](RULES.md#e2016) | registry/archive/Git 地址无效 |
| [T0174](TOOLS.md#t0174) | 1 | 主/因 | [E2113](RULES.md#e2113) | Cone.lock 内容无效 |
| [T0175](TOOLS.md#t0175) | 1 | 主/因 | [E2202](RULES.md#e2202) | 外部进程/工作线程不能启动 |
| [T0176](TOOLS.md#t0176) | 1 | 主/因 | [E2203](RULES.md#e2203) | 外部进程执行失败 |
| [T0177](TOOLS.md#t0177) | 1 | 主/因 | [E2303](RULES.md#e2303) | 工具链不符合选定目标合同 |
| [T0178](TOOLS.md#t0178) | 1 | 主/因 | [E2201](RULES.md#e2201) | 文件或目录 I/O 失败 |
| [T0179](TOOLS.md#t0179) | 1 | 主/因 | [E2115](RULES.md#e2115) | Git selector 无法确定提交 |
| [T0180](TOOLS.md#t0180) | 1 | 主/因 | [E2016](RULES.md#e2016) | registry/archive/Git 地址无效 |
| [T0181](TOOLS.md#t0181)–T0183 | 3 | 主/因 | [E2119](RULES.md#e2119) | 包归档结构无效 |
| [T0184](TOOLS.md#t0184) | 1 | 主/因 | [E2117](RULES.md#e2117) | 包下载失败 |
| [T0185](TOOLS.md#t0185) | 1 | 主/因 | [E2118](RULES.md#e2118) | 下载摘要与包元数据不符 |
| [T0186](TOOLS.md#t0186)–T0187 | 2 | 主/因 | [E2101](RULES.md#e2101) | 找不到依赖产物/包输入 |
| [T0188](TOOLS.md#t0188) | 1 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0189](TOOLS.md#t0189)–T0190 | 2 | 主/因 | [E2120](RULES.md#e2120) | registry 元数据格式或一致性错误 |
| [T0191](TOOLS.md#t0191) | 1 | 主/因 | [E2112](RULES.md#e2112) | 所需包数据未缓存 |
| [T0192](TOOLS.md#t0192)–T0196 | 5 | 主/因 | [E2120](RULES.md#e2120) | registry 元数据格式或一致性错误 |
| [T0197](TOOLS.md#t0197) | 1 | 主/因 | [E2011](RULES.md#e2011) | 保留坐标或 core kind 不合法 |
| [T0198](TOOLS.md#t0198)–T0199 | 2 | 主/因 | [E2120](RULES.md#e2120) | registry 元数据格式或一致性错误 |
| [T0200](TOOLS.md#t0200)–T0203 | 4 | 主/因 | [E2121](RULES.md#e2121) | 发布包的 manifest 未展开或不自足 |
| [T0204](TOOLS.md#t0204) | 1 | 主/因 | [E2011](RULES.md#e2011) | 保留坐标或 core kind 不合法 |
| [T0205](TOOLS.md#t0205)–T0209 | 5 | 主/因 | [E2121](RULES.md#e2121) | 发布包的 manifest 未展开或不自足 |
| [T0210](TOOLS.md#t0210)–T0215 | 6 | 主/因 | [E2018](RULES.md#e2018) | workspace 成员选择无效 |
| [T0216](TOOLS.md#t0216)–T0218 | 3 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |
| [T0219](TOOLS.md#t0219)–T0220 | 2 | 主/因 | [E2011](RULES.md#e2011) | 保留坐标或 core kind 不合法 |
| [T0221](TOOLS.md#t0221)–T0223 | 3 | 主/因 | [E2019](RULES.md#e2019) | workspace 继承无法解析 |
| [T0224](TOOLS.md#t0224) | 1 | 主/因 | [E2502](RULES.md#e2502) | 外部工具协议内容无效 |
| [T0225](TOOLS.md#t0225)–T0226 | 2 | 主/因 | [E2002](RULES.md#e2002) | 输入路径或文件种类不符合入口要求 |

## 原警告、note 和内嵌片段

W/N/P/R 分别为初稿的 warning、结构化 note、内嵌片段、文档警告原因表。此处前缀只用于资料索引，不是拟议的编号。一个原模板可按 reason 分流，或拆出说明句与 help 句。

| 原项 | 候选编号 | 部分/条件 |
| --- | --- | --- |
| [N01](INVENTORY.md#n01) | [N0001](RULES.md#n0001) | 完整 |
| [N02](INVENTORY.md#n02) | [N0001](RULES.md#n0001) | 完整 |
| [N03](INVENTORY.md#n03) | [N0001](RULES.md#n0001) | 完整 |
| [N04](INVENTORY.md#n04) | [N0004](RULES.md#n0004) | 完整 |
| [N05](INVENTORY.md#n05) | [N0003](RULES.md#n0003) | 完整 |
| [N06](INVENTORY.md#n06) | [N0002](RULES.md#n0002) | 完整 |
| [N07](INVENTORY.md#n07) | [N0004](RULES.md#n0004) | 完整 |
| [N08](INVENTORY.md#n08) | [N0004](RULES.md#n0004) | 完整 |
| [N09](INVENTORY.md#n09) | [N0401](RULES.md#n0401) | 完整 |
| [N10](INVENTORY.md#n10) | [H0401](RULES.md#h0401) | 完整 |
| [N11](INVENTORY.md#n11) | [N0002](RULES.md#n0002) | 完整 |
| [N12](INVENTORY.md#n12) | [N1701](RULES.md#n1701) | 完整 |
| [N13](INVENTORY.md#n13) | [N1701](RULES.md#n1701) | 完整 |
| [N14](INVENTORY.md#n14) | [N0001](RULES.md#n0001) | 完整 |
| [N15](INVENTORY.md#n15) | [N0001](RULES.md#n0001) | 完整 |
| [N16](INVENTORY.md#n16) | [N0001](RULES.md#n0001) | 完整 |
| [N17](INVENTORY.md#n17) | [N0001](RULES.md#n0001) | 完整 |
| [N18](INVENTORY.md#n18) | [N0001](RULES.md#n0001) | 完整 |
| [N19](INVENTORY.md#n19) | [N0001](RULES.md#n0001) | 完整 |
| [P01](INVENTORY.md#p01) | [H0701](RULES.md#h0701) | help 句 |
| [P01](INVENTORY.md#p01) | [N0705](RULES.md#n0705) | 说明句 |
| [P02](INVENTORY.md#p02) | [H0701](RULES.md#h0701) | help 句 |
| [P02](INVENTORY.md#p02) | [N0705](RULES.md#n0705) | 说明句 |
| [P03](INVENTORY.md#p03) | [H0701](RULES.md#h0701) | help 句 |
| [P03](INVENTORY.md#p03) | [N0705](RULES.md#n0705) | 说明句 |
| [P04](INVENTORY.md#p04) | [N0701](RULES.md#n0701) | 完整 |
| [P05](INVENTORY.md#p05) | [N0701](RULES.md#n0701) | 完整 |
| [P06](INVENTORY.md#p06) | [N1701](RULES.md#n1701) | 完整 |
| [P07](INVENTORY.md#p07) | [N1701](RULES.md#n1701) | 完整 |
| [P08](INVENTORY.md#p08) | [N1701](RULES.md#n1701) | 完整 |
| [P09](INVENTORY.md#p09) | [N1701](RULES.md#n1701) | 完整 |
| [P10](INVENTORY.md#p10) | [N1701](RULES.md#n1701) | 完整 |
| [P11](INVENTORY.md#p11) | [N1701](RULES.md#n1701) | 完整 |
| [P12](INVENTORY.md#p12) | [N1701](RULES.md#n1701) | 完整 |
| [P13](INVENTORY.md#p13) | [N1701](RULES.md#n1701) | 完整 |
| [P14](INVENTORY.md#p14) | [N1701](RULES.md#n1701) | 完整 |
| [P15](INVENTORY.md#p15) | [N1701](RULES.md#n1701) | 完整 |
| [P16](INVENTORY.md#p16) | [N0702](RULES.md#n0702) | 完整 |
| [P17](INVENTORY.md#p17) | [N0703](RULES.md#n0703) | 完整 |
| [P18](INVENTORY.md#p18) | [N0702](RULES.md#n0702) | 完整 |
| [P19](INVENTORY.md#p19) | [N0702](RULES.md#n0702) | 完整 |
| [P20](INVENTORY.md#p20) | [N0704](RULES.md#n0704) | 完整 |
| [P21](INVENTORY.md#p21) | [H0501](RULES.md#h0501) | 完整 |
| [P22](INVENTORY.md#p22) | [N0901](RULES.md#n0901) | 完整 |
| [R01](INVENTORY.md#r01) | [W1809](RULES.md#w1809) | 完整；reason = names a package, not a declaration |
| [R02](INVENTORY.md#r02) | [W1810](RULES.md#w1810) | 完整；reason = cannot be resolved |
| [R03](INVENTORY.md#r03) | [W1811](RULES.md#w1811) | 完整；reason = has an ambiguous owner / is ambiguous |
| [R04](INVENTORY.md#r04) | [W1811](RULES.md#w1811) | 完整；reason = has an ambiguous owner / is ambiguous |
| [W01](INVENTORY.md#w01) | [W1801](RULES.md#w1801) | 完整 |
| [W02](INVENTORY.md#w02) | [W1802](RULES.md#w1802) | 完整 |
| [W03](INVENTORY.md#w03) | [W1803](RULES.md#w1803) | 完整 |
| [W04](INVENTORY.md#w04) | [W1804](RULES.md#w1804) | 完整 |
| [W05](INVENTORY.md#w05) | [W1805](RULES.md#w1805) | 完整 |
| [W06](INVENTORY.md#w06) | [W1806](RULES.md#w1806) | 完整 |
| [W07](INVENTORY.md#w07) | [W1807](RULES.md#w1807) | 完整 |
| [W08](INVENTORY.md#w08) | [W1804](RULES.md#w1804) | 完整 |
| [W09](INVENTORY.md#w09) | [W2001](RULES.md#w2001) | 完整 |
| [W10](INVENTORY.md#w10) | [W1808](RULES.md#w1808) | 完整 |
| [W11](INVENTORY.md#w11) | [W1811](RULES.md#w1811) | 完整；reason = has an ambiguous owner / is ambiguous |
| [W11](INVENTORY.md#w11) | [W1809](RULES.md#w1809) | 完整；reason = names a package, not a declaration |
| [W11](INVENTORY.md#w11) | [W1810](RULES.md#w1810) | 完整；reason = cannot be resolved |
| [W12](INVENTORY.md#w12) | [W1812](RULES.md#w1812) | 完整 |
| [W13](INVENTORY.md#w13) | [W1808](RULES.md#w1808) | 完整 |
| [W14](INVENTORY.md#w14) | [H0903](RULES.md#h0903) | rename/qualify 建议 |
| [W14](INVENTORY.md#w14) | [W0901](RULES.md#w0901) | 完整 |

## 接入补查

初稿未清点 README 本地资源缺失的 warning 构造点；实现与 fixture 核对后补记 [W1813](RULES.md#w1813)。其来源是 [document/overview.rs](../../compiler/scoop/src/document/overview.rs)，旧标识为 `SCOOP_DOC_RESOURCE`。原 5,960 行清单保留，补查不改写历史清点数量。

## 主文案中已有的修复建议

这些建议原本内嵌在 error/warning 正文中。本轮按下表拆为主诊断的 H 附属建议，不作为独立主诊断，也不自动修改用户代码。

| 建议编号 | 原错误条目 |
| --- | --- |
| [H0501](RULES.md#h0501) | [F1278](FRONTEND.md#f1278) |
| [H0502](RULES.md#h0502) | [F0352](FRONTEND.md#f0352)、[F0355](FRONTEND.md#f0355)、[F0562](FRONTEND.md#f0562)、[F0571](FRONTEND.md#f0571)、[F1013](FRONTEND.md#f1013) |
| [H0702](RULES.md#h0702) | [F0466](FRONTEND.md#f0466)、[F0485](FRONTEND.md#f0485) |
| [H0703](RULES.md#h0703) | [F0487](FRONTEND.md#f0487)、[F0787](FRONTEND.md#f0787) |
| [H0704](RULES.md#h0704) | [F0088](FRONTEND.md#f0088)、[F0555](FRONTEND.md#f0555)、[F0556](FRONTEND.md#f0556)、[F0572](FRONTEND.md#f0572)、[F0586](FRONTEND.md#f0586) |
| [H0705](RULES.md#h0705) | [F0087](FRONTEND.md#f0087)、[F1465](FRONTEND.md#f1465) |
| [H0402](RULES.md#h0402) | [F0601](FRONTEND.md#f0601)、[F0790](FRONTEND.md#f0790)、[F0801](FRONTEND.md#f0801) |
| [H0901](RULES.md#h0901) | [F1027](FRONTEND.md#f1027)、[F1028](FRONTEND.md#f1028)、[F1039](FRONTEND.md#f1039)、[F1044](FRONTEND.md#f1044)、[F1045](FRONTEND.md#f1045)、[F1046](FRONTEND.md#f1046)、[F1498](FRONTEND.md#f1498) |
| [H0902](RULES.md#h0902) | [F1509](FRONTEND.md#f1509)、[F1512](FRONTEND.md#f1512) |
| [H0801](RULES.md#h0801) | [F1201](FRONTEND.md#f1201) |
| [H1501](RULES.md#h1501) | [F0537](FRONTEND.md#f0537)、[F0602](FRONTEND.md#f0602)、[F1143](FRONTEND.md#f1143) |
| [H1502](RULES.md#h1502) | [F1427](FRONTEND.md#f1427)、[F1458](FRONTEND.md#f1458) |
| [H1301](RULES.md#h1301) | [F0168](FRONTEND.md#f0168)、[F0172](FRONTEND.md#f0172)、[F0173](FRONTEND.md#f0173)、[F0174](FRONTEND.md#f0174)、[F0175](FRONTEND.md#f0175)、[F0177](FRONTEND.md#f0177)、[F0183](FRONTEND.md#f0183)、[F0192](FRONTEND.md#f0192)、[F0193](FRONTEND.md#f0193)、[F0195](FRONTEND.md#f0195)、[F0196](FRONTEND.md#f0196)、[F0197](FRONTEND.md#f0197)、[F0198](FRONTEND.md#f0198)、[F0199](FRONTEND.md#f0199)、[F0206](FRONTEND.md#f0206)、[F0207](FRONTEND.md#f0207)、[F0208](FRONTEND.md#f0208)、[F0212](FRONTEND.md#f0212) |
| [H1001](RULES.md#h1001) | [F0190](FRONTEND.md#f0190)、[F0215](FRONTEND.md#f0215)、[F0217](FRONTEND.md#f0217) |
| [H1201](RULES.md#h1201) | [F0066](FRONTEND.md#f0066)、[F0450](FRONTEND.md#f0450) |
| [H1202](RULES.md#h1202) | [F0019](FRONTEND.md#f0019) |
| [H1002](RULES.md#h1002) | [F1398](FRONTEND.md#f1398) |
| [H0601](RULES.md#h0601) | [F1524](FRONTEND.md#f1524) |
| [H0101](RULES.md#h0101) | [F1484](FRONTEND.md#f1484) |
| [H2001](RULES.md#h2001) | [D0059](BUILD.md#d0059)、[D0068](BUILD.md#d0068)、[D0303](BUILD.md#d0303)、[D0332](BUILD.md#d0332)、[D0345](BUILD.md#d0345)、[D0600](BUILD.md#d0600)、[T0037](TOOLS.md#t0037)、[T0118](TOOLS.md#t0118) |
| [H2301](RULES.md#h2301) | [D0540](BUILD.md#d0540)、[D0547](BUILD.md#d0547)、[D0553](BUILD.md#d0553)、[D0595](BUILD.md#d0595)、[D0626](BUILD.md#d0626)、[D0628](BUILD.md#d0628)、[D0630](BUILD.md#d0630)、[D0631](BUILD.md#d0631)、[T0093](TOOLS.md#t0093) |
| [H2302](RULES.md#h2302) | [A2028](ARTIFACTS.md#a2028)、[T0097](TOOLS.md#t0097) |
| [H2101](RULES.md#h2101) | [T0095](TOOLS.md#t0095)、[T0103](TOOLS.md#t0103)、[T0105](TOOLS.md#t0105)、[T0107](TOOLS.md#t0107)、[T0115](TOOLS.md#t0115)、[T0116](TOOLS.md#t0116)、[T0153](TOOLS.md#t0153)、[T0154](TOOLS.md#t0154)、[T0156](TOOLS.md#t0156)、[T0191](TOOLS.md#t0191) |
| [H2102](RULES.md#h2102) | [T0096](TOOLS.md#t0096)、[T0106](TOOLS.md#t0106)、[T0108](TOOLS.md#t0108)、[T0168](TOOLS.md#t0168)、[T0169](TOOLS.md#t0169)、[T0171](TOOLS.md#t0171)、[T0188](TOOLS.md#t0188) |
| [H2103](RULES.md#h2103) | [T0111](TOOLS.md#t0111) |
| [H1801](RULES.md#h1801) | [T0042](TOOLS.md#t0042) |
| [H2303](RULES.md#h2303) | [A1893](ARTIFACTS.md#a1893) |

附注补充：[N0501](RULES.md#n0501) 对应 [F1278](FRONTEND.md#f1278)；[N2101](RULES.md#n2101) 对应 [T0155](TOOLS.md#t0155)。

文档边界校正：D0006–D0007 的 duplicate/missing Cone 来自当前编译器组装的站点目录，使用 ICE2501；D0369–D0371 的显式 README 与资源 URL 错误使用 E1801；D0375 的公开输出路径错误使用 E2004。T0040–T0046 中涉及发布 API 数据结构、schema、target 和 coordinate 的错误使用 E1801，实际文件读取失败仍使用 E2201。
