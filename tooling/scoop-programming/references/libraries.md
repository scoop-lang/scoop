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
| Markdown 源文档、解析、事务与生成 | `scoop.markdown` 的 README 与 `examples/source`；`parseMarkdown`／`MarkdownDocument.parse` 显式解析；`MarkdownProfile.Editor` 开启公式、front matter 和脚注，`MarkdownExtension`／`mdBookDirectives()` 注册自定义语法；`parsed.region(range)` / `region.after(change)` / `region.parse()` （MarkdownRegion／ParsedMarkdownRegion，使用全局源码坐标）； `MarkdownSyntax.inlineContentAt(position)`； `nodes()`／`parent(id)` 查询，`transform(parsed)` 修改结构；indentListItem／outdentListItem／unwrapListItem 调整列表层级，setBulletList／setOrderedList 显式转换并编号，保留正文和精确逆向；表格支持 insertTableColumn／removeTableColumn／setTableColumnAlignment，setTableCells 写入普通文字矩形并按需扩展，pasteTableTsv 支持带引号的 Tab／换行，整次修改可精确撤销。`inlineContent(id)` 提供段落／标题／单元格的显示文字，`replaceText`／`setStyle` 准备带源码差量与新选区的 `MarkdownInlineEdit`，用 `applyTo(document)` 发布；after 提供可继续输入的局部内容，nodes 可读取当前局部语法；空文件使用 root.id 取得位置 0 的空区域，change.mapBack 将新锚点映射到基线。`content.edit { ... }` 将多次语义操作准备成一次事务，restore 恢复同一区域的确切源码。`splitBlock(id, range)`／`joinBlocks(first, second)` 准备带结果光标的 MarkdownBlockEdit，拆分继承列表、任务与引用容器，合并要求同容器相邻段落／标题。UI 可交给同一 editor.apply。`markdownDocument`／`markdownText` 使用 receiver DSL 生成。`MarkdownProcessingSession`／`MarkdownProcessor` 配合应用 resolver 显式展开内容，`mdBookProcessor()` 支持 include，结果带来源映射与版本依赖。未安装的新库使用 path 依赖；解析不隐式读 include 或生成图片 |
| Markdown 编辑状态、源码与富文本 | `scoop.ui.markdown` 的 README、IMAGES.md 和 `examples/editor`；`markdownEditor(viewState, controller)` 提供 Source／Wysiwyg／SideBySide，viewState 保留两侧焦点与滚动，支持右侧只读、分栏比例及源码锚点同步。`showSource(range)` 验证并聚焦源码起点，组合入口只通知一次正文变化。`markdownSourceEditor`／`markdownPreview`／`markdownRichEditor` 可独立使用，Host 支持自绘及借用 initialScroll。`MarkdownEditorState` 共享选区、事务、撤销和组合；SourceInput 提供局部源码 native 快照，RichInput 按行内显示映射连续输入。RichHost 复用预览虚拟化，活动段落／标题／单元格立即显示局部编辑；块命令及跨块交互仍按实施记录推进。`MarkdownParseController` 借用 executor／proxy 合并后台解析、展开与显示投影；current 用于当前结构命令，latest 可保留旧显示。measurement.sourceRevision 是实际显示版本，parsed 可仍是后台基线。`MarkdownImageController` 按可见请求共享图像，提供本地 PNG／JPEG、Mermaid CLI、MathJax＋rsvg-convert 适配；`MarkdownImageRenderer` 接收自定义扩展，外部工具由宿主预先配置 |
| HTTP 请求、服务端、路由及流式交互                 | `scoop.http.client`、`scoop.http.server`；`http/get`、`http/existing-connection` 及各库 README |
| HTTP 协议与异步传输适配                           | `scoop.http`、`scoop.http.asyncio`；仅在需要协议层时使用                                       |
| 命令行参数与测试                                  | `scoop.cli`、`scoop.test`；`cli/options`、`test/assertions`                                    |
| 正则、URL、IDNA、字符集与压缩                     | `scoop.regex`、`scoop.url`、`scoop.idna`、`scoop.charset`、`scoop.compression` 的 README／示例 |
| 密码与 TLS                                        | `scoop.crypto`、`scoop.tls`、`scoop.tls.asyncio` 的示例与 API 文档                             |
| 数学、大整数和十进制数                            | `scoop.math` 的 README／示例                                                                   |
| 日志与控制台 provider                             | `scoop.log`、`scoop.log.console` 的 README／示例                                               |
| 终端、异步终端与交互行编辑                        | `terminal/output`；`scoop.terminal`、`scoop.terminal.asyncio`、`scoop.readline` 的示例           |
| UI、组件及桌面应用                               | `ui/state`；`scoop.ui` 及实际使用的组件/桌面 Cone 的 README 与 examples；独立控件修改借用模型前可调用 `State.checkMutation()`，`SceneContext.drawTextLayout` 在既有文字缓存路径绘制自有排版；使用当前目标可用的库  |

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
