# 原生桌面能力 Demo

同一个普通 Cone 演示 SVG／Lucide、字体选择与变体、Taskbar／Dock、系统通知、原生菜单栏／popup、独立托盘组件、邻近窗口定位、窗框与形状、owned 工具窗口、Owner／Application 模态及系统文件面板。使用公共 API，显式依赖对应桌面和托盘适配 Cone。

## 构建与运行

先按[仓库脚本说明](../../../../../scripts/README.md)准备当前目标的系统库，再构建：

```sh
scoop build sysroot/src/scoop.ui.component/examples/native-desktop --release --sysroot sysroot -o /tmp/scoop-native-desktop
/tmp/scoop-native-desktop
```

Windows 安装包的主驱动为 `sco.exe`，使用对应 `--target` 及 `.exe` 输出名；Linux 需要 GTK4／libadwaita 与 libdbusmenu-glib 0.4。托盘还需要支持 StatusNotifierItem 的桌面宿主。普通窗口和组件库仍不依赖托盘。

`--help` 不创建窗口。`--smoke` 在真实显示会话中执行一组有界流程并退出：两窗口、SVG 主题和桌面状态更新、通知设置查询、菜单更新与 popup 取消、窗框切换、工具／异形窗口、两种模态范围、文件面板取消、无窗口更新及再次打开窗口。它不发送通知、不请求授权、不发布持久 Jump List。该流程不代替真实点击、焦点、拖动、Snap、文件选择及多显示器视觉验收。Windows SSH 只编译 GUI；在登录桌面运行可执行文件及 `--smoke`。

## 应用身份与打包

附带的 Bundle.toml 使用稳定标识 `dev.scoop.examples.native-desktop`、显示名称 “Scoop Native Desktop” 和由同一 Lucide folder SVG 经 Skia 栅格化的图标，包含 Lucide 许可。以 Apple Silicon 的现有可执行文件为例：

```sh
scoop bundle --executable /tmp/scoop-native-desktop --target aarch64-apple-darwin --config sysroot/src/scoop.ui.component/examples/native-desktop/Bundle.toml
```

其他平台选实际完整 target。macOS 从生成的 `.app` 启动，Windows 安装 NSIS／MSI 后启动，Linux 安装 deb 并在桌面会话运行。Windows 的通知快捷方式与 COM 注册由安装器处理，示例不修改系统注册；MSVC 版本使用系统 VC runtime。裸可执行文件仍可展示普通 UI，未注册的通知入口报告 NotConfigured。

Linux desktop action 与 Windows Jump List 的 “New document window” 启动一个新示例进程；用于验收的参数包含空格、Unicode、百分号和引号，实际 argv 仅在 `--shell-action` 启动时追加到系统临时目录的 `scoop-native-desktop-launch.log`。普通启动不写这份日志。Jump List 是持久配置，取消 “Publish taskbar launch tasks” 会显式发布空任务列表；退出示例不代替这一操作。

## 体验顺序

1. 编辑文本，选择原生 Edit 菜单的 Cut／Copy／Paste／Undo；切换只读。popup 与托盘共享 checkbox、radio 和图标颜色，菜单项保持身份。试用右键、菜单键、Shift+F10 与按钮的方向键。
2. 打开另一文档并启用窗口菜单覆盖，切换焦点，观察 macOS 应用菜单和 Windows／Linux 每窗口菜单的区别。
3. 切换 System／Extended／Client。拖动自绘标题栏空白处，操作其中的输入与按钮；试用双击最大化、系统窗口菜单和缩放握柄。Windows 另展示 DWM 属性与系统按钮 Snap，macOS 展示 AppKit 标题栏选项。

   Windows 使用 Extended 前勾选 Hide menu bar，以空菜单覆盖当前文档的原生菜单栏；恢复菜单栏时，此文档先切回 Client。Win32 当前宿主明确拒绝非空 HMENU 与 Extended 的冲突组合。弹出菜单、编辑器右键菜单与托盘菜单仍可使用。示例背景填满客户区，同时按实际 frameMetrics 留出系统按钮的透明区域，避免遮住 DWM 绘制的按钮或在按钮下方留下黑块。
4. 打开 owned 工具窗口，验证它不阻塞父窗口；最小化／恢复父窗口并检查相对叠放。分别试用一次 raise 和持续 topmost，能力不支持时相应按钮禁用。
5. 打开椭圆／圆角窗口：默认勾选匹配原生区域，使用正方形内容且不启用示例未提供的边缘缩放。取消勾选后再打开，可对比仅视觉裁剪与真实原生形状；后端没有区域能力时先选择仅视觉模式。可从文档中切换该形状窗口的整窗鼠标穿透，再从同一位置恢复。每窗口菜单平台用独立空菜单覆盖应用默认菜单，让原生菜单条不占据形状窗口的顶部。`--smoke` 使用同一默认配置，不再暗中打开区域选项。
6. 打开 Owner 确认，另一文档应可交互；打开 Application 确认，两文档均受阻。确认内可继续打开嵌套 owner 模态和原生文件面板。关闭文档及退出都经过确认。
7. 使用打开单文件／多文件、保存位置及目录面板，查看真实结果。保存只选择路径，不写文件；原始结果仍为 `Path`，界面仅用 `display()` 展示。
8. 查询托盘矩形、显示器工作区、系统栏信息，尝试邻近面板及屏幕边缘翻转／滑动。提示点、完整屏幕替代工作区和无锚点窗口分别需要显式勾选。Wayland 不支持普通窗口全局定位时会显示原因。
9. 托盘宿主可用时关闭所有业务窗口，在托盘菜单中更新状态、打开原生文件面板或重新创建文档，最后选择 Quit 完成有序清理。注册成功不代表图标可见，需检查桌面宿主和 overflow 区。
10. 在 Fonts and variants 中搜索系统字体并选择实际字面；打开 TTF／OTF／TTC／OTC 可浏览文件中的家族和字面。可变字体显示命名实例和轴滑杆，默认隐藏标为 hidden 的轴，可显式展开。预览仅使用所选字面，缺字保留缺字显示；调整使用同一预览别名和编辑模型，不影响其他文档。重新点击 System fonts 才刷新系统目录，不持续扫描。
11. SVG icons 展示同一组 Lucide 在不同尺寸、主题与固定线宽下的效果。蓝／橙 folder 图像同时供原生应用、窗口和托盘使用，多像素尺寸预先生成并复用。
12. Taskbar, Dock and notifications 只展示当前 OS 的能力。macOS 可修改 Dock 文字角标、菜单和最小化 caption；Windows 可设置指定文档窗口的图标、overlay、进度和任务栏预览 tooltip。正常窗口标题仍由原窗口控制区编辑，安装后的显示名称来自 Bundle.toml。
13. 通知先查询系统设置，权限按钮只在系统要求授权时显示。允许后显式发送，再次发送替换同一 ID；支持撤回的平台显示撤回按钮。默认点击或 Open document 动作打开／聚焦文档。界面只把成功提交描述为系统接受请求，专注模式与实际显示由 OS 决定。

默认关闭最后一个文档退出应用；只有明确选择“关闭全部窗口，保留托盘”后，应用才继续无窗口运行。界面能力值是当前后端的实际声明，真实请求和查询仍可能因为窗口状态、桌面服务或暂时没有位置而失败。
