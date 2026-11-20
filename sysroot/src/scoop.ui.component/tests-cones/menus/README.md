# 菜单窗口测试

使用准备好的开发 sysroot，以 release 分别运行本 Cone 中的测试。每个 Application 测试独立进程执行，避免菜单 tracking、焦点和全局应用状态互相干扰。

- `menu families share real keyboard`：80 步菜单家族场景，覆盖键盘／鼠标、焦点、绑定、嵌套、关闭与保留行为；嵌套 ContextMenu 的右键与 Shift+F10 优先打开内层，内层禁用时由外层处理。
- `context menus combine long press`：29 步公共长按与自绘 ContextMenu 的实际组合。
- `native context menus enter real tracking`：相同 29 步组合进入真实 AppKit 原生 popup tracking，并等待完成回调。

两种长按场景都检查短按仍激活按钮、Touch／Pen、移动超限、第二触点、独立关闭、右键／Shift+F10、候选期间禁用、内层文本长按、最新回调、卸载与停帧。文本内层和外层菜单使用相同延迟，验证一次接触只由成功取得归属的内层处理。原生菜单在 tracking 中检查公开 handle 的 pending，更新菜单属性并等待逻辑 render，再调用 cancel、等待真实关闭并验证下一次长按；没有把请求入队当成展示成功。

契约见 [Component 菜单规范](../../../../../docs/specs/SCOOP-UI-COMPONENT-SPEC.md#33-dropdownmenucontextmenu-与-menubar)与[原生菜单规范](../../../../../docs/specs/SCOOP-UI-NATIVE-MENUS-SPEC.md#3-原生-popup-触发器)。受控接触队列与真实触屏／笔、其他平台和读屏器的验证范围分开记录。

2026-11-21：本机 macOS aarch64 的格式化、check 与 release 构建通过。使用正常 App 启动入口分别运行三项测试，80／29／29 步均完整结束、failures 为空，耗时约 19.60／7.28／7.31 秒。内层右键和 Shift+F10 优先、禁用内层后的外层处理，以及原有焦点、长按和原生 tracking 组合全部通过；原始报告保留在仓库外并在批次完成后清理。
