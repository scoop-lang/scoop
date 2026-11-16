# UI 原生菜单组件规范

本文规定 `scoop.ui.component` 的原生菜单描述与组件公共合同，低层菜单、角色、快捷键与原生使用见 [Window 菜单规范](SCOOP-WINDOW-MENUS-SPEC.md)，所属 scope、状态观察及错误边界见 [UI 桌面规范](SCOOP-UI-DESKTOP-SPEC.md)。托盘包装由独立 `scoop.ui.component.tray` Cone 消费这些公共描述，不给普通 UI／component 添加托盘依赖。

## 1. 菜单描述与求值

`NativeMenuContent(entries: Array<NativeMenuEntry>)` 是不可变描述，条目使用显式 `ViewKey`。同级 key 不能重复，子菜单各有自己的 key 空间；`ViewKey.Position` 不适用于原生菜单。支持 Action、Role、Checkbox、Radio、Submenu 和 Separator。它不接受任意 View、布局、Portal 动画或自绘菜单的关闭否决。

`NativeMenuItemSpec(key, label, enabled = true, image = None, shortcut = None)` 保存原生显示输入，图像使用 Window 的 `DesktopImage`，快捷键使用 `ShortcutChord`。enabled、文字与图像在调用者 render 中按正常值求值；checkbox 和 radio 显式保存 Binding。Role 与应用回调互斥，实际角色行为和快捷键可表示性由 Window 处理。

`nativeMenu(configure: NativeMenuBuilder.() -> Unit): NativeMenuContent` 同步收集描述。builder 提供 `add(entry)`、`item(key, label, onSelect, configure = {})`、`role(key, label, role, configure = {})`、`checkboxItem(key, label, checked, onCheckedChange = {}, configure = {})`、`radioItem<T: Equals<T>>(key, label, value, selected, onValueChange = {}, configure = {})`、`submenu(key, label, content, configure = {})` 和 `separator(key)`。item 的动作接收真实 `MenuActivation`，应用默认菜单可据此取得触发窗口。`NativeMenuItemBuilder` 的可配置字段为 enabled、image、shortcut，默认值与 spec 相同；不能配置 native 不支持的显示能力。

`NativeMenuContent.snapshot(): NativeMenuSnapshot` 读取 checkbox／radio Binding，产生本轮原生显示快照。组件必须在所属 render 中调用；builder 本身不读取 Binding，mount／update 不承担响应式读取。快照保留这轮的动作与 Binding 引用，不创建 OS 资源。

checkbox 激活从 Binding 读取最新值、写入反值并重读接受值，只在接受值变化时调用 onCheckedChange。radio 同样使用类型化值和 Equals<T>，写后重读，并只对接受的变化通知。受控模型拒绝改变时不保留独立的 native 选择状态。

## 2. 原生树与资源归属

`NativeMenuTree(application, snapshot)` 创建并拥有一个 Window `Menu`，`menu` 属性仅供附着或 native 借用；调用者不能在树之外修改其条目。它实现 Closeable，提供 isClosed、`update(snapshot)` 和 `activate(event): Boolean`。该窄资源供菜单栏、popup 与独立托盘组件共用；事件登记与所属 scope 的 dispatch 由实际使用者完成。

每棵树按完整 key 路径协调条目。同级重排保留 MenuItemId，移除再添加或改变条目类别建立新身份；不同实际菜单树具有独立 Menu／MenuItemId。每次 update 都替换动作和最新输入，即使显示完全相同；原生显示未变化时不调用 Menu.update，不请求 Surface 帧。DesktopImage 是不可变值，同一图像实例可直接复用。

更新先退休移除项和改变类别的项，再解除发生变化的旧快捷键，最后发布新顺序和显示，使交换两个既有快捷键不会被中间状态的重复占用阻止。最终冲突或 OS 无法表示的快捷键仍沿 Window 抛错。失败向所属 scope 传播，由其正常清理树与原生使用，不继续使用部分更新的资源。

activate 只接受自己的 MenuId、仍存在且祖先均启用的条目；Role、Separator、Submenu 没有应用动作。关闭或移除后到达的旧动作不执行。应用动作在实际使用者的窗口／应用 dispatch 中执行，错误不能越过原生回调栈。

关闭先撤销条目动作，再请求 Menu 关闭。实际 native 释放仍等待菜单栏、popup 或托盘的使用退休；托盘组件继续遵守 NativeMenuLease 的结束 tracking／detach／close 顺序。树不拥有 Application，也不销毁借用的 Window。

## 3. 原生 popup 触发器

`nativeMenuButton(text, menu, configure = {})` 返回普通按钮 View，`NativeMenuButtonProps` 保存 `PopupTriggerSpec`、NativeMenuContent，以及可选 theme、group、handle 和 onClosed。`NativeMenuButtonBuilder` 提供 enabled = true、variant = Default、size = Default、style = buttonStyle()、focus = FocusOptions()、fonts／theme／group／handle = None、onClosed = {}、key = Position；style／focus 的 receiver 构造沿用对应 builder。按钮外观、键盘激活与焦点来自现有 Button。

`nativeContextMenu(content, menu, configure = {})` 透传正文布局，`NativeContextMenuProps` 与 builder 另保存 enabled = true、handle = None、onClosed = {}；builder 的 key 默认 Position。右键按下、菜单键或 Shift+F10 发起原生展示。已有正文焦点保留；正文没有焦点时使用不进入 Tab 顺序的包装焦点作为后备。后代已消费的键盘默认动作不再重复打开菜单。

`NativePopupHandle()` 可选地关联一个已挂载触发器。`isPending` 可被 render 观察；`requestOpen(): Boolean` 从当前已提交锚点发起一次请求，`cancel()` 取消当前 session。一个 handle 不能同时绑定两个触发器，移除或替换 handle 后旧 handle 不再控制该触发器。没有有效提交区域、触发器不可用／不在当前输入范围、已存在 pending session 或 handle 未挂载时，requestOpen 返回 false；系统拒绝展示仍报告真实 Window 错误。

按钮和键盘请求用 InputArea.visibleBounds 的客户区逻辑矩形，右键使用真实事件的窗口坐标。布局中的未提交位置不用于原生展示。打开前同时检查 Window.state 和 isInputBlocked，避免模态限制已同步生效、输入通知尚未交付时继续打开。正在显示时描述更新复用同一菜单树；禁用或卸载触发器取消 session。onClosed 接收真实 PopupResult，完成后先清除 pending，再执行当前回调；回调可发起下一次明确请求。卸载停止所属回调，关闭 session 后释放菜单，不保留独立的受控 open 值，也没有关闭否决或自绘退出动画。

## 4. 应用默认菜单与窗口覆盖

`nativeApplicationMenuBar(menu, key): ApplicationItem` 在 ApplicationComponent.render 中求值 NativeMenuContent.snapshot，产生应用附属声明；key 必须明确。它不创建 Surface。`NativeApplicationMenuBarProps(snapshot)` 与 `NativeApplicationMenuBar` 提供同一 ApplicationElement 协议的显式入口。

`nativeWindowMenuBar(content, menu, configure = {}): View` 的 `NativeWindowMenuBarBuilder` 只配置 key（默认 Position）；`NativeWindowMenuBarProps(content, menu)` 与对应 Component 在自身 render 中读取菜单 Binding。包装只透传一份正文，不插入布局子项、尺寸或 gap。

两者向所属 scope 登记 UiMenuBarDeclaration，遵守 [UI 桌面规范](SCOOP-UI-DESKTOP-SPEC.md)第 6 节的唯一声明、能力映射、来源错误和原生解绑规则。每个实际附着拥有独立 NativeMenuTree；声明 update 更新已有树，清除声明撤销所属动作。默认菜单的同一描述可以投影到多个窗口，不能共享原生 Menu 实例。实际菜单栏的根条目必须是 Submenu，非法结构和快捷键按 Window 报告错误。

## 5. 标准编辑命令

原生菜单产生的 WindowEvent.TextCommand 经 Surface 的 InputEvent.TextCommand 沿当前焦点路径执行 capture／target／bubble／default；没有合格焦点时不把命令交给后台编辑器。TextEditHost 和 SelectableTextHost 在默认阶段复用各自 execute(TextEditCommand)，preventDefault 可阻止该次默认编辑。

两种文本宿主通过所属 InputScope.textCommandAvailability 发布七个原生编辑角色的状态。文本、选择、只读／禁用、复制策略和撤销历史变化同步更新；SelectableTextHost 只发布 Copy 和 SelectAll，未接受的新文本不使用旧选择能力。Paste 表示目标允许尝试粘贴，真正执行时才读取剪贴板，无文本时无操作。焦点、输入边界和卸载决定哪一份状态进入 Window；原生文件面板的编辑目标由系统处理，不修改后台 UI 文档。

TextEditOptions.protectedText 独立于显示投影。Default 复制策略在受保护用途或内置遮罩时禁用 Copy／Cut，显式 Allow 只开放复制；保护或复制策略热更新同步到菜单，已经选中的迟到动作仍按最新资格执行。Paste、DeleteSelection 和 Undo／Redo 保留各自的普通编辑规则，详细合同见[UI 规范 §7.5.3](SCOOP-UI-SPEC.md#753-编辑显示投影)。

DeleteSelection 仅删除当前非空选区，不等价于向前删除。TextEditHost 只有在启用、可编辑且有选区时发布该能力，实际执行仍使用当前策略；空选区为无操作，SelectableTextHost 不提供删除。该命令可用于原生菜单与语义编辑，不为它合成键盘事件或单独维护编辑历史。

## 6. 文本框内建上下文菜单

`TextEditOptions.contextMenu` 为 TextContextMenuOptions，默认 mode=PreferSystem、selection=Platform、labels=None。TextContextMenuMode 提供 Disabled、StandardNative、PreferSystem；TextContextMenuSelection 提供 Platform、Preserve、PlaceCaret、SelectWord。`textContextMenu { ... }` 和 copy builder 构造该不可变值，labels 使用 Window 的 TextContextMenuLabels。Input／Textarea／InputOtp 及组合入口透传此选项；禁用内建菜单时保留自定义包装的正常输入路由。

SelectableTextOptions 提供相同的 contextMenu 及默认值，SelectableTextHost.showContextMenu() 使用当前 Accepted 段落中的选区或插入位置作为键盘锚点；它不创建可写编辑宿主。菜单只保留 Copy／SelectAll 及二者之间的有效分隔线，复制使用 Accepted StyledText 的 alternateText；copy=false 禁用复制，disabled 不打开菜单。只读选择与可编辑宿主共用原生会话、最新可用性、标签和选区外点击策略。

TextEditHost 的 `showContextMenu(): Boolean` 从当前有效可见 caret／选区锚点请求键盘式菜单；没有可用 Accepted 几何、活动宿主未聚焦／禁用或已有 pending 菜单时返回 false。已经关闭或卸载时与其他宿主方法一样报告 IllegalStateException。右键和平台辅助点击在默认动作阶段进入同一路径，键盘入口为非重复的 ContextMenu 或 Shift+F10。已消费或 preventDefault 的请求不执行默认菜单。布局未提交时不以原点代替锚点；只读取匹配的已接受文本布局进行指针命中。

点击当前选择绘制区域时保持方向选区；选区外按 selection 处理。Preserve 完全保留，PlaceCaret 定位，SelectWord 使用既有 graphics.text 分词，Platform 使用模板的提示。先按普通输入规则取得目标焦点；打开菜单结束当前拖选并取消越界自动滚动，定位与结束 composition 沿现有编辑合同执行。不创建另一份文本模型或历史；菜单展示保留当前文档的 native 文本连接与实际选择。

每次请求绑定发起宿主及当时文档，以 PopupSessionId 匹配来源。菜单使用明确的 Action 项，激活时验证来源、宿主／文档绑定仍有效、实际焦点、输入边界和该命令的最新可用性，然后执行对应宿主的 execute；一个激活只执行一次。PopupCompleted 不再次执行编辑，Selected 与 MenuActivation 的到达顺序均可处理。只读与编辑能力变化更新菜单可用性；禁用、模型替换、菜单配置变化、焦点离开、窗口关闭或宿主卸载及时取消旧请求，失效的晚到动作不能修改新目标。菜单能力更新不请求绘制，Paste 在执行时读取当前剪贴板。

共享会话在更换编辑模型前失效；只读宿主在正文或行内替代文字变化时失效，单纯字体／颜色变化不取消菜单。打开过程中的焦点或选择回调若关闭宿主或替换文档，该次请求停止。两个宿主的菜单动作分别回到自己的 execute，不能落到当前任意焦点控件。辅助点击不启动文本拖选；公共 Press 对平台认可的辅助按下不建立主按钮激活候选，让正常菜单路由继续。

菜单资源在首次使用时创建；没有请求不分配 OS 菜单。关闭先停止动作及观察，再取消 session／关闭 Menu，原生 tracking 与 Window lease 继续使用既有收尾。菜单创建／显示失败走所属根错误边界。没有后台轮询、额外事件循环或新增编译器机制。实际系统模板复用与回退遵守 Window 菜单规范第 6 节，不能把模板存在当作全部系统文本服务已支持。
