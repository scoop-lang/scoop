# scoop.ui

普通 Scoop Cone 实现的声明式自绘框架。Component 是保留身份的 class，以成员 State 保存状态，通过 render 返回不可变 View 描述。Element 使用公开的约束、子项测量／放置、Canvas、InputArea 和 Surface 动画绘制自身。框架不提供 Label、Button、默认样式或容器布局策略。

当前已在 macOS aarch64 实现独立自绘闭环及公共文本子包；独立控件库按批次接入，x86_64 留待本期功能完成后补验，Windows 后续实施。公开合同见 [UI 规范](../../../docs/specs/SCOOP-UI-SPEC.md)，当前验证范围见 [实施记录](../../../docs/ui/PROGRESS.md)。

## 组合与状态

- `component(create, key)`／`component(props, create, key)` 在挂载时创建组件；`element` 以相同方式创建自绘元素，并接收显式 children。
- `lazyElement(source, create, key)` 及带 props 的重载创建按需容器，由 LazyElement 选择实际准备、测量和放置的条目。
- `state(initial)` 支持成员属性委托，也可以由普通业务 Store 持有并供多个组件共享。render 中读取的 State 建立依赖；写入通知在 owner 上合并，条件读取改变时替换订阅。
- `Props<P>.value` 提供最新父级输入；`State.binding()` 或 `binding(get, set)` 读写原状态，不创建另一份值。
- key 在直接父级内生效，节点类别及组件和 Props 的完整类型参与匹配。新挂载必须返回新的 class 实例；普通更新保留实例与本地状态。
- 业务依赖可通过语言 Context 在实际运行／挂载入口安装。临时创建 View 不捕获 Context，属性委托在组件初始化时绑定一次。

State 在创建它的线程读写；render／measure／scene／draw 不允许写入 State。后台结果通过窗口 ApplicationProxy 或现有 EventLoopSession 投递到 owner，再修改状态。逻辑更新不依赖 drawable，隐藏窗口仍可更新 Props、卸载子树及关闭资源。

## 自绘与资源

Element.measure 返回 LayoutMetrics，包含满足 Constraints 的尺寸与可选基线。每个直接子项测量一次，然后放置零次或一次；未放置子项保留挂载与状态，本帧不绘制。Element.scene 使用 SceneContext 声明变换、clip 和 InputArea，在背景与前景的 draw 块之间调用 children。draw 只接收带 Canvas／size／time 的 PaintContext；绘图块的局部变换不会改变随后登记的命中几何，成功提交后才发布区域。

`context.onFrameComplete { result -> ... }` 登记当前帧的一次性完成通知。Accepted 后可以使用已发布区域请求焦点，也可以写 State 或请求后续帧；Discarded 保留旧几何。通知执行前若节点卸载或根关闭则取消，不能把资源清理依赖在通知上；场景与绘图 context 的借用仍在同步回调结束时失效。

在 onMount 中使用 ComponentScope.resource 登记 Path、Image 或缓存，使用 createArea 和 animation 登记稳定的输入区域与 Surface Animatable。应用不为每个组件 use；作用域按子先父的顺序退休输入／动画、调用 onUnmount，再逆序关闭资源。可替换缓存应自行及时关闭旧值，不能每帧登记一个新资源。

自绘 Element 可用 `scope.focusTarget(area, FocusOptions(Some(handle)))` 登记可聚焦区域。FocusHandle 提供 hasFocus、isFocusActive、isAttached、isPending 和 requestFocus；它查询 Surface 的现有焦点状态。可见目标立即聚焦，等待首次或恢复放置的目标在 Accepted 后兑现；新的焦点请求会取消旧请求。关闭或更换外部 handle 会解除登记，不能在 render／measure／scene／draw 中请求焦点。

FocusOptions 的 order 默认 0，同值使用当前逻辑挂载顺序，tabStop=false 才退出 Tab。`scope.focusScope(FocusScopeOptions(loop = true))` 建立组内循环；普通 scope 在边界让父继续，窗口根循环。FocusScope.requestFocus 按 defaultFocus、首个 Tab 目标、fallback 选择。禁用、未放置和完全裁剪的目标不作为新的 Tab 停靠点；已聚焦目标仅被滚动裁剪时保持当前会话。

FocusTarget 和 FocusScope 先检查当前 Surface InputBoundary 的准入，域外请求立即失败，也不会作为等待首次放置的 pending 留到边界关闭后执行。已知窗口的 pending 和被拒绝的显式请求同样推进 Surface.focusRevision，新的请求会取消旧的自动聚焦或恢复意图。

`portal(content, PortalOptions(...), key)` 把一个子 View 放到当前窗口的层出口。原位置只返回约束下限的占位尺寸，内容在层约束下测量一次；它越过原祖先的 transform／clip，继续沿原挂载树路由输入并参与 Tab。相同 key 保留会话，逻辑重排不改变层的打开顺序，子 portal 位于父层之后。层先按逻辑所属模态域排列，因此背景 Tooltip 即使后来打开，也位于模态之下。

`PortalOptions(modal = Some(ModalOptions(initialFocus = Some(handle))))` 启用同窗口模态。挂载时保存原焦点并安装 InputBoundary，首次成功呈现后按 initialFocus、首个 Tab 目标、根代理聚焦；Tab 在当前域循环。关闭立即停止该层交互，直到成功提交移除帧才解除屏障并尝试恢复原焦点。新的显式聚焦意图优先，原目标不可用时选择当前域的回退目标；restoreFocus=false 可禁用恢复。普通／模态互相切换会重新挂载会话。

`OverlayDismiss(escape = true, outsidePress = OutsidePress.CloseAndConsume)` 配置一次只关闭最上层适用弹层，默认两项均不关闭。外部点击按逻辑子树判断，子菜单不因画在父面板外就被视作外部；面板需用普通 InputArea 声明内部空白区域。非模态还可用 CloseAndContinue 让原目标继续收到同一次按下，模态不能使用该策略。遮罩颜色、面板和侧边布局由内容组合，不自动修改背景滚动状态。

相对定位时，在触发元素的 `onMount` 中调用 `scope.layoutAnchor(handle)`，在其 `scene` 中调用 `context.anchor(handle, rect)`，再把同一 LayoutAnchor 交给 `OverlayPlacement.Anchored`。锚点用当前变换和 clip 取得窗口位置，不需要 InputArea。定位先给内容有界约束，再按实际尺寸 flip／shift；`context.overlayLayout` 可读取本帧最终 bounds、available、anchor 和 side 来绘制箭头。Viewport 形状提供窗口内的横纵对齐。

锚点不可见时默认在成功帧后关闭，也可以选择 Hide 保留实例等待恢复，或 KeepVisible 使用有效坐标留在视口内；解绑会关闭从属层。OverlayHandle 提供关闭与状态查询，onClose 每个会话只通知一次当前回调。已关闭会话不会因普通 Props 更新自动重开，调用方移除后重新挂载或更换 key。

最外层用 `UiHost.use` 包住 Application.run，或使用 `runUiApplication(title, create)`。SurfaceContent 拥有 UI 根，原生窗口关闭、异常和显式关闭都释放挂载树。UiWindow.close 只关闭所附着的 UI／Surface 内容，原生 Window 的归属仍由调用者决定。绘制或逻辑回调内请求关闭时，先停止新工作，回调结束后再释放资源。

`UiHost.shortcutScope(bindings)` 返回宿主拥有的 Surface HostShortcutScope，可在窗口创建前登记应用绑定；窗口内的控件行为及 ShortcutScope 优先，模态会截断宿主回退。每个 UiHost 至多一个活动登记，关闭单个 UiWindow 不会取消共享绑定，关闭 UiHost 立即停用当前登记。

无状态失效、显式帧请求或活动动画时，框架不遍历树、不投递空任务；Surface 停止请求原生帧。动画通过 Surface 同一时间轴采样，直接参与本帧测量与绘制，不逐帧重跑未失效的 render。

`ScrollState` 保存可观察的 offset、viewport、content 和 maximum，支持 scrollTo／scrollBy／reveal。自绘容器在 onMount 登记 `scope.scrollViewport(state)`，在 measure 调用其 `layout(viewport, content)` 取得本帧快照，在自己的 scene 调用 `present(context)`；成功提交才发布范围，被丢弃帧不覆盖旧结果。一个 State 同时只能绑定一个视口，登记可以整体替换 state，关闭立即解绑，业务 State 保留最后位置。

`state.animateTo(offset, spec)` 使用视口所属 Surface 的时间轴，要求已绑定，可在首布局前排队；`cancelAnimation()` 在最后发布的位置停止。直接定位和 reveal 同样停止旧动画。实际 ScrollArea 和滚动条属于独立控件库，自绘容器可通过这组公开接口实现自己的布局，不需观察 offset 来重建内容树。滚动纯状态测试在本 Cone，成功／丢弃帧、动画、绑定替换和停帧在 `tests-cones/scroll` 验证。

## 按需子项

`ItemSource` 提供 count、keyAt、indexOfKey 和 build；数据或索引改变时替换这个快照，key 必须是稳定且唯一的 Text／Number。框架只检查实际访问的映射，不预先构建全部 View。build 使用普通 render 依赖追踪；已有项只因滚动再次测量时不会重跑 build。删除已实现项在逻辑更新中完成，即使窗口隐藏也会释放输入和资源。

LazyElement 的生命周期及 scene 与 Element 相同，measure 接收 LazyMeasureContext。容器用 `measureItem(index, constraints)` 按需取得子项，再用 `placeItem(index, origin)` 放置；一个索引本轮只测量一次。框架在这两个步骤之间管理 build、挂载和 update，生命周期可以正常写 State，外层 measure 和子项 render 仍保持纯度。子项在 scene.children() 中按 source 索引顺序呈现。

本批离开选择范围的项正常卸载。`context.pinnedItems` 给出焦点、待聚焦请求、按钮／滚动路由及文本会话所属的已实现项，容器也须测量、放置这些项，再以统一 clip 控制可见性；交互结束及待聚焦请求结束后触发必要布局以恢复淘汰。每个列表为同一滚动候选至多额外保留一项，不产生持续帧。实际 VirtualList 由独立控件库实现；协议、隐藏删除、重排、失败清理和挂载中关闭分别由 `tests-cones/lazy`、`tests-cones/errors` 覆盖。

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

自定义可变尺寸滚动容器可在布局中提供 `ScrollAdjustment(scroll.requestRevision, offset)`，交给 `ScrollViewport.layout`。调整只在定位意图仍相同时应用，成功帧才更新 ScrollState；新请求和活动动画优先。`ScrollSnapshot.revealedOffset` 纯计算指定矩形的显露位置，可复用同一套对齐规则。
