# Receiver 查找与显式外层访问

通过 scoop.test 验证嵌套 receiver 的显式访问、普通词法捕获、同类型对象身份、当前函数属性优先级、初始化表达式、inner 构造及函数值适配。provider 提供真实跨 Cone 泛型 receiver 入口、属性和扩展调用。

原 receivers/nested、lookup、initializers fixture 只断言运行结果，没有必须保留的诊断、产物、ABI 或外部进程合同，因此其行为迁到本 Cone。拒绝隐式外层访问的编译错误及诊断位置保留在 language/receivers 的负例 fixture；HIR 和编辑查询另有定向 Rust 测试。

在仓库根目录使用已准备的开发 sysroot 运行：

```sh
SCOOP_SYSROOT="$PWD/sysroot" target/debug/scoop test --release tests/cones/receivers
```
