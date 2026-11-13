# Scoop Markdown 编辑器规范

日期：2026-11-14。完整范围见[编辑器设计](../ui/markdown/DESIGN.md)，实施进度见[记录](../markdown/PROGRESS.md)。本规范随实际能力逐项补充；文档模型遵守[Markdown 文档规范](SCOOP-MARKDOWN-SPEC.md)。

## 1. 分层

`scoop:scoop.ui.markdown` 是纯 Scoop library Cone。它依赖独立的 `scoop.markdown` 和现有 UI／窗口／文本能力；文档 Cone 不反向依赖 UI。源码、富文本和双栏共享一个 MarkdownDocument、一套文档选区和历史。不同视图的布局与滚动独立，源内容只有同一条事务写入路径。

## 2. 文档编辑状态

`MarkdownEditorState(document, selection)` 借用文档，通过普通内容订阅跟踪修改；状态绑定创建线程，UI 渲染／测量／scene／draw 期间不得修改。关闭状态释放订阅与历史，不关闭或重置借用的文档。关闭幂等，关闭后编辑接口报告状态错误。

`MarkdownSelection` 保存两个源码 UTF-8 位置及各自的 Before／After 偏向，保留活动端方向。选区端点必须是当前文档中的标量边界；不把 Long 字节位置当作字符索引。范围与插入位置沿文档差量映射。源码输入可直接 replace／replaceSelection；批量替换使用 apply，结构修改使用 transform，均调用文档的已有事务。

状态以公开 State 读取追踪发布当前 snapshot、selection、canUndo、canRedo 与保存 revision。状态 API 先检查 UI 的修改时机，再准备与发布文档修改，不能先改文档、后因 State 禁止写入而留下部分 UI 状态。外部文档修改也必须发生在相同 owner 的正常编辑时机。

## 3. 统一历史

一次源事务保存正反替换及前后选区，普通程序化修改和结构命令形成单独历史项。输入种类 `Typing`／`Backspace`／`Delete` 仅在连续位置、折叠选区和相同种类时合并；选区移动、显式 breakUndoGroup、格式命令和 undo／redo 结束合并。合并只收集实际插入／删除的片段，不按每次按键重新拼接整个历史字符串。

undo／redo 各通过一次源码事务恢复整个历史项，并恢复相应方向与偏向的选区。新编辑清除重做分支。Reset 表示整体重载，清空历史并将选区端点收敛到新文档中的合法位置；普通外部 Edit 形成可撤销的独立项。

`markSaved(revision)` 记录宿主已成功写出的本文件 revision；较旧的保存完成不把后续输入标成已保存。`isDirty` 按当前文档 revision 与保存 revision 比较。保存的文件与异步 I/O 由宿主管理，状态不隐式写磁盘。IME 的接受／取消、输入片段和视图模式在接入窗口输入时补充具体合同。
