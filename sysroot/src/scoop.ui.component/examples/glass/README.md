# Glass studies

在窗口里滚动八行带有等高线纹理的色样。固定工具栏左侧的 Gaussian blur 将穿过它的细线模糊，右侧的 Refraction 在圆角附近弯折线条。两个 On／Off 按钮可独立切换效果，在同一滚动位置对比原始纹理。底部 Glass opacity 按钮在 100% 与 50% 之间切换整个工具栏的透明度，包括背景效果、文字和按钮。

在已准备开发 sysroot 的仓库根目录运行：

```sh
export SCOOP_SYSROOT="$PWD/sysroot"
target/release/scoop run sysroot/src/scoop.ui.component/examples/glass --release
```

[色样](src/swatches.scoop)共享一份 SkSL 程序，各行绑定自己的颜色和相位，再把 Shader 传给 Canvas.drawRect。[工具栏](src/panel.scoop)使用 ImageFilter.blur 和 ImageFilter.runtimeShader。折射程序在局部逻辑坐标中偏移 background 输入，最大位移和声明的采样半径均为 13。StylePainter 依次绘制背景效果、半透明底色、标题和按钮。程序归组件作用域，依赖尺寸的滤镜随布局结果释放。

GlassOpacity 在 cachedScene 内调用 SceneContext.saveBackdropLayer，并呈现真正处理背景的 GlassPanel 子项。组继承当前背景，退出时按整体透明度在原背景与完整效果之间插值；缓存的组命令仍会读取滚动后的背景。示例的范围提示在面板外保留 24 个逻辑单位，覆盖当前 blur 的 3 倍 sigma 和折射采样半径；改动效果范围时需一起调整。调用方也可省略 bounds 让 graphics 按当前目标安排范围。API 的另一种用法是在 saveBackdropLayer 的 backdrop 参数中传入滤镜，在组进入时先处理背景，再绘制清晰前景。

在图形会话中运行真实窗口测试：

```sh
target/release/scoop test sysroot/src/scoop.ui.component/examples/glass --release
```

[窗口测试](tests/window.scoop)在两种效果开启时只移动 ScrollState，读取实际提交帧中两个面板的固定位置。随后分别关闭效果，对照同一位置的背景，再滚回起点并重新开启，检查纹理还原。新增步骤把整组透明度置为 50% 后再次滚动，用 0% 的纯背景和 100% 的完整效果核对中间值，最后检查静止时不继续提交帧。像素读回只用于测试。

2026-11-23 的 release 验证覆盖 macOS arm64 Metal 窗口，并用原生滚轮事件复核了同一组像素断言。graphics 的 Raster／Metal 测试也在 Rosetta x86_64 上运行通过。

2026-11-28 的 macOS arm64 release 验证通过新增的 10 阶段窗口测试；下层合成 API 的 Raster／Metal 像素和场景缓存验收见 [SkSL 实施说明](../../../../../docs/graphics/sksl/DESIGN.md#6-验证入口)。
