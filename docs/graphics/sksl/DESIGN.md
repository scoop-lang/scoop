# scoop.graphics 的 SkSL 与组件效果

2026-11-28。形状着色、子树滤镜与背景采样采用同一份 SkSL 程序和 Skia 公共接口。公开合同见 [graphics 效果规范第 5 节](../../specs/SCOOP-GRAPHICS-EFFECTS-SPEC.md#5-sksl-着色与背景采样)。[独立窗口示例](../../../sysroot/src/scoop.ui.component/examples/glass/README.md)展示高斯毛玻璃、圆角折射与整体透明度。

## 1. 程序与输入

`ShaderProgram.compile(source)` 将普通字符串交给 SkRuntimeEffect 编译并保留诊断。组件在挂载时编译一次，也可以借用应用持有的程序。每次 makeShader 或 runtimeShader 均在同步配置块内根据 Skia 反射绑定 uniform 和 shader 输入，结果保存独立快照。数值绑定不要求业务代码计算字节偏移。已有 Image 可调用 toShader 作为输入。程序、shader、滤镜均由创建线程管理，记录重放持有实际依赖。

## 2. 绘制入口

| 需要处理的像素 | 接入方式 |
| --- | --- |
| 形状覆盖区域 | 将 Shader 传给 Canvas.drawRect、drawPath 等形状绘制方法 |
| 已有图像 | 用 setChild 绑定 Image.toShader，在 SkSL 中用 eval 采样 |
| 一组子项 | 用 ImageFilter.runtimeShader 创建滤镜，在 saveLayer 与 restore 之间绘制子项 |
| 当前组件背后的内容 | 在 SceneContext.draw 中调用 Canvas.drawBackdrop，或向 StylePainter.scene 传入 backdrop，然后绘制前景 |
| 保留下层的一组组件 | 在 Canvas／SceneContext.saveBackdropLayer 中绘制，入口 backdrop 可先处理下层，opacity 作用于完整组 |

## 3. 生命周期与刷新

程序在同一 owner 上复用。小型参数变化只创建新的实例，已录制的命令保留原有参数。组件作用域持有长期资源，尺寸相关的资源也可以随 LayoutResult 释放。图像与滤镜持有的 GPU 输入沿用创建图像时的 context。

工具栏在滚动视口之后绘制。Skia 在同一帧、同一合成层读取此前的像素，在抗锯齿路径内替换滤镜结果。背景跨出轮廓的采样按滤镜的实际邻域申请。路径、滤镜和绘图命令可以跨帧复用，新的背景来自当前目标。含背景读取的绘图记录按整段命令重放，瓦片缓存跳过该段。

需要整体透明度时，外层使用 `withState { group -> group.saveBackdropLayer(opacity); ... }`，组内继续用 StylePainter 的 backdrop 和 children。新组继承当前下层 D，退出时把已含背景的结果 R 按预乘 RGBA 插值 `D * (1 - opacity) + R * opacity`；Skia 的 kInitWithPrevious／backdrop 初始化与 Arithmetic blender 完成此操作。没有 CPU 截图或逐帧 GPU 等待。普通 saveLayer 从透明开始，嵌套在其中的背景组只读取该隔离层。

只需统一处理组背景时，可直接传 `saveBackdropLayer(opacity, backdrop = Some(filter))`；滤镜在进入时执行，随后绘制的文字和子项保持清晰。没有入口滤镜的 UI 场景可缓存组命令及其值参数，重放时仍取当前背景。带借用滤镜的 scene 沿原资源规则直接执行。drawLayer 始终隔离内容；关闭瓦片缓存不改变这个边界。bounds、输出 clip 和采样范围见[下层合成规范 §2.1](../../specs/SCOOP-GRAPHICS-EFFECTS-SPEC.md#21-与下层共同合成)。

## 4. 动画示例与停帧

下面的 SkSL 在元素上绘制一道从左向右移动的光带。元素宽度为零时跳过效果绘制；size 使用元素逻辑尺寸，progress 取 0 到 1，tint 使用普通颜色输入。

```glsl
uniform float2 size;
uniform float progress;
layout(color) uniform half4 tint;

half4 main(float2 p) {
    float x = p.x / size.x;
    float center = -0.25 + 1.5 * progress;
    half strength = half(max(0.0, 1.0 - abs(x - center) / 0.2));

    half alpha = tint.a * strength;
    return half4(tint.rgb * alpha, alpha);
}
```

组件的使用流程为：

1. onMount 时取得或编译 effect，准备绑定，通过 scope.animation(0.0) 创建进度动画，并登记自有资源。
2. 点击等事件发生时，将进度按所选动画从 0 推进到 1；不在 paint 中启动或重启动画。
3. 每个实际帧由 Surface 先采样动画；Element.scene 的 draw 块使用当前尺寸、Animatable.value 与 Props 中的颜色更新 uniform，随后绘制效果。
4. 动画结束后呈现最终画面并撤销帧需求；组件卸载时按 scope 的正常顺序释放动画、绑定和自有缓存。

业务 State 仍在 Component.render 中读取并传入 Props。单纯的动画参数在 paint 中读取已经采样的 Animatable，不为每帧 uniform 更新再写 State，也不要求父组件逐帧重建描述。只改变 uniform 不重新编译 SkSL；程序或输入结构变化按实际需要更新相应缓存。

effect 本身没有时钟。声明 time uniform 不会自动提供时间或安排帧，包含正弦、噪声等函数也不会自发驱动绘制。持续效果由调用方显式启动动画或提出帧需求；暂停、取消、完成和卸载后撤销需求，隐藏状态沿宿主既有规则停止无效绘制。没有失效和活动动画时，保留 shader／uniform 缓存即可，不运行检查时间的 timer，不遍历组件树，也不持续提交 GPU 工作。

## 5. 坐标、颜色与图层边界

SkSL main 收到 shader 的局部坐标；没有额外 shader 局部矩阵时，可以对应 Element 的局部逻辑坐标。图像 shader 的采样位置按图像像素尺寸表达，不能直接当成 0 到 1 的 UV；尺寸适配与矩阵转换由调用方和绑定明确，DPI 根变换仍只施加一次。

颜色 uniform 使用 layout(color) 标记时，按 Skia 约定传入非预乘 sRGB 颜色并转换到绘制工作色彩空间；普通数值 uniform 不自动按颜色转换。shader 返回预乘 alpha，示例中先算 alpha 再乘 RGB；已经通过图像 shader 取得的预乘颜色不能重复乘 alpha。效果继续遵守当前 sRGB 输出合同，不因 SkSL 可以计算任意数值而自动扩展为 HDR 输出。

runtimeShader 通过 sourceChild 指定源输入，sampleRadius 用局部逻辑单位声明该输入在每个轴上的最大采样位移。子树使用 saveLayer，并在 restore 时滤镜合成。背景效果由 drawBackdrop 在当前层立即处理，外层隔离组决定可见的背后内容。

着色器改变像素，不自动改变组件布局、子项位置或 InputArea 命中。扫光、变色等效果通常沿用原几何；涉及位移或非线性扭曲时，要明确保留原输入区域，或由效果适配提供对应坐标映射。不能从任意 SkSL 程序自动推导可交互几何、caret 位置或逆变换；普通几何动画优先沿现有 Canvas／Surface 共享变换处理。

## 6. 验证入口

graphics 的 Cone 测试覆盖编译诊断、反射类型、参数快照、输入寿命、GPU context、分组滤镜及半透明背景重放。Glass studies 的窗口测试使用同一工具栏与真实滚动列表，对比两种滤镜单独开关时的帧，并在复位后核对像素。运行命令见[示例说明](../../../sysroot/src/scoop.ui.component/examples/glass/README.md)。

2026-11-28 的下层合成验证使用 macOS arm64、release：graphics 的 85 个测试全部通过，其中新增 5 组同时检查 Raster／Metal 的预乘插值、半透明与圆角空组、嵌套和异常恢复、邻域采样、清晰前景、2 倍 DPI 及关闭源资源后的记录重放。真实玻璃窗口通过 10 阶段滚动／透明度／停帧对照；UI scene-cache 测试同时通过，直接检查无入口滤镜的组复用场景计数、重放时的背景变化，以及借用滤镜时重新执行 scene。Scoop 格式与 check、C++ 告警检查、受影响 API 文档生成及 saveBackdropLayer 查询均通过。本次新增能力尚未在其他目标实跑。

同日新增 `Canvas.drawBackdrop(bounds, filter, mask)`，以 shader alpha 表达连续变化的背景轮廓，遮罩仅影响这次绘制。macOS arm64 release 的 graphics 全部 88 个测试通过；新增 Raster／Metal 对照覆盖 0／50／100% 覆盖率、半透明颜色、祖先裁剪、2 倍变换、输出边界外采样、关闭资源后的录制重放，以及非法边界、已关闭遮罩和不相容 GPU 输入的错误收尾。C++ 格式／告警检查、Scoop 格式／check、无警告文档生成与两个 drawBackdrop 重载的 API 检索通过。其他目标未实跑。

## 7. 调研依据

- Skia 官方：[SkSL 与 Runtime Effects](https://skia.org/docs/user/sksl/)，包括输入 shader、局部坐标、颜色 uniform 与预乘 alpha。
- 固定版本源码：[SkRuntimeEffect.h](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/include/effects/SkRuntimeEffect.h)，包含编译结果、uniform／child 反射与 shader／color filter／blender 创建入口。
- 固定版本源码：[SkImageFilters.h](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/include/effects/SkImageFilters.h)，包含 RuntimeShader 图像滤镜、输入绑定与采样范围约定。
- 本仓库：[绘图绑定](../../../sysroot/src/scoop.graphics/native/shared/drawing.cpp)、[Skia 依赖说明](../../../sysroot/vendor/skia/README.md)、[UI 公共自绘协议](../../specs/SCOOP-UI-SPEC.md#7-自绘布局输入与动画)。
