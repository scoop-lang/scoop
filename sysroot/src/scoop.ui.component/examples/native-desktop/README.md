# 原生桌面能力 Demo

同一个普通 Cone 演示原生菜单栏／popup、独立托盘组件、邻近窗口定位、窗框与形状、owned 工具窗口、Owner／Application 模态及系统文件面板。使用公共 API；显式依赖 `scoop.tray` 与 `scoop.ui.component.tray`。

## 构建与运行

先按[仓库脚本说明](../../../../../scripts/README.md)准备当前目标的系统库，再构建：

```sh
scoop build sysroot/src/scoop.ui.component/examples/native-desktop --release --sysroot sysroot -o /tmp/scoop-native-desktop
/tmp/scoop-native-desktop
```

Windows 安装包的主驱动为 `sco.exe`，使用对应 `--target` 及 `.exe` 输出名；Linux 需要 GTK4／libadwaita 与 libdbusmenu-glib 0.4。托盘还需要支持 StatusNotifierItem 的桌面宿主。普通窗口和组件库仍不依赖托盘。

`--help` 不创建窗口。`--smoke` 在真实显示会话中执行一组有界流程并退出：两窗口、菜单更新与 popup 取消、窗框切换、工具／异形窗口、两种模态范围、文件面板取消、无窗口更新及再次打开窗口。该流程不代替真实点击、焦点、拖动、Snap、文件选择及多显示器视觉验收。Windows SSH 只编译 GUI；在登录桌面运行可执行文件及 `--smoke`。

## 体验顺序

1. 编辑文本，选择原生 Edit 菜单的 Cut／Copy／Paste／Undo；切换只读。popup 与托盘共享 checkbox、radio 和图标颜色，菜单项保持身份。试用右键、菜单键、Shift+F10 与按钮的方向键。
2. 打开另一文档并启用窗口菜单覆盖，切换焦点，观察 macOS 应用菜单和 Windows／Linux 每窗口菜单的区别。
3. 切换 System／Extended／Client。拖动自绘标题栏空白处，操作其中的输入与按钮；试用双击最大化、系统窗口菜单和缩放握柄。Windows 另展示 DWM 属性与系统按钮 Snap，macOS 展示 AppKit 标题栏选项。

   Windows 使用 Extended 前勾选 Hide menu bar，以空菜单覆盖当前文档的原生菜单栏；恢复菜单栏时，此文档先切回 Client。Win32 当前宿主明确拒绝非空 HMENU 与 Extended 的冲突组合。弹出菜单、编辑器右键菜单与托盘菜单仍可使用。示例背景填满客户区，同时按实际 frameMetrics 留出系统按钮的透明区域，避免遮住 DWM 绘制的按钮或在按钮下方留下黑块。
4. 打开 owned 工具窗口，验证它不阻塞父窗口；最小化／恢复父窗口并检查相对叠放。分别试用一次 raise 和持续 topmost，能力不支持时相应按钮禁用。
5. 打开椭圆／圆角窗口，分别选择仅视觉形状与要求匹配原生区域。可从文档中切换该形状窗口的整窗鼠标穿透，再从同一位置恢复。每窗口菜单平台用独立空菜单覆盖应用默认菜单，让原生菜单条不占据形状窗口的顶部。
6. 打开 Owner 确认，另一文档应可交互；打开 Application 确认，两文档均受阻。确认内可继续打开嵌套 owner 模态和原生文件面板。关闭文档及退出都经过确认。
7. 使用打开单文件／多文件、保存位置及目录面板，查看真实结果。保存只选择路径，不写文件；原始结果仍为 `Path`，界面仅用 `display()` 展示。
8. 查询托盘矩形、显示器工作区、系统栏信息，尝试邻近面板及屏幕边缘翻转／滑动。提示点、完整屏幕替代工作区和无锚点窗口分别需要显式勾选。Wayland 不支持普通窗口全局定位时会显示原因。
9. 托盘宿主可用时关闭所有业务窗口，在托盘菜单中更新状态、打开原生文件面板或重新创建文档，最后选择 Quit 完成有序清理。注册成功不代表图标可见，需检查桌面宿主和 overflow 区。

默认关闭最后一个文档退出应用；只有明确选择“关闭全部窗口，保留托盘”后，应用才继续无窗口运行。界面能力值是当前后端的实际声明，真实请求和查询仍可能因为窗口状态、桌面服务或暂时没有位置而失败。
