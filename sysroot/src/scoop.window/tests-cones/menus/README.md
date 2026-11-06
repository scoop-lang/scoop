# 原生菜单测试

本 Cone 验证真实原生菜单、跨 Cone 借用、tracking 中的更新、窗口关闭与 popup 完成；图像数据检查不需要桌面会话。使用已准备好的目标 sysroot，以 release 运行：

```sh
scoop test sysroot/src/scoop.window/tests-cones/menus --release --sysroot sysroot --nocapture
```

macOS 默认用例包括 NSMenu、快捷键、失效动作、关闭期间的标准角色和三个 popup session。编辑焦点用例需要解锁的前台桌面，单独使用 `--ignored` 运行；不要与另一个创建 Application 的用例合并在同一进程。真实 popup tracking 期间的 I/O 推进另由 `scoop.window.asyncio` 的 Cone 测试覆盖。

Linux 在带输入 seat 的真实桌面或测试合成器下，分别以 `SCOOP_WINDOW_BACKEND=x11` 和 `SCOOP_WINDOW_BACKEND=wayland` 运行；测试桌面也应设置相应的 `GDK_BACKEND`。需要 D-Bus session。测试实际展开 GtkPopoverMenuBar、在 tracking 中更新、经 GTK 键盘入口激活动作，并展示和关闭 GtkPopoverMenu。没有 seat 的 headless compositor 不能作为这一组合的通过依据。

Windows 原生 Application 用例需要桌面会话，默认标记为 Ignore；在本机终端单独添加 `--ignored` 执行。它验证窗口菜单栏、原生命令编号与大于 32 位的公共 ID 映射、旧编号退休及借用关闭。SSH 环境执行构建及默认图像测试，不据此声称 GUI 验收通过。三个目标分别为 `x86_64-pc-windows-msvc-ucrt`、`x86_64-w64-windows-llvm-mingw-msvcrt`、`x86_64-w64-windows-llvm-mingw-ucrt`。
