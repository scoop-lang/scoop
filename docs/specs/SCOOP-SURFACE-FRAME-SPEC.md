# Scoop Surface 原生窗口区域规范

状态：D7 实施合同。沿用 [Surface 规范](SCOOP-SURFACE-SPEC.md)第 4 节的 Accepted 边界与[原生窗框规范](SCOOP-WINDOW-FRAME-SPEC.md)。Window 仍允许无 Surface 的调用方直接提交区域；本接口只负责把真实呈现场景接入该合同。

## 1. 区域句柄与能力

`SurfaceWindow.createWindowRegion(kind, parent: InputScope? = None): WindowRegionHandle` 创建该 Surface 拥有的区域登记。`WindowRegionKind.Shape／Input／Hit(WindowHitRole)` 分别声明整个窗口的裁剪、独立输入覆盖及 Caption／Resize／SystemButton／Client 命中语义。每个 Surface 同时至多有一个 Shape 和一个 Input 登记；Hit 可以有多个。

Shape／Input 不接受 parent；Hit 默认属于 Surface 的根输入 scope，也可明确归属同窗口的 parent。创建时按实际 WindowFrameCapabilities 验证所需能力，不能表达的角色报告 Unsupported。它不会把不支持的原生缩放／系统按钮命中静默改成 managed 回调；组件可以根据能力明确选择 beginResize 等不同入口。

WindowRegionHandle 提供只读 kind、isClosed、可变 enabled（默认 true）和幂等 close。它不拥有 Window 或 InputScope。Surface 关闭时关闭剩余登记，清除自己已经发布的区域；没有使用本接口的窗口不被额外重置。登记关闭不恢复历史原生属性。

## 2. 场景提交

`FrameContext.place(handle, shape: Rect／Path)` 在当前 Canvas transform／clip 中登记候选。候选通过 graphics 的 CanvasGeometry 和 rasterizeRegion，按当前 WindowMetrics.scaleFactor 转换成窗口逻辑坐标的二进制覆盖。它不推导图片 alpha 或修改实际绘制。

同一帧不能重复放置同一个句柄，不能使用另一窗口或已经关闭的登记。Shape／Input 的空覆盖是空区域；该帧未放置的登记表示恢复默认覆盖。多个 Hit 按实际 scene 顺序发布，重叠与系统保留按钮的优先级沿用 window 合同。

只有 RenderFrame.submit 返回 Accepted 后，候选才能发布到 native；Discarded 或绘制失败保留上一有效提交。尺寸、DPI 或窗框变化撤销旧快照并请求必要帧；变更前已捕获的候选不能在变更后重新发布。Accepted 表示提交与几何发布完成，不承诺像素扫描和系统命中原子发生。

候选记录 Window.frameRevision，并在发布前比较当前值；实际窗框已变化而 FrameChanged 仍在队列中时，同样丢弃候选并请求新帧。该检查只读取原生配置版本，不逐帧重新查询标题栏和按钮几何。

enabled、所属输入 scope 的关闭／禁用及 AllInput 边界可立即撤销 Hit 资格。恢复资格可以继续使用仍有效的 Accepted 几何；新句柄和新位置必须等新 Accepted。逻辑边界检查由既有 InputRouter 驱动，没有轮询或新的输入线程。原生窗口模态的范围继续由 window 自己处理。

## 3. UI 桥接

ComponentScope 提供 `createWindowRegion(kind)`，随挂载管理登记；Hit 使用该 scope 的输入路由。SceneContext 增加对应 `place(handle, Rect／Path)`，只在真实选中的布局 scene 中采集候选。使用原生区域的 scene 不复用不含该候选的绘制命令缓存；纯绘制缓存和其他子项继续使用既有策略。

WindowChrome 的整个窗框模式、标题栏布局及窗口区域配置仍由根组件唯一声明。普通子组件可拥有独立的 Hit 登记，不能抢占整个窗口 Shape／Input 配置。具体组件入口与降级策略由控件规范定义。

## 4. 呈现与验证

AppKit CAMetalLayer 安装时读取 NSWindow.opaque；之后 window 的透明度／背景更新同步当前 contentView.layer.opaque。透明度本身不表示窗框几何发生变化；实际尺寸和窗框变化通过对应事件驱动必要重绘。Win32 DirectComposition 和 GTK GtkGLArea 保持预乘 alpha；GTK 继续使用已有物理 framebuffer 尺寸，不再次应用显示器分数缩放。

测试覆盖最终 transform／clip、Discarded 不发布、尺寸改变撤销、空覆盖与未放置、卸载／禁用、输入边界、Surface 关闭后的 Window 存活，以及原生区域与透明呈现的组合。使用真实 renderer 和原生快照或输入探针，不把观察自有候选数组当作 OS 命中结果。
