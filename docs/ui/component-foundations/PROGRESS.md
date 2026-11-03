# 控件基础补齐实施记录

日期：2026-11-04。状态：实施中，当前进行 P2。

实现分支为 `codex/ui-component-foundations`。每个大项完成定向 macOS 验证后提交、推送，再把最新 origin/main 合入实现分支；只对后续实际改动或合并冲突影响的部分复验。临时脚本和原始输出在仓库外使用 `/Users/chenxu/repos/scoop-work/ui-component-foundations`，每项结束清理不再需要的文件，保留后续会复用的构建缓存。

设计见 [DESIGN.md](DESIGN.md)，平台合同、布局、交互和文本的细节分别见 [PLATFORM.md](PLATFORM.md)、[LAYOUT.md](LAYOUT.md)、[INTERACTION.md](INTERACTION.md)、[TEXT.md](TEXT.md)。本记录与已完成的 [A–F 基础交互记录](../foundations/PROGRESS.md) 分开，避免把此前验收当作本轮交付。

| 批次 | 状态 |
| --- | --- |
| P1 平台键盘合同 | 已完成 |
| P2 文本服务与源码边界 | 实施中 |
| L1 布局协议与空间分配 | 待实施 |
| L2 方向、Wrap 和 Grid | 待实施 |
| I1 公共激活与拖动 | 待实施 |
| I2 弹层、候选项和悬浮 | 待实施 |
| T1 编辑事务策略 | 待实施 |
| T2 文本投影与选择 | 待实施 |
| V1 装饰与图像 | 待实施 |
| S1 阅读位置与滚动意图 | 待实施 |
| C1 组合验收 | 待实施 |

本轮实现先验证 macOS aarch64，最后补做受影响生产库与代表组合的 x86_64 Rosetta 验证。Windows native 实现、Windows 构建／桌面验收、accessibility、原生控件交互和 shadcn 控件库不在本轮范围。

## 设计阶段

- 核对现有 Window／Surface／UI／Component／Style 的规范、实现和 A–F 验收记录，确定现有能力与真正缺口。
- 写明 Primary／Super、编辑命令、剪贴板／IME 和字体 fallback 的分层；约定上层无 OS 分支，Windows 后端按同一公共合同接入。
- 写明重新测量与 lazy 最终收尾、受控 portal 与退出动画、编辑策略与 composition／位置映射的约束，给出逐批迁移和定向测试范围。
- 本阶段只有设计与文档导航改动，没有新增 API、native 实现或控件，也未执行生产库构建和运行测试。

后续每批只记录实际修改、验证范围、结果、剩余失败和必要限制；原始输出存放仓库外，不把规划用例记为已通过。

## P1：平台键盘合同

- window 提供 ShortcutChord、Primary 解析、精确匹配、等价比较和当前键盘布局标签；Surface 的绑定、scope 和 host 派发迁入 shared。Modifiers 保留真实 Super，并增加可选 AltGr 状态；macOS Fn 只表示实际按住的修饰键。
- AppKit 标准编辑键位移入 window，UI 只执行已交付的语义命令，Enter／PrimaryEnter 提交使用同一 chord。已迁移现有绑定、示例与组合测试，不保留旧 Binding 配置或 UI 后备键位表。
- aarch64 macOS release：window／surface／ui 生产库构建成功；Scoop 格式化与三个 Cone 的 tests 类型检查通过，修改的 native 源码通过严格编译警告检查。
- 定向运行 22 项全部通过：window 的 Unicode、两种 Primary 解析、标签所有权、真实 AppKit 编辑与 Fn 共 7 项；Surface 的 DSL／作用域／宿主路由共 13 项；UI 编辑器与模态组合各 1 项。无未闭合失败。Windows 只覆盖共享解析算法，不代表其后端验收；x86_64 留到 C1。
- 已清理这一项不再使用的独立类型检查锁文件；保留后续会复用的生产库、测试构建目录和共享 Cone cache。
