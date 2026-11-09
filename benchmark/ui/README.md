# UI 原生算法测量

`clips.cpp` 测量保存画布、追加裁剪、捕获几何和释放快照的组合成本。使用真实 Skia recording Canvas，避免把大图像的 CPU 抗锯齿光栅化混入几何测量。每层保留自己的快照，最后验证中心点命中；矩形与椭圆路径分别覆盖 8、64、256、1024 层。

在仓库根目录以 C++20、`-O2 -DSK_RELEASE` 编译 `benchmark/ui/clips.cpp` 与 `sysroot/src/scoop.graphics/native/shared/geometry.cpp`，头文件目录为该 shared 目录和已准备目标的 `include/skia`，链接同目标的 `libscoop_skia.a` 及其平台库。在同一机器上顺序运行修改前后的 executable，各执行三次，以每个深度的中位数比较。构建与原始结果保存在仓库外。

这个测量只反映 native 几何算法，不代表整个 UI 帧率或 GPU 占用。具体环境、结果和功能验证范围见 [实施记录](../../docs/ui/infrastructure/PROGRESS.md)。
