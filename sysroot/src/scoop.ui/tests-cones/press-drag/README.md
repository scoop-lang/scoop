# Press、Drag、LongPress 与 Swipe 窗口测试

从本 Cone 用 `scoop test --release --filter 'long presses share'` 运行公共长按场景；先按仓库开发 sysroot 流程准备 UI 及依赖。每个带 Application 的测试应在独立进程中运行，不与其他 GUI 测试并发。

长按用例通过 native 接触队列进入真实 macOS 窗口，共 34 步，覆盖普通短按、一次长按、Touch／Pen 来源、父子相同期限的竞争、区域局部坐标、移动超限后返回、第二触点、显式取消、配置和回调更新、被阻止的移动／结束、原生取消、Accepted 变换、隐藏窗口、Drag 接管、零延迟及回调内关闭。行为与真实 Press 组合；测试结束检查挂载、区域、行为和原生观察者均已收尾。

原 `press composes with axis` 和 `contacts preserve tap intent` 分别验证鼠标／键盘及直接接触的 Press／Drag 组合。长按的非法时长／距离与默认值由 scoop.ui 的 `long press settings validate` 普通测试覆盖。

`--filter 'swipes combine'` 单独运行 36 步 swipe 场景，覆盖直接接触／笔尖、可选鼠标、原生离散来源、短按保留、距离／时长／主轴约束、来源与配置更新、回调替换、被阻止的移动／结束、第二触点、双指变换接管后不补 swipe、Accepted 镜像方向、准入重入更新、原生接替直接候选及回调内关闭。纯值规则由 scoop.ui 的 `--filter 'swipe '` 两项测试验证，包括配置非法值和边界时长。

公共合同见 [LongPress](../../../../../docs/specs/SCOOP-UI-SPEC.md#7101-公共-longpress) 和 [SwipeGesture](../../../../../docs/specs/SCOOP-UI-SPEC.md#7111-公共-swipegesture)。受控队列验证输入路由与实际窗口行为；真实触屏／笔和其他平台另按设备矩阵验收。
