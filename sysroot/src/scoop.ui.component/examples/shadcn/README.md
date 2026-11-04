# shadcn Gallery

真实原生窗口，使用 `scoop.ui.style.shadcn` 与同一套 `scoop.ui.component`。
当前展示 Button、Input、Checkbox 的普通、invalid、disabled 和真实焦点状态；目录随控件批次扩充，尚不代表完整 64 项已交付。

在准备好开发 sysroot 后运行：

```sh
SCOOP_SYSROOT="$PWD/sysroot" target/release/scoop run sysroot/src/scoop.ui.component/examples/shadcn --release
```

顶部切换明暗与 LTR／RTL；左侧搜索、选择控件。可以点击、Tab／Space 操作、输入中文，或通过“Focus the normal control”观察程序聚焦。切换主题、方向和页面保留应用模型；搜索不修改控件状态。此轮只在 macOS 验证。
