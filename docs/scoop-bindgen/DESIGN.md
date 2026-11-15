# scoop-bindgen 设计

设计日期：2026-11-09。核对基线：`10a20c6c78`。状态：设计完成，待技术探针与实现。

本文在[调研报告](INVESTIGATION.md)上确定命令、配置、类型映射、生成流程和验收边界。
文中的命令与配置是拟实现合同，不表示当前工具已经存在。本次只增加设计文档；现有语言、
runtime、产物和构建行为仍以三份规范为准。

## 1. 目标与主要决定

`scoop-bindgen` 将 C／Objective-C 头文件转换为可以独立编译、消费和审查的普通 library
Cone，减少手写 extern、布局声明和 native facade 的工作。使用者显式提供头文件、目标及
链接需求；工具不下载原库，也不从头文件猜测完整的库使用协议。

| 项目 | 决定 |
| --- | --- |
| 入口 | 独立 Rust 宿主工具，通过现有外部命令协议运行 `scoop bindgen` |
| 解析 | libclang CIndex；使用真实 target、SDK、include、宏与语言配置 |
| 输出 | 每个完整 target 一份普通 Cone，包含 Scoop 源码及需要的 C／Objective-C 源码 |
| 原始 API | 保留 C ABI，extern 调用为 unsafe；不默认转换字符串、错误码或 ownership |
| 分类 | 每个选中根都有 `Direct`、`Shim` 或 `Unsupported` 结果，显式排除另记 `Excluded` |
| C 首期 | 标量、指针、typedef、开放枚举、可表达 struct、函数、函数指针、global／TLS |
| C 扩展 | inline、宏、固定 vararg、调用约定和按值聚合类型适配 |
| Objective-C | Darwin 上生成 ARC `.m` facade，用静态类型发送消息，Scoop 侧使用固定 C ABI |
| 生成检查 | 显式重新生成，并以普通 library build 检查候选结果；不运行目标程序 |
| 日常构建 | 直接消费生成 Cone／`.slib`，不自动调用 bindgen；继续使用已有构建缓存 |

这里的 `Direct` 指不需要 bindgen 改写原 API 的 ABI，不表示编译器必然采用机器级直接调用。
例如合法 `@CLayout` 参数仍可能使用编译器已有的 C storage bridge。

C++、Objective-C++、Swift-only API、通用 Blocks／delegate 桥接不进入首期。工具不进入
parser → HIR → MIR → LIR 流水线，不增加 bindgen 专用 `.slib` section、runtime handle
协议、来源认证或资源配额。高层库仍可以在另一个手写 Cone 中封装生成结果。

## 2. 规范依据与对调研的收敛

实现必须复用以下合同：

- [语言规范](../specs/SCOOP-SPEC.md) §4.1、§4.2.1、§9.1.2：struct、整型枚举与常量；
  §11.18：`Closeable`／`use`；§12.2：Cone 与 native 配置；§13～14：FFI、指针及 callback。
- [运行时规范](../specs/SCOOP-RUNTIME-SPEC.md) §3.5、§3.8、§4.3、§5.5：线程边界、
  release hook、managed callback 与异常不得穿越 C ABI。
- [实现规范](../specs/SCOOP-IMPL-SPEC.md) §1、§2.4～2.5、§2.7～2.8：完整目标、C ABI
  分类、外部命令、native 编译、普通缓存与链接。
- [包管理规范](../specs/SCOOP-PACKAGE-SPEC.md)：普通 Cone 依赖与构建准备；
  [路线图](../ROADMAP.md)中已经完成的 FFI、native source、目标目录和外部命令能力。

调研中的方向在本设计中作以下具体化：

| 调研中的建议 | 本设计采用的规则 |
| --- | --- |
| opaque record 的专名 handle | 使用 `Ptr<Unit>` 的透明别名，不生成空 struct；别名不提供 nominal 隔离 |
| `const char *` 写成 `Ptr<Int8>` | plain `char` 的符号性取自目标；未经声明不能统一视为 signed char |
| 只读 global／TLS | 只读普通 global 可用 extern `val`；只读 TLS 生成 getter，不能生成 `@ThreadLocal val` |
| union／array 的 storage shim | 通过 native 局部值和 `memcpy` 保留按值语义，Scoop 端显式使用字节存储 |
| Objective-C 异常可配置捕获 | 首期 facade 一律捕获；不提供允许异常穿越 Scoop 的关闭选项 |
| Objective-C 消费 receiver 延后 | 任意 consumed receiver 仍拒绝；明确配置的 `alloc + init` 组合留在一个 native 工厂内 |
| 可能自动生成 release hook | 首期完全不生成；确定性释放使用 `close/use` |
| compilation database 导入 | 首期不接收；先把实际使用的 include／define／flags 显式写入 TOML |
| libclang 随 LLVM 可用 | 显式增加 CIndex 动态库构建与打包；`libclang-cpp` 不能代替 `libclang` |

这些选择不要求增加 Scoop 语言类型或放宽 FFI 规则。将来确有消费者需要 union、inline
array、move-only ownership 等语言能力时，另作 spec-first 设计。

## 3. 命令、结果与目录

```sh
scoop bindgen generate --config scoop-bindgen.toml --target aarch64-apple-darwin
scoop bindgen check --config scoop-bindgen.toml --target aarch64-apple-darwin
scoop bindgen inspect --config scoop-bindgen.toml --target aarch64-apple-darwin \
  function:demo_shift
```

Windows 用户入口为 `sco bindgen`，外部程序仍为 `scoop-bindgen.exe`。`--config` 与
`--target` 必需，不根据宿主、cwd 中某个 Cone 或已有生成目录猜测目标。

| 参数 | 合同 |
| --- | --- |
| `--config path` | 完整配置文件；配置内路径以该文件目录为基准 |
| `--target triple` | 使用共享目录解析并规范化；Windows 必须明确 CRT 环境 |
| `--sysroot path` | Scoop 安装根；优先级为显式参数、`SCOOP_SYSROOT`、外部命令安装位置 |
| `--cc path` | 生成结果验证时的目标 C compiler，沿现有 toolchain 选择规则 |
| `--native-sysroot path` | 目标系统 SDK／sysroot，不是 Scoop 安装根 |
| `--libclang path` | 显式选择宿主 CIndex 动态库；省略时使用安装包内的固定位置 |
| `--scoop path` | 验证生成 Cone 所用的主驱动；默认取同一安装根的 `bin/scoop` 或 `bin/sco.exe` |
| `--message-format human\|json` | 人类可读结果或工具本地结构化结果；外部工具日志写 stderr |

外部命令之前的 `scoop --sysroot` 只决定主驱动到哪里查找命令，不会自动变成子程序参数。
开发环境使用 `SCOOP_SYSROOT`，或把 bindgen 自己的 `--sysroot` 放在子命令之后。
选项通过 argv 传递，不经 shell；本工具不增加任意命令 hook。

- `generate`：解析、规划、发射并完成候选 Cone 的 release library build，成功后发布源码。
  编译检查包括 Scoop 后端和实际 native source，但不链接最终 consumer，不验证未提供的原库。
- `check`：执行相同生成与候选编译，随后比较全部受管源码文件的路径集合及字节；不更新旧目录。
  差异列出新增、修改、删除的路径，人类模式可附文本 diff。
- `inspect`：对指定声明解释解析、选择、映射与适配结果；不发射 Cone，不启动 Scoop build。
  允许查询未被 allow 的声明；无法唯一定位时列出候选，不自动选择。

退出码：`0` 为成功，`1` 为配置／解析／映射／编译／I/O 失败，`2` 为 `check` 发现差异。
`inspect` 成功解释 `Unsupported` 也返回 `0`，其结果携带分类；声明不存在、歧义或 TU
解析失败返回 `1`。这使诊断查询和生成成败可以分别判断。

输出布局固定为：

```text
generated/demo/<canonical-target>/
  Cone.toml
  src/bindings.scoop
  src/wrappers.scoop             # Only when a configured wrapper is emitted.
  native/bindings.c              # C shims and checks against the original headers.
  native/bindings.m              # Objective-C facade and its header checks.
```

`binding.output` 指定 `generated/demo` 部分。C 模式至少生成一个包含实际 header 检查的
`.c`；Objective-C 模式由 `.m` 承担相同职责，不再为同一事实另发一份 `.c` 检查。
验证用 `.slib`、对象、`Cone.lock`、日志及临时文件都不发布到生成源码目录。

例如 §4.1 配置生成的 manifest 包含以下实际构建输入；core 沿普通隐式依赖提供：

```toml
schema = 1

[cone]
group = "example.bindings"
name = "demo"
version = "0.1.0"
kind = "library"

[native]
include = ["../../../vendor/demo/include"]
c_flags = ["-std=c17", "-DDEMO_NO_LEGACY=1"]

[[native.sources]]
path = "native/bindings.c"

[[native.libraries]]
name = "demo"
kind = "dynamic"
```

Scoop source 使用默认的 `src/` 发现规则。Objective-C 对应使用 `objc_flags` 和 `.m`；
原始宏、flags、include 与 library 条件先按 target 展开，再写入这份单目标 manifest。

同一个 coordinate 的各 target 版本是独立构建输入。首期不生成多目标聚合 manifest，也不
增加条件依赖；consumer 显式选择正确的目标目录，或消费该目标生成的 `.slib`。
生成的 native TU 检查 `SCOOP_TARGET_OS_*`／`ARCH_*`／`ENV_*`，误用其他 target 时给出
编译错误，不能靠目录名约束实际构建目标。

## 4. 配置合同

### 4.1 最小 C 配置

```toml
schema = 1

[binding]
group = "example.bindings"
name = "demo"
version = "0.1.0"
package = "example.bindings.demo"
output = "generated/demo"

[headers]
language = "c"
include = ["demo/demo.h"]
include_dirs = ["vendor/demo/include"]
defines = { DEMO_NO_LEGACY = "1" }
flags = ["-std=c17"]

[selection]
header_filter = ["demo/**"]
allow_function = ["^demo_.*$"]
allow_type = ["^demo_.*$"]
allow_variable = ["^demo_.*$"]
allow_macro = ["^DEMO_VERSION_.*$"]
exclude = ["function:demo_legacy"]
opaque_types = ["record:demo_context"]

[[native.libraries]]
name = "demo"
kind = "dynamic"

[[rename]]
declaration = "function:demo_open"
scoop = "openRaw"

[policies]
capture_errno_functions = ["function:demo_open"]
gc_leaf_functions = []
closed_enums = []
storage_functions = []
```

未知字段和版本、无效目标条件、非法 identifier／coordinate、无法定位的精确 selector
都报配置错误。仅有正则 allowlist 的空匹配产生 warning；所有根均为空时报错。
同一属性由两条精确 policy 给出不同值时报告两处位置，不按配置顺序覆盖。

`headers.include` 按顺序构成一个 TU 的 include／import 列表，顺序可以影响宏，不能排序。
默认 C 标准为 C11；Objective-C 默认使用 C11 与 ARC。`flags` 是实际参数数组，沿已有
native flags 规则限制 target、sysroot、输入语言、阶段和输出选项；不接受响应文件、插件或
链接参数。define 名按字节排序，值保留原文本；有顺序依赖的宏在一个显式 wrapper header
中表达。

显式 flags 在结构化 include／define 之后按原顺序追加；其中的 `-D/-U` 按普通 compiler
参数顺序生效，`inspect` 显示最终 argv。生成 manifest 使用相同次序，不能在序列化时重新
排序并改变宏语义。

`include_dirs` 接受普通路径及 `{ sysroot = "..." }`，后者复用语言规范 §12.2.1 的
Scoop target sysroot 相对目录规则。普通相对路径在写入生成 `Cone.toml` 时重新相对化到
最终 Cone 根；绝对路径保持绝对，不复制头文件。发行安装不保证含有第三方开发头，使用者
需要显式提供这些输入。

### 4.2 条件、选择和 policy

首期只对 include、宏、flags、libraries 和 selection 提供追加式目标段：

```toml
[[platform]]
when = { os = "windows", env = "msvc-ucrt" }
include_dirs = ["vendor/demo/windows/include"]
defines = { DEMO_WINDOWS = "1" }
allow_function = ["^demo_win_.*$"]
libraries = [{ name = "kernel32", kind = "default" }]
```

`when` 使用共享 target catalog 的 os／arch／env 语义。基础配置之后按声明顺序追加所有
匹配段，数组保留次序；宏重名且值不同时报错，相同值合并。不会把几个目标合成一个 TU。
非匹配段不读文件、不探测工具。不同 API 面需要大量不同 policy 时使用不同配置文件，首期
不引入任意深度合并或配置继承语言。

`header_filter` 使用 include 搜索目录相对的逻辑路径、`/` 分隔及 glob；直接指定的头文件
保留其 include 拼写。一个文件匹配多个搜索根时取最先声明的根。匹配只用于选择，不检查
文件的 owner、inode 或 symlink 来源。省略 filter 时，仅把显式 include 的文件作为根候选。

allow 正则匹配完整 C spelling，不匹配 USR 或宿主路径。先过滤根候选，再应用精确
`exclude`，最后计算类型闭包；显式排除是可见的选择结果，不是失败后的自动降级。
rename、policy 或适配配置引用一个被排除的声明时报错。引用闭包不能因 header filter 被
截断；若不希望展开某 record 的字段，使用 `opaque_types` 指定指针视图。

精确 macro／vararg／UTF-8／callback 适配条目和 Objective-C selector／constructor 是额外
的显式根，不受只用于正则候选的 header filter 限制。rename 与普通 policy 不扩大根集合，
必须指向已选根或其类型闭包；引用实际存在但不在该集合中的声明同样报告配置位置。

policy 只作用于列出的 exact declaration。首期不提供全局 leaf、全局 nonnull、所有 enum
closed 或自动 ownership 推断选项。`inspect` 同时显示原始事实与 policy 改动。

### 4.3 窄范围适配配置

P2 增加以下配置，不提供可以执行任意 C／Scoop 代码的配置 DSL：

```toml
[[macro_function]]
declaration = "macro:DEMO_CLAMP"
scoop = "demoClamp"
parameters = ["int", "int", "int"]
result = "int"

[[fixed_vararg]]
declaration = "function:demo_log"
scoop = "demoLogInt"
variadic_parameters = ["int"]

[[utf8_input]]
declaration = "function:demo_open"
parameter = "path"
scoop = "openUtf8"

[[sync_callback]]
declaration = "function:demo_visit"
callback_parameter = "visitor"
context_parameter = "user_data"
callback_context_index = 0
scoop = "visit"
```

C 类型文本必须能直接用于 probe 的参数／结果声明；复杂 function pointer 和 array
declarator 使用头文件中已有 typedef 名。由 Clang 检查这些类型，不在配置解析器内实现 C
类型语法。固定 vararg 的原有固定参数从声明取得，配置仅增加尾部参数。

`utf8_input` 明确承诺输入为 UTF-8 NUL 字符串且只在同步调用期间借用；生成
`withCString` 包装，不改变 raw pointer API。其他 encoding、返回 buffer 释放规则和错误
转换先留在手写 Cone，不从一个字符串参数扩展成通用 marshaling 系统。

`sync_callback` 明确承诺 native 在返回前结束全部 callback、不会保存 function/context，
且 callback 失败后的全零返回值能被该 API 接受。它只支持一个 callback/context 对，参数
必须完整匹配 §7.5 的形状。registered、one-shot 异步 ownership 与取消规则不使用该配置。

## 5. 目标和 Clang 环境

### 5.1 三种角色分别确定

1. **宿主 CIndex**：在运行 bindgen 的宿主上加载，首期使用工具链固定的 LLVM／Clang
   22.1 系列构建；具体 patch 取维护脚本当前版本。它不是目标平台的 DLL／dylib。
2. **解析目标**：取 `scoop-target-data` 中的 canonical target、LLVM triple 和 native
   environment。尤其不能把两种 MinGW CRT 的共同 LLVM triple 当作完整目标。
3. **native compiler**：沿 `scoop-toolchain` 选择，验证与日后 Cone build 使用相同的目标
   配置；Darwin `.m` 使用 Apple Clang，Linux 可以是现有 GNU／musl compiler。

解析器与 native compiler 不要求版本字符串相同。支持范围是 Clang 能解释、且生成的
header checks 能在实际 native compiler 下成立的声明。编译器专属宏导致签名、布局或
常量不同，必须在生成或候选编译时报错；不能默认两套预处理分支相同。

### 5.2 唯一解析配置

从已经选择的 C toolchain 取得 SDK、CRT include 顺序、系统目录、deployment baseline，
组合显式配置形成一份 `HeaderParseOptions`。它包含 target、语言／标准、resource dir、
ordered includes、defines、flags；CIndex probe 与所有生成 native TU 使用同一份有效
header 配置。toolchain 的目标／SDK 信息通过一个按需投影取得，不把 compiler object
命令反向拆字符串，也不复制另一套平台发现逻辑。

相同配置指相同的有效语义，不要求 MSVC 风格与 GNU 风格 argv 字面相同。标准、include、
define 等按已选 driver 方言投影；无法同时用于 CIndex 和实际 native compiler 的显式选项
报错，不能只在一侧忽略。

当前公共 API 没有直接提供全部 CIndex 所需参数。实现时在 `scoop-toolchain` 增加这个
具体投影，并复用 native command 的已有组成函数；不新增通用工具执行协议。读取这些
参数不准备最终 linker、C++ runtime 或未使用的系统库。

Darwin 的最低部署版本采用当前 Scoop toolchain 已确定的值；首期不加入 bindgen 独有的
`deployment_target` 覆盖。否则解析认为可用的 API 可能高于实际生成对象的最低系统版本。
如果以后允许用户改变最低版本，应先扩展统一构建合同，再由 bindgen 消费。

Windows 的 Microsoft SDK／toolset 与 MinGW CRT 配置头沿各自 provider 提供，不能回退
到宿主系统头。Linux musl 使用所选 musl headers，不能只更换 triple 而仍解析 glibc。
隐式 `CPATH` 等环境搜索输入按现有 native 环境规则处理，不能只影响 CIndex 一侧。

### 5.3 CIndex 生命周期与错误

使用 `clang-sys` 的 runtime loading，在进程内固定一个已选 CIndex library；RAII 管理
index、TU、CXString、token、evaluation result 等资源。`HeaderIr` 不保留依赖 TU 寿命的
裸 cursor。`inspect --help` 等不需要解析的操作不加载动态库。

启用 detailed preprocessing record，收集声明、宏及实际 source location。所有 Clang
error/fatal 阻止生成；warning 保留位置和原信息。未选中声明可以不进入映射，但不能忽略
整个 TU 的语法错误。首期不读取不稳定 JSON AST，也不预建 LibTooling helper。

默认动态库放在 `<SYSROOT>/libexec/llvm/lib/`，Windows 为其 `bin/` 下的 CIndex DLL。
加载实际文件失败时报告所选路径与 loader 错误，不静默尝试任意系统 libclang。
`--libclang` 也必须满足本工具用到的实际 CIndex 符号集；支持检查是普通 API 兼容判断，
不参与生成 Cone 身份。

## 6. 内部数据、选择闭包与命名

### 6.1 数据流

```text
Config + Target + HeaderParseOptions
                  |
                  v
             CIndex translation unit
                  |
                  v
        HeaderIr + source diagnostics
                  |
                  v
     Selection + type closure + naming
                  |
                  v
   BindingPlan: Direct / Shim / Unsupported
                  |
                  v
   Scoop source + native source + Cone.toml
                  |
                  v
        ordinary release library build
                  |
                  v
             compare or publish
```

`HeaderIr` 只描述头文件中已解析的事实；`BindingPlan` 描述可生成的具体绑定。发射器不得
重新推断 calling convention、ownership 或 layout，也不能遇到未知类型时拼一个近似类型。

### 6.2 必需的数据形状

| 数据 | 必须保存的信息 |
| --- | --- |
| 声明 | typed id、原 spelling、source span、canonical declaration、linkage、实际定义位置 |
| `CType` | scalar kind／宽度／符号、qualified pointer、array、function prototype、record／enum 引用 |
| record | `Incomplete` 或完整的字段序列与 size／alignment／bit offsets；union、bit-field、array 属性 |
| function | 固定参数或 variadic、结果、calling convention、storage、inline、实际属性 |
| variable | 完整 object type、外部存储、const／volatile／atomic 与 TLS kind |
| macro | object／function-like、参数、token 与定义位置；求值结果单独用成功／失败变体 |
| Objective-C | interface／protocol／category 引用、完整 selector、class／instance、accessors、可用性与 ownership 属性 |
| plan | 确定的公开名字、完整 Scoop 签名、原 symbol 或完整 shim 方案、使用的 policy、必要检查 |

record、enum、typedef、function、variable、macro 和 Objective-C method 使用不同 id
类型。id 在本次导入会话内唯一；跨类型引用使用封闭枚举，不用字符串 FQN 回退。USR／
canonical cursor 只用于本次 Clang 声明去重；不成为 Scoop 实体 id 或持久产物身份。

不完整 record、无 prototype 的函数和取不到 layout 的类型使用显式变体及原因，不能用
`size = 0`、空字段或缺失返回类型伪装成完整定义。自引用 pointer 只引用 record id，不展开
无限树；按值递归和 typedef 错误由 Clang 诊断，选择遍历用 visited set 结束循环。

### 6.3 根与引用闭包

根由 §4 的选择规则确定。函数参数／返回值、record 字段、typedef、函数指针签名需要的
类型按引用加入闭包；只加入实际需要的 declaration，不把依赖 header 的全部函数一起导出。
出现在公开签名里的生成类型必须同样 public；不能用 internal 类型拼成非法 public API。

同一 tag 的前置声明和定义合并，保留定义位置；`typedef struct foo foo` 使用一个 Scoop
类型名字，不生成同名 alias 环。显式 opaque 的 record 在 pointer 边停止字段展开。
完整 record 无法直接表达时可以产生明确的 opaque pointer 视图，但这不提供按值类型；
任何实际 by-value 使用必须单独进入 Shim／Unsupported 决策。

只要一个保留的根为 Unsupported，`generate/check` 整体失败。使用者可以通过精确 exclude
缩小输出，但工具不能“尽量成功”并悄悄丢掉 API。没有被选择且不可达的声明不产生逐项噪声。

两个配置独立生成同一 C record 会产生两个不同的 Scoop nominal type。首期不按同名、
USR 或相同布局把它们跨 Cone 合并；需要共享这些类型的 API 先放进同一个 binding Cone。
将来有跨库消费者需要外部类型映射时，再设计显式依赖与 exact type 引用。

### 6.4 名称稳定性

默认保留 C spelling；Scoop keyword 用语言已有的反引号 identifier 表示，非法字符通过
确定的转义表示。参数无名时按位置生成 `arg0`、`arg1`；同作用域参数重名按原位置消歧。
普通公开 declaration 冲突要求精确 rename，不通过遍历次序分配数字后缀。

声明 selector 为 `function:name`、`record:tag`、`typedef:name`、`enum:tag`、
`variable:name`、`macro:name`。匿名 nested record 以最近具名 owner 和字段路径定位；
完全匿名且只提供常量的 enum 不强造公开类型。不能取得稳定公开名字的根要求配置 rename。

opaque record 和 Objective-C object 的 raw alias 使用 `<原类型名>Handle`，类型为
`Ptr<Unit>`，每个使用位置再决定 Option。managed Objective-C wrapper 使用原 class 名；
这两种名称冲突也走普通 rename 诊断，不以相同拼写隐藏 alias 与 owner 的区别。

Objective-C selector 使用 `objc:Class/-method:part:` 或 `objc:Class/+method:part:`，
property 使用 `objc-property:Class/name`。默认 Scoop method 名保留全部 selector 片段并
用下划线连接；与原下划线、property 或继承方法冲突时要求 rename。

shim symbol 由 binding coordinate、完整 target、声明 kind、原 selector 和适配 kind 的
可逆 ASCII 编码组成：各分量的 UTF-8 bytes 编成小写十六进制，以 `_` 分隔，统一以 `sbg_`
开头。不使用 USR、地址、时间戳或当前根序号；增加另一个根不改变旧 symbol。符号较长可
接受，无需再建内容指纹或符号映射服务。

## 7. C 类型与声明映射

### 7.1 标量、指针和 typedef

| C 类型／位置 | Scoop 表示与决策 |
| --- | --- |
| `void` result／`(void)` 参数表 | `Unit`／空参数表；不产生一个 `Unit` 参数 |
| `_Bool` | `Boolean`，保持目标 C Boolean ABI |
| 8／16／32／64 位有符号整数 | `Int8`／`Int16`／`Int`／`Long` |
| 8／16／32／64 位无符号整数 | `UInt8`／`UInt16`／`UInt`／`ULong` |
| plain `char`、`long`、`size_t`、`wchar_t` | 按所选 target 的 canonical 宽度和符号性映射；`wchar_t` 不映射 Scoop `Char` |
| `float`／`double` | `Float`／`Double` |
| 直接可表达的 `T *` | nonnull 为 `Ptr<T>`；nullable／unspecified 为 `Option<Ptr<T>>` |
| `void *`、opaque record pointer | `Ptr<Unit>` 或其具名别名，按同样规则决定 Option |
| 函数指针 | 完整、cdecl、非 variadic 时为 `FunPtr<F>`／`Option<FunPtr<F>>` |
| typedef | ABI 先展开；保留有用的公开 typealias，不凭名称改变 signedness、Boolean 或 ownership |
| long double、128 位、complex、vector、atomic 按值 | 首期 Unsupported，不降精度、不替换为同尺寸整数或 byte struct |

多级指针逐层处理 nullability，例如未注解的 `int **` 为
`Option<Ptr<Option<Ptr<Int>>>>`。参数 array 使用 Clang 已完成的 parameter adjustment；
不能把 record 中的 inline array 也当作 pointer。

`const`／`restrict` 保存在文档与 IR，raw 层没有只读 pointer 类型。mutable record field
仍生成 Scoop 的 `val` 字段；需要修改 native 存储时通过显式 unsafe pointer 操作和
`@InteriorMutable` 合同，不能生成 Scoop 不允许的 struct `var`。

对 volatile／atomic pointee 或其他无法表达的 data pointee，可以保留 opaque pointer
并明确禁止由生成 API 直接 load/store；这只承诺地址可传递。地址空间、特殊 pointer
representation 无法映射时仍拒绝。不能把未知 calling convention 的函数指针降成 data
pointer，也不能靠 `Ptr<Unit>` 隐藏一个不合法的 by-value 参数。

### 7.2 enum 与常量

默认 C enum 为底层整数 alias 与各 enumerator 的 `public const val`；底层类型、负数与
大无符号值使用 Clang 的精确结果。相同数值的多个 C 名字都保留，flags 的组合值仍合法。
匿名 enum 只发射常量。这允许库返回当前 header 未列出的值。

`closed_enums` 才生成 Scoop 整型 enum。重复数值选原声明顺序中的第一个作为 variant，
其余名称发射指向该 variant 的同类型 const；所有原 enumerator 名仍通过顶层 const 保留。
空集合、不可表示数值或冲突名字报错，不生成重复 variant。该 policy 要求实际 native
输入只产生声明的值，不能用 flags 或命名习惯推断。

### 7.3 record 与布局

Direct record 必须是非空 complete struct，字段递归可表示，没有 union、bit-field、inline
array、flexible array、volatile／atomic／ARC ownership 字段或其他未支持表示。以目标
scalar layout 和字段布局计算 Scoop 候选，要求 size、alignment、每个 field offset 全部
等于 Clang 结果。

布局计算复用 `scoop-lir` 已有的 `StorageLayoutCursorV1` 与 CLayout placement policy，
只输入已确定的字段 geometry；不创建 Scoop 实体、不重放 HIR 的语言语义。自然布局优先；
必要时检查 `aligned/packed ∈ {0,1,2,4,8,16}` 的有限组合，等价组合按固定次序选一个。
找不到精确组合时转为 opaque／storage adapter，不能追加猜测的 padding 字段。

字段保留原始 scalar 类别和顺序；同 size/offset 不足以把 float field 换成 integer。
by-value record 的调用约定继续交给 Scoop 编译器与目标 C compiler，不在 bindgen 中
实现 SysV／Win64／AArch64 ABI classifier。

对于会暴露可写 typed pointer 的 record，生成 `@InteriorMutable`，包括字段引用闭包中
实际受到该可变访问影响的嵌套 record。只按值使用或仅通过 const pointer 观察的类型不因
来自 C 就一律标注。完全 opaque 的 record 不生成伪造的字段或零尺寸 nominal type。

### 7.4 函数、global 与 TLS

Direct function 必须有 external linkage、固定 prototype、默认目标 C calling convention，
且每个参数／结果满足本节映射。`f()` 的无 prototype 声明不能当作 `f(void)`；显式 C
variadic、非默认 convention、static inline 分别按 §8 适配。C symbol alias／asm label
不能可靠转成 Scoop 逻辑 symbol 时，用具名 C wrapper 或报错，不猜测对象文件的前缀。
带平台 availability 的 C 声明也按 §9.5 的同一 deployment 规则处理，不只检查 Objective-C。

| 外部存储 | 输出 |
| --- | --- |
| 非 volatile／atomic 的 const global | `@Extern val` |
| 普通可写 global | `@Extern @Global var` |
| 可写 TLS | `@Extern @ThreadLocal var` |
| const TLS | 在 native getter 中读取，每次访问保持调用线程的 TLS 语义 |
| 可表达 scalar 的 volatile global／TLS | native getter／setter，实际读写保留 volatile |
| atomic object、不可表达的 global 按值 view | Unsupported；可另选择有明确签名的原库访问函数 |
| header-local static object | 不导出；显式选择时报 Unsupported，避免为每个 TU 制造独立状态副本 |

volatile getter／setter 不扩大为通用内存访问 API；`@InteriorMutable` 不能替代 volatile
或 atomic。raw extern 默认使用空 `lib`，`native.libraries` 明确记录链接需求，由现有
链接规则绑定实际符号；不根据 header 目录猜测库名。

### 7.5 callback

raw 层只提供真实 `FunPtr` 签名；不从 callback 的存在推断同步、同线程或无 GC。
静态 `::function` 继续受语言规范 §13.10 的 `@NoGC`、同步同线程限制；异步／foreign-thread
使用者必须使用已有 `ForeignCallback` 协议。

配置 `sync_callback` 时，生成器检查 callback/context 参数都存在、未重复使用，callback
非 variadic 且全部参数／结果可 Direct，context index 指向 `void *`。配置明确将这一
callback 位置的 context 声明收窄为非 null：native 必须原样回传传入的 cookie。
若原 typedef 的未注解 context 默认可空，为该函数位置发射独立的具体签名，不修改其他
调用位置的 typedef；raw 声明和便利 wrapper 使用同一个收窄后的签名。

wrapper 注册 Reusable callback，传入 `function + context`，在 native 返回且全部 callback
完成后读取失败，并在所有出口释放唯一注册 ownership。callback 抛出的异常由现有 runtime
保存，wrapper 在回到 managed 侧后重抛；native 返回值和捕获的 errno 先保存在局部值中。
native 可能保存地址、后台调用或不接受失败后的全零结果时，不生成该便利 wrapper。

不生成动态 executable trampoline，也不为无 context API 建全局 closure slot。非 cdecl
函数指针不能通过给一个具名函数加 shim 就获得通用适配，首期保留 Unsupported。

## 8. C shim、宏和 ABI 检查

### 8.1 具名函数适配

所有 shim include 原始 header，调用真实的声明，不复制一份手写原函数 prototype 来蒙混
通过编译。生成 symbol 对 Scoop 暴露默认 C calling convention，原调用由目标 C compiler
按 header 的精确类型编译。

- `static inline`：固定参数／返回 wrapper，header 内的实现留给 C compiler。
- 非默认 calling convention 的具名函数：目标 compiler 可以表达时，生成默认 convention
  wrapper；无法表达则 Unsupported。Win64 中被 Clang 规范化为相同 ABI 的注解不强造适配。
- function-like macro：按配置的类型接收参数，在 C 函数体内展开宏。调用者的实参先作为
  函数参数求值一次；这不是保留任意 C 宏语法的逐 token 替换。`#`／`##`、type-token、
  statement macro 等不符合表达式 wrapper 的形状时拒绝。
- fixed vararg：普通固定参数加配置的尾部参数，调用原 variadic 函数；默认整型提升和
  `float → double` 由 C compiler 处理。格式字符串与实参的业务匹配仍是 raw 调用者契约。

不能仅因 header 标为 `inline` 就假定存在外部 symbol。没有函数体也没有可链接声明的
header-local function 必须报错；工具不制造空实现。

### 8.2 不可直接表示的按值聚合类型

`storage_functions` 是显式 opt-in。它允许 complete、非零尺寸、可普通复制的 C struct／
union 使用字节存储适配，主要覆盖 fixed array、union、bit-field 和较高 alignment。
含 flexible array、volatile／atomic、非平凡语言对象或取不到固定 size 的类型仍拒绝。

转换仅发生在函数外壳：Scoop 的输入是至少 `sizeof(T)` 字节的有效 C object representation；
shim 将其复制到真正的 native `T` 局部变量，再按值调用原函数。结果先存为 native `T`，
再复制到调用者提供的输出区。以下为结构示意，实际 symbol 使用 §6.4 的完整编码：

```c
#include <stdint.h>
#include <string.h>
#include "demo/demo.h"

void sbg_demo_transform_storage(
    const uint8_t *input,
    uint8_t *output
) {
    demo_packet value;
    memcpy(&value, input, sizeof value);
    demo_packet result = demo_transform(value);
    memcpy(output, &result, sizeof result);
}
```

这里的 `demo_packet` 可以包含 Scoop 当前不支持的 inline array。Scoop 签名显式使用
`Ptr<UInt8>`，公开名字带 `WithStorage` 后缀或配置的 rename，不能继续宣称接受一个 Scoop
`demo_packet` 值。函数中的完整输入复制先于原调用，输出复制晚于返回；原函数没有获得
调用者输入 buffer 的地址，因此保留 C 的按值传参语义。

生成相应 size／alignment 常量；字节区自身不要求 native `T` alignment，因为实际解引用
发生在正确对齐的 native 局部变量上。buffer 长度、有效位型和内含 pointer 的生命周期是
明确的 unsafe 前置条件。全零 buffer 不自动代表有效值，未初始化内容不允许作为输入；
输出 padding 不承诺稳定，不把 object representation 当成序列化格式。

调用者可以通过 `MutableArray<UInt8>.withDataPointer` 借用输出区，并通过原库初始化
函数或一个手写 native adapter 取得合法输入。bindgen 不生成一套 native allocator、
uninitialized owner 或任意 union 活动成员状态机。此方案不用于 callback 的反向 ABI。

### 8.3 对实际 header 的检查

每次生成对已消费的事实发射必要 C11 检查：

- 完整 target 的 OS／arch／env 宏，以及参与映射的整数宽度／符号性与 pointer 宽度。
- Direct record 的 `sizeof`、`_Alignof`、`offsetof` 及字段类别。
- 选中 enum 的底层表示及已发射常量；选中 object-like 数值宏的值。
- 函数和变量的实际类型，使用原 C 类型构造的 `_Generic`／typed declaration 检查；不能
  仅靠“能隐式转换的函数调用”接受有符号性或 pointer 层数变化。

检查原 C prototype 时保留原来的 C type identity；Scoop ABI projection 则按位宽及 scalar
类别表达。`long` 和 `long long` 不能因为同宽就在 C 类型兼容检查中互换；也不能拿任意
canonical `int64_t` spelling 代替本应核对的原 C prototype。

这些检查用于连接“Clang 解析的 header”和“普通 native build 使用的 header”两个真实
边界。编译器继续负责 Scoop 声明自身的 C layout 与物理调用 ABI，bindgen 不在每个 stage
重新验证它们。header checks 与 shim 同一个 TU 编译，不额外执行目标上的布局探针程序。

原 header 改动后，即使生成文件尚未更新，普通构建也会因其真实 include 输入更新而重编译
该 TU。检查能发现已消费事实的变化，但不会因此自动导入新增 API；发现完整输出变化使用
显式 `bindgen check`。不把这些检查扩展为对原库二进制的 ABI 认证。

### 8.4 object-like 宏

只处理 allowlist 中仍然有效的宏定义。对数值表达式在包含原 header 的 synthetic TU 中
增加有实际 C 类型的 probe declaration，交给 Clang constant evaluation；不实现自己的
C 常量求值器，不执行宏可能包含的函数调用。

8～64 位整数和能精确往返的 binary32／binary64 值生成 `const val`。负零保留；无法由
当前常量表达式精确保存的浮点值转为明确的 typed getter 或 Unsupported，不把 long double
悄悄舍入。`sizeof`、枚举引用及 target define 的数值自然属于本次 target 输出。

窄字符串 literal 宏默认生成返回静态字节地址及长度的 native getter，长度不含最后的
终止 NUL，保留内部 NUL；不把 C 地址写成 Scoop const，也不默认声明字节编码为 UTF-8。
wide string、指针运算和运行期表达式需要明确适配，首期不自动转换。

依赖 `__FILE__`、`__LINE__`、`__DATE__`、`__TIME__`、`__COUNTER__` 等调用位置／时刻的
宏不能作为生成常量；直接或通过其他宏展开到它们时都报告原因。求值失败或 CIndex 不能
提供完整结果时，该选中宏 Unsupported，不因为其他声明可以生成就忽略它。

### 8.5 errno 与 GCLeaf

Direct 函数按配置生成 `@Extern(captureErrno = true)`，返回 `(R, Int)`。不默认解释
成功条件，也不自动抛异常。

对有结果转换、复制或清理步骤的 shim，不能只在最外层 extern 开启 errno 捕获：清理可能
覆盖原值。应在 shim 内原调用前清零 errno，原调用返回后立即保存，再进行其他适配；通过
额外的 `int32_t` 输出槽交回 Scoop，普通 `@Unsafe` wrapper 组合成 tuple。该路径不再给
shim extern 叠加 `captureErrno`，避免返回两份含义不同的快照。

`gc_leaf_functions` 只对具体声明启用，要求整个生成 shim 及原函数调用链满足语言规范
§13.4.1。与 managed callback 便利层冲突时直接报配置错误。Objective-C facade 不接受
此 policy；不能由无 managed 参数、函数较短或 Clang `pure/const` 属性推导 GCLeaf。
首期不提供自动 `GCLeafFunPtr` 类型改写。

### 8.6 最小 Direct 输出示例

原 header：

```c
#include <stdint.h>

typedef struct demo_point {
    int32_t x;
    int32_t y;
} demo_point;

demo_point demo_shift(demo_point point, int32_t dx);
```

对应原始 Scoop 声明：

```scoop
package example.bindings.demo

@CLayout
public struct demo_point(val x: Int, val y: Int)

@Extern(name = "demo_shift")
public fun demo_shift(point: demo_point, dx: Int): demo_point
```

该签名不需要 bindgen 的 storage adapter；编译器已有 bridge 负责实际聚合参数／返回。
生成的 native TU 只 include header 并核对映射事实。consumer 仍需按普通 manifest 声明并
提供 `demo` 链接库。

## 9. Objective-C 消费端

### 9.1 选择和支持范围

仅接受 Darwin 两个目标及 `headers.language = "objective-c"`，生成 `.m`，由实际 Apple
Clang 使用 ARC 编译。C 声明仍使用前述选择与映射；方法通过完整 selector 精确选择，选择
一个 class 不等于导出该类、父类和 category 的全部方法。

```toml
schema = 1

[binding]
group = "example.bindings"
name = "foundation"
version = "0.1.0"
package = "example.bindings.foundation"
output = "generated/foundation"

[headers]
language = "objective-c"
include = ["Foundation/Foundation.h"]

[objective_c]
classes = ["NSLocale"]
methods = ["objc:NSLocale/+currentLocale", "objc:NSLocale/-localeIdentifier"]
properties = []
managed_wrappers = true

[[native.libraries]]
name = "Foundation"
kind = "framework"
```

receiver 和类型闭包按需补齐；例如上述 `NSString *` 结果需要一个拥有 handle 的 wrapper
类型，但不因此导入 NSString 全部方法。class method、instance method、property accessors
各有确定 native symbol。property 展开实际 getter／setter，尊重 readonly 与自定义 accessor。
category 方法按 receiver 合并并记录原 category 来源，冲突原型报错。

`managed_wrappers` 默认 false；不启用时仍生成 raw facade、release 和错误 buffer free
入口，并说明 +1 由 raw 调用者负责。开启后才生成本节的 owner class 与 managed 异常转换。

protocol-qualified object pointer 保留 capability 信息，首期不生成 Scoop protocol 实现。
lightweight generic 只保留静态说明；`instancetype` 按实际 receiver 的静态 class 处理。
不自动建立 Foundation 的完整 Scoop 继承树，不做隐式 downcast。

### 9.2 facade 形状

每个方法生成固定 C ABI facade，使用头文件里的真实 Objective-C 类型调用
`[receiver selector:argument]`。不从 Scoop 引入 variadic `objc_msgSend`，不手选
`objc_msgSend_stret`，也不增加 runtime dispatch 层。

边界统一为 `int32_t status + 显式结果输出槽 + 错误输出槽`。结果槽类型按每个方法确定；
`void` 方法没有结果槽。错误槽固定包含 `uint8_t *bytes` 和 `uint64_t length`，成功时为
空。它们生成普通 `@CLayout` 类型，不是新 runtime ABI。

对象输出槽始终使用 nullable pointer storage，只有成功且满足 nullability 后才被 wrapper
转换为 nonnull 对象。其他无法合法全零初始化的 GC-free 结果使用既有
`MaybeUninit<R>`，通过 `addressOf(slot).cast<R>()` 提供 native 输出地址；只有 status
成功后才能 `assumeInit`。`MaybeUninit` 本身不进入 C ABI。

每个调用使用 `@autoreleasepool`。外层 `@try/@catch` 覆盖消息发送、结果转换、ARC
临时变量清理及 pool drain，不能把可能抛异常的 pool 清理放在 catch 范围外。
生成 `objc_flags` 显式包含 `-fobjc-exceptions` 与 `-fobjc-arc-exceptions`，解析侧使用
同一选择；仅有 `-fobjc-arc` 不足以保证异常路径的 strong 局部值清理。配置不能关闭这两项，
但不改变其他 Cone 的默认 Objective-C 编译选项。

### 9.3 object ownership

raw 对象 handle 为具名 `Ptr<Unit>` alias；`id`、class-specific object、protocol object、
`Class`、`SEL` 保留不同名称，但 alias 不构成独立 nominal 类型。`Class`／`SEL` 是非 owning
token，不执行 instance 的 retain／release。Core Foundation 类型仍按 C API 处理。

- 输入对象在调用期间借用，facade 以静态类型与 ARC strong 局部值保活；不消费 Scoop 的
  ownership。nullable 为 nil，未注解同样保守视为 nullable。
- 一切需要离开调用的对象结果统一为 **owned +1**。先把真实方法结果交给 ARC 管理，再由
  `__bridge_retained` 形成一份新 ownership；方法族原有的 +0／+1 规则由 Clang 处理。
- borrowed pointer 不跨出 pool。即使原方法返回 borrowed object，facade 也先保留它再
  返回；不能把短期借用地址装进一个稍后关闭的 wrapper。
- release facade 用 `__bridge_transfer` 消费恰好一份 +1，内部同样有异常边界；不将
  Objective-C object pointer 存入 Scoop Throwable 等待 GC 清理。

native 结果先保留在局部待交付值中，完成正常清理和 pool drain 后才发布输出槽。成功前
发生异常则清理尚未交付的 +1；清理本身也在 native 异常边界内处理。已经发生的原 API
副作用不回滚。失败结果槽不得含有一份未说明责任的 ownership。

`managed_wrappers = true` 为实际选中的 object type 生成普通 `final class : Closeable`：

1. 私有 constructor 接管一份 +1；raw pointer 到 owned wrapper 的入口明确为 unsafe
   `takeOwnership`，不提供隐式转换。
2. owner 保存 `Option<Ptr<Unit>>`。调用前检查未关闭；`close()` 先取走 handle 并置为
   `None`，再调用 release facade；重复 close 无操作，native 清理失败也不重复消费旧值。
3. raw 返回成功到 managed owner 建立之间，任何可恢复异常都必须释放尚未接管的 +1。
   同一次结果不能自动创建两个独立 owner；需要两份 ownership 时显式 retain。
4. 不生成 `release {}`，不宣称 `Closeable` 自动提供线程安全。使用者按原 API 的线程要求
   串行使用／关闭同一个 owner；临时 ARC strong 值保证合法的同步重入期间对象仍被保活。
5. raw pointer、buffer 或其他外部生命周期仍暴露在签名中时，wrapper 保持 `@Unsafe`。
   仅完成 ownership 包装不构成对整个 framework API 的安全性推断。

所有方法按普通 NativeSafe 边界调用，复用现有 GC 协调。默认显式 `use/close`，遗漏关闭
会泄漏；不以 GC hook 自动调用任意 Objective-C dealloc。

### 9.4 初始化与消费参数

任意 `ns_consumed` 参数、`ns_consumes_self` receiver 和已拥有对象上的 `init` 调用
Unsupported。Scoop 没有对应的通用 move-only 状态，不能在异常后继续暴露已经被消费的 owner。

唯一首期构造适配是配置明确选择的工厂：

```toml
[[objective_c.constructor]]
class = "NSURL"
initializer = "initWithString:"
scoop = "createUrl"
```

该条目也是一个 selection root，并把 class 加入闭包。facade 在一次调用中执行
`[[NSURL alloc] initWithString:...]`，ARC 处理替换 receiver、nil 和异常；只有最终初始化
结果形成对外 +1。`alloc` 的未初始化结果不暴露给 Scoop，initializer 不能另生成为普通
instance method。参数本身若有 consumed ownership 仍拒绝，不因是工厂而放宽。

### 9.5 nullability、可用性和异常

`_Nullable`／unspecified 对应 Option；`_Nonnull` 的对象结果实际为 nil 时，facade 返回
contract-violation status，不把 nil 交给 nonnull `Ptr`。头文件未给出的 nonnull 事实不由
类名、method family 或文档措辞推断。

按已解析的 deployment baseline 检查 availability。显式选择 unavailable、已 obsoleted
或 introduction 高于该 baseline 的方法时报错；闭包中不需要的方法不受影响。
deprecated 可生成并给 warning。首期不自动 weak-link 或插入 runtime availability 分支。
类、方法及所需参数类型的可用性都需要成立。

status 取以下固定值；这是当前生成 Cone 内部的 facade 合同，不是 Scoop runtime 的错误码：

| status | 意义与 wrapper 行为 |
| --- | --- |
| `0` | 成功，读取结果并接管相应 ownership |
| `1` | 捕获 Objective-C 异常，复制文本后抛普通 `ObjectiveCException` |
| `2` | nonnull 等声明契约违反，抛普通 `ObjectiveCContractException` |
| `3` | 无法取得错误文本／native 清理失败等 facade 失败，使用固定 managed 消息 |

异常 name/reason 的文本在 native 侧复制为 malloc UTF-8 字节，不能将 NSException 或
autoreleased NSString 地址交给 Scoop。异常转文本本身可能失败，使用嵌套 native catch
和无对象的固定 status 兜底；不递归要求再次构造 Objective-C 错误对象。

Scoop 检查 `length <= Long.MAX_VALUE.toULong()`，用
`String.fromUtf8(pointer, length.toLong())` 复制，并在所有出口通过同一 facade 的 free
helper 释放 buffer。空指针只允许配合零长度，直接使用空文本或对应 status 的固定消息；
不能为调用解码器而构造一个 null `Ptr`。文本转换失败也必须释放。
Throwable 只保存 managed String／数值。release 失败时 owner 已 inert，异常只报告清理
失败，不暗示用户可以重试释放相同 handle。

### 9.6 明确后置的 API

首期不接受 Blocks、delegate 自动实现、escaping callback、对象 out/inout 双重指针、
Objective-C++ 或 Swift-only 声明。特别是 `NSError **` 不能直接把 `void **` 强转成 ARC
的 autoreleasing slot，再把 pool 内对象裸地址返回。

后续的 `NSError **` 适配应先在 `.m` 内使用真正的局部对象槽，将每个对外对象转成 +1，
raw 层保留原返回值与 error 对象；只有明确 success policy 才转换为 throw。
noescape Block、registered delegate 和异步 completion 分别需要具体消费者及完整的
native owner／copy／unregister／quiescence 合同，不合并成通用 callback 配置。

## 10. 生成、验证和发布

一次 `generate/check` 执行以下顺序：

1. 读取配置并确定目标、选择规则和输出位置；普通配置错误在加载 Clang 前报告。
2. 按输出目标取得普通文件锁。`inspect` 不改输出，不需要该锁。
3. 解析一个 synthetic TU，按需执行有限的类型／常量 probe；形成完整 IR、选择及 plan。
4. 所有保留根都可生成后，在最终目录的同级临时目录写完整候选。使用固定的路径布局、
   UTF-8、LF 和确定排序；原始头文件的 include 顺序保持不变。
5. 对生成的 Scoop 文件运行已有格式化器，再调用普通 `scoop build <candidate> --release`
   验证 library；显式传入 target、sysroot、cc／native-sysroot 及目录外的 target-dir。
   不先重复调用 `scoop check`，也不再单独重编译同一 native TU。
6. build 成功后移除候选中的验证副产物，仅保留受管源码。`check` 比较并报告；`generate`
   比较后仅在内容有变化时替换目录，无变化不更新生成文件 mtime。

验证用 target-dir 放在配置目录的 `target/bindgen/<canonical-target>/` 下，日志与编译
结果沿普通本机构建生命周期处理。临时目录与最终目录具有相同父目录和层级，重新相对化的
header 路径在两者中含义相同。候选与最终源码都不嵌入临时路径或生成时间。

检查与格式化通过现有 Scoop 入口完成，不依赖 parser／HIR stage implementation，不直接
拼接 compiler machine request。library build 不产生最终 executable，也不执行目标代码；
真正的原库 symbol 可用性和行为由 §13 的 consumer 链接／运行测试确认。

源码目录只容纳 §3 的受管文件。用户正常构建产生的 `Cone.lock` 与 `target/` 不参与生成
比较，发布时原样保留，通过改名迁移，不扫描缓存内容；bindgen 自己的验证仍使用目录外的
target-dir。存在其他额外手写文件时报告其路径并拒绝替换，要求将手写 API 放在其他 Cone。
这只是避免覆盖非生成内容，不鉴定已知文件的来源。已知生成文件可随重生成更新，已不需要
的 shim／wrapper 文件随目录替换删除。

解析、规划和验证失败都保留旧输出。目录发布复用现有工具的正常替换方式：可用时原子交换；
否则在锁内先将旧目录改名到同级备份，再发布候选，失败则恢复旧目录。恢复也发生 I/O 错误时
报告旧内容的保存位置，不删除它。不新增持久事务日志或缓存修复系统。

首期没有 bindgen 自身的生成缓存；显式生成／check 就是请求重新读取 header。`check`
比较旧源码是它的产品功能，不是普通 build 的缓存命中复核。日常 `scoop build` 只按现有
输入更新规则复用 Cone 与 native 对象，命中时不运行 CIndex、重新检查生成文件或读取 header。

## 11. 诊断与可审查输出

人类诊断同时给出配置位置、原 header 的 spelling／expansion 位置和引用路径，例如：

```text
error[bindgen.unsupported-type]: long double cannot cross this binding
  at include/demo/api.h:42:18
  selected by scoop-bindgen.toml:16
  function demo_measure -> parameter callback -> function result
  help: exclude this function or provide a fixed C adapter with an explicit conversion
```

诊断类别至少区分 config、clang、selection、name-conflict、unsupported-type、
unsupported-abi、layout、ownership、availability、generated-build 和 output。
同一个公共依赖导致多个根失败时报告首个失败事实及受影响根，不把相同原因重复展开成大量
完整类型树。Clang 的 warning 不因已有 bindgen 诊断而丢失。

`inspect` 输出：原声明及来源、目标与有效解析参数、canonical type／layout、引用路径、
选中或排除依据、名字／symbol、Direct／Shim／Unsupported 原因和每项 policy 来源。
opaque 视图要明确写出“仅支持指针传递，无 Scoop 按值类型”，不能只显示成功。

JSON 使用工具本地 `schema = 1`，结果包含 command、target、输出路径、逐项 decision、
诊断和差异路径。它用于测试与编辑器消费，不作为持久 ABI、build 授权清单或可反向加载的 IR。
生成源码仅保留简短原声明与逻辑 header 位置注释；完整绝对路径、实际 argv、构建日志留在
当次诊断，不把整个 SDK 路径和检查报告提交进 Cone。

## 12. 代码组织与仓库接入

新增独立的 `tooling/scoop-bindgen/` Cargo package，按现有外部工具方式使用自己的
`[workspace]`／Cargo.lock。内部按实际责任拆分：

```text
src/
  main.rs, cli.rs, config.rs
  target.rs
  clang/                  # CIndex ownership, collection, probes.
  ir/                     # Header facts and typed declaration references.
  select.rs, names.rs
  plan/c.rs, plan/objc.rs
  emit/scoop.rs, emit/c.rs, emit/objc.rs, emit/cone.rs
  diagnostic.rs
  output.rs               # Candidate build, comparison and publication.
```

通过 path dependency 复用 `scoop-target-data`、`scoop-toolchain` 以及必要的
`scoop-manifest`／`scoop-lir` 纯数据和 geometry API。当前这些 crate 确实会传递依赖
HIR／LIR／slib 数据 crate；“独立工具”不等于零 IR 数据依赖。限制是不能依赖 compiler
stage implementation、初始化 inkwell 或把 CIndex declaration 写入编译器 IR。
不为追求零传递依赖重构整个 toolchain。

新增第三方依赖限于实际需要的 `clang-sys` runtime loading、regex／glob 匹配；CLI、TOML、
serde、临时目录、文件锁沿仓库已有库选择。实现时核对版本、Rust 最低版本与维护状态，
通过 lock 固定；本设计不把 crate feature 名称当成 Clang 22.1 已验证的证据。

`scripts/build_llvm.py` 的当前 distribution components 包含 `clang-cpp`，没有显式
包含 CIndex `libclang`；`scripts/package_llvm.py` 也不会因 runtime 动态加载自动发现
它。因此 C 阶段就需要在维护 LLVM 准备中构建／安装 CIndex，不能等到 Objective-C 才处理。

正式发行时新增 `scripts/build_scoop_bindgen.py` 并接入现有工具组装：命令在
`commands/`，libclang 作为显式 runtime dependency root 加入 `bin-<host>` 的动态依赖
收集，复用已有 builtin headers、RUNPATH／DLL 加载与许可证流程。不另发完整 LLVM SDK。
Windows 的实际 CIndex DLL 文件名以本次构建结果为准，包装路径不能只假设上游 Unix 命名。

第三方头文件仍来自用户／维护输入；不为了 bindgen 恢复所有 base／GUI 包的开发头分发。
生成 `.slib` 沿普通 target 产物格式消费，不带 CIndex，不要求运行 bindgen 才能链接。

## 13. 测试与验收

### 13.1 测试职责

| 层次 | 验证范围 |
| --- | --- |
| Rust 单元测试 | 配置条件、选择闭包、forward declaration、命名、有限布局候选、mapping／plan 的完整结果 |
| CIndex 集成测试 | 真实小 header 的 target 类型、宏求值、source span、属性与诊断；不以伪造 IR 代替所有输入 |
| 少量生成 golden | 最小 C 与 Objective-C 样本的可读源码／manifest；固定排序和必要签名，不保存完整 SDK AST |
| 生成 Cone 的独立 consumer 测试 Cone | 普通 API 行为、数值、开放 enum、字符串、同步 callback 和 close；使用 `scoop.test` |
| 正式 CLI／ABI fixture | 命令转发、配置错误位置、生成失败不发布、check diff、native 编译／链接、跨目标布局和原库消费 |

行为测试放在独立测试 Cone，依赖生成 Cone；不向受管输出中混入手写 tests。普通测试使用
release，共享预编译系统库和 Cone cache。GC stress 只用于 managed callback 保活等定向
GC 测试；需要 profile 组合的 ABI 正确性样本单独组织。

C 专项必须包括：全部整数宽度及 target-dependent `char/long/wchar_t`、多级 pointer
nullability、opaque/self-reference、nested／packed／aligned record、by-value 双向传递、
enum aliases／未知值、mutable global／TLS／const TLS、inline、对象／函数宏、fixed vararg、
errno 与转换组合、同步 callback 抛异常后的释放。Unsupported 按编译器规则组织，包括
long double 按值、无 prototype、非 cdecl function pointer、atomic、flexible array 和
无法表示的布局；不为每个第三方库复制同一组反例。

storage adapter 要用 native 观察确认“按值副本”的修改不会回写输入 buffer，并覆盖 result、
input/output alias、非自然对齐字节区、union／fixed array 和原生初始化的合法位型。
不能只断言生成文本中出现了 `memcpy`。

生成流程覆盖两次字节一致、不变时不改 mtime、真实 header／define／配置变化、缺失实际
include、Clang／候选编译失败保留旧输出、废弃文件移除、普通并发生成与输出目录差异。
缓存测试保持 header、工具与原产物存在，以真实调用次数和结果确认复用。

### 13.2 目标矩阵

目标列表直接取共享 catalog，当前验收集合如下：

| canonical target | C 解析与生成库编译 | consumer 运行 | Objective-C |
| --- | --- | --- | --- |
| `aarch64-apple-darwin` | 必需 | Darwin arm64 | 必需 |
| `x86_64-apple-darwin` | 必需 | Darwin x86_64 或 Rosetta | 必需 |
| `x86_64-unknown-linux-gnu` | 必需 | Linux GNU | 不适用 |
| `x86_64-unknown-linux-musl` | 必需 | Linux musl | 不适用 |
| `x86_64-pc-windows-msvc-ucrt` | 必需 | Windows 对应 CRT | 不适用 |
| `x86_64-w64-windows-llvm-mingw-msvcrt` | 必需 | Windows 对应 CRT | 不适用 |
| `x86_64-w64-windows-llvm-mingw-ucrt` | 必需 | Windows 对应 CRT | 不适用 |

不要求一个宿主具备所有 SDK。各 CI／验证机使用相应工具；缺少某目标环境时如实记录该项
未验收，不能用仅改 `--target` 的 AST 输出代替 native 编译或运行。正式宣布全目标可用前，
上述对应运行项必须完成。Darwin 两架构聚合 ABI、Unix／Windows 的 `long` 与不同 CRT
headers 是重点组合。

### 13.3 真实库与 Objective-C 验收

C 扩展阶段使用 zlib 与 libpng 两个真实库：zlib 覆盖 stream、callback 类型、错误码和宏；
libpng 选择无 `setjmp/longjmp` 穿越 Scoop 的 simplified read/write API，覆盖 opaque
类型、包含 array 的布局和图像 buffer。`png_image` 的 version 初始化等无法通过 opaque
字段完成的操作使用一份明确列出的手写 C adapter，不把它算作自动字段绑定。需要更多复杂
API 时显式 exclude 并记录选中面，不能把一个库的少量函数通过视为完整库支持。
版本与许可证由实施时的可重复输入固定。

Objective-C 使用 Foundation 代表 API及自有小 `.h/.m` 样本，覆盖 class／instance method、
property 自定义 accessor、category、nullable／nonnull、工厂返回替换对象或 nil、borrowed
结果保留、重复 close、接管失败回滚、autorelease pool、异常文本分配失败和 release 异常。
自有样本用计数和同步点观察 ownership，不依赖 GC 恰好触发或 Cocoa run loop 的偶然时机。
Blocks、consumed 参数、裸 initializer、对象双重指针和非 Darwin 目标均有定位明确的反例。

各目标的 ABI fixtures 完成后，还需 consumer 从生成 `.slib` 直接链接运行；不要求删除或
隐藏生成源码来证明这种消费能力。原始日志和逐次报告放在仓库外，文档只保留验证范围、
结果与未闭合问题。

## 14. 实施顺序与退出条件

每步只交付已经闭合的命令能力；尚未实现的 selection 在当前阶段给出明确诊断，不发布
空 shim 或假签名。后续能力不成为 C 首期使用的前置门槛。

| 阶段 | 交付 | 退出条件 |
| --- | --- | --- |
| B0 技术探针 | 固定 libclang 的加载、target 参数投影、CIndex 最小事实读取；同步写入实现规范的命令／职责合同 | 用真实 C header 在已有验证环境取得类型与布局；缺失 API 与工具输入有明确结论，无通用 helper 框架 |
| B1 Raw C | generate/check/inspect、配置、闭包／命名、Direct 类型、global/TLS、const TLS getter、header checks、候选 library build／发布 | 自有 C 库形成生成→编译→链接→运行闭环；七目标对应项完成，生成差异与失败不发布测试通过 |
| B2 C 适配 | inline、宏、fixed vararg、calling convention、storage、errno、窄 UTF-8／同步 callback wrapper | zlib／libpng 选中面和 ABI 组合可用；Unsupported 规则与边界可审查 |
| B3 Objective-C | ARC facade、方法／property／category、+1 ownership、Closeable、受控构造、异常／availability | Darwin 双架构的 Foundation 与自有样本通过，所有正常／失败路径 ownership 闭合 |
| B4 发行 | 构建脚本、宿主 CIndex 打包、许可、安装后命令与用户说明 | 正常安装输入下生成并消费一个 C Cone；Darwin 再消费 Objective-C Cone |

B4 可以在 B2 后先交付 C 工具，不等待 Objective-C callback 能力。Blocks、NSError
convention、delegate／escaping callback 和外部类型复用只在真实消费者出现后另作窄设计，
不预留空配置字段、未实现后端或兼容转发层。

实现前对文档作以下同步：实现规范 §2.7 登记 bindgen 外部命令、生成／验证职责，§1 与
发行设计补充 CIndex runtime dependency；必要的 toolchain 参数投影仍属于该规范的目标
选择职责。语言规范、runtime 规范和 `.slib` 格式在本设计范围内无需改变；若实现发现冲突，
先修订设计／对应 spec，不通过生成器特例绕过。

## 15. 实施前仍需实测的具体事项

以下是尚未执行的技术验证，不是需要先建立的新机制，也不表示已经验证成功：

| 事项 | 需要取得的证据 | 不满足时的处理 |
| --- | --- | --- |
| clang-sys 与 22.1 CIndex | 各宿主实际 library 名、可加载依赖、所需符号和最小 parse | 修正固定构建／加载路径；缺符号明确报错，不换 JSON AST 协议 |
| native 参数投影 | GNU／musl、Windows 三 CRT、Darwin SDK 的真实 include 与宏一致性 | 补足现有 toolchain 的具体参数投影；不使用宿主 fallback |
| CIndex 属性覆盖 | calling convention、TLS、nullability、Objective-C consumed／retained、availability／accessor 的真实 cursor 结果 | 只为缺失事实增加局部 token／编译 probe；仍无法取得时拒绝相应声明 |
| packed／aligned 与递归 record | 共享 geometry 计算及 native static assertions 的一致结果 | 走显式 storage adapter／opaque pointer，不伪造 layout |
| 数值宏与字符 literal | 精确类型、值、负零和 expansion 来源 | 无法精确发射时 typed getter 或 Unsupported，不写第二个 C evaluator |
| Objective-C 清理出口 | pool drain、ARC 清理、异常转文本及 owner 接管失败的自有样本 | 调整 native 边界，未闭合前不宣布 B3 完成 |
| CIndex 发行成本 | 实际增加的压缩大小、loader dependency 和许可项 | 记录实际成本；沿现有 bin 组件组织，不重新打包整套开发 SDK |

本设计确定了失败出口，技术探针可以据此局部修订具体实现选择；不把整个工具的完成定义
扩展成通用 C／Objective-C 语义证明或全 SDK 覆盖。
