# Scoop UI 应用挂载与原生资源规范

状态：D7 实施合同。补充 [UI 规范](SCOOP-UI-SPEC.md)第 9 节，交付与平台验证见[实施记录](../desktop-ui/PROGRESS.md)。菜单、模态、文件对话框及托盘的原生行为仍由各所属 Cone 定义。

## 1. 应用根

`UiHost.attachApplication(application, create: (ApplicationScope) -> C): UiApplication?` 挂载一个 `C: ApplicationComponent`。application 必须是当前正在驱动该 UiHost 的 ActiveApplication；每个宿主至多一个未退休的应用根。成功返回 UiApplication，首轮构造／render／挂载失败在完成清理并报告后返回 None。重复挂载、错误线程或关闭后的宿主属于状态错误。

UiApplication 提供 isClosed 与幂等 close。关闭只卸载应用根；窗口根、业务 State、Application 和调用方持有的资源仍遵守自身归属。UiHost 关闭及 Application Exiting 会关闭剩余应用根。窗口全部关闭后，应用根仍能更新和接收所属原生事件；实际应用是否退出仍由 ApplicationOptions.lastWindowPolicy 决定。

应用根不创建 Window、Surface、默认 FontCollection 或 GPU context。它与窗口根复用同一个按需逻辑更新队列、State 依赖收集、Props 替换、key 匹配及清理规则；只有原生显示／回调变化时不制造 Surface 帧。

## 2. 描述与挂载协议

`ApplicationComponent(scope)` 是持久实例，定义 `render(): ApplicationContent`、onMount、onUnmount。`ApplicationContent(items: Array<ApplicationItem>)` 是本轮附属条目序列。`applicationElement(props, create, key, comparison = None)` 创建带显式 ViewKey 的 ApplicationItem；无 Props 的重载只接受 create、key 和 comparison。key 在这份完整序列内匹配，重复的非 Position key 是参数错误；同 key、同具体元素类型可复用实例，移除后重新插入创建新实例。

`ApplicationElement(scope)` 定义 onMount、update 和 onUnmount，没有 measure／scene。工厂接收 `(Props<P>, ApplicationScope)`；Props 在每次输入替换时更新，即使 InputComparison 判定无需调用 update，业务动作也读取最新 Props。外部 Cone 可通过相同协议提供原生元素；UI 核心没有含托盘类型的封闭枚举。

render 只构造描述并读取 State／Binding。原生创建和更新在构造资源、onMount、update 或所属业务回调中执行；Binding 的显示值必须在 render 中读取，以登记真正的依赖。直接从 update 读取任意 Binding 不建立额外的响应式通道。

scope 在工厂执行前已具有完整应用与资源归属。首次 render 在根 onMount 前完成；根 onMount 成功后挂载附属元素。元素 onMount 后执行首次 update。只有成功完成 onMount 的实例执行 onUnmount；工厂、首轮 render／mount、update 失败都清理已取得资源。实例保存单次挂载归属，不能在同一根或其他根重复挂载，卸载不会重置归属。

## 3. Scope 与资源事件

ApplicationScope 提供 application、uiHost、isMounted、resource、onDispose 和 dispatch。ComponentScope 增加同名的 application、uiHost 与 dispatch 桥接，原有 nativeWindow、input、布局和字体能力保持必需。ApplicationScope 不提供窗口、绘制或输入区域能力。

`dispatch(action): Boolean` 仅在所属挂载仍可工作时执行同步动作；动作结束后仍有效才返回 true。窗口 scope 进入所属 SurfaceWindow.dispatch，应用 scope 进入应用根错误边界。独立托盘 Cone 用这个入口包装自己订阅的 TrayEvent，不替换 ApplicationHandler，也不把托盘类型引入 UI。

两种 scope 提供以下类型化登记，返回可提前关闭的 Closeable，并随 scope 自动注销：

| 登记 | 回调值 |
| --- | --- |
| observeMenu(menu, action) | MenuActivation |
| observePopup(session, action) | PopupResult |
| observeModal(session, action) | ModalResult |
| observeFileDialog(session, action) | FileDialogResult<T> |

登记按对应的 MenuId／PopupSessionId／ModalSessionId／FileDialogSessionId 路由，只交付所属资源。session 已完成时，在该次登记的所属同步边界交付既有结果一次；未完成时等真实完成事件，不轮询。回调执行前移除一次性 session 登记，显式关闭或卸载后的迟到事件不执行旧动作。菜单动作保留完整 MenuSource，独立托盘组件通过 NativeMenuUseId 判断自身的原生使用。

ComponentScope 另提供 `observeFrameMetrics(action: (WindowFrameMetrics?) -> Unit)`。登记后立即在所属边界交付当前值，之后只在该窗口 FrameChanged／GeometryChanged 时读取 native 快照；关闭窗口后不再查询。它使实际窗框变化驱动 State，而不是逐帧查询或长期使用创建时的期望值。

## 4. 错误与退休

UiHost 增加 `onApplicationError: (Throwable) -> Unit = { error -> throw error }`，位于可选窗口错误处理器之后、handler 之前。应用根失败先停用自身、移除 State 和事件登记、关闭实际菜单／托盘等资源，再调用该处理器。处理器正常返回可保留其他窗口；重新抛出则沿现有宿主失败出口关闭整个 UiHost 并传播。

窗口资源回调失败仍交给 onRootError(WindowId, Throwable)，不伪造窗口身份来报告无窗口错误。渲染、挂载、更新或资源动作中请求关闭时先停止新工作，等当前同步借用退栈后完成 onUnmount 与资源释放。清理按现有 ResourceUseException 合并，并继续处理其余资源。异步 finally 和 native session 退休仍由下层宿主推进。

## 5. 验证范围

以普通 Cone 验证无窗口应用根的首次构造、State／Binding 更新、动态依赖、key 重排与移除、最新 Props、实例归属及清理；与窗口根共享 State 时各自失效。原生菜单和文件 session 的实际事件进入所属回调，注销和失败后停止交付。应用根失败且处理器正常返回时，其余根保持可用；默认策略继续传播。原生图像／菜单更新不产生空闲帧或隐藏窗口。

同批次的 Accepted 窗框区域发布及 ComponentScope／SceneContext 桥接见 [Surface 原生窗口区域规范](SCOOP-SURFACE-FRAME-SPEC.md)；上述应用根不依赖该几何能力。
