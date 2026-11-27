# scoop.ui.style

四套风格工厂均支持独立的 `palette: ThemePalette?` 输入：None 使用风格预设，提供语义配色则同时生成对应的文字和各控件状态颜色，并保留该风格的几何与动效。系统配色与风格选择可以分别存储，再通过同一 ThemeSource 发布组合结果。

普通 Scoop 库，提供公共主题绘图协议、几何、语义参数，以及可复用的装饰与内容效果。`scoop.ui` 不依赖本库，基础控件和第三方自绘使用同一套公开接入。macOS 验证见[实施记录](../../../docs/ui/PROGRESS.md)，场景协议迁移见[基础交互实施记录](../../../docs/ui/foundations/PROGRESS.md)，Linux GNU 验证见[Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。

CheckboxPart 可通过 `mixedAppearance { ... }` 独立配置半选外观，`mixedAppearance = None` 恢复继承普通 appearance；`mixedShape = MixedMarkShape.Square` 绘制方形半选标记，默认 Bar 保持横线。普通／半选外观共用几何、局部覆盖和主题偏好解析，焦点留白取两者最大值。Fluent 2 预设使用透明底色的方形半选及独立 Radio 配色；可在 [组件 Gallery](../scoop.ui.component/examples/shadcn/README.md) 的 Style 下拉框中与其余主题比较。

```toml
[dependencies]
"scoop:scoop.ui.style" = "0.1.0"
```

通过 Props 更新普通描述，应用不关闭 Style，也不为每个装饰或组件写 use：

```scoop
styled(content) {
    style {
        background = Some(Paint(Color.White))
        border = Some(Border.all(1.0, Color(0.7, 0.7, 0.7)))
        corners = Corners.all(8.0)
        boxShadows = [BoxShadow(Point(0.0, 3.0), 4.0, 4.0, Color.Black.withAlpha(0.2))]
        contentEffects = [ContentEffect.Glow(2.0, 2.0, Color(0.0, 0.4, 1.0, 0.5))]
    }
}
```

styled 始终保留包装，包括空 Style；增加或移除效果不重建内容的实例和 State。它原样传递子项的约束、尺寸与基线，不创建 InputArea 或增加隐式 padding。点击范围来自内容本身；`clipContent = true` 使用同一圆角轮廓裁剪最终内容输出和子输入。

顺序是外框阴影、背景、经过有序效果的内容、向内的边框。BoxShadow 使用填满的圆角外框；DropShadow 使用真实内容 alpha；Glow 是零 offset 的着色 alpha 模糊与原内容 SrcOver 合成。`[Blur(...), DropShadow(...)]` 与相反顺序不同。文字直接使用实际绘制结果，透明图像和子组件采用同一路径；不需要 glyph 转 Path。

Border.Physical 接收四边 BorderEdges，Border.Logical 使用 start／top／end／bottom 的 BorderEdge；Corners.Physical 接收 graphics.CornerRadii，Corners.Logical 明确四个逻辑角。scene 按当前 LayoutDirection 解析，再统一计算外轮廓与各边内缩；半透明接合、背景、内容 clip、shadow 和主题 focus 共用轮廓。独立离屏 paint 可传入 direction，Surface／Checkbox 的 metrics 和 geometry 也接收同一方向。

sigma 是 Gaussian 标准差，sigma 和 offset 使用局部逻辑单位。根 DPI 只施加一次，图层没有按布局框添加隐式 crop，因此向外的光晕和阴影可以显示。祖先 clip 和实际目标仍限制可见范围。`StylePainter.scene(context, style, Some(filter)) { local -> local.children() }` 在样式圆角内处理已绘制的背景，随后填充、绘制前景并描边。filter 可以是 ImageFilter.blur 或 runtimeShader，由调用者持有。[工具栏示例](../scoop.ui.component/examples/glass/README.md)

完全自绘 Element 在 onMount 中保存 `scope.resource { StylePainter() }`，在 scene 中调用 `painter.scene(context, style) { content -> ... }`；回调使用 SceneContext，可调用 children 或提交 draw 块。也提供显式 Rect，以及独立的 paint(Canvas, Rect, Style, direction = LayoutDirection.Ltr, content) 入口，供离屏使用。原生滤镜／轮廓由 painter 缓存并及时替换关闭，组件卸载或窗口关闭由 scope 清理。回调内不能重入或提前关闭同一 painter，另一个 painter 可以正常嵌套。

玻璃可直接用资源无关的材质描述，多个组件共享同一个值，由各自的 StylePainter 缓存当前滤镜：

```scoop
val glass = glassMaterial {
    blurSigma = 8.0
    refraction = 8.0
    edgeWidth = 16.0
    saturation = 1.1
    tint = Color.White.withAlpha(0.16)
    highlight = Color.White.withAlpha(0.36)
}
val panel = style {
    this.glass = Some(glass)
    corners = Corners.all(20.0)
}
val frost = glass.copy { refraction = 0.0 }
```

材质先模糊当前下层，再沿圆角边缘折射、调整饱和度、染色并加反光，内容保持清晰。长度使用局部逻辑单位；`refraction` 也是声明的每轴最大采样位移，`edgeWidth` 控制向内的光学边缘宽度。颜色遵守预乘 alpha 合成，反光保留背景 alpha。`styleOverride { glass(None) }` 显式清除继承材质；空 override 继续继承。Style.glass 与显式 backdrop 滤镜不能同时提供。描述本身不需要 close，不持有背景像素；静止材质不会请求后续帧。[玻璃示例](../scoop.ui.component/examples/glass/README.md)展示独立模糊、折射与整组透明度。

主题可以按用途选择玻璃，各项默认关闭：

```scoop
val theme = materialTheme {
    tokens {
        glassSurfaces {
            toolbar = Some(glass)
            popover = Some(frost)
            dialog = Some(glass)
        }
    }
}
```

`SurfaceRole` 的 Content／Control／Input／Navigation／Toolbar／Popover／Dialog／Notification 与 SurfaceKind 的颜色和形状分类独立。`SurfaceVisual(..., role = SurfaceRole.Toolbar)` 把用途传给主题，第三方组件可用 `theme.tokens.glassSurfaces.get(role)` 读取相同策略。GlassSurfaces 支持 glassSurfaces 工厂、copy 与嵌套配置；不同用途之间不自动继承。主题先解析局部 glass 覆盖再生成默认填充，`glass(None)` 同时恢复原默认底色；显式 background 覆盖仍最后生效。高对比度与减少透明度偏好关闭主题材质，恢复偏好后从基础主题恢复原策略。

`GlassEffectPainter.scene(context, outlines, material, blendRadius)` 把多个物理 RoundedOutline 绘成连续的共享材质；paint 提供相同 Canvas 入口。blendRadius 默认为 24，0 保留普通并集；表面靠近时形成平滑连接，重叠部分只处理一次下层。遮罩、折射和高光沿同一融合轮廓计算，子内容随后清晰绘制。传入 Color 替代 material 的重载以相同形状绘制实体底色，适用于减少透明度。painter 是需注册到 scope 的 Closeable，只保留当前形状和光学资源，不改变输入几何。组件层的 [GlassEffectContainer](../scoop.ui.component/README.md) 同时处理 keyed 内容、形变动画、布局、裁剪与主题偏好。

`styleOverride { ... }` 只替换明确指定的属性：StyleValue.Inherit 继承本轮输入，StyleValue.Set(None) 可以清除可空背景或边框；`border(None)` 等同名设置方法把同一槽写为 Set(value)。赋入 StyleValue.Inherit 恢复继承。`existing.copy { ... }` 保留未改的配置，发布后修改 builder 不改变结果。字体、段落和混排仍使用 graphics.text 的 TextStyle／SpanStyle／ParagraphStyle。

没有独立效果时钟，也没有子树像素缓存。真实帧每次绘制当帧内容，仅复用当前滤镜与几何。参数动画在 Element 的 update／输入中设置 Surface Animatable，在 paint 中读取采样值；完成、暂停、取消或隐藏后按 Surface 规则停帧。

Theme 提供普通 tokens 与各部件的 metrics／geometry，ThemePainter 消费同一份几何和视觉数据。基础能力包括表面、Checkbox 三态标记、Radio 标记，以及带轨道／滑块／可选端部按钮的 Scrollbar；这些入口只需要 Canvas，不依赖窗口、输入区域或挂载。四套风格是独立普通 Cone：

| Cone | 工厂 | 参数 |
| --- | --- | --- |
| scoop.ui.style.macos | macosTheme | mode，ShapeThemeBuilder 配置块 |
| scoop.ui.style.material | materialTheme | mode，ShapeThemeBuilder 配置块 |
| scoop.ui.style.fluent | fluentTheme | mode，ShapeThemeBuilder 配置块 |
| scoop.ui.style.shadcn | shadcnTheme | mode，ShapeThemeBuilder 配置块 |

预设返回实现 Theme 的 ShapeTheme，支持接收者配置与 copy。配置块在完整数据上同步执行一次，嵌套块保留当前字段，完整赋值替换：

```scoop
val theme = macosTheme(ThemeMode.Dark) {
    tokens {
        typography { body { fontSize = 15.0 } }
        motion { hover = None }
    }
    surfaces { action { shadows = [] } }
}
val changed = theme.copy {
    checkbox { appearance { mark = StateColors.solid(Color.White) } }
}
```

独立主题用 shapeTheme(tokens, surfaces, checkbox, radio, scrollbar) 提供完整必需数据；themeTokens、themeTypography、themeEffects、themeMotion 以及 surfacePart／surfaceParts、markerAppearance、checkboxPart／radioPart、scrollbarPart 提供各组构造与 copy。surfaceParts 必需提供 plain、action、outlinedAction、elevated，输入表面与描边操作面可分别配置；对应 SurfaceKind.Plain／Action／OutlinedAction／Elevated。ThemePalette 的完整颜色对、input／ring、ChartPalette 和 SidebarPalette 没有通用默认值，完整配色一次给齐，后续通过 palette 块或 copy 修改。编辑配色、字体或某个部件只修改该字段，需要同步改变多个外观时显式配置它们。新增 [shadcn](../scoop.ui.style.shadcn/README.md) 预设沿用相同入口；显式减少动态通过 `motion { preference = MotionPreference.Reduced }` 配置。

第三方 Element 在 onMount 中保存 `scope.resource { ThemePainterCache() }`，在 paint 中使用当前 Props 的 Theme：

```scoop
val visual = CheckboxVisual(CheckMark.Mixed, InteractionVisual(focusVisible = true))
val geometry = theme.checkboxGeometry(bounds, visual)
painters.get(theme).drawCheckbox(context.canvas, geometry, visual)
```

metrics 可查询建议／最小尺寸、padding、边框内缩及焦点外扩；geometry 使用调用者最终 bounds，返回实际外框、内容区、圆形、标记线段或滑块行程等类型化描述。Options 中显式给出的尺寸与样式在计算前解析。SurfaceGeometry.style 保存本次外观，contentStyle 只保留内容效果和 clip；在 drawSurface 的 Background 和 Foreground 阶段之间，用自己的 StylePainter 应用到实际内容。Radio 始终保持圆形，局部参数可覆盖尺寸、环宽、圆点内缩及颜色；完整 Radio 分组与滚动行为由控件／模型负责。

部件选项通过 surfaceOptions／checkboxOptions／radioOptions／scrollbarOptions 配置，copy 基于已有值修改。Surface 和 Checkbox 的 style 成员块会保留之前设置的覆盖，空块不会重置；直接赋入完整 style 则替换：

```scoop
val options = checkboxOptions {
    size = Some(20.0)
    style { corners(Corners.all(4.0)) }
    style { border(None) }
}
val changed = options.copy { markColor = Some(Color.White) }
val geometry = theme.checkboxGeometry(bounds, options = changed)
```

`theme.tokens` 提供 palette、typography、spacing、corners、effects 和 motion。文字覆盖用 SpanStyle.resolve；外框与内容效果继续使用本库 Style。Theme 不持有 native 资源；ThemePainterCache 保留当前 painter，换主题时先创建再替换，及时关闭旧资源。绘图中的临时 Canvas 不能保存，颜色和动画采样不会创建独立时钟。

可编辑正文使用 `tokens.typography.input`，`themeTypography` 初始取 body，可通过 `typography { input { ... } }` 独立配置；文字缩放同时作用于该角色。`tokens.textField` 为 `TextFieldTreatment.Standard` 或 `Outlined`，前者使用外置标签，后者使用带缺口的浮动标签，由 Input／Textarea 消费普通描述并保留同一编辑宿主。Material 默认选用 Outlined。

表面可用 `hoverShadows: Array<BoxShadow>?` 配置悬停投影，None 继承普通 shadows，Some([]) 清除悬停投影；按现有 hover／press 采样淡入淡出，按下或禁用时恢复普通投影。Checkbox／Radio 的 appearance 可提供 `stateLayer = Some(MarkerStateLayer(size, empty, selected))`，在标记背后绘制圆形交互状态层；size 表达直径，empty／selected 使用 StateColors。最终圆形与颜色通过 geometry.stateLayer 提供给 painter，普通焦点围绕该圆形；Checkbox Options 显式覆盖 corners 时，焦点跟随解析后的标记轮廓。仅覆盖焦点颜色、宽度或外扩保留主题轮廓，验证环始终围绕标记本身。这两种绘图配置不创建输入区域或动画时钟；标记控件按 metrics.focusOutset 为外围装饰预留空间，高对比度／减少透明度会隐藏这些默认装饰，并保留标记焦点的几何。

默认装饰需要适配实际底色时，可调用 `contrastThemeColors(pair, canvas, minimum = 7.0)`。它把 ColorPair 的背景合成到不透明 canvas，再尽量保留原色并调整完整配对，返回两种不透明颜色；只有背景无法容纳目标对比度时才改变背景。minimum 是有限的 1～21，颜色通道须有限，canvas 必须不透明；非法值抛出 IllegalArgumentException。该函数与内置主题共用 sRGB 对比度算法，不读取系统设置。用于文字通常采用 7.0，边界采用 3.0；应用明确指定的局部样式仍由应用决定。

应用持有稳定 ThemeSource，使用 `context(source) { ... }` 包围实际 Application.run／runWithAsyncio 及挂载入口。Component 可在挂载工厂的构造阶段保存来源，在 render 中调用 `source.read(scope)`；首轮 render 早于 onMount，不应等到 onMount 才初始化这个引用。更新主题 State 通知实际订阅者。局部主题通过明确 Props 传入，临时 Context 不会自动成为挂载树 Provider。三套主题、公共绘图部件和实际控件的完整用法见[主题展示](../scoop.ui.component/examples/themes/README.md)。

`ThemeSource.read(scope)` 自动跟随 UI 根合并后的减少动画、高对比度、减少透明度和文字缩放；`source.current.value` 仍是应用提供的基础主题。两个窗口可以共享来源并使用不同的 `UiPreferences`，不会重复放大字号或改写共享 State。自绘组件也应使用 read；纯数据处理可调用 `source.resolve(ThemePreferences(...))` 或 `theme.withPreferences(...)`。第三方 Theme 必须实现 withPreferences，并同时调整实际部件和 tokens；只读 `Theme.preferences` 返回当前已经应用的偏好，包装主题转交所包装结果。控件用该值解析自己产生的默认选区和底色，不重新读取系统设置。

内置主题在高对比度下使用不透明配色，强化文字、边界与焦点，取消整体禁用透明度；原有禁用透明度先用于解析文字和选中标记的颜色，再保证禁用对比度，保留启用与禁用的色差。减少透明度会将默认填充合成到背景并移除默认阴影／辉光。文字缩放改变字体度量，不改变窗口 DPI。应用的显式文字、颜色和效果覆盖仍由应用决定。减少动画通过有效 ThemeMotion 关闭过渡与连续加载动效，不建立额外时钟。详细规则与对比度阈值见主题规范第 6.1 节。

详细合同见[公共样式规范](../../../docs/specs/SCOOP-UI-STYLE-SPEC.md)、[图像滤镜规范](../../../docs/specs/SCOOP-GRAPHICS-EFFECTS-SPEC.md)和[主题绘图规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md)。普通绘图／缓存测试位于本 Cone，实际窗口与错误收尾位于 tests-cones/ui 和 tests-cones/errors，均使用 scoop.test。

## Linux GNU

Linux GNU 使用同一套公共绘图实现，支持 raster 和 Skia Ganesh GLES 目标，窗口可采用 Wayland 或 X11。使用已准备的 GNU sysroot，运行 `scoop test sysroot/src/scoop.ui.style --release --sysroot ~/.scoop`。三套主题的像素、几何及 GPU／raster 对照在 `tests-cones/themes` 中验证；依赖与实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。

Scrollbar 的调用方可以把固定命中条带作为 geometry bounds，以 `scrollbarOptions { thickness = Some(currentWidth) }` 提供本帧绘制宽度；主题沿交叉轴居中。`ScrollbarVisual.showTrack = false` 只取消轨道绘制，opacity 作用于整条；自动隐藏、命中、重复操作和系统偏好由上层共享滚动控件负责。
