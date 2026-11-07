# UI 应用根测试

普通测试 Cone，验证不创建业务 Window 或 Surface 的 ApplicationComponent。需要已准备的 window、window.asyncio、graphics.text、Surface 和 UI 系统库；测试仍使用真实原生 Application、菜单及文件 session。

```sh
scoop test sysroot/src/scoop.ui/tests-cones/application --release --sysroot sysroot
```

覆盖 State 合并更新和动态依赖、最新 Props 与 InputComparison、key 重排和移除、实例归属、资源逆序清理、构造／render／mount／update／回调失败、嵌套关闭与默认错误传播。原生菜单动作经 NativeMenuLease 的真实使用入口交付；文件请求在展示前取消，验证类型化结果、晚登记和显式注销。测试断言没有窗口事件，不显示文件选择器。

Linux 的 Application 需要有效 GTK 显示会话，分别使用 `SCOOP_WINDOW_BACKEND=x11` 和 `wayland`；X11／Wayland 所需显示变量沿用普通 window 测试。Windows SSH 会话无法创建原生 Application，使用 `--no-run` 编译完整程序，在桌面会话执行本 Cone；构建成功不代表原生运行验收。当前结果和限制见[实施记录](../../../../../docs/desktop-ui/PROGRESS.md)。
