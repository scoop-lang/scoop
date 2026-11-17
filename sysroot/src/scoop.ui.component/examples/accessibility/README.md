# 无障碍设置示例

使用普通 Input、Checkbox、Slider、Collapsible、Table、Dialog 和 Label 展示默认语义、显式名称、稳定 automationId、数值、展开、表格导航、模态边界和状态公告。文字编辑、键盘操作与辅助技术动作共用实际模型。

在准备好目标开发 sysroot 后运行：

```sh
scoop run --release
```

辅助技术可以通过“Display name”“Receive updates”“Volume”等名称访问控件；自动化客户端可使用 display-name、updates、volume、details、delivery-options、save、status 和 confirmation。保存打开对话框，关闭请求沿 Dialog 原有流程；背景保持自己的身份，在模态期间退出操作范围。

macOS 使用 AppKit，Windows 使用 UIA，Linux GNU 使用嵌入 GTK 的 AT-SPI 内容树。各平台的构建、外部客户端和读屏器验证结果见[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。示例源码不是平台验收记录。
