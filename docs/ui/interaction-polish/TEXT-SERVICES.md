# 系统文本服务与平台惯例

本记录区分公共接口可接入的能力与实际设备／提供者验收。规范见 [Window](../../specs/SCOOP-WINDOW-SPEC.md#222-系统文本服务)、[UI](../../specs/SCOOP-UI-SPEC.md) 和[原生文本菜单](../../specs/SCOOP-UI-NATIVE-MENUS-SPEC.md#6-文本框内建上下文菜单)。本批本地验证结果汇总到 [PROGRESS.md](PROGRESS.md)，未据源码宣称 Windows／Linux 或外部提供者已经通过。

| 能力 | macOS | Windows | Linux GNU |
| --- | --- | --- | --- |
| 标准编辑菜单 | 从 NSTextView.defaultMenu 提取可执行标准模板；同一 Scoop 文档 | 标准原生 popup 与文本命令 | GTK 原生 popup 与文本命令 |
| 拼写、语法与建议 | NSSpellChecker 独立检查；语法按细节范围转换 | ISpellChecker；语言须实际安装，无通用语法提供者 | GTK 不附带文本检查器，明确不可用；应用可接入自己的 policy／范围装饰 |
| 可选自动替换 | 系统允许的拼写更正、引号、破折号、文本替换 | 原生更正／删除建议 | 无统一系统入口 |
| 词典 | NSView 的公开 definition 接口，使用真实可见锚点 | 无统一普通窗口入口 | 无统一 GTK 入口 |
| Services／翻译 | 已注册文本服务，经 NSPerformService 和独立临时 pasteboard；翻译取决于安装的提供者 | 无相同统一服务入口 | 无相同统一服务入口 |
| 听写 | 经既有 NSTextInputClient 连接提交，不伪造启动命令 | 经系统文本输入服务提交 | 由已配置输入服务提交 |
| PRIMARY／中键粘贴 | 不可用 | 不可用 | 原生 GdkClipboard PRIMARY；尊重 gtk-enable-primary-paste，区分 Wayland 无协议的本地占位 |
| 触觉 | NSHapticFeedbackManager；只能确认请求提交，不能据此判断设备存在 | 没有普通窗口的统一入口 | 没有 GTK 的统一入口 |

TextContextMenuOptions.systemServices 默认启用，仅用于 PreferSystem；标准原生模式保持标准编辑项。所有建议、词典和服务导出都遵守复制／受保护规则，替换绑定原文档与修订；Service 返回文本沿 Paste 策略，拼写建议沿普通编辑策略，单独选择不进入正文过滤或撤销。自动替换默认关闭，只处理输入完成且遇到词边界的原生提交，先合并系统替换再进入应用 policy，不将正在 composing 的正文送去更正。

当前使用 NSSpellChecker 的只读结果与 NSView 的词典入口，没有实现完整 NSTextCheckingClient 的注释、候选气泡、全篇原生字符几何和排版操作。NSTextCheckingController 要求这些真实能力，不能拿当前只有 caret 的原生输入几何冒充完整正文几何。因此不展示未接入的连续拼写面板／排版动作，也不创建隐藏 NSTextView、另一份可写文档或撤销模型；应用仍可用现有范围装饰显示自己的检查结果。

GTK 4.18.6 的公开 `gdk_display_get_primary_clipboard` 在 Wayland primary-selection 协议缺失时返回普通 GdkClipboard，仅能进程内保存内容。实现通过公开 GObject 类型身份识别这个占位，不调用 GTK 私有协议、不把普通 Clipboard 当作回退。`PrimarySelectionClaim.close()` 仅清自己仍拥有的内容；读取前后的 UI 目标核对以及“先读旧 PRIMARY，再改变选择”属于宿主的粘贴事务。

2026-11-18 的 X11 跨进程检查已通过：公开 Window API 与 `xclip` 双向传输中文／emoji 和换行，外部拥有者接替后旧 claim 失效且关闭不影响新内容，重新认领及释放正常，普通 Clipboard 保持独立。当前用户 GNOME `wayland-0`／GTK 4.22.4 会话的只读查询报告 PRIMARY 可用，实际中键与跨程序收发待人工确认；此前私有 Weston 的无协议结果仅适用于该测试环境。

核查依据为本机 SDK 的 NSSpellChecker.h、NSTextCheckingController.h、NSTextCheckingClient.h、NSView.h、NSApplication.h、NSHapticFeedback.h，Windows 的公开 ISpellChecker／ISpellCheckerFactory 接口，以及 GTK [4.18.6 gdkseat-wayland.c](https://gitlab.gnome.org/GNOME/gtk/-/blob/4.18.6/gdk/wayland/gdkseat-wayland.c) 与 [gdkdisplay.c](https://gitlab.gnome.org/GNOME/gtk/-/blob/4.18.6/gdk/gdkdisplay.c)。原始下载、探针和日志放在仓库外。
