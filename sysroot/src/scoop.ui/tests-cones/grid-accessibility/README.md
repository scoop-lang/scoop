# 双轴虚拟表格的原生动作

使用生产 LazyElement、ScrollViewport、SemanticsHandle 和真实 AppKit 单元格代理，描述一亿行、300 列；只挂载窗口附近及明确动作需要的行列。一个测试进程只运行一个 Application。

多级表头用真实 `columnHeaders` 关系核对已挂载格的组／列表头顺序、未挂载时的列元数据回退、隐藏表头过滤、重新挂载和退休。测试消费与源码对应的预编译依赖，不会为表头查询创建整行单元格。原生辅助入口读取 ColumnHeaderUIElements 与行列跨度，几何断言失败时输出实际及期望矩形。

这组关联检查已在本机 macOS aarch64 随完整 29 阶段 release 回归通过，进程正常退出、failures 为空，耗时约 8.53 秒；格式化、check 与原生辅助库 release 构建通过。修复只改变 macOS 虚拟单元格的表头查询，未重建编译器和其他 GUI 库。

准备与源码对应的本机 Window、Surface 和 UI 产物后运行：

```sh
scoop check . --tests --release
scoop test . --release
```

29 个阶段验证数据查询的只读边界、目标行列的同步 Realize、回收、Reveal 的 Accepted 几何、精确 Focus／清除、外部焦点取消、滚动取消、列重排、禁用、删除与同 key 重建，以及根关闭。场景扣除顶部及两侧遮挡，固定列只沿纵轴显露；嵌套 withScrollReveal 不能扩大外层区域，兄弟视口及已经结束的场景拒绝。窗口需要可激活的本机桌面。这里直接调用 AppKit 的公开无障碍协议，不代替跨进程客户端或 VoiceOver 验收。

2026-11-20 本机 macOS aarch64 的格式化、check 与 29 阶段 release 运行通过；一亿行、300 列场景累计构造少于 300 行和 2000 格，元数据查询期间构造数不变，Realize 精确新增一行一格，关闭后所有实例释放。中心格原生 Frame 完整落在可用区域，固定列显露和 Focus 均不改变横向位置。3 项区域计算、4 项既有留白测试及祖先视口原生回归通过。UI API 文档已生成，withScrollReveal 入口可查询。原始报告保存在仓库外。

用例同时在真实惰性单元格内合并 Text 子节点，检查每次新挂载的首次 Accepted 几何，以及目标行中随测量创建的固定兄弟格。另一个普通 Portal 在自己的测量阶段创建标签，验证浮层入口也能在首次 Accepted 发布位置。旧发布顺序下首次几何断言失败；提前完成逻辑语义发布后，29 阶段、上述浮层检查及既有 accessibility-content／accessibility-scene 实窗回归均通过。名称、值在捕获后的真实变化仍撤下旧几何，Discarded 保留规则不变。
