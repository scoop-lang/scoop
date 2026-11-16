# 滚动条的原生无障碍场景

使用匹配工具链与开发 Cone 产物，在可呈现 macOS 桌面运行：

```sh
scoop test sysroot/src/scoop.ui.component/tests-cones/scroll-accessibility --release
```

挂载真实 ScrollArea 与只读 Textarea，由公开 NSAccessibility 方法读取横／纵轴、写入位置并执行 Increment。等待真实 Accepted 帧后核对 ScrollState 和原生比例，覆盖视觉 Hidden 时保留能力且无条几何、恢复实际条、禁用、替换 ScrollState 后保留节点但更新目标、只读文本仍能滚动及关闭后的旧对象。

本用例需要实际 drawable，不能把源码检查或可执行文件构建记作运行通过。底层范围、参数和原生动作在 Window 的 accessibility Cone 中验证；没有 drawable 的组件属性与挂载回归在相邻 accessibility Cone，纯滚动请求计算在 scoop.ui 的 ScrollSemanticsTest。
