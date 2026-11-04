# shadcn 控件实施记录

分支：`codex/shadcn-components`。按 [设计](DESIGN.md) 的 P0–P9 顺序推进；本轮只在 macOS 验证，最终补 x86_64 Rosetta。每个独立变更验证后提交、推送并合并 `origin/main`。尚未完成的目录项不计为交付。

## P0a：完整配色与主题

- ThemePalette 直接迁移为完整的语义颜色对、独立 input／ring、五色 ChartPalette 和 SidebarPalette。三个已有主题及控件、示例、测试同步迁移，没有旧字段别名或运行时缺省补色。
- 新增独立 `scoop.ui.style.shadcn`，复用公共 ShapeTheme／ThemePainter；实现 Base Nova／Neutral 的 Light／Dark 配色、14／20 文字、32 高操作表面、16 标记和 3 宽焦点环。源 OKLCH、sRGB 转换结果及 MIT 许可证随源码保留。
- ThemeMotion 的显式 Reduced 模式在局部 transition 解析后生效；Button／Checkbox 消费同一策略。此项不包含 OS 动态偏好接入。
- macOS aarch64 release：17 个所需生产库构建通过；主题测试、控件库和现有主题示例的 tests 类型检查通过。主题的 21 项定向测试全部通过，覆盖四套配置、原主题几何／Raster／Metal／资源回归以及 shadcn token、几何／像素和减少动态。25 个修改／新增 Scoop 文件格式检查及 diff 检查通过。
- 本地预编译 UI、Skia 和文本后端落后于现行源码，分别导致接口缺失、PathOps 缺符号和旧 Rust std 的 compact-unwind personality 超限；按已有准备脚本重建后全部闭合，没有修改编译器或链接策略。GN、Skia 与 Rust 文本依赖构建保存在 `../scoop-dep`，外部下载使用指定代理；本项原始日志、过期参考归档和独立检查锁已清理。
- 后续分别交付 P0b 的统一焦点可见性与 P0c 的真实窗口 Gallery；P1–P9 尚未实施，64 项目录当前完成数仍为 0。x86_64 留在整体收尾；Windows／Linux 未验证。

## P0b：统一焦点可见性

- Surface 的同一聚焦入口记录 Pointer／Keyboard／Programmatic 来源；Automatic／Always／Never 策略决定可见指示。真实按键只改变来源和必要指示，不改焦点修订号；同目标、同指示的重复请求不产生额外帧需求。
- FocusHandle、FocusOptions、Tab／roving／lazy 导航和延迟显露沿同一路径传递来源，模态关闭恢复目标原有来源。Press 与文本宿主同时公开实际 focused 和 focusVisible，主题几何、绘制及局部 focused 覆盖只消费后者。文本宿主的 Automatic 包含可编辑及只读输入的指针焦点。
- 仅指示变化通过独立通知发布，不重发 FocusChanged、不重建 IME、不改选区。Surface 的 74 项相关定向测试通过；其中新测试覆盖真实／synthetic 键盘、三种策略、OS 失焦／恢复、失败请求、禁用／关闭及 IME 所有权。原有重复聚焦测试捕获的额外刷新已修复并复验通过。
- macOS aarch64 release 的四个真实窗口用例全部通过：Tab／延迟显露／句柄生命周期、模态恢复、Button／Checkbox 的指针与键盘焦点及 Metal 停帧、文本输入／只读／IME／主题切换／虚拟移除。八个受影响生产库构建、相关 tests 类型检查、38 个 Scoop 文件格式检查及 diff 检查通过；没有扩大到全仓测试。
- 已清理本项 Surface／焦点／模态测试输出和独立检查锁；保留后续控件批次会复用的主题／控件测试目录及共享编译缓存。P0c Gallery 尚未交付，64 项目录完成数仍为 0；x86_64 留到收尾，Windows／Linux 未验证。
