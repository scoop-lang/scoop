# macOS 可执行文件体积调研结果

2026-11-20。自绘 Counter 的两个主要成本都可以直接消除。Mach-O 裁剪省下 15,490,912 字节，接着只对 `scoop.surface` 和应用根使用整字枚举表示，又省下 3,830,784 字节。真正使用段落排版的程序有另一条路径。保持 SkParagraph 和 HarfBuzz，改用 Skia 自带的 Apple Unicode 后端，在已经开启裁剪的基础上再省 11,694,000 字节。两组结果来自不同应用。实施顺序和验收见[设计](DESIGN.md)。

## 1. 已选对象还能裁掉多少

测量的是 ARM64 Mach-O 可执行文件的磁盘字节数。两列使用相同的对象、库、入口、符号设置、目标和 LLD。实验只给最终链接追加 `-dead_strip`，保留现有批次的 `-force_load`、签名和 `-no_deduplicate`。

| release 样本 | 现状（字节） | 加入 DCE（字节） | 降幅 | 运行或归属 |
| --- | ---: | ---: | ---: | --- |
| Hello | 816,544 | 760,448 | 6.87% | 两者均输出 `Hello, Scoop!` |
| file-report | 1,097,392 | 1,013,424 | 7.65% | UTF-8 统计相同，缺文件均走异常路径并退出 1 |
| 自绘 Counter | 33,176,768 | 17,685,856 | 46.69% | map 中 `_icudt78_dat` 的 10,876,560 字节进入 dead 列表 |
| 纯 `scoop.text` 的 trim 和 ASCII 小写 | 816,576 | 776,992 | 4.85% | 均输出 `abc`，两张 map 都没有上述 ICU 数据符号 |
| `scoop.idna` 域名示例 | 1,363,360 | 975,968 | 28.41% | 中德文、A-label 和非法域名结果相同 |
| `scoop.charset` 的 NFC、字素示例 | 1,577,584 | 1,095,600 | 30.55% | 重音、ZWJ、组合字符输出相同 |
| `scoop.charset` 编码示例 | 1,594,304 | 1,228,192 | 22.96% | GBK、Windows-1252、严格 ASCII 和替换结果相同 |
| 实际段落排版 | 19,992,080 | 16,218,272 | 18.88% | 均得到 2 行、15 个边界、0 个缺字，ICU 数据在两张 map 中都存活 |

另外三组定向样本在两种链接下均通过。Objective-C ARC、selector 和 AppKit 返回预期值，C++ 静态构造与退出流程输出 `cxx-exit` 和 `flushed`、退出 37，移动 GC 在 `SCOOP_GC_STRESS_MOVE=1` 下验证活根、固定槽和无引用 payload。三个主样本的 `__DATA_CONST,__scoop_gc` 在 DCE 前后分别为 56,816、87,216、967,016 字节。运行检查和 section 大小是两种独立证据。Counter 本身只测了链接和尺寸。[Objective-C fixture](../../tests/fixtures/compiler-correctness/native-objc/values/fixture.toml) · [C++ fixture](../../tests/fixtures/compiler-correctness/native-cxx/exit/fixture.toml) · [移动 GC fixture](../../tests/fixtures/gc-correctness/moving-references/fixture.toml)

Scoop 层已经跟踪选中成员的物理引用，并用保留的定义重建五张 image 登记表。当前把静态存储、GC 根登记、初始化 cell 和初始化登记都当作初始根。所有 generated bridge 成员也会入选。Hello 登记了 33 个 lazy、0 个 eager 初始化单元，file-report 为 51／5，Counter 为 73／7。即使 linker 看不到对某个 lazy 单元的程序访问，已生成的 image 表仍会引用它。33 是候选数，逐个判活后的节省尚未测定。[成员选择](../../compiler/linker/src/program/selection/reachable.rs) · [image 表生成](../../compiler/linker/src/program/image.rs) · [LLD 22.1.8 的存活规则](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.8/lld/MachO/MarkLive.cpp)

## 2. 大枚举生成的 ARM64 机器码

原 Counter 做完 DCE 后，最大的十个 Scoop callable 合计 4,430,436 字节，占 `__text` 的 35.6%。最大的是 `SurfaceHost.onEvent`，它按值接收和转发嵌套的 `ApplicationEvent`、`WindowEvent`。codegen 为 tagged enum 发射 64 位 tag，把每段非引用空间表示为 `[N x i8]`，在固定引用偏移插入托管指针。临时原型只把两处非引用区间交给现有 `MaybeUninit` 的整字存储辅助函数，保证 tag、大小、对齐和引用槽位置一致。两边使用同一编译器提交、LLVM 和优化选项。[真实调用点](../../sysroot/src/scoop.surface/src/shared/host/host.scoop) · [tagged enum 表示](../../compiler/codegen/src/llvm_types.rs) · [整字存储实现](../../compiler/codegen/src/llvm_types/maybe_uninit.rs)

| 同输入、同一测量口径 | 字节数组 | 整字 |
| --- | ---: | ---: |
| `onEvent` 对象的 `__text`（字节） | 1,251,924 | 102,096 |
| `onEvent` 的 ARM64 指令数 | 312,981 | 25,524 |
| 其中 `add / ldrb / strb` | 145,365 / 39,021 / 39,023 | 291 / 1 / 3 |
| `scoop.surface` 全部 LinkObject 的 `__text` 之和（字节） | 6,003,028 | 1,910,432 |
| `scoop.surface.slib` 文件（字节） | 17,043,052 | 15,594,784 |
| Counter 普通链接（字节） | 33,176,768 | 29,329,456 |
| Counter 加 DCE（字节） | 17,685,856 | 13,855,072 |

对照重建出的基线 `scoop.surface.slib` 与发行版逐字节一致。Counter 的对照只重编该库和应用根，其余系统库沿用发行版。整字版本仍有 12,524 条 `str` 和 12,261 条 `ldr`，有界块复制还有研究价值。用整字版本加 DCE 跑真实的 `SurfaceHostTest`，原生事件循环、帧回调、输入视图和窗口退出均通过。[SurfaceHost 测试](../../sysroot/src/scoop.surface/tests/host.scoop)

嵌套可空枚举返回 fixture、移动 GC fixture、另一个跨 Cone 的大值探针均在两种表示和两种链接下执行成功。大值探针在枚举的两个 512 字节数据段之间携带对象引用，跨库按值返回，触发 GC 后分别验证空、单引用和大 payload 三条路径。[枚举返回 fixture](../../tests/fixtures/compiler-correctness/codegen/nested-enum-return/fixture.toml)

## 3. 哪些文本操作适合交给 macOS

`scoop.text` 是 `scoop.std` 的纯 Scoop 包，提供 ASCII、搜索、数字格式化等操作。原生 Rust 静态库 `libscoop_text_backend.a`（4,182,680 字节）由 `scoop.charset` 和 `scoop.idna` 声明。图形段落则由 `scoop.graphics.text` 引用 Skia 的 ICU4C。这三条路径可以独立改变。[纯文本包](../../sysroot/src/scoop.std/src/text) · [charset](../../sysroot/src/scoop.charset/Cone.toml) · [IDNA](../../sysroot/src/scoop.idna/Cone.toml) · [图形文本桥接](../../sysroot/src/scoop.graphics.text/native/shared/text.cpp)

### 图形排版

发行版已经用 Core Text 获取系统字体，段落分析显式调用 `SkUnicodes::ICU::Make()`。项目锁定的 Skia 提供 `SkUnicodes::Apple::Make()`，字素与分词使用 Core Foundation，双向文本沿用 Skia 的小型 ICU Bidi 子集。临时重建关闭 `skia_use_icu`、打开 `skia_use_apple_unicode`，把静态归档中的 `skunicode_icu + icu` 换为 `skunicode_apple + icu_bidi`，并将桥接切换到 Apple 工厂。SkParagraph、HarfBuzz、字体和绘制路径都参与正常 Scoop 构建。[当前 Skia 构建](../../scripts/build_skia.py) · [锁定版本的 Apple 实现](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/modules/skunicode/src/SkUnicode_apple.cpp) · [上游构建目标](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/modules/skunicode/BUILD.gn)

| 同一个段落程序（字节） | Skia ICU | Skia Apple |
| --- | ---: | ---: |
| 普通链接 | 19,992,080 | 8,163,584 |
| 加 DCE | 16,218,272 | 4,524,272 |
| 加 DCE 后 `__text` | 3,624,856 | 3,183,076 |
| 加 DCE 后 `_icudt78_dat` | 10,876,560 | 无此符号 |

两种后端的普通链接及 DCE 版本均实际执行排版，输出一致。另测前导 U+FEFF、内嵌 NUL、CRLF、旗帜和 emoji ZWJ，公开的 UTF-8 字节边界相同。两个后端分别运行 `scoop.graphics.text` 的完整 release Cone 测试，均为 66/66，覆盖混排、行内盒、导航、命中、字体与 Metal 绘制。共用 Skia 归档的 `scoop.graphics.svg` 在两边也均为 14/14。独立 Core Text 探针用 `CTTypesetter` 排出两行、七个 glyph run，其中一个由右向左。现有 Skia Apple 后端已经覆盖本期段落需求。[图形文本测试](../../sysroot/src/scoop.graphics.text/tests) · [SVG 测试](../../sysroot/src/scoop.graphics.svg/tests) · [Apple Core Text 排版接口](https://developer.apple.com/documentation/coretext/cttypesetter)

### 域名、Unicode 与字符集

使用系统自带的 SDK 头和 `libicucore.tbd`，实编、实链、执行 UTF-8 UIDNA、大小写和属性入口。独立双向域名探针由同一个 Scoop 根调用一段原生桥接，输入 `bücher.example`，输出两种名字。加 DCE 后，静态 Rust 为 740,944 字节，动态系统 ICU 为 559,440 字节，差 181,504 字节。它量到的是域名转换路径。上表 975,968 字节的正式 IDNA 示例还包含 Scoop API 和错误处理。当前 Rust 与本机 ICU 均报告 Unicode 17.0.0。[公开 UIDNA 接口](https://unicode-org.github.io/icu-docs/apidoc/dev/icu4c/uidna_8h.html) · [现有域名包装](../../sysroot/native/text_backend/src/domain.rs)

| 实测输入或操作 | 当前后端与 macOS 的结果 | 对设计的影响 |
| --- | --- | --- |
| 25 个域名输入 | 带 STD3、Bidi、ContextJ、双向非过渡选项的 UIDNA 与当前库对 `faß.de`、雪人、大小写 A-label、合法／非法连接符和混合 Bidi 得到相同有效性。`a·b.com` 两边均接受。原始系统 UIDNA 给根点报空标签，64 字节标签和 254 字节域名只有 ToASCII 报长度错 | 适配根点、尾点、DNS 长度和错误位置，两种输出共用一次有效性判定 |
| Foundation 的 `URLComponents.host → encodedHost` | 接受 `a_b.example`、`a.-b`、`xn--` 和 64 字节标签。将 `。` 作为整个 host 时取不到有效 URL | 域名库选 UIDNA，URL 继续走现有主机解析合同 |
| NFC、大小写与字素 | 7 组样本的 NFC 和三种大小写映射相同，包括 Hangul、终态 sigma、ß、内嵌 NUL。系统 CF 原始字素分段会把 CRLF 拆开。`A👩‍💻é🇨🇳` 两边的标量边界同为 `[0,1,4,6,8]` | 显式 UTF-16 单元构造保留前导 U+FEFF，合并 CRLF，转换回 Scoop 索引 |
| 标量属性和列宽 | 13 个标量的 `UCHAR_POSIX_ALNUM` 相同。`wcwidth_l(en_US.UTF-8)` 将 U+17D8 从 3 列改为 1，U+1FAE9 从 2 改为 -1，NUL 从 -1 改为 0 | readline 会把负宽度的可打印字符显示为替代字符，保留现有宽度实现 |
| 字符编码 | Big5 `88 62` 当前得到 U+00CA U+0304，系统 iconv 得到 U+F325。GBK `80` 当前为欧元，系统报错。Shift-JIS `81 60` 分别为 U+FF5E、U+301C。ISO-2022-JP 末尾 `1B 24` 处，Scoop 已消费 2 字节并报告偏移 0 的不完整输入，iconv 返回 EINVAL 且消费 0 | 保留 encoding_rs 和已有增量状态、背压及异常语义 |

系统行为允许随 macOS 演进。编辑索引、原始输入字节偏移、流式消费量和输出内存所有权仍由公开 Scoop API 定义。[UTS #46](https://www.unicode.org/reports/tr46/) · [Apple 的 encodedHost](https://developer.apple.com/documentation/foundation/urlcomponents/encodedhost) · [UAX #29](https://www.unicode.org/reports/tr29/) · [WHATWG Encoding](https://encoding.spec.whatwg.org/) · [readline 渲染](../../sysroot/src/scoop.readline/src/render/layout.scoop)

## 4. 复测口径与库大小

- 基线提交 `c4139ec2a`，发行包 `0.0.0-dev.20261119-aarch64-apple-darwin`。实测 macOS 27.2.1、Apple M4 Pro、SDK 27.2、Mach-O 最低版本 27.0.1、LLVM/LLD 22.1.8。原生系统接口探针使用 Apple clang 21。所有百分比以同一行或明确指定的对照为分母。研究工作目录和原始 map 均在仓库外。
- 链接复测用 `scoop build <输入> --release --sysroot <发行根> --linker <LLD 包装器> -o <目录>/program`。包装器将原参数原样交给配套 LLD 生成 map，再以相同 basename 追加 `-dead_strip -o <另一目录>/program -map <另一张 map>`。用 `stat -f %z` 取字节数，以 map 的 live／dead 区分符号归属。样本入口见 [Hello](../../sysroot/share/scoop/examples/programs/start/hello)、[file-report](../../sysroot/share/scoop/examples/programs/apps/file-report)、[Counter](../../sysroot/src/scoop.ui/examples/custom)、[IDNA](../../sysroot/src/scoop.idna/examples/domain_names.scoop)和[charset](../../sysroot/src/scoop.charset/examples)。
- 段落样本以 `FontCollection(true)`、字号 18 和宽 96 对 `abc אבג 中文 e\u{301} 👩‍💻` 做 `Paragraph.layout`。机器码复测用同一源树重建两个 `scoopc`，分别 `scoop build <scoop.surface> --emit-slib --scoopc <编译器> --release`，再让 Counter 消费该库。按链接 map 和符号输出找到 callable 的唯一定义对象，用 `llvm-objdump -d` 统计其代码段，排除别名和 GC 数据。
- 发行包 44 个 release `.slib` 共 548,121,728 字节，压缩的 HIR／MIR／LIR 合计 382,528,914 字节。离线用目录成员序号替代已有成员 ID 引用，同压缩级别下这组帧降至 369,565,826 字节。该估算仅含帧，正式 reader 和构建成本尚未验证。此项留给后续的库分发优化。

## UI 事件的固定单调时钟

在 macOS arm64 上，以 `6bfbafa10` 的相同 release 编译器、链接选项和 Counter 源码，仅迁移 UI 时钟并重编其依赖。Counter 从 **12,550,608** 降到 **11,592,848 字节**，减少 **957,760 字节（7.63%）**。其 Mach-O 的 `__TEXT,__text` 从 7,339,016 降到 6,495,560 字节。[Counter 样本](../../sysroot/src/scoop.ui/examples/custom)

下表中的大小来自可执行程序实际运行 `sizeOf<T>()`。GC 槽数取自同批发布的 `.slib` 中各值的扫描偏移。扫描槽数包含所有可能的变体。

| 值类型 | 原字节数 | 固定时钟后字节数 | 原 GC 槽 | 固定时钟后 GC 槽 |
| --- | ---: | ---: | ---: | ---: |
| InputTime | 56 | 24 | 4 | 0 |
| PointerState | 296 | 264 | 4 | 0 |
| KeyEvent | 360 | 328 | 7 | 3 |
| PointerEvent | 1,624 | 312 | 24 | 0 |
| ContactEvent | 1,064 | 304 | 16 | 0 |
| GestureEvent | 944 | 288 | 12 | 0 |
| WindowEvent | 4,224 | 688 | 60 | 4 |
| ApplicationEvent | 4,448 | 912 | 65 | 9 |

此前 `PointerState` 的四个 GC 槽全部来自 `InputTime`。两个变体各自保存一个 `TimeMark`，其中含时间源和 Duration 的托管引用。含引用的变体各占一整个独立 payload，全体无引用变体共用一块、大小取最大值。这条规则在每层嵌套生效。旧 `WindowEvent` 的布局为 8 字节 tag、184 字节共享区、1,064 字节 Contact、360 字节 Keyboard、1,624 字节 Pointer、944 字节 Gesture 和 40 字节 TextInput，合计 4,224 字节。[枚举布局实现](../../compiler/lir-lower/src/metadata/representation.rs) · [窗口事件变体](../../sysroot/src/scoop.window/src/application.scoop)

新时间点为 16 字节。指针、触点和手势都成为纯值后，三者在 `WindowEvent` 中共用 312 字节。加上 8 字节 tag、328 字节 Keyboard 和 40 字节 TextInput，得到 688 字节。高频输入保留按值传递。此轮收益全部来自去掉时钟引用及其嵌套影响。[指针输入](../../sysroot/src/scoop.window/src/input/pointer.scoop) · [输入时间](../../sysroot/src/scoop.window/src/input/time.scoop)

从两版 `scoop.surface.slib` 提取 `SurfaceHost.onEvent` 的唯一定义对象，`llvm-objdump -d` 得到 21,680 → 5,940 字节，5,420 → 1,485 条 ARM64 指令，其中 `ldr` 与 `str` 合计 4,707 → 886 条。归档本身的大小也有变化：`scoop.window.slib` 减少 129,164 字节，`scoop.surface.slib` 减少 331,152 字节，`scoop.ui.slib` 减少 794,947 字节，新增公共值类型的 `scoop.platform.slib` 增加 136,483 字节。这些是单独分发的压缩库文件，最终 binary 使用前述直接测量值。[事件转发](../../sysroot/src/scoop.surface/src/shared/host/host.scoop)
