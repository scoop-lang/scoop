# Scoop 原生 GPU buffer 接入

本规范扩展 [Surface 的 graphics 资源合同](SCOOP-SURFACE-SPEC.md#7-原生资源与错误)。接口属于普通 `scoop.graphics.native`，不改变 UI 的 Element、布局、输入或帧调度 API。这里管理用于绘图与合成的二维 GPU 像素 buffer，适用于外部渲染器、解码帧、相机、计算输出及离屏处理；原生生产者仍使用所属设备的 API 管理其内部顶点、命令与 staging 资源。

## 1. 设备与格式

有效的 `Canvas.gpuDevice(): GpuDevice?` 在 GPU 画布返回独立持有的设备包装，Raster 返回 None。调用者负责 close，可在后续帧复用；该对象与来源画布使用同一实际 context、线程和原生队列，不延长 Canvas 借用。关闭此包装不关闭来源窗口或仍持有资源的其他包装。

`GpuDevice` 提供 backend、createBuffer、importImage、withNative、drain、isClosed 和 close。Metal、D3D12、GLES 是实际 backend，不做 renderer 自动切换。原生设备借用分别提供 Metal device／command queue、D3D12 adapter／device／direct queue，以及 EGL display／context。原生生产者可在借用中按平台 retain／AddRef 规则取得自己的设备引用、创建长期持有的渲染器或解码器；EGL context 的生命周期仍由来源 context 管理。独立原生对象的线程和队列使用遵循所属平台，访问本库可写 buffer 仍须进入对应借用并满足同队列顺序。

`NativeGpuFormat` 分别携带 Metal 的 MTLPixelFormat、D3D12 的 DXGI_FORMAT、GLES 的 sized internal format 原值。创建时还明确 `GpuPixelLayout`，描述 Skia 如何解释该纹理的通道；这不是格式转换。支持范围由实际设备和后端决定，格式、通道解释、尺寸或可渲染性不相容时失败，不偷偷选择另一格式、软件目标或读回路径。当前通道解释为 RGBA8、BGRA8、RGBA16F、RGB10A2、R8 和 RG8。图像颜色空间显式选择 sRGB 或线性 sRGB；这不扩展窗口的 HDR 呈现合同。

## 2. 可写 buffer

`GpuDevice.createBuffer(width, height, format, layout, colorSpace = Srgb, scale = 1.0): GpuBuffer` 分配同一 context 的可渲染、可采样、单层、单采样二维纹理，初始通道清零；含 alpha 的格式为透明黑，无 alpha 的格式保持其原生不透明解释。GPU 资源由 Skia 管理；native format 保留，尺寸不为 RGBA8 人为换算或限制。scale 只影响 Scoop draw 的根坐标。

`GpuBuffer` 提供像素尺寸、format、layout、colorSpace、scale、draw、snapshot、readPixels、withNativeWrite、isClosed 与 close。draw 和 Canvas 借用沿 RenderTarget，snapshot 返回不可变 GPU Image，后续 draw 或原生写入不改变旧图像。仍有快照引用或 GPU 使用时，下一次写入通过同格式 GPU 存储写时复制；不能让 Skia 的默认格式选择改变原生格式。普通快照与提交不进行 CPU 读回。readPixels 才是显式同步与 RGBA8 导出。

`@Unsafe withNativeWrite(block)` 同步借出设备与当前实际纹理。进入前通知 Skia 内容将改变，保留已交付快照，flush 并异步提交已有 GPU 工作；原生生产者必须在回调返回前把工作提交到借出的同一队列。GLES 使用已激活的同一 context。返回后重置 Skia 的原生状态跟踪，并以队列完成通知保留实际纹理到原生工作结束；即使之后不再绘制，也不依赖下一帧释放或保护资源。D3D12 借用包含当前 resource state，生产者自行建立 barrier，并在正常和异常出口恢复这个状态。借用期间拒绝同一 buffer 的 draw、snapshot、读回、再次借用或关闭。

句柄只在回调内有效；下一次借用可能因写时复制取得另一纹理，不允许缓存旧地址。回调不能保存 Canvas、挂起、跨线程或与同一 context 的 Scoop 绘图重入。原生编码已经产生的像素不回滚；异常仍结束借用并恢复后端状态跟踪。外部队列的依赖必须由生产者通过该平台原生 fence／event 在共享队列上排好，不增加统一 fence 数值语义或 CPU 等待来模拟跨平台对齐。

## 3. 导入外部图像

`@Unsafe GpuDevice.importImage(texture, width, height, format, layout, owner, origin = TopLeft, colorSpace = Srgb, alpha = Premultiplied): Image` 以不可变的借用纹理接入 Canvas.drawImage、图像区域绘制、绘图记录和 UI。它不要求外部纹理可渲染，不做像素复制、格式转换或 CPU 读回；生产者在导入前完成同队列提交或原生队列依赖。alpha 描述原生像素的预乘、直通或不透明解释，不修改原生存储。多平面图像按原生可采样平面分别导入，YUV 等颜色转换归原生生产者，不把某个平台的格式能力强加给其他平台。

原生纹理描述分别携带 MTLTexture、ID3D12Resource 与初始 resource state、或当前 GLES context 的 texture name。调用方保证它们与声明的尺寸、格式、设备和线程一致。库执行实际需要的格式／设备兼容检查；不能验证的原生句柄有效性仍属于 Unsafe 责任。

`NativeGpuImageOwner` 是 C ABI 值，包含原生 payload、retain 和 release 函数。两函数必须来自纯 native 代码，不能是托管 closure、ForeignCallback trampoline 或 Scoop 静态 callback。导入尝试取得一份引用，失败也配对释放；Image、Drawing 和已提交 GPU 使用共同保留引用，直到 Skia 确认可以释放纹理。关闭最外层 Image 不构成 GPU 已结束使用的通知。

release 接收 payload 与该纹理最终的原生状态；D3D12 为最后的 resource state，其他后端为零。原生池可据此归还或再次写入 buffer。回调可能发生在原生资源清理线程，必须短时、不阻塞、不抛异常、不调用 Scoop／GC／UI；需要线程归属的清理由原生 owner 自己安排。payload 不得是移动 GC 对象裸地址。原生生产者在 release 之前不得修改、回收或重复发布同一存储为其他帧。

`GpuDevice.withNative` 为创建、上传及导入生产者提供同一设备／队列的同步 Unsafe 借用。进入前异步提交 Skia 工作，退出时重置后端状态跟踪；不授予改写已发布 Image 的权限。设备及 buffer 操作仍限 owner 线程。显式 drain／设备关闭排空实际 GPU 使用；普通 buffer close 只释放持有关系，已提交使用由后端完成。

## 4. 验收

所属 graphics Cone 使用真实 Metal／GLES／D3D12 验证：同设备分配、实际原生绘制与 Canvas 合成、快照后再次原生写入、跨 context 拒绝、原生 owner 的成功／失败配对和延迟释放、记录保活、异常／重入／跨线程与关闭，以及平台支持的额外 native 格式。普通 UI 沿既有 Image 绘图消费资源；没有新帧时不采集、上传、提交或呈现。视频暂停和静止的 3D 组件仍须主动停止外部生产及帧需求。
