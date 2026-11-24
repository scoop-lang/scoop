# scoop.ui.style.material

普通自绘主题库，提供 `materialTheme(ThemeMode.Light)` 与 `materialTheme(ThemeMode.Dark)`，返回实现 Theme 的 scoop.ui.style.ShapeTheme。以 Google Material Web 的 Material Design 3 为基准，使用完整的浅／深色语义角色、Roboto 字体回退、40 高胶囊按钮、18 复选框、20 单选框及外围 40 大小的圆形交互状态层。不依赖控件库，不创建输入区域、State 或任务，也不更改原生窗口装饰。

填充按钮默认无阴影，悬停使用 8% on-primary 状态色与轻微提升阴影，按压使用 12% 状态色；描边按钮内部透明，内容左右各留 24。禁用按钮的底色为 12% on-surface，文字为 38% on-surface，不再次降低整体透明度。复选框／单选框的悬停和按压反馈在圆形状态层上绘制，不把未选中的内部染成按钮底色；选中 Checkbox 使用 primary 底色及 on-primary 勾／横线，Radio 使用 primary 圆环与圆点。禁用标记使用中性色，取消状态层；焦点使用带 2 间隔的 3 宽外环。默认不使用辉光。

输入表面 Plain 与描边操作面 OutlinedAction 分别配置：Input／Textarea 使用透明底色、固定 4 圆角、on-surface 正文和贴边的 2 宽焦点描边；多行高度增加不会变为胶囊轮廓。`tokens.textField = TextFieldTreatment.Outlined` 让真实 Input／Textarea 的 label 随焦点或内容浮到上边框缺口；标签两侧各留 4，支持 RTL。输入正文使用 16／24，浮动标签与辅助文字使用 12／16；单行容器默认高 56。普通错误描边为 1 宽，聚焦后为 2 宽。

通过 Theme 查询表面、Checkbox／Radio 标记和 Scrollbar 的 metrics／geometry，再交给主题创建的 ThemePainter 绘制。离屏直接使用普通 Canvas；自绘 Element 通过 ComponentScope 持有 ThemePainterCache，换主题时替换并关闭旧 painter，应用无需逐组件 use。

`materialTheme(mode, palette = Some(colors))` 可独立覆盖配色来源，系统配色同样通过此参数传入；文字、表面、选择标记、边框和交互状态使用所给语义色，字体、尺寸、轮廓与动效仍属于当前风格。省略 palette 使用风格预设。

`materialTheme(mode) { ... }` 在所选模式的完整数据上配置 tokens、surfaces、checkbox、radio 和 scrollbar；`theme.copy { ... }` 从当前值继续修改。嵌套块保留未修改字段，完整赋值替换该字段；palette 的修改只修改该配色值，相关部件外观需要同时改变时显式配置。主题工厂返回不可变的普通描述。ThemeSource.current 是整体切换入口；主题及模式变化保留组件身份，实际使用者按新几何布局。公共语义颜色、字体、尺寸、圆角、效果与动画参数通过 tokens 提供。字体使用 family 回退，字体实际存在与 shaping 由 graphics.text 负责。

四类部件共同使用公共 ShapeTheme 算法，各预设显式声明自己的参数，控件不按风格分支。完整 Radio 分组、滚动、动画驱动及点击行为仍属于控件或模型。

`surfaces { action { hoverShadows = Some(...) } }` 可覆盖悬停投影，None 继承普通投影；`checkbox { appearance { stateLayer = Some(MarkerStateLayer(...)) } }` 与 radio 的同名字段配置标记圆形状态层。绘图只消费现有 hover／press／selection 采样，不新增径向扩散动画或输入处理。减少动态继续由 ThemeMotion 处理；高对比度／减少透明度隐藏默认状态层和投影，保留焦点轮廓。滚动条使用桌面条带尺寸及中性语义色，Material Web 本身没有提供可逐项复刻的滚动条组件。

参考：[Text fields 规格](https://m3.material.io/components/text-fields/specs)、[标签与多行输入规则](https://m3.material.io/components/text-fields/guidelines)、[Material Web M3 tokens](https://github.com/material-components/material-web/tree/main/tokens/versions/v0_192)、[Checkbox](https://github.com/material-components/material-web/blob/main/checkbox/internal/_checkbox.scss)、[Radio](https://github.com/material-components/material-web/blob/main/radio/internal/_radio.scss)。字体、尺寸、状态透明度和配色由预设固定声明，系统配色仍独立传入；阴影复用本库 Gaussian 投影，不依赖 Web/CSS 运行时。

用法及资源／效果合同见 [style README](../scoop.ui.style/README.md) 和[主题规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md)。四组主题的比较、离屏像素、字体、资源和 Metal／GLES 验证位于 [themes 测试 Cone](../scoop.ui.style/tests-cones/themes/Cone.toml)，基础交付的历史结果见[实施记录](../../../docs/ui/PROGRESS.md)。Linux GNU 使用同一套主题描述及绘图代码。

## Linux GNU

使用已准备的 GNU sysroot，运行共享的主题测试：

```sh
scoop test sysroot/src/scoop.ui.style/tests-cones/themes --release --sysroot ~/.scoop
```

本主题可用于 Wayland 与 X11 的自绘窗口；名称表示视觉风格，不要求相应操作系统。依赖、测试范围及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
