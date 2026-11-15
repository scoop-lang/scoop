# 平台合同与源码边界

本文属于 [控件基础设计](DESIGN.md)，规定已完成的 P1／P2 合同；实际交付与验证范围见 [实施记录](PROGRESS.md)。对应 [Window](../../specs/SCOOP-WINDOW-SPEC.md)、[Surface](../../specs/SCOOP-SURFACE-SPEC.md) 和 [Graphics.Text](../../specs/SCOOP-GRAPHICS-TEXT-SPEC.md) 规范。

## 1. 原始按键与应用语义分开

`PhysicalKey`、`LogicalKey`、`KeyLocation` 和 `Modifiers` 继续描述实际事件；不能把 Windows Ctrl 伪装成 Super。`Primary` 是绑定描述中的应用主修饰键，由 window 根据宿主约定解析。

| 描述 | macOS | Windows 合同 |
| --- | --- | --- |
| 原始 `superKey`／`SuperLeft`／`SuperRight` | Command | Windows 键 |
| 原始 `control` | Control | Control |
| 原始 `alt` | Option | Alt |
| `Primary` | Command／Super | Control |
| `Primary + K` 显示 | `⌘`、`K` | `Ctrl`、`K` |
| `Super + K` 显示 | `⌘`、`K` | `Win`、`K` |
| `Primary + Super + K` | 与 `Super + K` 等价 | `Ctrl + Win + K` |

大小写锁定、左右修饰键信息和未知按键编码仍是原始事件数据。普通 shortcut 不隐式要求某一侧或锁定状态；Logical 和 Physical 是调用者明确选择的两种语义，不根据匹配失败相互回退。

### 1.1 描述、匹配和标签同源

把现有 `ShortcutKey`、`ShortcutModifiers` 从 Surface 移入 window，组合成不可变 `ShortcutChord`。Surface 的 `ShortcutBinding` 改为保存 chord、enabled、allowRepeat、canExecute 和 action；scope／host scope 继续属于 Surface。

window 提供三类入口：

- `ShortcutChord.matches(event)`：匹配按键及解析后的完整修饰键集合；不决定事件传播、repeat 或是否执行动作。
- `ShortcutChord.equivalentTo(other)`：比较当前宿主下等价的组合。Surface 在绑定更新时用它检查同作用域内重复的 enabled 绑定，匹配过程复用同一解析规则。
- `Window.shortcutLabel(chord)`：取得当前键盘布局下的 `ShortcutLabel(parts, separator)` 值快照。Kbd 和菜单直接渲染这些片段；不在 UI 中拼 Command／Ctrl 或检查 OS。

`Primary` 与原始修饰键解析后按集合合并。因此 macOS 上 `Primary + K` 与 `Super + K` 是重复绑定，Windows 上则不是；Windows 上 `Primary + K` 与 `Control + K` 重复。重复 enabled 绑定在替换前报告参数错误，不能发布半份 bindings。

Logical 字符匹配沿用大小写无关、保留重音区别和规范等价的合同，Dead 不参与 shortcut。匹配和显示不从 TextInputEvent 反推键盘事件，不执行 locale 敏感的任意大小写转换。Physical 标签由 window 查询键盘布局；无法取得可读字符时使用明确的物理键名称，不将 US 键位字母当成所有布局的实际字符。

标签是查询时的值。窗口获得焦点或调用者重新展示菜单时可以重新查询；本轮不增加全局键盘配置订阅框架。未知 native key 没有稳定通用标签时给出显式的未知键表示，绑定仍按原有 native 编码规则处理。

### 1.2 文本输入保护

Surface 继续负责 scope 优先级、target → ancestor 的默认行为、模态域、repeat、synthetic、canExecute 和动作消费。window 负责把 OS 修饰键／键盘布局信息解释成真实键盘语义。

原生 IME 已消费的按键不能被生成第二次编辑命令。文本焦点中的普通字符、Dead、AltGr 输入不能被祖先的裸字符快捷键抢走。文本编辑默认行为先处理；未处理的应用组合才进入允许的祖先／宿主 scope。按键 up、synthetic release 和 repeat 的既有限制保持。

为 Modifiers 增加可选的 `altGraph` 文本修饰状态：Some(true) 表示后端确认为 AltGr，Some(false) 表示确定不是，None 表示不能判断。Windows AltGr 的识别由后端提供；UI 不用 `Control && Alt` 推断，也不把所有 Ctrl+Alt 应用组合一概禁用。无法区分的输入在文本焦点下按保护文本的规则处理。Fn 同样依据实际可获得的修饰语义，不能把 OS 的“功能键类别”标志误认成用户额外按下 Fn。

这些规则不增加全局热键或系统保留组合的截获保证；普通窗口只处理实际收到的事件。

## 2. 编辑命令由 window 产生

保留已有 `TextEditCommand`、`TextMovement`、`TextDeletion`。把 [原 UI 编辑键位表](../../../sysroot/src/scoop.window/platform/darwin/editing_keys.scoop) 的 AppKit 键位表移入 window 的平台策略，UI 不再保存平台快捷键后备。

`KeyEvent.textCommands` 的新合同为：当前后端针对该事件确定的有序语义编辑命令。优先使用原生编辑系统的命令；只有原生没有产生对应命令、没有通过 IME 消费该键时，才由同一 window 后端补充本平台的标准映射。没有编辑含义的键仍为空数组；空数组不再表示“请 UI 猜平台”。

UI 只执行 Move／Delete／Copy／Cut／Paste／Undo／Redo／SelectAll／InsertNewline／InsertTab／Cancel 等命令。字符插入只来自 `TextInputEvent`，不能再由 logical character 合成插入。原生与后备都识别同一键时只交付一次语义命令。

macOS 的 Command／Option／Control 编辑习惯由 AppKit 后端实现；Windows 的 Ctrl、Home／End、Ctrl+Home／End 等规则由 Windows 分支实现。二者不要求使用同一原始 chord，却必须调用同一组语义命令。

`TextSubmitKeys.Enter／PrimaryEnter` 是编辑宿主的提交策略，仍在 UI；判断时使用 window 的 chord 匹配。单行／多行、Tab 导航／插入、只读和 enabled 也继续属于 UI，不下沉成 OS 控件策略。

## 3. 剪贴板的共同文本合同

`Clipboard`、`ActiveApplication.clipboard` 和 `Window.clipboard` 是公共 window API。共同的 owner、状态检查和文本处理移入共享源码；平台文件只完成原生格式转换和读写。

保留现有所有权规则：应用 owner 线程；应用退出后不可用；窗口关闭不使仍存活应用的入口失效；入口不持有 native window lease，也不要求 close。没有文本返回 None，存在空文本返回 Some("")；原生失败抛出 WindowException，不能伪装成空值。

为避免把 NSPasteboard 的表示能力变成跨平台承诺，统一纯文本数据合同：

1. 文本使用 Unicode，保留非 BMP 字符和组合序列；不进行 NFC 或大小写归一化。
2. API 的行尾为 LF。读取把 CRLF 和独立 CR 转为 LF；写入先做同样归一化，再由后端转换成原生所需的行尾格式。
3. 文本内容不包含 U+0000。`writeText` 在修改剪贴板之前以参数错误拒绝内嵌 NUL；读取到原生格式中实际属于文本内容的 NUL 时，以 Unsupported 错误报告，不能静默截短。原生字符串的结尾 NUL 不属于文本内容。
4. 剪切先成功写入，再改变编辑模型。写入前的参数／转换失败不清空剪贴板；原生写入失败的外部状态按 OS 实际结果报告，不承诺无法提供的系统事务回滚。

这会直接修改当前 macOS [剪贴板测试](../../../sysroot/src/scoop.window/tests/clipboard.scoop) 对 CRLF 和内嵌 NUL 的往返预期。原因是 Windows 标准 Unicode 剪贴板文本以 NUL 终止；不为保留旧行为增加私有交换格式。编辑模型本身仍可保存普通 Scoop 字符串，剪贴板限制不扩散成语言字符串限制。

`ClipboardText`／`writeTexts` 允许 Markdown 编辑器同时提供纯文本和 `text/markdown`，接收端以 `readText(mediaType)` 选择格式。纯文本继续遵守以上换行规则，其他文本表示保留实际换行；所有表示共用 owner、NUL 和失败合同。多格式输入先完整检查，再经同一系统剪贴板发布，不增加进程私有内容回退。当前合同见[窗口规范 §2.2](../../specs/SCOOP-WINDOW-SPEC.md#22-gui-文本剪贴板)。

## 4. IME 文档与 caret 的共同合同

把目前在 Darwin 文件中提供的 `publishTextInputState`、`publishTextInputCaret` 及清除 caret 的公共语义整理到共享 window API。实际 native 文档查询仍由后端完成；不把原生借用或回调指针传给 UI。

- `WindowTextInputConnection` 仍标识一个窗口上的一次连接；重开、换绑和失焦结束旧连接，旧来源事件不能修改新模型。
- 文档快照包含正文、方向选区、marked range 和 revision。公共范围使用 UTF-8 标量边界；AppKit／Windows 的 UTF-16 或其他原生索引在 window 转换。
- 一批原生操作按来源顺序处理。`after`／origin、连接与 revision 保证已处理的编辑和已发布的文档对应；上层不依赖原生序号的具体格式。
- 文档更新可以立即发布。caret 另在成功场景后发布，只有与活动连接、当前 revision 及已处理输入对应时才接受；None 撤销几何。旧 caret 不可沿用到新正文。
- OS 的同步文档查询读取已经发布的 native 快照，不同步回调任意 UI 布局，不启动另一次 measure。
- 未知文本范围或位置返回 unavailable，不用整个窗口矩形冒充准确候选位置。

Surface 的 caret 适配调用同一个公共入口，负责把已提交区域局部坐标转换为窗口逻辑坐标。window 再处理 DPI、窗口／屏幕坐标及原生候选窗定位。不能由各编辑控件写一份平台转换。

## 5. 图形、字体、宿主与时间

`scoop.graphics` 保持共享 Canvas／Path／Image／Drawing 与独立 GPU 后端。新增非对称圆角等形状属于共享几何，通过现有 Skia 路径实现；不在 UI 中增加 Metal／D3D12 调用。

`scoop.graphics.text` 的平台职责不只有“列出字体”，还包括系统字体发现、匹配及 fallback。CoreText 和 DirectWrite 字体管理器继续位于 text 后端。系统默认 sans／serif／monospace 的解析应由这个公共入口保证，不能要求主题猜 OS；显式字体和 `systemFallback = false` 仍能用于确定性排版和测试。

三套主题可以表达自己的字体偏好，但必须有公共 fallback。缺少 `.AppleSystemUIFont`、Roboto 或 Segoe UI 时，任何主题都能依照同一合同排版。字体所有权、字体集合快照、替换后的 Paragraph 缓存失效仍沿用已有合同，不因主题名字限制平台。

Surface 保持 `FrameCompletion`、Accepted／Deferred、输入场景和帧机会的公共意义。不同后端的 display link、DXGI 等差异留在 Surface；UiRoot 不按 OS 决定何时发布焦点、锚点或清理退出画面。

`scoop.window.asyncio` 只整理公共声明与平台适配边界。继续复用现有 Application、Job、owner 线程、watch 和最早 deadline，不增加 UI 专用线程、时钟或事件循环。输入时间和 Surface 动画均使用现有单调时间合同。

## 6. 源码整理与 Windows 接入

| 迁移前位置 | P1／P2 调整 |
| --- | --- |
| `scoop.surface/src/platform/darwin/shortcuts/` | 通用 binding／scope／host／dispatch 移到 shared；key、modifiers、匹配与标签归 window |
| `scoop.window/platform/darwin/shortcuts.scoop` | 公共声明移到 window `src`；原生 Unicode／键盘布局查询保留为内部后端实现 |
| `scoop.ui/src/text/editor/keyboard.scoop` | 删除 UI 平台映射，语义命令生成归 window 后端 |
| Darwin 剪贴板／文本快照入口 | 公共检查及合同共享，原生操作私有化；Windows 分支按同一入口补齐 |
| `scoop.surface` 两后端 caret 适配 | 使用同一快照与几何发布语义，不让 UI 选择旧／新接入 |
| UI、component、style 和三套主题的 `cone.toml` | 生产纯 Scoop 源码不再使用 Darwin 专属条件；真实 native 测试按后端组织 |

不新增运行时平台服务注册、feature negotiation 或假窗口。基础后端缺少本合同所需能力时，不返回空命令／空剪贴板来冒充实现；本分支也不把 Windows 全依赖闭包可构建列为验收。本轮不编写 Windows native 代码，Windows 分支补齐后按普通依赖重建。

Windows 分支的交接清单如下；每项只有在该分支真实实现并验证后才能关闭：

| 合同 | Windows 后端需要完成的工作 | 上层应保持 |
| --- | --- | --- |
| 键盘与 shortcut | Primary=Control、原始 Super=Win、AltGr／Fn 语义、logical 比较和布局标签 | 同一 ShortcutChord／Binding／Kbd |
| 编辑命令 | 原生处理优先、本平台标准后备、避免与 TextInput 重复 | 同一 TextEditHost 命令执行器 |
| 剪贴板 | 标准 Unicode 文本、行尾转换、共同 NUL／失败／owner 规则 | 同一 Copy／Cut／Paste |
| IME | 活动连接、文档快照、UTF-16／UTF-8 转换、原生查询、候选位置 | 同一事务模型与 caret 提交 |
| 输入生命周期 | 捕获中的鼠标序列、取消、失焦／关闭、单调时间 | 同一 Press／Drag／模态行为 |
| 呈现与调度 | 帧完成边界、遮挡／恢复、一次性 deadline、asyncio 唤醒 | 同一 UiRoot／动画／退出逻辑 |
| 图形与字体 | 共享绘图／图像／缓存能力和 DirectWrite fallback | 同一 Style／Image／ParagraphHost |

### 6.1 后端接入点

P1／P2 的共享 Scoop 源码调用以下内部后端入口；补齐后端时沿用现有普通源文件选择，不增加运行时注册层：

- window：`platformPrimaryModifier`、`platformShortcutCharactersEqual`、`platformShortcutLabel`、`platformTextCommands(pointer, key, modifiers, state, synthetic)`。
- window 文本服务：`platformReadClipboard`／`platformWriteClipboard`；`platformPublishText(window, connection, state, after)`、`platformPublishTextCaret(window, connection, revision, area)`、`platformClearImeCaret(window)`。两个发布入口沿现有状态约定返回 0 成功、1 过期／未就绪、2 非法修订或序号；清除入口使用普通 native status。其他错误沿 window 错误映射。
- graphics.text：`st_system_fonts` 和 `st_system_family(category)`。category 的 0／1／2 对应 sans-serif／serif／monospace；返回能由该字体管理器匹配的实际 family。别名和显式字体优先级由共享字体集合处理。
- surface：caret 已使用共享 window 入口；帧、捕获与取消仍依赖既有后端合同。
- window.asyncio：现有共享 WindowLoopHost／Application 执行器与 `platformWatch` 分层已满足要求，继续复用现有 watch、deadline 和关闭顺序，无需新增抽象。

旧 Win32 空编辑命令后备与忽略 revision 的 Surface caret 路径已移除。Windows／Linux 本轮不构建、不声明已可用；相应分支需要真实实现上述入口和文档前述语义，不能用空返回值代替。纯 UI 六个生产 Cone 的源码入口不再限定 Darwin；按平台选择的 native 测试仍保持原有选择条件。

## 7. 验收重点

公共匹配算法以普通值测试两种 Primary 映射、组合归并、Logical／Physical、Unicode 字符和锁定状态。测试可向内部纯算法传入修饰键映射，不新增供应用选择 OS 的公共配置对象。

macOS 真实窗口覆盖 Cmd／Ctrl／Option 编辑、应用 shortcut 与文本焦点优先级、IME 已消费按键、组合提交、剪贴板新合同、原生失败及换绑后的迟到事件。保留现有无焦点 host shortcut、模态 scope 和失败帧 caret 回归。

生产源码核对用于确认边界：上层无 AppKit／Win32 import、OS 编译条件、平台编辑键位表和额外 native source。它只是正常代码审查，不建立源码来源审计或发布证明机制。
