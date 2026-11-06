# 原生桌面能力实施记录

开始日期：2026-11-07。开发分支：`codex/native-desktop`。目标为完整实施[主设计](DESIGN.md)和[窗口／对话框设计](WINDOWS-AND-DIALOGS.md)，最终交付三平台安装包；本文件只记录实际进度与必要限制。

## 1. 工作约定

- 每个完整大项先在本地实现、格式化、检查并验证，再到适用的其他平台验证；不为每个小改动重复全量测试。
- 每项完成后提交、推送，再 fetch 并合并 origin/main；若合并影响已验证范围，执行对应的必要复验。
- Linux 使用 ssh nuc12 下 ~/repos/ 内的新目录；Windows 使用 /usr/bin/ssh 192.168.2.67 下 E: 内的新目录。Windows 经 SSH 执行构建和可运行测试，GUI 交互由用户随后验证。
- 原始日志、临时探针与安装包保存在仓库外；每轮结束清理已经不再需要的临时文件，保留后续构建需要的正常增量缓存。
- scoop.tray 与 scoop.ui.component.tray 独立交付，普通 window／UI 不引入额外托盘依赖。

## 2. 批次状态

| 批次 | 范围 | 状态 |
| --- | --- | --- |
| 准备 | 设计基线、开发分支、验证与提交约定 | 已建立分支；设计进入版本管理 |
| D1 | 公共合同、跨 Cone 原生使用、平台关键组合 | 菜单身份、图像、能力与 NativeMenuLease 已实施；应用资源登记随 D3 完成 |
| D2 | Menu／Popup、快捷键、原生菜单借用 | 已实施；macOS／Linux 运行验证及 Windows 三目标构建完成，桌面交互限制见下 |
| D3 | 独立 Tray Cone、定位、服务恢复与关闭 | 待实施 |
| D4 | Frame／Region、原生外观与系统操作 | 待实施 |
| D5 | Owned／Tool／Raise／Modal 与关闭依赖 | 待实施 |
| D6 | 原生文件／目录对话框 | 待实施 |
| D7 | UI 应用根、资源事件、Surface 发布边界 | 待实施 |
| D8 | 通用组件及独立托盘组件 | 待实施 |
| D9 | 综合 desktop demo | 待实施 |
| 发行 | macOS、Linux、Windows 安装包及脚本修复 | 待实施 |

## 3. 验证记录

设计基线已检查本地文档链接、章节、D1～D9 依赖顺序和格式。开发分支已在更新后的 main（2bd06a47，包含 Linux GUI 与 Windows TSF 合入）上重建。

2026-11-07，D2：已加入[菜单规范](../specs/SCOOP-WINDOW-MENUS-SPEC.md)与三个原生后端，包含稳定身份、CPU 图像、标准角色、实际快捷键、菜单栏、popup session、tracking 中更新及跨 Cone 菜单借用。关闭立即停止原生动作，解绑和销毁等待原生使用退休。原生 popup 从应用控制回调启动，保证 watch 先返回并重新挂接通知源。

- macOS aarch64：Scoop 格式化、原生严格编译检查和窗口库构建通过；菜单 Cone 2 passed、1 ignored；asyncio 的真实 popup tracking／I/O／关闭组合 1 passed。忽略的是独立编辑焦点用例：当前桌面处于 loginwindow，须在解锁的前台桌面单独运行。
- Linux GNU：窗口库和测试构建通过；X11 与带输入 seat 的嵌套 Wayland 各 2 passed。覆盖实际菜单栏展开、tracking 中更新、GTK 快捷键只派发一次、原生模型／借用、popup 选择和目标窗口关闭。
- Windows：MSVC UCRT、LLVM-MinGW MSVCRT、LLVM-MinGW UCRT 的窗口库和完整测试程序均编译通过；各目标默认测试 1 passed、1 ignored。SSH 会话无法创建测试所需的原生 Application，原生窗口／菜单用例保留到桌面会话执行；这些结果不代表 GUI 交互验收通过。

测试入口、所需桌面条件及独立焦点／Windows 用例的执行方式见[菜单测试说明](../../sysroot/src/scoop.window/tests-cones/menus/README.md)。macOS amd64 构建与三平台完整桌面组合继续随后续批次和最终打包验收推进。
