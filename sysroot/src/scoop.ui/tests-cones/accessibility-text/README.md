# UI 无障碍文本读取与编辑

使用配套工具链和当前 Window／Surface／UI 开发产物运行：

```sh
scoop test sysroot/src/scoop.ui/tests-cones/accessibility-text --release
```

macOS 用例将同一挂载的 TextField 语义与 TextEditHost 通过 updateTextEditor 接入。共用 native 辅助只调用公开 NSAccessibility 协议；通过实际模型、策略计数、undo 和发布结果验证动作，不把无返回值 setter 的调用当作写入成功。

覆盖 emoji、组合字符与中文的 UTF-8／UTF-16 转换、方向选区、正文／选中文本／attributed string 读取、空范围、越界与 surrogate 中间端点拒绝；单范围设置和多范围拒绝、只读选择、禁用／只读写入拒绝、外来文本清洗、单行／最大长度／Reject、撤销、composition 收尾，以及受保护文本的读取限制和允许的全文替换。属性在隐藏窗口中无需 measure／scene 就能发布；选择不会抢焦点，关闭后保留的原生对象不能读到旧正文或修改模型。

范围几何用例额外验证 AppKit 字符／行／按点查询、surrogate 拒绝、组合字符、caret 行号、软换行同一偏移的上下游切换、可见范围与实际选区矩形；真实窗口覆盖滚动、缩放、丢弃帧、正文先于布局更新、普通 update 解除绑定、恢复、缺少节点／文字登记和宿主关闭。测试通过与真实布局、模型和原生对象对照，不在 getter 中取得焦点。每个 Application 用例在独立进程运行。

独立 AX 模式使用 [native/client.c](native/client.c)，以公开 AXUIElement API 验证正文、字符／行／可见范围、StringForRange／AttributedStringForRange、范围矩形与按点定位。预期矩形来自实际 TextEditHost 排版和 Accepted 输入几何，再转换为 AX 的屏幕坐标。客户端通过 AXObserver 检查反向选区改为同范围正向选区时的一次选择通知、活动光标行号、没有正文通知，以及重复设置不重复通知。应用随后核对真实模型、焦点和后续编辑行为。

先编译客户端，并用 `scoop test ... --release --no-run` 构建测试。跨进程查询时将测试可执行文件放入具有正确 CFBundleExecutable 的普通 `.app` 中，直接启动包内二进制；编辑用例当前为 case 1，几何生命周期用例为 case 0，各用独立进程。示例：

```sh
clang -std=c11 -Wall -Wextra -Werror \
  sysroot/src/scoop.ui/tests-cones/accessibility-text/native/client.c \
  -framework ApplicationServices -o /absolute/work/ax-text-client
SCOOP_ACCESSIBILITY_TEXT_CLIENT=/absolute/work/ax-text-client \
  /absolute/work/ui-accessibility-text.app/Contents/MacOS/ui-accessibility-text \
  --scoop-test-cases 1
```

客户端检查现有 Accessibility 权限，不弹出授权提示或修改系统设置。未设置该变量时保留进程内协议验证；两种模式分别记录，均不代替 VoiceOver 或其他平台验证。完整范围和实际结果见[统一计划](../../../../../docs/ui/interaction-polish/PLAN.md)及[实施记录](../../../../../docs/ui/interaction-polish/PROGRESS.md)。
