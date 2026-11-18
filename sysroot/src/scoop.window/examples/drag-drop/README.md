# AppKit 数据拖出与接收

这是 macOS 的低层 Window 示例，两个普通窗口各自提供完整接收和拖出入口，不依赖 UI 场景。复用已准备的开发 sysroot，在仓库根运行：

```sh
scoop run sysroot/src/scoop.window/examples/drag-drop --release
```

将两个窗口移到同时可见的位置。在内容区按住左键即可从该窗口拖出当前数据；初始分别是 Unicode 文本和 URL。窗口标题说明当前内容和结果，目标窗口接受文本、URL 或完整文件列表后替换自身内容，之后也可将该内容拖出。示例只允许 Copy，不删除源文本或文件。

从原生编辑器、浏览器和访达拖入，再拖回对应外部程序，可以检查两个方向。也应分别检查同窗口、同程序两个窗口、外部进程、拒绝、Escape、源窗口失活与关窗。关闭活动源只撤销业务回调；AppKit 没有公开强制取消接口，不能据此声称 OS 已取消或外部目标一定没有接收。

此示例直接从 Down 启动，展示 Window 接口与系统会话；组件的阈值、预览自定义、文本选区拖出／事务和非拖动替代入口由后续 UI source 接入。独立的 [UI 接收示例](../../../scoop.ui/examples/drop-target/README.md) 使用 Accepted 场景和区域路由。

自动检查见 [drag_source.scoop](../../tests/drag_source.scoop)：原生 item writer 使用独立 pasteboard 和真实临时文件，输入测试使用真实 SWView 的事件方法，但只验证启动前取消／关闭；完成状态机通过显式 native 替身测试。它们不是实际 NSDraggingSession 或外部应用验收，运行此示例的系统行为结果须单独记录。
