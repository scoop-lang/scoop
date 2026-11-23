# Glass studies

在窗口里滚动八行带有等高线纹理的色样。固定工具栏左侧的 Gaussian blur 将穿过它的细线模糊，右侧的 Refraction 在圆角附近弯折线条。两个 On／Off 按钮可独立切换效果，在同一滚动位置对比原始纹理。文字和按钮绘制在玻璃之上。

在已准备开发 sysroot 的仓库根目录运行：

```sh
export SCOOP_SYSROOT="$PWD/sysroot"
target/release/scoop run sysroot/src/scoop.ui.component/examples/glass --release
```

[色样](src/swatches.scoop)共享一份 SkSL 程序，各行绑定自己的颜色和相位，再把 Shader 传给 Canvas.drawRect。[工具栏](src/panel.scoop)使用 ImageFilter.blur 和 ImageFilter.runtimeShader。折射程序在局部逻辑坐标中偏移 background 输入，最大位移和声明的采样半径均为 13。StylePainter 依次绘制背景效果、半透明底色、标题和按钮。程序归组件作用域，依赖尺寸的滤镜随布局结果释放。

在图形会话中运行真实窗口测试：

```sh
target/release/scoop test sysroot/src/scoop.ui.component/examples/glass --release
```

[窗口测试](tests/window.scoop)在两种效果开启时只移动 ScrollState，读取实际提交帧中两个面板的固定位置。随后分别关闭效果，对照同一位置的背景，最后滚回起点并重新开启，检查纹理还原。像素读回只用于测试。

2026-11-23 的 release 验证覆盖 macOS arm64 Metal 窗口，并用原生滚轮事件复核了同一组像素断言。graphics 的 Raster／Metal 测试也在 Rosetta x86_64 上运行通过。
