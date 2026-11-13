# scoop.ui

普通 Scoop Cone 实现的声明式自绘框架。Component 是保留身份的 class，以成员 State 保存状态，通过 render 返回不可变 View 描述。Element 使用公开的约束、子项测量／放置、Canvas、InputArea 和 Surface 动画绘制自身。框架不提供 Label、Button、默认样式或容器布局策略。

当前提供 macOS aarch64／x86_64 与 Linux GNU x86_64 实现，包含独立自绘闭环及公共文本子包；基础控件由独立的 `scoop.ui.component` 提供。公开合同见 [UI 规范](../../../docs/specs/SCOOP-UI-SPEC.md)，macOS 验证见 [实施记录](../../../docs/ui/PROGRESS.md)，Linux 验证见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。

## 组合与状态

- `component(create, key)`／`component(props, create, key)` 在挂载时创建组件；`element` 以相同方式创建自绘元素，并接收显式 children。
- `lazyElement(source, create, retention, key)` 及带 props 的重载创建按需容器，由 LazyElement 选择实际准备、测量和放置的条目；retention 缺省为 ItemRetention.Dispose。
- `state(initial)` 支持成员属性委托，也可以由普通业务 Store 持有并供多个组件共享。render 中读取的 State 建立依赖；写入通知在 owner 上合并，条件读取改变时替换订阅。
- 独立 Store 或控件在修改借用的文档模型之前，可用 `State.checkMutation()` 检查 owner 和 UI 阶段。它不读写值、不登记依赖或通知；正常 `value` setter 继续检查实际修改时机。
- `Props<P>.value` 提供最新父级输入；`State.binding()` 或 `binding(get, set)` 读写原状态，不创建另一份值。
- key 在直接父级内生效，节点类别及组件和 Props 的完整类型参与匹配。新挂载必须返回新的 class 实例；普通更新保留实例与本地状态。
- 业务依赖可通过语言 Context 在实际运行／挂载入口安装。临时创建 View 不捕获 Context，属性委托在组件初始化时绑定一次。

State 在创建它的线程读写；render／measure／scene／draw 不允许写入 State。后台结果通过窗口 ApplicationProxy 或现有 EventLoopSession 投递到 owner，再修改状态。逻辑更新不依赖 drawable，隐藏窗口仍可更新 Props、卸载子树及关闭资源。

## 应用根与原生资源

`UiHost.attachApplication(active) { scope -> MyApplication(scope) }` 挂载 `ApplicationComponent`，返回 `UiApplication?`。每个宿主至多一个应用根，可同时保留多个窗口根；应用根自身没有 Window、Surface、默认字体或 GPU context。其 render 返回 `ApplicationContent`，用 `applicationElement(props, create, key)` 描述具有 Props、onMount、update、onUnmount 的 `ApplicationElement`。菜单等原生资源在挂载或 update 中创建和修改，Binding 的显示值在 render 中读取。它与窗口树共用 State 依赖、key 匹配和按需更新队列。

ApplicationScope 和 ComponentScope 均提供 application、uiHost、resource、onDispose、dispatch，以及 observeMenu、observePopup、observeModal、observeFileDialog。事件按实际资源身份交付，登记可提前关闭，卸载自动注销；已完成的 session 在登记时交付一次已有结果。`dispatch` 把库自己的回调放回所属错误边界。应用根失败先清理，再调用 `onApplicationError`；窗口根失败仍调用 `onRootError`。处理器正常返回时其他根可继续工作，默认重新抛出并关闭宿主。

`UiApplication.close()` 只卸载应用根。窗口全部关闭后是否继续运行由 ApplicationOptions.lastWindowPolicy 决定；托盘应用应使用显式 Application 和 UiHost，而非默认关闭最后窗口即退出的 `runUiApplication`。独立托盘组件通过普通 ApplicationElement 协议接入，UI 核心不依赖托盘 Cone。

`ComponentScope.observeFrameMetrics` 立即交付当前窗框快照，随后由真实 FrameChanged／GeometryChanged 通知更新。`scope.createWindowRegion(WindowRegionKind.Hit(...))` 随挂载管理原生命中区域；在 `SceneContext.place(handle, rect或path)` 中放置，只有真实 Accepted 帧才发布最终 transform／clip。Shape／Input 每个窗口各允许一个登记，按后端能力检查；禁用、输入边界和卸载可立即撤销命中资格。细节见[应用挂载规范](../../../docs/specs/SCOOP-UI-DESKTOP-SPEC.md)和[Surface 区域规范](../../../docs/specs/SCOOP-SURFACE-FRAME-SPEC.md)。

无窗口生命周期验证见 [application](tests-cones/application/README.md)，共享状态、错误隔离与 Accepted 区域组合见 [desktop](tests-cones/desktop/README.md)。

## 自绘与资源

Element.measure 返回 LayoutMetrics，包含满足 Constraints 的尺寸与可选基线。每个直接子项放置前可重新测量，同约束相邻调用复用最后结果，然后放置零次或一次；未放置子项保留挂载与状态，本帧不绘制。Element.scene 使用 SceneContext 声明变换、clip 和 InputArea，在背景与前景的 draw 块之间调用 children。draw 只接收带 Canvas／size／time 的 PaintContext；绘图块的局部变换不会改变随后登记的命中几何，成功提交后才发布区域。

有限且有可靠内容版本的绘图使用 `context.drawLayer(key, bounds, revision, samplingOutset = 1.0) { paint -> ... }`。PaintKey 由元素持有；回调使用独立局部坐标，结果以 SrcOver 合成。ScrollViewport.paintCache 默认 None，Tiles 按设备像素块复用实际图像；场景、输入和 children 每帧照常装配。版本必须覆盖 State、时间、动画与资源变化，普通 draw 仍直接执行。ParagraphHost 的正文已接入此协议。

块采用与局部原点对齐的设备像素网格；纯平移复用重叠块，改变内容版本、尺寸、缩放或后端时重建。离开范围、未放置或卸载时及时释放。记录内的 ImageFilter 和未声明可独立裁剪的原生命令按 Drawing.canRasterizeTiles 结果直接重放；可把滤镜放到 SceneContext.saveLayer 的完整内容合成阶段，使其输入仍可缓存。

`context.onFrameComplete { result -> ... }` 登记当前帧的一次性完成通知。Accepted 后可以使用已发布区域请求焦点，也可以写 State 或请求后续帧；Discarded 保留旧几何。通知执行前若节点卸载或根关闭则取消，不能把资源清理依赖在通知上；场景与绘图 context 的借用仍在同步回调结束时失效。

在 onMount 中使用 ComponentScope.resource 登记 Path、Image 或缓存，使用 createArea 和 animation 登记稳定的输入区域与 Surface Animatable。应用不为每个组件 use；作用域按子先父的顺序退休输入／动画、调用 onUnmount，再逆序关闭资源。可替换缓存应自行及时关闭旧值，不能每帧登记一个新资源。

自绘 Element 可用 `scope.focusTarget(area, FocusOptions(Some(handle)))` 登记可聚焦区域。FocusHandle 提供 hasFocus、isFocusActive、isAttached、isPending 和 requestFocus；它查询 Surface 的现有焦点状态。已经显露的目标立即聚焦，滚动内容中的目标先逐层显露，等待首次或恢复放置的目标在 Accepted 后兑现；新的焦点请求会取消旧请求。关闭或更换外部 handle 会解除登记，不能在 render／measure／scene／draw 中请求焦点。

FocusOptions 的 order 默认 0，同值使用当前逻辑挂载顺序，tabStop=false 才退出 Tab。`scope.focusScope(FocusScopeOptions(loop = true))` 建立组内循环；普通 scope 在边界让父继续，窗口根循环。FocusScope.requestFocus 按 defaultFocus、首个 Tab 目标、fallback 选择。Tab 和 KeyNavigation 可以选择滚动内容内尚需显露的已放置目标；禁用、未放置或不能显露的目标被跳过，Tab 的异步显露失败会继续本次有限遍历。已聚焦目标仅被滚动裁剪时保持当前会话。

FocusTarget 和 FocusScope 先检查当前 Surface InputBoundary 的准入，域外请求立即失败，也不会作为等待首次放置的 pending 留到边界关闭后执行。已知窗口的 pending 和被拒绝的显式请求同样推进 Surface.focusRevision，新的请求会取消旧的自动聚焦或恢复意图。

`portal(openBinding, content) { ... }` 把一个子 View 放到当前窗口的层出口。原位置只返回约束下限的占位尺寸，内容在层约束下测量一次；它越过原祖先的 transform／clip，继续沿原挂载树路由输入并参与 Tab。相同 key 保留会话，逻辑重排不改变层的打开顺序，子 portal 位于父层之后。层先按逻辑所属模态域排列，因此背景 Tooltip 即使后来打开，也位于模态之下。

`portal(openBinding, content) { modal { initialFocus = Some(handle) } }` 启用同窗口模态。初始 false 不挂载内容或安装屏障；打开时保存原焦点并安装 InputBoundary，首次成功呈现后按 initialFocus、首个 Tab 目标、根代理聚焦。Tab 在当前域循环。关闭立即停止该层交互，退出呈现期间保留屏障，直到成功提交移除帧才释放并尝试恢复原焦点。新的显式聚焦意图优先，原目标不可用时选择当前域的回退目标；restoreFocus=false 可禁用恢复。普通／模态互相切换会重新挂载会话。

`OverlayDismiss(escape = true, outsidePress = OutsidePress.CloseAndConsume)` 配置一次只关闭最上层适用弹层，默认两项均不关闭。外部点击按逻辑子树判断，子菜单不因画在父面板外就被视作外部；面板需用普通 InputArea 声明内部空白区域。非模态还可用 CloseAndContinue 让原目标继续收到同一次按下，模态不能使用该策略。遮罩颜色、面板和侧边布局由内容组合，不自动修改背景滚动状态。

相对定位时，在触发元素的 `onMount` 中调用 `scope.layoutAnchor(handle)`，在其 `scene` 中调用 `context.anchor(handle, rect)`，再把同一 LayoutAnchor 交给 `OverlayPlacement.Anchored`。锚点用当前变换和 clip 取得窗口位置，不需要 InputArea。定位先给内容有界约束，再按实际尺寸 flip／shift；`context.overlayLayout` 可读取本帧最终 bounds、available、anchor 和 side 来绘制箭头。Viewport 形状提供窗口内的横纵对齐；AtPoint 使用窗口逻辑坐标和同一定位算法，适用于上下文菜单，无须创建占位锚点。

open Binding 是打开状态的唯一来源。Escape、外部按下和 OverlayHandle.requestClose 先调用 onCloseRequest；接受后写 false，setter 未采纳时继续打开。外部直接写 false 不经过 veto。transition 使用 PresenceOptions 的有限进入／退出动画；退出保留绘制子树并关闭输入，Absent 后卸载，Accepted 移除后才调用一次 onClosed。退出中重新打开保留同一子树、从当前进度反向，并等待新 Accepted 几何恢复输入。

锚点不可见时默认在成功帧后强制关闭，也可以选择 Hide 保留实例等待恢复，或 KeepVisible 使用有效坐标留在视口内；解绑或 owner 退休会结束从属层。强制关闭不经过 veto；AnchorUnavailable 尝试写 Binding=false，即使未采纳也结束当前呈现，直到绑定实际 false→true 才重开。OverlayHandle 可查询 isOpen／lastCloseReason。

`scope.presence` 管理呈现阶段与 0–1 进度，使用已有 Surface 时间轴；`scope.hover` 借用触发区域和逻辑后代的弹层区域，以一次性 deadline 和有限通过区域协调悬浮打开与关闭。只有打开或等待期间才观察当前窗口指针，离开窗口、失焦或卸载撤销等待。

`scope.activeItems` 按 NavigationSource 的稳定 key 更新活动项 Binding，可把 reveal 接到 VirtualListHandle.scrollToItem，输入框的真实焦点和 IME 连接保持原位。`scope.typeahead` 为非编辑列表提供短期 Unicode 缓冲与可替换匹配规则，数据替换／失焦／超时清空；编辑组合关闭其 keyboard 模式，直接使用编辑模型的已提交查询。

最外层用 `UiHost.use` 包住 Application.run，或使用 `runUiApplication(title, create)`。SurfaceContent 拥有 UI 根，原生窗口关闭、异常和显式关闭都释放挂载树。UiWindow.close 只关闭所附着的 UI／Surface 内容，原生 Window 的归属仍由调用者决定。绘制或逻辑回调内请求关闭时，先停止新工作，回调结束后再释放资源。

`UiHost.shortcutScope(bindings)` 返回宿主拥有的 Surface HostShortcutScope，可在窗口创建前登记应用绑定；窗口内的控件行为及 ShortcutScope 优先，模态会截断宿主回退。每个 UiHost 至多一个活动登记，关闭单个 UiWindow 不会取消共享绑定，关闭 UiHost 立即停用当前登记。

无状态失效、显式帧请求或活动动画时，框架不遍历树、不投递空任务；Surface 停止请求原生帧。动画通过 Surface 同一时间轴采样，直接参与本帧测量与绘制，不逐帧重跑未失效的 render。

`ScrollState` 保存最近 Accepted 的 offset、viewport、content 和 maximum，hasSnapshot 区分首个成功帧前的零值。scrollTo／scrollBy／reveal 立即登记请求，画面快照等成功呈现后更新；scrollBy 返回按最近范围收敛的请求位移，供嵌套 wheel 计算残量，连续请求在尚未呈现的目标上累积。自绘容器在 onMount 登记 `scope.scrollViewport(state)`，在 measure 调用其 `layout(viewport, content)` 取得本帧快照，在自己的 scene 调用 `present(context)`。被丢弃帧不覆盖旧结果，成功帧也不会吞掉期间发出的新请求。一个 State 同时只能绑定一个视口，关闭立即解绑，业务 State 保留最后位置。

定位方法的 origin 默认为 `ScrollOrigin.Programmatic`，输入行为使用 User。`scope.resource { scroll.observeIntent { event -> ... } }` 观察带 requestRevision 的 User／Programmatic／LayoutCorrection：用户在边界滚动仍有意图通知，自动保持阅读位置只在 Accepted 后通知修正。订阅可独立关闭，回调内新请求优先；不用全局事件总线。

呈现滚动内容时使用 `viewport.present(context, Some { content -> content.children() })`。内容回调自动获得统一的视口 clip 和负 offset 平移，也登记实际滚动路径；滚动条或固定装饰在回调外绘制。省略回调只发布布局。焦点显露按同一成功帧的几何从内向外进行，每次只调整一层并等待提交；Portal 在层出口建立自己的场景路径。更新的焦点或已调整视口上的滚动请求会取消旧显露，隐藏时等待正常恢复。

`state.animateTo(offset, spec)` 使用视口所属 Surface 的时间轴，要求已绑定，可在首布局前排队；`cancelAnimation()` 在最后发布的位置停止。直接定位和 reveal 同样停止旧动画。实际 ScrollArea 和滚动条属于独立控件库，自绘容器可通过这组公开接口实现自己的布局，不需观察 offset 来重建内容树。滚动纯状态测试在本 Cone，成功／丢弃帧、动画、绑定替换和停帧在 `tests-cones/scroll` 验证。

## 按需子项

`ItemSource` 提供 count、keyAt、indexOfKey 和 build；数据或索引改变时替换这个快照，key 必须是稳定且唯一的 Text／Number。框架只检查实际访问的映射，不预先构建全部 View。build 使用普通 render 依赖追踪；已有项只因滚动再次测量时不会重跑 build。删除已实现项在逻辑更新中完成，即使窗口隐藏也会释放输入和资源。

LazyElement 的生命周期及 scene 与 Element 相同，measure 接收 LazyMeasureContext。容器用 `measureItem(index, constraints)` 按需取得子项，再用 `placeItem(index, origin)` 放置；一个索引放置前可重新测量，最终选择集合在根或 portal 测量结束后统一收尾。框架在这两个步骤之间管理 build、挂载和 update，生命周期可以正常写 State，外层 measure 和子项 render 仍保持纯度。子项在 scene.children() 中按 source 索引顺序呈现。

`ItemRetention.Dispose` 让离开选择范围的项正常卸载；`KeepAlive(maxOffscreenItems)` 额外保留有限个最近使用的离屏实例，容量为零与 Dispose 相同。缓存项继续处理 Props／State 并持有资源，本帧不测量或呈现场景；再次进入复用同 key、同 shape 的实例，真实删除或 shape 改变仍正常退休。容量不包含本轮选择和活动交互固定的项。

`context.pinnedItems` 给出焦点、待聚焦请求、按钮／滚动路由、文本会话及模态输入边界所属的已实现项，容器也须测量、放置这些项，再以统一 clip 控制可见性；交互结束及待聚焦请求结束后触发必要布局以恢复淘汰。每个列表为同一滚动候选至多额外保留一项，不产生持续帧。实际 VirtualList 由独立控件库实现；协议、隐藏删除、重排、失败清理和挂载中关闭分别由 `tests-cones/lazy`、`tests-cones/errors` 覆盖。

LazyElement 可用 `scope.lazyFocus(viewport)` 登记按 key 聚焦，viewport 必须属于同一挂载。`requestFocus(key, ItemFocusEdge.First)` 或 Last 请求该项首／末个可用 Tab 目标；待实现项的索引会加入 pinnedItems，外层缓存条目也会因这个请求重新参与布局。容器读取 `pendingItem`，按自己的 extent 配合 ScrollAdjustment 初次定位；选择具体目标后该属性变为 None，后续显露交给框架。isPending 查询整个请求，cancelRequest 只取消等待。数据删除、新焦点／位置意图、换绑和关闭取消旧请求，不预建全量 FocusHandle。

登记可另传 `LazyFocusOptions(navigation, layout, order)` 并整体更新 options。ItemNavigation.None 保留普通已挂载目标的行为；Sequential 按数据顺序访问行内 Tab 目标；`Roving(direction, loop)` 使条目集合占一个 Tab 停靠点，方向键与 Home／End 请求实际条目。layout 使用 Horizontal／Vertical。嵌套列表和行内 FocusScope 通过同一成功帧遍历续行，空／禁用项按需跳过；roving 的返回 key 来自真实焦点活动，实例卸载后仍可重新定位该 key。

`ScrollViewport.present` 的 contentOrigin 默认是零。超长内容可先在 Double 中减去附近原点，再用这个原点呈现；框架一次平移 contentOrigin − offset，使绘制和输入几何保留局部精度，完整滚动范围与显露坐标不变。

未选中的缓存子树关闭帧需求，逻辑更新照常执行。`scope.animation` 和视口动画通过同一 Surface AnimationGroup 管理；有限动画可离屏完成，无限动画不会周期唤醒窗口。恢复只重新启用子树的帧资格，保留动画自身的 isFrameEnabled 选择，也不会唤醒嵌套列表中仍未选中的缓存项。`scope.requestFrame()` 在该子树内同样不要求绘制。

## 公共文本适配

`scoop.ui.text` 用 `InlineView(key, view, size, alignment)` 连接 graphics.text 内容中的 InlineKey 与普通 UI 子项。size 为 None 时采用自然尺寸，可指定固定 Size；对齐默认采用子项的 alphabetic baseline，缺失时取底边，也可以选择 ideographic baseline 或显式 InlineAlignment。

描述阶段把 `inlineChildren(content, inlines)` 作为 Element 的 children，Element 在 onMount 中登记 `scope.resource { ParagraphHost() }`。measure 调用 `host.measure(context, constraints, fonts, content, settings, inlines)`，scene 调用 `host.scene(context)`。这组接口完成子项测量、段落布局、可见对象放置、文字绘制和子项绘制；图片同样使用普通 Element 包装。

ParagraphHost 只拥有 Paragraph／TextLayout，不拥有字体或子组件。它按同一内容对象、字体身份及 revision、段落设置和实际宽度／metrics 复用缓存。布局动画改变 metrics 时重排，纯子项换色可复用布局；新内容描述可以触发重建。`host.layout` 借出最近成功测量的结果，供 hitTest／caret／selection 查询，调用方不关闭它。缓存替换或宿主关闭后旧布局资源失效。

行内出现的 key 与 View 自己的 key 分别参与普通匹配。插字、RTL 顺序或换行不丢失子组件状态；截断项保留挂载，成功帧后退出命中和焦点，恢复后继续使用原实例。宿主不隐式裁剪，调用方可用SceneContext 的 clip 包住整段文字及子项。

## 示例与定向测试

`SceneContext.children(order)` 可按完整子项索引排列呈现场景，例如 Absolute 的同层 zIndex；排列先检查完整性、重复和边界，再呈现已放置项。它与无参 children 共用一次呈现和借用规则，逻辑挂载、路由及 Tab 顺序保持不变。

`scope.keyNavigation(NavigationOptions(items, ...))` 将一组 FocusHandle 接入方向键与 Home／End。Horizontal／Vertical 按成员数组顺序，Grid 按 NavigationItem 的 row／column；横向映射复用 TextDirection。默认 roving 让组中仅一个有效成员进入 Tab，返回组时根据 Surface 的真实聚焦历史选择，鼠标和直接程序请求保持一致。配置可整体更新，重复成员、跨组句柄及重复网格位置报告错误。该助手只移动焦点，Enter／Space 和选择语义仍由控件处理；主题示例的主题／明暗按钮提供了实际接入。

[自绘 Counter](examples/custom/README.md) 只使用框架及下层公开接口，实现点击、Path 绘制、状态和宽度动画，不依赖控件库。

```sh
scoop test sysroot/src/scoop.ui --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
scoop test sysroot/src/scoop.ui/tests-cones/drawing --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
scoop test sysroot/src/scoop.ui/tests-cones/errors --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
scoop test sysroot/src/scoop.ui/tests-cones/focus --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
scoop test sysroot/src/scoop.ui/tests-cones/navigation --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
scoop test sysroot/src/scoop.ui/tests-cones/overlay --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
scoop test sysroot/src/scoop.ui/tests-cones/text --release --target aarch64-apple-darwin --sysroot /path/to/sysroot
```

普通测试验证状态／Binding、线程规则和约束，并以隐藏原生窗口验证多根组合；drawing Cone 从公开接口验证真实绘制、输入、动画、可选放置和回调关闭；errors Cone 验证构造、挂载、更新、布局和绘制失败下的清理。原生窗口测试顺序运行，避免焦点和遮挡相互影响。开发时先按仓库 scripts 说明准备目标 sysroot，并显式使用同一套 scoop／scoopc。

OverlayPlacementTest 检查有界定位、flip／shift 和参数错误；overlay Cone 检查真实像素与命中、嵌套层逻辑路由、Tab 与打开次序、resize、Hide／KeepVisible、Discarded、句柄替换、锚点重绑及停止渲染。自引用、依赖环、跨根和重复句柄同时覆盖正常错误处理与部分挂载清理。

UiTextCacheTest 使用真实字体／段落验证缓存替换与失败恢复；text Cone 通过框架公开 API 验证富文本、固定尺寸图片、自然尺寸交互子项、动画重排、截断／恢复、RTL 和内部 key 更换。固定字体由 [text-fonts 测试数据 Cone](../scoop.graphics.text/tests/data/FONTS.md) 提供，不加入生产默认字体。

自定义可变尺寸滚动容器可在布局中提供 `ScrollAdjustment(scroll.requestRevision, offset)`，交给 `ScrollViewport.layout`。调整只在定位意图仍相同时应用，成功帧才更新 ScrollState；新请求和活动动画优先。按 key 主动定位先用 `beginLayoutRequest()` 登记请求，通知返回后检查返回的 revision 仍为最新，再保存待布局解析的目标；数据变化后的自动修正沿用已有 revision。`ScrollSnapshot.revealedOffset` 纯计算指定矩形的显露位置，可复用同一套对齐规则。

输入限制使用 `scoop.ui.text.textEditPolicy { … }`，通过 TextEditOptions.policy 或 Input／Textarea builder 的 policy 传入。策略在编辑事务发布之前处理已提交文字，可组合单行归一化、粘贴变换、字符接受与 grapheme 长度限制；未提交 preedit 原样保留。策略回调返回带 UTF-8 区间的变换或拒绝，`onEdited` 只接收最终模型；普通 replace／replaceAll 不隐式读取宿主策略，显式程序过滤使用 `applyEdit(range, text, policy)`。

`TextEditHost` 的显示投影与编辑正文分离：`textMask()` 和 `textProjection { … }` 保留同一 IME／历史，并通过 `TextDisplay` 映射命中和几何。只读 `SelectableTextHost` 复用 StyledText／ParagraphHost，提供方向选区、行内替代文字复制及稳定 key 的范围动作。

窗口错误通过 `UiHost(onRootError = { id, error -> ... })` 接收：失败根完成输入与资源清理后报告一次，处理器正常返回可继续运行其他根，默认处理器重新抛出。`attach` 成功返回 `Some(UiWindow)`，已处理的初次挂载失败返回 `None`。宿主事件、共享渲染器、消息投递和错误处理器自身的失败仍关闭整个宿主。`scope.createDeadline` 使用原生一次性 deadline，随作用域退休关闭，并在所属根的错误边界内交付回调。

同一父级与 key 的 Portal 可更新 NonModal／FocusContained／Modal，不重新挂载内容。约束更新保留原始恢复焦点，新增聚焦等待 Accepted；关闭中的约束保留到移除帧被接受。模式变化本身不产生关闭回调。

## Linux GNU

支持 Wayland 与 X11，共用公共实现、Skia Ganesh GLES 和 Fontconfig／FreeType。使用已准备的 GNU sysroot，运行 `scoop test sysroot/src/scoop.ui --release --sysroot ~/.scoop`；图形应用通过 `SCOOP_WINDOW_BACKEND=wayland` 或 `x11` 选择显示协议。文本编辑与快捷键使用 Ctrl 作为 Primary；Home／End 移至行边界，Ctrl+Home／End 移至文档边界，Textarea 使用 Ctrl+Enter 提交。依赖、测试范围及实际限制见 [Linux GUI 记录](../../../docs/linux-gui/PROGRESS.md)。

UiHost 每次实际绘制先清空窗口内容，移动或移除元素不会保留旧像素。独立 `tests-cones/frames` Cone 用真实窗口的连续帧读回验证这一行为，可在 macOS 或 Linux 上运行；Linux 验收覆盖 X11、Wayland 和 Wayland 2 倍缩放。
