# UI 原生算法测量

`clips.cpp` 测量保存画布、追加裁剪、捕获几何和释放快照的组合成本。使用真实 Skia recording Canvas，避免把大图像的 CPU 抗锯齿光栅化混入几何测量。每层保留自己的快照，最后验证中心点命中；矩形与椭圆路径分别覆盖 8、64、256、1024 层。

在仓库根目录以 C++20、`-O2 -DSK_RELEASE` 编译 `benchmark/ui/clips.cpp` 与 `sysroot/src/scoop.graphics/native/shared/geometry.cpp`、`layers.cpp`，头文件目录为该 shared 目录和已准备目标的 `include/skia`，链接同目标的 `libscoop_skia.a` 及其平台库。在同一机器上顺序运行修改前后的 executable，各执行三次，以每个深度的中位数比较。构建与原始结果保存在仓库外。

这个测量只反映 native 几何算法，不代表整个 UI 帧率或 GPU 占用。具体环境、结果和功能验证范围见 [实施记录](../../docs/ui/infrastructure/PROGRESS.md)。

## shadcn 悬停场景

`profile_hover.py` 在 macOS 构建真实 Gallery 的独立 release 测量程序：使用同一套 demo 源码、系统配色和默认字体，等待首帧稳定，按 16 ms 间隔在左侧控件间往返移动鼠标 240 次，离开控件并等待动画收束，再观察静止窗口。计数来自实际 Metal command buffer 与 render pass，观察器只在该进程内安装。

先准备 sysroot，并重建修改涉及的 graphics、surface、UI、style、主题和 component 预编译库；程序消费这些当前产物，不自动准备系统库。默认使用 `target/release/scoop`、仓库 `sysroot` 和 `tmp/sysroot/aarch64-apple-darwin/cache`，可通过同名选项覆盖。构建目录必须放在仓库外，例如：

```sh
python3 benchmark/ui/profile_hover.py \
  --work-dir "$HOME/repos/scoop-work/hover-before" --runs 3 --trace-passes
```

修改后保留第一次的 executable，用另一目录交替运行前后程序，以减少温度、GPU 频率和运行顺序的影响：

```sh
python3 benchmark/ui/profile_hover.py \
  --work-dir "$HOME/repos/scoop-work/hover-after" --runs 3 --trace-passes \
  --baseline "$HOME/repos/scoop-work/hover-before/hover-profile"
```

比较时保持同一屏幕、缩放、窗口状态和桌面负载，不同时运行其他 GUI 验证。窗口请求 1120×780 逻辑像素，实际可用尺寸会受桌面限制；`--trace-passes` 记录最初几次 pass 的实际纹理尺寸、采样数和格式。每次运行的日志、构建日志及 `summary.json` 都写到 work-dir。

结果包括每帧 pass 数、渲染目标面积总和、GPU 执行时间，以及静止期的帧数和提交数。面积不按 MSAA 采样数或像素格式加权，也不等同于显存占用。GPU 时间包含该进程的提交，不能直接当作活动监视器中的整机百分比；WindowServer、其他应用、刷新率和动态频率都可能影响后者。不要将微秒／帧与浏览器的任务管理器百分比直接换算比较。
