# 双轴虚拟表格的原生动作

使用生产 LazyElement、ScrollViewport、SemanticsHandle 和真实 AppKit 单元格代理，描述一亿行、300 列；只挂载窗口附近及明确动作需要的行列。一个测试进程只运行一个 Application。

多级表头用真实 `columnHeaders` 关系核对已挂载格的组／列表头顺序、未挂载时的列元数据回退、隐藏表头过滤、重新挂载和退休。测试消费与源码对应的预编译依赖，不会为表头查询创建整行单元格。原生辅助入口读取 ColumnHeaderUIElements、行列跨度和行内唯一单元格数，几何断言失败时输出实际及期望矩形。

准备与源码对应的本机 Window、Surface 和 UI 产物后运行：

```sh
scoop check . --tests --release
scoop test . --release
```

43 个阶段验证数据查询的只读边界、目标行列的同步 Realize、回收、Reveal 的 Accepted 几何、精确 Focus／清除、外部焦点取消、滚动取消、列重排、禁用、删除与同 key 重建，以及根关闭。场景扣除顶部及两侧遮挡，固定列只沿纵轴显露；嵌套 withScrollReveal 不能扩大外层区域，兄弟视口及已经结束的场景拒绝。

跨列阶段把三个逻辑列映射到同一锚点，核对挂载前后的真实 columnSpan、唯一代理、唯一实际单元格、300 列中的 298 个行内子项及完整显露几何。改变跨度会退休被覆盖的旧单元格并取消其在途 Reveal；拆分、重排、禁用、移除锚点列、移除行和同 key 重建均验证对象生命周期。只对元数据发起查询时不增加 View 构造数，Realize 精确新增一行一格。

用例同时在真实惰性单元格内合并 Text 子节点，检查每次新挂载的首次 Accepted 几何，以及目标行中随测量创建的固定兄弟格。另一个普通 Portal 在自己的测量阶段创建标签，验证浮层入口也能在首次 Accepted 发布位置。

2026-11-20 本机 macOS aarch64 的格式化、check 与 43 阶段 release 运行通过，进程正常退出、failures 为空，耗时约 1.04 秒。Window 的 3 项 SemanticGrid 定向模型测试通过，覆盖锚点取值、动作转发和非法跨度；Window、Surface、UI、Style、Component 及四套主题的对应 release 依赖已构建，UI API 文档已生成。一亿行、300 列场景累计构造少于 300 行和 2000 格，关闭后所有实例释放。原始报告放在仓库外，完成后清理。

窗口需要可激活的本机桌面。这里直接调用 AppKit 的公开无障碍协议，不代替跨进程客户端或 VoiceOver 验收；本批仅验证本机 macOS。
