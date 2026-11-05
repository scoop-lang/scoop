# UI 整改实施记录

依据 [整改方案](REMEDIATION-PLAN.md)，基线为 `fb9d7dcd2c`，先整改现有框架与组件，再继续 shadcn 剩余项。每批完成完整能力后执行对应 release 验证，提交、推送并合并 origin/main。原始日志和临时产物保存在仓库外；本轮先验证 macOS aarch64，未运行 Windows／Linux。

## P0：基线与合同

状态：已完成，2026-11-06；macOS aarch64 release。

- 保留当前 Sidebar 交付基线，未完成 Toast 已另行保存，未计入组件完成数。
- 使用现有 release 工具链与共享 Cone 缓存，按当前源码对齐 window、surface、ui、style、四套主题及 component 的预编译库；未重建编译器或下载外部依赖。
- [UI 规范第 11 节](../specs/SCOOP-UI-SPEC.md#11-整改目标协议p0-合同按批次交付)明确类型化完整结果、释放责任、帧内结果表、失效及构造作用域目标，区分目标协议与现行 API。没有引入临时兼容协议。
- `ComponentMeasureReuseTest` 覆盖 Auto Grid 和 Frame 深度 1／3／6／10，使用实际组件与窗口，分别记录测试叶子及透明计数容器的调用数、不同约束数和耗时，并验证最终尺寸、Frame 基线、一次挂卸与场景呈现。

实际结果：

| 深度 | Auto Grid 叶子测量 | 叶子不同约束 | Grid 测试探针总调用／不同请求 | Frame 叶子测量 | Frame 测试探针总调用 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 3 | 3 | 4／4 | 1 | 2 |
| 3 | 27 | 3 | 40／10 | 1 | 4 |
| 6 | 729 | 3 | 1,093／19 | 1 | 7 |
| 10 | 59,049 | 3 | 88,573／31 | 1 | 11 |

每组包含实际挂载的一个计数叶子和每层一个透明计数容器，容器内使用当前 Grid／Frame；“探针总调用”不包含框架包装及 Grid／Frame 自身回调，不冒充全部布局工作量。深度 10 Grid 的本次单次测量为 96.913ms，仅作这次带探针运行的原始观测，不作为稳定帧时间基准。

验证：相关源码格式化、ui 与 component 的 `check --tests`、两 Cone 的 release 测试构建通过；`UiCompositionTest` 和 `ComponentMeasureReuseTest` 各 1 项通过。原评审提到的 FocusOrigin／FocusVisibility 预编译导入阻塞已通过当前源码重建对齐闭合。普通泛型约束可从具体实例推导布局类型，目标协议不需要新增编译器特性。未运行无关全量测试。

后续 P1～P6 尚未交付；不把基线计数或既有组件验收当作整改通过。
