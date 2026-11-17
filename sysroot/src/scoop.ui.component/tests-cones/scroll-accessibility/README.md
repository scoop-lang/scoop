# 滚动容器与虚拟集合的原生无障碍场景

使用匹配工具链与开发 Cone 产物，在可呈现 macOS 桌面运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/scroll-accessibility --filter ScrollAccessibilityTest --release
scoop test sysroot/src/scoop.ui.component/tests-cones/scroll-accessibility --filter VirtualCollectionTest --release
scoop test sysroot/src/scoop.ui.component/tests-cones/scroll-accessibility --filter VirtualCollectionActionsTest --release
```

挂载真实 ScrollArea 与只读 Textarea，由公开 NSAccessibility 方法读取横／纵轴、写入位置并执行 Increment。等待真实 Accepted 帧后核对 ScrollState 和原生比例，覆盖视觉 Hidden 时保留能力且无条几何、恢复实际条、禁用、替换 ScrollState 后保留节点但更新目标、只读文本仍能滚动及关闭后的旧对象。

三个应用场景串行、各用独立进程，均不请求窗口激活。VirtualCollectionTest 使用十万条数据，核对范围查询不 build、逻辑代理弱缓存、重排与删除、实际子树／几何／命中及回收；不可用描述不伪报截断成功。

VirtualCollectionActionsTest 使用真实按钮行及 Unicode 文本 key，直接调用原生自定义动作。Realize 返回前真实子树可访问，随后允许回收；Reveal 等待 Accepted，后来的定位取消旧请求。还覆盖禁用拒绝、请求尚未呈现时重排、删除后重插入的旧引用、在途关闭和实例释放。聚焦只验证真实请求已进入原控制器，并在同一 native block 内取消；不等待或获取系统前台焦点。完整前台焦点和读屏器验收仍按统一平台矩阵执行。

本用例需要实际 drawable，不能把源码检查或可执行文件构建记作运行通过。底层范围、参数和原生动作在 Window 的 accessibility Cone 中验证；没有 drawable 的组件属性与挂载回归在相邻 accessibility Cone，纯滚动请求计算在 scoop.ui 的 ScrollSemanticsTest。
