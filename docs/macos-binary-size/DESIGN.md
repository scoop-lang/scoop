# macOS 可执行文件瘦身设计

各阶段的测量与落地情况见[实施记录](PROGRESS.md)。首期目标为 `aarch64-apple-darwin` 的普通 release 可执行文件。先交付链接 DCE 和 tagged enum 机器码优化，然后收紧初始化根、替换 macOS 文本后端。优化对象是用户拿到的 Mach-O 文件。代表程序、逐项对照和工具版本见[调研结果](FINDINGS.md)，原始问题和测量口径见[调研方案](RESEARCH-PLAN.md)。

| 顺序 | 改动 | 用于决定是否交付的证据 |
| --- | --- | --- |
| 1 | 对普通发布的 Mach-O executable 开启原生 dead strip | 复现 Hello 的 6.87%、Counter 的 46.69% 降幅，实际入口、GC、异常与原生初始化可运行 |
| 2 | 用整字表示 tagged enum 的非引用区间 | `SurfaceHost.onEvent` 从 1,251,924 降至 102,096 字节。在第一步之上，部分库重编已使 Counter 再小 3,830,784 字节 |
| 3 | 按实际访问保留 lazy 初始化单元 | 用 Hello 和带副作用的跨库程序验证活单元和真实 binary 的变化，现阶段收益待测 |
| 4 | macOS 图形文本使用 Skia Apple Unicode | 同一个实际排版程序在 DCE 后从 16,218,272 降至 4,524,272 字节，现有 66 个 Cone 测试已双边通过 |
| 5 | IDNA 和常用 Unicode 操作使用 macOS 系统接口 | Scoop 域名桥接探针在 DCE 后省 181,504 字节。以完整 IDNA、charset、URL 和 readline 消费路径确定最终收益 |

## 1. 在原生最终链接中开启 DCE

**启用边界。** 先修订实现规范 §2.8 中 Darwin 禁用 `dead_strip` 的条款。对 Mach-O `Executable` 的现有普通发布分支（有效的 `keep_symbols=false` 且没有独立调试文件）向配套 LLD 传 `-dead_strip`。`scoop build`、`scoop run` 和从 root slib 出发的最终链接共用这个决策。保留调试符号／独立调试文件和 native-app 共享库沿用各自的既有链接策略。首期可直接复用输出类别和符号选项。[实现规范](../specs/SCOOP-IMPL-SPEC.md) · [Darwin profile](../../compiler/toolchain/src/final_link/darwin.rs) · [最终链接入口](../../compiler/linker/src/link/darwin.rs)

**保留边界。** 维持 Scoop 已选成员的批次和 `-force_load`。LLD 从入口、显式保留项和 Mach-O 的初始化等 section 沿重定位裁剪。Scoop 的启动入口引用各 image，image 引用已选登记，活 callable 对应的机器根、异常展开信息与可寻址代码必须一起存活。由 C、ObjC 或系统回调进入的代码要靠实际地址表、selector 元数据或已有明确的导出／保留规则建根。若某个按名查找的入口缺少实际根，只补该入口。维持 `-no_deduplicate`、当前 ODR 选择和签名方式。[Mach-O LLD 的根与传播](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.8/lld/MachO/MarkLive.cpp) · [runtime image 与 GC 合同](../specs/SCOOP-RUNTIME-SPEC.md)

**复用。** 让有效裁剪策略进入最终链接参数及现有结果键。Darwin profile 已编码其 `linker_args()`。若在依赖符号策略的调用点追加参数，则让外层 executable 更新键和已解析 link-plan fingerprint 能区分这次策略变化。发布工具链升级后应获得首个新结果，随后相同输入直接命中。这个改动只涉及最终链接。[外层构建键](../../compiler/scoop/src/build/executable/key.rs) · [解析后的链接输入](../../compiler/linker/src/link.rs)

**验收。** 同输入、同 basename 双链，用 link map 核对自绘 Counter 裁掉 ICU、真实段落仍保留其当前 provider，并运行 Hello、file-report 的成功及异常路径、C++ 构造、Objective-C selector、移动 GC 和栈展开用例。保留符号、dSYM 和动态库各检查现有入口与输出策略。不同 body 的 ICF、全程序 LTO 和更换优化等级都独立于本次测量。

## 2. 减少大枚举的机器指令

**先改物理表示。** 在 codegen 的 `tagged_ty` 中保留 `i64` tag。按现有 `RefScan` 提供的升序槽位划分区间，固定 GC 槽仍单独发射 `ptr addrspace(1)`。对齐的非引用区间复用 `MaybeUninit` 已有的整数存储函数。在本目标上，每段可用 `[k x i64]` 承载，按需保留尾字节和原有高对齐锚点。当前 tagged 布局的首个区间从偏移 8 开始，每个引用槽也按 8 字节对齐。以既有布局的 size、align、tag 和字段偏移约束表示。全部 bit pattern、未激活 payload 和 padding 延续现有值语义。[tagged 类型](../../compiler/codegen/src/llvm_types.rs) · [可复用的存储逻辑](../../compiler/codegen/src/llvm_types/maybe_uninit.rs)

**ABI 与 GC。** Darwin 小值 carrier、间接参数、byval/sret 继续服从 LIR 已决定的 ABI。按字节偏移访问 case payload，检查所有依赖 LLVM aggregate 字段索引的路径。在生产者和消费者两端检查直接传参／返回、嵌套可空枚举、跨库接口调用及含托管引用的大值。GC 可见槽位维持类型化的 load/store、实际堆写屏障、safepoint 活根和重定位。实施时先把非引用区间的表示自由度和这些约束写入实现规范 §2.4、§2.5。按既有编译器更新机制重建目标预编译库、ODR 产物及应用，验证所有库一致重建后的 Counter。本次 13,855,072 字节来自部分重编。[ABI 和 codegen 规范](../specs/SCOOP-IMPL-SPEC.md) · [机器根合同](../specs/SCOOP-RUNTIME-SPEC.md) · [LLVM statepoint](https://llvm.org/docs/Statepoints.html)

**再看剩余复制。** 以按字表示后的 `onEvent` 为样本，跟踪剩余 24,785 条 `ldr/str` 到具体的按值实参和返回临时量。优先消除 caller 与 callee 间相邻的重复物化。确实需要复制的大型连续无引用区间才试按已知对齐使用 LLVM `memcpy`，可能重叠的区间采用相应的重叠安全语义。含引用的值按上述槽位和非引用区间分开处理。若使用临时裸地址，拷贝前后须满足现有 safepoint 和屏障规则。此步单独比较单函数指令、整个 Counter、事件分发耗时，收益按新增实验计算。

**验收。** 首批复用嵌套枚举返回、定向移动 GC 和 `SurfaceHostTest`。增加一项跨 Cone 用例覆盖「大非引用区间／引用槽／大非引用区间」，跨调用与回收后读取值和对象身份。用 `llvm-objdump` 抽查热点，再用最终 Mach-O 和 `__text` 计量。微型程序可能因链接对齐或内联得到相同大小，避免用固定汇编快照约束所有程序。[枚举返回 fixture](../../tests/fixtures/compiler-correctness/codegen/nested-enum-return/fixture.toml) · [真实事件回路](../../sysroot/src/scoop.surface/tests/host.scoop)

### UI 使用固定单调时钟

窗口输入、帧采样、动画、手势和 UI 控件定时器都使用进程现有的 `MonotonicTimeSource`。这个决定同时适用于离屏渲染。每帧采样一次，帧内动画和完成回调共用该时刻。平台输入保留原生发生时间 `Event` 与接收时间 `Observed` 的区别，原生回调采到的读数随事件传递。macOS 两个入口都已读取 `CLOCK_MONOTONIC`。[窗口事件时间](../../sysroot/src/scoop.window/native/darwin/window.m) · [平台时钟](../../sysroot/src/scoop.platform/native/time.c)

在 `scoop.platform.time` 提供明确属于这个时钟的值类型 `MonotonicInstant`。系统读数保存整秒 `Long` 和秒内纳秒 `Int`。显式调用可传有符号纳秒调整量，比较、哈希和排期使用规范化读数。在此类型上比较时刻、求时间差和加减 `Duration`。窗口原生入口收到整数后直接构造时刻，应用主动取时由同一个 `MonotonicTimeSource` 读取。只在通用计时 API 的实际边界转换，避免在普通事件转发时构造 `Duration` 或保存 `TimeSource` 接口引用。

- `InputTime` 的 `Event` 和 `Observed` 各携带一个 `MonotonicInstant`，`mark()` 直接返回这个值。保留 `PointerState`、键盘、指针、手势和触点事件的值语义，继续用原有变体分派。调用方可继续匹配 `case Observed(val t)`，其中 `t` 具有固定时钟类型。系统偏好等低频快照按变更后的真实布局单独判断是否适合使用 class。
- 窗口的 `WaitUntil`、`WindowDeadline.schedule`，Surface 的帧时刻、动画起点与截止时刻，以及 UI、组件的输入速度、长按、滚动、轮播、Toast 等消费者，统一传递同一类型。窗口 deadline 的来源由参数类型确定。日历选择器的“今天”继续使用民用日期接口。
- `AnimationTimeline` 只接收失效回调。正常绘制从全局源取时，离屏和测试使用 `advanceTo(time) { ... }` 在显式时刻启动或改目标。启动或改目标需要精确对齐的测试在指定帧内执行，验证完成回调、延期、相同时刻和逆序采样。各个 timeline 只保存自己的帧状态。[窗口 deadline](../../sysroot/src/scoop.window/src/owner/deadlines.scoop) · [动画帧作用域](../../sysroot/src/scoop.surface/src/shared/animation/timeline.scoop)

实施时先更新 platform、window、surface 的相应规范和面向调用者的示例，再重编 UI 依赖。验收使用真实 AppKit 输入、窗口 deadline／动画完成用例、固定读数的离屏推进、跨秒算术和 release Counter。分别记录类型 `sizeOf`、GC 槽、`onEvent` 机器指令和最终 Mach-O 字节数。[实测结果](FINDINGS.md#ui-事件的固定单调时钟)

## 3. 从 Scoop 的根集合裁掉不可达 lazy 单元

**第一步只处理能归属到具体单元的存储。** 现有强初始化语义已经提供单元 ID、eager 或 lazy 调度、成功与失败存储、initializer、ensure、eager gateway 及依赖。最终链接使用的 `ProgramLinkProduction` 投影当前只有定义角色、符号、safepoint、入口和五张登记表。由 LIR 对已有语义取出链接所需的最小投影，按类型化 ID 建立单元与成员关系。已存有这些事实的产物可以复用，链接器不回到源码或重新解释 HIR/MIR。[既有初始化模型](../../compiler/lir/src/production/initialization_registrations/model.rs) · [当前链接投影](../../compiler/lir/src/production/cone_section/link.rs)

- eager 单元的启动登记和 gateway 始终为根，按既有次序运行，保留未被读取但有副作用或可能抛错的顶层初始化。普通静态存储、原生启动、显式入口、runtime 别名和无法归属到 lazy 单元的外部可见存储沿用现有根规则。
- 归属某个 lazy 单元的 cell、登记、结果存储、失败根及其 GC 登记改为随单元存活。来自存活 Scoop／原生代码的 ensure、描述符或存储引用，以及存活单元的语义依赖，均能使该单元和注册它所需的成员存活。成员共享时，任何一个存活所有者都能保留它。未从根可达的相互引用无需单独检测环。
- 用现有工作队列传播并重建五张 image 表，维持实际存活的静态根在 managed 初始化前注册、失败对象可再次抛出、类型与 immortal 登记等关系。首批维持 generated bridge 和现有 ODR bundle 的保留粒度。独立比较这一层与只有原生 DCE 的结果。[引用传播](../../compiler/linker/src/program/selection/reachable.rs) · [image 登记表](../../compiler/linker/src/program/image.rs)

**验收。** 先同步实现规范 §2.8、§2.14 对保留成员与 image 的描述。以未使用的 lazy、跨库读取的 lazy、跨库依赖、eager 副作用、eager 失败和 lazy 重入／失败为对照。检查正式链接 map、程序输出和回收后的失败对象。报告 Hello 实际删除的单元、登记数和整文件字节数，避免把全部 33 个候选当成可删除项。[runtime image fixtures](../../tests/fixtures/compiler-correctness/runtime-images)

## 4. 图形段落采用 Skia 的 Apple 后端

**选择现成的替换面。** macOS 准备 Skia 时设置 `skia_use_icu=false`、`skia_use_apple_unicode=true`。Darwin 归档列表使用 `skunicode_core`、`skunicode_apple`、`icu_bidi`，保留 SkParagraph 和 HarfBuzz。C++ 桥接按目标选用 `SkUnicode_apple.h` 和 `SkUnicodes::Apple::Make()`，使 graphics、graphics.svg、graphics.text 的 Darwin 编译宏与实际归档一致。限定在原来的目标资源准备与平台源选择上，普通消费者使用预构建结果。[Skia 构建与归档](../../scripts/build_skia.py) · [段落桥接](../../sysroot/src/scoop.graphics.text/native/shared/text.cpp) · [上游 Apple 实现与小型 Bidi 工厂](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/modules/skunicode/src/SkUnicode_apple.cpp)

**调用方行为。** macOS 采用 Core Foundation 的分词及断行行为，允许随系统词典或版本变化。继续使用段落现有的 UTF-8 字节位置、字素移动、上／下游 affinity、Bidi 选择几何、行内盒、样式混排与 SkCanvas/Metal 绘制。更新图形文本规范中固定 ICU 的表述和面向使用者的材料。对已知的 CRLF、组合字符、ZWJ、旗帜、前导 BOM、内嵌 NUL 保留结构性断言。复用现有 66 项 Cone 测试及真实排版程序，用现有 SVG 绘制测试覆盖共用同一归档的消费者。以最终 live map 中的 `_icudt78_dat` 和同一程序的磁盘尺寸确认效果。[图形文本规范](../specs/SCOOP-GRAPHICS-TEXT-SPEC.md) · [导航边界测试](../../sysroot/src/scoop.graphics.text/tests/boundaries.scoop) · [SVG 绘制测试](../../sysroot/src/scoop.graphics.svg/tests/drawing.scoop)

## 5. IDNA 与常用 Unicode 操作

保留 `scoop.std` 中的 `scoop.text` 源码实现。把 macOS 平台选择放在真正消费原生后端的 `scoop.idna` 与 `scoop.charset`。各功能有独立的原生符号引用，让只用域名的程序能裁掉编码表，只用 ASCII 的程序能裁掉整条依赖。先修订 IDNA、charset 规范里的固定实现与版本描述，并同步其面向用户的声明材料。[IDNA 规范](../specs/SCOOP-IDNA-SPEC.md) · [charset 规范](../specs/SCOOP-CHARSET-SPEC.md)

| 操作 | macOS 设计 |
| --- | --- |
| 域名与单标签 | 使用 SDK `unicode/uidna.h` 和显式动态库 `icucore`。以 `UIDNA_USE_STD3_RULES`、`UIDNA_CHECK_BIDI`、`UIDNA_CHECK_CONTEXTJ`、两个 `NONTRANSITIONAL` 选项创建 UTS #46 处理器。按 API 所需调用 `uidna_nameToASCII_UTF8` 和 `uidna_nameToUnicodeUTF8`。ASCII 结果检查标签 63 字节、无根点文本 253 字节上限。保留空输入、四种根点、尾点、早期非法字符、单标签限制和有依据的错误标签索引。分别处理 ICU 调用错误与 `UIDNAInfo.errors`，拒绝错误附带的部分结果。显示形式也走同一有效性判定。 |
| 原始 Punycode、版本 | Punycode 直接复用现有 RFC 3492 实现，覆盖空串、全 ASCII 尾连字符、内嵌 NUL。以域名接口实现为 Apple ICU、Unicode 数据来自 `u_getUnicodeVersion` 描述 `Idna.backendVersion/unicodeVersion`。URL 继续消费 IDNA 的结果，保有自己的主机语法、IP 与转义处理。 |
| NFC、大小写、分类 | 使用 `CFStringNormalize` 的 NFC、ICU 的 root lower/upper（显式空 locale）、默认 full fold 和 `u_hasBinaryProperty(UCHAR_POSIX_ALNUM)`。显式长度的 UTF-8 转 UTF-16 后用 `CFStringCreateWithCharacters` 构造字面文本，保留初始 U+FEFF、内嵌 NUL 和补充平面标量。按实际返回长度处理扩展，ICU 的 32 位长度上限在桥接处处理。 |
| 字素 | 以 `CFStringGetRangeOfComposedCharactersAtIndex` 遍历完整 UTF-16 串，合并 CRLF，并维护单向推进的 UTF-16／UTF-8 偏移。桥接输出沿用现有「首字节标记」缓冲区，Scoop 继续返回含首尾的标量索引。允许系统支持的新字素规则。 |
| 终端宽度、字符集 | 宽度先保留 `unicode-width`，增量字符集保留 `encoding_rs` 和既有 BOM、状态、错误偏移、输出背压逻辑。这两处都有会改变屏幕内容或解码数据的实测差异。 |

**库与所有权。** 在 Darwin 为迁移的入口编译小型 C／Core Foundation 适配，包括域名状态消息和版本入口。剩余 Rust 归档仅编入 Punycode、字符集和列宽所需依赖，移除已替代入口的 ICU4X 编译数据。复用现有按目标的打包选择。各入口按生产者选择释放函数。系统适配返回的缓冲通过同一 C 适配层释放，Rust 返回的缓冲继续交回现有 `scoop_text_free`。让域名、Punycode、NFC 和大小写的 Scoop 包装各自使用匹配的清理路径，复制完结果后在 finally 恰好释放一次。空结果使用 null/0，临时 CF 对象和 UIDNA 句柄由各自 API 释放。保留 Scoop 的异常种类、位置单位和可观察的流式进度。[当前 C ABI](../../sysroot/native/text_backend/include/scoop_text_backend.h) · [现有 Rust 输出分配](../../sysroot/native/text_backend/src/bytes.rs) · [RFC 3492](https://www.rfc-editor.org/rfc/rfc3492)

**验收。** 比较 `scoop.idna`、`scoop.charset` 的 Cone 测试及其正式示例，运行 URL 域名和 readline 编辑用例。覆盖四类点符、合法／非法 A-label、Bidi／ContextJ、63／64 与 253／254 边界、Punycode 原始输入、NFC、终态 sigma、ß、CRLF、ZWJ 和原始字节索引。可接受的差异逐例注明系统入口和 OS 版本。以正式示例在相同 DCE 设置下的 A/B 文件差值和 live map 归属报告结果。

## 验证范围

以上步骤可独立提交并复测。先记录两项最高优先级在完整 macOS sysroot 上的收益，再将文本改动与各自开启 DCE 的对照比较。首期测量范围为 Apple Silicon。其他操作系统与架构的数字留给各自的后续验证。压缩 `.slib` 元数据只影响库分发，独立于本期实施顺序。
