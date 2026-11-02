# UI 基础交互实施记录

日期：2026-11-03。范围按 [总设计](DESIGN.md)，本期实施 macOS；Windows 后续。各批只验证 aarch64，全部功能完成后再统一执行 macOS x86_64 验证。沿用现有普通 Scoop Cone、单一宿主事件循环和按需呈现。

## 批次状态

| 批次 | 状态 |
| --- | --- |
| A1：场景几何与纯绘图块分离 | 已完成 aarch64 实施和定向验证 |
| A2：帧完成通知 | 已完成 aarch64 实施和定向验证 |
| A3：输入作用域与路由阶段 | 已完成 aarch64 实施和定向验证 |
| B1：作用域快捷键 | 已完成 aarch64 实施和定向验证 |
| B2a：可见几何、焦点作用域与 Tab | 已完成 aarch64 实施和定向验证 |
| B2b：组内键盘导航与 roving | 已完成 aarch64 实施和定向验证 |
| C1：Absolute 与显式子项呈现顺序 | 已完成 aarch64 实施和定向验证 |
| C2：portal 与 anchor 定位 | 已完成 aarch64 实施和定向验证 |
| C3a：输入边界与按下预处理 | 已完成 aarch64 实施和定向验证 |
| C3b：同窗口模态、关闭与焦点恢复 | 已完成 aarch64 实施和定向验证 |
| C3c：宿主快捷键 | 已完成 aarch64 实施和定向验证 |
| D1a：滚轮残量与连续手势路由 | 已完成 aarch64 实施和定向验证 |
| D1b：ScrollArea、滚动条与显式平滑滚动 | 已完成 aarch64 实施和定向验证 |
| D2a：按需子项与固定 extent 虚拟列表 | 已完成 aarch64 实施和定向验证 |
| D2b：可变 extent、阅读锚点与按 key 定位 | 已完成 aarch64 实施和定向验证 |
| D2c1：窗口所属的一次性定时回调 | 已完成 aarch64 实施和定向验证 |
| D2c2：动画帧需求与离屏完成 | 已完成 aarch64 实施和定向验证 |
| D2c3：有限实例保留与子树帧需求 | 已完成 aarch64 实施和定向验证 |
| D2d：虚拟焦点导航与滚动显露 | 待实施 |
| E：绘制记录与分块像素缓存 | 待实施 |
| F：公共编辑与 Input／Textarea | 待实施 |

每个完成的变更提交并推送，然后合并最新 origin/main。普通行为使用相关 Cone 的 scoop.test，不运行全量测试；遇到编译器或底层库缺口先在对应层修复。本次工作树使用独立的开发 sysroot 和构建缓存，原始构建输出保留在仓库外，临时文件在每批结束时清理。

## A1：场景几何与纯绘图块分离

Element 采用 `measure → scene`；SceneContext 控制共同的变换、clip、图层、输入区域和子项呈现，draw 块只借用隔离的 PaintContext／Canvas。场景操作同步消费，不保留跨帧借用。StylePainter、ParagraphHost、现有控件、相关示例及测试已直接迁移。尚未提供绘制缓存，省略 draw 的回归只验证输入不依赖像素块执行。

格式化及 UI／Style／Component、自绘示例的类型检查通过；相关六个 UI 库完成 aarch64 release 构建。aarch64 的 scoop.test 结果：

| 验证范围 | 结果 |
| --- | --- |
| UI 核心、绘制、错误与公共段落 | 12 项通过 |
| Style 缓存、Raster／Metal 像素、UI 切换与异常清理 | 13 项通过 |
| Component 布局／样式、真实交互与容器、主题展示、下载窗口集成 | 13 项通过 |

新增回归覆盖绘图 Canvas 变换不影响随后区域、跨作用域借用失效、状态写入拒绝、异常状态恢复，以及未放置子项恢复后跳过绘图仍能点击。现有停帧、焦点、滤镜、资源释放和后台任务组合保持通过。首次绘制回归因 macOS 后台激活未取得真实焦点而超时，按既有焦点前提重跑通过，未放宽焦点断言。没有运行 fixture 或全量测试，下载示例只运行 DownloadWindowTest。

macOS x86_64 测试尚未执行，留到全部功能完成后统一验收。一次性测试报告由 runner 清理；后续批次仍需使用的独立 sysroot、编译产物和缓存保留在仓库外。

## A2：帧完成通知

SurfaceContent.onFrameComplete 接收本帧 time、窗口内容区 size 及 Accepted／Discarded。通知在 Canvas 和 frame 资源结束后交付，Accepted 的区域几何已经发布；Deferred、未绘制、异常或关闭路径不发通知。完成回调可请求下一帧和焦点，回调中的关闭仍遵守延迟释放，静止时不产生新工作。

SceneContext.onFrameComplete 将一次性动作绑定到当前挂载与当前帧；UiRoot 按登记顺序交付，节点退休或根关闭会取消后续动作。回调允许正常 State 更新，不能继续使用结束的绘制借用。绘制回归已把恢复子项后的聚焦移到此公开入口，直接验证提交边界可供焦点接入使用。

Surface／UI 格式化、生产类型检查和 aarch64 release 构建通过。5 项定向 scoop.test 通过：SurfaceHostTest、Surface content／errors、UI drawing／errors。覆盖真实改尺寸导致 Discarded、成功几何可立即聚焦、完成回调请求后续帧、回调内关闭、绘制失败不通知、完成动作异常后的清理及借用失效。Surface 主 Cone 只筛选 SurfaceHostTest，未运行无关用例；没有 fixture、全量或 x86_64 验证。测试临时报告已由 runner 清理。

## A3：输入作用域与路由阶段

Surface 增加独立 InputScope，InputArea.scope 复用同一逻辑路由节点；parent 由挂载关系确定，frame.place 只登记几何。窗口根在没有区域焦点或鼠标命中时仍可接收输入。UI 的 ComponentScope.input 按需连接挂载祖先，routeChildren 接入容器区域；SceneContext.children 和 ParagraphHost.scene 不再传输入父区域。

处理器采用 RoutedEventContext／InputEvent，按 capture、target、bubble、default 执行；stopPropagation、preventDefault 和 consume 分别表达传播与默认行为。回调使用冻结路径和坐标、最新 handler，退休节点被跳过；状态、IME 与取消通知保持完整。onEventFinished 为被拦截的释放提供不可取消的收尾。Button／Checkbox 的激活移入默认阶段，释放被阻止仍清除按下状态；自绘和 Surface 示例同步迁移。

变更文件格式化通过，相关七个库完成生产类型检查和 aarch64 release 构建；两个示例的生产类型检查通过。37 项定向 scoop.test 通过：Surface 输入 21 项、原生 IME 输入 1 项、UI 核心／绘制／错误／公共文本 12 项、Style UI 1 项、Component 交互／布局 2 项。新增覆盖精确阶段顺序、取消标记独立、无几何作用域、空白按下不改投、parent 与绘制次序独立、环与跨窗口错误、最新 callback、关闭／禁用后的完整 Cancel、借用失效及异常后的收尾。真实控件回归覆盖父级取消点击、单独取消鼠标／Space 释放、状态恢复和后续正常激活。

没有修改编译器或 core／std／test，没有 fixture、全量或 x86_64 验证。一次性报告已由 runner 清理，后续仍需使用的 sysroot 和构建缓存保留。

## B1：作用域快捷键

macOS 提供 ShortcutScope、ShortcutBinding、ShortcutKey 和 ShortcutModifiers。绑定随 InputScope 的默认阶段执行；同节点普通默认行为先执行，随后匹配快捷键，未执行则继续祖先。InputScope.addDefaultAction 提供可注销的行为登记，登记快照和独立上下文借用复用现有路由。UI 使用 ComponentScope.resource 管理 ShortcutScope，自绘 Counter 已接入 ⌘I。

logical Character 的无 locale 大小写及规范等价比较由 Window 调用 CoreFoundation 完成，保留重音区别，不在 Surface 手写 Unicode 表。完整修饰组合、logical／physical、repeat／synthetic、disabled／canExecute、配置冲突和编辑目标优先级均有覆盖。绑定整体更新；更新冲突保留旧配置，同一真实键事件匹配多条可执行规则时报歧义。

本批交付输入节点和窗口级快捷键；应用宿主 fallback 与模态截断在模态批次接入，文本编辑命令在公共编辑批次接入。

修改的 Scoop／Objective-C 文件格式化通过，相关八个库完成生产类型检查和 aarch64 release 构建，自绘示例类型检查通过。32 项定向 scoop.test 通过：Window Unicode 比较 1 项、Surface 输入 28 项、原生 IME 1 项、原生快捷键 1 项、Component 交互 1 项。独立 consumer 检查发现的公开构造器遗漏已修复；capture 回归使用真实子焦点目标，原生窗口用例按每进程一个 Application 分置测试 Cone。

Component 首帧正常但后台 activate 被 macOS 拒绝，准备期保留一次首帧后的原生聚焦请求；最终按仓库既有桌面测试说明通过临时应用包从前台运行同一 aarch64 测试程序。scoop.test 报告记录无失败并完整结束，未伪造焦点或放宽断言。没有修改编译器或 core／std／test，没有 fixture、全量或 x86_64 验证。一次性报告清理，独立 sysroot、构建缓存和后续桌面验收仍需使用的应用入口保留在仓库外。

## B2a：可见几何、焦点作用域与 Tab

CanvasGeometry 提供窗口包围盒与经过任意 path clip、视口裁剪的可见包围盒。路径求交复用 Skia PathOps；固定版本的现有最小 Skia 构建没有链接该 source set，本批补齐其直接依赖并重建 aarch64 资源。Surface 的新聚焦请求要求非空可见几何，已持有的焦点和 IME 会话可以在仅被 clip 裁掉时继续；真正移除、禁用或不可逆变换仍按原规则清理。每窗口 focusRevision 区分更新的显式焦点意图与 OS 激活变化。

UI 增加随挂载管理的 FocusTarget、FocusHandle 和 FocusScope。Tab 使用当前逻辑挂载顺序和显式 order，支持普通域向父继续、局部循环与窗口循环。待放置目标等到 Accepted 后聚焦，且先于用户帧完成动作执行；更新的请求、卸载、句柄替换和关闭取消旧意图。Button／Checkbox 公开 FocusOptions。组内箭头与 roving、滚动显露、模态限制由后续对应批次接入。

Scoop／C++ 格式化、Python Ruff 格式化与 lint、相关库生产类型检查和 aarch64 release 构建通过，自绘与主题示例类型检查通过。47 项定向 scoop.test 通过：Graphics 几何 3 项、Surface 输入 29 项、UI 核心 9 项，以及 UI focus／drawing／errors、Component interaction／layout、原生 IME 各 1 项。窗口用例通过前台应用入口串行运行原 scoop.test 程序，报告无失败并完整结束。

新增覆盖任意路径孔洞、变换与视口可见性；裁剪后的焦点和 IME 保持；负 order、keyed 重排、嵌套循环、Tab 取消／重复、禁用／隐藏跳过；首次提交聚焦、后续同区域请求取消旧意图、句柄替换、区域提前关闭后重新绑定和卸载收尾。没有修改编译器或 core／std／test，没有 fixture、全量或 x86_64 验证。一次性桌面报告已清理，后续所需的独立 sysroot、构建缓存、Skia 构建目录及前台应用入口保留在仓库外。

## B2b：组内键盘导航与 roving

KeyNavigation 按公开 NavigationOptions 的成员数组或 Grid 行列配置处理方向键、Home／End、循环和禁用跳过，横向映射复用 TextDirection。roving 默认只让一个有效成员进入 Tab；显式 tabStop=false 仍有效。组内默认动作沿既有路由执行，内层消费优先，到达边界也消费已认领的方向键，不修改选择状态或触发 Enter／Space。

Surface 在成功聚焦时记录 InputArea.focusRevision；组内历史直接使用这个记录，涵盖鼠标和直接区域请求，无额外观察处理器。成员配置在变更时检查重复句柄、跨组归属和网格冲突，先登记的未挂载句柄在绑定时检查逻辑归属；更新失败保留旧配置，关闭解除成员关系。主题展示的主题／明暗按钮使用同一导航助手，箭头只聚焦，Enter／Space 才应用选择。

Scoop 格式化、相关库与主题示例生产类型检查、aarch64 release 构建通过。10 项定向 scoop.test 通过：Surface InputFocusTest 7 项，UI navigation、既有 UI focus 和 ShowcaseWindowTest 各 1 项。导航回归覆盖稀疏网格、RTL、Home／End、循环、边界消费、不可见／禁用跳过、鼠标和直接请求的历史、tabStop／roving、配置失败、成员更换及关闭；示例实际执行 Tab／方向键／Enter，随后完成主题切换与停帧检查。主题 Cone 通过测试程序的 filter 只运行窗口组合用例。

桌面入口曾因临时应用替换后的签名失配未能启动，改为新文件替换并重新签名整个测试应用后重跑通过。示例的关闭回归发现准备期焦点重试查询已销毁窗口，已限制到准备阶段；相关焦点／导航测试也在关闭后停止重试，三个窗口用例重跑全部通过。没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告清理，后续所需构建缓存和测试应用入口保留。

## C1：Absolute 与显式子项呈现顺序

SceneContext.children(order) 接受直接子项索引的完整排列，在开始呈现前检查参数，与无参入口及 withState 共用一次呈现计数。Absolute 使用普通 Element 测量协议：先确定受父约束收敛的有限容器尺寸，再按 Start／End／Center／Stretch 测量和定位每个子项；无界轴要求明确尺寸，允许负偏移，不隐式 clip。zIndex 在 Props 逻辑更新时排序，只改变实际场景和重叠命中，保留 keyed 挂载与 Tab 顺序。padding 校验所得值随 Props 复用。

修改的 Scoop 文件格式化、相关六个 UI 库生产类型检查及 aarch64 release 构建通过。5 项定向 scoop.test 通过：AbsoluteLayoutTest 3 项、UI drawing 1 项、AbsoluteUiTest 1 项。纯布局覆盖有界／无界、显式尺寸收敛、四种轴策略、负偏移及非法／溢出参数；实际窗口读取叠放像素并派发鼠标／Tab，验证每项每帧只测量和呈现一次、zIndex 同值稳定、Props 更新保留实例与焦点顺序、祖先裁剪仍限制像素和命中。既有绘制回归验证无效排列不消耗呈现机会、跨 withState 仍只呈现一次。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告清理，独立 sysroot、构建缓存和后续测试入口保留。

## C2：portal 与锚点定位

Portal 成为独立的 View 描述形状，保留逻辑挂载、输入路由和 Tab 顺序，在原容器中只返回占位尺寸，内容延后到窗口层测量一次。LayoutAnchor 使用本帧 graphics 变换／clip 几何，Viewport／Anchored 定位提供有界约束、flip、shift 和匹配锚点宽度；SceneContext.overlayLayout 传递最终位置供内容绘制。层按打开顺序呈现，嵌套层在父层之后；绑定时检查同根、自引用与实际依赖环。

OverlayHandle 管理一次挂载会话，当前回调只收到一次关闭通知。Hide 保留实例等待锚点恢复，KeepVisible 使用当前有效坐标保持在窗口内；Close 和缺失位置在 Accepted 后关闭，Discarded 不误关。显式关闭立即停止子树交互，普通更新随后退休子内容；同 key 的普通更新不重开已关闭会话。锚点解绑、句柄替换和迟到清理按实际绑定身份处理，部分挂载失败也会释放登记。

修改的 Scoop 文件格式化、相关六个 UI 库生产类型检查及 aarch64 release 构建通过。9 项定向 scoop.test 通过：OverlayPlacementTest 5 项，以及 overlay、UI composition、drawing、errors 各 1 项。真实窗口验证祖先变换／裁剪外的像素与命中、嵌套路由、逻辑重排与层次／Tab 顺序、resize、三种可见性策略、帧失效、当前回调、重复关闭、重新挂载和停帧；错误回归新增重复句柄及自引用下的部分树清理，绑定回归覆盖循环与跨根拒绝。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告已清理，独立 sysroot、构建缓存和后续测试入口保留。

## C3a：输入边界与按下预处理

Surface 提供显式 InputBoundary 登记，当前域统一限制命中、capture／bubble／default、快捷键与程序化聚焦；没有区域几何的域根仍接收键盘和空白指针输入。更换域会在当前回调退出后向完整旧路径发送 BoundaryChanged 取消，剩余按键序列不改投背景。关闭 scope 不自动释放屏障，供下一批在层移除成功后解除；嵌套登记支持非当前项先关闭。

InputScope.beforePointerPress 只在当前域根处理一串鼠标按钮的第一次按下，先于 hover 和普通路由。返回值选择消费整串按钮或让冻结的原目标继续，不重新命中；异常同样结束该次捕获，避免孤立的释放事件。UI FocusTarget／FocusScope 使用独立的 isInputAllowed 查询拒绝域外 pending。回调内安装边界后重入的键盘事件回落到新域根，旧焦点的终结通知仍在正常收尾边界交付。

修改的 Scoop 文件格式化、Surface 与相关六个 UI 库生产类型检查和 aarch64 release 构建通过。44 项定向 scoop.test 通过：Surface 输入 41 项，以及 UI focus、既有 portal、原生 IME 各 1 项。覆盖域外祖先缺少几何、所有传播阶段截断、边界嵌套与关闭顺序、旧捕获取消、旧 IME 排队事务、安装失败清理、预处理坐标冻结／消费／异常，以及真实窗口中背景直接和待聚焦请求被拒绝。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告清理，独立 sysroot、构建缓存和后续测试入口保留。

## C3b：同窗口模态、关闭与焦点恢复

PortalOptions 接入 ModalOptions 与 OverlayDismiss。模态会话复用 InputBoundary、循环 FocusScope 和不可命中的根焦点代理；层先按逻辑模态域、再按打开顺序呈现，后来打开的背景层不能覆盖模态。Escape 由域根默认动作处理，内层行为优先，非重复按下一次只关闭一个适用会话。外部点击按逻辑归属判断；非模态可消费整串按钮或让原目标继续，模态只能阻挡或同时关闭。

关闭立即停止交互，屏障保留到实际移除层的 Accepted 帧；Discarded、首帧待显示和已卸载的待移除登记不会提前放开背景。焦点恢复使用保存的有效 handle 与域回退，支持嵌套及非当前层先关闭。Surface.reserveFocusRequest 让 UI 的 pending／被拒绝请求加入同一意图序号；挂载失焦回调、关闭回调中的新请求优先于旧自动聚焦／恢复。回调内关闭刚建立的模态也记录同一次退休，子内容未挂载或资源清理抛错时仍释放屏障。

修改的 Scoop 文件格式化、Surface 与相关六个 UI 库生产类型检查和 aarch64 release 构建通过。16 项定向 scoop.test 通过：Surface InputFocusTest 7 项、UI OverlayPlacementTest 5 项，以及 modal、既有 focus／portal／errors 窗口回归各 1 项。真实窗口覆盖背景捕获取消、Tab 循环、默认根聚焦、子菜单归属、关闭策略更新、普通弹层消费／继续、背景快捷键隔离、两种帧失效、完成回调内关闭、焦点原目标失效、嵌套与不同关闭顺序、立即关闭回调及静止停帧；错误回归增加模态子内容挂载失败和关闭回调失败。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告与已被替代的 C2／C3a 测试输出清理，独立 sysroot、共享构建缓存和当前测试入口保留。

## C3c：宿主级快捷键

HostShortcutScope 在 SurfaceHost 的各窗口根提供同一次默认阶段的最后回退，复用本地 ShortcutScope 的匹配算法。登记可早于窗口附着，新增窗口读取同一份绑定；普通默认行为、本地快捷键、宿主回退采用固定顺序。任意活动 InputBoundary 都截断宿主回退，窗口根本身作为边界时也一样。配置整体更新只校验一次，关闭单窗不取消共享登记，宿主关闭立即停用；canExecute 内关闭或替换登记、打开模态后不会执行旧动作。

UiHost.shortcutScope 返回其 SurfaceHost 的登记，UiHost.close 在停止组件和延迟释放绘制资源之前停用它。多个调用不得同时保留活动登记，旧登记的幂等关闭不会移除后来创建的新登记。

修改的 Scoop 文件格式化、Surface 与相关六个 UI 库生产类型检查和 aarch64 release 构建通过。48 项定向 scoop.test 通过：Surface 输入 45 项、UiHost 登记生命周期 1 项、宿主双窗口原生测试 1 项、已有 UI 模态回归 1 项。双窗口测试覆盖预先登记、局部优先、默认取消、根边界隔离、启停／更新、关闭首窗后附着另一窗口、重新登记，以及动作内关闭宿主后的资源恢复。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告与已被替代的 C3b Surface／UI／modal 测试输出清理，独立 sysroot、共享构建缓存和当前测试入口保留。

## D1a：滚轮残量与连续手势路由

RoutedEventContext.scrollRemaining 与 consumeScroll 在同一默认阶段共享残量，保持原 InputEvent 不变。每层使用自己的 lineStep 和事件开始时固定的区域几何，将实际消费位移换回原 Lines／Pixels 再扣除；Pixel delta 只做一次 DPI 归一。传播取消与默认取消继续独立，Contain 可在本层后阻止剩余默认工作。

连续 phase 和随后的 momentum 使用同一个逻辑路由，离散 Wheel 独立命中，不跟随鼠标按钮捕获。接触结束后保留可能进入动量的路径而不请求帧；源退休、边界切换及窗口关闭用独立 ScrollCancelled 结束旧滚动，剩余 Changed／momentum 不改投背景。边界切换的内部序号覆盖同一回调内打开又关闭的情况；终结通知按旧路径完整清理，即使一个处理器抛错也继续通知祖先。

修改的 Scoop 文件格式化、Surface 与相关六个 UI 库生产类型检查和 aarch64 release 构建通过；新增原生注入辅助通过 arm64 Clang 的警告检查。58 项定向 scoop.test 通过：Surface 输入 55 项、原生滚动 1 项、UI drawing／modal 回归各 1 项。覆盖不同容器行高、缩放与旋转、事件内发布新几何、借用与非法消费、零位移、模态与目标退休、按钮并行、取消异常，以及真实 AppKit 行／像素／动量传递。原生测试首轮将无关联 NSWindow 的 CG 事件按屏幕坐标注入，修正为直接 view 投递所需的窗口坐标后通过；测试关闭不再用附带断言遮住原始失败。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。已闭合的一次性桌面报告与被替代的 C3c Surface／modal 测试输出清理，独立 sysroot、共享构建缓存和当前测试入口保留。

## D1b：ScrollArea、滚动条与显式平滑滚动

UI 提供可观察 ScrollState、ScrollSnapshot、ScrollAlignment 和随挂载管理的 ScrollViewport。布局只生成本帧快照，Accepted 才发布范围和采样；较新的定位意图、绑定替换及关闭不会被旧帧覆盖。初始位置和 reveal 等待首个范围，外部 State 在解绑后保留位置。显式 animateTo 复用 Surface 时间轴，可在首布局前排队，取消停在最后发布的 offset，范围收敛后停止帧需求。

ScrollArea 支持单轴／双轴、一次内容测量、同一像素与命中裁剪、嵌套残量及 Contain；纯偏移不重建内容描述。覆盖式滚动条复用三套主题的几何与绘图，支持 thumb 拖动、轨道翻页和可选箭头；无可拖动距离时保持位置，释放被拦截仍结束拖动。条的点击保留子项焦点，视口键盘翻页只处理自身目标。Surface 增加 pointerPosition，以已提交几何更新未移动指针的局部位置，供条在容器移动后刷新 hover。

修改的 Scoop 文件格式化、Surface 与相关六个 UI 库生产类型检查和 aarch64 release 构建通过；桌面启动辅助通过 Ruff 格式化与检查。13 项定向 scoop.test 通过：Surface 指针查询 1 项、UI ScrollState 5 项、Component 滚动布局与三主题几何 4 项、UI 视口窗口 1 项、ScrollArea 原生交互 1 项、已有 drawing 回归 1 项。窗口覆盖真实 Discarded、同一绑定往返替换、布局后的新请求、首布局前动画、范围缩小与停帧、不同 lineStep 的双轴交接、Contain、默认取消、键盘／焦点、拖动与被阻止的释放、主题切换、reveal 和内容挂载计数。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告、启动脚本及被替代的 C3c UI／D1a Surface／drawing 测试输出清理，独立 sysroot、共享构建缓存和当前测试入口保留。普通容器已交付；按需子项、虚拟列表、实例保留及绘制缓存继续按后续批次实施。

## D2a：按需子项与固定 extent 虚拟列表

UI 提供 ItemSource、LazyElement 与 LazyMeasureContext，只有实际请求的 key 才创建条目子树。条目复用普通 Component 的依赖、Props 和资源清理；受控准备边界允许挂载回调写 State，其他逻辑失效留到下一轮，父子测量继续保持纯度。source 替换按已实现 key 更新索引、退休删除项和改变形状的内容；没有 drawable 时也执行逻辑清理。布局期间显式关闭根会终止装配并完整释放已取得的资源。

Surface 的 InputScope.activity／observeActivity 提供实际输入归属。焦点、按下、文本输入、滚动路径及 UI 待聚焦请求所属的条目临时保留并继续放置；普通离屏条目直接卸载。接触结束到 momentum 之间保留滚动候选，不产生连续帧；归属改变或待聚焦失败会请求必要的新布局释放条目。

VirtualList 使用固定 extent、Long 索引与有限 overscan 直接计算范围，首屏不枚举整个 source。纵横轴共用 ScrollArea 的视口输入、滚动条和三套主题；已实现范围相交的 key 保留本地 State，真正退出后重新进入则创建新实例。可变尺寸、实例保留、虚拟导航和绘制缓存继续按后续批次实施。

Scoop 格式检查、Surface 与相关六个 UI 库生产类型检查和 aarch64 release 构建通过。69 项定向 scoop.test 通过：Surface 输入 60 项、UI source 规则 2 项、Component 虚拟范围 3 项，以及 UI lazy、UI errors、VirtualList 和既有 ScrollArea 原生用例各 1 项。错误用例包含 22 种异常清理场景；原生列表覆盖一亿项 source、重排、隐藏删除、指针固定、被阻止的释放、离屏焦点、惯性候选、纵横切换、范围缩小和停帧。测试中的异步断言已按既有逻辑更新边界排队；指针保留用例在 target 阶段取消默认聚焦，单独验证按下归属，最终全部通过。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告与已被替代的 D1b Surface／UI／Component／ScrollArea 和旧 errors 输出清理；独立 sysroot、共享构建缓存、当前定向测试产物及前台应用入口保留。

## D2b：可变 extent、阅读锚点与按 key 定位

VirtualList 使用 ItemExtent.Fixed／Variable，变量尺寸以正估计值和稀疏前缀索引定位，只记录访问过的 key；实际零高度项继续推进索引。每帧只测量一次所需条目，根据新尺寸补足范围；尺寸配置、方向或交叉约束改变时重建尺寸索引，source 替换则按稳定 key 保留初始估计。原有固定列表直接迁移到同一接口和布局路径。

KeepVisible 用最近 Accepted 的首个可见 key 与项内偏移保持阅读位置；删除锚点后选原索引附近的存活项。FollowEnd 只在原先已到末端时跟随。VirtualListHandle 负责按 key 的一次定位请求，使用实际目标尺寸执行 Nearest／Start／Center／End；ScrollState 继续唯一持有位置。新增 ScrollAdjustment 与 requestRevision 把布局修正放入既有视口提交边界，较新的定位、活动动画和绑定变化优先，Discarded 不发布修正或阅读锚点。

Scoop 格式检查、相关六个 UI 库生产类型检查和 aarch64 release 构建通过。17 项定向 scoop.test 通过：UI ScrollState 6 项、Component 固定／可变尺寸与句柄 8 项，以及 ScrollViewport、固定 VirtualList、可变 VirtualList 原生用例各 1 项。可变窗口覆盖一亿项估计、零高度、overscan 中的高度变化、前插／重排／删除、三种显式对齐、末端跟随与向上阅读、未知 key、新请求优先、窄宽度重测、Discarded、完成回调中的新滚动、句柄替换和带 pending 的卸载。视口回归另外验证过期修正和活动动画忽略静态调整。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告与被替代的 D1b 视口、D2a UI／Component／固定列表测试输出清理；独立 sysroot、共享构建缓存和当前测试入口保留。

## D2c1：窗口所属的一次性定时回调

WindowDeadline 复用窗口事件循环已有的单次原生定时器，与应用 WaitUntil 共同选择最近期限。schedule／cancel／close 按窗口 owner 线程执行；到期先退出计划，再排队交付，取消、重排和关闭会废弃旧的排队回调。同期限按登记顺序交付，内部到期只产生 Wake，应用自己的到期才产生 Deadline。隐藏窗口可以执行回调，但不隐式重绘；关闭窗口立即撤销所属回调，不延长 native lease 生命周期。回调沿现有业务 Context 和异常收尾路径执行。

修改的 Scoop 文件格式检查、Window 生产类型检查和 aarch64 release 构建通过。2 项原生 scoop.test 通过：新增 WindowDeadlineTest 与既有 AppKitApplicationTest。覆盖同批取消／重排、过期时间异步交付、线程与时钟拒绝且保留旧计划、用户与内部期限交错、隐藏停帧、窗口关闭、回调异常后的完整收尾及现有输入／IME 回归。该能力供下一批离屏有限动画完成和公共文本 caret 定时使用；本批尚未交付 KeepAlive。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告已清理，独立 sysroot、共享构建缓存及当前测试入口保留。

## D2c2：动画帧需求与离屏完成

Animatable.isFrameEnabled 将是否要求帧与运行／暂停状态分开；关闭帧需求不改起始时间，恢复时直接采样当前进度。Surface 按 hasFrameWork 调度可见帧，通过 WindowDeadline 安排离屏或整个窗口不可绘制时的有限完成。无限重复不安排离屏周期唤醒；有限重复与解析 spring 复用原计划，结束时间向上取整到可表示的纳秒。完成队列、重新启动、暂停与取消仍沿同一时间轴工作，回调中新建的完成留到下一轮；回调内关闭保留原有延迟释放边界。

Scoop 格式检查、Surface 生产类型检查及 aarch64 release 构建通过。12 项定向 scoop.test 通过：动画 10 项、新增原生动画期限 1 项、既有 SurfaceHostTest 1 项。覆盖隐藏／恢复、有限和无限重复、spring、重排、暂停、零时长队列、非法期限与 owner 线程。真实窗口观察 display link 的 required／paused、回调和 present 计数，验证静止期间没有新增帧通知；还覆盖完成回调中关闭和既有多窗口／resize／输入宿主收尾。

合并 main 后同步构建独立开发命令、运行时及 UI 依赖，不修改 main 带入的编译器代码。没有 fixture、全量或 x86_64 验证。一次性桌面报告清理，当前测试产物、配套 sysroot、命令及共享构建缓存保留。

## D2c3：有限实例保留与子树帧需求

lazyElement 与 VirtualList 接受 ItemRetention，缺省 Dispose；KeepAlive 额外保留有限个最近使用的离屏实例，容量为零与 Dispose 相同，可见／overscan 和活动交互项不占容量。缓存项继续处理 Props、State 和资源生命周期，数据删除、shape 改变和容量收紧在隐藏窗口中也会清理。Surface InputActivity 包含仍登记的模态边界，从弹层创建到屏障解除持续固定其所属条目，不依赖首次自动聚焦。

ComponentScope 通过 Surface AnimationGroup 管理子树的帧资格，保留各动画自己的 isFrameEnabled 设置。有限动画离屏按时完成，无限动画与缓存中的局部状态更新不维持 display link；恢复外层条目不会唤醒内层仍未放置的项。帧资格在测量结束后按实际放置结果切换，只测量而不放置也不会反复唤醒。窗口先进入 Closing 时，UI 根让失焦／取消通知完成控件状态清理，并停止向已关闭呈现入口请求新帧。

Scoop 格式检查、Surface／UI／Component 生产类型检查、新增组合测试的测试源码类型检查和相关库 aarch64 release 构建通过。81 项定向 scoop.test 通过：Surface 动画 12 项、输入 61 项、UI source 规则 2 项，以及 UI lazy／errors／scroll、固定／可变 VirtualList、KeepAlive 原生组合各 1 项。原生组合覆盖一亿项 source 的有限构建、实例与资源复用／淘汰、隐藏清理、局部状态与 shape 更新、嵌套缓存、只测量未放置、真实 display link／present 计数、有限完成回调、弹层保留及带焦点关闭。验证中修正了测试绘图借用与异常诊断的用法，并闭合帧资格切换和关闭通知中的帧请求问题，最终全部通过。

没有编译器或 core／std／test 修改，没有 fixture、全量或 x86_64 验证。一次性桌面报告与被本批替代的旧测试产物清理，当前测试入口、独立 sysroot、命令和共享构建缓存保留。
