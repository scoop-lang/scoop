# 测量、布局、方向与视觉基础

本文属于 [控件基础设计](DESIGN.md)，规定 L1／L2／V1 的合同；三个批次均已完成，验证范围见实施记录。测量及方向归 `scoop.ui`，布局策略和内容适配归 `scoop.ui.component`，装饰归 `scoop.ui.style`，共用形状归 `scoop.graphics`。

## 1. 演进现有测量协议

选择直接放宽现有 `measureChild` 合同，允许容器先取得内容尺寸，再按分配结果重新测量。暂不另建 intrinsic／min-content 查询接口：当前首要问题是给真实子树分配最终约束，重复的查询协议会要求每种 Element 同时维护两套布局行为。

`MeasureContext` 的新规则：

1. 每个直接子项至少成功测量一次、最多放置一次。尚未放置的子项可以在不同约束下重新测量，最后一次成功结果成为本次子项的布局；放置后再测量报告状态错误。
2. `placeChild` 只记录最后结果的原点，不接受旧测量结果作为另一份可呈现场景。每次父节点重新进入 measure 都建立新的子项记录，不能遗留先前的 placements。
3. 相邻的相同约束请求可以直接复用最后一次结果及对应子树布局。不能仅缓存历史 LayoutMetrics，再将它与另一约束下留下的 placements 拼接；本轮不建立全树历史测量缓存。
4. 一次布局使用同一个时间采样。普通重新测量不更新 Props、执行 Component.render、重新挂载或改变 key；measure 仍不能写 State、启动动画或发出业务事件。
5. Element 可以替换自有排版／绘制缓存，及时关闭被替换资源；仍被最近 Accepted 场景用于命中或文本查询的资源，按既有提交规则保留到替换成功。
6. 测量异常沿现有根失败路径收尾，不能用旧结果假装本次布局成功。未放置子项保留实例，但本轮不呈现。

不提供无限求解循环、全局迭代预算或通用 constraint solver。库内 Flex／Wrap／Grid 使用下面明确的有限阶段；相同约束不重复测量，已知尺寸用显式 basis，避免无必要的深层探测。

### 1.1 LazyElement 必须同步调整

仅删除 `ChildMeasure.attempted` 检查是不够的。原有 `LazyChildren.finish` 在每次测量回调结束时立即改变 frame participation 并 trim；父布局重新测量时，这会使临时不可见项卸载又重建，丢失状态和输入归属。

因此同一根布局内先记录每个 lazy 节点最后一次测量的候选集合、placements 和最终约束，待最终布局确定后才执行可见性／保留收尾：

- 按需准备仍发生在现有显式 `measureItem` 边界；已准备的相同 key 在本次布局中复用，不因中间探测 trim。
- `LazyMeasureContext` 对未放置 item 同样允许重新测量；放置后的 item 不能在同一 context 再改尺寸。
- 最终使用集合以最后一次有效布局为准；焦点、按下序列、IME 或待聚焦请求所固定的项继续保留。临时探测项在最终收尾时按已有 retention 规则处理。
- 不复制整份 ItemSource、不创建全量条目、不为确定自然尺寸遍历所有虚拟项。滚动视口必须有有限尺寸或显式 basis；无界列表没有可供 Flex 查询的全内容自然高度。
- 场景、锚点、滚动快照和 frame-completion 动作只由最终 scene 产生一次。中间测量不发布 ScrollState 修正、焦点或阅读位置。

这只是延后当前 lazy 收尾时机，不新增第二棵树。`ParagraphHost`、TextEditHost、ScrollViewport、overlay 和 LazyElement 的缓存也必须逐项检查多次测量下的替换与所有权。

## 2. Flex：明确剩余空间分配

增强 Row／Column 的布局策略，提供可直接由第三方调用的 Flex 容器和显式 item 配置。普通 `add(view)` 使用内容 basis；需要分配规则时使用 `item(view) { ... }`，不通过识别某个子组件类型读取隐藏的 flex 属性。

每项具有 `basis`（Content／固定逻辑长度）、grow、shrink、主轴 min／max 和交叉轴对齐覆盖。grow／shrink 是有限非负权重；主轴约束沿用非负、max 不小于 min 的规则。交叉轴支持 Start／Center／End／Stretch／Baseline，主轴提供 Start／Center／End／SpaceBetween／SpaceAround／SpaceEvenly。容器配置 gap、方向和显式 overflow／clip；不是完整 CSS flexbox。

布局采用有限步骤：

1. 扣除 padding 和 gap。固定 basis 直接参与计算，Content 项在受父交叉轴约束的条件下取得基础尺寸。
2. 主轴有界时，正剩余按 grow 分配，负剩余按 `shrink × basis` 分配；达到 min／max 的项冻结，只在未冻结项中重新分配。每次冻结至少一项，或一次完成剩余分配。
3. 主轴无界时使用内容／固定 basis，不分配不存在的剩余空间；未被 min／max 改变的 Content 直接保留自然测量，避免嵌套布局重复施加没有分配需求的 tight 约束。需要滚动视口的项仍必须提供有限 basis 或尺寸。
4. 按最终主轴分配重新测量受影响子项。Stretch 使用最终交叉轴尺寸；自然交叉尺寸先由内容确定，必要时再作一次有界的 stretch 测量。基线使用实际最终 LayoutMetrics，缺少对应基线时以该项底边参与对齐。
5. 计算最终位置和容器尺寸，一次放置。几何保留逻辑浮点值，不逐项取整累积误差，DPI 在绘制根部处理。

例如 300 宽的行中放置 80 宽固定内容、8 gap 和一个 grow=1 的输入区，输入区最终收到 212 宽约束并据此换行／截断。不能先按无限宽排版，再只压缩 Row 自己的返回尺寸。

如果子项最小尺寸之和超过父约束，保留明确的最小尺寸及 overflow 结果，由容器选择裁剪或滚动；不能偷偷破坏 min 约束，也不捏造能同时满足矛盾约束的布局。普通 Row／Column 默认配置和 builder 在同一批迁移，删除旧的无界主轴默认算法。

## 3. Wrap 与 Grid

Wrap 按声明顺序生成行／列，有限主轴空间不足时换行；支持项间距、行间距、行内主轴对齐和交叉轴基线。单个过大项按自身可用约束重新测量，超出最小尺寸的情况沿用显式 overflow 规则。文本内部换行仍由 graphics.text 负责。

Grid 提供当前控件需要的有限轨道模型：固定长度、Auto、Fraction，以及显式 row／column 和 span。轨道和位置在配置构造时检查；不引入 CSS selector、任意模板字符串、subgrid 或布局约束语言。

- 先确定固定轨道及 gap，再用实际已提供子项测得 Auto 的内容需求，最后分配 Fraction。
- 跨轨项以其覆盖轨道的总尺寸测量。有限 Fraction 分配父级剩余空间，不由自然文本宽度撑大；仅含 Fixed／Auto 的跨度把剩余内容需求等分给 Auto，无界 Fraction 按权重参与。先处理单轨再处理较长跨度，最后按确定列宽重新测量高度。
- 日历可以使用七个等份列；普通表格可以由同一 Grid 管理表头和单元格，共享列宽。
- 虚拟表格不能为了 Auto 列宽扫描或挂载全部数据。为它提供显式的共享列尺寸值，由表头、用户指定宽度或应用选取的样本确定；表头和虚拟行消费同一份列尺寸。全量数据自动估宽属于后续表格策略。
- 不包含表格排序、筛选、分页、列重排或二维虚拟化；这些属于后续控件。

## 4. 方向继承与逻辑几何

新增 UI 的 `LayoutDirection.Ltr／Rtl` 与单内容 Direction 作用域。根默认 Ltr，局部显式覆盖。它沿挂载树解析，portal 继承逻辑祖先，不能从显示位置或系统 locale 猜测方向。

当前 task-local Context 在挂载时绑定，不能通过临时绑定一个值来实现动态树内 Direction。采用专门的方向继承：节点保存自身覆盖和有效方向，方向变化使实际继承该值的子树重新更新／布局；遇到显式覆盖时停止向下传播。保留 key、焦点、编辑状态和资源，不靠换 key 重建树。不借此建立通用动态环境／任意属性依赖系统。

公开的方向值供容器、弹层、导航和主题共同使用：

| 项目 | 规则 |
| --- | --- |
| 物理位置 | Left／Right／Top／Bottom 继续表示画布物理方向 |
| 逻辑边距和边框 | Start／End 通过有效方向解析；Top／Bottom 不变；逻辑和物理配置以不同值类型表达，避免同一字段有两种含义 |
| 水平布局 | 按逻辑起点排列；声明顺序与 keyed 身份不变；Tab 顺序仍按逻辑结构 |
| 弹层 | 横向 Start／End 对齐及新增逻辑侧受方向影响；上下定位及物理 Left／Right 请求不被重写 |
| 导航 | ArrowLeft／ArrowRight 是实际按键，由布局方向决定“前／后项”；菜单开／关子层使用相同解析 |
| 文本 | Paragraph 的 TextDirection 仍决定 Unicode bidi；组件默认可取布局方向，但显式文本方向独立生效 |
| 图形 | 不整体镜像 Canvas、文本或图片。只对声明为方向性的箭头等内容做显式变换 |

范围值的增加方向和视觉方向同样分开。水平 Slider 在 RTL 下可以从右向左递增，纵向范围按自身约定；拖动计算和键盘增减消费同一方向配置。

## 5. 边框、圆角和公共轮廓

Style 从一个 border／cornerRadius 演进为四边边框和四角圆角。共享的 `CornerRadii` 属于 graphics 几何；统一圆角可以用一个值构造四角，独立圆角不必手写 Path。StyleOverride 的继承／清除／显式设置规则保持。

每个边框有有限非负宽度和颜色，边框仍画在轮廓内部且不自动增加布局尺寸。相邻圆角之和超过边长时按统一比例缩小，形成确定的合法外轮廓；零尺寸、边框比盒子更厚等情况也必须给出有界几何。

背景、边框、内容 clip、阴影和控件自绘的 focus ring 共享解析后的轮廓。不同边宽的内轮廓和拐角接合统一计算，不能分别画四条互相覆盖的线，导致圆角缝隙或透明度加深。命中仍通过公开的区域形状／clip 规则声明，不由视觉背景自动制造 InputArea。

逻辑边框／逻辑圆角先按 LayoutDirection 解析，再进入纯物理几何函数。graphics 不依赖 UI 方向或主题。已有三套主题、Style builder、Button／Checkbox／Input／Textarea 同步迁移，不保留旧新两套装饰字段。

已有渐变、Path、clip、saveLayer 和 BlendMode 足以实现进度、shimmer、边缘淡出和大部分图表绘制。本轮只补实际公共几何，不以 SkSL、通用特效图或 runtime SVG 解析为前置。

## 6. Image 与 Path 内容适配

在 component 提供无交互的 Image／Path 展示适配，支持明确的逻辑自然尺寸或 viewBox、Contain／Cover／Fill／None、对齐及裁剪。像素尺寸与逻辑尺寸的关系由输入说明，DPI 只在根部施加一次；不能把高分屏缩放重复应用到位图或路径。

适配器借用调用方拥有的 Image／Path；不自动关闭输入资源，也不跨挂载保存已关闭的资源。自建的裁剪／轮廓缓存由自己的 scope 管理。替换内容后只重建确实变化的缓存，失败时沿已有资源规则收尾。

异步加载继续使用已有 ComponentScope 任务和应用的数据状态。公共展示适配不依赖 HTTP client、不自行访问文件系统，也不增加通用资源加载插件。Avatar／Attachment 的加载中、失败、上传状态和重试按钮留给控件组合。

图标可由共享 Path、位图或后续构建期资源生成取得。图标集的外观选择属于 shadcn 阶段，本轮只保证展示和几何能力。

## 7. 验收重点

- 同一子项两次不同约束测量、相同约束复用、放置后拒绝测量、失败时无旧 placements；嵌套 Flex 不在每帧做无意义的重复探测。
- lazy 临时探测不 trim 活动 key，最终不可见项按 retention 收尾；虚拟项仍按需创建，有限尺寸视口不枚举全数据。
- 固定＋弹性布局、收缩下限、Wrap 多行、跨轨单元格、基线与不同字体；窄窗口、零尺寸和不可满足 min 的明确 overflow。
- LTR／RTL 热切换、portal 继承、横向导航及局部文本方向；不重置 keyed 状态、不丢焦点或 IME 连接。
- 四角不同圆角、不同边宽、半透明拐角、clip／shadow 一致，图像 fit 及资源替换；使用共享算法和固定测试字体验证几何，不比较两平台系统字体的像素。
- 保留既有 ParagraphHost 行内对象、裁剪、变换、Accepted／Deferred 和 paint cache 回归；最终静止时停止布局和 GPU 提交。
