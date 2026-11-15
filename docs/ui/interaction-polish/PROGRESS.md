# UI 交互打磨与 accessibility 实施记录

更新：2026-11-16。工作分支：`codex/ui-interaction-accessibility`。范围与完成条件以 [PLAN.md](PLAN.md) 为准。

## 执行约定

按本轮要求先在 macOS 完成本地功能与验证，再集中进行 Linux／Windows 验证；不逐个小改动远程重跑。每个可交付变更提交并推送后，获取并合并 `origin/main`。意外发现的编译器／底层依赖缺口使用独立修复分支、PR 与主线合并，更新工具链后回到本分支。

下载使用本轮指定代理，本地依赖复用 `~/repos/scoop-dep`。Linux 后续使用 `ssh nuc12`、`~/repos/` 下独立目录及 `~/.scoop`；Windows 后续使用 `/usr/bin/ssh 192.168.2.67`、E: 独立目录及 `~/.scoop`。原始日志留仓库外，必要缓存保留，临时核查文件在不再需要时删除。

## 当前状态

| 批次 | 状态 | 已完成与剩余 |
| --- | --- | --- |
| S0 | 进行中 | 已核对本地工具链、AppKit 偏好与通知、现有测试入口；原生文本菜单、拖放、accessibility 原型与其他平台验证待继续 |
| S1 | 进行中 | Window 偏好快照／观察 API 与三个 native 后端已实现，macOS 定向测试通过；UI 根覆盖／通知与 caret 消费已接入；主题应用、点击计数和输入细节待继续 |
| S2～S11 | 待开始 | 保持统一计划的完整范围，未将基础 API 交付视为上层交互或 provider 已完成 |

## 已交付变更

### 系统交互偏好的 Window 接口

- `InteractionPreferences` 区分未知值与明确关闭，覆盖滚动条、运动、对比度、透明度、文字缩放、caret 和双击间隔；保留物理宽度、宽高比和时间单位的差异。
- ActiveApplication／Window 读取 owner 已发布的快照，观察者经普通队列通知；先发布后通知，去重，回调内登记／注销、窗口关闭与应用退出按实际生命周期处理。
- AppKit 使用公开查询与通知；GTK 使用 4.18 基线设置和 libadwaita；Windows 使用 Win32 设置与按接口查询的 UISettings 2／4／5。Windows／Linux 的代码尚待目标环境构建与运行，不据 macOS 结果声明通过。
- 同步 Window spec、公开注释、库 README 与编程材料导航。定向 Cone 的原生通知用当前进程内替身驱动，不修改全局系统设置，也不冒充人工设置热切换验证。

本地验证使用 `/Volumes/Data/home/chenxu/repos/scoop-install` 中配套的 `scoop` 与 sysroot（工具 revision `6bd92d3d7a14e4614320008b33586df8da178d85`），测试显式通过 path 依赖编译当前 Window 源码。仓库原有开发 sysroot 与该工具的 runtime ABI 不同，没有混用其预编译库；当前主线相对该工具未发生语言／runtime 合同修改。改动源码先通过 Scoop fmt／check 与 native 格式检查。

`window-preferences` 的两个测试已通过：不可用值／显式关闭的区别，以及 AppKit 通知、快照发布、去重、观察者增删、owner 检查和关闭。Window 生产／测试语义检查和既有 `window-deadlines` 生命周期回归也通过。真实系统设置的人工热切换、设备输入及其他平台仍按 S0／S11 的矩阵验收。

### UI 根继承与文本光标

- `UiPreferences`／receiver builder、UiHost／单窗入口、UiWindow 动态覆盖和 ComponentScope 快照／观察已接入；None 跟随系统，显式 false／零保留原意。不同根独立解析，已覆盖的系统变化不请求帧；通知支持重入、取消和根错误收尾。
- render／update 的显示依赖及 measure／scene／draw 的版本记录沿已有缓存失效路径处理；偏好读取不增加持续帧。组件 update 仍只登记明确的默认字体和交互偏好，不扩展为任意 State 跟踪。
- TextEditHost 默认继承闪烁和宽度，Input／Textarea／OTP 及其组合入口、示例与测试同步改为可选覆盖。系统物理宽度按窗口 scale 换算，宽高比使用排版光标高度；局部逻辑宽度优先。每份测量与 Accepted 帧保存采用的光标样式，旧帧查询不会读到下一帧的宽度。
- 根关闭撤销应用级偏好观察；文本宿主还观察实际 DPI 变化。偏好更新保留编辑模型、选择和历史，闪烁继续使用既有 WindowDeadline。

开发资源使用配套安装与仓库外 `scoop-ui-polish-work/cones` 的普通 `--cone-path`：定向构建 Window、Surface、UI、Style 与四套主题，复用其余安装产物和共享 cache；没有复制整套 sysroot 或修改安装目录。主题在此批次仅按 UI 接口依赖重建，减少动画／高对比／减少透明度／文字缩放的主题消费仍待下一批实现。

本批 macOS 验证：UI 生产／测试和 Component 测试代码的语义检查通过；46 个变更的 Scoop 文件格式检查通过。UiPreferences 两个值测试通过；独立 UI 偏好窗口测试通过，覆盖四种显示读取路径、实际 AppKit 通知、两根覆盖隔离、重入／注销、物理宽度与比例宽度、Accepted 查询、模型／选择／历史保留、闪烁启停／停帧和回调失败退休。既有 `ui-text-editing`（含真实系统 IME、投影、选择及滚动）与 `ui-layout-results`（布局 A→B→A、保留测量和 lazy 选择）定向回归通过。没有运行无关的全仓测试或远程矩阵；本批仍未声明完整 accessibility provider 或主题适配完成。

已定向生成 UI／Component API 文档，并用 `scoop api search/show` 核对 UiPreferences、UiHost.attach、UiWindow.preferences 的访问器，以及 caretBlink／caretWidth 的可选类型、默认值与导入路径。生成站点在核对后清理，保留后续构建需要的 Cone 产物与 cache。
