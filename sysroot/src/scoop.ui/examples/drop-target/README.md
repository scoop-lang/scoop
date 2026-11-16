# UI 双窗口拖放示例

macOS 的独立自绘应用，使用 ComponentScope.dragSource 启动原生来源，以及 Window → Surface Accepted 命中 → ComponentScope.dropTarget 接收通路。启动时创建同程序两个窗口。复用已准备的开发 sysroot，在仓库根执行：

```sh
scoop run sysroot/src/scoop.ui/examples/drop-target --release
```

左侧区域接收纯文本，右侧接收 URL；背景接收完整文件列表。将文件拖到任一子区域时，子区域拒绝对应格式，沿逻辑父链交给背景。正在接受的区域改变底色，释放后显示实际收到的内容或文件数量。示例只协商 Copy，保留源内容。

左侧拖出已接收的文本，初始内容包含中文、组合字符和 emoji；右侧拖出已接收的 URL，初始值为 https://example.com/。左侧点击、Tab 聚焦后 Enter／Space、Command+V 或 accessibility 的 Press 都读取同一个粘贴入口，结果与文本拖入写入同一份状态。右侧点击、键盘或语义 Press 复制它实际提供的 URL；Command+C 复制当前聚焦区域的数据。源拖动达到阈值后接管点击，释放时不能额外粘贴或复制。焦点环和来源／目标反馈沿各自状态显示。

三个区域有语义名称，接收结果发布为文本节点；原生聚焦复用实际 FocusTarget，不创建第二份业务状态。该示例只提供文本和 URL 拖出，文件接收保留完整路径列表；没有把文件名文本当成文件拖出。

下方多行编辑框使用 TextEditHost 的默认 Copy 接收，可将左右区域的文字／URL 或外部文本拖到具体落点。它继续支持普通输入、选择、粘贴、菜单与撤销，并发布同一编辑宿主的文本语义；区域级 Command+C／V 只在两个来源区域内生效。检查拖动期间原选区保持、临时 caret 随落点和边缘滚动更新，释放后插入一次，Undo 恢复原内容。文件列表不作为编辑文字导入；上方背景与两个彩色区域继续接收文件。

在可交互桌面上分别检查同窗口、两个窗口之间和外部原生文本编辑器／浏览器的双向拖放，另从访达拖入文件。检查拒绝时不高亮、子区域不会穿透兄弟、文件沿父目标接受、退出区域清除反馈、窗口缩放后落点使用新 Accepted 场景、关闭来源或目标后旧回调不再进入组件。右侧 URL 可以拖到左侧文本区，由原生 URL 的文本表示完成同窗口 Copy；可继续拖到第二窗口或外部程序。实际 OS 会话、VoiceOver 和键盘入口分别验收，构建成功不代表这些操作已经通过。

自动逻辑测试见 [drop_routing.scoop](../../tests/drop_routing.scoop)、[drag_source.scoop](../../tests/drag_source.scoop) 和 [text_drop_edit.scoop](../../tests/text_drop_edit.scoop)，几何测试见 [input_hit.scoop](../../../scoop.surface/tests/input_hit.scoop)，来源与真实帧接入见 [drag-source](../../tests-cones/drag-source/README.md)，文本目标接入见 [text-drop](../../tests-cones/text-drop/README.md)。接收逻辑测试中的候选路径是直接提供的；来源准备测试主动取消尚未启动的请求。文本目标测试调用 AppKit 公开 destination 协议，使用真实 Accepted 场景但没有创建系统拖动会话。它们和实际系统 tracking、外部程序、VoiceOver 的结果分别记录；文本拖出、Move 和其他平台继续按[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)实施。
