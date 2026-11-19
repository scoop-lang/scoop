# macOS 最终可执行文件体积调研方案

2026-11-20。基线为 `c4139ec2a` 的发行工具链。第一期在 macOS 上调查三个问题，依次为未使用代码消除（DCE）、大值类型的机器码、系统文本库的替代范围。主要指标是普通 release 可执行文件的磁盘字节数。

## 样本与对照

- 小程序使用随包 Hello 和 file-report，分别观察最小启动闭包、文件访问与异常。UI 使用自绘 Counter，并增加实际使用段落排版的样本，以区分未使用和确实需要的文本依赖。
- 沿用同一提交的编译器、release sysroot、目标、SDK 和默认符号设置。在相同输入上一次改变一个链接或代码生成条件，记录工具版本、样本入口和可复核的命令。记录完整文件、相关 section、最大的符号及运行结果。单函数、临时原型和整程序的数据分别标明口径。
- 正式 Scoop 消费样本用于检查归属和最终尺寸。独立原生探针用于检验系统 API 的公开性、行为与调用约束。构建缓存、链接 map、汇编和逐次输出放在仓库外。

## 1. DCE

复核现有 Scoop 成员选择和 Mach-O LLD 的原生裁剪。用同一批已选对象比较正常链接和 `-dead_strip`，从链接 map 确认被删除的数据究竟出自哪个归档。加入使用真实排版的对照，检查 GC 登记、异常、Objective-C 间接入口、原生初始化的保留条件。梳理 eager 与 lazy 初始化的引用方向，找出可以进一步缩小 Hello 的根，并记录无法由引用关系安全删除的情形。

## 2. 生成的机器码

从 Counter 的链接地址和 `--emit symbols` 定位最大的 Scoop callable。按真实调用签名检查带 tag 枚举的非引用区间、托管指针槽、值传递和栈副本。优先尝试与现有布局一致的整字存储或有界内存复制，用可重复的 ARM64 发射对照区分它们与一般内联、优化等级的影响。设计要给出适用的布局、GC 与 ABI 边界、需要重建哪些产物，以及真实程序的验证方法。

## 3. macOS 系统文本能力

先沿 Cone 和原生符号区分 `scoop.std` 内的 `scoop.text`、`scoop.charset`／`scoop.idna` 共用的 Rust 后端，以及 `scoop.graphics.text` 的 Skia 段落排版。分别测量真实使用和仅带传递依赖时的可执行文件。按用途考察 Foundation、Core Foundation、Core Text 及 SDK 公开的其他接口。

用相同输入比较现有行为与 macOS 系统入口。域名覆盖 Unicode／ASCII 双向、根点、非法 A-label、分隔符、Bidi／连接符和 DNS 长度。文本覆盖 NFC、字素边界及索引单位、大小写、列宽和带状态字符集的错误位置。记录可以采用的系统行为、对调用方可见的偏移及应保留的小型实现。按真实 API 核对 Foundation 的主机名与 URL 解析边界，以及图形排版的绘制、命中和原生资源约束。

## 交付

根据结果给出实施顺序、对应规范和代码入口、可独立验证的改动以及 macOS 定向验收。系统行为可作为 Unicode／IDNA 的参考基线，需标明具体 API、系统版本、实测差异和上层可观察结果。尚待验证的收益保留为估算。库分发尺寸另行记录，本期决策以最终 binary 为先。

## UI 输入值的嵌套布局

从 release `.slib` 追踪 PointerState.time 到 WindowEvent、ApplicationEvent 的 GC 槽。用同一版编译器重编依赖，对照固定单调时钟前后的 `sizeOf`、SurfaceHost.onEvent 机器码及最终 Counter 文件大小。用真实 macOS 输入、窗口 deadline 和离屏帧回归确认时间语义。
