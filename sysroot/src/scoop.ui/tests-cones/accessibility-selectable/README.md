# UI 只读富文本无障碍

使用配套工具链和当前 Window／Surface／UI 产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility-selectable --release
```

用真实 SelectableTextHost 与同挂载的 Text 节点验证 updateSelectableText。宿主正常 update 与选择通知刷新同一个语义 handle，不增加可写文档；测试辅助只调用公开 NSAccessibility 协议。

覆盖 UTF-8／UTF-16、emoji／组合字符、行内对象的替代文字、空替代文字、部分替代文字扩展到完整对象、caret 下游映射、方向选区和实际选择通知。只读全文／选区写入被拒绝；禁用和受保护状态、替代文字变化、单纯样式替换、菜单能力开关、无 drawable 发布以及关闭后旧对象分别验证。内联对象使用真实普通 Element 参与排版；逻辑 retained range 与行内子控件语义继续按统一计划实现。

本 Cone 不弹出菜单或依赖窗口焦点；从原生 ShowContextMenu 实际打开编辑／只读菜单的场景在[文本编辑实窗](../editing/README.md)。进程内原生方法、外部 AX 和 VoiceOver 的结果分别记录在[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。

已接受帧的原生范围查询还覆盖 alternateText 内的 Unicode 字符、整对象矩形、按点返回完整替代文字、空替代文字不制造字符，以及替代文字内部 caret 的下游映射。受保护发布和宿主关闭撤下原生文本几何；查询保持原键盘焦点。
