# 大型库编译内存调查

2026-11-23，在源码基线 `38a57c705190929bebe724faa4b33fc5584d9787` 上调查
`scoop.ui.component` 的真实源码编译。宿主为 macOS / Apple M3 Ultra，使用仓库配套的
LLVM 22.1.8；编译器和被编译的 Scoop 库均为 release。

该库有 438 个 `.scoop` 文件、68,110 行，源文本合计 2,407,818 bytes，消费 13 个
预编译依赖。两次对照使用相同源码、依赖、工具链及 `--jobs 4`，分别使用新的
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

| 指标 | 原版 | 共享对象与重定位数据后 |
| --- | ---: | ---: |
| 峰值 RSS | 27,206,696,960 bytes（25.34 GiB） | 18,692,079,616 bytes（17.41 GiB） |
| `.slib` 大小 | 150,493,710 bytes | 150,493,710 bytes |
| `.slib` 内容 | 两版逐字节相同 | 两版逐字节相同 |

峰值减少 8,514,617,344 bytes（7.93 GiB，31.3%）。这是一次配对测量，包含采样，
不作为稳定编译速度的结论，也不外推其他平台的绝对内存值。

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

剩余 17.41 GiB 仍然偏高，后续优先调查以下两项，尚未实施或承诺收益：

- **依赖元数据的展开与重复持有。** 13 个依赖的 HIR／MIR／LIR 解压原始数据合计
  305.30 MiB，但机器层依赖加载使在用堆从 2.52 GiB 增至 5.59 GiB。需要继续区分
  类型化解码结果、派生索引和不同消费者的副本，优先共享不变数据、去掉无实际用途的保留。
  单纯清理原始解压字节缓存只能影响几百 MiB。
- **缩短各阶段数据的存活时间。** 当前编排中 AST／语义世界、HIR、MIR、LIR 及产物准备
  数据有较长的重叠存活期。可在最后一个实际消费者完成后释放正文，只保留后续降级、
  诊断或打包需要的结构。具体释放边界需按真实引用关系处理，不能牺牲阶段输出完备性。

初步排查还用工作区原有编译器执行了 `--jobs 1`，峰值为 25.71 GiB；该编译器版本
早于上述源码基线，因此不用于计算优化比例，仅辅助判断并发数不是主要原因。
这次没有降低 LLVM 优化等级，也没有改变库拆分或语言能力。

验证：`cargo fmt --all`、`cargo clippy --workspace --all-targets` 和 release 构建通过；
Clippy 保留仓库已有告警。`cargo test -p scoop-slib -p scoopc` 的 379 项测试全部通过。
正式 CLI 覆盖 `generic-reader`、`runtime-images`、`direct-c/artifacts`、
`interfaces/artifacts`，共 19 个 fixture、21 个配置组合、134 次进程调用全部通过。
首次 fixture 的旧配套包管理器无法解析新增的 `ios` 目标条件，重建当前版本的配套命令后
重跑受影响用例通过。验证目标为 Darwin/AArch64；未执行全仓库 fixture，也未进行尚未准备好
的 iOS 目标产物与运行验收。
