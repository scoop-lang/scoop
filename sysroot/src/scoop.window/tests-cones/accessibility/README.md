# Window accessibility 边界测试

`SemanticNativeModelTest` 在三个桌面目标共用生产文本范围与集合实现，覆盖端点编辑映射、原子拒绝、折叠 caret、clone／reset／close、稳定 key 重排与删除重插、静态选择边界和 UTF-8／UTF-16／scalar 换算。它不需要前台窗口。Window 自身的 `SemanticPublicationTest` 另验证实时公告合并、busy 祖先、隐藏／保护／模态阻止及关系退休，窗口不请求激活。

本轮先完成整个领域的实现，再统一编译与运行相关测试；实际结果记录在 [实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。下面的进程内 AppKit 方法、外部客户端和读屏器覆盖分别记录。

本 Cone 在 macOS 使用真实 AppKit 语义对象，验证低层 provider 的属性发布、状态与角色、UTF-8 值写入、实际回调结果、Context、重入、节点／provider／窗口退休和异常。它不绘制 UI 控件，不作为 VoiceOver、Accepted 场景或完整 UI 接入通过的证据。

同一 Application 中还覆盖名称／说明／错误与 controls 关系：多标签及传递名称、静态文字、显式空值、内容名称回退、依赖更新、内容版本与几何失效、合法非名称循环、重复／自身引用拒绝、hidden 标签、自动清理和旧原生对象。实际名称环（含普通关系与内容引用的混合环）在 publish 时拒绝并保留原投影，修正后可重试；显式名称覆盖的依赖可正常发布。ignored 包装展开后代，禁用／隐藏继续约束子树，恢复保留原生对象与阅读顺序。原生查询使用公开 Children／Parent、TitleUIElement、ServesAsTitleForUIElements、LinkedUIElements 和 Help，不读取生产私有状态。

普通 `scoop test . --release` 在独立的原生主队列任务中执行 NSAccessibility 方法，确保动作不被业务回调的重入保护误接收。所有 native 检查失败均返回对应源码行号；最后一个动作故意抛出异常，验证正常退出和 callback 退休。

精确命中用例通过 graphics 创建变换后的带孔 Path 与矩形，关闭源 Canvas／Path／Geometry 后再查询真实原生对象。公开 accessibilityHitTest 覆盖 view／根／子树入口、窗口边界、层叠与阅读顺序分离、hidden／ignored／disabled、仅矩形时不猜命中、内容失效及旧对象退休。发布名称环失败后旧投影仍可命中，修正发布后不再使用旧快照。

滚动轴在同一 Application 中独立验证 ScrollBar 的方向、比例值与上下限、父容器轴关系、Increment 和 NSNumber 写入、非有限／越界／错误类型拒绝、只读／禁用／忽略、无范围与零范围、方向改变和旧对象退休。回调实际改变测试模型并重新发布，范围变化保留原几何及正文版本；该直接 provider 用例不冒充 UI 的 Accepted 视口验证。

跨进程验证使用本目录的 `native/client.c`，通过公开 AXUIElement C API 连接测试进程，按实际 identifier 查找并执行动作。不需要新增服务或第二个编辑模型。先在仓库外编译客户端，再设置 `SCOOP_ACCESSIBILITY_CLIENT` 为其绝对路径运行同一个 Cone：

```sh
clang -std=c11 -Wall -Wextra -Werror native/client.c \
  -framework ApplicationServices -o /absolute/work/ax-client
SCOOP_ACCESSIBILITY_CLIENT=/absolute/work/ax-client scoop test . --release
```

客户端必须具有系统 Accessibility 权限；测试只检查现有权限，不弹出提示或修改系统设置。客户端完成一次请求后退出，owner 按实际结果继续下一个场景；最终异常收尾仍由进程内调用覆盖，避免外部测试进程因应用退出丢失完成通知。外部客户端退出状态与 AppKit 方法测试分别记录。

外部客户端会验证按钮／切换、拒绝及成功的文本值设置、聚焦与解除焦点、隐藏／禁用，以及自身、provider 和窗口退休后的保留引用。关系的跨进程查询、完整文本范围、虚拟条目、精确命中和读屏器任务沿 UI 总计划继续。

焦点通过应用的 AXFocusedUIElement／AXFocusedWindow 和节点自身 AXFocused 核对；窗口自身的 AXFocused 不是后代焦点的同义值。AppKit 的 AXUIElementPerformAction 可能在禁用动作拒绝时仍返回 kAXErrorSuccess，标准禁用 NSButton 也复现此行为；客户端因此检查实际值不变，应用端继续严格检查业务回调次数。退休后的再次调用允许系统确认或对应 unavailable／unsupported 错误，但旧对象仍必须不可读取且不能再次执行业务。传输失败不当作禁用或退休成功。

选择角色的值校验同时覆盖 Switch／ToggleButton／RadioButton 的 Off／On 与 Mixed 拒绝；拒绝保留原几何和正文版本，未发布节点关闭不改变已发布原生树。实际选择控件的原生角色、动作、受控绑定和按 key 生命周期由 Component 的 choice-accessibility Cone 覆盖。
