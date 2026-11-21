# 双轴虚拟表格的原生动作

使用生产 LazyElement、ScrollViewport、SemanticsHandle 和真实 AppKit 单元格代理，描述一亿行、300 列；只挂载窗口附近及明确动作需要的行列。通过已有 `contentOrigin` 和相对当前窗口的条目位置避免绘制时叠加两个相反的大平移；离屏 Realize 仍保留目标的真实相对范围。一个测试进程只运行一个 Application。

多级表头用真实 `columnHeaders` 关系核对已挂载格的组／列表头顺序、未挂载时的列元数据回退、隐藏表头过滤、重新挂载和退休。测试消费与源码对应的预编译依赖，不会为表头查询创建整行单元格。原生辅助入口读取 ColumnHeaderUIElements、行列跨度和行内唯一单元格数，几何断言失败时输出实际及期望矩形。

准备与源码对应的本机 Window、Surface 和 UI 产物后运行：

```sh
scoop check . --tests --release
scoop test . --release
```

51 个阶段验证数据查询的只读边界、目标行列的同步 Realize、回收、Reveal 的 Accepted 几何、精确 Focus／清除、外部焦点取消、滚动取消、列重排、禁用、删除与同 key 重建，以及根关闭。场景扣除顶部及两侧遮挡，固定列只沿纵轴显露；嵌套 withScrollReveal 不能扩大外层区域，兄弟视口及已经结束的场景拒绝。远端回归直接显露第 5000 万行，核对约 12 亿像素位置的 24 像素高单元格仍保留完整范围，并保持外部焦点。

远端格还在实际行内、单元格外声明 Portal，验证其按钮能从窗口原生树发现、已发布可见几何并执行动作；关闭后旧节点退休，重开获得新节点，滚动回收 owner 时弹层一并退休。该用例覆盖虚拟表格只投影行列时不能丢失行内弹层的边界。

跨列阶段把三个逻辑列映射到同一锚点，核对挂载前后的真实 columnSpan、唯一代理、唯一实际单元格、300 列中的 298 个行内子项及完整显露几何。改变跨度会退休被覆盖的旧单元格并取消其在途 Reveal；拆分、重排、禁用、移除锚点列、移除行和同 key 重建均验证对象生命周期。只对元数据发起查询时不增加 View 构造数，Realize 精确新增一行一格。

用例同时在真实惰性单元格内合并 Text 子节点，检查每次新挂载的首次 Accepted 几何，以及目标行中随测量创建的固定兄弟格。另一个普通 Portal 在自己的测量阶段创建标签，验证浮层入口也能在首次 Accepted 发布位置。

2026-11-20 本机 macOS aarch64 的格式化、check 与 46 阶段 release 运行通过，进程正常退出、failures 为空，耗时约 0.91 秒。graphics 的 13 项 Geometry 定向测试通过，覆盖远端完整范围／坐标转换、原生借用、裁剪与命中；本批只替换 graphics 产物，其余已验证的 Window、Surface、UI、Style、Component 及主题依赖继续复用。一亿行、300 列场景累计构造少于 300 行和 2000 格，关闭后所有实例释放。原始报告放在仓库外，完成后清理。

窗口需要可激活的本机桌面。这里直接调用 AppKit 的公开无障碍协议，不代替跨进程客户端或 VoiceOver 验收；本批仅验证本机 macOS。

`NativeHierarchyTest` 另用两个各 25 阶段的 Application 验证 LazyElement 与 data collection 的层级描述和展开操作。一亿条记录只读取指定条目的深度／展开状态，测试来源的 build 一旦被调用即失败。公开 AppKit setter 经过实际模型，覆盖展开、收起、禁用、缺少动作或状态、应用拒绝、重排、删除后重用 key 及关闭；逐次核对模型变更，不能把 void setter 返回当作操作成功。这两项测试不要求系统焦点。

2026-11-21：层级批次的 Window／UI 与本 Cone 格式化、check、release 构建通过；Window 3 项集合模型测试通过。两个层级 Application 分别约 0.90／1.09 秒完成，原 51 阶段双轴回归约 1.40 秒完成，均 failures 为空且 end／complete 齐全。行数使用公开 rowCount，目标行通过按行列访问的单元格取得父行，避免完整 children 查询枚举整个集合。Window／UI API 文档生成及 hierarchyLevel／setExpanded 查询通过；本批仅验证本机 macOS。

2026-11-21：Portal 语义根修复后的格式化、check、release 构建及 51 阶段原生回归通过，约 1.27 秒，failures 为空并完整退出。新增窗口直接子项查询不枚举一亿行数据；菜单动作、关闭重开、行回收和原有几何／表头／跨度组合均通过。同一候选工具链下，`portal-modes` 的动态模态、嵌套优先级、编辑保留及中断关闭回归约 1.19 秒通过。

单行可选性复用这两个 Application 的真实集合：普通行与不可选择项共享选择快照，逐项核对 selected 属性、setter 和原生已选行列表。可选性切换保留身份，不可选择项仍 enabled；缺少动作、禁用、缺少选择模型、业务拒绝、删除重建及关闭均不能误调选择回调。测试仍只查询少量 key，build 一旦被调用即失败。

2026-11-21：单行可选性批次完成 Window 的 4 项集合模型测试；Lazy 与 data collection 两项 25 阶段 AppKit 回归分别约 1.00／1.06 秒通过，failures 为空且 end／complete 齐全。相关 Cone 格式化、check 与 release 构建通过，Window API 文档生成及 selectable 查询通过。本批仅验证本机 macOS，原始报告在合并后清理。

`StructuralRowsTest` 使用 `SemanticGrid.hasCells` 混合普通行与无列单元格的详情行。原生按区间读取单行，避免完整 children 枚举一亿项；离屏详情无子控件，挂载后暴露真实按钮与嵌套静态表格，按钮沿原动作执行。普通行改成详情行会退休旧单元格，恢复普通行产生新代理；回收和关闭后实际子节点失效。回调中的 UI State 写入被只读边界拒绝。

2026-11-21：结构行批次的 Window／UI、本 Cone 格式化与 check 通过；Window 9 项集合／单元格模型测试通过。结构行 5 阶段原生回归约 1.04 秒、原有 51 阶段双轴回归约 1.55 秒通过，均 failures 为空且完整退出。共享 C++ 投影通过本机语法检查，Window／UI API 文档生成及 hasCells 查询通过；本批不计作其他平台或 VoiceOver 验收。
