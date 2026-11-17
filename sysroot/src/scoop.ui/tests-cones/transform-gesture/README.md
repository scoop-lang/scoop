# 内容变换输入验证

使用当前开发 sysroot 和配套库产物构建，每个 Application 用例在独立进程执行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/transform-gesture --filter 'direct transforms preserve' --release
scoop test sysroot/src/scoop.ui/tests-cones/transform-gesture --filter 'direct transform cancellation' --release
scoop test sysroot/src/scoop.ui/tests-cones/transform-gesture --filter 'native transforms combine' --release
```

21 步直接接触场景覆盖短按保留、阈值、完整初始倍率、旋转跨半圈、Touch／Pen 来源、独立分量开关、配对结束、迟到输入和 Accepted 几何变化。40 步取消场景覆盖第三触点、阻止移动／结束、系统取消、重合位置、配置变化、其他 owner 已有归属、原生／直接来源切换、退化几何、取消回调禁用正在切换的分量、接管通知内关闭及 Started 回调内关闭。原生组合回归保留多种类结束与取消行为；配置用例可单独用 `--filter 'direct transform configuration'` 运行，不启动 Application。

触点载荷经 Window 的受控 native 队列进入真实 Surface 和 UI，使用实际窗口呈现与 Accepted 几何。此处不代替真实触屏／笔、Windows／GTK 原生入口或读屏器验收。具体结果与设备范围见[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
