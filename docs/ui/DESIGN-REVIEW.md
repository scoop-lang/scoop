# scoop.ui 框架设计调研报告

日期：2026-11-06。源码基线：`5dcb269e0c`。本文是设计评审与建议，不是已经生效的规范；采纳任何建议前，按 spec 先行原则先修订 [UI 规范](../specs/SCOOP-UI-SPEC.md)及相关设计文档。

后续复核与整改见 [UI 框架与组件库整改方案](REMEDIATION-PLAN.md)，其基线包含已合入的 shadcn 组件。新方案修正本文的影响范围和实现前提，取代第 8 节的推进顺序；第 6 节放宽 receiver 查找限制的建议已撤回，保留 D0a，通过 API 与组合方式减少调用噪音。项目未发布，整改不承担历史版本兼容性。

## 0. 范围与方法

- 阅读：[UI 总体设计](DESIGN.md)、[DSL builder 设计](DSL-BUILDERS.md)、[基础交互设计](foundations/DESIGN.md)、[控件基础补齐设计](component-foundations/DESIGN.md)、[shadcn 设计](shadcn/DESIGN.md)及 `SCOOP-UI-SPEC.md` 的布局、场景、错误章节。
- 源码：`scoop.ui`（约 12.8k 行）的 view／runtime／state／layout／paint／host／lifecycle／interaction 全部阅读，focus／lazy 抽查；`scoop.ui.component`（约 6.5k 行）抽查 Button、Flex、ThemeAccess 和 themes 示例。style 与三套主题只看了与控件交接的入口。
- 方法：静态阅读与调用路径推演。**没有运行测试、基准或 profiler**；文中的复杂度与调用次数是由代码推出的上界或典型路径，进入实施前应先用定向基准确认量级。
- 不涉及：文本排版算法、IME、Surface／Window 后端、主题视觉质量、accessibility。

## 1. 结论摘要

当前框架在资源生命周期、owner 线程、失败帧不发布几何、静止停帧等**正确性合同**上设计得相当细致，Component／Element 分层、公开 Element 协议、控件库只经公开 API 实现等**分层边界**也是清晰的。主要缺陷集中在**增量性**：更新、布局、场景三个阶段都以"全量重做"为默认且没有跳过通道，再叠加多次测量的乘法效应。这些问题在当前演示规模下不明显，但 [shadcn 全套控件](shadcn/DESIGN.md)（64 项，含 Data Table、Calendar、Sidebar）将建立在这些合同之上，越晚调整迁移面越大。

| # | 问题 | 级别 | 影响面 |
| --- | --- | --- | --- |
| 1 | rebuild 无条件级联整棵子树 | 高 | 所有状态变化，含 hover／press |
| 2 | 每帧全树 measure + scene，无跨帧布局缓存 | 高 | 所有帧，含仅绘制值变化的动画帧 |
| 3 | 嵌套多次测量呈指数增长 | 高 | 嵌套 Flex／Grid／Frame |
| 4 | 缺少统一的挂载树继承值机制 | 中 | 主题、字体、方向、禁用态等 |
| 5 | 资源只能在首次 render 之后获取 | 中 | 所有交互控件 |
| 6 | View 形状持续扩张、身份含配置位 | 中 | 框架可维护性、Portal 状态保留 |
| 7 | 任一回调异常关闭整个宿主 | 中 | 多窗口应用 |
| 8 | 线性扫描的实例表与订阅表 | 低–中 | 大列表、共享 State |
| 9 | 未缓存的 drawLayer 比直接绘制更贵 | 低 | 不在 Tiles 策略下的 drawLayer |
| 10 | DSL 调用噪音 | 低 | 应用代码可读性 |
| 11 | 文档与 spec 漂移 | 低 | 设计依据可信度 |

建议优先在 shadcn 实施前处理 1、2、3、5；4 与 6 可合并成一次挂载树合同调整。

## 2. 增量性缺陷

### 2.1 rebuild 无条件级联整棵子树

**现状。** [reconcile.scoop](../../sysroot/src/scoop.ui/src/runtime/reconcile.scoop) 第 90–91 行对每个匹配成功的子节点执行 `child.replace(...)` 与 `child.rebuild(turn)`；`rebuild` 对 Component 重跑 render，对 Element 继续 reconcile 子项并调用 `update()`。路径上没有任何"输入未变则跳过"的判断。[State](../../sysroot/src/scoop.ui/src/state/state.scoop) 的 setter 也不比较新旧值（设计文档明确"首期写入即通知，包括同值写回"）。

**推演。** 根组件的一个 State 写入会重跑整个应用的 render 并调用全部 `Element.update`。典型的 hover 路径：

1. `Press.publish` 写 `InteractionState`（这里有 `!=` 守卫，是少数例外）；
2. Button.render 重跑，生成新的 `LabelProps` 与 `ButtonInkProps`；
3. Label 组件重新 render，其文字元素 update；
4. 下一帧执行全树布局（见 2.2）。

即使 Label 的输入完全不变，第 3 步也无法避免。

**建议。**

- Props 跳过：`replace` 时若新旧 input 引用相同（`===`），或 `P: Equals<P>` 且相等，则只在节点自身被订阅失效时才重跑，否则停止向下级联。这只需在 `ComponentDescription`/`ElementDescription` 上提供一个比较入口，不需要新的 View 形状。
- State 可选等值抑制：对 `T: Equals<T>` 的 State 在 setter 中跳过同值通知，或提供显式 `state(initial, distinct = true)`；保持现有"同值也通知"作为非 Equals 类型的行为。
- 判定依据：按"一次 hover 只重跑被 hover 的 Button 及其 Ink，不触及兄弟子树"写 spec 合同和计数测试。

### 2.2 每帧全树 measure + scene

**现状。** [root.scoop](../../sysroot/src/scoop.ui/src/host/root.scoop) 第 87 行在每个实际帧从根以 `Constraints.tight(frame.size)` 开始测量；[measure.scoop](../../sysroot/src/scoop.ui/src/layout/measure.scoop) 第 97 行进入时即清除 `metrics`。节点没有布局 dirty 标记，也没有 relayout boundary。场景同样每帧全树重放；输入区域几何（`SceneContext.place`）和焦点场景（`FocusController.beginFrame`）依赖这次重放，因此 scene 目前也不可跳过。

像素缓存只有 `SceneContext.drawLayer`：调用方提供手写的 `revision: Long` 声明"所有影响像素的输入"，且只有在滚动视口给出 `PaintCachePolicy.Tiles` 时才真正缓存。

**推演。** 设计第 6 节"首期每帧遍历布局，昂贵计算由具体元素缓存"把成本转移给每个 Element：任何动画帧（包括 hover 颜色过渡、caret 闪烁、进度条）都会触发全窗口的 measure 回调和 scene 回调。ParagraphHost 能复用段落排版，但 Flex／Grid／Frame 的分配计算、Theme metrics 查询、主题几何仍会每帧执行。手写 revision 则把缓存正确性交给每个控件作者，漏算一个输入就会显示过期像素，而且没有框架层手段发现。

**建议。**

- 布局 dirty：节点在 rebuild、Props 替换或显式 `requestLayout` 时标记自身与祖先；未标记的节点在约束与上次相同时直接返回上次 `LayoutMetrics` 和 placements，不进入 Element.measure。动画需要影响布局时由 Element 显式请求（Animatable 采样可提供"读取即登记"的布局依赖），只影响绘制的动画不再触发布局。
- scene 与几何分离：把"命中区域、焦点、锚点"登记从每帧重放改为随布局结果缓存，只在布局或变换变化时重新提交；之后才能考虑子树级的 scene 跳过或保留式 display list。
- revision 自动化：至少提供由 Props 引用／相等性和主题身份派生的默认 revision，手写值作为补充，而不是唯一来源。

这三项相互依赖，建议按"布局 dirty → 几何登记缓存 → 绘制跳过"的顺序分批，每批以帧回调计数作为验收。

### 2.3 嵌套多次测量呈指数增长

**现状。** [UI 规范第 330 行](../specs/SCOOP-UI-SPEC.md)允许子项在放置前以不同约束重新测量，但只复用"相邻同约束"的最后结果。[flex.scoop](../../sysroot/src/scoop.ui.component/src/layout/flex.scoop) 对每个子项最多调用三次 `measureChild`（第 78 行内容 basis、第 112 行分配后、第 145 行 Stretch），源码注释也承认只有主轴无界时嵌套才保持线性。框架没有 intrinsic 尺寸查询，内容 basis 只能用一次完整测量代替。

**推演。** 每层对子项以 k 种不同约束测量，子项又对其子项重复同样的过程，d 层嵌套最坏约为 kᵈ 次叶子测量。Flex 的 k 最大为 3，常见的 Column > Row > Frame > Label 组合已有 3–4 层；Data Table 的单元格、Sidebar 的嵌套分组会更深。由于 2.2 的每帧全量布局，这个乘数在每个动画帧都出现。

**建议。**

- 帧内多结果缓存：MeasureContext 以 (子节点, Constraints) 为键保存本帧所有成功结果及对应的子树 placements，任何重复约束都命中，不再限于"相邻"。spec 现有"不缓存历史结果来配合另一套 placements"的限制需要相应改写：放置时恢复与最终约束对应的那一套子树结果。
- intrinsic 查询：增加只返回尺寸的 `minIntrinsic`／`maxIntrinsic`（或单一的 `intrinsicMain(crossConstraint)`）协议，默认实现回退到 measure 并缓存；Flex 的内容 basis、Grid 的轨道尺寸改用 intrinsic，不触发子树的放置。
- 与 2.2 的跨帧缓存结合后，稳定界面的重复测量应接近零。

## 3. 设计一致性

### 3.1 缺少统一的挂载树继承值机制

**现状。** 设计第 4.1 节明确"不增加 UiLocal、Provider 或挂载树环境继承"，但实际出现了三种互不相同的替代做法：

| 需求 | 当前做法 | 代价 |
| --- | --- | --- |
| 布局方向 | 专门的 `ViewBody.Direction` 和 [direction.scoop](../../sysroot/src/scoop.ui/src/runtime/direction.scoop) 中独立的 `refreshDirection` 传播路径 | 框架核心多一种节点形状与一条更新路径 |
| 主题 | [ThemeAccess](../../sysroot/src/scoop.ui.component/src/theme.scoop) 在挂载时从语言 Context 捕获一次，显式来源再走 Props | 局部主题只能逐层透传；Context 换绑不生效 |
| 字体 | 不继承 | [themes 示例](../../sysroot/src/scoop.ui.component/examples/themes/src/main.scoop)中每个 label／button 都写 `this.fonts = Some(this @Showcase.fonts)` |

局部主题、字体集合、禁用态、组件密度、文字缩放等后续需求，按现有路线会各自变成一种 ViewBody 或一条 Props 透传链。

**建议。** 提供一种类型化的挂载树继承值：描述侧 `provide(key: AmbientKey<T>, value: State<T> 或 T, content: View)`，读取侧在 render 中读取并登记为普通依赖（复用 StateSignal），不保存通用 Context 快照。Direction 改为它的一个实例，删除 `ViewBody.Direction` 与 `refreshDirection`；ThemeAccess 改为读取主题 key。这不是新的依赖注入框架：键是普通类型化值，传播只沿已有挂载树，订阅只用已有 State 机制，正好替换已经出现的三套做法。

### 3.2 资源只能在首次 render 之后获取

**现状。** [root.scoop](../../sysroot/src/scoop.ui/src/host/root.scoop) 第 52–53 行先 `rebuild`（即首次 render）再 `mountPending`（onMount）。InputArea、FocusTarget、Press、动画、painter 都只能在 onMount 中通过 ComponentScope 取得。

**后果。**

- [Button](../../sysroot/src/scoop.ui.component/src/button.scoop) 用 `State<PressControl?>` 保存交互控件，第 32 行在 onMount 中写入，导致每个 Button 挂载后立即再 render 一次。
- Element 字段普遍是 `Option` 加强制解包：`painters!!`、`motion!!`、`content!!`、`value.control.value!!`（第 122、177 行）。这正是 AGENTS 实现准则中"由结构保证不可能缺失，而不是靠 unwrap 兜底"所反对的形态，只是发生在库代码而非 IR。
- 首次 render 读不到交互状态，只能用默认 `InteractionState()` 兜底。

**建议。** 让工厂在创建时取得作用域，例如 `create: (Props<P>, ComponentScope) -> C`，成员可以是非 Option 的 `val`；onMount 保留给需要"已挂载"语义的副作用（启动任务、首次聚焦）。如果担心构造期失败的清理，现有"首次 onMount 失败仍清理已取得资源"的规则可以直接平移到构造期。

### 3.3 View 形状扩张与身份中的配置位

**现状。** 设计第 3 节称 View "仅保存空内容、Component 和 Element 三种形状"，[view.scoop](../../sysroot/src/scoop.ui/src/view/view.scoop) 实际有 Empty／Component／Element／Portal／Lazy／Direction 六种。每增加一种，[node.scoop](../../sysroot/src/scoop.ui/src/runtime/node.scoop) 的 replace／rebuild／mountPending／unmount、measure、finishMeasurement、scene、instances、direction 都要补分支。

此外第 79 行 `ViewShape.Portal(options.isModal)` 把模态配置放进了身份：同一弹层在模态与非模态之间切换会卸载并重建内容，丢失内部 State 与焦点。

**建议。**

- 3.1 落地后 Direction 可移除；Portal 与 Lazy 是否能表达为框架内部提供的特殊 Element（使用同一 Element 协议加少量内部能力），值得在下一轮整理时评估，目标是让核心节点类型回到 Empty／Component／Element 加少量确有必要的内部节点。
- 把 `isModal` 从 shape 移到可替换的 PortalOptions，由 PortalInstance.replace 处理屏障的建立与撤销。

## 4. 错误隔离

**现状。** [host.scoop](../../sysroot/src/scoop.ui/src/host/host.scoop) 的 `callback` 捕获任何异常后执行 `close()`（第 126 行），关闭宿主下的所有根和 SurfaceHost。设计第 10 节"首期沿 SurfaceHost 的现有异常通路关闭所管理的根和 Surface"在单窗口时合理，但设计第 9.1 节已支持多窗口共享一个 UiHost。

**后果。** 多窗口应用中，一个窗口的 render、measure 或输入处理抛错，会连带关闭其他与之无关的窗口和它们的业务状态观察者。

**建议。** 不引入错误边界组件，只把失败范围收窄到出错的 UiRoot：关闭该根及其 Surface，异常交给应用 handler（或作为 `ApplicationEvent` 交付），其余根继续运行；宿主级失败（例如 proxy 投递失败）仍关闭整个宿主。

## 5. 实现层面的低效

### 5.1 线性扫描的实例表与订阅表

- [instances.scoop](../../sysroot/src/scoop.ui/src/runtime/instances.scoop) 的 `ActiveInstances` 是全局 `ArrayList`，`createNode` 第 23 行与 `releaseInstance` 第 53 行都线性扫描。挂载或卸载 N 个节点为 O(N²)；VirtualList 滚动会持续挂卸，所有窗口共享同一张表。
- [observation.scoop](../../sysroot/src/scoop.ui/src/state/observation.scoop) 中 `StateSignal.subscribe`／`unsubscribe` 线性去重。一个共享 State 被 M 个列表项观察时，每次全量 rebuild 的重新订阅是 O(M²)；`DependencyCollector.read` 同样线性。

建议改为按引用身份的 HashSet（或在 MountedNode 上直接保存"是否已登记"位来检测重复挂载，省掉全局表）。

### 5.2 未缓存的 drawLayer 比直接绘制更贵

[cache.scoop](../../sysroot/src/scoop.ui/src/paint/cache.scoop) 的 `drawUncached` 每次都 `recordDrawing`，再经 `replayPaintLayer`（第 144–147 行）执行 `saveLayer` 后回放。不在 Tiles 策略下时，一次 drawLayer 比直接 `draw` 多一次录制和一个离屏层。若隔离语义只需要独立的 Canvas 状态，可直接走 `withState` 绘制；只有确实需要合成隔离（透明度、混合）时才 saveLayer。

## 6. API 易用性

**现状。** DSL 迁移的 D0a 规则禁止 receiver lambda 隐式访问外层 receiver。结合 3.1 的字体不继承，[themes 示例](../../sysroot/src/scoop.ui.component/examples/themes/src/main.scoop)的 render 中大量出现：

- `this @Showcase.xxx` 前缀，几乎每个子项都有；
- `this.style = Some(spanStyle { ... })`、`this.width = Some(42.0)` 这类 Option 包装；
- 布局属性（`spacing`、`alignment`）可以写在子项之后，阅读顺序与结构不符。

**建议。**

- 规则层面：保留 D0a 的外层 receiver 隐式绑定限制；原先改为 `@DslMarker` 风格限制的建议已撤回。使用普通 View 函数和具名数据／动作参数减少跨 receiver 访问，显式外层引用继续可用。
- 库层面：优先使用已有嵌套配置入口减少多余的 Option 包装，按实际字段区分继承、清除和显式值，不机械地为每个属性增加另一套 setter。
- 共享显示资源先提供根默认字体与控件显式覆盖；业务输入和动作继续显式传入。第 3.1 节的通用挂载树继承值不作为前置，具体方案与验收见 [整改方案第 7 节](REMEDIATION-PLAN.md#7-显示默认值与-dsl-整改)。

## 7. 文档漂移

- [DESIGN.md 第 151 行](DESIGN.md)仍写"首期每个子项每帧只测量一次"，spec 第 330 行已改为允许重测；
- DESIGN.md 第 3 节"View 仅保存三种形状"与现状不符；
- 第 6 节"首期每帧遍历布局"缺少对多次测量叠加后成本的说明。

建议在处理 2.x 时一并修订 DESIGN.md，避免后续设计继续引用过期前提。

## 8. 建议的推进顺序

| 批次 | 内容 | 验收要点 |
| --- | --- | --- |
| R1 | Props 跳过与可选 State 等值抑制（2.1）；实例表与订阅表改为 HashSet（5.1） | hover 单个 Button 时 render／update 计数只涉及该 Button 子树；挂卸 1 万项线性 |
| R2 | 帧内多约束缓存与 intrinsic 查询（2.3） | 深度 6 的嵌套 Flex 每帧叶子测量次数与深度呈线性 |
| R3 | 跨帧布局 dirty 与仅绘制动画不触发布局（2.2 前半） | 颜色过渡动画期间 Element.measure 调用为 0 |
| R4 | 工厂取得 ComponentScope（3.2）；挂载树继承值替换 Direction／ThemeAccess（3.1、3.3） | Button 挂载只 render 一次；控件无 `!!` 字段；局部主题与字体无需逐层透传 |
| R5 | 几何登记缓存与绘制跳过（2.2 后半）；根级错误隔离（4） | 静止子树不进入 scene；一个窗口抛错其他窗口继续 |

R1–R3 主要是框架内部改动，对控件库 API 影响小，建议在 shadcn 第一批之前完成；R4 会改动所有控件的构造方式，越早做迁移面越小；R5 可与 shadcn 并行。每一批都应先补定向计数测试，量出改动前的基线，再确认改进是否达到预期量级。
