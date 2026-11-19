# 分隔面板触摸与鼠标验证

使用已准备的开发 sysroot 和当前配套库产物运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/resizable --filter 'contact splitters' --release
```

触点测试在分隔线外 12 个逻辑单位开始，验证 hitWidth=32 的真实命中；鼠标和触点共用约束分配、键盘、提交／回滚、外部替换、RTL／方向、重排、零几何、阻止结束和卸载场景。

`--filter ResizableMountWindowTest` 使用三平台共有的窗口用例依次挂载单面板、双面板、三面板、单面板、空布局和双面板，均使用默认不可折叠配置。等待实际呈现后检查把手焦点挂载数量、整体尺寸及卸载，防止无障碍初始化访问不存在的相邻面板。这个用例不依赖原生输入注入。

触点载荷通过受控 native 队列进入实际 Window、Surface 和 UI，等待实际呈现；不代替真实触屏／笔或 Windows／GTK 原生入口验收。每个 Application 用例需要独立进程，鼠标场景用 `--filter 'native splitters'` 选择；桌面须允许窗口呈现与聚焦。验证结果见[交互实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
