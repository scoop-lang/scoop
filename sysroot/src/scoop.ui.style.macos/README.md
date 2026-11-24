# scoop.ui.style.macos

普通自绘主题库，提供 `macosTheme(ThemeMode.Light)` 与 `macosTheme(ThemeMode.Dark)`，返回实现 Theme 的 scoop.ui.style.ShapeTheme。视觉基准固定为 **macOS Sequoia（15）的 Aqua／Dark Aqua**，不采用 Liquid Glass，也不随宿主系统升级改变轮廓。不依赖控件库，不创建输入区域、State 或任务，也不更改原生窗口装饰。

常规操作面高 22、圆角 5，采用 13／16 的系统风格文字与普通字重；辅助文字为 11／14。复选框与单选圆为 14，半选使用强调色底上的横线。普通与默认操作按钮保持 hover 原色，按下立即反馈，禁用时回到中性底色。3 宽焦点环贴合外缘，按钮只有轻微投影，浮动面使用柔和分层阴影，不使用强调色辉光或玻璃效果。滚动条在 15 宽条带内绘制中性圆头滑块，无端部按钮。

macOS 另提供 `macosSystemPalette(mode)`，在主线程读取指定明暗外观下的 AppKit 系统语义色快照，包含用户当前强调色。它返回普通 ThemePalette，可传给 `macosTheme(mode, Some(macosSystemPalette(mode)))` 或其他风格工厂。应用在需要刷新配色时重新读取并发布 ThemeSource；该函数不注册监听器。系统颜色与风格独立，使用当前系统配色也保留 Sequoia 的控件轮廓；省略 palette 时使用跨平台固定预设。

通过 Theme 查询表面、Checkbox／Radio 标记和 Scrollbar 的 metrics／geometry，再交给主题创建的 ThemePainter 绘制。离屏直接使用普通 Canvas；自绘 Element 通过 ComponentScope 持有 ThemePainterCache，换主题时替换并关闭旧 painter，应用无需逐组件 use。

`macosTheme(mode, palette = Some(colors))` 可独立覆盖配色来源，系统配色同样通过此参数传入；文字、表面、选择标记、边框和交互状态使用所给语义色，字体、尺寸、轮廓与动效仍属于当前风格。省略 palette 使用风格预设。

`macosTheme(mode) { ... }` 在所选模式的完整数据上配置 tokens、surfaces、checkbox、radio 和 scrollbar；`theme.copy { ... }` 从当前值继续修改。嵌套块保留未修改字段，完整赋值替换该字段；palette 的修改只修改该配色值，相关部件外观需要同时改变时显式配置。主题工厂返回不可变的普通描述。ThemeSource.current 是整体切换入口；主题及模式变化保留组件身份，实际使用者按新几何布局。公共语义颜色、字体、尺寸、圆角、效果与动画参数通过 tokens 提供。字体使用 family 回退，字体实际存在与 shaping 由 graphics.text 负责。

四类部件共同使用公共 ShapeTheme 算法，各预设显式声明自己的参数，控件不按风格分支。完整 Radio 分组、滚动、动画驱动及点击行为仍属于控件或模型。

用法及资源／效果合同见 [style README](../scoop.ui.style/README.md) 和[主题规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md)。四组主题的比较、离屏像素、字体、资源和 Metal／GLES 验证位于 [themes 测试 Cone](../scoop.ui.style/tests-cones/themes/Cone.toml)，历史实施记录见[实施记录](../../../docs/ui/PROGRESS.md)。Linux GNU 使用同一套主题描述及绘图代码。外观参考 Apple 的 [AppKit 外观说明](https://developer.apple.com/videos/play/wwdc2020/10104/)、[按钮](https://developer.apple.com/design/human-interface-guidelines/buttons)及[选择控件](https://developer.apple.com/design/human-interface-guidelines/toggles)。

## Linux GNU

使用已准备的 GNU sysroot，运行共享的主题测试：

```sh
scoop test sysroot/src/scoop.ui.style/tests-cones/themes --release --sysroot ~/.scoop
```

本主题可用于 Wayland 与 X11 的自绘窗口；名称表示视觉风格，不要求相应操作系统。依赖、测试范围及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。
