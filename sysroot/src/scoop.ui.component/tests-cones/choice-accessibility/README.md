# 选择控件的原生无障碍回归

通过公开 NSAccessibility 方法检查实际 Switch、Toggle、RadioGroup 和 ToggleGroup，复用 [UI 原生测试辅助](../../../scoop.ui/test-native)。每次操作由原生主队列发起，实际绑定及正常 owner 更新后读取结果，不需要等待 drawable。

覆盖角色／子角色、静态标签合并、Field 标签／说明／错误／必填、受控拒绝与转换、单选的幂等性、单选／多选 ToggleGroup、禁用与隐藏、按 key 重排／删除／重建及客户端保留旧对象。独立窗口和辅助技术的呈现／焦点验证仍由[总计划](../../../../../docs/ui/interaction-polish/PLAN.md)记录。

使用已更新的开发 Cone 路径运行 `scoop check --tests --release` 和 `scoop test --release`；源码检查或构建不代替运行通过。
