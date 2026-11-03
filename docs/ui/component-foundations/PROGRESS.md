# 控件基础补齐实施记录

日期：2026-11-04。状态：实施中，当前进行 L2。

实现分支为 `codex/ui-component-foundations`。每个大项完成定向 macOS 验证后提交、推送，再把最新 origin/main 合入实现分支；只对后续实际改动或合并冲突影响的部分复验。临时脚本和原始输出在仓库外使用 `/Users/chenxu/repos/scoop-work/ui-component-foundations`，每项结束清理不再需要的文件，保留后续会复用的构建缓存。

设计见 [DESIGN.md](DESIGN.md)，平台合同、布局、交互和文本的细节分别见 [PLATFORM.md](PLATFORM.md)、[LAYOUT.md](LAYOUT.md)、[INTERACTION.md](INTERACTION.md)、[TEXT.md](TEXT.md)。本记录与已完成的 [A–F 基础交互记录](../foundations/PROGRESS.md) 分开，避免把此前验收当作本轮交付。

| 批次 | 状态 |
| --- | --- |
| P1 平台键盘合同 | 已完成 |
| P2 文本服务与源码边界 | 已完成 |
| L1 布局协议与空间分配 | 已完成 |
| L2 方向、Wrap 和 Grid | 实施中 |
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

## P2：文本服务与源码边界

- Clipboard 及 IME 文档／caret 的声明、owner 和参数检查迁入 window 共享源码；Darwin 文件仅提供原生转换和发布。剪贴板读写归一化 CRLF／CR 为 LF，写前拒绝 NUL，读取到内容 NUL 报 Unsupported。Surface 使用同一个 caret 适配，删除忽略连接与 revision 的旧路径。
- graphics.text 为 sans-serif／serif／monospace 提供真实系统 family，集合创建时解析并保存；显式同名字体优先，关闭 fallback 时不加入系统别名，已有 Paragraph 保留自己的字体快照。三套主题已有的偏好列表直接使用该共同 fallback。
- 移除 ui／component／style／三套主题生产源码的 Darwin 条件；window.asyncio 已有的共享宿主与 platformWatch 分层继续复用。PLATFORM.md 已列出后续 Windows／Linux 分支所需入口，本轮没有补写或验收这些后端。
- aarch64 macOS release：window、graphics.text、window.asyncio、surface、ui、style、三套主题及 component 共 10 个生产库构建成功。修改的 Scoop 通过格式化，window／graphics.text 的 tests 类型检查通过，native 源码通过严格警告编译。
- 5 项定向测试全部通过：剪贴板往返／NUL／原生失败／owner，原生文档修订与范围查询，以及三项字体类别／别名覆盖／禁用 fallback。没有复跑 P1 的快捷键、编辑器或模态测试。清理了本项的独立检查锁文件，保留依赖缓存与下一项草稿。

## L1：布局协议与空间分配

- MeasureContext／LazyMeasureContext 允许放置前重新测量，相邻同约束复用最后成功结果；失败清除本次结果，放置后拒绝再测量。每次节点测量清空旧 metrics／placements，不缓存历史尺寸来配另一套子树布局。
- lazy 候选集合在根或 portal 最终测量后统一提交；中间试测不 trim、不切换输入参与状态，也不验证临时省略的 pinned 项。最终只收尾实际测量的子树，沿用已有按需挂载、保留和失败退出规则。
- Row／Column 与公共 Flex 共享 basis／grow／shrink、min／max 冻结分配、交叉轴 stretch／baseline、主轴留白、padding 和显式 clip。已迁移 builder、示例及旧布局测试。无界且未改变 Content basis 时保留自然测量，避免嵌套布局重复施加无分配需求的紧约束。
- aarch64 macOS release：ui、style、三套主题和 component 共 6 个受影响生产库构建成功；Scoop 格式化与 5 个相关 Cone 的 tests 类型检查通过。依赖重建服从现有产物指纹规则，没有扩大到编译器或其他平台验证。
- 15 项定向测试全部通过：Flex 分配／冻结／极大权重／基线／配置 4 项，Frame／Stack／参数 3 项，builder 5 项；真实 lazy、绘制和控件布局组合各 1 项。组合覆盖不同约束与失败恢复、临时省略焦点项、实例与局部 State 保留、80＋8＋212 的实际文本换行、行内控件、裁剪和静止停止提交。无未闭合失败。
- 清理本项独立检查锁文件，保留后续复用的产物、Cone cache 和 L2 草稿；x86_64 仍在 C1 统一验收。
