# scoop.ui.style.fluent

普通自绘主题库，提供 `fluentTheme(ThemeMode.Light)` 与 `fluentTheme(ThemeMode.Dark)`，返回实现 Theme 的 scoop.ui.style.ShapeTheme。以 [Fluent 2 的官方 token](https://github.com/microsoft/fluentui/tree/master/packages/tokens/src) 和 [Fluent UI React 的控件样式](https://github.com/microsoft/fluentui/tree/master/packages/react-components) 为四类公共绘图部件的视觉对照，不依赖控件库，不创建输入区域、State 或任务，也不更改原生窗口装饰。

正文和操作文字为 14／20，辅助文字为 12／16，操作字重 600，优先使用 Segoe UI 并提供通用字体回退。操作面高 32、圆角 4，使用独立的 normal／hover／pressed／disabled 配色；明暗模式的默认品牌背景分别为 #0f6cbd／#115ea3，均配白色文字。普通控件保持平面，浮动面使用 Fluent shadow16 的两层阴影，CSS blur 已转换为 Gaussian sigma；不添加强调辉光。

Checkbox 和 Radio 为 16 大小，使用各自的外观：勾选框填充 compound brand，半选框保留透明底并绘制品牌色方形，单选框保留空心圆环和品牌色圆点。深色选择标记使用较亮的 compound brand 色，和操作面的深蓝区分。禁用标记、文字、边框使用独立中性色。焦点边框宽 2；滚动条为中性细滑块，hover／pressed 不变成品牌蓝，默认不带端部按钮，仍可通过 ScrollbarOptions.buttonLength 显式添加。

hover／press 使用 Fluent 的 100ms 与 curveEasyEase 曲线；Checkbox／Radio 的选择标记即时更新。减少动态沿公共 ThemeMotion 偏好生效。

通过 Theme 查询表面、Checkbox／Radio 标记和 Scrollbar 的 metrics／geometry，再交给主题创建的 ThemePainter 绘制。离屏直接使用普通 Canvas；自绘 Element 通过 ComponentScope 持有 ThemePainterCache，换主题时替换并关闭旧 painter，应用无需逐组件 use。

`fluentTheme(mode, palette = Some(colors))` 可独立覆盖配色来源，系统配色同样通过此参数传入；文字、表面、选择标记、边框和交互状态使用所给语义色，字体、尺寸、轮廓与动效仍属于当前风格。省略 palette 使用风格预设。

`fluentTheme(mode) { ... }` 在所选模式的完整数据上配置 tokens、surfaces、checkbox、radio 和 scrollbar；`theme.copy { ... }` 从当前值继续修改。嵌套块保留未修改字段，完整赋值替换该字段；palette 的修改只修改该配色值，相关部件外观需要同时改变时显式配置。主题工厂返回不可变的普通描述。ThemeSource.current 是整体切换入口；主题及模式变化保留组件身份，实际使用者按新几何布局。公共语义颜色、字体、尺寸、圆角、效果与动画参数通过 tokens 提供。字体使用 family 回退，字体实际存在与 shaping 由 graphics.text 负责。

四类部件共同使用公共 ShapeTheme 算法，各预设显式声明自己的参数，控件不按风格分支。完整 Radio 分组、滚动、动画驱动及点击行为仍属于控件或模型。

用法及资源／效果合同见 [style README](../scoop.ui.style/README.md) 和[主题规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md)。三组主题的比较、离屏像素、字体、资源和 Metal／GLES 验证位于 [themes 测试 Cone](../scoop.ui.style/tests-cones/themes/Cone.toml)，当前结果见[实施记录](../../../docs/ui/PROGRESS.md)。Linux GNU 使用同一套主题描述及绘图代码。

[组件 Gallery](../scoop.ui.component/examples/shadcn/README.md) 的右上角 Style 下拉框可直接选择 Fluent，并组合 Light／Dark、RTL 和减少动态查看同一组控件。

## Linux GNU

使用已准备的 GNU sysroot，运行共享的主题测试：

```sh
scoop test sysroot/src/scoop.ui.style/tests-cones/themes --release --sysroot ~/.scoop
```

本主题可用于 Wayland 与 X11 的自绘窗口；名称表示视觉风格，不要求相应操作系统。依赖、测试范围及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
