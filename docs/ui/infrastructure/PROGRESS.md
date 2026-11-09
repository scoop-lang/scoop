# UI 基础设施优化

本任务在 `codex/ui-infrastructure` 开发，依次完成下列独立大项。保持既有 UI API；原生 GPU 接入在 graphics 的低层资源边界扩展。每项在本地完成实现与定向验证后集中补验其他平台，提交、合并最新 `origin/main` 并推送。

| 大项 | 范围 | 状态 |
| --- | --- | --- |
| 绘图与输入几何 | 共享裁剪前缀、矩形交集、深层作用域与几何快照成本 | 三平台完成 |
| 设备与原生 GPU buffer | 原生纹理的持有、访问与 GPU 使用期限，平台格式保留，接入现有 Canvas | 三平台完成 |
| UI 更新与呈现 | 基于真实路径减少重复失效传播、队列及每帧处理开销，保持布局与输入语义 | 待实施 |

验证使用所属 Cone 的 release 测试和必要的真实 GPU／窗口测试，按大项运行，不在小改动间重复全量测试。性能记录区分 native 算法和完整 UI 场景；日志与逐次数据放在仓库外。Windows SSH 环境无法执行的交互或 GPU 验证注明原因，留待用户统一验证。

重点场景为 shadcn Gallery 内鼠标在控件间移动及悬停。用户观察到原生 demo 在活动时 GPU 占用可能超过 30%，相近外观的浏览器 demo 不超过 5%；这是待复现的现象，不作为本次已经测得的基线。检查局部交互期间的静态内容重放、离屏纹理、阴影／模糊和帧需求；前后比较保持相同窗口尺寸、显示缩放和交互。

## 绘图与输入几何

保存栈、嵌套 Canvas 和 CanvasGeometry 共享不可变裁剪前缀。连续矩形合并为精确交集，路径链保存累计保守包围盒；命中先排除包围盒之外的点，复杂命中与二进制区域仍使用 Skia。纯矩形可见范围直接求交，快照不因 restore、兄弟作用域或源目标关闭而改变。

macOS aarch64 的 graphics release 测试 **51／51** 通过，包括实际 Metal、离屏、裁剪孔洞、DPI、嵌套作用域及新增的 128 层快照验证。Scoop 格式检查、native `-Wall -Wextra -Werror` 和 diff 检查通过。

Linux `nuc12` 在独立的 `~/repos/scoop-ui-infrastructure` 检出目录复用现成编译器与 native 资源，graphics release **48／48** 通过，包含 surfaceless GLES。Windows 对这批几何变更的覆盖在下述 GPU buffer 的集中验收中完成。

Apple M4 Pro，Apple Clang C++20 `-O2`，三轮顺序执行的 native 微基准中位数如下。8 层小场景分别减少约 9%／17%；随着深度增加，原先复制全部前缀的二次增长被消除。这不是 GPU 使用率测量。

| 裁剪 | 深度 | 原实现 µs／轮 | 当前 µs／轮 |
| --- | ---: | ---: | ---: |
| 矩形 | 64 | 63.54 | 30.54 |
| 矩形 | 256 | 640.26 | 122.38 |
| 矩形 | 1024 | 10355.77 | 489.55 |
| 椭圆路径 | 64 | 65.47 | 29.27 |
| 椭圆路径 | 256 | 651.13 | 113.94 |
| 椭圆路径 | 1024 | 10549.28 | 456.80 |

## 原生 GPU buffer

新增普通 graphics native API：从 Canvas 保留同一 GpuDevice、按平台原生格式创建 GpuBuffer、原生写入借用，以及通过纯 native owner 导入不可变 Image。UI API 不变。写时复制保留指定原生格式；Image／Drawing 与 GPU 消费共同保活。原生完成通知不持有会回指 GPU context 的 SkSurface，避免循环持有。Metal 使用 command buffer 完成通知，D3D12 使用 fence／Win32 线程池等待并保留资源及底层 allocation，GLES 沿 driver 的提交和删除语义。

macOS graphics release **59／59** 通过。新增真实 GPU 测试涵盖原生队列写入、异常后恢复、旧快照、外部导入失败配对、记录与 GPU 使用期限、释放后原生帧池复用、跨 context／线程、R8／RG8、RGBA16F、sRGB，以及没有后续 Skia 工作时排空延迟的原生提交。原生格式不支持时正常抛错，不能把未知 Metal 格式交给会中止进程的 Skia 格式表。

Linux `nuc12` 的 surfaceless GLES＋Raster release 测试 **54／54** 通过，使用独立检出与同一批真实 native 生产者测试。Windows `E:/scoop-ui-infrastructure` 的 D3D12＋Raster release 测试同样 **54／54** 通过，包含实际硬件纹理写入、resource state 交还与复用；本项无需跳过 Windows 验证。两者复用目标机已准备的 compiler／sysroot 资源。

接口选择与资源使用示例见 [原生 GPU 图像接入](NATIVE-GPU.md)。
