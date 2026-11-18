# 双窗口交互展示

在已准备当前库的开发 sysroot 上执行 `scoop run sysroot/src/scoop.ui.component/examples/touch-input --release`。示例打开两个窗口，按普通系统策略获得用户输入，不循环请求前台激活。

同一场景包含：

- 滚动条闲置隐藏、hover 宽度／深浅和滚动回弹的配置；系统减少动画仍由框架处理。
- 单／多行及受保护输入、长文、只读文字选择、文本菜单、可选系统替换、PRIMARY 和长按触觉选项。
- 两窗口共用的 200 项虚拟列表，稳定 key 随重排保留；每项提供拖放、Move up／Move down 按钮和上下文菜单，按钮可用键盘或读屏器操作。目标重排与来源完成共用一次业务提交，跨程序 Move 只在来源未变时删除一次。
- 文本／URL／文件接收区；可以从接收区拖出已有内容，也能使用 Paste text／Choose files 的等价入口。接收文件只保留路径，不自行读取文件正文。
- 可捏合、旋转的图形，以及同一操作的 Zoom／Rotate／Reset 按钮；第二窗口文本拖放、模态边界、Tooltip／HoverCard 长按与图表选择。

PRIMARY 只在 GTK 的实际原生选区协议存在时可用；macOS／Windows 不回退到普通剪贴板。自动替换默认关闭，系统 Services／词典按实际平台和已安装提供者提供；受保护／遮蔽正文不导出。键盘用途不替代字符验证，软件键盘显示／遮挡和触觉请求都不被当作设备实际响应的证明。

源码消费者的构建和纯业务测试不替代真实触屏／笔、跨程序文件传输、系统服务或读屏器验收。三平台范围统一见[交互计划](../../../../../docs/ui/interaction-polish/PLAN.md)和[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
