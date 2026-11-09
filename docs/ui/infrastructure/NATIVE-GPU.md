# 原生 GPU 图像接入

API 位于 `scoop.graphics.native`，完整合同见 [GPU buffer 规范](../../specs/SCOOP-GRAPHICS-GPU-SPEC.md)。现有 UI 继续通过 Canvas 绘制 Image；设备和原生资源管理不进入 Element、布局或输入 API。

## 选择路径

| 场景 | 接入方式 | 存储与生命周期 |
| --- | --- | --- |
| 3D、计算或自定义绘制输出 | `GpuDevice.createBuffer` + `GpuBuffer.withNativeWrite` | 本库分配 GPU 存储；正常和异常出口均建立队列完成通知 |
| 解码器、相机或已有原生纹理 | `GpuDevice.importImage` | 借用生产者的不可变存储；原生 owner 管理保活和回收 |
| 仅使用 Canvas 绘制 | 原有 `Canvas.createSurface` | 沿用 OffscreenSurface，无需取得原生句柄 |
| CPU 导出、截图 | `GpuBuffer.readPixels` | 显式等待并导出直通 alpha 的 sRGB RGBA8 |

在有效 GPU Canvas 上调用 `gpuDevice()`，把设备包装注册到使用者既有的 Closeable 资源作用域中。只需创建一次，可以跨帧复用；Raster 返回 None。设备、buffer、Image、Canvas 继续限所属线程使用。不要每帧重复建立 GPU context。

## 可写输出

创建时明确选择目标平台的原生格式，例如 Metal 的 `MTLPixelFormatRGBA16Float` 为 115，D3D12 的 `DXGI_FORMAT_R16G16B16A16_FLOAT` 为 10。`GpuPixelLayout.Rgba16F` 说明通道解释，是否可用由当前 backend 决定。不存在自动挑选通用格式、转成 RGBA8 或切换 Raster 的后备行为。

同一 buffer 可以交替使用 Canvas 和原生 API。下面的函数接收应用实现的原生编码器，函数本身不创建新的资源协议：

```scoop
import scoop.graphics.*
import scoop.graphics.native.*

@Unsafe
fun drawNativeOutput(
    canvas: Canvas,
    buffer: GpuBuffer,
    destination: Rect,
    encode: (NativeGpuDevice, NativeGpuTexture) -> Unit
) {
    buffer.withNativeWrite(encode)
    buffer.snapshot().use { image -> canvas.drawImage(image, destination) }
}
```

`encode` 分支使用原生 API：Metal 给出 device、command queue 与 MTLTexture；D3D12 给出 adapter、device、direct queue、ID3D12Resource 和当前 resource state；GLES 给出已激活的 EGL display/context 和二维 texture name。生产者必须在回调返回前提交到这条队列。D3D12 原生写入自行执行 barrier，并恢复传入的 state。

进入借用会提交之前的 Skia 工作；退出重置后端状态跟踪。旧 snapshot 仍被 Image、Drawing 或 GPU 使用时，下一次写入换用同格式存储。原生 texture 地址可能改变，每次都使用本次借用给出的值。

借用的 device 原值不能直接当作长期拥有的引用。原生渲染器可以在借用内按平台规则 retain／AddRef 设备、创建管线和自己的内部资源；其后访问本库 buffer 仍须通过 `withNativeWrite`。外部独立队列须用平台 fence/event 与共享队列建立依赖，库不猜测跨队列顺序。

## 外部帧

`importImage` 接收 `NativeGpuTexture`、像素尺寸、原生格式、通道解释，以及 `NativeGpuImageOwner`。原生 owner 的 C ABI 为：

```c
struct NativeGpuImageOwner {
    void *payload;
    void (*retain)(void *payload);
    void (*release)(void *payload, uint32_t final_state);
};
```

这是原生代码的回调，不是 Scoop 闭包或 ForeignCallback。`payload` 通常指向解码帧或原生帧池的一项，不能指向移动 GC 对象。接口通过 `GCLeafFunPtr` 接收这些原生函数指针；来自 C/C++ 库的 Extern 返回值是适当来源。

一次进入 native 的导入取得一份引用。失败配对释放；成功后，最外层 Image 关闭时可能仍有 Drawing 或 GPU 使用，须等待 release 再修改或回收对应像素。D3D12 的 `final_state` 为后续 producer barrier 所需状态；其他平台为零。release 只表示消费方已结束，原生 owner 仍负责自己生产任务的引用与完成顺序。

回调可能发生在清理线程，应只执行短时原生释放或原生池归还，不调用 Scoop/GC/UI、不抛异常。需要特殊释放线程的原生库自行安排其清理。设备关闭和显式 `drain` 会等待实际队列；正常每帧路径无需 CPU 等待。

原生多平面格式按可采样平面导入，例如 R8、RG8；库不负责 YUV 转换。生产者可以先在自己的 GPU 管线完成转换，再发布可合成纹理。alpha 的预乘／直通／不透明解释、图像原点和颜色空间由导入参数明确描述。

暂停视频或静止 3D 内容时，保留最后一张 Image，停止原生生产和 UI 帧需求。仅显示未变化图像不需要重新导入、上传、读回或开启持续呈现。

## 验证入口

graphics Cone 的 `tests/gpu.scoop` 在三个 backend 共用，验证原生绘制、写时复制、记录保活、帧池再次发布、原生通道、跨 context、线程和关闭语义。平台测试补充真实目标与平台特有条件；Metal 额外检查延迟原生队列与 sRGB 格式。`test-native` 中的生产者只用于这些真实 GPU 验证，不是应用必须依赖的封装。
