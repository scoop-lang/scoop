# UI 基础交互实施记录

日期：2026-11-02。范围按 [总设计](DESIGN.md)，本期实施 macOS；Windows 后续。各批只验证 aarch64，全部功能完成后再统一执行 macOS x86_64 验证。沿用现有普通 Scoop Cone、单一宿主事件循环和按需呈现。

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
| C2：portal 与 anchor 定位 | 待实施 |
| C3：同窗口模态、关闭与宿主快捷键 | 待实施 |
| D：普通／虚拟滚动、实例保留 | 待实施 |
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
