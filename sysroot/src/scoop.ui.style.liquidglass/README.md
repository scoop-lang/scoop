# scoop.ui.style.liquidglass

完整的 Liquid Glass 自绘主题，使用 `scoop.ui.style` 的共享几何、材质和资源管理。应用依赖本 Cone，并向原有 UI 根提供同一个 ThemeSource：

```kotlin
import scoop.ui.style.*
import scoop.ui.style.liquidglass.liquidGlassTheme

val source = ThemeSource(liquidGlassTheme())
```

`liquidGlassTheme(mode = ThemeMode.Light, palette = None) { ... }` 与其他主题工厂采用相同的 ShapeThemeBuilder 配置。默认提供完整明暗配色；显式 palette 决定语义色，configure 最后覆盖。切换 `source.current.value` 保留组件、编辑与焦点身份。主题不持有原生资源，不需要 close。

操作、导航、工具栏和浮动层默认采用玻璃；正文与输入保留稳定底色。每类用途可通过 `tokens { glassSurfaces { ... } }` 覆盖。局部 `style { glass(None) }` 清除材质，明确的背景与文字覆盖由应用配对。根的提高对比度／减少透明度改用实体底色，减少动态停止过渡，文字缩放沿实际测量生效。

| 用途 | 模糊 sigma | 折射位移 | 边缘宽度 |
| --- | ---: | ---: | ---: |
| Control | 4 | 2 | 5 |
| Navigation、Toolbar | 8 | 3 | 9 |
| Popover、Dialog、Notification | 12 | 5 | 16 |

以上为局部逻辑像素。小控件保留窄折射边，大浮动面使用更宽的磨砂区域。可从 `theme.tokens.glassSurfaces.get(role)` 直接取得不可变材质供 `GlassEffectContainer` 复用，或 copy 后赋给单个用途；正文和输入用途返回 None。内容染色随实际前景求出最低可读透明度，不缓存下层像素。

完整公共合同见[主题规范](../../../docs/specs/SCOOP-UI-THEME-SPEC.md#51-liquid-glass)；阶段和性能验收记录见[实施记录](../../../docs/ui/liquid-glass/STYLE.md)。
