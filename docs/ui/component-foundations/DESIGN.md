# UI 跨平台抽象与控件基础补齐

日期：2026-11-04。状态：实施中；逐批交付范围见实施记录。本文定义复刻 shadcn 控件之前要完成的基础库工作，承接已经完成的 [UI 基础交互 A–F](../foundations/DESIGN.md) 和 [接收者 DSL 迁移](../DSL-MIGRATION.md)。逐批状态见 [实施记录](PROGRESS.md)；现有接口与验证结论仍以各库 spec 和此前的实施记录为准。

本轮先整理平台合同，再补齐公共布局、交互、文本和绘制能力。沿用 `scoop.graphics`、`scoop.graphics.text`、`scoop.surface`、`scoop.window`／`scoop.window.asyncio` 的分层，不新增统一 platform service、渲染框架或另一套 UI runtime。所有新增能力均为普通库能力，不修改语言、编译器阶段或 runtime ABI。

## 1. 范围与交付目标

后续控件范围以 2026-11-04 调研的 [shadcn Components](https://ui.shadcn.com/docs/components) 为基线：66 个条目中排除 Code Block 和 Native Select，共 64 个。保留自绘 Select 和 Combobox。包括 Attachment、Bubble、Marker、Message、Message Scroller、Questionnaire、Trace 等当前目录中的条目，不把旧目录当成完整范围，也不把 Blocks 或独立 AI Elements 目录并入任务。

本轮约束如下：

- accessibility 语义树、角色、辅助技术桥接及相关功能统一留待后续；正常键盘操作、焦点管理、选择和输入法仍是本轮功能要求。
- 不实现原生 Select／Combobox、系统菜单或文件选择控件。剪贴板、IME、窗口输入和呈现属于自绘控件所需的基础能力，继续由底层接入。
- Windows native 实现和桌面验收由另一分支完成。本轮实现与验收使用 macOS；同时写全 Windows 必须满足的公共合同。
- `scoop.ui`、`scoop.ui.component`、`scoop.ui.style`、各自绘主题及后续 shadcn 库不加入特定 OS 的 native 源码、条件编译或运行时平台判断。Windows 基础库满足合同后，上层使用同一份源码重新构建。
- 项目尚未发布；直接修改 API、源码组织和调用方，删除被替换的设计，不增加兼容别名、转发层或两套实现。
- 本轮交付公共能力和验证组合，不制作 shadcn 的外观、完整控件目录或各控件的业务模型。

“跨平台”要求相同的上层 API、交互规则、资源所有权和失败语义，不要求系统字体、字形或设备光栅结果逐像素相同。公共算法的单元测试可以覆盖不同键位约定，但不能据此宣称 Windows 的窗口、IME、剪贴板或 GPU 后端已通过验收。

## 2. 设计时的基础与缺口

已经具备的事件路由、焦点、portal、虚拟化、文本编辑和绘制缓存继续复用。下表记录设计时识别的演进点；已完成项见实施记录。

| 部分 | 设计时事实 | 本轮工作 | 后续使用者 |
| --- | --- | --- | --- |
| 平台语义 | `Primary` 在 Surface 的 Darwin 实现中直接等于 `superKey`；UI 内保存 AppKit 编辑键位；部分公共剪贴板和文本快照入口只在 Darwin 文件中声明 | 把键位描述、平台解析、显示标签、编辑命令和文本服务合同收回 window，Surface 只做作用域与路由 | Kbd、Command、Menu、Input、Textarea、Combobox |
| 测量与布局 | `MeasureContext` 限制直接子项恰好测量一次；Row／Column 在主轴给所有子项无界约束 | 有限的重新测量、明确的剩余空间分配、换行及共享轨道布局 | Sidebar、Input Group、Button Group、Calendar、Table、Data Table |
| 方向与装饰 | 文本和导航有方向参数；普通布局、弹层对齐和 Style 仍主要使用物理方向／统一圆角 | 树内方向继承、逻辑边、独立边框／圆角及统一几何 | Direction、Sheet、Bubble、菜单和组合输入 |
| 通用激活与拖动 | Button／Checkbox 共享内部 Activation；Surface 已固定按下路径并提供取消 | 公开可组合的激活行为、按下序列归属、轴向拖动、范围及吸附计算 | 任意内容 Trigger、Slider、Resizable、Drawer、Carousel |
| 候选项与悬浮 | 已有 Tab、roving 和网格焦点导航；候选高亮和选择仍需业务自行处理 | 分开真实焦点与活动项，补齐 typeahead、悬浮延迟和跨间隙移动 | Select、Combobox、Command、Menu、Hover Card、Tooltip |
| 弹层退出 | `PortalInstance.close` 立即停用并撤去子树，`onClose` 无法用于保留退出画面 | 关闭请求、退出呈现、最终移除分离，保留现有模态屏障和成功帧边界 | Dialog、Sheet、Drawer、Popover、Toast |
| 文本策略与显示 | 已有事务、IME、撤销、选择、剪贴板和 Input／Textarea；没有公共输入变换及显示投影 | 事务内过滤／长度策略、粘贴变换、遮罩与选区映射、可选择的 StyledText 展示 | Input OTP、密码输入、Typography、Message、Marker |
| 图像与阅读位置 | 已有 Path／Image、渐变、图层、可变高度列表和 FollowEnd；缺少常用图像适配和公开阅读锚点 | Image／Path 自绘适配、按 key 的阅读快照和恢复、滚动意图区分 | Avatar、Attachment、Message Scroller、长列表 |

关键源码依据：

- [Surface 快捷键匹配](../../../sysroot/src/scoop.surface/src/shared/shortcuts/matching.scoop)、[Window 编辑键位](../../../sysroot/src/scoop.window/platform/darwin/editing_keys.scoop)、[提交键判断](../../../sysroot/src/scoop.ui/src/text/editor/commands.scoop)。
- [测量协议](../../../sysroot/src/scoop.ui/src/layout/measure.scoop)、[线性布局](../../../sysroot/src/scoop.ui.component/src/layout/containers.scoop)、[lazy 子项收尾](../../../sysroot/src/scoop.ui/src/lazy/items.scoop)。
- [弹层生命周期](../../../sysroot/src/scoop.ui/src/overlay/instance.scoop)、[成功帧与模态收尾](../../../sysroot/src/scoop.ui/src/overlay/controller.scoop)、[内部激活行为](../../../sysroot/src/scoop.ui.component/src/input/activation.scoop)。
- [文本事务](../../../sysroot/src/scoop.ui/src/text/editor/input.scoop)、[现有单行归一化](../../../sysroot/src/scoop.ui/src/text/editor/normalize.scoop)、[Style](../../../sysroot/src/scoop.ui.style/src/style.scoop)、[内部阅读位置](../../../sysroot/src/scoop.ui.component/src/virtual/position.scoop)。

## 3. 各层职责

| 库 | 负责的公共合同 | 平台相关部分 |
| --- | --- | --- |
| `scoop.window` | 原始键盘／指针事件、快捷键描述与解析、键位标签、语义编辑命令、剪贴板、文本连接／快照、窗口和 deadline | OS 事件及索引转换、键盘布局／惯例、剪贴板与 IME、窗口系统 |
| `scoop.window.asyncio` | 窗口宿主与已有 asyncio 的组合、线程和关闭顺序 | watch／唤醒／原生消息循环适配 |
| `scoop.graphics` | 画布、路径、图像、裁剪、变换、图层和共享几何 | GPU 后端、设备／目标及资源接入 |
| `scoop.graphics.text` | 字体集合、排版、Unicode 边界、caret／选区几何 | 系统字体发现、匹配及 fallback；排版主体继续共享 |
| `scoop.surface` | 帧与输入场景发布、事件路由、焦点会话、按下序列、快捷键作用域、动画 | surface／drawable、帧机会及呈现接入 |
| `scoop.ui` | 挂载与 State、测量／scene、方向继承、公共交互行为、portal 生命周期、滚动和文本宿主 | 无控件专用平台实现 |
| `scoop.ui.component` | Flex／Wrap／Grid 等布局策略、图像适配、候选项和范围等可复用组合；迁移已有控件 | 无 |
| `scoop.ui.style` 与主题 | 装饰几何、颜色、字体选择、部件绘制和交互状态外观 | 无；`macos`、`fluent` 是自绘风格名称 |

上层可以调用基础库公开的 native 互操作包。例如现有 `scoop.graphics.native.captureGeometry` 是共享绘图能力，不等于 UI 依赖 AppKit 或 Win32。禁止的是上层导入 OS API、附带特定平台源文件或重新解释平台事件，不是机械禁止名称中含 `native` 的公共基础入口。

## 4. 具体设计

| 文档 | 决定 |
| --- | --- |
| [PLATFORM.md](PLATFORM.md) | `Primary` 与原始 Super／Control 分离；window 统一描述、匹配和显示；编辑命令与剪贴板／IME 合同；源码边界与 Windows 接入清单 |
| [LAYOUT.md](LAYOUT.md) | 重新测量及 lazy 收尾；Flex／Wrap／Grid；方向继承、逻辑边、装饰和图像适配 |
| [INTERACTION.md](INTERACTION.md) | 激活、拖动归属、候选项／typeahead、悬浮、受控关闭与退出、滚动意图和阅读锚点 |
| [TEXT.md](TEXT.md) | 原生事务与过滤顺序；历史与 composition；显示投影和可选择的 StyledText |

共同规则：

1. 保留一棵挂载树、一套事件路由和一份编辑模型。portal 改变显示位置，退出动画保留已有实例；不复制可交互子树。
2. 行为与外观通过普通 Props、State、InputArea 和内容 View 组合；不移植 React `asChild`、DOM 属性合并或隐藏的全局命令总线。
3. 所有输入只依据最近成功提交的场景；状态变化可以立即撤销资格，新位置／新目标须等 Accepted 帧。退出和重复测量不能绕过这个边界。
4. deadline、动画、任务和资源由现有 owner／ComponentScope 管理。静止及已完成的动画不产生周期唤醒；不创建另一套时钟、事件循环或调度器。
5. 公共入口沿用接收者 DSL。必需模型、内容、动作前置；配置块同步构造完整值；可变 Props 和回调在更新时换成最新值。
6. 公共能力先由已有 Row／Column、Button／Checkbox、Input／Textarea、ScrollArea／VirtualList 和普通第三方 Element 使用，避免只为测试里的假想使用者设计 API。

## 5. 实施顺序

批次的“完成”同时包含所属 spec、实现、实际调用方和必要验证。设计文档中的拟新增类型不会因为文档合入而成为已交付 API。

| 批次 | 工作 | 前置 | 批次完成条件 |
| --- | --- | --- | --- |
| P1 平台键盘合同 | window 快捷键描述／解析／标签；Surface 共享作用域；编辑命令和提交键迁移 | 无 | UI 没有 AppKit 键位表或将 Primary 写死为 Super 的判断；既有快捷键、编辑器和模态路径通过 |
| P2 文本服务与源码边界 | 剪贴板和 IME 公共声明／实现分离；字体 fallback 合同；移除纯 Scoop 上层的 Darwin 源码限制 | P1 | macOS 完整接入公共合同；Windows 所需后端入口逐项明确；纯 UI 生产源码不含 OS 分支 |
| L1 布局协议与空间分配 | 重新测量、lazy 最终收尾、Flex 与既有线性布局迁移 | P2 | 窄窗口、混合固定／弹性内容、嵌套滚动和文字换行不溢出分配框；身份及成功帧规则保持 |
| L2 方向、Wrap 和 Grid | 方向继承、逻辑边／对齐、有限网格和换行布局 | L1 | 同一实例可切换 LTR／RTL；共享列宽与多行基线正确；虚拟表不扫描全量数据 |
| I1 公共激活与拖动 | 迁移 Activation；序列归属、轴向拖动、范围／吸附计算 | P2 | Button／Checkbox／滚动条使用公共行为；拖动接管不会额外触发点击；取消和越界收尾完整 |
| I2 弹层、候选项和悬浮 | 受控关闭／退出；活动项导航与 typeahead；跨间隙悬浮 | I1、L2 | 多层弹层、文本保持焦点、退出中重开和失败帧通过；动画停止后停帧 |
| T1 编辑事务策略 | 公共编辑策略、长度／字符约束和粘贴变换 | P1、P2 | 中文 IME、混合 batch、撤销／重做、拒绝／异常均满足原子性 |
| T2 文本投影与选择 | 遮罩、caret／选区映射、StyledText 选择与链接／标记范围 | T1、I1 | 原文与显示文字不混用；单一焦点／IME 连接；拖选与链接激活不冲突 |
| V1 装饰与图像 | 独立边框／圆角；公共 Image／Path 展示 | L2 | 主题与已有控件迁移；非对称轮廓、裁剪、图像 fit 和资源替换通过 |
| S1 阅读位置与滚动意图 | 公开已提交阅读锚点、恢复和请求来源 | L1、I1 | prepend、异步增高、滚动中更新、位置恢复与用户中断通过 |
| C1 组合验收 | 下述公开 API 组合与 macOS 两架构补验 | 所有批次 | 上层无需新增 native 即能构成代表交互，遗留问题闭合，记录实际验证范围 |

P1／P2 先确定公共平台合同。之后布局、交互、文本可以按依赖分别推进；不把全部控件的状态模型、皮肤或扩展系统塞进任何基础批次。

## 6. 规范与调用方迁移

| 改动 | 同步规范与消费者 |
| --- | --- |
| 键盘、剪贴板、文本连接、deadline 的公共化 | `SCOOP-WINDOW-SPEC.md`；Window／Surface 示例、shortcut／IME 测试与 UI editor |
| 事件序列归属、shortcut scope、caret 提交 | `SCOOP-SURFACE-SPEC.md`；所有 InputArea 默认行为及失焦／取消测试 |
| 测量、方向、portal、公共行为和文本宿主 | `SCOOP-UI-SPEC.md`；ParagraphHost、LazyElement、overlay、focus、scroll、editor 和第三方测试 Cone |
| 布局策略、候选项、图片、阅读位置 | `SCOOP-UI-COMPONENT-SPEC.md`；现有控件／builder、示例和 component tests |
| 圆角几何、字体 fallback、装饰 | `SCOOP-GRAPHICS-TEXT-SPEC.md`、`SCOOP-UI-STYLE-SPEC.md`、`SCOOP-UI-THEME-SPEC.md`；graphics 公共几何相关规范、三套主题与样式调用方 |

每批先把签名、默认值、所有权、异常和生命周期写入所属 spec，再改实现。旧设计中的“每子项恰好测量一次”“Primary 等于 Command”“macOS 专有文本入口”等合同随对应批次直接替换。旧阶段的验收记录保留历史事实，以新记录说明之后的变化，不倒改历史测试结果。

## 7. 验收方式与退出条件

普通算法和 API 行为放在所属 Cone 的 `scoop.test`；需要真实 Application 的测试仍使用现有独立测试 Cone／native 测试辅助。沿用现有测试入口，不为每项能力建立 fixture 或新测试框架。原生 Application 用例按独立进程执行，剪贴板测试保存并恢复原内容。

每批先格式化、检查，再执行受影响的 release 构建和定向测试。只在真实改动涉及编译器、ABI 或 CLI 合同时添加相应 fixture；普通库改动不运行编译器全量测试或全量 GC stress。先完成 aarch64 验证，最后对受影响的生产库和代表组合补做 x86_64 Rosetta 构建／运行，明确其不等于真实 Intel 硬件验证。

组合验收使用无 shadcn 皮肤的普通 View 和 Element：

- 可调整宽度的主内容／侧栏、固定按钮加可压缩输入、换行项和共享列宽；运行中切换方向，保留焦点和 keyed 状态。
- 带搜索输入的候选弹层、嵌套弹层和延迟悬浮；展示同一 ShortcutChord 的标签并触发动作；确认模态退出中不能点透，重开不重建编辑状态。
- 自绘多槽文本和遮罩文本、可选择的 StyledText、范围链接；用真实中文输入法、粘贴、撤销、拖选验证底层组合。
- 可拖动的分隔区域和吸附内容带、可变高度长列表；验证取消、阅读锚点、历史 prepend、图像加载改变高度及手动滚动中断。

这些组合检验公共能力，不成为另一套临时控件库。各基础能力应有真实消费者；组合代码只负责把公共部件连起来，不允许在示例里用 AppKit／Win32 补缺口。

本轮结束时交付：更新后的公共合同、macOS 实现、已迁移的现有库和示例、必要的共享算法及原生组合测试，以及 Windows 接入清单。只有 Windows 分支完成同一合同并实际构建／运行后，才记录 Windows 可用。之后再开始 64 个控件的 API、状态模型、主题和逐项行为对照。

## 8. 留给控件阶段的工作

Calendar／Date Picker 的日期选择与本地化、Table／Data Table 的排序／筛选／分页、Chart 的坐标轴和系列、Toast 队列、Message Scroller 的会话阅读策略，以及完整表单校验／提交都属于控件或应用模型。本轮提供布局、输入、绘图、编辑和阅读锚点，不提前做通用表单、表格、聊天、日期或图表引擎。

图标使用公共 Path／Image 即可；不以 runtime SVG 解析器、SkSL、独立原生弹出窗口、媒体解码、触摸手势竞争、富文档编辑或新的 GPU 互操作作为控件前置。将来遇到实际缺口时再扩展对应基础库。
