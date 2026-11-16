# UI 拖放接收示例

macOS 的独立自绘应用，使用真实 Window → Surface Accepted 命中 → ComponentScope.dropTarget 接收通路。复用已准备的开发 sysroot，在仓库根执行：

```sh
scoop run sysroot/src/scoop.ui/examples/drop-target --release
```

左侧区域接收纯文本，右侧接收 URL；背景接收完整文件列表。将文件拖到任一子区域时，子区域拒绝对应格式，沿逻辑父链交给背景。正在接受的区域改变底色，释放后显示实际收到的内容或文件数量。示例只协商 Copy，保留源内容。

左侧同时提供普通粘贴：点击、Tab 聚焦后 Enter／Space、Command+V 或 accessibility 的 Press 都读取同一个剪贴板入口，结果与文本拖入写入同一份状态。三个区域有语义名称，接收结果发布为文本节点；原生聚焦复用实际 FocusTarget，不创建第二份业务状态。

在解锁的桌面上分别从原生文本编辑器、浏览器和访达拖入数据，并检查：拒绝时不高亮、子区域不会穿透兄弟、文件沿父目标接受、退出区域清除反馈、窗口缩放后落点使用新 Accepted 场景、关闭窗口后旧回调不再进入组件。使用两个独立进程运行此示例可检查接收窗口各自的资源；它不是拖出源，不能据此宣称双向或同程序跨窗口拖放已经验收。

自动逻辑测试见 [drop_routing.scoop](../../tests/drop_routing.scoop)，几何测试见 [input_hit.scoop](../../../scoop.surface/tests/input_hit.scoop)。逻辑测试中的候选路径是直接提供的；真实离屏几何测试、编译成功、实际窗口命中及外部系统会话的结果须分别记录。完整 source、文本编辑事务和其他平台继续按[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)实施。
