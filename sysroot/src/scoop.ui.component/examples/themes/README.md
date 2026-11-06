# 主题与公共绘图部件展示

这是使用正式控件库的普通 executable Cone，显式依赖 macOS、Material、Fluent 三套主题。运行前按仓库的 sysroot 准备流程构建这些库。

```sh
scoop run sysroot/src/scoop.ui.component/examples/themes --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
```

Linux GNU 使用同一个示例和 Skia GLES/EGL 呈现。准备包含 Skia、window、surface、UI 和三个主题的 sysroot 后，可分别选择桌面后端：

```sh
SCOOP_WINDOW_BACKEND=wayland scoop run sysroot/src/scoop.ui.component/examples/themes --release --sysroot ~/.scoop
SCOOP_WINDOW_BACKEND=x11 scoop run sysroot/src/scoop.ui.component/examples/themes --release --sysroot ~/.scoop
```

Wayland 使用 `WAYLAND_DISPLAY`，X11 使用 `DISPLAY`；不设置 `SCOOP_WINDOW_BACKEND` 时优先选择当前 Wayland 会话，否则使用 X11。

顶部选择风格以及 Light／Dark／Follow system。风格与明暗分别保存，跟随系统使用窗口的 ThemeChanged 通知；手动选择只改变自绘内容，不修改系统设置或原生窗框。

表格直接使用 Theme 的几何查询和 ThemePainter 的 Canvas 绘图，展示表面、Checkbox、Radio、两轴 Scrollbar 和带标签的操作表面。列包括普通、悬停、按下、禁用、选中、混合和焦点；不适用的组合显示横线。Scrollbar 的 Selected 列展示另一滑块位置，完整滚动行为不属于这个示例。

下方使用真正的 Label、Button、Checkbox、Frame、Row、Column 和 Stack。点击计数按钮、切换自有状态 Checkbox，或给控件设置焦点后切换主题，可以观察尺寸、字体和外观更新，而计数与勾选状态保留。Enabled 控制计数按钮是否接收激活；Disabled 按钮始终禁用。

底部的 Input／Textarea 共享同一套公共文本编辑能力，切换主题保留正文、选区和历史。名称为空时显示验证说明；名称的 Enter 和备注的 Ctrl+Enter（macOS 为 ⌘Enter）都增加上方计数，备注的普通 Enter 换行。Enabled 同时控制两个输入控件。长备注可以使用主题滚动条，选择、撤销、中文输入和剪贴板沿同一个编辑模型工作。

Drop shadow、Glow、Blur 依次作用于整个预览子树的内容 alpha，外框装饰仍由主题绘制。切换效果只更新现有 Frame 的 Props，不增删包装层。阴影和光晕不增加布局、基线或点击范围。表格中的悬停／按下状态是固定绘图输入，下方真实控件的状态过渡使用 Surface Animatable。

`GalleryPainter` 是示例自身的普通绘图辅助对象，可直接在 RasterSurface 上使用。它复用当前主题 painter 与文字布局，主题或字体 revision 改变时替换资源；挂载版本由 ComponentScope 统一关闭。应用只为共享字体及 UiHost 保留最外层 use。

本 Cone 的 scoop.test 包含主题来源与系统跟随、六种组合在两种 DPI 的离屏绘图。macOS 窗口测试覆盖 Metal／UI 切换及原生主题通知；Linux 窗口测试在 X11 与 Wayland 上覆盖整个展示的 GLES 像素读回、主题／效果切换、隐藏重显、资源释放和最终停帧。更细的几何／像素合同在 style 的 themes 测试 Cone；macOS 的完整控件输入和焦点保持另在 component 的 interaction 测试 Cone 验证。
