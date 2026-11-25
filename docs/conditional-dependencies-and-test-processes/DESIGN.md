# Cone 条件依赖与逐用例测试进程

条件依赖已实现，逐用例测试进程待实现。条件依赖解决平台后端只在部分目标上存在的问题。逐用例进程隔离让需要独占进程状态的测试获得一次全新的程序启动。例如 `scoop.window` 的原生测试需要各自创建 `Application`，同一 Cone 中的尺寸换算测试只需要普通实例。当前窗口库通过整个 Cone 的 `[test] isolate = true` 运行这两类测试。[窗口库清单](../../sysroot/src/scoop.window/Cone.toml) · [Application 测试](../../sysroot/src/scoop.window/tests/application.scoop) · [窗口值类型测试](../../sysroot/src/scoop.window/tests/values.scoop)

## 条件依赖

在 `[dependencies]` 和 `[test-dependencies]` 的依赖表值中增加可选 `when`。以下是一个库的清单片段。

```toml
[dependencies]
"dev.example:common" = { path = "../common" }
"dev.example:appkit" = { path = "../appkit", when = { os = "darwin" } }
"dev.example:linux-poll" = { path = "../linux-poll", when = { arch = "x86_64", os = "linux", env = ["gnu", "musl"] } }

[test-dependencies]
"scoop:scoop.test" = "=0.1.0"
"dev.example:gnu-probe" = { path = "../gnu-probe", when = { os = "linux", env = "gnu" } }
```

| 请求的 target | 直接生产依赖（省略隐式 core） | 仅在测试根增加 |
| --- | --- | --- |
| `aarch64-apple-darwin` | `common`、`appkit` | `scoop.test` |
| `x86_64-unknown-linux-gnu` | `common`、`linux-poll` | `scoop.test`、`gnu-probe` |
| `x86_64-unknown-linux-musl` | `common`、`linux-poll` | `scoop.test` |

`when` 与 `[[sources]]` 共用已登记的构建目标和谓词。不同键取“与”，同一键的字符串数组取“或”。省略条件或写 `when = {}` 匹配全部目标，某一维写空数组则匹配零个目标。`env` 指目标 ABI 环境。在 macOS 上为 Linux GNU 构建时，使用表中的 Linux GNU 行。使用后端的源码可在 `[[sources]]` 或 `[[test.sources]]` 上写同一条件。未选中的依赖不提供直接可见的名称，无条件源码若仍导入它，按普通名称查找规则诊断。[语言规范 §12.2.2～12.2.4](../specs/SCOOP-SPEC.md#1222-scoop-源码选择与平台条件)

### 声明与求解顺序

1. **读清单。** 校验所有声明的 key、版本、来源字段及 `when` 的键、类型和值，并将错误定位到声明。字符串依赖是无条件简写。一个 Cone 对同一 `group:name` 只有一个直接声明位置，生产与测试表互斥，即使写上互斥条件也按重复声明报错。
2. **展开继承。** 成员可写 `{ workspace = true, when = { os = "windows" } }`。`[workspace.dependencies]` 提供完整的版本与来源，路径仍相对定义它的清单。条件属于成员使用这条边的位置。`[workspace.dependencies]` 和根 `[overrides]` 自身均不接受 `when`。成员清单中的继承拼写错误在这里报告。
3. **选择直接边。** 对每个源码节点，PM 以本次请求的 canonical target 筛选生产边。仅 Test 根再筛选自己的测试边，其依赖一律使用 Production。对匹配的依赖才应用根 override、推断 path 版本、查找 artifact/sysroot、查询 registry 或获取 Git。未匹配的边不要求 locator 存在，也不因 override 而重新出现。筛选后沿既有规则补齐精确的生产 core 边，显式 core 条件未命中时仍有隐式 core。
4. **求解和编译。** 同一解析单位的有效边遵守现有单版本、来源一致和无依赖环规则。相同的包仍可能由另一条有效边引入。`Cone.lock`、`prepare`、`metadata` 和 `deps --why` 使用该 `(root, purpose, target)` 的精确图。driver 按图构建，`scoopc` 消费 direct/support 产物。已有 `.slib` 的依赖已经针对其目标绑定，消费时沿用产物内的精确引用。[包管理规范 §3～7](../specs/SCOOP-PACKAGE-SPEC.md#33-解析隔离)

清单变动可以触发 PM 重新读取。若只改了本目标未命中的合法声明，当前精确图及编译输入相同，已有 lock 选择和构建结果可复用。命中条件改变了当前直接边时，默认解析更新该记录，`--locked` 对缺少或已不满足的记录报错。其他目标的记录按各自请求使用。PM 的 `add` 更新已有依赖时保留其 `when`，`remove` 删除该条完整声明。

### 源包与 registry

发布的独立清单保留各目标的生产与测试声明以及原来的条件维度。静态 registry 的生产依赖记录可附 `"when": {"os": "linux", "env": ["gnu", "musl"]}`，缺省为无条件。PM 从索引枚举候选的依赖时即按消费目标筛选，选定并首次展开源码包时沿现有清单与索引比对包含该条件。这样发布者所在的目标不会裁掉消费者需要的分支。测试依赖只随源包清单交付。清单 schema 1 和 registry 格式 1 只增加这一可选字段，lock 保存现有精确边，不保存谓词。发布工具实施时按这一规则转换所有分支，当前静态 registry 消费可用手工准备的索引验收。[包管理规范 §8～9](../specs/SCOOP-PACKAGE-SPEC.md#81-静态读取协议)

## 测试进程

`scoop.test.Test` 的签名改为 `public annotation class Test(val useProcess: Boolean = false)`。Scoop 源码中的布尔类型名是 `Boolean`。下面的第一个方法修改进程内唯一的对象，第二个方法可以在同一轮验证其初始状态。

```scoop
package dev.example

import scoop.test.*

private object Session {
    var claimed: Boolean = false
}

@Test
private class SessionTests {
    @Test(useProcess = true)
    fun claimOnce() {
        assertFalse(Session.claimed)
        Session.claimed = true
        assertTrue(Session.claimed)
    }

    @Test
    fun initiallyAvailable() {
        assertFalse(Session.claimed)
    }
}
```

| 配置位置 | 作用 |
| --- | --- |
| 方法上的 `@Test(useProcess = true)` | 该用例独占一个新子进程，即使过滤后只选中它 |
| 容器上的 `@Test(useProcess = true)` | 每个选中的用例各起一个进程。方法写 `false` 也服从容器的隔离要求 |
| 测试根的 `[test] isolate = true` | `scoop test` 本次选中的全部用例分别起进程 |
| 三处均为 `false` 或省略 | 选中的普通用例依次在同一个测试父进程中运行 |

前端按真实注解身份和已绑定的布尔常量合并容器与方法的要求，写入 HIR 的每个 `TestCase`。命名参数、位置参数、常量、别名和错误位置沿用普通注解规则。生成的 TestEntry 读取这个布尔值，命令行和编辑器仍从 `.slib` 展示目录选取原有序号。测试目录为空时沿用空入口，无须给该目标隐式添加 `scoop.test`。清单开关由驱动在运行时传入。直接启动可执行文件使用编入程序的注解，无需查找项目清单。[现有注解声明](../../sysroot/src/scoop.test/src/markers.scoop) · [HIR 测试目录](../../compiler/hir/src/test_suite.rs) · [入口合成](../../compiler/hir-lower/src/test_entry.rs)

### 谁启动子进程

1. `scoop test` 处理 `--filter`、`--ignored`，按目录顺序选定用例。空选择、`--list`、`--no-run` 均不启动测试程序。有选中用例时只启动一次套件，将选中的扁平序号传给它。整 Cone 隔离时在现有 `--scoop-test-cases` 参数前加内部参数 `--scoop-test-isolate-all`，取代驱动按用例拆分启动的循环。
2. 生成的 TestEntry 是测试父进程的入口。它在选中用例后检查隔离要求。普通用例直接调用已有适配。隔离用例由入口调用 `scoop.test.execution` 启动当前已链接的可执行文件，传入独立的 argv 参数 `--scoop-test-child <index>`。程序路径取自当前运行映像，支持用户通过相对路径启动后改变工作目录。子进程继承当前 cwd、普通环境与 stdin。仅在子进程环境中移除父端的 `SCOOP_TEST_RUN_DIR` 并设置 `SCOOP_TEST_CASE_RESULT` 为本例的绝对结果路径，以免重新打开整轮事件文件。
3. 子进程入口优先识别单用例模式，按一个合法序号直接分派。普通用例沿用 `runReportedTestCase`，子进程用同一个生成的 `initialize` 回调调用 `runTestCase`，将结果交给父入口。构造、可选 `BeforeTest`、正文与可选 `AfterTest` 都在子进程中至多执行一次，按原有失败规则跳过相应阶段。子进程直接执行已经选定的 Ignore 用例，也不会再派生测试进程。私有模式缺少结果位置、参数数目错误或序号越界时以运行错误退出。
4. 父进程等待并回收当前子进程，再执行下一例。对于“普通 A、隔离 B、普通 C”，A 和 C 共享同一父进程，B 在独立进程的主线程完成整个生命周期。目录顺序与当前串行语义一致，`-j` 继续控制编译并发。进程内 singleton 随每次启动重建。固定端口、系统文件锁等资源由 OS 在进程间共享，两个独立的 `scoop test` 调用仍需自行协调这些资源。

直接运行程序的无参数模式、`--scoop-test-cases`、`--scoop-test-filter` 和 `--scoop-test-filter-ignored` 都进入同一个父入口。每次启动仍会执行普通程序的 eager 初始化，选中之后才执行该用例的构造与 hooks。[语言规范 §16.5](../specs/SCOOP-SPEC.md#165-每用例执行与失败保留) · [运行时启动合同](../specs/SCOOP-RUNTIME-SPEC.md#7-程序启动与退出)

### 结果、退出状态与输出

父入口是整轮 `events.jsonl` 的唯一写入者。对隔离用例，它先写并刷新 `begin`，按本轮开始顺序分配 stdout/stderr 文件，然后启动子进程。文件序号与完整目录序号分开，例如只选中目录项 7 时，使用第 0 组输出文件。子进程从启动到退出的两路字节均归入该用例，包括 native 输出、崩溃前已写出的字节和 shutdown 诊断。父进程自身的启动与退出输出仍归目标。直接重定向到文件可同时承接大量 stdout 和 stderr。`--nocapture` 以及未设置外层结果目录的独立运行继承两路标准流，状态交接仍执行。

子进程在完整生命周期结束后交付一个单用例结果文件。首行保存目录序号、`passed` 或 `failed` 及单调时钟测得的生命周期纳秒数，第二行使用现有的 `failures` JSON 数组编码。两项失败仍依次表示首次失败和清理失败，断言位置、expected/actual、cause 都在持有真实 Throwable 的子进程中渲染。子进程写完再将临时文件改名到指定结果路径，然后正常返回 0，断言失败也以记录交付。父入口据首行汇总状态，向外层 `end` 转送同一份失败数据。无外层结果目录时，父入口只为实际执行的隔离用例创建临时位置，消费完成后清理。

| 父进程观察 | 该用例和后续调度 |
| --- | --- |
| 收到完整结果，子进程退出 0 | 采用记录中的通过或失败，继续下一例 |
| 子进程提前 `exit(0)`、结果缺失或截断 | 当前用例失败并报告缺少完整结果，保留已取得的输出，继续下一例 |
| 非零退出、POSIX 信号或 Windows 异常退出 | 当前用例失败，保留 OS 状态、已有结果及两路输出。收尾失败也不能覆盖此前的断言详情，继续下一例 |
| 无法启动子进程，或当前可运行测试的环境禁止再启动自身 | 当前用例失败，报告启动错误及用例位置，继续下一例 |
| 父进程被取消、不能确认子进程已退出或整轮结果通道写入失败 | 中止本轮，剩余用例记为未运行 |

隔离用例的 `end` 及 JSON 用例报告增加 `process`，沿用整轮报告已有的 `{"code": 0, "signal": null}` 或 `{"code": null, "signal": 6}`。Windows 退出码保留完整 32 位。启动失败时 `process` 为 null，`interruption` 记录实际错误。所有中断即使 `failures` 为空也计为失败。父入口完成所有选中用例后只写一次 `complete`，全部通过返回 0，任一失败返回 1。JSON 沿用 stdout/stderr 各自的 UTF-8 文本或 hex 编码，通过用例的输出按现有展示规则处理。独立运行遇到子进程中断时，父入口将用例名和中断原因写至 stderr。[现有事件读取](../../compiler/scoop/src/cli/test_run/events.rs) · [报告字段](../../compiler/scoop/src/cli/presentation/test_report.rs) · [结果与捕获](../../sysroot/src/scoop.test/src/execution/report.scoop)

进程操作放在 `scoop.test` 现有的私有 native 测试支持中。macOS/Linux 使用 `posix_spawn` 和 `waitpid`，Windows 使用 `CreateProcessW` 并等待和关闭进程句柄。等待沿普通 NativeSafe FFI 执行。这个边界使 `scoop.platform` 自身的测试仍可使用 `scoop.test`。若目标环境不提供自启动能力，该目标的测试支持在实际请求启动时返回具体错误。交叉目标的 `check --tests`、`test --no-run` 和 `--list` 仍可按已有构建条件完成。[测试库清单](../../sysroot/src/scoop.test/Cone.toml) · [平台库清单](../../sysroot/src/scoop.platform/Cone.toml)

## 实施与验收

| 实施位置 | 可观察的验收结果 |
| --- | --- |
| Manifest、workspace 和 PM 求解 | 在同一宿主请求表中三个 target，核对精确边。分别改变单个 `arch`、`os`、`env` 条件，覆盖字符串、数组及组合。未命中分支的非法谓词仍定位到清单，未安装的后端仅在命中时报告定位错误。 |
| PM 与构建的交接 | 用带条件的传递依赖和测试依赖验证 Production/Test 可见性、workspace 继承、override 及 core。两个 target 的 lock 记录可共存。只改未命中分支时已有 `--locked` 构建复用，改为命中后旧图不满足。用本地 registry 记录实际请求，并核对发布清单与索引的条件。 |
| 测试注解与入口 | HIR 测试覆盖默认值、方法或容器置 true、类型错误与别名。真实测试程序交错运行普通和隔离用例，用 PID、对象初始状态和 hook 顺序验证进程归属及一次执行。两个隔离用例各自成功取得同一进程级资源。 |
| CLI 与独立运行 | 对同一 executable 检查直接启动、名称过滤、单序号、Ignore、整 Cone 隔离和空选择。验证清单开关叠加方法注解仍只有一个工作子进程。列举与仅构建均无用例副作用。 |
| 失败与输出 | 同测正文和清理双失败、子进程提前退出 0、非零退出、信号及写完结果后的 shutdown 失败。后续普通用例仍能运行。分别验证双流大输出、非 UTF-8、看似事件的用户文本及 `--nocapture`。在可运行的 macOS、Linux、Windows 宿主用实际退出状态验收。 |
| 两项组合 | 给原生窗口测试文件和它导入的测试辅助 Cone 配置同一目标条件。匹配目标上由子进程调用该库，未匹配目标的测试图排除该直接边，其他普通用例照常运行。 |

普通库行为留在所属 Cone 测试，编译错误、进程边界和 CLI 报告使用相应的定向测试。实施时先让 `scoop.window` 的一组真实 `Application` 用例与值类型用例通过混合运行验证，再按该 Cone 的实际资源使用决定是否撤去整 Cone 开关。两项新写法可用后同步编程参考和示例。[编程材料更新流程](../scoop-programming/UPDATE-WORKFLOW.md)
