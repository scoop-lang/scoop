# 公共文本编辑

状态及分层见 [基础交互能力设计](DESIGN.md)。目标是普通单行／多行纯文本编辑的完整闭环，供 Input、Textarea、搜索框、Combobox／Command 输入区复用；不包含 Code Block、语法服务或富文档节点模型。

## 1. 复用什么，新增什么

已有 `TextInputState` 负责 UTF-8 范围和 IME 批次的纯值应用；已有 `TextLayout` 负责 shaping、换行、grapheme 边界、命中、caret 和双向文字的 selection 几何；已有 Surface 负责输入区域与旧／新 IME 会话切换。这些继续是唯一对应实现。

在 `scoop.ui.text` 增加以下公共适配，不复制字体排版或 native 输入法桥：

| 对象 | 职责及所有权 |
| --- | --- |
| `TextEditorState` | 可观察的文本／selection／composition、修订及编辑历史；由调用方或组件持有，可在 UI 卸载后继续存在 |
| `TextEditHost` | 一个挂载编辑视图的输入、焦点、布局、滚动显露、选择／caret 绘图和 IME 接入；资源随 ComponentScope 关闭 |
| `TextEditOptions` | 单行／多行、readOnly、换行／Tab／提交策略及实际编辑所需参数；不包含主题外观 |
| `TextEditAppearance` | 显式字体、颜色、选择与 caret 参数；来自控件／应用，不在框架里查默认主题 |

一个 TextEditHost 连接一个 state；同一 state 可以有只读观察者，但同一时刻只允许一个活动编辑会话，避免两份 selection／composition 互相覆盖。不为当前需求扩展协同编辑。

Input／Textarea 在 `scoop.ui.component` 中组合该适配、边框、padding、placeholder、焦点环和验证说明。第三方只依赖 ui.text 就能自绘文本框；框架不识别具体 Input 类型。

## 2. 状态与编辑事务

`TextEditorState` 内部以 TextInputState 的文本、选区、marked range 和 revision 为规范表示；不再并行维护一个可写 String binding 作为第二份真值。公开读取 text／selection 的 getter 参与普通 State 观察，修改通过 `replace`、`setSelection`、`applyInput`、`undo`、`redo` 等明确入口完成。

调用方可把 TextEditorState 直接放在业务 Store。从服务端整体替换内容使用显式 `replaceAll`，取消当前 composition、清理不适用历史并收敛 selection；普通输入不是每次 onChange 再整体覆盖回来。编辑通知提供已生效结果，不能触发递归的相同编辑回放。

每个编辑入口先产生完整合法结果，成功后一次发布。范围仍是 Long UTF-8 字节偏移，保留 anchor／cursor 方向；文档修改、选择或组合变化使用单调 revision，caret 的屏幕移动不推进文档 revision。只使用现有实际范围／边界检查，不额外重放历史证明状态正确。

历史以用户可理解的编辑事务组织：连续普通键入／退格可以在相同选择方向与编辑种类下合并，粘贴／剪切／外部替换／焦点切换形成边界。撤销保存需要恢复的文本差量和前后 selection，不为每次 caret 闪烁保存全文。

组合输入从首次 preedit 起保存替换前基线；preedit 更新只更新当前组合，不各自生成 undo 项。Commit／明确结束组合后作为一次历史操作；取消组合恢复基线文本和选区，revision 继续前进，不倒退到旧快照的版本号。空 preedit 的底层 apply 规则仍是删除正在显示的组合段，恢复替换前内容属于上层编辑事务的责任。

undo／redo、外部文档替换、pointer 改变选区等需要打断组合的操作，先按 IME 连接规则结束或取消组合，再做一次确定操作；不能让 native 仍认为旧 marked range 存在。失焦／禁用／卸载按平台撤销组合并完整结束旧输入会话，后续批次不落到新 host。

## 3. 选择、导航和绘制

首批共同提供：

- 点击定位、Shift 点击扩展、拖选；双击选择 word，三击选择逻辑行，拖到 viewport 边缘自动滚动；多行 selection 复用 TextLayout 的多个几何片段。
- 左右移动／删除以 grapheme 为基本单位，不能切开 emoji ZWJ、组合音标或 surrogate 对应的 UTF-8 字符；word、行、文档起止由语义命令表达。
- 视觉左右移动尊重 bidi 和 affinity；上下移动保存期望 x，以布局行和 hitTest 确定新位置。软换行两侧的同一 byteOffset 可以有不同 affinity。
- Shift 扩展保留固定 anchor；未带 Shift 的移动按命令折叠原选区。选择／caret 更新后通过共用 reveal 显露活动端。
- readOnly 仍允许聚焦、选择、复制和导航，拒绝会改变文本的默认动作，并结束文本输入连接而保留键盘焦点；Surface 的文本接入开关须能随当前编辑资格变化。disabled 按普通输入禁用处理。单行 Enter 触发 submit，多行 Enter 插入换行，Tab 按显式策略。

`graphics.text` 需补 word 范围／边界、视觉相邻 caret、行导航查询。使用已有 Skia／HarfBuzz／ICU 排版链路或所选成熟 Unicode 实现提供的结果，不用空格切词、ASCII 索引或按 code point 近似 grapheme。公共坐标仍是字节和 affinity，不把 UTF-16 索引泄露到 UI。

布局缓存随文本、字体、可用宽度和相关排版输入变化。选择和 caret 是独立场景绘图块，可在静态正文缓存之上更新；placeholder 不写入文档。marked range 的装饰来自 composition 状态，不能混入业务 text 的控制字符。

多行编辑适配复用 ScrollState；单行可以水平显露 caret。caret 候选框沿场景变换到窗口逻辑坐标，再由 Window 转成原生坐标；UI 不重复做 DPI 换算。组合期间依靠 SCROLLING 中的 reveal 约束保持 caret 可见；未组合的离屏聚焦项在下一次编辑时请求显露。没有有效几何时原生查询返回 unavailable，不伪造上次已经移走的位置，也不承诺平台一定允许单独隐藏候选窗。

caret 闪烁只在活动编辑焦点、适用选择状态且实际可见时预约下一次改变；失焦、关闭或不可见时取消。每次输入／导航重置可见阶段，用宿主一次性 deadline 请求所需帧，不能运行一个永久 60 Hz 动画检查是否该闪烁。拖选自动滚动同样只在真实拖选和边缘状态期间推进。

## 4. GUI 剪贴板与键盘命令

`scoop.window` 增加应用 owner 上的纯文本剪贴板读写。当前 terminal OSC52 clipboard 不能替代 GUI clipboard。首批 Copy／Cut／Paste／SelectAll 只处理 Unicode 纯文本，不加入 HTML、富文档、拖放或剪贴板历史服务。

Copy 读取当前选区；Cut 在成功写出后删除；Paste 成为一个编辑事务，并按单行／多行规则处理换行。没有可用文本时不修改文档。readOnly 允许 Copy／SelectAll，禁用 Cut／Paste；失败通过普通结果／错误报告，不先删除再发现写入失败。

输入文字只来自 TextInputBatch。Keyboard 默认动作提供移动、删除、选择、剪贴板、撤销和提交，不把 `LogicalKey.Character` 再插入。平台 Primary 和默认编辑键位由公共编辑映射处理，应用仍可通过路由入口显式覆盖。

为保留原生编辑解释，Window 把已识别的 `doCommandBySelector` 等语义命令按顺序附在这次 Keyboard 交付中，例如 MoveWordBackward／DeleteForward／InsertNewline；同一原生解释产生多个命令时保留列表顺序。编辑器优先使用这些语义；没有语义载荷时再采用平台缺省映射。原生仅转发了一个键时不额外生成第二次编辑，组合或 commit 已消费该键时继续禁止再执行其确认命令。未知 selector 沿已有 Keyboard 回退，不静默吞掉所有按键。

这仍是一条路由：按键信息与可选语义载荷一起进入 capture／target／bubble／default。不建立旁路，让快捷键和编辑器各自先执行一次。ShortcutScope、模态和 IME 的优先规则见 [INPUT](INPUT.md)。

## 5. Native surrounding 文本快照

当前 AppKit 只掌握 marked segment 与候选框，完整的 attributed substring／selected range 查询没有文档快照。基础 preedit 演示能工作，但这不足以承诺普通原生文本服务完整可用。

Window 增加当前输入连接的只读快照入口，包含文档文本、selection、marked range、revision，以及该连接的有效身份。TextEditHost 在每次已生效编辑后立即更新这个快照，和有没有 drawable 无关；caret 几何在成功帧后单独更新。不能让输入法等 GPU 成功提交才读到新文本。

原生 `NSTextInputClient` 等同步查询只读原生持有的不可变副本，不在查询栈里任意回调托管 UI 或启动布局。UTF-16／UTF-8 换算复用现有桥，拒绝切断 surrogate 或引用未知范围。能提供精确结果的查询返回对应范围，尚无有效布局几何的查询明确返回 unavailable，不用编造坐标完成协议。

只有引用实际文档快照的批次携带对应 baseRevision；CurrentSelection 的基础输入继续按现有合同处理。过期范围拒绝整批并按当前输入连接撤销／重建组合，不部分应用，也不把旧范围重新解释为新文档的位置。旧连接关闭到 Disabled 的队列仍由旧接收者处理，不能靠切换一个全局 editor 指针直接送给新目标。

普通 Input 的首批目标是纯文本。如果增加 password 模式，需要同时明确显示掩码到真实字节位置的映射、复制策略和原生 secure input／surrounding 暴露策略；不能只在 paint 中画圆点就宣布密码框完整。这一模式不作为普通文本闭环的隐含前置。

## 6. 交付与验收

先完成 TextEditorState 的纯事务与历史、graphics.text 导航查询，再连接 Focus／Shortcut／Scroll／IME，最后组合 Input／Textarea 外观。组件默认行为进入 component spec，纯文本和窗口协议分别进入其所属 spec，不把编辑器所有功能塞进 Surface。

至少覆盖：单／多行、逆向 selection、emoji／ZWJ／combining marks、混合 RTL、软换行 affinity、上下移动期望 x、拖选自动滚动、word 删除、剪切写出失败、粘贴单事务、撤销／重做、连续 preedit／commit／cancel、替换非空选区后取消组合、旧 revision 与失焦切换、只读、首次聚焦与程序 reveal。

原生组合覆盖中文／日文候选确认、选择后替换、失焦取消、候选窗在缩放／滚动／Dialog 中的位置；确认组合的 Enter／Escape 不额外触发表单／弹层动作。缓存命中、虚拟项临时固定、窗口隐藏和提交失败不能改变已生效文档或把输入投递到错误 editor。实际 AppKit 查询与剪贴板需要真宿主验证，不能只用构造的 TextInputBatch 单元测试代替。
