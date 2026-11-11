# 有状态计数器

完整主题通过 `scoop example ui/state` 发布，源码由 catalog 直接引用本目录。

在真实图形会话中执行 `scoop test --release .`，再用 `scoop run --release .` 观察重复点击、新增、删除、重排和关闭。计数器由稳定 ID 标识，回调读取当前 props，组件作用域持有快捷键注册。纯模型与原生宿主测试分别验证状态规则和生命周期。
