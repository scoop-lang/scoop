# 使用配套库与 API

先查询 `scoop example --search "关键词"`，读取相关主题，再扩展到当前版本的真实 API。不要仅因教程没有相应主题就认为某个库不存在。

## 查找顺序

1. 用 `scoop api search "Option orElse"` 等命令按名称、库、receiver 与说明查找，再用 `scoop api show orElse --cone scoop.std` 读取全部重载。`--format json` 保留 signature、receiver、属性 getter/setter 可见性、documentation、source 和 imports；从 imports 的 coordinate 声明依赖，从 path 取得实际导入路径。关键字或含空格的名称使用反引号。
2. 默认查询安装根的宿主默认目标；应用选择其他目标时传 `--target`。自己的文档用 `scoop doc . --output-dir ./api-docs` 显式生成，再传 `--docs ./api-docs` 查询。文档缺失时工具会报告读取错误；不要把它解释为 API 不存在，也不要通过查询隐式重建工具链。
3. `<SYSROOT>/docs/index.html` 是离线页面入口，选择实际目标；`<SYSROOT>/src/<cone>/examples/README.md` 和库根 README 提供完整用法及运行条件。
4. 对未记录的行为、默认值正文或尚不确定的组合，用 `rg` 在该库的源码和测试定位实际声明。例如 `rg -n 'fun decodeFromString|fun encodeToString' <SYSROOT>/src/scoop.json`。查找 package、可见性、泛型条件、默认参数和 suspend 标记。文档的 `…` 表示省略默认表达式，不能当作实际值。
5. API 文档不代表库已进入当前项目。核对 Cone 的准确坐标，声明直接使用的库并添加 import；源码依赖使用项目指定版本。文档里没有的说明保持未知，不依据名称猜测资源责任或异常。

编辑器已有 Scoop LSP 时，利用诊断、补全、签名、跳转定义、引用和重命名核对项目语义。LSP 不代替正式 CLI 构建／测试，也不负责安装工具链。

## 按需求找库

下面是配套库的查找入口，不是固定 API 清单；以当前安装根实际文件为准。

| 需求                                              | Cone 与材料                                                                                    |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Option 类型、值类型、基本容器、StringBuilder、资源协议 | `scoop.core`；`language/*`、`text/strings`、`collections/list`                              |
| Result 类型、Option/Result 的映射、展平和惰性替代 | `scoop.std` 的 `src/results/`；操作选择见 [design.md](design.md#组合-optionresult保留失败信息) |
| 编译期嵌入文件／目录、读取构建环境                | `scoop.core` 的 `scoop.compiler` package；`language/compiler-inputs`                           |
| 集合算法、字典、字节／文本、日期与 Duration       | `scoop.std`；`collections/map` 和库 `examples/`                                                |
| 文件、目录、进程、环境变量、线程                  | `scoop.platform`；`io/files`、`io/resources`、`io/processes`、`io/environment`；整合输入与计算见 `apps/file-report` |
| 事件循环、任务、异步 I/O                          | `scoop.asyncio`；`async/tasks`                                                                 |
| JSON 与类型 codec                                 | `scoop.json`；`json/typed`                                                                     |
| GUI 连续点击 | `scoop.window.PointerEvent.Button` 最后一项为连续点击次数，释放保留对应按下的次数。平台层使用系统阈值和原生计数，提供三击及更多次点击；取消、拖动等结束旧序列。控件消费该次数，不重复实现固定双击 timer。共享编辑器已有双击词／三击逻辑行及继续拖选；行为与已验证的平台范围见 Window README、UI 实施记录。 |
| GUI 原生拖放接收 | macOS 的 `Window.registerDropTarget(DropHandlers)` 来自 `scoop.window`，每窗一个。offer 只看格式、允许操作、窗口逻辑位置和修饰键，返回 DropDecision；Drop 才将所选表示复制为 Text／Url／Files，并由 drop 的 Boolean 报告实际接受。不得把排队当完成或把已协商 Move 偷换成 Copy。handlers／enabled 变更使旧协商失效，periodicUpdates 可请求活动拖放期间持续更新，默认 false。close 幂等，关窗自动退休。接收回调使用 owner，同步完成；资源与完整用例见 Window README 和拖放测试 Cone。source、文本事务及其他平台仍按统一计划推进。 |
| UI 数据拖放目标 | macOS 的 `ComponentScope.dropTarget(area, DropAreaHandlers(...), enabled)` 返回 DropArea，借用当前挂载的 InputArea，卸载自动关闭。offer／drop 的第二个参数是区域局部位置，hover 提供可追踪的当前反馈；update 替换最新处理器和资格。使用 SurfaceWindow.hitInput 的同一 Accepted 路径，子目标拒绝只回退逻辑父目标，Drop 只交给一个仍匹配的接收者。关闭行为不关闭所借区域。见 UI 数据拖放规范、UI README 及 examples/drop-target 的完整接收／粘贴／语义组合。 |
| GUI 原生拖出源 | macOS 的 `Window.beginDrag(DragContent, effects, localData, onCompleted)` 只用于当前左键 Down／Dragged，数据复制后沿已有 owner 延迟通路启动。默认 Copy，localData 默认 None；仅真实 native 来源匹配当前 Application 会话时，目标 offer 才取得这个普通引用，不编码到 pasteboard。DragSession.effects 可在初始操作集合内调整后续协商，不改写已发生的实际结果。Dropped／NotDropped／Failed 在普通 owner 队列交付，Move 不自动删除业务数据。cancelPending 只取消未启动请求；close 撤销业务回调，AppKit 已启动会话保留 native 数据到实际结束。见 Window README 和 examples/drag-drop。 |
| UI 数据拖出来源 | macOS 的 `scope.dragSource(area, DragSourceOptions(threshold = 4.0)) { start -> ... }` 在逻辑距离达阈值后准备一次 DragSourceData，None 保留普通点击。来源借用本挂载区域，以 Surface claim 接管按下路径，卸载自动关闭。DragSourceData(content, effects, localData, onCompleted) 将实际来源与其完成回调配对；完成回调使用 named onCompleted 或最后的 trailing lambda。update 只换后续 prepare，isDragging 是可追踪的准备／活动状态，cancelPending 和 close 保留平台实际边界。非拖动入口及完整消费者见 scoop.ui/examples/drop-target，接口见 UI 拖放规范。 |
| GUI 文本拖放 | macOS 的 TextEditHost 默认接收 Text／Url Copy 或完整 Move，TextEditOptions、Input／Textarea／InputOtp 的 acceptDrops 单独关闭接收，drag: TextDragOptions 配置选区拖出、threshold、effects、copyPolicy 和 onCompleted。SelectableTextHost 与只读编辑器只导出 Copy；protectedText／遮罩默认不导出，独立 drag.copyPolicy.Allow 才显式开放拖出。匹配当前模型、修订、投影的 Accepted 排版提供落点 caret 和边缘滚动；DragDrop 复用 Paste／一般 policy，Copy 可裁剪，Move 有损即拒绝。同文档 Move 一次编辑一个 undo，原范围内无操作；跨文档各自历史，正文变化不使用旧 offset 删除。TextDragResult.SourceRetained 表示目标已接收而源未删除。组合输入透传，OTP 保留原模式／长度限制。完整消费者与平台边界见 scoop.ui/examples/drop-target 和 UI 拖放规范。 |
| GUI 文本与 Markdown 剪贴板 | `scoop.window` 的 Clipboard；`readText(mediaType = "text/plain")` 区分格式缺失和空文本，`writeText` 写纯文本，`writeTexts(List<ClipboardText>)` 同时提供一项内容的多个文本表示。使用无参数的具体 text MIME，按 ASCII 小写规范化；所有表示先检查且不允许重复格式或 NUL。纯文本归一为 LF，Markdown／HTML 等保留换行。接口要求活动应用的 owner 线程，不保活窗口、不解析正文；剪切先成功写出再删选区。示例与平台映射见库 README |
| GUI 系统交互偏好 | `scoop.window` 的 `ActiveApplication.interactionPreferences`／`Window.interactionPreferences` 与 `observeInteractionPreferences`；读取不可变快照，变化沿 owner 通知。None 与明确 false 分开，caret 闪烁为单次显隐间隔、零为常显，物理宽度与文字缩放／DPI 分开。观察者不内联通知，在消费者卸载时 close；系统未提供的值由 UI 显式回退。接口与平台来源见库 README。UI 根通过 `uiPreferences { ... }`／`UiWindow.preferences` 覆盖系统值，None 恢复跟随；`scope.interactionPreferences` 自动登记显示依赖，作用域观察自动注销。文本光标默认继承，局部覆盖使用 `caretBlink = Some(duration)`、`caretWidth = Some(width)`；后者为逻辑像素。见 scoop.ui README。 |
| GUI 无障碍基础 | macOS 的 `scoop.window.accessibility.*` 提供 Window provider、角色、属性、动作与身份；低层调用使用 createNode／update／reparent／publish；update 的 SemanticRelations 支持 labelledBy／describedBy／errorMessages／controls，显式名称／说明／错误优先，目标关闭后读回 relations 不含旧引用，UI 的相同关系声明继续沿用清理后的当前值。contentLabels 提供低于显式 name 和 labelledBy 的内容回退，实际名称环在 publish 时拒绝，修正后可重试。UI Element 在构造时用 `scope.semantics` 登记资源，在 update 中替换事实与实际 handler，在 Scene 中用 `context.semantics` 的 Rect／Path 重载登记 Accepted 几何与精确命中，按实际层叠顺序查询；Rect 保持可缓存，Path 不借用原路径并使当前场景不缓存；`routeSemanticsChildren` 设置子挂载归属，`bindFocus` 借用实际 FocusTarget。mergeLabels 合并纯静态文本叶子，保留可操作后代；ignored 仅忽略本节点，hidden 排除子树。逻辑发布不要求 drawable，卸载与根关闭自动退休，重排／合并／忽略保留身份。未知文本与空串分开，受保护文本不发布明文；名称／值及标签依赖变化清除旧几何，后续场景以 contentRevision 匹配后恢复。用法见 UI／Window README 和对应无障碍规范；其余控件、全文本／虚拟化、其他平台和读屏器验收仍按 UI 计划推进。 |
| GUI 滚动条行为 | ScrollArea／VirtualList／Textarea／MessageScroller 的 `scrollbarBehavior { ... }` 提供 FollowSystem／Static／Custom、Overlay／Gutter、闲置隐藏、宽度／透明度、独立命中厚度、出现条件、Page／Jump 和按住重复。`scrollbars` 的 Auto 只按原始正文溢出显示；Overlay 通过 ScrollPadding 留出可滚到的边缘空白，Gutter 固定占位，Textarea 默认 Gutter，其余默认 Overlay；`autoHide = Some(false)`、`widenOnHover = Some(false)`、`shadeOnHover = Some(false)` 可分别关闭效果，零时长立即切换。减少动画与减少透明度读取 UI 根偏好，具体默认值和示例见 Component README。 |
| GUI 文本菜单删除 | `MenuRole.DeleteSelection`／`TextEditCommand.DeleteSelection` 只删除非空选区，使用 TextEditHost 的当前策略和独立撤销事务，不修改剪贴板。空选区不做向前删除、不结束组合；可用性通过 `TextCommandAvailability.deleteSelection` 明确发布，不根据 IME 开关猜测。标准角色与同一输入路由的合同见 Window 菜单规范和 UI README。 |
| GUI 受保护文本 | `TextEditOptions`、Input／Textarea／InputOtp 的 `protectedText=true` 独立于 textMask 显示投影；显示密码时保留标记。默认 Copy／Cut 禁用，显式 TextCopyPolicy.Allow 只开放复制，不解除保护用途；粘贴、删除、策略和撤销沿同一宿主。组合输入透传，不另建显示文档。见 UI 与 Component README。 |
| GUI 控件默认语义 | Label、Button、Checkbox、Switch／Toggle／RadioGroup／ToggleGroup、Input／Textarea、SelectableStyledText 与 Typography 的 Props 和 builder 使用 `semantics = ComponentSemantics(name = Some("名称"), automationId = Some("id"))`，类型从 `scoop.ui.component` 导入；还可覆盖 description、error、relations、hidden／ignored，真实角色、值、状态与动作由控件提供。Button 合并静态标签，保留独立操作子项；Checkbox 调用原 Binding，输入框沿原 TextEditHost 的模型、策略、undo 与菜单。placeholder 不作为名称。包装继续传递配置，当前原生 provider 为 macOS；其他控件及表单状态按计划接入。组合控件的后代 Element 可为逻辑祖先持有的 handle 登记 Rect／Path，不能登记兄弟或跨根资源。详见 component／UI README。 |

| GUI 表单语义关系 | FieldTarget 同时关联真实标签、说明和错误；Button／Checkbox／Switch／Radio 条目／Input／Textarea 沿原 FieldLink 发布 labelledBy／describedBy／errorMessages。Field 默认采用 label 的目标，普通 title 可用 target 显式指定；fieldTitle／fieldDescription／fieldError 的 target 与 FieldLabel／FieldText／FieldError 的 semantics 提供独立组合及覆盖。多个标签按关联登记顺序解析，显式非空关系和显式文字优先；卸载／换绑刷新当前节点，不依赖绘制或获取焦点。自定义标签的独立按钮保留可访问性。Field.required 同时更新实际控件的 SemanticProperties.required；AppKit 提供只读 Required 属性，不执行校验、不改变正文／焦点，已报告必填时不重复朗读 requiredText。invalid 等状态及其他控件仍按计划推进。用法见 Component README 与规范第 15／23 节。 |
| GUI 选择语义 | Switch／Toggle 的原生切换调用实际 Binding；RadioGroup 重复选择不取消、不重复调用 setter。RadioGroup／ToggleGroup 的组和条目均有 ComponentSemantics，单选／多选、稳定 key、roving 焦点及 FieldTarget 保持原行为；绑定拒绝或转换后报告实际值，删除退休旧对象。AppKit 使用 Button 加 Switch／Toggle subrole、RadioButton 与 RadioGroup。接口和定向验证见 Component README 与选择控件测试 Cone。 |
| GUI 语义滚动 | ScrollArea／VirtualList 的 semantics 覆盖名称与标识，Input／Textarea 复用原编辑视口；视觉 Hidden 不移除非零轴的滚动能力。自定义 ScrollBar 节点在正常 update 与 Accepted 回调后调用 scoop.ui 的 updateScrollBar(ScrollViewport, SemanticOrientation, lineStep, properties, relations)，或 scoop.ui.text 的 TextEditHost 重载。Window 的 SemanticScrollRange 保存逻辑 offset／viewport／content，setScrollPosition 接收 0～1 比例，scrollBackward／scrollForward 调用实际步长；属性读取不滚动，位置等待 Accepted 后更新，换绑撤下旧范围。已挂载 UI 语义节点自动提供 reveal，使用 Scene 的 Accepted 几何与原 ScrollReveal 路径，保持焦点和选区；等待后续帧返回 Busy，真正可见才 Completed。无需 InputArea，默认显露不阻止静态标签合并；显式 SemanticActions.reveal 可覆盖实现。AppKit 使用公开的 Scroll to Visible 自定义动作。导入与验证范围见 Window／UI／Component README。 |
| GUI 无障碍文本 | 自定义编辑 Element 在正常 update 中调用 `SemanticsHandle.updateTextEditor(host, properties, relations)`（扩展从 `scoop.ui.text` 导入），必须使用同一挂载的 TextField 与 TextEditHost；正文、方向选区和实际焦点复用现有模型，调用方提供名称／说明／错误与关系。AppKit 纯文本范围使用 UTF-16，公共 selection 使用 Utf8Selection；只读可选择，受保护文本不公开正文／长度／选区。原生写入使用 TextEditSource.Accessibility，与 Paste 共用外来文本清洗、过滤和 undo；单纯选区的 composition 收尾仍是 EditCommand。正文几何、单位导航、保留范围和其他 provider 继续按计划接入，实际范围见 UI／Window README。 |
| GUI 只读文本菜单 | `SelectableTextOptions.contextMenu` 使用相同的 TextContextMenuOptions，默认 PreferSystem；SelectableTextHost.showContextMenu() 提供 Copy／SelectAll，按 Accepted 选区或插入位置定位。SelectableStyledText.options 与 Typography.selection 直接透传；复制保留 alternateText，正文变化取消旧菜单，辅助点击不激活行内主按钮。见 UI 与 Component README。 |
| GUI 只读文本语义与菜单动作 | 同一挂载的 Text 节点通过 `updateSelectableText` 发布 SelectableTextHost 的正文／选区；在正常 update 和 onSelectionChanged 后刷新，alternateText 与占位符沿现有区间映射转换。只读宿主不提供写入，部分替代文字选择覆盖整个对象。SelectableStyledText 已自动登记及刷新语义，并保留最新 onSelectionChanged；Typography 的 selectable 两分支传递同一 ComponentSemantics，分支替换退休旧对象。updateTextEditor／updateSelectableText 在菜单模式启用时提供 SemanticActions.showContextMenu，AppKit 映射 ShowMenu；该明确请求可聚焦目标，以 Accepted 锚点打开原菜单会话，未就绪不假报成功。示例与生命周期见 UI／Window README。 |
| GUI 文本上下文菜单 | `scoop.ui.text` 的 `TextEditOptions.contextMenu` 与 Input／Textarea／InputOtp 的 `contextMenu { ... }` 提供 Disabled／StandardNative／PreferSystem，默认 PreferSystem；selection 控制选区外点击，labels 可逐项本地化。右键、平台辅助点击和菜单键走同一目标，`TextEditHost.showContextMenu()` 使用有效 caret／选区锚点。动作绑定发起模型，失焦／换绑／禁用／关闭时取消；复制、粘贴、删除和撤销仍用宿主策略。低层 `ActiveApplication.textContextMenuTemplate` 返回纯值模板，SystemTemplate 只代表可执行的系统子集，未实现的拼写／翻译服务不出现在菜单中。见 Window、UI 和 Component README。 |
| GUI 主题与系统偏好 | `scoop.ui.style` 的 `ThemeSource.read(scope)` 同时读取基础主题与 UI 根的减少动画、高对比度、减少透明度和文字缩放；自绘组件也使用此入口。`current.value` 保留基础值，`resolve(ThemePreferences(...))`／`Theme.withPreferences` 供纯值调用，不叠加字号。第三方 Theme 需同时更新 tokens 和实际部件。局部显式样式由应用决定；API 与对比度规则见该库 README 和主题规范第 6.1 节。 |
| Markdown 文档、解析、事务与生成 | 独立仓库 [scoop-lang/scoop-markdown](https://github.com/scoop-lang/scoop-markdown) 的 [scoop.markdown](https://github.com/scoop-lang/scoop-markdown/blob/main/scoop.markdown/README.md)；从该仓库的 Cone 目录声明源码依赖，无 UI 文件处理示例位于 `scoop.markdown/examples/source` |
| Markdown 编辑、展示与扩展 | 同仓库的 [scoop.ui.markdown](https://github.com/scoop-lang/scoop-markdown/blob/main/scoop.ui.markdown/README.md) 提供源码／所见即所得／双栏与 inline／block 扩展；完整窗口示例位于 `markdown-editor`，公开操作见 [API 导航](https://github.com/scoop-lang/scoop-markdown/blob/main/docs/API-GUIDE.md) |
| HTTP 请求、服务端、路由及流式交互                 | `scoop.http.client`、`scoop.http.server`；`http/get`、`http/existing-connection` 及各库 README |
| HTTP 协议与异步传输适配                           | `scoop.http`、`scoop.http.asyncio`；仅在需要协议层时使用                                       |
| 命令行参数与测试                                  | `scoop.cli`、`scoop.test`；`cli/options`、`test/assertions`                                    |
| 正则、URL、IDNA、字符集与压缩                     | `scoop.regex`、`scoop.url`、`scoop.idna`、`scoop.charset`、`scoop.compression` 的 README／示例 |
| 密码与 TLS                                        | `scoop.crypto`、`scoop.tls`、`scoop.tls.asyncio` 的示例与 API 文档                             |
| 数学、大整数和十进制数                            | `scoop.math` 的 README／示例                                                                   |
| 日志与控制台 provider                             | `scoop.log`、`scoop.log.console` 的 README／示例                                               |
| 终端、异步终端与交互行编辑                        | `terminal/output`；`scoop.terminal`、`scoop.terminal.asyncio`、`scoop.readline` 的示例           |
| UI、组件及桌面应用                               | `ui/state`；`scoop.ui` 及实际使用的组件/桌面 Cone 的 README 与 examples；独立控件修改借用模型前可调用 `State.checkMutation()`，`SceneContext.drawTextLayout` 在既有文字缓存路径绘制自有排版；使用当前目标可用的库  |
| 原生宿主的几何与文本查询 | `scoop.graphics.native` 的 `@Unsafe CanvasGeometry.withNativeGeometry` 借出 NativeGeometry，提供精确 contains、toLocal 和完整／可见 mapRect；`scoop.graphics.text.native` 的 `@Unsafe TextLayout.withNativeLayout` 借出同一排版的 hit／caret／boxes／单位与行查询。原生消费者以配套 retain／release 管理跨借用和跨线程寿命，不回调 Scoop；普通 struct 复制不拥有额外引用。文本偏移仍为布局正文 UTF-8，投影、层叠、内容版本及 Accepted 发布由宿主负责；完整合同见 graphics／text README 和对应规范 |

## JSON 与 DTO

从 `json/typed` 复制实际的 companion 声明：

```scoop
struct User(val name: String, val note: String? = None) {
    public companion object : ValueEncoder<User>, ValueDecoder<User>
}
```

`Json.encodeToString`／`Json.decodeFromString<User>` 使用公开 companion 能力；其他入口也可能接受显式 codec。struct 本身不意味着任意序列化入口都可用。增改字段时确认字段类型具有相应能力，并为缺字段行为提供真实默认值。JSON null、缺失字段和 Scoop 的 Option 分别遵循对应 API 合同。

## 异步与网络

`async/tasks` 展示普通 main 调用 `eventLoop()` 并通过 `loop.run { ... }` 进入挂起上下文；回调中的 `this` 是 CoroutineScope，不是额外的 lambda 参数。`async` 的回调类型是 `suspend CoroutineScope.() -> T`，有 receiver 的函数类型与 `suspend () -> T` 不同。需要返回值用 `async`／`await`，无业务结果用 `launch`／`join`。不要发明 `runBlocking` 或全局后台任务入口。

`http/get` 同时提供本机服务端与客户端。保持 `withHttpClient`、`withResponse` 等生命周期作用域，等服务开始监听后再运行客户端。读取正文遵守一次消费和读取上限的实际 API；例如示例的 `body.text(1_048_576L)`。这种 I/O 上限不代表所有业务请求都应使用同一数值。

已有 `AsyncHttp1Connection` 时，`http/existing-connection` 展示 `HttpApplication.executeHttp1`：把当前尚未消费的 incoming request、真实连接元数据与请求 ID 交给同一套 middleware／路由／响应执行。调用方负责接收请求头、连接收尾和循环公平性；单条执行建立自己的请求作用域，完成清理与观察后返回。`HttpRequestOptions` 独立配置正文限制、总期限和完成回调，也可放入 `httpServerOptions { request { ... } }`。此入口借用连接，WebSocket 升级需要 `HttpServer` 的连接接管路径。

更复杂的请求、路由、TLS、WebSocket 或取消处理按库 README 和具体声明实现。不要从同步函数名推导挂起版本，也不要将旧示例中的类型名移植到另一版工具链。

## 文件、终端与资源

从 `io/files` 查 `Path`、`OsString` 和文本／字节操作；从 `io/resources` 查正常与异常路径的关闭。路径表示、编码和字符串不是可任意互换的类型。只清理当前任务实际创建的临时资源。

外部命令用 `io/processes` 查 `Command` 与 `ChildProcess`。需要有限等待时使用 `child.communicate(timeout: Duration, input: List<Byte>? = None)`；同一单调期限覆盖输入、两个输出和进程等待。`ProcessTimeoutException.stdout/stderr` 保留部分字节，返回异常前库收尾所拥有的 child 与端点；后代进程不属于其管理范围。正常退出、非零退出、启动失败与超时分别处理；清理还失败时保留 `ResourceUseException.primaryFailure/cleanupFailure`。普通 `close` 不杀进程，不能用外包线程后放弃 join 的办法模拟超时。

终端／行编辑示例可能需要真实 TTY，异步 HTTP 示例需要活动事件循环与服务，线程共享状态需要同步。用符合各自前提的方式验证，不能把无 TTY 或服务未启动造成的失败归为语言错误。
