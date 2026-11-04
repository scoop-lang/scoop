# shadcn Gallery

真实原生窗口，使用 `scoop.ui.style.shadcn` 与同一套 `scoop.ui.component`。
当前展示 Button、Input、Checkbox 的普通、invalid、disabled 和真实焦点状态，以及 Button 的六种变体／八种尺寸、Badge 的展示与动作模式、Spinner 的运行与减少动态模式。内容页面包含 Card、Alert、Empty、横纵 Separator、图片 Aspect Ratio、Avatar、Skeleton 和 Progress。目录随控件批次扩充，尚不代表完整 64 项已交付。

在准备好开发 sysroot 后运行：

```sh
SCOOP_SYSROOT="$PWD/sysroot" target/release/scoop run sysroot/src/scoop.ui.component/examples/shadcn --release
```

顶部切换明暗与 LTR／RTL；左侧搜索、选择控件。可以点击、Tab／Space 操作、输入中文，或通过“Focus the normal control”观察程序聚焦。切换主题、方向和页面保留应用模型；搜索不修改控件状态。此轮只在 macOS 验证。

Badge 页面可以切换禁用、撤销动作和替换回调。Spinner 页面中的滚动面板用于观察离屏停帧与滚回恢复；切页、隐藏窗口、停止和减少动态均不会留下持续渲染。自定义 Button／Badge 内容通过 ActionContentStyle 消费解析后的文字、图标尺寸、颜色与间距。

Card、Alert、Empty 页面可切换可选区域和宽窄布局；Card 同时切换 Default／Sm 及头部分隔线，Empty 切换自然媒体／图标底板。正文按钮和输入在这些变化中保留状态。Aspect Ratio 使用示例自行绘制、随挂载释放的同一图像，展示 16:9／1:1 与 Cover／Contain 的组合。

Avatar 页面提供三种尺寸、角标、分组和计数，切换两张本地绘制的头像及空／加载／失败状态，并与 Empty 组合。Skeleton 可显示或隐藏带独立计数和活动图标的 Card，切换后原状态保留。Progress 提供确定值、未知进度、自定义范围及数值文字；两页均可滚动检查动画离屏与恢复，并切换减少动态。
