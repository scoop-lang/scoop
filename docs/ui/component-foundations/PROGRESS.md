# 控件基础补齐实施记录

日期：2026-11-04。状态：设计完成，实施未开始。

设计见 [DESIGN.md](DESIGN.md)，平台合同、布局、交互和文本的细节分别见 [PLATFORM.md](PLATFORM.md)、[LAYOUT.md](LAYOUT.md)、[INTERACTION.md](INTERACTION.md)、[TEXT.md](TEXT.md)。本记录与已完成的 [A–F 基础交互记录](../foundations/PROGRESS.md) 分开，避免把此前验收当作本轮交付。

| 批次 | 状态 |
| --- | --- |
| P1 平台键盘合同 | 待实施 |
| P2 文本服务与源码边界 | 待实施 |
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
