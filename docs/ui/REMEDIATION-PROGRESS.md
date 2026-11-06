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

## P1：完整测量结果与帧内复用

状态：已完成，2026-11-06；macOS aarch64 release。

- Element／LazyElement 使用 `LayoutResult<L>` 与 `scene(context, layout)`；每次成功测量独立保存类型化数据、子结果引用和放置，根布局轮次按节点、输入版本、约束及方向复用真实请求。父结果共享子结果，不复制整棵布局树；未选中的结果统一释放。
- ParagraphHost、SelectableTextHost、TextEditHost 显式持有对应排版结果，ScrollViewport 显式接收 ScrollLayout。编辑与滚动的公开几何仍只在 Accepted 后发布，试测不会替换可交互状态。
- Lazy 每次测量保留独立选择，最终结果决定保留范围和呈现；迁移全部既有组件、测试与示例，移除 measure 写入最近一次 drawing／geometry 后供 scene 读取的通路。
- 新增实窗回归覆盖 A→B→A、失败约束、共享子结果、段落及编辑布局、滚动快照、一次释放和一亿项惰性源的局部访问。Carousel 根据最终选择更新离屏页的输入资格，避免不可见按钮获得焦点。

与 P0 相同构造的实际计数：

| 深度 | Auto Grid 叶子测量：P0 → P1 | Grid 测试探针总调用：P0 → P1 | P1 不同探针请求 | Frame 叶子／探针总调用 |
| --- | ---: | ---: | ---: | ---: |
| 1 | 3 → 3 | 4 → 4 | 4 | 1／2 |
| 3 | 27 → 3 | 40 → 10 | 10 | 1／4 |
| 6 | 729 → 3 | 1,093 → 19 | 19 | 1／7 |
| 10 | 59,049 → 3 | 88,573 → 31 | 31 | 1／11 |

每个探针的调用数等于其不同约束数，Grid 的该固定构造随深度线性增长。深度 10 Grid 本次测量为 0.072ms；与 P0 一样，仅记录这次运行的观测，不作为稳定性能基准。

已通过：195 个 Scoop 文件格式检查、ui／component 生产及测试检查、style 与四套主题构建、53 个受影响测试／示例 Cone 检查；UI 普通测试 99 项、component 普通测试 161 项。UI 中创建 Application 的测试分别用独立进程执行，闭合一次进程内创建第二个 Application 的测试组织限制。Download 示例依赖的已有预编译库也已按当前 runtime 对齐，并修正 ColorPair 调用；未下载依赖或重建编译器。

16 个关键实窗回归全部通过：自绘、段落、原生编辑／IME、Lazy、滚动、错误清理、Flow／控件布局、OTP、固定及可变虚拟列表、保留项、Carousel、Select、模态组件族和 Sidebar。覆盖最终布局的输入／焦点几何、动画停止、隐藏与恢复、拖动、菜单和资源生命周期。本批未重复运行无关全量验收。

本批只提供帧内复用。跨帧失效、输入比较、构造作用域、静态场景与窗口错误隔离仍属于后续 P2～P6，不把本批结果当作它们的验收。


## P2：跨帧布局与失效分类

状态：已完成，2026-11-06；macOS aarch64 release。

- `UiInvalidation.None／Scene／Layout` 与 `ComponentScope.requestLayout()` 区分像素更新和真实布局变化。根保留当前完整布局结果，未失效的节点直接复用；祖先布局变化不重测有效兄弟。
- Surface 动画只增加同步读取登记与 revision，仍使用原时间轴。布局和场景分别保存实际读取；共享 FontCollection 在已请求帧入口检查一次 revision，不增加周期帧。
- ActionInk、Frame、Checkbox／Radio／Switch、SliderThumb、ItemInk、InputGroup 和 ResizableHandle 将当前颜色、交互采样与绘制几何放到 scene。Progress、Spinner、Skeleton 和文本宿主按真实输入分类更新；普通 ScrollArea 的偏移在 scene 应用，虚拟列表继续按偏移重新选择布局。
- 待聚焦开始、切换或结束时失效相关 Lazy 布局，保留项准备与淘汰不再依赖每帧全量测量。首个 Accepted 后才启动的滚动动画也按视口用途失效，闭合初始结果尚未读取动画时的复用遗漏；两个问题均由原有实窗回归检出并修复。

定向结果：`UiFrameReuseTest` 覆盖纯颜色、布局动画、字体 revision、显式布局请求、滚动和约束变化；稳定子项只测量一次，变换后的输入位置与最终布局一致。`ComponentFrameReuseTest` 在 Spinner、Progress、Checkbox、Switch、滚动和编辑光标组合中观测到 23 个场景帧，初始稳定后新增 measure 为零，并验证 Spinner、Progress、Checkbox、Switch 和光标的实际像素发生变化。该帧数是本次实际观测，不作为固定性能指标。

已通过 Surface 动画 13 项测试，以及 UI 组合、描述身份、完整布局结果、焦点和方向回归。111 个变更 Scoop 文件格式检查、相关生产／测试静态检查和另外 7 个调用方 Cone 检查通过；受 UI 语义指纹影响的 style、四套主题及 component 已重建对齐。23 个既有实窗 Cone 和 4 个独立计数／协议 Cone 全部通过，包含 Flow、控件布局、OTP、虚拟列表、保留项、Carousel、Select、模态组件族、Sidebar、InputGroup、Resizable、Disclosure、绘制缓存和 Slider。绘制缓存保持像素、输入和裁剪断言，并在最后一个 Accepted 帧后验证完整静止区间。

新增实窗回归分为独立 Cone，避免同一进程重复创建 Application，也避免每次定向复验都编译整个普通测试集。拆分后的 UI 普通测试 97 项、component 普通测试 161 项整组通过；Grid／Frame 的深度计数继续符合 P1 的线性结果。本批仍不包含 P3 输入比较、P4 构造作用域或 P5 场景回调跳过。

## P3：逻辑更新与订阅关系

状态：已完成，2026-11-06；macOS aarch64 release。

- Component／Element／LazyElement 描述提供稳定的 `InputComparison<P>`。当前 Props 始终先替换；自身 State 或方向变化强制求值，Element children 和 Lazy source／retention 独立处理，父级跳过不清除后代排队工作。默认输入仍保守更新，同引用 State 写回仍通知。
- Label／Button 的便利入口比较实际显示输入；不透明 View 内容保持保守。Button 使用读取当前 Props 的稳定点击转发，builder 共享不可变默认样式；交互与主题 State 保留各自更新。
- 依赖按 StateSignal 的类型化序号收集，成功 render 差量更新订阅。登记保存位置，解除时交换末项并清除引用；稳定关系复用原登记，失败 render 不发布半套依赖。

29 个变更 Scoop 文件格式与相关静态检查通过，预编译 UI、style、四套主题及 component 已对齐。UI 普通测试 100 项、component 普通测试 162 项，以及 9 个实窗 Cone 全部通过：新增输入复用和 Button 回调／Link 悬停回归，既有 Lazy、方向、描述身份、错误清理、控件交互、Sidebar 和 Select。

组合测试直接验证稳定登记身份、切换后仅移除失效关系和卸载后的引用清除；实窗覆盖 256 个共享 State 观察者、父子同轮更新、独立后代更新、同引用可变输入、Binding 换绑、规则替换、方向、children 重排和 Lazy source 替换。Button 仅换回调时没有新增布局阶段的主题尺寸查询，实际点击调用新动作；Link 悬停像素变化并正确恢复。新旧主题探针区分 measure 与 scene，保留场景几何对应绘制次数的精确断言及原有像素、焦点、资源与停帧检查。本批未改变场景查询策略，未重复无关平台或全量 fixture 验收。

## P4：构造作用域与默认字体

状态：已完成，2026-11-06；macOS aarch64 release。

- Component／Element／LazyElement 改为持有 ComponentScope 的公开抽象基类，全部工厂接收本次 scope。输入区域、控制器和显示资源可以在构造中登记，onMount 保留为无参数的业务挂载通知；移除仅为挂载后发布 scope／control 而存在的 State。
- 实际实例保存单次挂载归属，删除全局 ActiveInstances 表；重复返回同一个实例、返回其他作用域的实例，以及卸载后再次返回均在实例自身拒绝。构造失败释放已经取得的资源，不调用尚未完成挂载的业务卸载通知，原始错误与清理错误均保留。
- UiHost.attach／runUiApplication 接受可选默认字体，UiWindow.defaultFonts 可以替换；ComponentScope.defaultFonts 供实际消费者读取。默认集合由根统一拥有，外部集合只借用；替换默认值及字体 revision 只使实际依赖者更新，显式覆盖保持独立。旧的自有 fallback 在后代资源退休后关闭。
- Label、可选择文字、Input／Textarea／OTP 和已有组件／示例统一消费根默认字体。TextEditHost 在实际呈现的 Element 构造中取得，保留视口、输入和布局失效的同一归属；组合组件只通过真实交互通知保存外观值，不发布宿主引用或为默认交互值补渲染。

359 个 Scoop 文件格式检查通过，UI、style、四套主题及 component 已重建对齐。受影响的 63 个 Cone 静态检查通过，其中 58 个调用方 Cone 包含 61 项生产／测试入口检查。UI 普通测试 100 项、component 普通测试 162 项通过；20 个相关实窗 Cone 已闭合，涵盖新增挂载／字体、错误清理与实例拒绝，既有描述、Lazy、编辑、控件交互、选择组、菜单、OTP、Command／Combobox／Select、Sidebar、Slider、Disclosure、Tabs、Carousel、Drawer 和帧复用。

挂载测试包含 64 个 Component、64 个 Element 和 64 个 LazyElement，首轮只执行各自所需 render，193 个计数资源各释放一次，并验证构造期 LazyFocus。字体测试覆盖两个借用窗口、根来源替换、集合 revision、显式覆盖及持有布局的释放顺序。编辑回归检出并闭合一次宿主放在外层 Component 导致 ScrollViewport 归属错误的问题；修正后原生编辑、焦点、滚动、IME、隐藏恢复与纯绘制帧的断言均通过。未重建编译器、下载外部依赖或运行其他平台／全量 fixture。

## P5：场景与图层复用

状态：已完成，2026-11-06；macOS aarch64 release。

- `cachedScene(content, revision)` 通过普通 Element 建立显式边界，不增加 ViewBody 分支。每份有效布局保留稳定的局部场景操作，每帧按当前祖先变换、clip、顺序和输入状态重新装配；外层 revision 变化也使嵌套边界失效。原生几何仍只在 Accepted 后发布。
- 直接 draw、借用 Path／ImageFilter、滚动视口、overlayLayout、时间／动画及帧完成通知继续执行对应场景工作；稳定子项可以独立复用。没有保存借用上下文或一次性回调，也没有通过持续帧检查缓存。
- 有限 drawLayer 在 None 策略下保留完整 Drawing，Tiles 独立持有可见范围的像素。缩放／缓存参数变化仅替换瓦片，内容版本及后端不兼容才替换记录；Clear／Src 独立合成语义不变。记录回调的已登记时间、动画和字体依赖随命中继续保留。公共文字在无缓存需求时直接 draw，在显式边界／Tiles 内使用自身排版版本录制。
- shadcn Gallery 接入缓存边界。现有布局、编辑／IME、Portal、滚动缓存和组件动画实窗测试也在边界内执行，继续验证当前交互、像素及静止停帧。

新增 `UiSceneCacheTest` 的本次运行包含 12 个实际帧、两枚保持挂载身份的静态叶子。每枚叶子仅执行 3 次 scene（首帧、隐藏后恢复、外层边界版本变化）和 2 次绘图录制（首帧及隐藏后恢复），单纯平移、裁剪、顺序和禁用变化复用记录。测试同时核对输入位置、锚定 Portal、嵌套边界、动画依赖命中后变化、当前帧完成通知，以及一次 Discarded 不发布候选几何。既有组件组合观测到 23 个场景帧，初始布局后新增 measure 为零，实际动画与光标像素仍变化。

另以 512×256 RasterSurface、512 个矩形、每项预热 5 次后执行 200 次，分别观测以下操作成本；这是 macOS release 的单次离屏 CPU 样本，没有 UI 挂载，不代表真实窗口帧时间或 GPU 分配成本。重放包含独立图层合成，光栅项包含离屏创建、绘制、snapshot 和关闭。

| 操作 | 本次平均耗时 |
| --- | ---: |
| 录制 512 个矩形 | 173.3µs |
| 隔离重放保留的 Drawing | 226.8µs |
| 离屏光栅化并取得 snapshot | 179.7µs |
| 合成已有图像 | 34.8µs |

24 个变更 Scoop 文件格式检查、9 个相关 Cone 静态检查通过，UI、style、四套主题及 component 预编译库已对齐。12 项绘图测试全部通过，覆盖记录与瓦片的独立复用／退休、缩放、滤镜、字形外扩、失败替换、Clear／Src 和 Metal 后端；6 个相关实窗 Cone 全部通过。旧 Portal 测试的“每帧必须重新测量”断言与 P2 合同冲突，已改为核对选中布局、场景尺寸与出口 bounds 的一致性，原有像素、输入、焦点、可见性和关闭断言保持。未重复无关全量测试，未重建编译器或下载依赖；P6 的动态模式与窗口错误隔离继续下一批。
