# 双轴虚拟表格的原生动作

使用生产 LazyElement、ScrollViewport、SemanticsHandle 和真实 AppKit 单元格代理，描述一亿行、300 列；只挂载窗口附近及明确动作需要的行列。一个测试进程只运行一个 Application。

准备与源码对应的本机 Window、Surface 和 UI 产物后运行：

```sh
scoop check . --tests --release
scoop test . --release
```

24 个阶段验证数据查询的只读边界、目标行列的同步 Realize、回收、Reveal 的 Accepted 几何、精确 Focus／清除、外部焦点取消、滚动取消、列重排、禁用、删除与同 key 重建，以及根关闭。窗口需要可激活的本机桌面。这里直接调用 AppKit 的公开无障碍协议，不代替跨进程客户端或 VoiceOver 验收。

2026-11-20 本机 macOS aarch64 的格式化、check 与 24 阶段 release 运行通过；一亿行、300 列场景累计构造少于 300 行和 2000 格，元数据查询期间构造数不变，Realize 精确新增一行一格，关闭后所有实例释放。已有祖先视口原生回归、Window 表格身份测试和 3 项滚动惯性测试通过。UI／Window API 文档已生成，并确认新入口可查询。原始报告保存在仓库外。
