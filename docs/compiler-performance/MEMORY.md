# 大型库编译内存优化

2026-11-23，在源码基线 `38a57c705190929bebe724faa4b33fc5584d9787` 上调查
`scoop.ui.component` 的真实源码编译。宿主为 macOS / Apple M3 Ultra，使用仓库配套的
LLVM 22.1.8；编译器和被编译的 Scoop 库均为 release。

该库有 438 个 `.scoop` 文件、68,110 行，源文本合计 2,407,818 bytes，消费 13 个
预编译依赖。各次对照使用相同源码、依赖、工具链及 `--jobs 4`，分别使用新的
Cone cache 和输出目录，确认本库实际重新编译。系统文件缓存保持正常。
`/usr/bin/time -l` 记录最大 RSS，另对编译器子进程采样；阶段边界临时记录 RSS
及 macOS allocator 的在用堆大小。临时测量代码已撤除，原始日志保留在仓库外。

复现编译形状如下；每次测量分别提供新的缓存和输出目录，并选择对应版本的编译器：

```sh
/usr/bin/time -l target/release/scoop build sysroot/src/scoop.ui.component \
  --sysroot "$PWD/sysroot" --scoopc /path/to/scoopc \
  --profile release --emit-slib --jobs 4 \
  --cache-dir /path/to/fresh-cache --target-dir /path/to/fresh-output \
  --message-format json
```

| 累计改动 | 峰值 RSS（bytes） | 峰值 RSS（GiB） |
| --- | ---: | ---: |
| 原版 | 27,206,696,960 | 25.34 |
| 共享对象与重定位数据 | 18,692,079,616 | 17.41 |
| 提前释放各阶段 IR 和临时索引 | 17,469,308,928 | 16.27 |
| 释放依赖解码缓冲并精简实体驻留表 | 16,361,734,144 | 15.24 |
| 转移打包 payload 所有权并逐层释放 | 15,876,620,288 | 14.79 |
| 合并 main `4da7ec626` 后复验 | 16,136,372,224 | 15.03 |

同一源码基线上的四类优化使峰值减少 11,330,076,672 bytes（10.55 GiB，41.6%）。
随后合并 main 的 ELF／COFF 目标契约重构 `4da7ec626`，在相同 Scoop 源码与依赖上再次冷编译，
峰值为 15.03 GiB，比初始基线少 11,070,324,736 bytes（10.31 GiB，40.7%）。
表中保留合并前后的独立结果。各版本各测量一次，包含采样，
部分测量同时运行 Rust 构建或测试，不作为稳定编译速度的结论，也不外推其他平台的绝对内存值。
每次产出的 `.slib` 均为 150,493,710 bytes，与原版逐字节相同；SHA-256 为
`be016f002d98003c3dbb76f451cdfe627a77489ce61a53df1d3ab9928a943d28`。

阶段记录说明高占用在进入 LLVM 之前已经存在，而最高峰出现在对象处理与打包阶段：

| 原版阶段边界 | 进程 RSS | 在用堆 |
| --- | ---: | ---: |
| 依赖 HIR 读取完成 | 1.58 GiB | 1.34 GiB |
| 当前库 HIR 降低完成 | 3.21 GiB | 2.02 GiB |
| 依赖机器层元数据读取完成 | 7.03 GiB | 5.59 GiB |
| LIR 降低完成 | 9.46 GiB | 7.62 GiB |
| LLVM 发射开始 | 9.98 GiB | 8.13 GiB |
| LLVM 发射结束 | 11.69 GiB | 8.60 GiB |

对象需求分类、GC／运行时登记、内容摘要等记录通过 `Clone` 保留
`VerifiedCurrentConeStrongRelocationClosureV1`。原结构内的成员解析索引与整库
重定位绑定表都是 `Vec`，嵌套克隆会重复分配其中的成员、符号、字符串和重定位数据。
采样中的大量分配及释放落在这条路径上。将两个只读字段改为 `Arc<[T]>` 后，
各记录共用同一份数据；真正改写对象的 GC 表压缩继续建立新结果。
规范对应[实现规范 2.5](../specs/SCOOP-IMPL-SPEC.md)，产物格式、ABI 和检查边界不变。

继续沿依赖读取、阶段编排和产物打包检查，修复了以下三类重复持有：

- **阶段 IR。** 告警、测试目录和文档投影完成后释放 AST 与 HIR 查询世界；MIR 和类型桥
  导出完成后释放 HIR 正文；LIR 和布局导出完成后从 MIR 转移打包所需的规范记录；
  LLVM 与 generated-C 发射完成后释放 LIR 正文。选中依赖的临时索引和诊断目录也在最后
  一个消费者之后释放。foundation、公开 metadata 和生产记录仍然完整，阶段观察按原时机完成。
- **依赖元数据。** 文件形式的编译依赖用临时缓冲解压，形成完整的类型化记录后立即释放
  HIR／MIR／LIR 原始 payload。稳定借用成员字节的接口继续共享原有缓冲，解码和 I/O
  规则由两条路径共用；后续机器层复用已解码的 HIR。实体驻留表删除与 typed id 索引、
  共享规范键重复的 `CanonicalKeySlot`，保留原有身份类型与内容冲突检查。
- **metadata 打包。** `LayerAssembly` 接收 section payload 的所有权，取消整层深复制。
  HIR 接口投影编码后释放；语义指纹消费完成后逐层编码、释放中间缓冲，再构建 LIR 层。
  规范排序、共享 ID 表、指纹算法与归档字节保持原规则。

独立加载同一组 13 个依赖时，机器层加载完成的在用堆从 4.33 GiB 降至 3.82 GiB，
减少 517.55 MiB。其中已消费原始 payload 的缓存容量减少 369.66 MiB，其余来自实体
驻留表精简。这些依赖原始 HIR／MIR／LIR 解压数据合计 305.30 MiB；缓冲容量可高于
有效长度。报告区分 allocator 的在用堆和进程 RSS，释放后 allocator 可以保留页面用于复用。
生命周期与 metadata 所有权规则分别见实现规范 2.0 和 2.6。

本次保持相同的 LLVM 优化等级、库划分和语言能力。最终 15.03 GiB 仍然是较大的占用，
上述收益针对已确认的重复分配与过长存活期，不代表所有大程序都具有相同优化幅度。

合并 main 后复验：`cargo fmt --all`、`cargo clippy --workspace --all-targets` 和 release 构建通过；
Clippy 保留仓库已有告警，改动处没有新增告警。
对 `scoop`、`scoopc`、`scoop-identity`、`scoop-mir`、`scoop-lir`、`scoop-slib`、
`scoop-codegen`、`scoop-linker`、`scoop-target-data` 和 `scoop-toolchain` 执行 Cargo 测试，
共 1,989 项全部通过。扩展 Rust 测试首次使用了旧 debug 配套包管理器，出现 ABI 和协议失配；
按当前源码重建 debug 配套命令后完整重跑通过。
正式 CLI 覆盖 `generic-reader`、`runtime-images`、`direct-c/artifacts`、
`interfaces/artifacts`、`doc-cli/basic`、`cache/process-observations`、
`source-inputs/testing` 和 `test-artifact-dependencies`，共 23 个 fixture、25 个配置组合、
156 次进程调用全部通过。配套 `scoop`、`scoopc`、`scoop-link` 和 `scoop-pm` 均按当前源码
重建，本机 runtime 按合并后的构建参数重新准备，CLI 验收使用新的共享 Cone cache。
fixture runner 的 Ruff 0.16.10 格式化和检查通过，Python 规则测试 80 项通过、4 项 Windows
专属测试按平台跳过。验证目标为 Darwin/AArch64；未执行全仓库 fixture，
也未进行尚未准备好的 iOS 目标产物与运行验收。

## 后续内存与编译速度优化

后续工作以已合并的 `d38e2b7dc` 为基准，继续使用上述源码、13 个预编译依赖、
LLVM 22.1.8、release 和 `--jobs 4`。每次为当前库建立新的 cache 与输出目录，
进程观察确认 13 个依赖为 prebuilt、当前库为 compiled。原始记录保留在仓库外。

| 累计改动 | 冷编译耗时 | 峰值 RSS（bytes） | 峰值 RSS（GiB） |
| --- | ---: | ---: | ---: |
| 已合并基线 `d38e2b7dc` | 321.63 s | 15,939,272,704 | 14.84 |
| 对象成员二分查询 | 268.66 s | 16,507,355,136 | 15.37 |
| 候选快照共享声明和来源记录 | 131.01 s | 16,292,593,664 | 15.17 |
| 后端前释放依赖 reader 与语义 IR | 134.52 s | 12,918,145,024 | 12.03 |
| MIR／LIR 查重和反查使用索引 | 117.24 s | 12,851,068,928 | 11.97 |
| 共享成员解析记录并消除 registration 对象副本 | 110.82 s | 12,102,942,720 | 11.27 |

对象计划已经按成员 ID 排序，Scoop/generated-C 对象绑定及最终成员装配直接在该表上
二分查询，取消逐对象扫描整张成员表。缺失成员与 unit 归属诊断保持原规则，
没有新增常驻查询表。这一项主要减少 CPU 工作，不改变对象记录的存储方式；
单次测量的 RSS 未下降，不将其计入内存优化收益。

两次耗时均为单次带采样记录，采样窗口数量不同；53 秒的差值只作为初步观测，
不宣称稳定加速比例。此次 `.slib` 为 150,493,710 bytes，SHA-256 与上文基线相同。
格式化、workspace Clippy、release 配套命令构建通过，成员计划和实际对象绑定的
7 项定向测试通过。全量 fixture 与跨平台验证在全部后续改动完成后执行。

HIR 候选快照继续使用已有的写时复制存储，共享声明表、实体 arena 和身份投影；
候选真正修改某张表时才分离。默认参数的 AST 按表达式共享，追加的来源记录使用
持久化序列共享条目。候选失败回滚、成功提交和最终 Export HIR 的拥有型输出不变。
全部 2,148 项 HIR lowering 测试、workspace Clippy 和 release 构建通过。
新的冷编译仍为 13 个预编译依赖和 1 个实际编译库，产物与前两次逐字节一致。
这一项单次测量比前一项少 137.65 秒，峰值 RSS 变化较小；耗时只报告本机观测，
不外推稳定加速比例。此时的采样热点已进入 MIR exact type 注册，将随查找优化继续处理。

语义导出结束后，driver 先编码不会再变化的 HIR／MIR 和 LIR 接口，再释放其查询输入。
编译请求转移依赖 reader 的所有权，避免请求外层一直持有 HIR 与身份会话；机器依赖
只留下对象校验需要的符号所有权和 ODR 复用需要的对象摘要，符号表直接转移。
物理导入记录通过共享不可变存储供后续对象检查使用。最终 LIR foundation、对象生产
和链接闭包仍在对象处理完成后编码，格式、指纹与必要检查不变。

独立加载相同 13 个依赖的在用堆为 4,104,460,544 bytes；转移发布数据并释放 reader、
会话及文件输入后为 132,124,064 bytes（126.00 MiB），保留 467,004 条符号记录。
实际冷编译峰值比前一项减少 3,374,448,640 bytes（3.14 GiB），耗时接近；
allocator 在用堆和进程最大 RSS 是不同测量，不直接互相相减。
380 项 LIR、271 项 slib 和 79 项 driver 编译发布测试、workspace Clippy 及 release
构建全部通过，冷编译产物与基线逐字节一致。

MIR exact type 注册按 HIR arena ID 直接查询，完成注册后集中检查一对一关系，
取消每次注册前扫描全部已有类型。exact type、callable 物化和 nominal shape 的反查
使用排序位置表，索引不复制 IR 正文；重复类型与物化记录仍报告原来的首个位置。
LIR 导出只在当前投影期间建立 layout／descriptor 的位置索引，已有 foundation 的
layout／scan 表直接按身份二分查询，保留规范键、重复项及物理布局检查。
380 项 LIR、196 项 LIR lowering、248 项 MIR 和 213 项 MIR lowering 测试通过，
workspace Clippy 和 release 构建通过。冷编译为 117.24 秒、11.97 GiB，产物逐字节一致；
这是单次带采样观测，主要收益为减少查找时间，峰值内存与前一项接近。

对象成员内部的定义、重定位及 atom 索引改为共享不可变记录，GC 压缩只重读变化成员；
原对象的 section inventory 也共享。符号名从解析表一路共享到定义、重定位和绑定，
临时符号索引借用已有名字，按符号序号查询返回引用。registration 结果不再复制并
持有原对象字节，归档直接从 driver 已有缓冲装配；原对象检查、实际内容指纹和最终
成员身份检查保持。测试覆盖变化成员与旧结果隔离、未变化成员共享，以及输入缓冲
释放后的 registration 记录使用。271 项 slib、79 项 driver 发布测试、workspace Clippy
和 release 构建通过。冷编译产物逐字节一致，峰值比前一项少 748,126,208 bytes
（713.47 MiB）；110.82 秒仍为单次本机观测。
