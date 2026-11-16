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
| GUI 文本与 Markdown 剪贴板 | `scoop.window` 的 Clipboard；`readText(mediaType = "text/plain")` 区分格式缺失和空文本，`writeText` 写纯文本，`writeTexts(List<ClipboardText>)` 同时提供一项内容的多个文本表示。使用无参数的具体 text MIME，按 ASCII 小写规范化；所有表示先检查且不允许重复格式或 NUL。纯文本归一为 LF，Markdown／HTML 等保留换行。接口要求活动应用的 owner 线程，不保活窗口、不解析正文；剪切先成功写出再删选区。示例与平台映射见库 README |
| GUI 系统交互偏好 | `scoop.window` 的 `ActiveApplication.interactionPreferences`／`Window.interactionPreferences` 与 `observeInteractionPreferences`；读取不可变快照，变化沿 owner 通知。None 与明确 false 分开，caret 闪烁为单次显隐间隔、零为常显，物理宽度与文字缩放／DPI 分开。观察者不内联通知，在消费者卸载时 close；系统未提供的值由 UI 显式回退。接口与平台来源见库 README。UI 根通过 `uiPreferences { ... }`／`UiWindow.preferences` 覆盖系统值，None 恢复跟随；`scope.interactionPreferences` 自动登记显示依赖，作用域观察自动注销。文本光标默认继承，局部覆盖使用 `caretBlink = Some(duration)`、`caretWidth = Some(width)`；后者为逻辑像素。见 scoop.ui README。 |
| GUI 无障碍基础 | macOS 的 `scoop.window.accessibility.*` 提供 Window provider、角色、属性、动作与身份；低层调用使用 createNode／update／reparent／publish；update 的 SemanticRelations 支持 labelledBy／describedBy／errorMessages／controls，显式名称／说明／错误优先，目标关闭后读回 relations 不含旧引用。contentLabels 提供低于显式 name 和 labelledBy 的内容回退，实际名称环在 publish 时拒绝，修正后可重试。UI Element 在构造时用 `scope.semantics` 登记资源，在 update 中替换事实与实际 handler，在 Scene 中用 `context.semantics` 的 Rect／Path 重载登记 Accepted 几何与精确命中，按实际层叠顺序查询；Rect 保持可缓存，Path 不借用原路径并使当前场景不缓存；`routeSemanticsChildren` 设置子挂载归属，`bindFocus` 借用实际 FocusTarget。mergeLabels 合并纯静态文本叶子，保留可操作后代；ignored 仅忽略本节点，hidden 排除子树。逻辑发布不要求 drawable，卸载与根关闭自动退休，重排／合并／忽略保留身份。未知文本与空串分开，受保护文本不发布明文；名称／值及标签依赖变化清除旧几何，后续场景以 contentRevision 匹配后恢复。用法见 UI／Window README 和对应无障碍规范；内置控件、全文本／虚拟化、其他平台和读屏器验收仍按 UI 计划推进。 |
| GUI 滚动条行为 | ScrollArea／VirtualList／Textarea／MessageScroller 的 `scrollbarBehavior { ... }` 提供 FollowSystem／Static／Custom、Overlay／Gutter、闲置隐藏、宽度／透明度、独立命中厚度、出现条件、Page／Jump 和按住重复。`scrollbars` 的 Auto 只按原始正文溢出显示；Overlay 通过 ScrollPadding 留出可滚到的边缘空白，Gutter 固定占位，Textarea 默认 Gutter，其余默认 Overlay；`autoHide = Some(false)`、`widenOnHover = Some(false)`、`shadeOnHover = Some(false)` 可分别关闭效果，零时长立即切换。减少动画与减少透明度读取 UI 根偏好，具体默认值和示例见 Component README。 |
| GUI 文本菜单删除 | `MenuRole.DeleteSelection`／`TextEditCommand.DeleteSelection` 只删除非空选区，使用 TextEditHost 的当前策略和独立撤销事务，不修改剪贴板。空选区不做向前删除、不结束组合；可用性通过 `TextCommandAvailability.deleteSelection` 明确发布，不根据 IME 开关猜测。标准角色与同一输入路由的合同见 Window 菜单规范和 UI README。 |
| GUI 受保护文本 | `TextEditOptions`、Input／Textarea／InputOtp 的 `protectedText=true` 独立于 textMask 显示投影；显示密码时保留标记。默认 Copy／Cut 禁用，显式 TextCopyPolicy.Allow 只开放复制，不解除保护用途；粘贴、删除、策略和撤销沿同一宿主。组合输入透传，不另建显示文档。见 UI 与 Component README。 |
| GUI 只读文本菜单 | `SelectableTextOptions.contextMenu` 使用相同的 TextContextMenuOptions，默认 PreferSystem；SelectableTextHost.showContextMenu() 提供 Copy／SelectAll，按 Accepted 选区或插入位置定位。SelectableStyledText.options 与 Typography.selection 直接透传；复制保留 alternateText，正文变化取消旧菜单，辅助点击不激活行内主按钮。见 UI 与 Component README。 |
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
| 原生宿主的精确几何查询 | `scoop.graphics.native` 的 `@Unsafe CanvasGeometry.withNativeHitTest` 借出 NativeGeometryHitTest；原生消费者以配套 retain／release 管理跨借用和跨线程寿命，contains 只读查询已捕获的变换、路径 clip 与 viewport，不回调 Scoop。普通 struct 复制不拥有额外引用，层叠与 Accepted 发布由宿主负责；完整合同见 graphics README 和 Surface 规范 |

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
