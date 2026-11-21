# Tree 的原生语义消费者

本机 AppKit 定向测试在同一个 Application 中覆盖 LazyElement 与 data collection 的单列、多列 Tree。只发布逻辑行，不构造 View，检查 Outline／Row、披露父子关系、层级、表头与单元格、Mixed 勾选、拒绝和禁用、折叠后代理退休以及删除后同 key 重建。

```sh
scoop test sysroot/src/scoop.ui/tests-cones/tree-accessibility --release
```

Window 的普通集合测试覆盖同级位置和父 key 的局部验证，以及选择快照保留新增元数据。仅记录本机验收，不表示其他平台已运行测试。

2026-11-22 本机 aarch64-apple-darwin release 验证通过：本 Cone 的一项实窗测试覆盖四种组合，Window 的 SemanticCollectionTest 共 6 项通过，既有 grid-accessibility 的惰性层级表格动作回归通过。相关 Window／UI API 文档与 setChecked 查询同步通过。
