# UI 交互打磨与 accessibility 实施记录

更新：2026-11-18。工作分支：`codex/ui-interaction-accessibility`。范围与完成条件以 [PLAN.md](PLAN.md) 为准。

## 执行约定

按本轮要求先在 macOS 完成本地功能与验证，再集中进行 Linux／Windows 验证；不逐个小改动远程重跑。每个可交付变更提交并推送后，获取并合并 `origin/main`。意外发现的编译器／底层依赖缺口使用独立修复分支、PR 与主线合并，更新工具链后回到本分支。

执行 [PLAN.md 的优先级待办](PLAN.md#当前待办与执行顺序)：一级完成全部功能及本地适用验证，二级完成三个平台的功能验收，三级才处理配色。高优先级未全部完成时，不得继续低优先级工作；限制或阻碍不自动解锁下一级。配色不作为功能缺口，只在一个平台调整和验证。

下载使用本轮指定代理，本地依赖复用 `~/repos/scoop-dep`。Linux 后续使用 `ssh nuc12`、`~/repos/` 下独立目录及 `~/.scoop`；Windows 后续使用 `/usr/bin/ssh 192.168.2.67`、E: 独立目录及 `~/.scoop`。原始日志留仓库外，必要缓存保留，临时核查文件在不再需要时删除。

## 当前状态

### 当前执行与封存项（2026-11-18）

- Accessibility 领域的公共模型、全部相关控件、三平台 provider 实现与 macOS 本轮集中验证已完成。触摸／手势与动画领域的剩余实现及本地集中验证也已完成；S0～S6 和 S11 的剩余基础交互、系统文本服务及共享示例也已完成，首轮运行验证 35 项全部通过。一级功能与本地适用验证已闭合，当前执行二级三平台集中验收。每轮汇总并修完全部已发现缺陷后再进入下一轮，已通过且输入未变的结果复用。本轮平台构建、定向运行与三平台独立客户端检查已通过，真实设备与读屏器的剩余验收继续保留在二级。
- 先前未提交的菜单／面板／Select 配色工作已经从工作树单独封存，保留于本地 stash `85242e2f60c3c76284d6e89377cd6356c3b4e78c`（`deferred-ui-colors-until-all-functions-and-three-platform-validation-20261117`）。三级解锁前不恢复、不继续测试或交付；此前已经提交的变更保留。
- 封存前已结束的 Style 与 Component 定向配色检查／测试结果保留，不据此把配色算作功能完成。后续每批只更新最高未完成级别的当前项，复用输入未变化的验收结果，不重新尝试系统前台焦点激活。

### 三平台集中验收与平台缺陷修复（进行中）

- 实际工具验收发现的两项缺陷已修复：Dialog／Sheet／Drawer 默认 X 按钮以 “Close” 命名；AppKit 按需保留已导出的虚拟行、单元格及列代理，直到实体退休或 provider 关闭，避免外部 AX 引用在查询后失效。反向引用不形成所有权环，仍不预建全部 View。
- 本批 macOS 六个受影响库、两个测试 Cone 和两个现有示例构建通过。三个目标场景各用独立进程串行运行并确认完整测试事件：按名称关闭与模态 veto；十万条列表及表格的代理寿命、重排、删除／恢复和释放；显式 Realize／Reveal、取消与退休。独立 AX 客户端复验两列表头列及 200 项列表，返回的行／列引用均可继续读取，不再出现 InvalidUIElement。
- macOS CUA 通过系统语义及画面坐标／键盘完成表单、原生编辑菜单和模态操作，画面路径完成虚拟列表重排与恢复。修复后实际工具能读取列表项目与真实按钮，按语义按钮重排并恢复顺序，按名称关闭弹窗，执行 Zoom／Rotate／Reset 并读回变换结果。远处虚拟项的完整工具操作及 VoiceOver 扩展组仍待验收，不以进程内回归替代；已通过的平台结果继续复用。
- 默认关闭名称已同步到 Linux GNU 与 Windows MSVC＋UCRT：两端的 Component／Shadcn 及两个示例构建均通过，未重建未受影响的底层库或重复整套交互测试。Windows 确认无验收程序运行后，将新 exe 发布到原 `manual-validation-round2` 目录，原人工步骤与路径继续有效；Linux 新产物保留在 `manual-build-close-name`，现有窗口可继续完成未受名称变化影响的拖放与 PRIMARY 验收。
- Linux PRIMARY 的 X11 跨进程检查通过：独立 Scoop 程序使用公开 Window API，与 `xclip` 双向传输中文／emoji 和换行，验证外部接管、旧 claim 关闭不清外部内容、重新认领与释放，普通 Clipboard 全程保持独立。检查使用临时 Xvfb，会话及全部持有者进程已结束。当前用户的 GNOME `wayland-0`／GTK 4.22.4 会话经只读查询报告 PRIMARY 可用；其真实中键收发仍待人工操作，不能套用此前私有 Weston 无协议环境的结果。Linux 剩余跨程序文本／URL／文件拖放步骤与样例文件已准备，按同一轮集中反馈。
- Windows MSVC＋UCRT 参考环境的实际英语拼写提供者检查通过：无可见窗口的独立程序经公开 Window API 调用 `en-US` 检查器，识别 `😀 spleling` 中 UTF-8 范围 5～13 的错词，并返回 `spelling` 等有效建议。macOS 既有 SystemTextServicesTest 已对同一错词、范围和建议作肯定断言，直接复用其通过结果；不是仅凭能力标志或允许空结果确认提供者。macOS 同批无窗口后台查询返回 0 个 Services 项，尚未覆盖实际服务执行，不能据此判断系统没有安装服务。词典面板、Services、听写与触觉等交互／设备结果仍分别记录。

- 本轮人工反馈的 Linux 缩放缺陷已闭合：此前 133% 下只有触屏命中偏移、运行中改变缩放停止接受输入；用户已在修复版确认窗口移到左上／右下时命中准确，运行中切换缩放后触屏与鼠标均正常。Windows 无障碍示例画面正常但窗口与控件不能操作的缺陷已修复并通过自动回归，待新 exe 的人工启动与 Narrator 确认。
- Windows 停顿程序的实际 owner 调用栈表明：语义引用重复绑定同一节点仍发布 State，触发 ActionInk 更新与持续重建；Win32 posted 唤醒同时饿死原生消息。已补同目标绑定的幂等处理和原生队列公平处理，输入与 TranslateMessage 的字符保留正常次序。Linux 修正原始接触事件的 GtkNative translation 方向，并在 widget scale-factor 通知时刷新完整 metrics。
- 本轮自动回归共 9 个独立进程通过：引用观察在 macOS、Windows MSVC、Linux Wayland 各一项，覆盖重复／重排绑定不重建、实际解绑／换绑／退休仍通知；GTK 生命周期与纯缩放通知在 Wayland、私有 X11 的 1×／2× 环境通过，X11 同时逐事件对照原始接触换算与 GTK 鼠标坐标；Windows 三个目标各通过持续 Poll 下真实定时器与 UTF-16 字符派发。首轮用例中的回调资源封装和异步坐标对齐问题已闭合。用户另确认 macOS VoiceOver A1～A7 整组正常，包括编辑／选择、勾选、数值、展开、表格、公告及模态隔离；Linux 真实分数缩放结果与自动检查分别记录。
- 范围按平台集中验证。Windows 在 MSVC 参考目标构建完整 UI／控件并运行交互与 UIA 客户端；两个 MinGW CRT 目标验证窗口原生边界、UIA COM 接口、文本快照和生命周期。已完成且未受影响的共享控件结果复用，避免在每个 ABI 上反复构建全部控件。GUI 用例串行运行，每个 Application 用例使用独立进程。
- macOS 的既有集中范围 292 项通过；新增配置前初始化及隐藏模态收尾两项回归通过，重命名后扩展的同一个初始化用例不重复累计。独立 AX 客户端的标签、Unicode／UTF-16 文本与选择、选择控件、范围、展开、表格／表头、模态与通知八项检查通过，收到文本值及选择变化通知。未请求系统前台激活。
- Linux 的 X11／Wayland 原生生命周期回归通过；独立 AT-SPI 客户端的八类检查均已通过，最后一轮只复查受影响的模态关闭、背景恢复及后续动作。共享隐藏窗口回归在两后端通过：无绘制帧也能结束有限退出、释放屏障、完成关闭通知，并正常重新打开和退休 owner。用户此前确认的 macOS 真实手势和拖放结果继续复用。
- 已修复配置前错误拒绝 accessibility provider／drop target、GTK 空 PRIMARY、AT-SPI 表头、X11 内容尺寸与窗口装饰的换算，以及 Wayland suspended 状态遗漏。模态在不可绘制时沿正常逻辑更新收尾，几何仍只由真实 Accepted 帧发布。窗口的失去前台焦点不作为该收尾条件。
- Windows 的 SDK 头文件／常量、HRESULT 类型及文本用途 GUID 定义已修复。两个 LLVM-MinGW CRT 目标各 33 项原生接口、文本、发布及生命周期检查通过，包括 UIA 属性／动作、滚动和保留文本范围。启动环境已有 Per-Monitor V1 时，仅升级创建线程到 V2，既有 HWND 保留原模式；真实 HWND 模式及 System Aware 拒绝回归通过。
- MSVC 的完整 UI／控件、主题与正式示例构建通过；本轮集中运行及缺陷回归去重后共 213 项通过。独立 UIA 客户端的树与标签、Unicode 文本编辑／选择、选择控件、数值、展开、表格／表头、模态和通知八项检查通过，收到 18 条变化通知。客户端按发布时序取得 provider 和匹配的真实文本布局，按既有 Busy 合同有限重试；未修改库去伪造即时布局或绕过模态拒绝。
- 修正 HWND 根的 UIA RuntimeId：根交由系统 host 提供标识，子节点保留追加身份，关闭后旧引用不可用；三个 Windows 目标的原生 COM 回归通过，MSVC 参考目标的外部 UIA 检查也通过。补齐 Windows UI 测试所需的共享文本布局源码后，此前受阻的测试程序及本轮选定用例均通过。最后一轮没有遗留失败，没有重新构建未受影响的生产控件库或重跑 macOS／Linux。
- Windows 双窗口长文本检查发现 TSF 查询可令 owner 线程停止响应。已修正 QueryInsert 返回越出当前文档的范围、GetText 分段文本／run／下一位置不一致、插入查询忽略 QUERYONLY／NOQUERY、锁结果和 TSF 自身编辑的反向通知。三个 Windows 目标的严格原生检查、Window 重建和各三项文本查询／快照／IME 回归均通过；新回归覆盖一单元及 128 单元分块、surrogate 跨块、真实 ITfRange 读取、只读查询不产生编辑以及一次插入只交付一个批次。双窗口的实际拖放继续使用新程序人工复验。
- 三平台的 accessibility 与双窗口交互示例已重新构建。无障碍示例增加独立 Announce status 操作，便于观察 polite 公告而不同时打开模态。人工操作统一见 [MANUAL-VALIDATION.md](MANUAL-VALIDATION.md)：Windows 交付 exe 由用户启动；Linux 使用 `wayland-0`，macOS 既有真实手势与拖放结果复用。
- 用户已确认 Linux Orca 的文本编辑、勾选、数值、展开／折叠、表头与单元格、状态公告、模态隔离及关闭整组正常；后续虚拟列表、变换按钮、受保护文本和触屏组反馈基本正常，发现的缩放问题经修复后定向复验正常。运行环境为 `wayland-0`／GTK Cairo；软件键盘和未反馈的设备组合仍单独记录。
- Windows GUI 验收运行在已登录的交互桌面，不请求前台激活；构建留在 SSH 会话。SSH 的 Session 0 服务桌面没有启用 DWM，曾导致显示保护 API 失败；同一程序在交互桌面通过，该环境错误不通过修改库来绕过。
- 环境范围：当前 Linux GTK／Weston 组合的默认 GSK Vulkan 后端在快速隐藏后重新显示时发生协议错误；同一程序在 GTK Cairo 后端通过，本轮后续 Linux 功能验收采用 Cairo，未修改库去强制该后端。Narrator、读屏器尚未覆盖的扩展场景、笔、软件键盘及其他实际桌面设备组合仍待验，不能据接口测试或合成输入宣称完成二级，也不解锁配色。
- 已结束本轮私有 X11 测试会话，清理过时传输包及一次性诊断／批处理脚本；原始结果与后续可复用的构建缓存保留在仓库外。

### 基础交互与系统文本服务整批实现与集中验证

- Windows TSF 的独立选区变化沿同一 UTF-8 文本事务提交，保留选区方向、composition 与修订；选择本身不经过正文过滤，也不生成撤销项。GTK PRIMARY 的拥有、替换和清理独立于 Clipboard；两类文本宿主发布可复制选区，中键先读旧 PRIMARY，再按 Accepted 落点执行原粘贴策略。
- 通过公开接口接入系统拼写／语法建议、可选自动替换、词典、Services 和长按触觉；控件及包装透传配置。检查结果只读，建议和服务返回绑定原文档／修订，自动替换与应用 policy 合并为一次编辑；受保护／遮蔽正文不自动导出。平台能力与完整原生文本检查客户端的边界见 [TEXT-SERVICES.md](TEXT-SERVICES.md)。
- 将已有触摸示例扩展为[双窗口交互展示](../../../sysroot/src/scoop.ui.component/examples/touch-input/README.md)：滚动／文本配置、单多行编辑、虚拟重排、文本／URL／文件接收、图像变换和模态共用公共 API；重排、粘贴、文件选择及变换均有按钮／菜单等价入口。源完成按实际 Copy／Move 处理，原地重排不再次删除，源版本变化保留原数据。
- macOS 集中验证通过：9 个生产库、4 个测试程序及双窗口示例 release 构建，三层 API 文档生成与 13 项接口查询，修改源码格式与原生严格编译。运行首轮 13 个独立测试容器共 35 项全部通过，覆盖 UTF-8 选区／composition、拼写范围、编辑策略／撤销、PRIMARY 生命周期的不可用分支、组件透传及重排完成。App 用例不请求前台激活；未改动的设备与拖放结果复用。
- 编译阶段的 FFI Unsafe 范围、可空标签、builder 接收者及测试状态捕获问题已集中修完；运行首轮没有遗留失败。Windows／Linux 的目标编译、PRIMARY 真实协议传输、系统服务实际提供者、触觉设备和读屏器保留在二级，不据本机模型用例宣称通过。

### 动画与定时行为整批实现与集中验证

- Hover、Typeahead、caret／文本边缘拖选、拖放边缘滚动、Select／滚动条重复、Carousel 与附件吸附接入现有窗口活动通知；隐藏、遮挡或实际失活立即撤销各自不再适用的预约。拖入目标不以未聚焦为拒收条件，窗口恢复不补发旧交互。
- 自动隐藏滚动条在不可呈现时撤销 idle 并直接收束，常显策略保持。Carousel 恢复及运行中修改 interval 从当前时刻重新完整等待，相同间隔的刷新保留截止点，迟到回调只前进一项。新增 CarouselClock 只服务这一实际消费者，仍使用原 WindowDeadline。
- 核对并复用已有 LoadingMotion、Spinner／Progress／Skeleton、InteractionMotion、Presence、Surface 动画完成与减少动画合同；Toast 保留剩余时间，DatePicker 保留一次逻辑提交。没有新增调度器、可见性输入框架或持续轮询。
- macOS 首轮集中验证通过：7 个受影响库构建、UI／Component 测试容器编译、12 个 Scoop 文件格式检查，以及 5 项定向测试。实窗用例未请求前台激活，覆盖隐藏取消、恢复不重放、手动关闭注销及最后停帧；纯时间用例覆盖轮播配置／迟到与滚动条 idle。没有失败需要进入下一轮。规范和 README 同步，无公共签名变化；其余平台和实际系统矩阵仍在二级。

### 触摸／手势整批实现与集中验证

- 三平台共用 Window／UI 拖放来源、目标和文本事务。Windows 接入 OLE 与真实兼容鼠标入口，GTK 接入 GDK 内容提供者及异步接收；UI 不重复接收系统提升的鼠标交互。直接 Touch／Pen 来源使用既有 ContactId、阈值和 claimContacts，系统接管收束普通输入；选区拖出保留端点把手、滚动竞争、Copy／Move 与单次撤销合同。
- 文本输入公开用途、软键盘请求与实际遮挡状态；受保护用途优先 Password，自动请求只按明确输入执行一次。Windows 使用公开 TSF／InputPane，GTK 只提供公开的显示能力，macOS 明确返回不支持。真实遮挡沿 Accepted 几何显露 caret，支持浮动键盘四侧可用空间，用户定位中断旧请求，隐藏后不恢复过时位置。
- 输入框外边距、候选项、Combobox、Sidebar、Chart 与附件滚动接入直接输入。Tooltip／HoverCard 默认允许长按，释放后保持、外部按下或失活关闭，普通短按继续执行原动作；图表通过短按选择数据，键盘与语义入口共用模型。自定义手势仍沿已有原始接触与归属边界扩展。
- 新增 [触摸输入示例](../../../sysroot/src/scoop.ui.component/examples/touch-input/README.md)，更新跨平台拖放示例、规范与编程材料。生产依赖构建、两个示例的 release 构建、三层 API 文档生成及 14 项接口查询通过。macOS 本批 14 个独立测试容器共 32 项通过，覆盖来源准备／延迟结果、系统接管取消、用途／遮挡几何、Accepted 视口显露及停止、平台复制键、组件选项、菜单实际动作与图表 Touch／Pen 选择。70 个 Scoop 文件与 31 个 native 文件格式、macOS 新增原生代码严格语法／警告检查通过。窗口测试没有请求系统前台激活，未重复已有设备验收。
- 编译轮次中已修复消费者类型／导入与旧测试接口；源码核对同时修复平台复制键、周期拖放回调中的重入关闭保活、不可用目标停止定时器及 GTK 空文本来源。真实触屏／笔、原生软键盘、Windows／Linux 目标构建和外部客户端仍在二级，不能由本机受控输入或源码实现替代。

### Accessibility 整批实现与本地验证

- 公共模型与三平台投影的实现已补齐：状态、选择／展开、数值范围、实时公告、表格与虚拟集合、文本范围与子对象、实际布局显露、模态边界和关系生命周期。Windows 使用 UI Automation，Linux 通过公开 GtkAtSpiSocket 与 AT-SPI 接入现有 GTK 应用树；不另建应用循环。此处记录实现范围，不代表 Windows／Linux 已通过平台验收。
- 控件接入覆盖基础编辑与选择、菜单／Tabs／Dialog、Calendar／DatePicker、OTP、表格／Chart、Command／Combobox、虚拟列表和会话控件。动作沿已有业务控制器和 Binding，拒绝的修改保留真实状态；装饰内容默认不增加无意义节点。Toast 和显式公告沿 busy 祖先合并延迟，隐藏、模态背景及保护文本不交付迟到公告。
- 本轮测试源码覆盖稳定代理与表格、保留文本范围的编辑映射、公告合并、受控控件动作、表头与合并单元格、模态背景旧引用。新增可运行示例见 [accessibility](../../../sysroot/src/scoop.ui.component/examples/accessibility/README.md)。新增窗口场景不请求系统激活；已经确认的真实设备与拖放结果继续复用。
- 统一格式化、本机 native 严格语法／警告检查、生产依赖及受影响测试 Cone 的编译、示例 release 构建与 API 文档生成／查询通过。macOS 本轮共 22 项通过：Window 集合／表格／发布与原生模型 8 项，UI 文本／内容／显露 7 项，组件基础／组合控件／模态／选择／滚动／虚拟集合 7 项。每个窗口测试类在独立进程运行，未请求前台激活；Windows／Linux 尚未进行本轮目标编译或平台运行。
- 首轮缺陷已闭合：文本模型更新所有登记范围文档，取消对活跃编辑宿主的错误依赖；模型先行、旧场景尚待逻辑更新时保留范围暂不可用，正常版本差不再导致发布失败；解除并恢复宿主绑定不会复活旧范围。新增无 drawable 的模型事务回归覆盖插入／undo／reset、保护、换绑和关闭，原生编辑动作场景检查暂不可用及下一次发布恢复。表头关系用例等待既有逻辑更新，测试按完整容器名隔离执行，避免类名子串误选另一 Application。

| 批次 | 状态 | 已完成与剩余 |
| --- | --- | --- |
| S0 | 实现完成，待二级验收 | 已核对本地工具链、AppKit 偏好与通知、NSTextView 默认菜单模板；AppKit 基础语义查询／动作原型通过，独立 AX 客户端的基础查询／动作／焦点／退休已通过；系统文本服务与附加能力边界已完成；完整外部客户端与读屏器、拖放及其他平台验证在二级 |
| S1 | 实现完成，待二级验收 | Window 偏好快照／观察 API 与三个 native 后端已实现，macOS 定向测试通过；UI 根覆盖／通知、caret 与主题消费已接入；四套主题离屏测试、实窗双根偏好热切换及帧复用回归通过；输入载荷与平台 input 模块已重构，macOS 输入／消费者回归通过；Windows／GTK 点击计数已修正，真实平台输入及其余细节待验 |
| S2 | 实现完成，待二级验收 | Window／AppKit 基础投影及 UI scope、逻辑发布、焦点绑定、关系、内容合并／忽略、Accepted 场景与精确命中已实现；基础控件、Switch／Toggle／RadioGroup／ToggleGroup、只读文本、Field 注释关系与滚动容器的原生回归通过，Accepted 场景、缓存重放与精确命中的实窗回归通过；基础跨进程 AX 回归已通过；全部相关控件、复杂关系和实际激活点已接入；完整外部／读屏器验收仍在二级 |
| S3 | 实现完成，待二级验收 | DeleteSelection、两类文本宿主的三种菜单模式、目标绑定、选区策略、键盘入口和独立 protectedText 用途已接入；AppKit 菜单与定向测试通过，两类宿主的真实文本菜单与 IME 组合实窗回归通过；文本 provider、原生选区及系统文本服务已完成；完整多击与其他平台矩阵在二级 |
| S4 | 实现完成，待二级验收 | 共享滚动条行为、配置入口、命中、轨道跳转及重复操作、统一 Gutter／Overlay 与边缘显露已接入，43 项定向测试及 ScrollViewport／ScrollArea／VirtualList／TextControls 实窗回归通过；Window／AppKit 范围与滚动动作、UI 宿主和组件条已接入，原生方法与请求逻辑回归通过；已挂载节点的 Reveal、逐层 Accepted 完成及取消实窗通过，虚拟集合显露已通过本地验证，外部辅助技术与其他平台仍待验收 |
| S5 | 实现完成，待二级验收 | 编辑／只读宿主的正文、选区、替代文字映射与 ShowContextMenu 已接入，Input／Textarea、SelectableStyledText／Typography 及包装自动发布宿主语义；macOS 原生方法与编辑策略定向测试通过；Field 标签／说明／错误和只读必填属性沿原连接发布；两类宿主的实际菜单呈现与编辑策略组合实窗回归通过；当前布局的文本范围／单位查询已接入，AppKit 实窗及独立 AX 文本范围／选区通知回归通过；保留范围、invalid／busy、选择／展开、公告和模态边界已补齐；真实读屏器及其他平台保留在二级 |
| S6 | 实现完成，待二级验收 | 三平台 source／target 与共享 UI／文本事务已实现，Touch／Pen 原生来源随 S8 接入；用户已确认 macOS 同框移动、区域复制、跨窗口及外部程序双向文本拖放正常。本地模型、准备及目标协议测试通过；虚拟重排与键盘／菜单等价操作已接入共享示例；完整系统矩阵和目标平台验证在二级 |
| S8 | 功能与本地验证完成 | 原生／直接手势、文本选择与端点、剩余控件长按／短按、原生数据拖放、用途／软键盘与实际遮挡、自定义手势扩展边界已接入；本批 32 项定向测试及两个示例构建通过。用户既有真实捏合／旋转与拖放结果复用；三平台真实触屏／笔、软键盘和完整设备矩阵在二级 |
| S7 | 功能与本地验证完成 | 稳定 key、元数据／行列查询、轻量代理、真实子树／几何、选择与活动项关系、Realize／Reveal／Focus、回收和删除、Command／Combobox／DataTable 等消费者已接入；完整读屏器、真实聚焦与其他平台验收保留在二级 |
| S9 | 功能与本地验证完成 | 弹性边界、直接接触惯性、嵌套交接、阅读锚点及偏好组合已交付；剩余定时行为的窗口中断、恢复与停帧已收尾，本轮 5 项定向测试通过。目标平台实际行为仍在二级；其余配色封存到三级 |
| S10 | 实现完成，平台验收中 | Linux Wayland／X11 的独立 AT-SPI 客户端已通过；Windows 两个 MinGW CRT 的原生接口及生命周期检查通过，MSVC 完整 UI 与外部 UIA 继续验证。真实读屏器仍在二级 |
| S11 | 实现完成，待二级验收 | 已有控件的语义、动作、键盘／触摸等价入口及共享双窗口场景已完成本地整批验证；三平台完整组合、外部辅助技术和实际工具使用在二级验收 |

## 已交付变更

### 原生手势与内容变换

- AppKit 的 Magnify／Rotate／Swipe 沿 Window input 和原队列交付；倍率增量、顺时针弧度、向右／向下的离散方向在平台边界明确转换。连续缩放与旋转分别维护活动状态，失焦、模态及原生接管沿已有取消入口终止，迟到更新不重启。原生手势不伪造触点或修改鼠标按钮。
- Surface 在 Began 时记录最近 Accepted 几何的目标路径；后续交付沿该路径，目标／边界失效、失焦和关闭显式取消。短暂打开又关闭边界也终止原序列，不因最终路径相同而继续默认行为。局部位置随当前 Accepted 几何转换，倍率与角度保持原单位；滚轮残量和 momentum 路由不参与手势消费。
- UI 的 `ComponentScope.transformGesture` 明确组合缩放与旋转，报告累计值、本次增量和完整结束／取消。选项变更、无效累计变换、阻止结束事件和处理器内关闭均清理会话；关闭只注销自己的登记。后续原始 contact 与自定义识别继续沿统一输入和归属边界接入，未提前加入识别器插件或占位 API。
- [可运行示例](../../../sysroot/src/scoop.ui/examples/gestures/README.md)使用同一业务操作处理卡片缩放、旋转、恢复和切换；按钮提供 Tab、Enter／Space 与语义 Invoke。已用真实应用检查画面、焦点边框、按钮／键盘结果和原生语义树更新，修正了角度显示截断。补充 `SemanticRole.Image` 与 AppKit 图像角色映射，图像名称和几何沿现有合同发布，不隐含点击或焦点动作。
- aarch64-apple-darwin release 共 21 项定向回归通过：Window 原生手势与既有 AppKit 输入各 1 项、Surface 手势路由 7 项及滚动回归 10 项、UI 组合手势实窗 1 项、含 Image 的 Window accessibility 原生方法 1 项。4 个 GUI 场景分别在独立进程执行。手势用例通过受控 NSEvent 属性进入生产 responder／队列，验证单位、相位、取消和组合。用户另在示例中确认真实触控板捏合与双指旋转正常，卡片和数值同步、松手后停止；原生 Swipe 的真实设备输入及读屏器验收仍待完成。
- Scoop 格式与检查、Darwin 生产输入和新增原生测试的严格语法／警告检查通过。Window、Surface、UI 规范、accessibility 设计、README、示例与编程材料同步，三库生成 API 的导入路径和签名已核对；Window、Window.asyncio、Surface、UI、Style、四套主题与 Component 共 10 个开发产物已更新。Windows／GTK 手势采集、直接触屏与完整设备矩阵仍按 S8 推进。

### 平台输入边界与载荷

- 依用户建议完成 `window-clicks` 与三平台输入路径、公共解码、NativeInputDelivery、Surface claim 的源码核对，并查阅 Apple／Microsoft／GTK 官方输入资料。选用 Window 内部的平台输入子系统，统一载荷与寿命，保留平台原生入口；Surface 负责接触路由，UI 负责原始触摸手势识别。
- 调研确认 `window-clicks` 当前是计数测试 Cone；另发现 GTK begin／scroll／end 的 0／1／2 与公共 AppKit 位值解码不一致。三平台现在共用事件头与有名的键盘／鼠标载荷，公共解码不再解释 AppKit 位值；GTK 连续滚动显式转换为 Began／Changed／Ended，取消终止活动阶段，普通 wheel 保持 None。行单位不缩放，表面像素按事件 scale 转为物理像素。
- 三平台现在分别由 `input` 模块维护 keyboard、pointer、lifecycle 与原生 delivery。Windows／GTK 的窗口状态集中到 SWInputState，GTK 的布局和设备修饰状态仍属于 Application；AppKit 沿 SWView 初始化、激活／失活和重置输入。GTK controller 在释放窗口输入状态前断连；按下的 GdkEvent 直接传给入队转换并保留，不再临时存入窗口字段。
- NativeInputDelivery 的 Scoop 实现及三平台原生附件复制／释放迁到 input 内部，窗口操作／拖放时序保持。后续自定义手势的接入边界已加入设计和 S8：保留原始 contact 路由，内建与未来识别逻辑共享归属／取消规则，最终操作沿命令与 accessibility 入口执行。原 accessibility 设计同步约束等价操作和结果通知；本轮未增加自定义识别器 API。
- aarch64-apple-darwin release 共 25 项定向测试通过：6 项生产点击计数、4 项输入值／native 载荷、4 项 AppKit 输入／IME／文本命令／拖放来源／文档事务、6 项窗口操作与模态、5 项 UI 编辑／滚动／拖出／文本接收／Press 与轴向接管。每个 Application 场景在独立进程执行。新增载荷用例覆盖七种独立相位及分数 DPI 的像素／行单位，AppKit 原生入口另覆盖 5 种 contact 阶段及 3 种 momentum 阶段。Scoop 检查、32 个 Objective-C 文件与必要 C helper 的严格语法和警告检查通过。
- Window、Window.asyncio、Surface、UI、Style、四套主题与 Component 共 10 个开发产物已按依赖顺序更新。Windows／GTK 真实输入结果未新增，具体范围见[输入边界方案](INPUT-BOUNDARY.md)。

### 文本范围查询所需的底层接入

- 原生文本与几何查询已独立通过 [PR #144](https://github.com/scoop-lang/scoop/pull/144) 合入 main；本地 main、工作分支和开发 graphics／text 产物已更新。TextLayout.withNativeLayout 保留同一排版的 native 查询，普通资源关闭后仍可使用；绘制与并发查询共用排版缓存的串行访问边界，没有额外 shaping 或逐字符矩形副本。
- 几何借用统一为 withNativeGeometry／NativeGeometry，包含原 contains 与新增 toLocal／完整或精确可见 mapRect。现有 UI 场景和 Window 测试调用同步迁移；Window 的独立 NativeSemanticHitTest 合同保持。
- 底层 aarch64-apple-darwin release 的几何 3 项、文本库 66 项，以及格式、C++20 严格编译、Scoop 检查、生成 API／import 核对均通过。这一底层交付不包含上层场景；后续 S5 接入和实际验证见下节。

### 原生文本范围与已呈现场景

- Window 增加 NativeSemanticTextGeometry／NativeSemanticTextRange 和 setTextGeometry，节点、候选及已发布投影各自持有原生引用。AppKit 的公开字符／行／按点范围、范围屏幕矩形、caret 行号与可见范围使用同一正文，正确转换 UTF-8／UTF-16；原生 getter 不进入 Scoop，不取得焦点或重新排版。
- UI 的 updateTextEditor／updateSelectableText 绑定真实宿主；文字视口与节点位置一起经 Accepted 发布，过期、缺失登记、解除绑定、正文更新和关闭撤下旧几何。普通 update 解除专用绑定，专用入口恢复；滚动只替换场景引用。投影区间、grapheme、OTP cell／caret 和 alternateText 均复用原宿主数据。
- aarch64-apple-darwin release 的 5 项原生文本查询测试通过：投影与词／字符单位、原对象关闭后的引用、视觉／逻辑行和末尾空行、精确 clip 孔洞、RTL／空分格输入、行内替代文字。裁剪已存在的 glyph box 不再回退到旁边 caret，以免把完全隐藏的文字报告为可见。
- 3 项实窗用例分别在独立进程通过：编辑器 Unicode 读取／写入／策略、只读替代文字与 caret、滚动／缩放／实际 Discarded／新正文先于布局／解除和恢复绑定／缺失登记／宿主关闭。原有只读、保护、撤销和 composition 验证保留。范围矩形与真实排版及 Accepted 输入几何逐项对照，查询不改变键盘焦点。软换行的上游 cursor 曾被报告为下一行，已复现并加入定向回归；两个宿主通过 SemanticTextAffinity 保留其真实偏向，单独偏向变化不撤下排版。
- Window／UI 规范、原 accessibility 设计、计划、README 与编程导航同步；新增 Window 原生 API 和两个 UI 宿主入口已由生成文档查询核对。文本范围的跨进程 AX 查询见下节；VoiceOver、逻辑 retained range 与其他平台继续按 S5／S10 推进。
- 受影响的 Window、Window.asyncio、Surface、UI、Style、四套主题与 Component 开发产物已更新；内置控件既有语义用例以新产物通过，包含正文／选区与 Field 关系。Scoop 格式和检查、Objective-C／C++20 严格编译、生成 API 及本地文档链接检查通过。临时 API 站点、应用包和逐次报告已清理，保留共享产物、启动脚本与后续使用的 AX 客户端。

### 独立 AX 文本范围与选区通知

- 编辑器和只读富文本使用独立 AXUIElement 客户端，通过正文、UTF-16 字符／行／可见范围、StringForRange／AttributedStringForRange、范围屏幕矩形与按点定位。矩形与实际排版及 Accepted 输入几何对照，覆盖行内 alternateText 的部分范围映射到完整对象；查询和选择保持键盘焦点。
- 两类宿主均复现同一缺陷：反向选区改为覆盖范围相同的正向选区后，真实 cursor 已改变，但 AppKit 只比较排序后的 NSRange，未发送选择通知。发布比较已补充原 anchor／cursor，保留现有偏向和正文处理。
- aarch64-apple-darwin release 的两个外部模式完整通过。AXObserver 分别收到一次 SelectedTextChanged、零次 ValueChanged；再次设置同一选区没有新增通知。编辑器同时检查活动 cursor 从首行移到末行，只读宿主检查实际选择回调仅增加一次；原有编辑、策略、撤销、composition、保护和关闭用例继续通过。
- Window native 与客户端的严格编译、Scoop 格式／语义检查及定向 release 构建通过。规范、设计和测试入口已同步；此修复不改变公共签名或 ABI，只重建 Window 与实际测试消费者。临时应用包及原始报告已清理，保留两个 AX 客户端、共享产物和启动脚本供后续使用。VoiceOver、逻辑 retained range、虚拟集合和其他平台仍待继续。

### 系统交互偏好的 Window 接口

- `InteractionPreferences` 区分未知值与明确关闭，覆盖滚动条、运动、对比度、透明度、文字缩放、caret 和双击间隔；保留物理宽度、宽高比和时间单位的差异。
- ActiveApplication／Window 读取 owner 已发布的快照，观察者经普通队列通知；先发布后通知，去重，回调内登记／注销、窗口关闭与应用退出按实际生命周期处理。
- AppKit 使用公开查询与通知；GTK 使用 4.18 基线设置和 libadwaita；Windows 使用 Win32 设置与按接口查询的 UISettings 2／4／5。Windows／Linux 的代码尚待目标环境构建与运行，不据 macOS 结果声明通过。
- 同步 Window spec、公开注释、库 README 与编程材料导航。定向 Cone 的原生通知用当前进程内替身驱动，不修改全局系统设置，也不冒充人工设置热切换验证。

本地验证使用 `/Volumes/Data/home/chenxu/repos/scoop-install` 中配套的 `scoop` 与 sysroot（工具 revision `6bd92d3d7a14e4614320008b33586df8da178d85`），测试显式通过 path 依赖编译当前 Window 源码。仓库原有开发 sysroot 与该工具的 runtime ABI 不同，没有混用其预编译库；当前主线相对该工具未发生语言／runtime 合同修改。改动源码先通过 Scoop fmt／check 与 native 格式检查。

`window-preferences` 的两个测试已通过：不可用值／显式关闭的区别，以及 AppKit 通知、快照发布、去重、观察者增删、owner 检查和关闭。Window 生产／测试语义检查和既有 `window-deadlines` 生命周期回归也通过。真实系统设置的人工热切换、设备输入及其他平台仍按 S0／S11 的矩阵验收。

### UI 根继承与文本光标

- `UiPreferences`／receiver builder、UiHost／单窗入口、UiWindow 动态覆盖和 ComponentScope 快照／观察已接入；None 跟随系统，显式 false／零保留原意。不同根独立解析，已覆盖的系统变化不请求帧；通知支持重入、取消和根错误收尾。
- render／update 的显示依赖及 measure／scene／draw 的版本记录沿已有缓存失效路径处理；偏好读取不增加持续帧。组件 update 仍只登记明确的默认字体和交互偏好，不扩展为任意 State 跟踪。
- TextEditHost 默认继承闪烁和宽度，Input／Textarea／OTP 及其组合入口、示例与测试同步改为可选覆盖。系统物理宽度按窗口 scale 换算，宽高比使用排版光标高度；局部逻辑宽度优先。每份测量与 Accepted 帧保存采用的光标样式，旧帧查询不会读到下一帧的宽度。
- 根关闭撤销应用级偏好观察；文本宿主还观察实际 DPI 变化。偏好更新保留编辑模型、选择和历史，闪烁继续使用既有 WindowDeadline。

开发资源使用配套安装与仓库外 `scoop-ui-polish-work/cones` 的普通 `--cone-path`：定向构建 Window、Surface、UI、Style 与四套主题，复用其余安装产物和共享 cache；没有复制整套 sysroot 或修改安装目录。主题在此批次仅按 UI 接口依赖重建，减少动画／高对比／减少透明度／文字缩放的主题消费仍待下一批实现。

本批 macOS 验证：UI 生产／测试和 Component 测试代码的语义检查通过；46 个变更的 Scoop 文件格式检查通过。UiPreferences 两个值测试通过；独立 UI 偏好窗口测试通过，覆盖四种显示读取路径、实际 AppKit 通知、两根覆盖隔离、重入／注销、物理宽度与比例宽度、Accepted 查询、模型／选择／历史保留、闪烁启停／停帧和回调失败退休。既有 `ui-text-editing`（含真实系统 IME、投影、选择及滚动）与 `ui-layout-results`（布局 A→B→A、保留测量和 lazy 选择）定向回归通过。没有运行无关的全仓测试或远程矩阵；本批仍未声明完整 accessibility provider 或主题适配完成。

已定向生成 UI／Component API 文档，并用 `scoop api search/show` 核对 UiPreferences、UiHost.attach、UiWindow.preferences 的访问器，以及 caretBlink／caretWidth 的可选类型、默认值与导入路径。生成站点在核对后清理，保留后续构建需要的 Cone 产物与 cache。

### 主题消费根交互偏好

- 新增 ThemePreferences、Theme.withPreferences、ThemeSource.resolve／read(scope)。来源的基础 State 与根解析结果分开，重复解析不叠加字号；内置控件与自绘示例统一使用所属 scope，第三方主题实现与测试包装同步新入口。
- 四套 ShapeTheme 预设同时更新 tokens 和实际表面、复选框、单选框、滚动条的部件数据。高对比度调整文字／标记与状态边框，保留独立焦点外环，取消整体禁用透明度；减少透明度合成默认填充并移除默认装饰。文字缩放保留字体族／字重／相对行高；减少动画使用既有 ThemeMotion，局部过渡不能覆盖根设置。
- Style 生产／测试语义检查、Component 全部测试源码及新增实窗测试源码检查通过。两个修改后的主题／下载示例生产检查通过，Window.asyncio 按已变更的 Window 接口定向重建。四套主题、明暗模式的三个新增测试通过，覆盖解析复用、不同偏好／基础主题切换、非法缩放、稳定状态对比度、部件实际几何，以及减少透明度的离屏像素。焦点边框在组合状态中低于 3:1 的问题已修正并通过该回归。
- 新增共享来源的双窗口测试，检查真实 Label／Input 度量、编辑模型／历史／焦点保留、动画启停与停帧、局部来源覆盖及恢复。原 ThemeAccess 的纯值来源测试移入这个实窗用例，未新增测试专用的 scope 构造入口；既有帧复用测试显式固定其需要的动画与字号配置，避免机器偏好改变测试前提。
- 实窗测试及原有 ComponentFrameReuseTest 均在首帧前超时。已核对 macOS 控制台会话：ScreenIsLocked=true；此时系统暂停窗口呈现。该项仍待桌面解锁后运行，不能据离屏或语义检查声明实窗通过。已向用户请求在方便时解锁，期间继续不依赖显示的工作；没有改动系统安全设置。
- 已生成并查询 Style API 文档，核对 ThemePreferences、withPreferences、ThemeSource.resolve／read 的签名与导入路径；库 README、Component spec 和编程材料同步。临时站点已清理，保留更新后的 Style、四套主题及 Component 产物和共享缓存。

本批仅完成主题偏好的公共接入与离屏验证，全部控件派生样式、真实设备／读屏器、三平台 provider 和完整 S11 矩阵仍按计划推进。

### 原生连续点击计数

- Windows 在原生 down／double-click 消息上延续三击及更多次计数，使用消息时间、屏幕物理位置及当前系统时间／距离阈值；拖动、失焦或取消结束计数，正常最后一次 Up 后的 ReleaseCapture 保留历史。每个已按下按钮保留自己的次数，释放不受其他按钮或拖动重置影响。
- Linux 直接消费 GtkGestureClick 的 count，移除重复的固定阈值；核对 GTK 4.18 实现中设置变化、设备／按钮切换、拖动及 released 的配对行为。显式输入取消重置 GTK 控制器，重入取消与窗口销毁释放各有正常出口。AppKit 保留其原生计数。
- 新增独立 `window-clicks` Cone，直接编译 Windows 实际使用的计数函数；6 个测试在 macOS release 通过，覆盖连续／首次原生双击、阈值边界及热变更、消息时间回绕、窗口屏幕位置变化、拖出返回、取消和计数饱和。C11 与包含真实调用的 C++20 编译检查通过；未建立 Win32 API 模拟层。
- 新增 Win32 SendInput 三次点击用例，并扩展 GTK X11 原生输入测试；它们只发送按下／释放，由平台产生 count。既有 Win32 协议测试检查取消后的 DBLCLK 从 1 开始、Up 配对；这些用例尚待集中 Windows／Linux 验证，未将本地算法结果当作平台通过。Wayland 和实际人工设备输入保持独立待验。
- 同步 Window spec、公开注释、库 README、设计现状与编程材料。没有新增计数 timer 或改变公共 InputTime 的 Observed 合同；临时下载的 GTK 参考源码在核对后清理，保留后续需要的构建缓存。

### Window 语义资源与 AppKit 基础投影

- 新增 `scoop.window.accessibility` 包；低层 provider／node 为 owner 资源，节点身份在窗口生命期不复用。属性、动作、父子关系与阅读顺序通过明确发布更新，名称／值变化清除旧几何；未知、空文本、受保护文本、隐藏和离屏分别处理。没有新增 Cone、IR、runtime 或另一套事件循环。
- AppKit 使用 NSAccessibilityElement 接入现有内容 view。getter 读取已发布原生状态，实际动作经捕获 Context 的 ForeignCallback 执行；业务／watch／另一动作中重入返回 Busy，BOOL 仅对已完成动作返回成功，无返回值的 setter 用实际值验证结果。异常沿既有应用退出路径处理。
- 节点关闭立即拒绝旧对象动作，包括尚未生成原生对象的节点；provider／Window 关闭撤销注册并在回调退栈后释放 callback。父节点退休覆盖后代，重新附着不复用身份。几何使用窗口逻辑坐标，未知布局不公布 frame，零面积不计为 visible child；未实现的精确命中不以矩形中心冒充。
- 10 个 Scoop 文件格式检查、Window 与测试 Cone 语义检查、生产／测试 Objective-C 严格编译检查通过；独立 AX 客户端的 C11 严格编译通过。macOS release 的 `window-accessibility` 单一应用测试通过，覆盖十二个连续原生动作阶段，以及属性发布前后、关系／几何校验、Unicode 写入与拒绝、焦点、隐藏／禁用、重入、节点／provider／Window 退休和异常收尾。测试通过公开 NSAccessibility 方法检查，不读取生产私有状态。
- 外部 AX 客户端连接权限检查通过，但尚未完成跨进程动作：AXWindows 返回一项，该引用的远程 role 为 AXApplication；同一个窗口在进程内为 AXWindow。独立的标准 AppKit NSWindow／NSButton 对照在普通窗口级别也复现此现象，且进程内 isAccessibilityElement=true。本机仍锁屏，当前无法确定异常的具体系统原因；待桌面恢复后复验，不把进程内通过记为外部 AX 或 VoiceOver 通过，也不改动系统安全设置。
- 已生成 Window API 文档并用 `scoop api search/show` 核对 Darwin 工厂、导入路径、SemanticProperties／Actions、Boolean focus、可选几何访问器和不可公开构造的节点身份。同步正式 spec、README、设计状态及编程库导航。临时文档站点、标准 AppKit 对照源码和二进制在核对后清理；保留外部 AX 客户端和后续构建缓存。

本批是 S0／S2 的低层基础，工厂当前仅在 Darwin 目标公开；UI scope 与 Accepted 场景、默认控件、完整文本、虚拟集合、其他平台 provider 和真实屏幕阅读器仍按统一计划推进。

### 仅删除选区的菜单命令

- 新增 MenuRole.DeleteSelection、TextEditCommand.DeleteSelection 与 TextCommandAvailability.deleteSelection。没有选区时不删除相邻字符、不结束组合、不调用编辑策略或增加撤销；可用性要求显式声明，不从 IME 开关猜测。AppKit 原生字段使用 delete:，Windows 原生 Edit／RichEdit 使用 WM_CLEAR，GTK Scoop 内容沿既有命令派发；按实际平台继续验证。
- TextEditHost 的删除与 Cut 复用同一选区编辑准备路径，保留策略过滤、异常前不发布、独立撤销和方向／affinity；删除不写剪贴板。只读／禁用和焦点检查沿原宿主执行，SelectableTextHost 不提供删除。Surface 传递新增可用性和真实命令，保留 preventDefault、焦点／边界资格与静止停帧。
- macOS 的三个编辑事务测试通过：空选区与活动组合／redo 保留、Unicode 反向选择与独立 undo／redo、策略拒绝／异常。两个 Surface 路由测试通过，覆盖可用性继承、失焦、禁用、边界、默认动作阻止及没有合成键盘事件。生产／测试 Scoop 格式与语义检查、AppKit Objective-C 严格编译检查通过；Window、Surface、UI 开发产物已按依赖定向更新。
- 已扩展真实编辑器用例，检查菜单能力、删除／撤销、空选区、只读／禁用与剪贴板保留；原生菜单焦点用例同时检查 Scoop DeleteSelection 事件与实际 NSTextField 删除及空选区禁用。后者原先用禁用菜单项模拟后台状态，现改为条目正常启用、Scoop 发布全 false，准确验证系统字段沿自己的状态判断。这两组源码检查通过，前台运行仍待桌面解锁，没有据离屏结果声明菜单交互通过。
- S0.3 的独立 AppKit 原型确认：公开 NSTextView.defaultMenu 无须创建隐藏编辑器即可返回系统模板，但其中富文本、拼写、替换及排版动作仍依赖真实编辑服务。文本菜单后续按下述可执行能力接入，不能直接展示全部模板来假报支持。
- 已生成并查询 Window／UI API 数据，核对两个 DeleteSelection 枚举入口、七项可用性的默认值、TextEditHost.execute 与导入路径；同步相关 spec、README、设计现状及编程材料。临时菜单模板程序和 API 站点已清理，保留开发 Cone 与缓存供后续工作使用。

### 文本宿主的原生上下文菜单

- Window 提供不持有编辑目标的 TextContextMenuTemplate、七项标签覆盖和平台辅助点击查询。AppKit 从公开 NSTextView.defaultMenu 保留可执行的标准编辑项目并补齐缺项；Windows／GTK 使用标准菜单回退。SystemTemplate 明确表示系统模板的可执行子集，未接通的拼写、替换、翻译等服务没有伪装成可用菜单项。未取得的系统标签使用英文默认值，应用可逐项覆盖。
- TextEditHost 已接入 Disabled／StandardNative／PreferSystem、Platform／Preserve／PlaceCaret／SelectWord 和键盘／程序入口；指针选择及菜单锚点使用 Accepted 文本布局，点击选区内保留方向。打开菜单停止当前拖选和越界自动滚动。菜单首次请求才分配原生资源，动作绑定发起宿主、模型与会话，按当前资格与策略执行；失焦、换模型、禁用、配置变化及关闭使旧动作失效，完成通知不重复编辑，没有后台轮询。
- Input／Textarea／InputOtp 与 builder、InputGroup、ButtonGroup、Command、Combobox、SidebarInput 已透传配置；OTP 仍只有一个编辑宿主和长度／模式策略。检查过程中补齐了 ButtonGroup 重建 InputProps 时的透传，增加实际组合入口回归。
- AppKit 原生菜单 Cone 的两项非忽略测试通过，包含真实默认模板标题、七种命令、标签覆盖、辅助点击、菜单会话和资源收尾；原生焦点测试仍按既有前台要求单独待验。UI 的两个新增定向测试通过，覆盖配置快照、菜单键、重复／合成／释放事件及额外修饰键。组件生产与全部测试源码检查通过，receiver DSL 的六项测试和 ButtonGroup 的三项测试全部通过。组合链接中的过期 UI 依赖已通过按顺序更新 Style、四套主题和 Component 开发产物闭合。
- 既有真实编辑器用例增加二十个菜单阶段，覆盖右键与 Control+左键、选区保持／定位、菜单删除、Shift+F10、粘贴时读取新剪贴板与长度过滤、撤销、只读、关闭菜单、preventDefault、换焦点／模型、禁用和关窗后的迟到动作。源码检查与 release 可执行文件构建通过；本机仍锁屏，没有据这些构建结果声明前台文本菜单交互通过。Linux／Windows、受保护用途、多击组合、系统文本服务、语义菜单动作和触摸长按继续保留在完整计划中。
- 已同步 Window、UI、Component spec、README 和编程材料，生成并查询三个 Cone 的 API，核对默认值、标签类型、showContextMenu 和 builder 重载及有效 import。临时 API 站点已清理，开发 Cone 和待解锁运行的测试程序保留。


### 滚动条活动反馈与操作

- ScrollArea、VirtualList、Textarea 和 MessageScroller 提供不可变 ScrollbarBehavior 与 receiver copy；InputGroup 透传多行配置。按范围判断的 Auto／Always／Hidden 与闲置隐藏分别解析，FollowSystem 消费根的有效偏好，未知值回退常显；显式 autoHide、展宽、深浅、触发条件和各过渡时长可独立覆盖。
- 共享控制器用一份显隐动画、一个 native deadline 和一个非消费式指针观察管理两轴；原有按比例的 thumb 拖动保留。按住、hover、滚动路由／momentum、相关键盘和程序动画暂缓隐藏；LayoutCorrection 不续期。隐藏后撤下输入区域，重新出现等待 Accepted。命中条带独立于绘制宽度，主题保留主轴几何，交叉轴缩放完整绘制，消费本帧透明度和 showTrack；Material 的大内缩不会挤空细条。
- 增加 Page／Jump 轨道行为、轨道与箭头的可关闭按住重复；按下接管与最终释放沿既有输入路由，preventDefault 的释放仍清理。主题／范围／状态替换及禁用继续使用 Accepted 几何与原取消通路。减少动画立即完成过渡；减少透明度取消半透明及淡入淡出，保留自动隐藏。
- 主题库的语义检查、生产构建及四套主题依赖重建通过；组件生产／测试代码检查及生产构建通过。20 项定向测试全部通过：8 项新行为／时序／四主题细条绘图测试、4 项滚动布局、2 项滚动装饰和 6 项 receiver DSL。原生场景复用既有窗口，新增 17 个阶段覆盖隐藏点击穿透、静止停帧、边缘唤醒、按住翻页、被拦截的释放、跳转拖动和常显偏好热切换；源码检查与 release 可执行文件构建通过。本机仍锁屏，尚未运行这些前台交互阶段，不能据构建结果声明实窗验收通过。
- 本批保留现有 ScrollArea／VirtualList 的 Overlay 与 Textarea 固定占位；统一布局选项、Overlay 对 caret／焦点显露的遮挡处理和经 S2 接入的滚动语义仍属于 S4 未完成部分。真实设备／平台验证按总计划进行，不以值测试或构建通过代替。

已生成并查询 Style／Component API，核对 ScrollbarBehavior 的完整默认值、构造方式、Textarea／ScrollArea builder 重载、VirtualList／MessageScroller Props 和 showTrack；临时 API 站点已清理。

### 滚动条占位与边缘显露

- ScrollbarPlacement 提供 Overlay／Gutter，ScrollArea／VirtualList／MessageScroller 默认 Overlay，Textarea 默认 Gutter；receiver copy 保留既有选择。Gutter 在一次正文测量前扣除固定最大条带，End 侧和双轴角部使用共享几何，范围与反馈过渡不改变占位；虚拟列表分别保留外尺寸和实际正文视口。
- ScrollPadding 同时扩展可滚动范围与排除显露边缘，snapshot 区分含留白的 content、不含留白的 bodySize 和有效 visibleSize。Auto 只按原始正文溢出生成条，不由留白生成另一条。默认位置及无新定位意图时的左／上留白变化按 LayoutCorrection 保持正文位置；明确位置和动画意图优先。
- FocusTarget、文本 caret／拖选／翻页沿同一快照，正文输入区域和 Accepted 显露边界同步变换。TextEditHost 的纯测量回调在排版后一次解析留白；Textarea 无须反复排版。虚拟阅读锚点仅修正主轴，保留交叉轴显露位置。accessibility 设计明确 provider 的 Reveal 复用这条路径，不自行计算未扣遮挡的区域。
- UI 生产／测试语义检查与生产构建通过，滚动定向测试 13 项全部通过；公共滚动原生场景已增加首尾焦点显露、正文位置保持和 LayoutCorrection 验证，源码检查与 release 可执行文件构建通过。组件生产／测试源码检查和生产构建通过，30 项定向测试全部通过，覆盖占位、滚动几何、活动反馈、消息阅读模型、receiver 默认值和文本尺寸；Style 与四套主题已按依赖更新。普通滚动区、虚拟列表、文本控件三个原生场景分别补充占位热切换、RTL、阅读锚点／交叉轴保持、文档末端及实例／选区保留；三个场景的源码检查和 release 可执行文件构建全部通过。本机仍锁屏，四个场景的前台实际运行尚未完成，未据构建结果声明交互验收通过。

UI／Component API 已生成并核对 ScrollPadding、ScrollSnapshot 的 padding／bodySize／visibleSize、ScrollViewport.layout、TextEditHost.measure，以及 ScrollbarPlacement、Behavior／builder 的 placement 和 Textarea 的 Gutter 默认值、receiver 入口与导入路径。新增纯值测试已登记 Windows／Linux GNU 的目标选择；这些目标在集中跨平台轮次运行，Darwin 源选择保持不变。临时 API 站点已清理，保留开发产物与待前台运行的测试程序。

### UI 语义资源、逻辑发布与场景接入

- ComponentScope.semantics 创建 scope 管理的稳定资源，根在首次登记时创建 provider。普通布局透明，多节点及显式内部父子关系按实际挂载顺序发布；子挂载路由可单独指定。关闭一个 handle 保留其他资源拥有的子节点，真正卸载和关根退休全部所属对象。工厂随当前 Window 能力仅在 Darwin 源中提供，资源、路由与场景代码共用，没有空 provider 或运行期平台分支。
- 属性和最新 handler 沿 UiHost 现有逻辑队列发布，不请求 drawable；只有结构变化才遍历挂载树。原生动作检查生命周期、输入边界和 UI 执行状态，经 Surface 错误边界执行并刷新实际 State 结果；普通 scope.dispatch 同样进入宿主回调边界。动作自删或关根保留实际完成结果，异常只关闭对应 UI 根。FocusTarget 绑定读取真实活动焦点，目标退休清除原生焦点事实。
- SceneContext.semantics 通过真实 Canvas 变换与 clip 捕获值型几何，进入缓存记录和重放。仅 Accepted 更新发布几何，Discarded 保留仍匹配的旧值；名称／值变化立即撤下旧几何，未放置不退休身份。捕获后释放 CanvasGeometry，不为属性发布保留绘图资源，也不把矩形中心当成可点击点。
- UI 生产／测试检查及 release 构建通过；两个测试 Cone 和共用 native 辅助检查通过，Objective-C 格式及严格编译检查通过。原生逻辑测试实际通过，覆盖隐藏窗口中 State 与直接 handle 更新、无 measure／scene、阅读顺序、角色替换、最新 handler、Unicode 与拒绝、禁用／隐藏／只读、原生动作自删、两种回调重入、根解除／重附着、旧对象退休和异常收尾。辅助只使用公开 NSAccessibility 协议，不读取生产私有结构。
- 独立场景测试的源码检查与 release 可执行文件构建通过，覆盖目标为缓存重放后的祖先变换／裁剪、零透明度、未放置、名称变更、原生焦点及目标退休、Accepted／Discarded。本机仍为 ScreenIsLocked=true，实际呈现及焦点验证尚未运行；没有据构建结果声明通过，也没有将进程内方法调用记为外部 AX／VoiceOver 通过。

已同步正式 UI 无障碍 spec、Window／UI 引用、设计状态、README 与编程库导航。README 的自定义 Element 示例实际检查通过；UI API 已生成并用 search／show 核对工厂的目标与默认值、SemanticsHandle 只读属性和方法、子项路由、场景入口与 import。临时示例和站点已清理，保留开发产物、共享缓存与待实窗执行的程序。内置控件默认语义、关系／内容合并、完整文本／集合、精确命中和其他平台仍按 S2 及后续批次推进。

### 受保护文本用途与显隐分离

- TextEditOptions 的 protectedText 明确声明受保护用途，默认 false，与显示 projection、只读状态及复制策略分别保存。Default 在受保护用途或内置遮罩时禁用 Copy／Cut，显式 Allow／Deny 保留原意；显示明文不会通过投影隐式开放复制。Paste、DeleteSelection、编辑策略和撤销沿现有事务，不增加文档或 IME 宿主。
- Input／Textarea／InputOtp 及 builder 提供同一标记，InputGroup、ButtonGroup、Command、Combobox、SidebarInput 完整透传。已有密码示例始终声明 protectedText=true，只切换显示投影。用途或复制策略改变后沿原菜单能力刷新，迟到命令继续检查当前资格；完整文本 provider、拖放和系统服务随所属批次消费此事实，没有因添加标记而声明这些能力完成。
- 20 个变更的 Scoop 文件格式检查通过；UI 和 Component 的生产／测试源码检查、release 生产构建及密码示例检查通过。公开接口变化涉及的 Style／四套主题产物已按顺序更新。复用同一组件测试程序，6 项 receiver DSL、3 项 ButtonGroup、4 项 InputGroup 测试全部通过，覆盖标记透传与快照保留。
- 既有编辑器组合用例增加受保护文本的明文显示、默认复制／剪切拒绝、显式允许、IME 连接及模型保持；原生菜单增加 Copy／Cut 禁用、Paste／Delete 可用、撤销和选中菜单项后的策略变化。源码检查与 release 可执行文件构建通过，桌面仍未恢复，未据构建结果声明这些前台交互通过。

同步 UI／Component／原生菜单 spec、accessibility 与交互设计、README 和编程库导航。两份 API 站点已生成并核对 protectedText 的类型、默认值、只读 Props 与可写 builder、导入路径；临时站点已清理。保留开发产物、共享缓存和待实窗执行的程序，其他平台继续留到集中验证阶段。

### 只读可选文本的原生菜单

- SelectableTextOptions 增加与编辑宿主相同的菜单配置，SelectableTextHost 提供键盘式入口；SelectableStyledText 与 Typography 沿现有 options／selection 完整透传。两类宿主共用一个原生会话控制器，只读菜单保留 Copy／SelectAll 及有效分隔线，复制沿 Accepted StyledText 的替代文本语义。
- 支持选区内保持方向、选区外 Preserve／PlaceCaret／SelectWord／平台策略、右键、平台辅助点击及菜单键／Shift+F10。共享 Press 对辅助按下不建立普通点击候选，菜单关闭或禁用也不会将 Control＋点击变成行内链接激活或文本拖选。
- 替换编辑模型前取消旧绑定；只读正文或行内替代文字变化也取消，单纯样式变化保留。焦点／选择回调换绑停止打开，复制开关和配置变化刷新资格；已选中但尚未交付的原生动作仍受同一绑定与最新能力约束。
- 受影响源码格式检查、UI 生产／测试检查与 release 构建通过，两项菜单配置／键盘入口测试实际通过；Style、四套主题和 Component 生产产物按依赖顺序更新。复用既有编辑实窗程序增加十阶段只读菜单场景，源码检查与 release 可执行文件构建通过，覆盖命令过滤、键盘锚点、方向选区、选词、辅助点击、迟到复制、替代文字换绑及样式变化。本机仍锁屏，前台运行尚待验证。

正式 spec、设计／计划、两库 README 和编程导航已同步。UI API 已生成并核对只读菜单配置及方法；使用公开 Component 入口的独立示例实际检查通过。临时 API 站点与示例已清理，保留后续所需的产物、缓存和实窗程序。没有因这批共用菜单接入而声明完整系统文本服务、文本 provider 或其他平台已经完成。

### 语义名称、说明、错误与控件关系

- SemanticRelations 为同一活动 provider 内的有序节点 ID 引用，提供 labelledBy、describedBy、errorMessages、controls；Window／UI update 同时替换事实、动作和关系，节点的只读 relations 返回清理后的当前值。替换前检查归属、重复、自身和名称依赖环，合法的非名称关系循环不被父子树规则误拒绝。目标退休自动清除引用，不跟随同位置或新角色转接。
- 名称、说明和新增 error 属性保留显式值优先及未知／空字符串差异；关联静态文字支持传递名称和非受保护 value。变更沿实际反向依赖更新，原生 getter 不读取业务状态或遍历名称图。hidden 标签可以贡献显式引用的文字，同时继续留在导航树外。
- 名称／值及名称依赖变化使旧几何失效，contentRevision 统一供 UI 候选场景核对；说明、错误和 controls 更新保留匹配的几何。UI bindFocus 读取已清理的当前关系，不重新提交退休引用。AppKit 映射单标签 TitleUIElement／ServesAsTitleForUIElements、多标签合成名称、LinkedUIElements 及包含说明／错误的 Help，并随发布发送适用通知。
- 12 个变更的 Scoop 文件格式检查、Window／UI 生产检查、两个 UI 测试 Cone 与 Window 测试 Cone 检查通过；生产与新增测试 Objective-C 严格编译检查通过。Window 的单应用测试实际通过，覆盖关系解析、依赖更新、错误更新、名称环／重复／自身引用拒绝、合法循环、几何版本、hidden、清理与旧对象，并保留原有原生动作、重入和关闭回归。
- UI 的无 drawable 逻辑测试实际通过：字段引用其他挂载的静态文字，隐藏窗口中的直接标签更新能改变原生字段名称，没有 measure／scene；跨根关系在修改前拒绝。场景测试改用标签依赖，源码检查与 release 可执行文件构建通过。本机仍为 ScreenIsLocked=true，Accepted 场景的前台运行仍待验证。

同步正式 Window／UI 无障碍 spec、两份设计、计划、README、测试说明和编程导航。两库 API 已生成并用 search／show 核对关系字段、error、update 默认值、只读关系／内容版本与导入路径；临时站点已清理。仅按当前消费者更新 Window、Surface、UI 产物，组件和主题待实际消费新接口时再按依赖更新。完整内容合并、控件默认语义、文本／虚拟集合、其他平台与外部客户端继续按统一计划推进。

### 内容名称合并与忽略节点

- SemanticProperties.ignored 仅忽略本节点并展开其后代，hidden 继续排除整个子树；忽略／恢复保留身份，不改变真实焦点，旧引用不能执行被忽略节点的动作，后代仍受祖先禁用约束。AppKit 按发布的父子／阅读顺序维护子项索引，普通名称和几何更新不重建索引。
- Window contentLabels 提供独立于 SemanticRelations 的内容名称回退，优先级低于显式 name 与 labelledBy，普通 update 保留内容引用；空字符串与未知值继续分开，目标退休自动清理。名称环检查从关系替换移到 publish 的实际名称解析，支持同一轮撤下旧内容依赖再建立新关系；显式名称已覆盖的依赖可发布，实际混合名称环失败保留旧原生投影，修正后重试成功。
- UI mergeLabels 默认关闭，开启后只合并无动作／焦点／语义子项且非受保护的 Text 叶子。ignored 包装允许继续查找后代，普通 Group、带子控件的文本、可聚焦文本与嵌套合并容器保持独立；资格或结构变化统一恢复／合并，纯文字变更复用既有依赖更新，不重走挂载树或请求 drawable。
- Window／UI 生产源码和受影响测试 Cone 检查、三个依赖产物的 release 构建、Scoop 格式与 Objective-C 严格编译检查通过。Window 原生测试实际通过，新增 UI 内容合并测试与既有无 drawable 逻辑回归实际通过，覆盖优先级、名称环恢复、展平、动态动作／焦点资格、真实原生动作、重排身份、隐藏／忽略和关闭；隐藏窗口中的纯文字更新没有 measure／scene。

已同步两份正式无障碍规范、设计与计划、README、测试说明和编程导航，生成并查询 Window／UI API，核对 contentLabels、ignored、mergeLabels 的访问器、默认值和 import；临时 API 站点已清理。桌面仍为 ScreenIsLocked=true，未重复启动依赖实际呈现的场景或将本批进程内原生方法记为外部 AX／VoiceOver 通过。内置控件、精确命中、完整文本与虚拟集合及其他平台继续按完整计划实施。

### 已接受场景的原生精确命中

- graphics 所需的不可变 native 命中快照已独立提交，并经 [PR #143](https://github.com/scoop-lang/scoop/pull/143) 合入 main；工作分支与开发 graphics／text 产物已更新。Window 的 NativeSemanticHitTest 只描述配套的 context／retain／release／contains，不依赖 graphics，也不在原生 getter 中回调 Scoop。节点与已发布投影分别保留引用，普通 geometry 赋值清除命中，内容失效、替换、退休和关闭释放各自引用。
- UI 的 Rect／Path 语义登记捕获真实变换、路径 clip、viewport 与层叠顺序；Rect 参与缓存重放，Path 与已有输入登记一致使该场景不缓存。候选、节点及原生投影寿命分别管理，只有内容版本匹配的 Accepted 场景生效；Discarded、场景失败与根关闭释放候选。AppKit 从屏幕点转换到当前窗口逻辑点，按前景顺序查询，支持 ignored 祖先映射和独立后代；阅读顺序不随场景顺序变化，没有命中快照时不猜矩形结果。
- 13 个变更的 Scoop 文件格式、6 个 native 文件格式及生产／测试 Objective-C 严格编译检查通过；Window／UI 及受影响测试 Cone 检查、Window→Surface→UI 的 release 构建通过。Window 原生测试实际通过，覆盖带孔路径、源对象关闭、层叠／阅读顺序、窗口边界、禁用／隐藏／忽略、内容失效、仅矩形赋值及发布失败保留旧快照。UI 无 drawable 逻辑与内容合并两组回归实际通过。
- UI 场景程序新增重叠矩形与带孔 Path、屏幕坐标查询、祖先裁剪、缓存重放、未放置、内容失效、层叠重排和 Accepted／Discarded 组合；源码检查与 release 可执行文件构建通过。桌面仍为 ScreenIsLocked=true，实际呈现及焦点场景未运行；进程内 NSAccessibility 方法结果不代替外部 AX、VoiceOver 或其他平台验收。ActivationPoint 的真实输入资格与可点击位置仍待接入。

正式规范、两份设计、计划、README、测试说明与编程导航已同步；Window／UI API 已生成并查询 setGeometry、NativeSemanticHitTest 的字段和 import，以及 SceneContext 的 Rect／Path 重载。临时 API 站点已清理，保留共享开发产物与待实窗运行的程序。内置控件、完整文本／集合、其余平台和本轮交互能力继续按统一计划推进。

### 纯文本正文、选区与真实编辑事务

- Window 的 Text／TextField 以已知非保护 value 提供正文，selection 保留公共 UTF-8 方向选区；AppKit 复用原文本偏移转换，提供长度、选中文本、UTF-16 单范围和纯文本 attributed string。范围端点不得切断 surrogate；选择可用于只读文本，选区替换检查实际正文版本与当前选区，拒绝禁用／只读写入。选区变化只更新逻辑事实与通知，不清除节点几何。原生发布使用已持有的内部版本，修复了发布过程中访问公共 contentRevision 被重入保护拒绝的问题，没有放宽公共边界。
- UI 的 updateTextEditor 借用同一挂载的 TextEditHost，发布现有模型、选区和真实焦点，保留名称、说明、错误与关系；捕获模型及版本，拒绝旧目标写入。Accessibility 来源与 Paste 共用外来文本清洗，再执行同一单行、字符／长度、Reject／Transform 和 undo 事务；纯选区结束 composition 沿 EditCommand，不重复清洗正文，也不抢焦点。公开选区使 Text 保持独立语义节点。protectedText 与复制开关／显示投影独立，禁止正文、长度和选区读取，保留实际允许的全文替换。
- 16 个 Scoop 文件与 7 个 native 文件格式检查、生产／测试 Objective-C 严格编译检查、Window／UI 及定向 Cone 源码检查通过；Window→Surface→UI 的 release 产物已更新。TextEditPolicyTest 的 9 个值测试实际通过，覆盖新来源、清洗、拒绝与 undo／redo。Window 原生回归、UI 逻辑动作、内容合并和新增文本编辑四个 Cone 实际通过；新用例覆盖 Unicode 范围、单选区／多选区拒绝、只读／禁用、策略、composition、受保护读写、无 drawable 发布和关闭后旧对象。
- 受共用 native 测试接口变化影响的 UI 场景程序已重新构建成功；需要真实呈现／焦点的场景及既有 IME 快照实窗回归仍待桌面恢复后验收，未把进程内方法结果记为外部 AX／VoiceOver 通过。本批复用的 UTF-8→UTF-16 转换算法没有改变；范围几何、单位导航、retained range、内置表单和其他平台继续按 S5／S10 推进。

已同步正式规范、设计、计划、库与测试 README、编程导航及原文本策略设计。Window／UI API 通过 search／show 核对 selection、setSelection、replaceText、updateTextEditor 的签名／默认值／import 与 Accessibility 来源；临时站点在核对后清理，保留开发产物、共享缓存与待实窗运行的程序。

### 只读富文本选区与语义菜单动作

- updateSelectableText 将同一挂载的 SelectableTextHost 接入 Text 节点，正文保留行内 alternateText，选区复用已有区间替换映射；选择部分替代文字覆盖整个对象，caret 使用下游端点，空替代文字按端点 affinity 处理。普通 update 和选择通知刷新原 handle，不替换业务回调，不创建可写文档。宿主按不可变内容身份复用正文与映射，拖选不反复复制全文；内容替换、受保护发布和关闭清理缓存，旧内容动作不能转向新内容。
- SemanticActions.showContextMenu 接到 AppKit ShowMenu 与同一 owner／输入边界，两个文本宿主使用原生菜单会话和 Accepted 锚点。明确菜单请求可先取得实际焦点；未就绪、已有会话或模式禁用不假报成功，只读仍可打开真实可用菜单。原生动作能力增删发送 LayoutChanged，能力相同的 handler 替换不重复通知，也不重建阅读索引。原生菜单继续提供自己的语义。
- 18 个 Scoop 文件与 4 个 native 文件格式、生产／测试 Objective-C 严格编译检查、Window／UI 和三组相关 Cone 源码检查通过。Window→Surface→UI 的 release 开发产物已更新；新增只读富文本、扩展的原生语义动作以及原有编辑语义三个 Cone 实际通过，覆盖 Unicode 与替代文字偏移、方向选区、拒绝／只读／隐藏、内容与样式替换、菜单能力开关、无 drawable 更新和旧对象退休。正文投影复用调整后，只读用例再次通过。
- 实窗编辑程序通过 release 可执行文件构建，两个菜单场景改为从原生主队列调用公开 ShowMenu，并检查返回值、tracking、项目可用性及模型／剪贴板结果。桌面仍为 ScreenIsLocked=true，实际菜单呈现与焦点场景未运行；动作通知的外部客户端观察、AX／VoiceOver 和其他平台也未据进程内方法结果记为通过。

正式规范、设计、计划、使用说明和编程导航已同步；新建只读语义与既有编辑实窗的 README 说明入口和验证边界。Window／UI API 通过 search／show 核对 showContextMenu 与 updateSelectableText 的默认值、签名和 import；临时站点在核对后清理，保留后续需要的开发产物、缓存及实窗程序。

### 基础控件默认语义与组合资源归属

- Label、Button、Checkbox、Input／Textarea 的 Props 和 builder 提供 ComponentSemantics。静态正文保留 alternateText，按钮合并纯标签并保留独立操作子项；最新 onClick 沿原稳定入口，Checkbox 用原 Binding 确认真实结果。输入框使用原 TextEditHost 发布正文、选区、只读、受保护用途和菜单；placeholder 不充当名称，既有 description 作为默认说明／错误，重复装饰文字不另占阅读节点。ButtonGroup、InputGroup、Command、Combobox、Sidebar、原生文件按钮及文字包装同步传递覆盖值。
- 语义几何允许绘制 Element 登记本挂载或逻辑祖先的资源，保留真实 FocusTarget、原有输入与资源归属；兄弟、后代、跨根和重复登记仍拒绝。Rect／Path 的组合场景用例已覆盖此路径并重新构建。UI handle 记住上次成功的关系声明，未变化时复用 provider 已清理的当前关系；关闭标签后继续更新控件不会重提退休引用，显式更换关系仍须验证。
- Scoop 格式检查、native 辅助代码严格编译、UI 与组件（含测试）源码检查通过；必要的 UI、SVG、图标、style／主题和 component release 开发产物已更新。UI 原有语义动作回归与新的基础控件原生回归实际通过，覆盖无 drawable 的逻辑更新、最新回调、纯文字比较、名称／关系覆盖、关系退休、独立子按钮、三态／拒绝／禁用、Unicode 文本选区与 undo、只读、保护用途和旧对象退休。
- 场景程序完成 release 构建；真实呈现、焦点、外部 AX／VoiceOver 和其他平台仍未据进程内结果记为通过。组件只在下层已有 provider 的 Darwin 目标创建真实资源，Windows／Linux 的 provider 及其余控件继续按 S2／S5／S10／S11 推进，不以透明的未接入目标关闭计划。

正式 UI／Component spec、设计、计划、README 与编程导航已同步。Component API 经 search／show 核对覆盖值、Props／builder、默认值和 import，README 示例实际检查通过；临时站点与示例在核对后清理，保留后续所需开发产物、缓存和实窗程序。


### 内置只读文本默认语义

- SelectableStyledText 使用原 SelectableTextHost 发布正文、选区及真实菜单能力；选择通知先刷新同一语义资源，再调用一次最新业务回调。Props／builder 提供 ComponentSemantics，Typography 的可选与不可选两分支传递相同覆盖值，切换分支退休旧对象。
- 六个 Scoop 文件格式检查、Component 及扩展测试 Cone 源码检查通过；组件 release 开发产物已更新。基础控件原生回归实际通过，新增覆盖行内替代文字的部分选择映射、最新回调、只读写入拒绝、禁用选择，以及 Typography 分支替换后的正文、名称和旧对象失效。
- 正式规范、设计、计划、README 与编程导航已同步；API search／show 核对两类 Props／builder 的新字段、默认值及 import，84 个本地 Markdown 目标检查通过。临时 API 站点及不再使用的输出已清理，保留后续需要的开发产物和共享缓存。

本批未改变下层文本范围映射；真实呈现、菜单焦点、外部 AX／VoiceOver、标题级别和行内范围动作的专门语义仍按 S5／S11 验收。


### Field 注释关系与动态连接

- FieldTarget 复用原 FieldLink，把 FieldLabel、FieldText 与 FieldError 的实际节点连接到 Button、Checkbox、Input／Textarea。注释 Group 合并静态内容，保留自定义标签内的独立按钮；错误去空去重与视觉一致，项目符号为装饰。注释关闭／重建在不重新渲染控件父级的情况下刷新关系，换绑不重建输入或编辑模型。
- Field 可显式指定 target，缺省从 label 取得；与已有 label 指向不同目标时报告普通参数错误。独立标题／说明／错误支持 target 和 ComponentSemantics。显式文字、空串及每类非空关系保留优先级，自动关系只使用同窗口的活跃节点。
- 生产及定向 Cone 源码检查、Scoop 格式检查与 release 构建通过；扩展的原生回归实际通过，覆盖注释生命周期、动态文字与错误、独立子按钮、覆盖优先级、分别换绑、静态 title 和文本模型保留。新增禁用用例先复现了标签容器仍报告可用的问题；已同步实际 InputArea 的 enabled，禁用及恢复的原生查询复跑通过。
- 既有 Field 交互程序完成 release 可执行文件构建；桌面锁定下没有运行呈现、指针或焦点验证，也未把进程内查询记为外部 AX／VoiceOver 通过。正式规范、两份设计、计划、README 和编程导航已同步；API 查询核对 Props、builder 及继承字段，README 示例实际检查通过，102 个本地 Markdown 目标检查通过。临时站点和示例已清理。

required／invalid 等表单状态、复杂焦点、模态和其余控件仍按 S5／S11 推进，本批只交付上述真实注释关系及验证。

### Field 必填属性

- SemanticProperties.required 默认 false，AppKit 使用公开的 isAccessibilityRequired 只读属性。状态变化发送适用的属性布局通知，不当作正文值变化；UI 的标签合并与两个文本宿主保留该事实，不触发编辑或焦点动作。
- Field.required 沿现有 FieldTarget／FieldLink 进入 Button、Checkbox、Input／Textarea。每个 Field 拥有自己的必填声明，同窗口任一活跃来源为 true 时目标为必填；来源关闭、换绑和全部恢复 false 直接刷新现有节点。没有新增验证器或命名覆盖状态。只有同窗口目标已提供语义时才把重复的 requiredText 当作装饰，其余情形继续保留提示阅读。
- 18 个 Scoop 文件与 4 个 native 文件的格式检查、Objective-C 严格编译检查通过。Window／UI 含测试的源码检查、Window／Surface／UI／style／shadcn／Component 的 release 产物构建通过；只更新实际依赖这些接口的产物，没有每次执行全库测试。
- Window 原生回归验证必填变化保留正文版本、几何和精确命中。Component 原生 Cone 检查及实际运行通过，覆盖动态启停、独立来源关闭／恢复且父级不重新渲染、多来源保留、控件／声明分别换绑、禁用时仍必填、只读属性能力及旧对象退休；原有文本、选择、关系和动作回归一并通过。

正式规范、两份设计、计划、README 与编程导航已同步，Window API 查询核对 required 的公开属性、默认值和导入路径；临时 API 站点已清理，本批文档的 149 个本地链接目标检查通过。实窗呈现、外部 AX／VoiceOver 和其他平台未据进程内方法结果记为通过；invalid／busy、复杂焦点、滚动语义及其余完整计划继续推进。

### 滚动范围与真实滚动动作

- Window 增加 ScrollBar、SemanticScrollRange 及位置／增减动作。AppKit 通过公开 NSAccessibility 方法读取轴、归一化位置与上下限，容器返回对应轴的实际子节点；NSNumber 写入和增减进入真实 handler。未知与零范围分开，非法比例、只读／禁用／忽略、缺少动作及退休对象按实际状态拒绝。
- UI 的 updateScrollBar 分别连接原 ScrollViewport 与 TextEditHost；范围来自所属挂载最后 Accepted 的快照，动作登记原 ScrollState 的 User 请求。连续绝对请求保留另一轴尚未提交的滚动，增减共用既有边界处理；旧绑定回调失效。请求完成不提前发布位置，也不当作 Reveal 已呈现完成。
- 共享 ScrollControl 为 ScrollArea／VirtualList 提供容器节点、原焦点和子项路由；每轴滚动条由原 ScrollBarControl 持有。Input／Textarea 的条挂在原 TextField 节点下，只读文本仍可滚动。视觉 Hidden／自动隐藏保留非零逻辑轴，未知／零范围与不适用轴忽略；实际条绘制时登记自己的几何，不借容器位置虚构点击区域。
- 32 个 Scoop 文件与 5 个 native 文件的格式检查、Window／UI 与定向测试 Cone 源码检查、Objective-C 严格编译检查通过；Window→Surface→UI→style→shadcn→Component 的 release 开发产物已更新。Window 原生回归实际通过，新增覆盖范围、位置／增减、参数拒绝、只读／禁用、隐藏／缺失／零范围、方向变更和保留旧对象；仅范围变化保留正文版本与几何。UI 的 3 个定向测试通过，覆盖双轴投影、连续请求、发布边界、非法输入和极大有限步长。
- 基础组件的原生 Cone 实际通过，覆盖 ScrollArea 名称／身份、尚无 Accepted 范围时不提供轴以及根关闭；原有文本、选择、表单与动作回归一并通过。新增 scroll-accessibility 场景的源码检查及 release 可执行文件构建通过，覆盖真实条的隐藏／显露、位置写入、禁用／恢复、换绑、只读文本滚动和关闭。该场景依赖 drawable；本机桌面仍锁定，因此只记录构建通过，实窗运行、外部 AX／VoiceOver 与其他平台仍待验。

正式规范、两份设计、计划、README 与编程导航已同步；生成并查询 Window／UI／Component 的 API 文档，核对范围类型、两个 updateScrollBar 重载、动作以及 ScrollArea／VirtualList Props 和 builder 的语义入口。152 个本地 Markdown 目标检查通过，临时 API 站点已清理，保留后续所需的开发产物与共享缓存。Reveal、虚拟集合的完整可访问性和其余计划保持未完成状态。


### Switch、Toggle 与选择组的默认语义

- Window 增加 Switch／ToggleButton／RadioButton／RadioGroup。AppKit 使用公开的 Button + Switch／Toggle subrole、RadioButton 和 RadioGroup；按角色进入真实 toggle 或 invoke。二态控件拒绝 Mixed，参数错误保留原几何和正文版本，不改变已发布树。
- Switch／Toggle 的 Props 与 builder、RadioGroup／ToggleGroup 的组和条目统一提供 ComponentSemantics。Switch 与 Radio 条目复用原 FieldTarget 发布标签／说明／错误与 required；Toggle 和普通 Radio 标签使用既有内容合并，独立交互后代保留。语义使用原 Press、FocusTarget、选择 Binding 与 key，不增加另一份状态或焦点目标。
- 原生动作仅在 Binding 实际采纳请求时报告成功；Radio 重复选择已选项不取消也不重复写入。单选／多选 ToggleGroup 保留原规则，重排更新阅读顺序并保留对象，删除及重建后旧对象不能继续操作。
- 17 个 Scoop 文件与 2 个 native 文件的格式检查、Objective-C 严格编译检查通过；Window 含测试的源码检查，以及 Window→Surface→UI→style→shadcn→Component 的 release 产物构建通过。Window 原生回归实际通过，覆盖新增角色值校验及既有文本、关系、滚动、动作和关闭。
- 新 choice-accessibility Cone 的源码检查与实际运行通过，覆盖角色／子角色、Field 动态命名和必填、绑定拒绝／转换、重复选择、禁用／隐藏、按 key 重排／删除／重建、单选／多选及保留旧对象。StyledToggle 使用真实 StyledText 重载配置继承的 semantics，并通过同一运行回归；已有基础控件原生回归一并通过。

规范、两份设计、计划、README 与编程导航已同步；API 查询核对六类 Props／条目及实际声明 builder 的字段、默认值、访问器和导入路径，继承调用由真实消费者编译与运行验证。临时原生查询程序和 API 站点已清理，保留开发产物与共享缓存。本批没有把进程内方法结果记为 Accepted 几何、真实焦点操作、VoiceOver／外部 AX 或其他平台通过；这些范围及其余控件继续按 S2／S5／S11 推进。

### Window 的 AppKit 拖放接收边界

- 新增 DragFormat／DragEffect、metadata DropOffer、自有 DragContent／DropRequest 和 Window.registerDropTarget。每窗一个资源；单项文本／URL 支持多表示，Files 要求完整列表，Enter／Over／prepare 不读正文，Drop 返回实际接收结果。已协商 Move 不在 BOOL 阶段改成 Copy。
- AppKit destination 处理当前格式／effect、窗口逻辑位置、修饰键及同进程标记；handler 替换、禁用、读取期间状态变化、关窗、迟到调用和重入撤销旧协商。业务同步完成，数据复制后归接收者，不保留 pasteboard 指针。正常延迟原生操作允许在普通 pump 退栈后完成 leave，业务与更深原生重入仍拒绝。
- 新增[拖放测试 Cone](../../../sysroot/src/scoop.window/tests-cones/drag-drop/README.md)，24 个主队列协议步骤通过：真实独立 pasteboard、延迟正文、Unicode／空串／URL／真实文件、多表示、拒收与不部分接收、重入、handler 自关／关窗、保留旧 view、Context 与异常收尾；在 native payload 释放后的 owner 回调继续读取已接收值。测试自行清理临时文件和 pasteboard。
- Window 生产源码检查与 release 构建、测试源码检查、新增原生运行及既有 Window accessibility 原生回归通过。7 个 Scoop 文件和 5 个 native 文件格式检查、Objective-C 严格编译通过；API 查询核对工厂、公共值、handler 默认值及资源访问器和导入。
- 同步 Window 拖放规范、主规范、设计、计划、README 与编程导航，局部 Markdown 链接核查通过；临时 API 站点清理，保留开发产物和共享缓存。

本批仅交付 AppKit 接收边界。source、Surface／UI 路由、文本事务、三个范围的真实 dragging session、独立进程双向操作及其他平台仍按 S6 推进，不把进程内协议方法调用记成系统会话验收。

### UI 拖放接收与 Accepted 命中查询

- SurfaceWindow.hitInput 返回同一 Accepted 输入场景的前景路径与冻结局部坐标，复用真实路径／裁剪／变换和 AllInput 资格；不跟随普通 capture，不派发事件或改变焦点。三个 InputHitTest 实际通过，覆盖离屏 Skia 几何、孔洞、Discarded、当前资格与模态边界。
- UI 的 scope.dropTarget 借用挂载已有 InputArea，按需创建每根唯一的 AppKit 接收资源；候选只沿逻辑父链回退，Drop 重新确认落点与处理器修订，接收后自关仍保留实际结果。DropHover 可追踪读取，禁用／卸载／关窗清理反馈，offer 不在同步查询中重建控件。
- 新 UiDropRoutingTest 用两个真实隐藏窗口和真实挂载，实际通过父目标回退、不穿透兄弟、格式／操作拒绝、处理器更换、模态、数据不匹配、禁用延迟 leave、借用资源、自关及根错误收尾。此测试直接提供候选路径，不声称覆盖实际窗口命中；也断言隐藏窗口未进入 measure／scene。
- Window 增加默认关闭的 periodicUpdates，UI 请求 AppKit 活动拖放更新；切换不使协商失效，禁用／关闭不请求。原 24 阶段 destination 测试加入此选项后再次实际通过。
- 新增 [drop-target 示例](../../../sysroot/src/scoop.ui/examples/drop-target/README.md)，完整 release 检查与链接通过。它包含文本／URL 子区域、文件父目标、当前反馈及接收结果；粘贴、键盘／语义 Press 和文本接收进入同一业务状态，实际 FocusTarget 和文本结果进入原语义树。
- 14 个 Scoop 文件及 2 个 native 文件的格式检查、Objective-C 严格编译通过；Surface／UI 含测试检查及 Window→Surface→UI 的开发 release 产物完成更新。API 文档及查询核对 hitInput、InputHit、periodicUpdates、dropTarget、DropArea／handlers／hover 的签名、访问器、默认值和导入路径；规范、设计、计划、README 和编程导航同步，局部 Markdown 目标检查通过。

本批的实际桌面 Accepted 接收、外部 dragging session、VoiceOver 操作及其他平台尚未验收；原生协议方法、离屏几何、挂载逻辑与示例构建分别记录，不能互相替代。source、文本事务及三个范围的双向系统会话仍按 S6 继续。临时 API 站点清理，保留后续需要的开发产物与共享缓存。

### Window 拖出接口与准备／完成收尾

- 新增 Window.beginDrag、DragSession 和 DragResult，使用当前左键 Down／Dragged 的原生事件，准备时复制 Text／Url／非空绝对 Files，再沿已有延迟原生操作通路启动。Url 同项提供 URL 和文本，路径转换核对原值，不读文件正文；原生预览和修饰键交给 AppKit。
- 普通指针沿 SystemOperation 通知结束，窗口系统移动仍只接受原来的 Down。完成结果回普通 owner 队列且只交付一次，先释放登记再执行业务。Dropped(Move) 不自动删除源；Native source 查询不进入 Scoop，源窗口失活不取消跨程序会话。
- 明确 AppKit 公开 API 的取消边界：cancelPending 只取消未启动请求，close 清除业务回调；活动 OS 会话的数据与对象保留到真实 ended，不伪造 Cancelled 或完整 Move，未知结果保留 None。关窗自动撤销业务入口，原生迟到引用不保留 Scoop context。
- 两个 DragSourceTest 实际通过。原生 item writer 使用独立 pasteboard 和真实临时文件，验证 Unicode／空串、URL 多表示、完整文件列表与无效输入；真实窗口事件方法验证 Down／Dragged、错误参数、窗口归属、启动前取消／关窗及窗口移动限制。显式 NativeDragSource 替身单独验证 Copy／Move／Link／NotDropped、失败、重复／迟到通知、关闭和完成处理器异常；这些替身结果不表示 AppKit 会话已运行。测试清理其临时文件和 pasteboard。
- Window 含测试检查、生产 release 构建、源定向测试及既有 destination 的 24 阶段回归通过；[两窗口拖出／接收示例](../../../sysroot/src/scoop.window/examples/drag-drop/README.md)的完整 release 检查和链接通过。8 个 Scoop 文件、6 个 native 文件的格式检查与 Objective-C 严格编译通过；API 查询核对 beginDrag、结果／资源类型、取消、查询及关闭的签名、默认值、访问器与导入。
- 同步 Window 主规范与拖放规范、设计、计划、README 和编程导航，局部 Markdown 目标检查通过；临时 API 站点清理，保留开发产物与共享缓存。

实际 NSDraggingSession、外部程序的双向 Copy／Move、活动源关闭和跨窗口系统会话尚未验收。UI source、文本拖出与落点事务、其他平台继续按 S6 实现，不以本批准备通路或替身测试关闭对应条目。

### UI 来源阈值、准备与实际帧接入

- ComponentScope.dragSource 借用同一挂载的 InputArea，左键单击超过窗口逻辑阈值后才准备数据并沿 Surface claim 接管；未达阈值或 prepare 返回 None 保留点击。DragSourceStart 同时提供 Accepted 局部坐标和原始／当前输入，DragSourceData 将具体来源与本次完成处理器配对，普通 render 更新不会把旧结果交给新来源。
- 可追踪的 isDragging、update、cancelPending 与 close 已接入；准备回调内取消会使当前准备失效，完成先清状态再走本根业务派发。SystemOperation 和正常失焦不取消已经准备的数据会话，禁用／卸载／关根关闭关联资源。AppKit 活动 OS 会话继续沿 Window 的实际关闭边界，不伪造取消或来源删除。
- 两个 UiDragSourceTest 实际通过，覆盖轴向阈值、不同显示 scale、非法配置、同挂载归属、评估阶段限制与借用资源收尾。UiDragSourceSessionTest 使用真实 Accepted 窗口帧、平移／缩放几何、真实 AppKit view 输入方法，实际通过阈值前点击、拒绝仅准备一次、原回调配对、准备内取消、未启动原生请求的一次完成、禁用和关根收尾。此测试主动在 NSDraggingSession 启动前取消或关闭请求，不冒充系统拖动验收。
- 既有 UiDropRoutingTest 回归通过。双窗口示例接入文本／URL 拖出与原接收区域，保留文件父目标、同一模型的复制／粘贴、键盘入口、真实焦点和语义 Press；release 检查与链接通过，实际跨窗口／跨程序拖动和 VoiceOver 操作待验。
- 10 个 Scoop 文件的格式检查通过；UI 含测试检查、来源接入 Cone 检查及 Surface→UI 开发产物更新完成。9 次 API 查询核对创建入口、Options／Data／Start、更新／取消／关闭、状态访问器、默认值与导入。规范、设计、计划、README 和编程导航同步；局部 Markdown 的 123 个本地目标检查通过，临时 API 站点清理。

本批关闭 UI 来源准备通路的局部交付。文本选区拖出、接收事务与 caret、同文档 Move、活动来源的模态／关闭组合和三范围完整 OS 会话保持未完成；静态 API、实际窗口帧、注入输入、原生 tracking 和外部程序结果分开记录。

### 文本 Copy 接收、编辑事务与边缘滚动

- TextEditHost 在 macOS 接入同一区域的 DropArea；TextEditOptions、Input／Textarea／OTP 的 Props／builder 增加 acceptDrops=true，组合输入透传。只协商 Text／Url Copy，禁用、只读、关闭或旧模型／修订／投影的 Accepted 几何拒绝接收；受保护用途允许导入，不因此开放导出。
- 临时落点 caret 复用实际排版、投影、槽位、裁剪和滚动变换，保持正文选区。数据拖入在有效视口内侧 16 个逻辑像素启动边缘滚动，窄视口不超过半边；普通拖选继续越界滚动。每次滚动等 Accepted 后重新定位，再判断焦点显露，避免抢回原光标；Leave 和退休撤销 deadline。
- TextEditSource.DragDrop 复用 Paste 清洗、一般 transform／accepts／grapheme 长度限制。拒绝不发布候选，完整保留 composition；接受将结束 marked 与落点插入放入同一批次，一次发布和一个独立 undo 项，沿原模型发送编辑及语义通知。接收后才请求普通焦点，业务关闭／换绑后不聚焦新内容。
- TextDropEditTest 的五项模型测试通过，覆盖清洗顺序、反向选区、Unicode／grapheme 裁剪、策略拒绝／异常、组合结束、修订与分离撤销。TextDropHostTest 使用真实 Accepted 窗口、平移后的实际命中、AppKit destination 方法与原生 setMarkedText，最终版本完整通过 Text／Url、开关／只读／禁用、同正文同修订的模型换绑、遮罩、composition 接受／拒绝、自动滚动／Leave 停止、空段落落点、一次接收和回调内关闭根。
- InputOtpTest 的五项测试及 ComponentReceiverDslTests 的六项测试通过，覆盖 DragDrop 的 OTP 分隔符清洗、字符拒绝、长度裁剪与 undo，以及 Input／Textarea／OTP 的默认开关、不可变 Props 快照和宿主选项透传。两组复用同一测试产物。
- UI／Component 含测试检查通过，更新 UI、Style 和四套主题的开发产物；双窗口示例增加实际多行编辑器与同一宿主的文本语义，区域快捷键限制到自己的来源区域，release 检查与链接通过。UI／Component 的 10 项 API 查询核对类型、DragDrop 分支、默认值、访问器、Textarea 继承与导入。29 个 Scoop 文件的格式检查及 native 严格编译通过，179 个本地 Markdown 目标检查通过；两个临时 API 站点已清理。

本批测试使用独立 pasteboard，未改系统剪贴板；原生协议方法在 AppKit 主队列的普通 owner 派发栈外执行，未放宽库的重入限制。曾因系统未授予窗口键盘焦点停在预编辑前置条件，用户准备手动操作后，原命令复用最终产物重跑完整通过；保持真实焦点与原生选区检查，临时 app bundle 已清理。没有启动 NSDraggingSession，尚不能据此确认实际光标观感、完整系统拖动、跨程序结果或 VoiceOver 操作。文本拖出与 Move 仍保持 S6 原范围，其他平台没有据 macOS 结果记为完成。

### 文本选区拖出、完整 Move 与实际窗口验证

- TextDragOptions 为编辑与只读宿主提供独立开关、阈值、Copy／Move、导出策略与完成回调，Input／Textarea／OTP 和组合输入透传；SelectableStyledText／Typography 使用同一只读来源。选区内普通点击延迟到释放定位，阈值后复用 DragSource；Shift、多击和选区外拖选沿原路径。只读仅 Copy，受保护或遮罩文本默认不拖出，独立显式导出配置不改变保护用途。
- Window／DragSourceData 的 localData 仅关联当前 Application 的真实原生来源，不编码到 pasteboard。来源捕获正文修订与选区，单纯选择变化不撤销移动资格，正文变化即使随后 Undo 也不能恢复旧范围的删除资格；DragSession.effects 可在初始操作集合内调整后续协商，实际结果不被后续收窄改写。
- 文本目标接受完整 Move；有损清洗／裁剪拒绝，同文档删除、落点校正、插入和最终选择一次发布、一个 undo，落回原范围无操作。跨文档按各自历史提交，源再次核对内容与策略后删除一次；SourceRetained 明确报告目标已接收但源未删除。正文、选择、语义通知仍使用同一模型，完成处理器属于开始时的配置。
- TextDragEditTest 九项和既有 TextDropEditTest 五项通过，覆盖 Unicode 前后移动、范围边界、撤销、正文修订、策略／长度／单行有损拒绝和删除条件。DragSourceTest 两项通过，覆盖动态操作集合、禁止扩展、关闭后的 localData 清理及真实 Move 结果保留。TextDropHostTest 最终产物完整通过原接收／IME／滚动场景及九组来源接入，其中四次准备检查原生数据／操作后在系统启动前关闭，不误报业务完成。
- 控制台运行曾在预编辑阶段失去键盘焦点；将相同测试二进制放进普通 macOS 应用包后，保留全部断言执行完成。标准 SCOOP_TEST_RUN_DIR 事件记录包含该测试的 begin、无失败的 end 和 complete，耗时约 0.89 秒。没有伪造焦点或绕过原生选区检查，也没有把准备探针当作完整 OS 会话。
- 双窗口示例修正 ParagraphHost 与普通编辑器子项混用的测量错误：说明文本独立成元素，编辑器保留自己的布局与语义。示例显示实际 Copied／Moved／SourceRetained 结果。用户手动确认同框移动、拖到彩色区域复制、同程序跨窗口及外部程序双向文本拖放均正常；CUA 读回复制结果、源保留及跨窗口编辑内容。实际外部程序的具体名称未记录，不据此扩展为所有程序或文件格式通过。
- Window／UI／Component 含测试检查及示例 release 检查／链接通过；ComponentReceiverDslTests 六项和 InputOtpTest 五项通过，复用同一测试产物。更新 Window、Surface、UI、Style 与四套主题的开发产物；43 个 Scoop 文件、3 个 native 文件格式检查和 native 严格编译通过。15 项 API 查询核对默认值、类型、导入、Props／builder 访问器及 Textarea 的继承声明；规范、设计、计划、库／示例 README 和编程导航同步，本地 Markdown 链接检查通过。三个临时 API 站点和两个临时应用包在验证后清理，保留后续需要的开发产物与共享缓存。

本批已完成本地文本来源与 Move 的交付，S6 的文件、取消／活动源关闭、模态和其他平台仍按原计划继续；VoiceOver 操作、真实设备与完整 accessibility provider 没有据文本拖放结果记为完成。

### 桌面恢复后的本地实窗回归

- 沿用普通 macOS 应用包运行正式测试二进制，以 SCOOP_TEST_RUN_DIR 的完成事件和失败列表判定结果；窗口场景顺序执行，每个 Application 用例单独启动进程。没有改变焦点策略或放宽真实呈现、原生选区和停帧断言。
- 文本来源内部刷新在宿主、挂载或绑定退休后只收束资源，修复根停止后的焦点／IME 终止通知再次更新已关闭 DragSource 的错误。公共来源关闭后的更新仍拒绝；编辑、帧复用与文本拖放宿主的关闭回归通过。
- accessibility 场景通过变换／裁剪、Rect 缓存重放、Path 洞、属性失效、焦点、遮挡重排及 Accepted／Discarded 组合。测试先验证透明度为零时保留语义，再恢复有实际 Drawing 的场景并等待重新参与的 Accepted 帧，之后严格验证纯标签更新的缓存复用。
- 编辑长场景的最终版本连续通过三次，覆盖真实 IME、投影、多击选择、编辑／只读原生菜单、语义菜单入口、受保护用途、编辑策略、撤销、换绑、迟到动作和菜单中关窗。测试修正右键事件的真实按钮号与 Shift+F10 的 AppKit 功能键文字；以 PopupCompleted 区分会话完成，并区分禁用但仍挂载与已经关闭的宿主。链接扩选从折叠选区开始，保留默认数据拖出行为。
- 帧复用和双窗口主题偏好热切换通过，保留编辑状态／焦点并在动画结束后停帧。基础 ScrollViewport、TextControls 及 TextDropHost 实窗回归通过；基础滚动测试按现有 State 合同在 render 读取动态内边距与焦点点位，再通过 Props 传入 Element。

- ScrollArea 与 VirtualList 的完整实窗场景通过。共享 Home／End 仅定位启用轴，修复水平 RTL 列表误写纵向留白偏移；悬停触发开启后重新核对当前 Accepted 指针，覆盖边缘开关、离开后重新进入和视口内直接开启三条路径。轨道跳转／按住重复／释放停止、Gutter／Overlay、RTL、范围变化和边缘焦点显露保持严格断言。占位测试改用子元素局部坐标，并分离子场景重绘与主场景推进，确保后续步骤实际执行。
- 最终组件产物再次通过帧复用、双窗口主题偏好和 TextControls 回归。相关源码检查、release 构建与格式检查通过；UI、Style、四套主题及 Component 的开发产物保持配套，复用共享依赖缓存。规范、计划与 README 同步更新，没有改变公开签名。

上述原生方法和实际窗口结果不代替语义 Reveal、外部 AX 客户端、VoiceOver、完整硬件输入或其他平台的验收。

临时应用包、原生对照探针和逐次运行报告已清理；保留后续使用的 GUI 启动脚本、配套开发产物与共享缓存。


### 已挂载语义节点的滚动显露

- SemanticActions.reveal 增加不改变键盘焦点或选区的显露动作；AppKit 使用公开 NSAccessibilityCustomAction，已发布动作列表只读，weak target 不保活窗口。当前 SDK 的 ScrollToVisible 常量没有对应的公开 BOOL protocol 方法，因此没有调用私有 selector；只有 Completed 映射为原生成功。
- UI 在 Scene 登记点保存同一 ScrollReveal 路径，普通 Text 不需要 InputArea／FocusTarget，默认显露不阻止纯标签合并。显露复用 ScrollPadding、实际变换与 Nearest 定位，从最内层逐帧推进；Busy 保留有效请求，匹配的 Accepted 几何确已可见后才可完成。查询、未放置和过期几何不触发布局；后续滚动、内容变化、视口换绑、禁用／退休与关根结束旧请求。
- 新的 accessibility-reveal 实窗测试完整通过，包含两层带留白的视口、真实 Discarded、原生未完成／完成返回、编辑焦点及反向选区保持、禁用／隐藏／忽略／InputScope 边界、自有裁剪、未放置、无帧名称失效、新滚动意图、换绑、内容修订、显式 handler 和关根后保留动作拒绝。测试显式设置初始位置，避免把普通留白阅读锚点修正混入请求断言。
- 现有 accessibility-content、accessibility-scene 和 focus 三个实窗回归通过，覆盖默认动作与标签合并、缓存重放／精确命中／内容失效，以及原键盘焦点显露路径。每个 Application 独立进程执行，四个最终结果均有无失败的 end 和 complete 事件；没有扩大到无关全仓测试。
- 生产与新用例的源码检查、release 构建、native 严格编译、12 个 Scoop 文件格式和 diff 检查通过。Window、Window.asyncio、Surface、UI、Style、四套主题与 Component 的开发产物已按依赖更新，复用共享缓存。Window API 文档生成与查询确认 reveal 的可选回调、None 默认值及导入路径；两份规范、设计、计划、库／测试 README 和编程导航同步，96 个本地 Markdown 目标检查通过。临时 API 站点、应用包及逐次报告已清理，保留后续需要的启动脚本与缓存。

本批只关闭已挂载节点的本地显露能力与原生协议回归。外部 AX／VoiceOver、未实例化条目的 Realize／Reveal 和其他平台继续按 S7／S10／S11 验收。


### macOS 独立 AX 客户端基础回归

- 桌面恢复并用普通应用包运行后，AXWindows 正常返回 AXWindow，先前窗口引用报告 AXApplication 的现象未再出现。客户端通过公开 AXUIElement API 连接独立进程，现有 Accessibility 权限可用。
- 完整外部模式通过按钮、切换、拒绝及成功的 Unicode 值写入、真实聚焦与解除、隐藏／禁用，以及节点、provider 和窗口关闭后的保留引用。应用继续检查实际业务调用次数、状态与身份；最后一个故意抛出异常的收尾动作沿原进程内路径验证。正式测试事件包含无失败的 end 和 complete。
- 修正客户端焦点断言，直接核对应用的 AXFocusedUIElement／AXFocusedWindow 及目标 AXFocused。窗口自身 AXFocused 可以为 false，而当前子节点确实拥有焦点，不能要求两者相等。
- 独立标准 AppKit 禁用 NSButton 对照复现 AXUIElementPerformAction 返回 kAXErrorSuccess 的平台行为。客户端允许这一系统确认，同时严格验证值不变和业务未再次执行；退休后继续验证旧引用不可读取。库的 BOOL／SemanticActionResult 合同未改变，未把系统确认改报成业务成功。
- 客户端 C11 严格编译和格式检查通过，测试使用当前 Window release 产物；规范、设计和 README 同步平台边界，本地文档目标检查通过。原生对照程序、临时应用包和逐次报告已清理，保留后续使用的 AX 客户端与构建缓存。完整关系／文本几何／虚拟集合的外部查询、VoiceOver 与其他平台仍按原计划验收。


### 原始触点、成组归属与按压／拖动

- Window 增加 ContactId、Touch／Pen 和 Began／Moved／Ended／Cancelled，保留 PointerState 与系统真实提供的可选压力。Windows 使用 WM_POINTER 和对应查询，统一交回 DefWindowProc，UI 过滤兼容鼠标副本；GTK 在捕获阶段通过 GtkEventControllerLegacy 读取 GdkEventSequence／笔工具，转换到内容视图坐标，避免 GtkGestureClick 再次交付。单点、失焦、模态和系统窗口操作各有取消入口。AppKit 继续使用真实原生 gesture，不把 NSTouch 设备坐标映射成假触屏。
- Surface 按触点固定 Accepted 路径，claimContacts 先整体发布新归属再取消原后代；回调错误或收尾失效清理本次新归属，原先持有的其他触点保留。Ended 不能新取得归属，晚到事件不重投。beforePress／InputPressSource 统一鼠标和触点的域外按下，hitTest 为无 hover 的抬起提供精确命中。InputActivity.hasContact 跟踪身份与所属 scope，让虚拟条目在接触期间保持。
- Press 提供有限非负的 touchThreshold；创建时先校验再注册处理器，非法创建／更新不会留下继续响应的动作。移出阈值再移回、第二触点、取消和被阻止的结束均不补点击。公共 Drag 的 contacts 选项独立于鼠标 button，DragSample.input 保留 Mouse 或 Contact 来源；第二触点结束单接触拖动，取消只撤销自己持有的接触。自定义识别继续复用原始事件、成组归属和终止规则，无需在 Window 枚举中加入业务手势。
- Slider 复用同一个 Drag 消费触摸和笔尖，保留键盘、绑定、焦点与提交／取消语义。鼠标和触点的卸载场景共同发现取消时读取已退休主题的问题；取消改为只读取当前业务配置和值，保留原有回滚条件，不放宽 ComponentScope 或 ThemeSource 的存活检查。
- Window 的 2 项 C ABI／解码测试和 Surface 的 13 项新触点测试通过；另有 21 项既有归属、域外按下、原生手势和输入活动回归通过。Press／Drag 的触点与原鼠标两个窗口用例通过，覆盖普通 tap、超阈值／被拦截的移动、第二触点、笔尖、独立鼠标取消、非法配置、阻止结束、同步关闭及键盘与鼠标重叠时的取消。取消组合建立真实的鼠标按下路径，保持 Surface 只向活动路径通知的规则。
- Slider 的触点和鼠标两个窗口用例分别通过同一组 64 阶段场景，包括键盘、提交／回滚、业务拒绝与外部更新、配置／方向、焦点、被阻止的释放和卸载收尾。四个最终 UI 用例均在独立 Application 进程执行，正式测试事件包含无失败的 end 和 complete；本批共 40 项定向测试通过。
- Window／Surface／UI／Component 规范、库 README、输入边界方案和编程导航同步；离线 API 文档生成，5 项查询核对 ContactState、claimContacts、InputPressSource、hasContact 和 DragSample 的签名与导入。源码与受影响测试 Cone 检查、release 构建通过；Window、Window.asyncio、Surface、UI、Style、四套主题及 Component 共 10 份开发产物按依赖更新。37 个 Scoop 文件与 26 个 native 文件的格式检查、本地可编译 native 代码的严格编译以及 225 个本地 Markdown 目标检查通过。临时应用包与逐次报告已清理，保留后续使用的启动脚本、API 文档、配套产物与共享缓存。

本批触点窗口测试使用受控 native payload 经过实际队列、解码、Surface 和 UI，不能代表 macOS 直接触屏或 Windows／GTK 的设备输入。Windows／GTK 原生编译、兼容鼠标去重、OLE 触摸启动和真触屏仍按集中平台验收执行。直接触摸的多击、多指识别、文本端点／软键盘、其他控件组合及辅助技术设备行为继续属于 S8／S10／S11。

### 直接触摸滚动与嵌套余量

- UI 增加随挂载关闭的 ScrollPan，借用同一 InputArea／ScrollViewport，TouchScrollOptions 配置启用、阈值和锁轴。ScrollChain 移到 UI 供公共行为与控件共用。首个过阈值的 Moved 才接管并计入完整位移，随后消费逐次增量；新的外部定位、第二触点、被阻止的移动／结束、配置变化、禁用、换绑及关闭取消旧行为。回调内新的请求优先，自己的 User 通知与 LayoutCorrection 不误取消当前滚动。
- Surface 由接触归属者在每个 Moved／Ended 的 Default 阶段明确开放 consumeContactMovement，祖先按各自固定的 Accepted 几何消费剩余量，复用原滚轮的两端点转换与方向／有限值／不超量校验。普通 Slider／Drag 的 claim 不开放共享，自定义识别可使用同一入口，无需扩展 Window 手势枚举或新增注册框架。
- ScrollArea／VirtualList 的 Props 和 builder 提供 touchScroll，沿共享 ScrollControl 接入 Touch／Pen。父滚动接管后取消旧行的输入保留，虚拟列表正常回收；活动接触阻止已显示滚动条的空闲隐藏，结束后沿原 deadline 收尾，不创建周期时钟。
- 18 项 Surface contact 测试、4 项既有滚轮残量回归和 1 项配置测试通过。新增的 UI 48 步窗口场景，以及 ScrollArea／VirtualList 各 20 步场景通过；原有滚动区与虚拟列表两个窗口回归也通过，共 28 项定向测试。实际队列、Accepted 位置、像素读回、取消及资源关闭均有断言；各 Application 独立进程运行。
- 像素测试显式给轨道设置可见颜色，避免默认透明轨道掩盖结果。十万项列表在滚动后的稳定帧最多保留 6 行，原触点行已经关闭；布局最终选择前新旧两个范围允许短暂共存，峰值最多 12 行。停留和抬起后的等待均保留滚动条与停帧断言。
- Window 输入边界方案、Surface／UI／Component 规范、库 README、测试说明与编程导航同步；离线 API 文档生成，5 组查询核对入口、配置默认值、Props／builder、ScrollChain 归属及 contactRemaining。受影响源码与测试 Cone 检查通过，Surface、UI、Style、四套主题及 Component 共 8 份开发产物按依赖更新。25 个 Scoop 文件的格式检查通过；临时应用包与逐次报告已清理，保留后续使用的启动脚本、API 文档、开发产物与共享缓存。

本批新增输入验证使用 macOS 实窗中的受控 native contact 载荷，不代替 Windows／GTK 编译、真触屏或笔设备验收。直接触摸的多击、多指变换、文本端点／软键盘及其他控件组合继续属于 S8；软件惯性、越界回弹与减少动画的运动策略继续属于 S9。键盘和 accessibility 仍复用原 ScrollState 命令与 Accepted 几何。

### 文本短按、长按菜单与直接滚动

- TextTouchOptions 配置启用、窗口逻辑距离阈值和一次性长按，None 仅关闭长按。编辑与只读宿主使用同一个内部 ContactPress；短按按 Accepted 布局定位 caret，长按选词或保留已有方向选区，再进入原生菜单。菜单关闭仍可选词，键盘与 accessibility 使用原命令入口；受保护用途、导出策略和文档投影保持一致。
- 编辑视口独立配置 touchScroll，单行水平滚动、多行双轴滚动沿同一 ScrollViewport 和嵌套余量路径。Press 按下只登记候选，结束激活时消费事件；行内链接 tap、文本长按和外层滚动按原 contact 归属竞争，未新增公开识别器框架。被阻止的输入、第二触点、正文／绑定变化、焦点／配置变化和关闭均收束候选及 deadline。
- Input／Textarea／InputOtp、InputGroup、ButtonGroup、Command、Combobox 和 SidebarInput 透传配置。首次接触聚焦保留新候选，不为旧 caret 发起显露；OTP 在挂载期间复用同一默认投影，避免焦点重绘取消长按。长按结束清除移动标记，下一次被拦截的移动不会补 tap。
- 新的两类文本宿主 66 步和六类文本控件 20 步实窗测试通过，覆盖 Unicode、笔来源、原生菜单与剪贴板、投影／保护、行内动作、滚动竞争、取消、停帧和重入卸载。既有编辑／IME、Press／Drag、嵌套滚动、文本控件和 OTP 的 32 步分组／composition／策略／关闭回归通过；加上配置校验，共 8 项定向测试，其中 7 个窗口用例各自使用独立 Application 进程，均有无失败的 end 和 complete 事件。
- UI、Style、四套主题及 Component 的开发产物已按依赖更新；源码及受影响测试 Cone 检查、release 构建和 30 个 Scoop 文件格式检查通过。两库离线 API 文档重新生成，9 组查询核对类型、默认值、Props／builder 及导入；规范、输入边界、计划、库／测试 README 与编程导航同步，本地 Markdown 目标与 diff 检查通过。

本批触摸测试通过受控 native contact 队列验证 macOS 实窗行为，不能代替真实触屏或笔设备。用户已确认的触控板捏合／旋转结果单独保留；选区端点、多击、触摸拖放、多指变换、软键盘、其他控件和集中平台验收仍按原计划继续。临时应用包和逐次报告在验证后清理，保留后续需要的启动脚本、API 文档、开发产物与共享缓存。

### 分隔面板、轮播与抽屉触摸组合

- ResizableHandleOptions 与 builder 增加 hitWidth，默认 20，要求有限正值，随主题密度缩放；只扩大居中的命中带，不改变面板布局与分隔线。修复纵向命中带被后一个面板编辑器覆盖的问题，使用已有 SceneContext.children(order) 调整呈现顺序，挂载、Tab 和语义阅读顺序仍按声明。
- Drawer 从真实输入位置和 Accepted 几何判断正文／把手，不再依赖 hover。正文 ScrollPan 使用同一阈值，新增 TouchScrollOptions.claimAtBoundary 默认 true，Drawer 正文设为 false，在起始方向无余量时让出候选；已开始的滚动继续保持原 Chain／Contain。零阈值正文等待实际位移，公共 Drag 的未接管候选忽略坐标未改变的 Moved，同时保留按下即开始的零阈值行为。
- Carousel 观察整个交互组的活动触点，关闭 hover／focus 暂停也不会在手指仍按住时自动翻页；最后一根触点结束后重新等待完整 interval。继续复用原定时器和绑定／命令入口，不为触摸建立另一套模型。
- Resizable 57 步、Carousel 61 步、Drawer 73 步分别通过触点和鼠标两个独立窗口用例，覆盖约束、方向、正文滚动、键盘、焦点、嵌套、外部替换、取消、卸载和停帧。Resizable 触点从细线外 12 个逻辑单位开始，Carousel 验证两根触点和完整恢复间隔，Drawer 包含零阈值按下／原地移动及向内滚动。公共 Press／Drag 两项窗口回归、原 ScrollPan 48 步及新增起始边界 13 步也通过，共 10 个窗口用例；每项都有无失败的 end 和 complete 事件。TouchScrollOptions 与 Resizable DSL 两项配置测试通过，覆盖默认值、显式关闭、hitWidth 透传和非法尺寸。
- Carousel 动画暴露 SkRect 先以单精度相减导致的矩形包含误差。独立 [PR #147](https://github.com/scoop-lang/scoop/pull/147) 改为先提升边再计算 Double 尺寸，12 项 Graphics 几何回归与独立预编译消费者通过；已合入 main、同步工作分支并更新本机 ARM64 Graphics 产物，保持 accessibility 的包含校验和动画断言。
- UI／Component 规范、设计、计划、库／测试 README 与编程导航同步；两库 API 文档生成，4 组查询核对签名、访问器、构造默认值及导入。UI、Style、四套主题与 Component 开发产物已按依赖更新，生产及受影响测试源码检查、release 构建通过；上一批 OTP 投影参数的两个内部测试调用一并同步。28 个 Scoop 文件格式检查及本地 Markdown 目标、diff 检查通过；临时应用包、逐次报告和独立复现 Cone 已清理，保留开发产物、API 文档与共享缓存。

本批接触输入经受控 native 队列进入 macOS 实窗，不代替真实触屏／笔、Windows／GTK 或读屏器验收。用户确认的真实触控板捏合／旋转仍单独保留；其余 S8 组合与 S7、S9～S11 范围继续按计划实施。

### 直接接触的双指缩放与旋转

- TransformGesture 从原始 Touch／Pen 识别双指变换，与原生手势共用事件、累计值和业务操作；新增 contacts 开关和默认 8 的 contactThreshold，两个分量可独立关闭。样本 input 保留 Native 或两个真实 ContactId／ContactKind，不为 UI 识别增添 Window 手势种类。
- 在同一 Accepted 局部几何中计算前后两组位置、跨度、连续角度与质心。达到阈值后一次 claimContacts 两个身份，首个样本包含完整初始变化；原有归属冲突时放弃候选。任一指抬起结束并取消另一指的归属，第三触点、阻止输入、失效几何、配置变化和关闭收束整个配对，剩余接触及迟到更新不恢复旧操作。
- 回归复现并修复了来源切换的回调重入问题：取消回调关闭缩放后，正在切换过来的原生缩放曾继续发出 Started。现在回调返回后重新检查该输入种类的最新配置，原生 Began／Discrete 共用此检查；成组接管通知和 Started 回调内关闭也不遗留归属。
- aarch64-apple-darwin release 的 3 项窗口用例和 1 项配置用例全部通过。直接变换 21 步覆盖阈值、完整倍率、跨半圈旋转、Touch／Pen、分量开关、结束和 Accepted 变换；取消 40 步覆盖输入阻止、第三指、已有归属、两种来源切换、重入配置／关闭与退化几何。原生组合回归继续通过，各窗口用例独立运行并取得无失败的 end 和 complete 事件；配置用例检查默认值、开关及非法阈值。
- UI 规范、交互与 accessibility 设计、计划、库／测试／示例 README 和编程导航同步；UI API 文档生成后用 7 组查询核对选项、默认值、来源载荷及取消原因。UI、Style、四套主题与 Component 共 7 份开发产物按依赖更新，示例重新检查并构建；生产／测试源码检查、10 个 Scoop 文件格式检查、168 个本地 Markdown 目标和 diff 检查通过。7 份逐次 GUI 报告及测试应用包已清理，保留开发产物、API 文档、启动脚本与共享缓存。

受控 native contact 队列的 macOS 实窗结果与用户已确认的真实触控板结果分开记录；真实触屏／笔、Windows／GTK 入口和读屏器仍待集中验收。公共手势、文本端点、多击、软键盘与其余完整计划继续推进。

### 文本直接触摸的连续多击

- TextTouchOptions 新增 multiTap（默认 true）和 multiTapInterval（默认 None，跟随系统 doubleClickInterval，未知时 500 ms）。同一宿主、相同 Touch／Pen 来源的连续成功短按按开始时间和窗口距离计数，第二次选词，第三次及以上选逻辑行；显示投影、grapheme 边界与范围算法复用鼠标选择。
- ContactPress 共享计数、来源与取消规则，只保存上一次成功短按，不为间隔建立新定时器。域外接触、失败／被阻止的输入、第二触点、长按、配置变化、失焦、外部选区和正文修改都会打断序列；触摸选择自身的正常选区发布保留计数。行内链接消费的短按只激活动作，选区回调内再次修改或关闭仍清理旧状态。
- aarch64-apple-darwin release 新增 69 步窗口场景通过，覆盖两类宿主、连续四次短按、Touch／Pen 切换、emoji、间隔／距离、开关、Accepted 缩放、长按、区间投影、受保护遮罩与回调重入。原 66 步触摸场景和键盘／鼠标／IME 综合场景分别在独立进程通过，配置用例验证默认值、独立开关与非法间隔。
- Input、Textarea、InputOtp、InputGroup 两类输入及 ButtonGroup 输入框的触摸场景扩展为 64 步并通过，逐项检查二／三击以及 multiTap 关闭后的连续短按，继续覆盖启用、长按、None 长按和模型保留。本批合计 4 个独立窗口用例和 1 项配置用例通过，窗口用例均取得无失败的 end 和 complete 事件。
- UI／Component 规范、交互与 accessibility 设计、计划、库／测试 README 和编程导航同步；UI API 文档及 3 组查询核对构造参数、默认值、只读属性和有效导入。UI、Style、四套主题与 Component 共 7 份开发产物按依赖更新，生产／测试源码检查、release 构建、14 个 Scoop 文件格式和 229 个本地 Markdown 目标及 diff 检查通过。临时测试应用与逐次报告已清理，保留开发产物、API 文档和共享缓存。

本批验证经受控 native 队列进入 macOS 实窗，真实触屏／笔和 Windows／GTK 仍按集中平台轮次执行；文本选区端点、触摸数据拖放、通用长按／swipe、软键盘及其余完整计划继续推进。

### 文本选区把手与边缘滚动

- TextTouchOptions.selectionHandles 接入 TextSelectionHandleOptions：enabled 默认 true，radius=6、hitRadius=18、color=None，独立控制把手显隐、绘制／命中半径及颜色。编辑器沿 caretColor，只读文本沿不透明 selectionColor；普通短按、多击、长按与键盘／语义选择保留各自入口。
- 两类宿主共用非焦点子区域、接触归属与当前场景的绘制／裁剪。达到移动阈值才接管；折叠选区移动 caret，非空选区固定另一端并允许交叉。抓取补偿保留窗口坐标，范围和 affinity 使用原投影及布局。相邻把手分开绘制和命中，不建立新的文本模型、焦点或语义节点。
- 边缘滚动借用现有 Accepted 视口路径，编辑器及只读父视口都沿真实 ScrollState 消费余量；每次滚动等待新布局，再按固定窗口位置更新选区。结束／取消清理归属、意图观察和一次性 deadline，外部定位及模型变化不复活旧操作。第二触点取消拖动时曾被重新当成普通短按，已修正并覆盖；无把手拖动时不拦截其他识别器的第二触点。
- aarch64-apple-darwin release 的 73 步把手窗口场景和 35 步 RTL／取消场景通过，覆盖两端、反向／折叠、grapheme、遮罩、缩放、连续移动与最终释放、内／外视口滚动、停帧、阻止输入、系统／第二触点取消、外部定位／替换、禁用及回调关闭。六类文本控件场景扩展为 114 步并通过，逐项验证真实把手拖动和单独关闭后的短按行为。
- 原 69 步多击、66 步触摸／菜单及键盘／鼠标／系统 IME 组合分别通过；本批合计 6 个独立窗口用例与 1 项配置用例通过。实际窗口截图核对编辑／只读双端点、默认颜色及视口裁剪。此结果来自 native 接触队列与真实窗口，不替代真实触屏／笔或读屏器验收。
- UI／Component 规范、accessibility 设计、计划、库／测试 README 及编程导航已同步；生成 UI API 并完成 3 组构造器／属性／导入查询，7 份 UI／Style／主题／Component 开发产物更新。21 个 Scoop 文件格式、生产与实际消费者检查、release 构建及本地文档链接检查通过。临时应用包、截图工具及逐次报告清理，保留开发产物、API 文档和共享缓存。

文本触摸数据拖放、通用 long press／swipe、软键盘、滚动物理与其余 accessibility 及跨平台计划继续推进；S8 和完整计划仍未结束。


### 公共长按与两类上下文菜单

- 新增 `scope.longPress`、LongPressOptions 和 LongPressSample，配置 enabled、500 ms 默认 delay 与 8 个窗口逻辑单位的 threshold；样本保留真实 ContactState 与 Accepted 局部位置。文本宿主和公共行为共用 ContactPress 的候选、归属和一次性 deadline，无短按回调的行为不消费普通 Ended。配置变化取消旧候选，只换动作使用最新回调；零延迟、第二触点、失效几何、被阻止的输入、失焦／隐藏与关闭都有收尾。
- ContextMenu／NativeContextMenu 的 Props 与 builder 提供 longPress。普通按钮短按、内层文本长按、滚动、右键及 Shift+F10 沿原路径组合；成功长按按真实接触位置打开同一菜单。原生 popup 的临时 pending 不写入持久启用状态，菜单显示期间更新属性后，关闭后的长按仍按最新配置识别。accessibility 设计同步说明键盘与既有文本 ShowContextMenu 独立，原生条目不复制到另一棵语义树。
- aarch64-apple-darwin release 的 34 步公共长按、两组各 29 步自绘／原生菜单、原 72 步菜单家族、两项 Press／Drag 及五项文本窗口回归全部通过，共 11 个独立进程用例，均取得无失败的 end 和 complete。原生菜单在真实 tracking 中更新参数、确认逻辑 render、取消并等待实际关闭，再验证下一次长按。文本回归包含 66 步触摸菜单、69 步多击、73 步把手、35 步 RTL／取消及键盘／鼠标／系统 IME。1 项普通配置测试覆盖默认值和非法时长／距离。
- UI／Component 规范、输入边界、accessibility 设计、计划、库／测试 README 与编程导航同步；两库 API 文档生成并通过 8 组查询核对工厂、样本、参数、导入和两种菜单配置。7 份 UI／Style／主题／Component 开发产物更新，18 个 Scoop 文件格式、源码检查、release 构建及本地文档链接检查通过。临时应用与逐次 GUI 报告清理，保留开发产物、API 文档和共享缓存。

本批触点经受控 native 队列进入 macOS 实窗，真实触屏／笔及 Windows／GTK 仍按集中轮次验收。触摸数据拖放仍依赖 S6 的原生来源接入：当前 AppKit 只接受真实左键来源，不以伪造事件宣称支持。通用 swipe、软键盘、滚动物理与其余 accessibility 计划继续推进，S8 和完整目标尚未结束。


### 公共 swipe 与轮播／抽屉

- 新增 SwipeGesture、SwipeGestureOptions 与真实来源样本。直接触点／可选鼠标复用 Drag 的阈值、归属及取消，在正常释放时检查距离、主轴与原始按下时间；系统离散 swipe 独立启用，按 Accepted 几何转换方向，不伪造距离或速度。配置、动作与准入可更新，关闭跟随挂载；普通滚轮保持原行为。
- Drag 增加独立 mouse 开关及样本 startTime，被阻止的 Moved 通过事件完成通知取消，避免后续释放继续提交。公共 swipe 不另建拖动状态机或时钟；与直接双指变换组合时，第二触点及成组接管后不会补 swipe。
- Carousel／Drawer 的 Props 与 builder 默认启用 nativeSwipe，可单独关闭。轮播沿同一 LTR／RTL、循环、边界和选择绑定；抽屉沿相邻 snap、正文滚动边界、把手 only、嵌套及原关闭请求，保留 veto／绑定拒绝。活动拖动被原生 swipe 接替时先取消，迟到释放不重复提交。
- 嵌套窗口用例复现 Drawer 外部 snap 改变后索引更新而显示高度停在旧值。Drawer 现在在 render 中登记该状态依赖，控制器正常同步位置；无需调用方额外替换 Props，也不把外部改变回放为用户选择通知。相同场景和原有鼠标／触点组合均通过。
- macOS release 的 2 项规则测试、36 步公共行为、27 步轮播和 41 步抽屉窗口场景通过；原 Press／Drag／LongPress、原生／直接双指变换及两控件的鼠标／触点回归一并通过，共 13 个独立窗口用例。覆盖取消、回调重入、真实局部方向、绑定拒绝、无重复动作、停帧和资源收尾。
- 两份 spec、输入边界、accessibility 等价操作、README、编程导航与手势示例同步。示例构建通过，按钮／键盘／语义 Invoke 沿相同卡片切换命令；8 项 UI 与 4 项 Component 生成 API 查询通过，7 个受影响开发库产物已更新。格式、必要生产／测试构建和本地文档链接检查通过；临时应用包和逐次报告清理，保留后续需要的 API 文档与共享缓存。

这些触点和离散 swipe 用例经受控 native 队列进入实际 macOS 窗口，真实触屏／笔、系统 swipe 的设备交付与 Windows／GTK 仍按集中轮次验收。触摸数据拖放、软键盘、滚动物理、其余 accessibility 与完整平台计划继续推进，S8 和完整目标尚未结束。


### 弹性滚动边界与共享呈现

- ScrollPhysicsOptions 提供 Platform／Hard／Elastic、阻力、最大视觉距离、短内容开关与 spring 参数。Platform 明确采用 Darwin 弹性、Windows／Linux 硬边界的 UI 默认策略；连续 Pixels／直接 Touch／Pen 可回弹，离散滚轮只改变合法位置，原生 momentum 不叠加模拟减速。公共 ScrollViewport、文本编辑器及 ScrollArea／VirtualList／MessageScroller／Input／Textarea／OTP 共用配置，组合输入保留透传。
- Surface 的 consumeScroll／consumeContactMovement 增加可选最终余量处理，使用本次冻结坐标和原路由资格，在普通 Default 消费结束后调用一次，零余量也收尾。UI 沿实际参与路径分配合法位移和最终边界，反向输入先恢复祖先已有拉伸；Contain、不同轴、缩放、失效 scope、异常及业务重入不会重复消费或恢复旧效果。
- ScrollLayout.overscroll／presentationOffset 统一内容呈现，ScrollSnapshot 和虚拟索引始终合法；正文、命中、caret、显露与语义几何沿同一 Accepted 变换。编辑器点击区域保持完整视口，拉伸空白不产生命中孔洞。回弹复用已有 AnimationGroup，结束后停帧，动画帧不发布新的用户定位或未变化的 snapshot。
- 根与局部减少动画取逻辑或，内置控件转交主题 Reduced；热切换收束回弹，平滑定位直接到原目标。新输入、显式定位、取消、配置／尺寸变化、禁用／离屏／卸载终止旧效果。旧滚动状态回归发现初始化活动通知误取消首帧前排队定位，已改为识别实际活动状态丢失，同一回归通过。
- aarch64-apple-darwin release 的新 30 步直接接触、26 步原生滚轮、13 步文本回弹及两组各 21 步普通／虚拟列表场景通过；十万项列表最多同时实现 12 行。原两项滚动接触、滚动状态、编辑／IME、两项原生 accessibility 文本和两项组件触摸回归通过，本批共 13 项窗口用例分别在独立进程通过。
- 组件主题热切换测试改为显式配置基础主题 tokens，避免将一次 withPreferences 解析结果当作持久基础配置；Reduced 与根偏好均验证立即收束。组件一次启动前置超时在后续运行未复现；原生菜单复制断言首次失败，相同构建在独立 Application 进程重跑通过，未复现。保留原焦点与停帧检查，不据这些结果声称已定位为系统焦点问题。
- Surface 四项最终余量测试、原四项 Wheel／五项接触消费测试及 UI 两项物理规则测试通过，共 15 项普通测试。三库 API 文档及 23 项查询核对公开参数、默认值、属性、继承和导入；spec、accessibility 设计、输入边界、计划、README 与编程导航同步。8 份 Surface／UI／Style／主题／Component 开发产物更新，53 个 Scoop 文件的格式、受影响生产／测试检查、release 构建和本地 Markdown 链接检查通过。临时测试应用与逐次报告清理，保留开发产物、API 文档和共享缓存。

本批仅交付弹性边界，软件触摸惯性、阅读锚点与其余视觉组合仍按 S9 继续。触点用例通过受控 native 队列进入真实 macOS 窗口，不替代真实触屏／笔、读屏器与 Windows／GTK 集中验收；S9 和完整目标尚未结束。


### 直接接触惯性与嵌套交接

- ScrollPhysicsOptions.inertia 接入 ScrollInertiaOptions，默认启用 Touch／Pen 的释放减速，独立配置减速度、最小和最大速度。单位为窗口逻辑坐标和秒，构造时检查有限值及实际停止时间／距离的可表示范围。最近 100 ms 的原始 InputTime 样本估计速度，合并同刻样本，倒序或停住后释放不沿用旧速度；鼠标 wheel、系统 momentum 和 native gesture 不叠加软件运动。
- 一次释放复用源视口的 Surface 动画组，按实际 Default 参与者共用一段解析轨迹；父子不各自启动相同速度，关闭惯性或减少动画的祖先截断交接，Contain 保持原规则。各轴独立到边界停止，Elastic 将余速交给已有 spring。Surface Animatable.animateTo 的 initialVelocity 仅允许有限的显式 spring 初速度，None 保留当前采样速度，不改变后续重复段规则。
- 根在布局前准备位置请求，Accepted 才发布 ScrollSnapshot 与正文、命中、caret 和语义几何。运动不保留已经结束的 contact，也不产生额外 User／Programmatic 意图。新输入、显式定位、配置／尺寸变化、根／主题／局部减少动画、禁用、离屏、卸载及关闭终止旧运动；释放后的公共 ScrollPan.cancel 也能收尾。
- aarch64-apple-darwin release 的新 18／31 步共享视口、两组各 12 步普通／虚拟列表及 15 步文本惯性场景通过。覆盖缩放、Pen、停止距离、逐轴边界、取消、主题热切换、文本选区与 Accepted caret 几何；十万项来源同时实现不超过 12 行，旧行退休不终止视口惯性。原直接滚动与弹性边界、文本触摸／回弹／系统 IME、滚动状态和两项原生 accessibility 文本回归通过，共 17 项独立窗口用例取得无失败的 end 与 complete。
- 两处测试预期在本批收敛：旧只读文本父视口的直接位移断言未关闭新默认惯性，现明确关闭并由独立带时间样本用例验收惯性；普通 ScrollArea 的垂直末端断言改为检查纵轴最大值及横轴保持，避免把滚动条留槽形成的横向范围当作纵向运动目标。相关用例修正后通过。
- Surface 动画 8 项与 UI 速度／参数 3 项普通测试通过。三库 API 文档生成，12 项查询检查构造器、默认值、只读属性、初速度参数、消费接口与导入；四份 spec、交互／accessibility 设计、输入边界、计划、README 和编程导航同步。8 份开发库产物按依赖更新，29 个 Scoop 文件格式、原生辅助的严格编译／格式、受影响源码检查、release 构建、Markdown 本地链接及 diff 检查通过。临时测试应用与逐次报告清理，保留开发产物、API 文档与共享缓存。

本批触点通过带明确时间的受控 native 队列进入 macOS 实窗，真实触屏／笔、读屏器以及 Windows／GTK 按集中平台轮次验收。阅读锚点、其余视觉组合、触摸数据拖放、软键盘及剩余 accessibility 计划继续推进；S9 和完整目标尚未结束。

### 运动中的阅读锚点与消息跟随

- ScrollState.hasPositionAnimation 只表示显式 animateTo 的等待或播放，isAnimating 继续包含惯性与回弹。惯性采样不推进定位意图序号，VirtualList 的 KeepVisible／FollowEnd 继续使用最近 Accepted 的阅读基点；没有尺寸变化时叠加正常续滑位移，prepend、行高、视口或留白尺寸改变时先修正阅读位置，再停止旧物理运动。显式按 key 显露、恢复及平滑定位保留优先级，None 仅保持合法数值位置。
- 修复惯性期间 prepend 丢失阅读 key 的问题，变更前由实际窗口用例复现。最后一段惯性位移保留到 Accepted，Discarded 不替换锚点或提前结束运动；纯惯性不报告 LayoutCorrection。另一次真实回归复现了场景重绘重复发送旧布局修正，现按本次实际发布的位置去重，同一回归通过。
- MessageScroller 保留同一次用户惯性到达末端的自动跟随资格，来源或范围变化不替用户恢复策略。已在实际末端且没有回合留白时直接进入 Following，消除重复 End 请求提前取消回弹的问题；接近末端或需要清除留白时仍执行实际定位。18 步运动场景覆盖 prepend／可见行增高、丢弃帧、回弹中新增消息、惯性返回末端、显式显露、数据收缩和卸载，原 44 步完整消息列表回归同时通过。
- aarch64-apple-darwin release 的新 26 步可变尺寸列表场景与原定位／阅读回归通过，十万项来源最多同时实现 16 行。共享惯性、直接接触／原生滚轮回弹、滚动状态、普通／虚拟列表惯性和原固定列表 RTL／原生 momentum 回归通过；本批共 12 项实窗用例分别取得无失败的 end 与 complete。状态、留白和意图 13 项普通测试通过。
- 两处测试预期已按真实布局收敛：纵向定位只要求纵轴到达末端，横轴保持原位；尚未实现的 prepend 项使用来源估计值，不能提前按真实行高计算。重绘测试改用仍然挂载的视口请求帧，随后复现并闭合实际重复通知，未以失效测试入口代替通过结果。
- 两份 spec、交互／accessibility 设计、计划、库 README、测试说明和编程导航同步。UI API 文档生成，5 项查询核对新只读属性、既有定位与修正接口和导入；17 个 Scoop 文件格式、受影响生产／测试语义检查、release 构建、218 个本地 Markdown 链接及 diff 检查通过。7 份开发库产物按依赖更新，临时测试应用与逐次报告清理，保留共享产物、API 文档和缓存。

本批继续使用受控 native 接触队列验收 macOS 窗口行为，不代替真实触屏／笔、读屏器和 Windows／GTK 集中验证。高对比度／减少透明度、文字缩放、非活动选区与焦点反馈，以及其余停帧／通知组合继续按 S9 收尾；S7、其余触摸接入和跨平台 accessibility 仍属于完整目标。

### 文本选区与主题偏好

- TextEditAppearance／SelectableTextOptions 增加 selectionTextColor 与 inactiveSelectionTextColor，None 保留原文字样式。文字覆盖复用已经排版的字形与覆盖率，选区矩形按并集合成；有覆盖时颜色／范围参与绘制缓存版本，无覆盖时选择变化继续复用正文。编辑、只读富文本与行内子视图保留原几何和语义来源。
- Theme.preferences 公开已应用的偏好，内置主题及包装直接转交。Input／Textarea／OTP／Typography 共用默认选择配色：高对比度采用完整前景／背景配对，减少透明度采用不透明背景，普通偏好保留原文字。OTP 同步解析槽底色；显式局部覆盖继续优先。Typography.selection 的 None 采用当前主题，Some 保留应用完整选项。
- 修复可选正文中的行内动作持有逻辑焦点时，窗口失焦后未更新选区外观的问题。宿主观察实际窗口状态并请求必要的一帧，关闭时注销观察；不改变系统激活策略，不清除逻辑选择或编辑历史。
- aarch64-apple-darwin release 的 20 项普通测试通过，包含字形覆盖与缓存、四套主题明暗配色、减少透明度与显式覆盖，以及既有 TextField／OTP／Typography 行为。5 项独立窗口用例通过：双窗口选区外观、两根主题与 Input／Textarea／OTP 缩放组合、编辑范围几何及编辑／只读外部 AX 客户端。外观场景覆盖范围／颜色热更新、关闭覆盖、折叠、停帧和卸载；AX 范围、UTF-16 与选择通知沿既有模型验证。
- 额外旧 caret 动画回归未取得通过结果，执行受系统前台焦点条件限制；不把该次结果记为通过，也不据此改动系统激活。依用户要求终止焦点重试及扩大回归，复用已有验收结果。后续按实际改动范围验证，前台策略限制不再成为无关变更的交付门槛。
- 四份 spec、accessibility 研究、计划、三库 README、测试说明和编程导航同步；三库 API 生成并核对 15 项声明。7 份受影响开发产物更新，修改文件格式及必要生产／消费者检查通过。临时应用与逐次 GUI 报告清理，保留共享产物、API 文档和缓存。

其余控件派生颜色与焦点反馈、其余停帧／通知组合继续按 S9 推进；S7、剩余触摸接入和跨平台 accessibility 保持完整计划范围。

### 操作与选择控件的派生配色

- Button／Badge 的非 Default 变体、Toggle 和 Tabs 在高对比度下保持完整语义颜色配对，不再对已解析颜色套固定 alpha 或混合另一配对的文字颜色。悬停过渡的起止同样按当前状态同步前景与底色，旧采样不留下短暂错配；普通偏好的配色与过渡保持原行为。
- 高对比度下，Switch 的关闭轨道与 thumb 保持可辨识对比度，开启时使用 primary 配对。Slider／Sidebar 的默认状态环和 Sidebar 标题不衰减高对比颜色，文字缩小时默认环宽仍至少 2 个逻辑单位。默认半透明底色与装饰在减少透明度时合成到已知主题表面；无填充保持无填充，显式 surface／thumb／states／outline 仍优先。
- aarch64-apple-darwin release 的 6 项新增纯配色测试通过，覆盖四套主题的明暗模式、活动／禁用／选中、过渡起止、0.5／1／2 倍字号、实际表面与环几何，以及局部透明颜色覆盖。启用文字／背景检查至少 7:1，禁用文字至少 4.5:1，Switch 标记和状态环至少 3:1；默认环宽与最终 alpha 一并检查。
- 既有按钮 outline／validation、Secondary 混色、Tabs 状态、Switch 几何和 Toggle 选择／局部覆盖的 5 项定向回归通过，复用同一测试程序；本批共 11 项普通测试，没有启动 GUI 或重复焦点、输入、文本与 AX 验收。
- Component spec、库 README 和计划同步，公共 API 无变化。13 个修改或新增 Scoop 文件格式化，受影响生产／测试源码检查通过；只重建 Component 开发产物，复用 UI／Style／四套主题与共享缓存。

剩余配色现归入三级，待全部功能与三平台验证完成后继续；停帧／通知、S7、触摸及跨平台 accessibility 的功能缺口保持一级。

### 虚拟集合的数据查询与逻辑条目

- Window 增加不可变集合描述、数字／文本 key、按序号／key 查询及明确启用的名称／automation ID 搜索。未提供属性、描述暂不可用、查无条目与不支持搜索分别表达；查询只读同一数据快照，不创建控件或改变滚动与焦点。
- AppKit 的集合按请求范围提供 rows／children，visibleRows 来自实际呈现；普通客户端引用只保留轻量逻辑代理，缓存使用弱值。重排保留代理，过滤／删除永久退休旧引用，同 key 再插入创建新代理。描述不可用时整次范围查询失败，不伪报截断成功。
- UI 将 Lazy 的实际挂载行关联到逻辑 key，行内控件沿真实子树提供能力，几何与命中只取 Accepted 场景。VirtualList 提供 itemSemantics；未提供描述时仍保留身份与集合数量。回收撤下具体子控件和几何，不等于删除逻辑条目；集合内挂载变化只通知该集合。
- Window 的 2 项纯行为测试通过。macOS release 的 VirtualCollectionTest 在一个不请求激活的窗口中通过 7 个阶段：100000 条数据、局部实例化、查询前后 build 数不变、弱引用释放、描述不可用、重排／删除／重新插入、程序化滚入后真实子树／几何／命中、回收和关闭。累计行创建少于 120，描述读取少于 50；未将进程内 NSAccessibility 方法验证当作外部 AX 或 VoiceOver 通过。
- 现有 ScrollAccessibilityTest 定向回归通过，保留实际增减／比例写入、隐藏／显示、禁用、状态换绑、只读文本与关闭覆盖。测试按对应配置的 Accepted 帧推进，并等待淡入后真实几何，避免把完全透明的首帧当作可见滚动条；未调整生产滚动或焦点行为。
- Scoop 格式化、Window／UI／Component 生产与相关测试语义检查、原生 Objective-C 严格编译通过。三层规范、库 README 与编程材料已同步，生成的三套 API 文档及符号查询通过。依赖按接口变化重建，原有输入、拖放、设备手势与焦点验收不重复执行。

本批完成数据访问与生命周期；条目操作由下面一批继续，整体 S7 保持进行中。

### 虚拟条目的实例化、显露与在途保留

- Window 的 SemanticCollectionActions 将明确操作与只读描述分开。AppKit 的 Realize Item／Scroll to Visible 使用公开自定义动作，聚焦使用公开 setter；执行时按当前 key、元数据、owner／模态／祖先与退休状态检查，普通查询不执行动作。数据的 focusable 缺省 false，只在有真实目标时声明。
- UI Realize 复用 LazyChildren 的准备／挂载，在同步原生返回前发布实际行内控件；临时保留只延续到所需边界，随后按原回收规则释放。Reveal 绑定实际 ScrollViewport，纳入已有 pinnedItems，沿原 SemanticRevealController／ScrollReveal 等待 Accepted 可见区域；首帧之前同样观察后续定位意图。Focus 走现有 LazyFocus，清除定位到相同 key 的请求与真实目标，请求替换遵循原控制器。
- macOS release 的 VirtualCollectionActionsTest 通过 11 个原生阶段，使用十万条数据、真实按钮行和 Unicode 文本 key。覆盖同步返回前的子树访问、旧按钮退休、实际显露后调用按钮、定位取消、聚焦请求与同一 native block 内取消、禁用拒绝、尚未呈现时重排、删除后重插入及在途关闭；累计创建少于 150，结束后实际挂载数为零。没有将聚焦排队或进程内方法调用记为真实前台焦点／外部读屏器通过。
- 原 VirtualCollectionTest 定向复查通过，查询仍不增加 build；普通显露复用既有嵌套视口场景，以 SemanticRevealWithoutActivationTest 运行并通过，保留 Accepted／Discarded、逐层滚动、文本选区、拒绝、中断与关闭断言。已有活动焦点用例和设备输入结果继续复用。
- Window、UI、Component 及两个相关测试 Cone 检查通过，Scoop 格式化和生产／测试 Objective-C 严格编译通过。规范、三层库说明、测试说明和编程材料已同步；Window／UI API 文档生成及新增签名查询通过。仅重建接口变更影响的依赖并运行相关窗口场景。

下一批为现有虚拟列表消费者补同源描述和集合关系；S7、其余触摸／手势、动画功能与三平台 provider 继续实施。配色保持三级。
