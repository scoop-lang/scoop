# 自绘 Counter

只依赖 scoop.ui 及 graphics／surface／window 的普通 executable Cone，不依赖控件库。点击蓝色图形或按 Primary+I（macOS 为 ⌘I，Linux 为 Ctrl+I）会增加组件的成员 State；下面的圆点显示计数，图形宽度通过 Surface spring 动画变化。没有交互且动画结束后不继续请求帧。

```sh
scoop run sysroot/src/scoop.ui/examples/custom --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
SCOOP_WINDOW_BACKEND=wayland scoop run sysroot/src/scoop.ui/examples/custom --release --sysroot ~/.scoop
SCOOP_WINDOW_BACKEND=x11 scoop run sysroot/src/scoop.ui/examples/custom --release --sysroot ~/.scoop
```

Counter 只返回 Element 描述，实例和 State 在更新时保留。Counter 在 onMount 中通过 scope.resource 登记窗口根 ShortcutScope，按平台逻辑键匹配 Primary+I；不重复注册全局热键。Element 在 onMount 中创建 Path、输入区域和动画，全部登记到 ComponentScope；示例不为它们逐个 use。Path 构造时的临时 builder 在本地 use 中关闭。窗口根由 runUiApplication 管理，关闭窗口或抛错都会卸载组件。

输入区域只消费自身处理的事件；未处理的键盘事件继续参与根的快捷键匹配，因此点击图形取得焦点后仍可使用 Primary+I。

示例沿用 macOS 与 Linux GNU 的共享 UI 实现，Linux 使用 Skia GLES/EGL 和 Wayland／X11。该示例演示自绘、状态和动画，不承担 Button／Checkbox 的完整键盘与点击语义。
