# UI 基础设施优化

本任务在 `codex/ui-infrastructure` 开发，依次完成下列独立大项。保持既有 UI API；原生 GPU 接入在 graphics 的低层资源边界扩展。每项在本地完成实现与定向验证后集中补验其他平台，提交、合并最新 `origin/main` 并推送。

| 大项 | 范围 | 状态 |
| --- | --- | --- |
| 绘图与输入几何 | 共享裁剪前缀、矩形交集、深层作用域与几何快照成本 | 三平台完成 |
| 设备与原生 GPU buffer | 原生纹理的持有、访问与 GPU 使用期限，平台格式保留，接入现有 Canvas | 三平台完成 |
| GPU 绘制与合成 | 按本次实际命令约束透明度图层、圆角环专用绘制、已知装饰范围 | 三平台 graphics 验证完成 |
| UI 更新与呈现 | 帧依赖检查合并共享祖先失效，保持后续更新、布局与输入语义 | macOS／Linux 完成；Windows 隐藏窗口测试受 SSH 会话限制 |

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

## GPU 绘制与合成

对未声明 bounds、没有滤镜的普通 GPU SrcOver 透明度组，暂存当前组的 Skia 命令，由 Skia 计算实际设备空间范围，再按范围与一个物理像素的抗锯齿余量合成。每次绘图回调仍运行，变换、嵌套借用、图片持有、复杂 clip、组内滤镜外扩与异常收尾保留原语义；没有读取像素回 CPU，也没有新增 GPU 等待。显式 bounds、Raster 和 Drawing 保持各自流程。

PathBuilder 保留两个圆角轮廓的构造信息，完整包含关系只在未改变的构建结果上检查一次。EvenOdd 圆角环填充交给 Skia 的专用绘制，普通边框不再每帧生成通用路径图集。相交轮廓、相接边、NonZero、描边和其他路径继续使用一般路径；抗锯齿细边不足一个物理像素、或显式 Drawing 的最终缩放未知时也保留一般路径，避免覆盖范围被放大。阴影及分色边框的已知源范围同时传给已有 saveLayer bounds；未知子内容没有附加布局裁剪。UI 公开 API 未改变。

macOS graphics release **64／64**、Linux GLES **59／59**、Windows D3D12 **59／59** 通过。新增覆盖包括层恢复／异常退出、重复透明度只应用一次、图片提前关闭、组内和组外滤镜、复杂裁剪与变换、圆角交叉区域、相同轮廓、描边、填充规则及 builder 快照。源范围与显式大范围图层逐像素对照保留必要容差；亚像素边框在同一 GPU 上与等价的一般路径逐像素对照，包含 Drawing 记录后缩小回放。

Apple M4 Pro，同一显示与实际 drawable 2240×1346，真实 shadcn Gallery；基线与最终构建各运行三次并交替顺序。基线为前两项优化完成后、引入本节渲染优化与帧失效合并之前的程序；已有 main 的文字缓存／图层优化不计作本项收益。每次在控件间往返 240 次后等待动画结束，统计实际完成的 command buffer。三次中位数：

| 指标 | 改前 | 改后 | 变化 |
| --- | ---: | ---: | ---: |
| GPU 执行时间／帧 | 0.761 ms | 0.373 ms | −51.0% |
| render pass／帧 | 3 | 2 | −33.3% |
| 渲染目标面积总和／帧 | 7,209,344 像素 | 3,047,808 像素 | −57.7% |
| 静止期新帧／提交 | 0／0 | 0／0 | 保持按需停帧 |

原来每帧有 1024×1024、4×MSAA 的路径图集，及 2048×1536 的透明度纹理。改后该场景不再生成路径图集，透明度纹理为 256×128，后者面积为原来的 1／96。面积统计不按 MSAA 或像素格式加权；GPU 时间也不是整机 GPU 使用率，不能据此宣称用户观察的 30% 已下降到某个 OS 百分比。

复现命令及前后交替运行方式见 [UI 测量说明](../../../benchmark/ui/README.md)。原始日志和 JSON 保存在仓库外。

## UI 帧失效传播

仅在进入 measure／scene 之前的同步帧依赖检查中使用一次共同标记：同一类变化沿共享祖先合并，先收到场景变化的祖先仍可升级为布局变化。普通 State／输入、生命周期、后续帧及帧完成回调不复用该标记，不用跨帧“已脏”状态吞掉更新。

真实隐藏窗口中建立 24 层公共路径与 64 个叶子，验证每个变化叶子都失效，共享祖先每类版本只推进一次，未涉及兄弟保持有效；随后验证新帧、未消费的旧输入、连续普通请求和取消依赖。测试复用 composition 所属的单一 Application，避免在同一进程建立第二个 native application。

集成回归：macOS UI **102／102**、style **19／19**、component **208／208**；Linux UI **101／101**、style **19／19**。Windows 增补同一隐藏窗口用例和 style 的真实 D3D12 绘制入口，style **19／19** 及 UI 三项纯逻辑测试通过。

Windows UI 的隐藏窗口组合测试在创建 Application 时失败，尚未进入 UI 逻辑。SSH 位于 Session 0，最小 Win32 程序确认初始化要求的 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` 返回 `ERROR_ACCESS_DENIED (5)`；当前 console 用户在 Session 1。按任务约定，此项留待 Windows 交互会话统一补验。没有绕过 DPI 合同或把失败标成通过；真实 D3D12 离屏绘图不受该限制。
