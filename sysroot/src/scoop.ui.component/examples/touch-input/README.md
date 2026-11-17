# 直接触摸与文本输入

在已准备当前库的开发 sysroot 上执行 `scoop run sysroot/src/scoop.ui.component/examples/touch-input --release`。

示例包含邮件／电话号码用途、受保护输入、可拖放的多行正文、Tooltip／HoverCard 长按入口和图表短按选择。短按提示触发器执行原按钮动作；长按只显示提示，释放后保持。卡片正文仍可操作，外部按下关闭。图表另有普通按钮入口，键盘导航和语义集合保持可用。

文本输入的用途只提示原生键盘布局，不自行拒绝文字。Windows 使用系统实际遮挡显露 caret；GTK 可请求显示但不提供通用遮挡查询，macOS 没有对应通用接口，因此状态显示不可用。程序不把请求成功当作键盘已经显示。

本例可在 macOS、Windows 和 Linux GNU 构建。受控接触测试与编译不替代触屏／笔、系统键盘和跨程序拖放的实际平台验收。
