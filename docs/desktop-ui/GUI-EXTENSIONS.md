# SVG 图标、字体选择与桌面集成设计

日期：2026-11-08。状态：按批次实施中，实际交付和平台验证见[实施进度](GUI-EXTENSIONS-PROGRESS.md)。本文记录已确认的范围、现有实现依据、拟定接口与验收条件；不把下文全部新增 API 视为已经交付。实施每个批次时，先更新所属 spec，再修改实现、调用方和测试。

本文接续[原生桌面能力设计](DESIGN.md)，覆盖 Skia SVG／Lucide、系统与临时字体、Taskbar／Dock、桌面通知，以及 Windows native desktop demo 的圆形窗口问题。窗口、菜单、帧提交和 UI 挂载继续遵守既有规范；本轮不增加语言、编译器 IR 或 runtime 专用机制。

## 1. 已确认的需求与边界

1. SVG 使用仓库现有 Skia 的 SVG 模块，提供静态图形消费能力及 Lucide 图标库；不实现完整 SVG 标准，也不引入另一套 SVG 引擎。
2. FontCollection 需要支持列出和选择系统字体；临时加载字体文件或内存数据；列出并选择字体内的字面、命名实例和可变轴。字体选择必须落实到实际字体，不能只保存一个显示名称。
3. Taskbar／Dock 的 icon、caption、menu、progress 均纳入设计。caption 包含图标角标文字／数量，以及系统显示名称、窗口标题或原生悬停说明；这些含义分别表达。
4. 桌面集成只提供 OS／桌面环境已有的原生能力。缺失的角标、进度条和菜单不通过自绘图标、定时动画、额外窗口或桌面扩展补齐。
5. Desktop notification 使用系统通知服务，覆盖提交、替换、撤回、动作及必要的应用注册；不使用 UI 内的 Toast 冒充系统通知。
6. Windows 问题的报告是：native desktop demo 的圆形窗口仍有边框，外观不是真正的圆形。先区分 demo 配置与后端缺陷，再修复实际原因；本设计不宣称已经在 Windows 重现或定位根因。

平台范围沿当前 GUI：Darwin aarch64／x86_64、三个 Windows x86_64 target，以及 Linux GNU x86_64 的 Wayland／X11。Linux musl 不新增 GUI 资源。公共接口根据真实能力返回结果，不以三平台功能对齐作为交付前提。

## 2. 当前实现与改动位置

| 范围 | 已有事实 | 本轮工作 |
| --- | --- | --- |
| SVG | 固定 Skia `f326a1e25fc736704063922b9ce76b527b1c5efe` 含 SVG DOM；[构建脚本](../../scripts/build_skia.py) 关闭 SVG／Expat，未发布 SVG 模块和头文件 | 启用依赖、增加普通图形 Cone、UI 展示和 Lucide 资源 |
| 图形接入 | [withSkiaCanvas](../../sysroot/src/scoop.graphics/src/native/canvas.scoop) 提供受作用域约束的原生画布借用；graphics.text 已使用相同通路 | SVG 直接绘制到现有 CPU／GPU Canvas |
| 字体装载 | [FontCollection](../../sysroot/src/scoop.graphics.text/src/fonts.scoop) 已有 `register(bytes, family, collectionIndex)`，native 复制数据并建立字体；Paragraph 保留快照 | 补目录、元数据、精确选择、复用已打开字面和选择替换 |
| 字体变体 | [TextStyle](../../sysroot/src/scoop.graphics.text/src/style.scoop) 已有 weight／width／slant、FontVariation 与 OpenType feature，已有实际轴绘制测试 | 补可发现的轴／命名实例和有明确范围的选择入口 |
| UI 字体更新 | UiWindow.defaultFonts 可替换；[FontWatch](../../sysroot/src/scoop.ui/src/layout/dependencies.scoop) 仅在已经请求的帧中检查 revision | 字体登记改变后按需唤醒实际使用者，不能等待偶然重绘 |
| 桌面对象 | window 已有 owner、native lease、DesktopImage、Menu 和应用级 UI 挂载；托盘独立实现 | 复用生命周期和图像／菜单数据，增加 Taskbar／Dock 与通知 |
| 应用部署 | [scoop-bundle](../scoop-bundle/DESIGN.md) 已有 macOS app、Linux desktop metadata 和 Windows NSIS／MSI | 补本轮实际需要的应用标识、通知注册及原生启动任务字段 |
| Windows 形状 | 有 SetWindowRgn、DWM 外观接口、Surface Accepted 区域发布；[Windows Surface](../../sysroot/src/scoop.surface/native/windows/surface.cpp) 已使用 DirectComposition 和预乘 alpha | 检查 demo 与 Client 窗框组合，修复真实边框／区域问题 |

### 2.1 Cone 分层

| Cone | 职责与依赖 |
| --- | --- |
| `scoop.graphics.svg`（新增） | 依赖 graphics 及实际使用的基础库，拥有 SVG 文档和 Canvas 绘制扩展；不依赖 UI、window 或字体选择 UI |
| `scoop.icons.lucide`（新增） | 依赖 graphics.svg 的普通资源描述，提供选定的 Lucide SVG；不依赖组件库 |
| `scoop.graphics.text`（已有） | 扩展字体目录、字面与变体查询／装载，继续独立于窗口 |
| `scoop.ui`（已有） | 接收字体变更，使用现有依赖追踪、更新队列及资源作用域 |
| `scoop.ui.component`（已有） | 增加 SVG／Icon 展示，消费 Lucide；字体选择示例复用现有选择器、滑块和文本控件 |
| `scoop.desktop`（新增） | 依赖 window／platform／std；提供 Taskbar／Dock 与 `scoop.desktop.notification`，共用应用 owner 和原生资源收尾，不依赖 Skia 或托盘 |
| `scoop.ui.component.desktop`（新增） | 依赖 desktop、ui 和 component，提供应用级原生条目与通知控制器；普通组件不反向依赖它 |

Taskbar／Dock 与通知放在同一个桌面扩展 Cone 内，按 package 和 native 文件分开，不另建平台插件系统。不同 OS 的特有入口放在 `scoop.desktop.platform.macos`／`windows`／`linux`。现有 `scoop.tray` 与 `scoop.ui.component.tray` 保持其托盘职责。

`scoop.icons.lucide` 的数据层不依赖 UI，避免 component 为默认图标依赖 Lucide 后产生循环。各 native 层只接收 DesktopImage 的普通像素描述；SVG 到该描述的显式光栅化辅助函数放在 UI 桌面适配层。

## 3. Skia SVG 与 Lucide

### 3.1 SVG 范围

首批验收覆盖 `svg`、`g`、path 的标准路径命令、circle、ellipse、rect、line、polyline、polygon，以及 viewBox、preserveAspectRatio、变换、填充、描边、继承属性、currentColor 和透明度。Skia 已有的渐变、裁剪、mask、pattern 与滤镜可沿后端使用，选择少量代表素材验证其接入，但不承诺它们的完整标准覆盖。

固定后端缺少完整 CSS 样式表／选择器、symbol、marker、non-scaling-stroke、脚本、动画和 foreignObject 等能力。首批不接入 SVG 文本字体和外部资源加载；图标、文字排版仍分别走 SVG 和 graphics.text。内联 `style` 仅支持 Skia 能识别的属性。

解析失败、非 SVG 根及原生构造失败报告普通图形错误。Skia 会跳过部分未知元素／属性，此行为明确写入文档，不把加载成功解释为完全呈现。内部图标通过源资产检查和实际绘制测试保证正确；运行时不另建 XML／CSS 验证器，也不逐帧重新遍历文档校验。

### 3.2 图形 API 与资源

拟定的最小接口形状：

```text
SvgDocument.parse(source: String): SvgDocument
SvgDocument.decode(bytes: Array<Byte>): SvgDocument
SvgDocument.intrinsicSize: Size?
SvgDocument.viewBox: Rect?
SvgDocument.isClosed: Boolean
SvgDocument.close()

Canvas.drawSvg(document, viewport: Rect, options: SvgDrawOptions)
SvgAsset(source: String)
```

SvgAsset 是可共享的不可变源码描述，不持有 native 资源。SvgDocument 是解析后的 Closeable，限创建线程串行使用；输入在解析时复制或转换为自有存储，不能保留移动 GC 内存地址。文档关闭不销毁已经记录进 Drawing 的 Skia 绘图资源。

`intrinsicSize` 仅在根尺寸可独立解析且为有效正尺寸时存在；百分比尺寸不伪造自然大小。viewBox 是文档图形坐标，不等同于像素尺寸。缺少自然大小时，UI 使用显式 naturalSize，或在存在有效 viewBox 时取其宽高比；两者都不存在则要求调用者提供尺寸。

viewport 是逻辑坐标。普通 draw 保留文档 viewBox／preserveAspectRatio 的内部映射；UI 的 ContentFit 先计算文档 viewport 的外部放置，不重复映射 viewBox。DPI 仅由渲染目标施加。固定根宽高的 SVG 需要明确的映射，不能只调用 SkSVGDOM.setContainerSize；该调用只影响相对尺寸根。

SvgDrawOptions 包含可选 currentColor、可选 rootStrokeWidth 和整体 opacity。未指定 currentColor 时保留文档自己的颜色语义；指定时作为本次根 color 覆盖，子节点明确声明的 color／fill／stroke 仍按 SVG 继承规则处理。rootStrokeWidth 以根图形坐标覆盖根 stroke-width，子节点自己的 stroke-width 仍优先；这个公开入口供独立的 UI Icon 调整线宽，不要求跨 Cone 访问 native 内部状态。整体 opacity 使用已有 saveLayer 合成，不逐个图形乘透明度。

原生绑定通过 withSkiaCanvas 借用画布，正常与异常路径恢复保存状态。需要临时修改根表现属性时保存并恢复原属性，本次颜色或图标描边参数不能留给下一次绘制。解析与绘制分开，颜色变化不替换源码或重新解析。原生绘制默认沿 `independentTiles = false` 的既有合同，不因属于图标就为任意 SVG 宣称可独立分块。

### 3.3 UI 与图标参数

`svgView(document, configure)` 借用调用者持有的文档，复用 ImageView／PathView 的约束、ContentFit、Alignment、clip 和 key 规则。它没有文件／网络装载或重试行为，也不擅自关闭借用文档。

`icon(asset: SvgAsset, configure)` 在挂载时拥有解析资源，并在来源改变时替换和关闭旧文档。相同 asset、仅颜色／尺寸／线宽变化时复用文档；卸载关闭资源，不建立全局常驻 DOM 表。Icon 的默认视觉尺寸为 24 逻辑单位，SVG 图形网格保持原始 24×24。

Icon builder 提供 size、color、strokeWidth、absoluteStrokeWidth、key。Lucide 默认 strokeWidth 为图形坐标中的 2，linecap／linejoin 保留资源中的 round；普通缩放使线宽一起变化。absoluteStrokeWidth 为 true 时，在本轮实际等比缩放下将逻辑线宽换算回图形坐标；不把它宣称为任意 SVG 的 vector-effect 支持。零尺寸图标不绘制，也不产生除零换算。

颜色／尺寸通过现有 ActionContentStyle 或显式主题来源传入。图标本身不添加输入、焦点或时钟。方向性图标由调用者按 LayoutDirection 选择／变换，其他图形不自动镜像。Spinner 使用相同 SVG 资产替换手写图形，旋转、Reduced、可见性和停帧继续由现有动画实现负责。

### 3.4 Lucide 资源与依赖构建

调研检查的 `lucide-static 1.70.0` 含 2322 份 SVG，全部为 24×24，根样式为 fill=none、stroke=currentColor、stroke-width=2，绘图元素只有本节基本形状；少量子元素有自己的 fill 覆盖。这个结果仅说明该版本的资源范围，不是渲染验证结果。

首批收录现有组件使用的图标及常用操作／导航图标，基准清单为 check、x、chevron-left／right／up／down、ellipsis、search、loader-circle、calendar-days、panel-left、info、circle-check、triangle-alert、octagon-x、plus、minus、arrow-left／right／up／down、file、folder、save、copy、trash-2、download、upload、settings、menu、external-link；迁移时补入实际已使用而清单遗漏的上游图标。维护清单和固定版本，不顺带引入全量目录。

保存原始 SVG，使用维护脚本生成每个图标独立的命名入口；入口引用 SvgAsset 和 includeStr，不在 Cone 初始化时解析全库，也不通过全量运行时枚举表关联所有资源。实际消费者检查最终链接体积，不能仅凭源码结构宣称未引用图标一定剔除。

许可证沿上游保留 ISC 及适用的 Feather MIT 声明。源码选择、声明生成和许可随图标库交付；构建普通应用不下载 Lucide。

Skia 准备脚本启用 SVG／Expat，使用固定 Skia DEPS 的 Expat 版本，显式关闭 system Expat 并合并对应静态依赖，保持三平台依赖组合明确。增加 svg、skresources 及其实际构建输入／输出，发布所需头文件和许可证。更新判断仍在解包、工具探测和构建之前；普通应用消费已准备的 sysroot。

## 4. 字体目录、临时加载与变体选择

### 4.1 四种不同的选择

| 用户所选内容 | 含义 | 处理方式 |
| --- | --- | --- |
| 字体家族与普通字面 | 同一 family 中的 Regular、Bold、Italic、Condensed 等 | 列出真实字面与 weight／width／slant，打开被选中的实际 SkTypeface |
| 字体集合文件 | TTC／OTC 中的多个字体字面 | 列出文件内字面及 collectionIndex；选择对应字面，不把索引当作粗细值 |
| 可变字体命名实例 | 字体保存的 Regular、SemiBold、Display 等轴坐标组合 | 列出实例名称及完整坐标，选择后实例化实际字体 |
| 可变轴 | wght、wdth、opsz、slnt、ital 以及字体自定义轴 | 查询实际名称、最小／默认／最大值及 hidden 标志，按用户坐标建立实例 |

OpenType feature（如 liga、kern）继续沿现有 FontFeature 使用，不混入字体变体列表。weight／width／slant 的字面匹配值和同名字体轴也不互相代替；例如 OS/2 width class 不是 wdth 轴的坐标单位。

### 4.2 目录与字面 API

增加两个实际资源类型 FontCatalog 与 FontFace，均实现 Closeable，并在创建线程串行查询、使用和关闭；不引入字体插件、系统安装器或全局字体注册表。FontCollection 继续拥有排版时可用的字体集合。

```text
FontCollection.systemFonts(): FontCatalog
FontCatalog.fromData(bytes: Array<Byte>): FontCatalog
FontCatalog.load(path: scoop.platform.fs.Path): FontCatalog
FontCatalog.families(): Array<FontFamilyInfo>
FontCatalog.faces(family: FontFamilyId): Array<FontFaceInfo>
FontCatalog.openFace(id: FontFaceId): FontFace

FontFace.info: FontFaceInfo
FontFace.axes: Array<FontAxis>
FontFace.namedInstances: Array<FontNamedInstance>
FontFace.coordinates: Array<FontVariation>
FontFace.withVariations(values: Array<FontVariation>): FontFace

FontCollection.register(face: FontFace, family: String)
FontCollection.replaceFamily(family: String, faces: Array<FontFace>)
FontCollection.observeChanges(onChanged: (Long) -> Unit): Closeable
```

FontCatalog 是一次显式取得的目录快照，保存查询和打开字面所需的原生引用。系统目录只列 OS 字体管理器实际公开的家族／字面，不承诺枚举隐藏字体；关闭 systemFallback 不禁止显式查询系统目录，也不会因查询自动开启 fallback。用户刷新目录时创建新快照，不后台轮询或每次 render 扫描系统字体。

FontFamilyInfo 含类型化 FontFamilyId 和实际家族名称。FontFaceInfo 含独立的 FontFaceId、family、styleName、可用时的 PostScript name、weight／width／slant、是否为可变字体，以及文件来源已知时的 collectionIndex。显示名称不承担字面身份；ID 用于所属目录内查询，关闭目录后不能再次用于 openFace，也不作为持久偏好保存。目录实际关联对应的 style set／字面，不在点击时重新以名称进行近似匹配。

文件目录从自有字节构建，普通 TTF／OTF 表现为一个字面，TTC／OTC 列出多个字面。load 复用 Path.readAllBytes，系统文件选择仍由现有原生文件对话框负责；没有自动下载、安装到 OS 或修改进程外字体状态。慢 I/O 可由应用现有 asyncio／任务完成，再把字节交回字体 owner 线程构建目录和字面。

目录关闭不关闭已经打开的 FontFace；register／replaceFamily 成功后集合独立持有所需引用，调用者可关闭 FontFace。Paragraph／TextLayout 继续保留构造时的字体快照，后续替换或关闭来源不改变已发布结果。查询失败、坏字体、无效字面索引或已关闭对象沿现有错误规则报告，不以空目录或系统默认字体掩盖错误。

### 4.3 命名实例和轴

FontAxis 提供四字节 tag、可用时的本地化名称、minimum、defaultValue、maximum、hidden。FontNamedInstance 提供可用的实例名称／PostScript name，以及按轴对应的坐标数组。标签、名称和坐标分开存储，保留字体自定义轴。不可用的元数据明确报错；空 axes／namedInstances 只表示成功查询后的实际空集合。

withVariations 返回独立 FontFace，不修改来源。未指定的轴保留来源当前坐标；传入命名实例的完整坐标即选择该实例，再传覆盖坐标可以作局部调整。重复 tag、未知轴、非有限值、越界值作为参数错误，不静默钳制。查询回实际坐标用于显示与验收，不能仅回显用户输入。

轴滑块只有用户修改时才创建新实例，不每帧 clone。选择普通静态字面则使用目录中的实际字面，不通过假粗体／假斜体代替。UI 应用字体后同步该字面的 weight／width／slant，避免 SkParagraph 因沿用另一选择的样式而合成粗体或倾斜。

后端复用现有 SkFontMgr 的家族／style set 枚举、SkTypeface 的字体表和可变轴接口，以及 makeClone。命名实例、名称和字体集合计数复用已固定的 HarfBuzz：hb_face_count、hb_ot_var_get_axis_infos、named_instance 和 hb_ot_name_get_utf8 等接口。系统字体经 SkTypeface 字体表访问，文件字体直接使用自有字节；不另写 OpenType／fvar／name 表解析器。

SkParagraph 的 TextStyle 虽有 setTypeface，但当前主 shaping 路径仍通过 FontCollection 和 family／style 匹配；不能只调用该 setter 就宣称完成精确选择。本期将选中 FontFace 注册到应用指定的 family 别名中，用 replaceFamily 将预览别名替换为当前字面；配合该字面的样式值，锁定实际匹配来源。原有 TextStyle.variations 保留其低层用途，字体选择器使用已实例化字面时不再次设置一套重复坐标。

replaceFamily 接收非空字面数组，一次替换该别名下的全部显式字面；先建立完整新集合，再发布一次 revision。构造／登记在发布前失败时保持原集合和 revision；发布后的观察者异常遵循第 4.4 节。它直接解决预览选择反复向同一 alias 追加字体、旧字面仍参与匹配的问题。已有 register(bytes, family, collectionIndex) 继续服务简单装载，复用同一 native 字面创建／登记实现。

HarfBuzz 已链接进现有 Skia 依赖，只补发布该元数据调用实际需要的同版本头文件及绑定；不为这项功能新增另一套字体依赖版本。

### 4.4 UI 更新与字体选择示例

FontCollection 成功改变登记后更新 revision，并向 owner 线程中的变更订阅者通知一次。订阅关闭后不再通知，集合关闭结束订阅；观察失败不回滚已经成功发布的字体。该接口只发布字体集合变化，不依赖 UI State。

UI 的 FontWatch 改为订阅集合变化，把实际读者加入现有失效／更新队列，并合并请求一帧；退休最后一个读者时关闭订阅。现有逐帧字体 revision 轮询职责由这一通知取代，ParagraphCache 仍以输入和 revision 判断是否需要重新构造段落。字体目录查询与没有内容变化的选择器操作不唤醒其他窗口。

隐藏窗口仍记录布局失效，恢复后重排；输入框保留 TextEditorState、选区、IME 和焦点。对默认字体的替换沿 UiWindow.defaultFonts，显式字体的控件订阅自己的集合。字体修改发生在普通操作回调或装载完成处理中，遵守 UI 的 render／measure／scene／draw 纯度约定。

示例提供系统 family 搜索、字面列表、临时字体文件打开、TTC／OTC 字面列表、命名实例选择、实际轴滑块和即时文本预览。它使用现有 Select／Combobox／Slider／Input 等组件，不要求新增通用字体选择框架或 OS 字体选择对话框。持久偏好保存实际 family／PostScript name 和坐标；下次启动重新解析，缺失时由应用明确选择替代字体。

## 5. Taskbar／Dock：按原生能力表达

### 5.1 应用标识与文字含义

在 ApplicationOptions 增加可选 applicationId，供 Windows 显式 AppUserModelID 和 Linux 应用／desktop 关联在首个窗口前设置。部署值与 bundle.identifier 对齐；macOS 的实际 bundle identifier 来自 app，运行期字段不改写 Info.plist。相互冲突的配置报告错误，不从可执行文件名猜测身份。

普通 Window.setTitle 已覆盖窗口标题。应用显示名称和默认应用图标来自 bundle／PE 资源／desktop metadata；它们不等价于动态角标或 tooltip。特殊文字入口只在原生定义对应含义时提供：macOS miniwindowTitle，Windows thumbnail tooltip。首批不提供一个会在不同平台偷偷修改不同东西的 setCaption。

### 5.2 首批平台矩阵

| 功能 | macOS | Windows 桌面程序 | Linux GNU |
| --- | --- | --- | --- |
| 默认应用名称／图标 | app metadata | PE／安装快捷方式及 AppUserModelID 关联 | desktop metadata 与 app_id／WM_CLASS 关联 |
| 运行期应用图标 | NSApplication.applicationIconImage | 无统一的进程级 setter；按实际窗口设置图标 | 无统一的运行期应用图标 setter |
| 窗口图标 | 可设置最小化窗口的 miniwindowImage；不冒充进程图标 | WM_SETICON 的大小图标 | GDK／X11 可用路径；Wayland 首批以 desktop 关联为准，未确认支持的动态图标返回 Unsupported |
| 角标文字／数量 | NSDockTile.badgeLabel | 本期普通 Win32 路径不提供文字／数字角标；有原生 overlay image | 无统一 API，本期不实现桌面专有 launcher 角标协议 |
| 窗口标题／专用说明 | Window.title／miniwindowTitle | Window.title／SetThumbnailTooltip | Window.title；应用显示名称由 desktop metadata 提供 |
| 应用菜单 | applicationDockMenu 的 NSMenu | 原生 Jump List tasks；不等价于任意 popup Menu | 安装期 desktop actions，由桌面 Shell 呈现 |
| 进度 | 无独立标准 Dock progress API，本期 Unsupported | ITaskbarList3 原生进度状态／数值 | 无统一 API，本期不实现桌面专有 launcher progress 协议 |

此矩阵按当前部署形态定义，不声称所有 Windows 应用模型或所有 Linux Shell 都缺少相应能力。后续增加平台已有的能力需有具体使用者及独立验证，不能为了让矩阵一致引入自绘。尤其不使用 NSDockTile.contentView 自画进度、不把数字画成 Windows overlay 图像，也不通过修改标题文字模拟进度。

### 5.3 对象与接口

`ActiveApplication.openDesktop(): DesktopApplication` 创建当前应用的桌面集成资源，复用 NativeApplicationLease；同一应用只保留一个活动实例。`DesktopApplication.attachWindow(window): DesktopWindow` 绑定同一应用的现有窗口，一个窗口只允许一个活动绑定，重复附着报告 ResourceBusy；不创建隐藏业务窗口或改变窗口所有权。

DesktopApplication 提供应用级 capability、图标／角标、Dock 菜单附着和通知中心入口。DesktopWindow 提供窗口级 capability、原生图标、进度及平台专用说明。应用级状态不会自动复制到全部窗口；Windows 进度确实属于指定任务栏窗口，macOS Dock 角标确实属于应用。

capability 分别描述 applicationIcon、windowIcon、badgeText、overlayIcon、dockMenu、jumpListTasks、progress、miniwindowCaption、thumbnailTooltip。接口本身可调用时也不保证 Shell 当前正在显示对应按钮；尚未建立任务栏按钮时保留最新请求并返回 Requested。真实不支持返回 Unsupported，原生错误按既有异常合同报告。

progress 使用 `None`、`Indeterminate`、`Normal(completed, total)`、`Paused(completed, total)`、`Error(completed, total)`。数值采用原生可表示的无符号整数，total 大于零且 completed 不超过 total；无需把文件字节数转换为猜测的百分比。它只更新原生状态，不启动应用动画／计时器。

图片继续使用 DesktopImage，native 复制并持有所需 NSImage／HICON。替换后及时释放原有对象，不能让 WM_SETICON 持有已销毁的 HICON。DesktopWindow 关闭清除它拥有的进度／overlay 和图标覆盖，释放 native window lease；不关闭业务窗口。绑定观察目标窗口的关闭，及时退出原生使用，不能用 lease 无限阻止 HWND 销毁。DesktopApplication 关闭先停止通知／菜单回调，再清除自身角标、应用图标覆盖及附着；不恢复另一控制者保存的任意历史状态。

### 5.4 菜单的真实差异

macOS Dock 菜单复用现有 Menu、NativeMenuLease 和 MenuItemId。通过现有 NSApplication delegate 接口接入，不能替换或覆盖窗口库已有 delegate。开始／结束 tracking、更新延迟、关闭与动作资格遵守原有菜单规则；相同 Menu 不能同时附着到菜单栏、托盘或 Dock。

Windows 用独立的 `JumpListTask` 描述，包含标题、可选说明、启动本应用的参数及可选原生图标位置。ICustomDestinationList／IShellLink 消费这些描述；不把任意层级 Menu、勾选状态或 managed 闭包转换成 Jump List。任务是 Shell 启动动作，应用退出后仍可存在，参数沿正常程序入口处理。不会为了模仿 Dock 回调而引入单实例服务或后台代理。显式清除 tasks 才删除发布内容，关闭运行期控制对象不删除用户的 Jump List。

Linux 首批将应用启动动作作为 bundle.linux desktop actions 发布；不提供虚假的运行期菜单附着。任务栏／Dock 的系统菜单项、固定／取消固定及 Shell 自有选项不由库接管。

### 5.5 原生接入

Windows 在 TaskbarButtonCreated 后才调用 ITaskbarList3。Explorer 重建按钮后重发仍存活窗口的最新图标／进度／说明；无变化不反复调用，不轮询 Shell。未显示、隐藏或没有任务栏项的工具窗口与正常任务栏窗口分别处理。

macOS 用 NSApplication／NSDockTile／NSWindow 的对应属性和 delegate，保留现有 activation policy；没有 Dock 的应用模式不隐式切换为普通应用。Linux 继续使用 GTK／GDK 和现有 GLib 主上下文，不新增 X11 或 Wayland 窗口宿主。

## 6. 系统桌面通知

### 6.1 使用模型

NotificationCenter 由 DesktopApplication 取得，限 owner 线程操作。拟定接口为查询 settings、显式 requestPermission、post、withdraw 和 observe；异步原生操作的结果回到现有应用队列。

NotificationContent 首批包含标题、纯文本正文、默认／静音选择和可选具名动作；应用图标及显示名称来自 OS 应用注册。NotificationId 与 NotificationActionId 为不同类型的普通标识，用于替换、撤回和动作路由。平台对标识或动作的实际限制在请求入口报告，不任意截断。

post 接收显式 NotificationId。相同 id 再次提交请求替换原通知；OS 可能再次提醒，这不承诺为无声进度更新。回调区分 Submitted 和 Failed：Submitted 只表示系统服务接受请求，不表示弹窗可见、已送达用户或已读。系统免打扰、用户设置与通知中心策略继续由 OS 决定。

settings 区分可用、需要用户授权、被禁用、部署未配置和服务不可用；并列出 actions、replace、withdraw、激活／关闭事件及进程退出后激活的实际支持范围。检查系统能力不触发授权面板；只有应用显式调用 requestPermission 时才走 OS 原生授权流程。没有独立授权 API 的平台返回其实际设置，不自绘一套权限确认。

observe 交付正常的 Activated(id, action, payload)、Failed 及后端实际报告的关闭事件；没有关闭原因时保留未知，不合成计时到期事件。动作不直接保存 UI View 或旧 render 闭包。应用根据普通动作标识打开窗口或定位业务内容，退出后重新启动时也不试图恢复上次进程的闭包。

withdraw 请求撤回已知通知；返回值不宣称用户没有看过它。NotificationCenter.close 停止本地订阅、排队结果和原生使用，已交给 OS 的通知按系统生命周期保留；需要移除时调用 withdraw。应用退出不等待用户关闭通知，也不为通知继续保留渲染器或隐形业务窗口。

### 6.2 平台实现与部署

| 平台 | 实现 | 必要条件与边界 |
| --- | --- | --- |
| macOS | UserNotifications：UNUserNotificationCenter、request、category/action | 使用真实 app bundle 和稳定 bundle id；保留并桥接通知 delegate。支持系统原生授权和系统交付的启动响应，不把裸 CLI 冒充已注册 app |
| Windows | Windows.UI.Notifications WinRT ABI，必要的 COM 激活回调 | 当前 NSIS／MSI 安装形态使用显式 AppUserModelID、开始菜单快捷方式和通知激活注册；不引入 Windows App SDK 作为所有 GUI 的新前置 |
| Linux GNU | GIO／GDBus 调用 org.freedesktop.Notifications | 查询 GetCapabilities，使用 Notify／CloseNotification 和信号；与现有 GLib 主上下文集成。没有通知服务时返回不可用，不启动替代服务 |

Windows 的安装器负责写入正确的快捷方式 AppUserModelID、ToastActivatorCLSID 及所需 COM LocalServer 登记，卸载时按原安装规则移除自身条目。库负责在运行期登记类工厂、接收并复制原生响应。未安装或未完成显式开发注册时报告 NotConfigured，不在每次发送通知时暗中创建快捷方式或写注册表。用于激活的 CLSID 只是 OS COM 对象标识，不承担任何产物认证作用。

macOS／Windows 的进程退出后激活使用 OS 启动正常应用入口，再向本次 NotificationCenter 交付可序列化动作数据；正常启动初始化须接入该入口，不能要求旧对象仍存活。首批 Linux 的 freedesktop Notifications 路径只承诺运行进程收到动作信号，不增加进程外代理来补齐冷启动。各平台在能力查询中如实区分。

Linux 服务返回的 uint32 notification id 与应用 id 分开映射；服务重启后旧映射失效。服务恢复仅恢复可用性和订阅，不自动重发已提交通知，避免重复提醒。没有服务端历史查询时，不承诺新进程可撤回上一进程的任意通知。

标题、正文及动作文本按平台格式正确转义；Windows toast XML 由固定原生模板构造，Linux 以纯文本正文处理，不能把普通文本当标记语言执行。第一期不扩展 push、日程／位置触发、通知图片附件、输入框或业务任务调度。

### 6.3 回调、生命周期与 UI

UserNotifications completion、COM 回调或 D-Bus 响应可能不在 Scoop owner 线程。native 先复制普通数据，交给既有线程唤醒／事件队列，在 owner 上才进入 managed 回调；不能从外来线程直接调用要求 owner 的 NativeApplicationLease.post。关闭后迟到事件只完成实际 native 收尾，不调用已退休业务处理器。

需要扩充窗口 native 适配点时，只增加本轮实际使用的 delegate／消息／线程投递入口；不建立通用平台事件总线。菜单、通知和 Shell 状态更新均不请求 Surface 帧。

UI 包装提供 scope-owned 通知控制器和普通操作回调。发送通知是显式动作，不能放在 render 中；重建描述不重复发送。无窗口时使用 ApplicationScope；窗口卸载停止该 scope 的观察和回调，但不擅自撤回属于业务的系统通知。

## 7. Windows 圆形窗口问题

### 7.1 当前证据

1. [DocumentView](../../sysroot/src/scoop.ui.component/examples/native-desktop/src/document.scoop) 的 matchShape 默认 false；默认打开的是“Visual shape only”，不是原生 bounding region 模式。自动 smoke 会把它设为 true，因此不能用 smoke 成功推断默认人工路径正确。
2. [ShapeView](../../sysroot/src/scoop.ui.component/examples/native-desktop/src/dialogs.scoop) 已请求 Client、透明背景和椭圆 clip，但仅在 matched 时设置 windowShape／inputShape。窗口初始尺寸是 390×390，resizable 仍沿默认值。
3. [Win32 sw_style](../../sysroot/src/scoop.window/native/windows/window.c) 对 Client 使用 WS_POPUP，但 resizable 时仍添加 WS_THICKFRAME。它与 DWM 可见边框是否形成报告中的现象，需要真实窗口验证；不能仅看到该 bit 就把所有 Client 窗口的缩放能力删掉。
4. Surface 已经使用 CreateSwapChainForComposition、DirectComposition visual 与 DXGI_ALPHA_MODE_PREMULTIPLIED。没有证据支持以重写透明合成后端作为前置。
5. 原生区域通过内容到外框偏移和 DPI 换算，并在尺寸／窗框变化后失效。Surface 比较 frameRevision，只在 Accepted 帧发布新区域。继续沿这一既有协议修复。

### 7.2 定位与修复顺序

先在 native desktop demo 的默认人工路径重现，分别记录：仅视觉 clip／匹配原生区域、可缩放／不可缩放、有无应用自身装饰边框。读取实际 HWND style、frame/client 矩形、GetWindowRgn 结果和 DWM 属性，结合桌面截图区分主题绘制、系统边框与形状外残留像素。

demo 的“圆形窗口”默认示例改为：在 windowRegion 可用时显式请求原生形状，使用实际正方形内容尺寸，关闭示例没有提供的边缘缩放；另保留清晰命名的“仅视觉裁剪”对照模式。操作区域显示的是这两种具体功能，不让用户通过一个未勾选的选项误以为已经请求了原生圆形窗口。

如果真实 region 和不可缩放配置已解决报告问题，只修改 demo 和相应用例。若 Client 仍产生违反现有“隐藏系统可见边框”合同的边框，再局部修复 Win32 style／NCCALCSIZE／DWM 处理，并覆盖可缩放 Client 的独立回归。不能用仅 Windows 11 支持的 DWM 属性掩盖 Windows 10 基线问题，也不能把所有 Client 窗口改为不可缩放。

原生圆形的四角应同时不显示窗口内容并不拦截窗口外点击；视觉 clip 只能证明前者。需要跨进程输入验证时用真实桌面目标，不把 HTTRANSPARENT 当作通用穿透实现。透明反锯齿边缘与二进制 HRGN 的边界允许有其原生差异，不把普通阴影误判为可点击矩形区域。

改尺寸、跨 DPI、最小化／还原、窗框切换后，以实际新尺寸生成区域；Discarded 不发布候选，旧几何失效后等待新 Accepted。检查的目标是实际区域和绘制结果，不为这次 bug 新建几何状态机或重复全树验证。

### 7.3 完成条件

- 默认示例在 Windows 桌面显示真正的圆形窗口，没有矩形系统边框或角部底色；圆内控件、拖动和关闭正常。
- GetWindowRgn 有实际区域，中心在内、四角在外，区域对应本帧实际内容和 DPI；能与仅视觉模式区分。
- 分别验证初次显示、形状切换、最小化／还原、DPI 变化、关闭以及异常帧；可缩放 Client 作为独立用例保持正常。
- Windows 10 支持基线和 Windows 11 至少各有一次真实桌面验证；三种 Windows target 完成构建与必要的 ABI／运行验证。没有相应桌面环境时记录缺测，不能以交叉编译、SSH 启动或 macOS 离屏测试替代。

## 8. UI 展示与应用生命周期

native desktop demo 增加三个明确区域：SVG／Lucide 图标与主题切换、系统／文件字体选择及变体预览、当前 OS 的 Taskbar／Dock／通知操作。圆形窗口仍在原形状区域中展示修复结果。

只显示当前后端支持的操作；需要部署注册、系统授权或当前没有 Shell 服务的状态以实际查询说明。示例文字使用普通产品术语，不把 HRESULT、native lease 或内部记录编号放进操作流程。错误详情仍可通过已有报告入口查看。

`scoop.ui.component.desktop` 用既有 ApplicationElement 挂载 Dock／Shell 状态和菜单，窗口级条目绑定显式 Window。它复用 NativeMenuContent 的描述转换，但 Windows Jump List 保留启动任务的独立描述。图像可从 SVG 显式生成 DesktopImage 后交给 native；这只生成图标输入，不用于模拟系统缺失的角标或进度。

应用、字体目录、SVG 文档及 native controller 均由明确 owner 关闭。静止资源不请求帧；字体成功变化才唤醒使用者；通知、菜单和任务栏变化不制造 UI 绘制工作。无窗口应用继续由现有 LastWindowPolicy 和应用根决定存活，不增加后台守护进程。

## 9. 实施批次

| 批次 | 工作 | 依赖与交付 |
| --- | --- | --- |
| G1 | Skia SVG 构建、graphics.svg、解析／绘制／资源测试 | 可独立离屏使用，明确固定后端支持边界 |
| G2 | SvgView／Icon、选定 Lucide 资产与已有图标迁移 | 依赖 G1；完成主题、尺寸、线宽、生命周期和真实消费者 |
| G3 | FontCatalog／FontFace、系统和文件目录、命名实例／轴、replaceFamily | 独立于 G1；复用 Skia／HarfBuzz，完成实际字面选择与快照测试 |
| G4 | 字体变更唤醒与字体选择示例 | 依赖 G3；验证静止窗口、编辑状态及多窗口隔离 |
| G5 | Windows 圆形 demo 定位与必要的局部修复 | 可独立开展；先修实际路径，不等待桌面扩展完工 |
| G6 | applicationId、desktop Cone、Taskbar／Dock 原生功能与安装期任务 | 与既有窗口／bundle 合作；按平台能力分别交付 |
| G7 | 系统通知、部署注册、动作和启动响应 | 依赖 G6 的应用标识和生命周期；逐平台完成真实系统通知 |
| G8 | UI 桌面适配、综合 demo、发行资源与平台收尾 | 复用前述能力，不添加为展示而存在的第二套实现 |

每批次先同步所属规范再实施，格式化／lint 后执行对应测试。GUI 新库与 native 输入按现有 sysroot 准备和 lib-gui 发布方式集成；应用不自动准备依赖。平台通过范围分别记录，未验证的平台不写成已支持的实测结论。

## 10. 验收与文档同步

### 10.1 必要测试

| 范围 | 验证重点 |
| --- | --- |
| SVG | 基本形状与路径、非零 viewBox、固定／相对尺寸、嵌套变换、局部样式、整体透明度、错误输入、关闭及 Drawing 保活 |
| Lucide | 全部选定资源可解析；代表图标真实像素与外观；不同尺寸／DPI／颜色／线宽；同一资源连续绘制不串色；按钮／菜单／Spinner 组合 |
| 字体目录 | 系统实际家族／字面、同名样式不混选；TTF／OTF 与多字面 TTC／OTC；坏文件、索引和已关闭目录 |
| 字体变体 | 带许可证的固定可变字体，多个命名实例、自定义轴、默认／边界坐标、非法轴；查询实际坐标并验证度量或字形像素改变 |
| 字体生命周期 | 目录／FontFace／集合关闭后的已有 Paragraph／TextLayout；别名替换、失败不发布、静止 UI 唤醒、编辑状态与卸载 |
| Taskbar／Dock | 原生图标、文字、菜单、进度的实际可见结果；多窗口；隐藏／工具窗口；Explorer 重建；无变化停工；Unsupported 无模拟副作用 |
| 通知 | 查询和显式授权、拒绝／未配置／无服务、真实提交／替换／撤回、动作、迟到回调、服务重启、支持平台的正常启动激活 |
| Windows 形状 | 本文第 7 节的真实桌面与区域条件，尤其默认人工路径与 smoke 的差异 |

普通 API、错误路径和像素行为放所属 Cone 的 scoop.test；fixture 仅承担实际需要的产物、链接／ABI、命令行和安装／独立进程行为。测试使用 release 和共享预编译系统资源，不复制完整 sysroot，不通过隐藏源码或工具证明缓存复用。

字体目录测试不锁死某台机器的全部系统字体列表；从实际可枚举项目选择并验证原生字面，精确变体断言使用固定许可字体。Taskbar／Dock／系统通知和 Windows 形状要有真实桌面验收；原生 API 返回成功或屏幕外像素测试不足以证明系统呈现正确。

原始日志、截图集合和逐次报告置于仓库外。设计／进度文档仅保留范围、结果概要、失败闭合与缺测边界。

### 10.2 规范与记录

- SVG 批次新增 SVG 专项 spec，并同步 Surface 图形接入、UI component 媒体展示和 shadcn 图标约定；原先少量手写 Path 的阶段性决定随本设计更新。
- 字体批次更新 graphics.text 的目录、元数据、资源和集合变化合同，以及 UI 字体失效／缓存规则；语言和 runtime 规范无需新增字体机制。
- Shell 批次更新 window 的 ApplicationOptions／应用标识，新增 desktop 专项 spec；同步 bundle 的平台 metadata 和原生启动任务配置。
- 通知批次新增通知专项 spec，明确原生授权、请求完成、动作、关闭及部署注册；UI 应用挂载规范只补实际适配点。
- 形状批次仅在真实修复需要改变现行合同时修改 window／Surface／UI chrome spec；纯 demo 修复不改写底层合同。
- 实施进度以独立、精简的批次记录链接到本文；本文在实施前保持“设计”状态，不改旧 D1–D8 的完成记录来表示新增功能已经交付。

## 11. 调研依据

- [Skia 固定版本 SVG 元素与解析](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/modules/svg/src/SkSVGDOM.cpp)、[SVG DOM 接口](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/modules/svg/include/SkSVGDOM.h)。
- [SkFontMgr](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/include/core/SkFontMgr.h)、[SkTypeface](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/include/core/SkTypeface.h)、[实际 shaping 路径](https://github.com/google/skia/blob/f326a1e25fc736704063922b9ce76b527b1c5efe/modules/skparagraph/src/OneLineShaper.cpp)。字体元数据依据同版本依赖中的 HarfBuzz hb-face／hb-ot-var／hb-ot-name 公共接口。
- [Lucide](https://github.com/lucide-icons/lucide)；调研检查资源来自 `lucide-static 1.70.0`，首批清单见第 3.4 节，G2 固定资产与许可文件。
- [NSDockTile](https://developer.apple.com/documentation/appkit/nsdocktile)、[applicationDockMenu](https://developer.apple.com/documentation/appkit/nsapplicationdelegate/applicationdockmenu(_:))、[UserNotifications](https://developer.apple.com/documentation/usernotifications)；能力同时核对了本机 macOS SDK 的公开头文件。
- [ITaskbarList3](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-itaskbarlist3)、[ICustomDestinationList](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-icustomdestinationlist)、[桌面通知 AppUserModelID 注册](https://learn.microsoft.com/en-us/windows/win32/shell/enable-desktop-toast-with-appusermodelid)。
- [freedesktop Notifications 协议](https://specifications.freedesktop.org/notification-spec/latest/protocol.html)、[GDK 原生窗口图标](https://docs.gtk.org/gdk4/method.Toplevel.set_icon_list.html)。GDK 文档的最新版本不自动提升本仓库 GTK 基线，实际使用须核对当前准备的头文件和后端。

编写本设计时只完成源码／公开接口调研；后续实际构建、呈现和平台集成的验证结论见实施进度。
