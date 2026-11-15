# scoop-bindgen：从 C／Objective-C 头文件生成 Scoop 绑定的调研与实施建议

调研日期：2026-11-03。源码基线：`b8ca1c9ba6`。本文调研一个宿主工具
`scoop-bindgen`，目标是从 C 或 Objective-C 头文件产生可编译、可链接、可审查的 Scoop
绑定。本文是实施建议，不修改当前语言、runtime ABI、`.slib` 格式或默认构建流程。

## 1. 结论

建议把 `scoop-bindgen` 实现为独立 Rust 工具，并通过已有外部命令机制以
`scoop bindgen ...` 调用：

1. 使用目标工具链配套的 **libclang CIndex** 解析头文件。解析必须带入真实 target、
   sysroot、deployment target、include、define 和语言选项，不能用正则、宿主默认 ABI 或
   不稳定的 JSON AST 猜测声明。
2. 先建立 bindgen 自己的小型、类型化 `HeaderIr`，再把每项声明分类为
   **Direct／Shim／Unsupported**。CIndex 的 USR 只用于本次导入去重，不成为 Scoop 实体
   id、产物身份或信任机制。
3. 输出一个普通、目标相关的 library Cone：机械的 unsafe 原始绑定放在 Scoop 源码中；
   `static inline`、宏、不能直接表示的 ABI 细节放入生成的 `.c` shim；Objective-C 消息、
   ARC 所有权和异常边界放入生成的 `.m` shim。
4. 原始层只忠实表达 ABI，不猜测所有权、字符串编码、错误规则、回调生命周期、
   `@GCLeaf` 或 closed enum。需要这些语义的便利封装必须来自明确配置，并与原始层分开。
5. Objective-C 第一版不扩展 Scoop 语法，不从 Scoop 手写调用 `objc_msgSend`。生成的
   ARC `.m` facade 用头文件中的静态类型发送消息，再暴露固定 C ABI 给 Scoop。
6. 第一阶段只做 C；Objective-C 作为建立在同一 IR、选择器和发射框架上的后续阶段。
   C++、Objective-C++、Swift-only API、通用 Blocks／delegate 自动桥接不进入首版。

这一路线复用当前已经具备的 C FFI、`@CLayout`、`Ptr`／`FunPtr`、managed callback、
native source、Objective-C ARC 和 framework 链接能力。无需把 bindgen 放进 parser → HIR →
MIR → LIR 流水线，也不需要在 IR crate 中复制一套 C／Objective-C 语义。

## 2. 当前 Scoop 能直接承载什么

### 2.1 已有边界

当前规范已经提供以下生成目标：

- C ABI 顶层 extern function、global 和 TLS；当前实现只接受 `cdecl`；
- 固定位宽有／无符号整数、Boolean、Char、Float、Double、data pointer、code pointer、
  合法 `@CLayout` struct；
- `Option<Ptr<T>>` 和 `Option<FunPtr<F>>` 的 null 表示；`Ptr<Unit>` 作为 opaque `void *`；
- NativeSafe `FunPtr`、显式 `GCLeafFunPtr`，以及带 function + context 的
  `foreignCallback`；
- `captureErrno = true`；
- `String.withCString`、`String.fromCString` 和带长度的 `String.fromUtf8`；
- Cone 中的 C／C++／Objective-C native source、`c_flags`／`cxx_flags`／`objc_flags`、
  target condition 和 framework；Darwin `.m` 使用 Apple Clang、C11 与 ARC；
- 编译器生成 C storage bridge 的既有路径，可承担目标 C ABI 的 aggregate 分类。

直接依据见[语言规范第 13 章](../specs/SCOOP-SPEC.md)、
[runtime 规范第 3～4 章](../specs/SCOOP-RUNTIME-SPEC.md)和
[实现规范第 2 章](../specs/SCOOP-IMPL-SPEC.md)。

### 2.2 必须尊重的限制

- C varargs 不可直接声明；需要一份固定签名的 shim。
- C++／Objective-C 异常不得展开穿过 C ABI。
- `@GCLeaf` 是强调用链契约，不能因为参数 GC-free、函数短小或名字像 getter 就自动添加。
- C `const` 不属于 Scoop pointer 类型；生成器只能保留在文档、命名及高层 wrapper 约束中。
- Scoop integer enum 只允许已声明值；普通 C enum 允许库返回未来新增值，两者不能默认等同。
- `@CLayout` 只接受可递归表示、非 ZST 的字段，且显式 `aligned`／`packed` 仅接受
  0、1、2、4、8、16。
- `.mm` 尚不受 native source 合同支持。因此 Objective-C++ header／API 不能通过偷偷生成
  `.mm` 进入首版。
- Scoop managed reference 不能直接进入 C ABI；所有 native object、Core Foundation object 和
  user-data 都必须经过 GC-free handle/pointer。

## 3. 产品边界和输出合同

### 3.1 命令与安装位置

首版建议放在独立 workspace/package `tooling/scoop-bindgen`，产物安装为
`<SYSROOT>/commands/scoop-bindgen`；Windows 为 `.exe`。已有外部子命令协议会让
`scoop bindgen` 原样转发参数，不需要给主 driver 增加一个特殊内置命令。

工具应复用 `scoop-target-data` 的 target 名称和 `scoop-toolchain` 中确实可复用的宿主工具链
发现逻辑，但不依赖 inkwell、HIR／MIR／LIR 或 compiler stage 实现。

建议的最小 CLI：

```text
scoop bindgen generate --config scoop-bindgen.toml --target <canonical-target>
scoop bindgen check    --config scoop-bindgen.toml --target <canonical-target>
scoop bindgen inspect  --config scoop-bindgen.toml --target <canonical-target> <declaration>
```

- `generate` 在临时目录完成全部解析、发射和生成源码编译检查，成功后再更新输出目录；
- `check` 重新生成到临时目录并比较规范化输出，不写现有文件，供 CI 使用；
- `inspect` 解释某一声明为什么是 Direct、Shim 或 Unsupported，展示 canonical C type、
  target layout 和来源位置；它是诊断工具，不建立另一套验证模式。

不建议实现自己的 daemon、插件系统、远程缓存、来源审计、摘要收据或跨阶段资源预算。
普通构建缓存仍由 Scoop build 按既有更新规则处理。

### 3.2 生成目录

每个 target 产生一份普通 Cone。建议布局：

```text
generated/libfoo/<canonical-target>/
  Cone.toml
  src/bindings.scoop
  src/wrappers.scoop          # 只有配置了便利层时存在
  native/scoop_bindgen.c      # 只有存在 C shim 时存在
  native/scoop_bindgen.m      # 只有存在 Objective-C facade 时存在
```

`Cone.toml` 明确列出生成 source、native source、include 和 native libraries/frameworks。
同一 header 在不同 target 下可能有不同 typedef 宽度、宏、可见声明、calling convention 和
layout，不能把一次宿主解析结果冒充多目标通用绑定。以后发布时可以把各 target 生成 Cone
分别编成目标 `.slib`；这不需要给 `.slib` 增加 bindgen 专用段。

生成器只拥有其配置指定的输出目录。手写高层 API 应放在另一个普通 Cone，依赖生成 Cone；
生成器不改写用户现有 `Cone.toml`、不合并手写源码，也不要求在生成文件中保留可编辑区块。

### 3.3 两层 API

**原始 ABI 层**是必需输出：名称经过确定性转义，签名机械映射，调用保持 `unsafe`，并保留
原声明位置和 C 拼写的简短注释。它不把 `char *` 自动变成 `String`，不把整数自动变成
Boolean，不把返回值自动解释为错误。

**便利层**是可选输出，只承接配置明确说明的语义，例如：

- 参数是输入 UTF-8 NUL 字符串，调用期间借用；
- 返回 pointer + length 是需要复制的 UTF-8；
- 负数返回值配合 errno 形成异常；
- 返回对象是 owned／borrowed；
- callback 是 synchronous／one-shot／registered，context 和 unregister 分别是谁；
- 某个 C enum 在 ABI 与演进规则上确实 closed。

便利层不是人工智能式猜测器。头文件注解能够给出可靠事实时可以成为配置默认值，但任何
影响所有权、异常、GC 或生命周期的决定都应能在 `inspect` 中追溯到头文件属性或用户配置。

## 4. 配置模型

建议使用版本化 TOML，字段只覆盖生成器真正消费的输入。示例是方向，不是已经冻结的格式：

```toml
schema = 1

[binding]
group = "example.bindings"
name = "foo"
version = "0.1.0"
package = "example.bindings.foo"
output = "generated/foo"

[headers]
language = "c" # c | objective-c
include = ["foo/foo.h"]
include_dirs = ["vendor/foo/include"]
defines = { FOO_NO_LEGACY = "1" }
flags = ["-std=c17"]

[selection]
header_filter = ["foo/**"]
exclude_header = ["foo/detail/**"]
allow_function = ["^foo_"]
allow_type = ["^foo_"]
allow_variable = ["^FOO_"]

[[libraries]]
name = "foo"
kind = "default"

[[rename]]
c = "foo_open"
scoop = "openRaw"

[policies]
closed_enums = ["foo_mode"]
gc_leaf_functions = ["foo_fast_hash"]
capture_errno_functions = ["foo_open", "foo_close"]
```

Objective-C 目标再使用独立段：

```toml
[objective_c]
frameworks = ["Foundation"]
classes = ["NSLocale", "NSURL"]
protocols = []
catch_exceptions = true
deployment_target = "13.0"
```

需要补充的规则：

- 命令行 target 必须是唯一解析 target；配置可以用与 Cone 一致的 os／arch／env 条件覆盖
  include、define、library 和 selection，但不能在一次解析中混合多个 target。
- allowlist 是根集合，类型、函数签名和字段所需声明按引用闭包加入；显式 allow 的根如果无法
  生成必须报错，不能静默丢弃。
- header filter 决定哪些声明可作为公共根；来自系统 header 的依赖类型仍可作为闭包节点，
  但不会因此把整个 libc 或 framework 导出。
- rename 冲突、Scoop 关键字、非法标识符和 overload 碰撞在 IR → Scoop name 阶段统一处理。
  自动后缀必须稳定且可预测；两个仍无法区分的公共声明报错，不用 FQN 或地址兜底。
- `gc_leaf_functions`、closed enum、字符串和 ownership 策略都应是窄范围显式配置；不提供
  “全局假定所有函数 leaf／所有 enum closed”开关。

项目已有 `compile_commands.json` 时，可以提供 `--compile-command <file>` 读取一条命令的
target、include、define 与语言选项。JSON Compilation Database 的 `arguments` 数组应优先于
需要 shell 拆词的 `command` 字符串。最终实际参数必须由 `inspect` 可见。

## 5. 解析和内部架构

### 5.1 为什么选择 libclang CIndex

Clang 官方把 LibClang 定位为相对稳定、适合其他语言调用的高层 C 接口；它提供 C／
Objective-C declaration cursor、canonical type、USR、source location、attribute、nullability、
size／alignment／field offset、token 和预处理记录。它没有暴露完整 Clang AST，但 bindgen
首版需要的是头文件公共声明和 ABI 事实，不需要改写 Clang AST。

不建议以 `clang -Xclang -ast-dump=json` 为产品格式：Clang 源码明确说明 JSON AST dump
不承诺跨大版本稳定或完整。LibTooling 能看到更完整 AST，但会引入与目标 Clang 同步构建的
C++ helper；只有 P0 证明某个必需 Objective-C 事实无法由 CIndex、token 和编译探针取得时，
才为该具体事实增加小型 helper，不先搭一套通用 Clang 插件框架。

Rust 侧可先评估维护中的 `clang-sys` runtime loading。其版本 feature 可能落后于本项目固定的
Clang 22.1 命名，但本方案使用的大多数 CIndex API 很早已稳定；P0 必须在打包的 22.1 libclang
上实际验证符号集、动态库发现和 version gate，不能仅凭 crate feature 名称宣布兼容。

### 5.2 解析流程

```text
config + target/toolchain
          │
          ▼
synthetic translation unit ── libclang diagnostics
          │
          ▼
declaration collection + canonicalization
          │
          ▼
selection roots + transitive type closure
          │
          ▼
typed HeaderIr
          │
          ▼
Direct / Shim / Unsupported planner
          │
          ├── Scoop emitter
          ├── C shim emitter
          └── Objective-C facade emitter
          │
          ▼
format + compile generated native source + atomic publish
```

synthetic translation unit 只包含配置的 `#include`／`#import` 和必要 probe 声明。所有 Clang
error 都应带原 header 位置转成 bindgen 诊断；有 error 时不发射半套绑定。

### 5.3 `HeaderIr` 需要保存的最小事实

- `DeclId`、canonical declaration、来源、spelling、linkage、visibility；
- typedef、record、enum、function、variable、macro、Objective-C interface／category／protocol／
  property／method 的不同 typed id；
- canonical `CType`：整数 rank/width/signedness、float kind、pointer 与 nullability、array、
  function prototype/calling convention、record/enum identity、qualifier；
- record 的 complete/incomplete、size、alignment、字段顺序与 bit offset、bit-field、flexible
  array、packed/aligned attribute；
- function 的 fixed/variadic、storage/linkage、inline、参数与返回、属性；
- Objective-C selector 的全部片段、class/instance、property accessors、method family、
  nullability、availability、ownership/consumed/result-retained 属性；
- 宏的 token、是否 function-like，以及能否经合成 probe 得到精确常量值。

这个 IR 只服务生成器，不写进 `.slib`。声明去重使用 canonical cursor／USR；USR 不参与
Scoop 名称、外部符号、缓存资格或任何来源认证。

### 5.4 建议模块

```text
src/
  cli.rs
  config.rs
  target.rs
  clang/
  ir/
  select.rs
  name.rs
  plan/
    c.rs
    objc.rs
  emit/
    scoop.rs
    c.rs
    objc.rs
  diagnostic.rs
```

只有出现第二个真实调用点时再抽公共层。尤其不要在 compiler 的 AST/HIR/meta crate 中增加
C declaration 或 Objective-C object model。

## 6. C 映射规则

### 6.1 标量、pointer 和 alias

| C 声明 | 原始 Scoop 表示 | 处理要求 |
| --- | --- | --- |
| `void` result | `Unit` | `void` 参数列表不是一个 Unit 参数 |
| `_Bool` | `Boolean` | 不把任意整数 typedef 猜成 Boolean |
| `signed/unsigned char/short/int/long/long long` | 匹配目标宽度和符号的固定整数 | 宽度和 plain `char` 符号性来自目标 Clang |
| `int8_t`…`uint64_t` | 对应 Scoop 固定整数 | 精确映射 |
| `float`／`double` | `Float`／`Double` | 保留所有位型 |
| `long double`、`_Float128`、`__int128`、complex、vector、atomic | Unsupported 或显式 shim | 首版不伪装成相近类型 |
| `T *` | `Ptr<T>` | `_Nullable` 为 `Option<Ptr<T>>`；未标注按可能为 null 处理 |
| `void *` | `Ptr<Unit>` | 可空规则相同 |
| `const char *` | `Ptr<Int8>` | raw 层不自动转 String |
| typedef | 可表达时生成 typealias | ABI 判断总是使用 canonical type |

`size_t`、`ptrdiff_t`、`intptr_t`、`wchar_t` 等必须按目标 Clang 给出的 canonical 宽度和
signedness 生成，不能按生成器宿主写死。`const`／`volatile`／`restrict` 保存在 IR 和文档中；
其中只有能由 Scoop 类型/注解表达的部分进入签名。volatile global 或 pointer pointee 不能被
便利层当作普通可优化内存访问。

### 6.2 enum 与 bit flags

默认把 C enum 生成为“底层整数 alias + 每个 enumerator 的 const”，原因是 C 库可以返回
头文件当前版本未知的新值，而 Scoop integer enum 是 closed value set。

只有以下任一条件成立时才可生成 Scoop integer enum：

1. 用户在配置中显式列为 closed；
2. 头文件有生成器明确支持、语义为 closed 的稳定 attribute，并且 `inspect` 报告该依据。

bitmask/flags 同样使用整数 alias + const，不生成普通 enum。匿名 enum 的常量可以进入当前
package；同名常量冲突按普通名称冲突处理。

### 6.3 struct、union、array 与 bit-field

首版只有满足下列条件的 complete record 才直接生成 `@CLayout struct`：

- 非空，全部字段递归 C-FFI-safe 且非 ZST；
- 没有 union、bit-field、flexible array member、C vector 或不能表示的 inline array；
- Clang 给出的 size、alignment 和每个 field offset 能由 Scoop 当前自然布局及允许的
  `packed`／`aligned` 组合精确表达；
- 生成器随后发射 C `_Static_assert` 检查 `sizeof`、`_Alignof` 和 `offsetof`。

找不到精确 Scoop layout 时不得靠 padding 字段碰运气。首版将该 record 作为 opaque 类型，
允许 `Ptr<Unit>` 或专名 raw handle 经过 pointer 使用；如果某个函数按值使用它，则该函数需要
专用 C shim 把参数/结果归一化为可表示 storage，或明确 Unsupported。

fixed array、union、bit-field 和 flexible array 先按 opaque/shim 处理。以后如果真实 API 大量
依赖固定 inline array 或 union，再单独设计一种语言／FFI 表示；bindgen 本身不能先发明新的
Scoop ABI 类型。

### 6.4 function 与 function pointer

以下函数可 Direct：external linkage、非 variadic、可表达的 fixed prototype、`cdecl`、全部
参数和结果可投影到当前 C-FFI-safe 类型。其余规则：

- `static inline`：生成一个 external、固定名称的 C wrapper；
- function-like macro：只有配置提供确切参数/结果 C type 时才生成 wrapper；
- C varargs：只有为某一固定调用形态显式配置参数列表时生成 wrapper，不支持通用 varargs；
- 非 cdecl calling convention：能由目标 C compiler 用 cdecl facade 正确调用时生成 shim，
  否则 Unsupported；
- 返回／参数为不直接表示的 aggregate：优先生成 typed storage shim，不把值强制改成 pointer
  语义；
- C++ name mangling、member function、template 和 exception 不属于 C 首版。

function pointer 使用 `FunPtr<F>` 或 `Option<FunPtr<F>>`。生成器永不自动选择
`GCLeafFunPtr<F>`；只有用户显式为这一类型承担完整契约时才生成 leaf 形式。

回调分三类：

- 只接受静态同步 callback：可直接暴露 `FunPtr`；
- 有 callback + `void *user_data` 且调用生命周期明确：便利层可用 `foreignCallback`；
- 缺少 context 槽、生命周期不明、unregister/flush 规则不清：只生成 raw API，或要求配置，
  不自动捕获 closure。

### 6.5 global、TLS 与宏

- 只读 external object → extern `val`；
- 可写 object → `@Global var`；
- thread-local object → `@ThreadLocal`；
- interior mutable／volatile 必须按规范的显式语义处理，不能仅由 `const` 拼写反推；
- 没有 external storage 的 header-local static 不作为 global 导出。

object-like 宏先通过预处理记录收集 token，再在 synthetic translation unit 中构造类型明确的
probe declaration，使用 Clang constant evaluation 得到整数、浮点或字符串值。能精确表示的值
生成 Scoop const；依赖 `sizeof`／target define 的值因此自然按 target 生成。不能求值或值超出
Scoop 常量表达范围时记录 Unsupported，不实现自己的 C constant evaluator。

## 7. Shim 的职责

生成 C shim 时应始终 include 原始 header，而不是复制声明。它只做下列普通 ABI 适配：

- 给 `static inline`、typed macro 和目标 calling convention 建立 external cdecl symbol；
- 把不可直接表示但可机械转换的 aggregate 参数／结果变成 canonical storage 参数；
- 把 configured fixed vararg call 固化为非 variadic 函数；
- 对 enum、record 和函数签名发射必要的编译期检查；
- 在异常可能存在的 C++/Objective-C 外壳中捕获异常并返回显式 status，禁止 unwind 穿边界。

wrapper symbol 建议使用 binding coordinate、声明 kind 和稳定序号构成私有前缀；它只需在本次
生成输出中确定、避免 native 冲突，不需要内容哈希、签名证明或防伪链。

生成 native source 的编译本身就是最终 ABI 检查：header 改变导致 prototype、layout 或宏不再
匹配时应在生成或普通 Cone build 失败。成功生成后不在每个 compiler stage 重新读 header 或
重放同一检查。

`captureErrno` 只按配置添加；shim 不应在所有调用上保存 errno。`@GCLeaf` 同样只按显式
配置生成，且 `inspect` 必须提示调用者正在承担无法由头文件证明的 runtime 契约。

## 8. Objective-C 方案

### 8.1 为什么必须生成 `.m` facade

Objective-C 动态消息不等于“所有方法都能用一个 variadic `objc_msgSend` 签名调用”。实际调用
仍受方法的精确返回／参数 ABI、struct return、selector、ARC method family、nullability、
availability 和 Blocks 影响。Apple 也要求手工使用 `objc_msgSend` 时按精确 method prototype
转换函数指针。

因此第一版应让 Clang 编译一个静态类型正确的 ARC facade：

```text
Scoop managed/raw wrapper
        │ fixed C ABI
        ▼
generated Objective-C .m facade
        │ typed [receiver selector:argument]
        ▼
Objective-C runtime / framework
```

这样消息 ABI、ARC retain/release 和 framework header 由同一目标 Apple Clang 判定；Scoop 侧
只看固定 C ABI。不要在 Scoop 中生成一组手写 `objc_msgSend` extern，也不需要给 Scoop 新增
Objective-C message syntax 或 runtime dispatch。

### 8.2 object handle 和所有权归一化

raw 层把 `id`、class pointer、protocol-qualified object pointer、`Class` 和 `SEL` 映射为不同的
Scoop alias/wrapper 名称，但物理边界统一使用 opaque pointer；nullable 使用 Option。

所有跨边界返回的 Objective-C object 应归一化成以下两种显式形式：

- **owned +1**：facade 通过 ARC bridge 保留对象，Scoop wrapper 获得一份必须关闭的 ownership；
- **borrowed for call**：只在一次同步 wrapper 调用内借用，不把 pointer 暴露到调用外。

对 public managed wrapper，建议生成 `final class` + `Closeable`，由 `close()` 调用 native release
facade，并让每份 ownership 恰好释放一次。默认不要自动生成 `release {}`：Objective-C object
销毁可能执行用户 native dealloc；只有配置明确声明这条 release 路径满足 Scoop release hook
的无回调、无 managed/runtime 依赖等完整契约时，才允许把它作为 best-effort 后备。没有该
声明时，用户必须显式 `use`／`close`，遗漏会泄漏但不会在 GC hook 中违反 runtime 契约。

参数对象在 facade 中以 ARC `__bridge` 借用；owned 返回通过 `__bridge_retained` 交给 Scoop；
release facade 用 `__bridge_transfer` 消费 +1 ownership。Core Foundation handle 保持 C API 的
独立 create/get/copy 规则，不因为能和 ObjC toll-free bridge 就自动套用 ARC 策略。

`ns_consumed`、`ns_consumes_self` 等消费参数／receiver 的 API 首版 Unsupported，因为 Scoop
当前 wrapper 没有通用 move-only ownership 表达；不能在调用失败或异常路径上假装对象仍可用。

### 8.3 method、property、category 和 protocol

- class method 和 instance method 生成不同 facade symbol；selector 必须保存全部冒号片段；
- property 展开成实际 getter/setter，遵守自定义 accessor 名和 readonly；
- category 方法合并到接收类的可选 wrapper surface，但记录原 category 和 header 来源；
- protocol 可先作为“参数可接受的 object capability”生成 raw handle；自动生成实现 protocol 的
  Scoop object／delegate adapter 延后；
- 同名 selector 的不同 receiver class 不冲突；同一 Scoop wrapper 内由确定性 name mangling 或
  显式 rename 解决 overload；
- generic Objective-C lightweight type argument 首版只保留注释和选择信息，不把它虚构成
  Scoop runtime generic ABI。

### 8.4 nullability、availability 和错误

- `_Nullable` → Option；`_Nonnull` 返回 nil 视为 native contract violation，facade 返回明确
  status，由 Scoop wrapper 抛出普通 managed exception；
- `null_unspecified` 保守映射为 Option；配置可以对具体声明收窄，不能全局假定 nonnull；
- 依据所选 deployment target 过滤 `unavailable`，保留 deprecated 诊断；高于 deployment
  target 才可用的 declaration 首版不自动生成 weak-link/runtime availability 分支；
- `NSError **` 不默认变成 throw。只有显式 policy 或生成器支持的 `swift_error`／API convention
  明确给出 success 条件时，便利层才把 error 转成 exception；raw 层保留 pointer/status 形式。

每次 facade 调用进入 `@autoreleasepool`，避免 Scoop 长生命周期线程没有 Cocoa run-loop pool
时累积 autoreleased object。

若开启异常捕获，facade 使用 `@try/@catch`，把异常 name/reason/description 复制到普通 malloc
UTF-8 buffer，返回 status + pointer + length。Scoop 立即用 `String.fromUtf8(pointer, length)`
复制并释放 buffer，再抛只持有 managed String 的 `ObjectiveCException`。不得让 Objective-C
exception、object pointer 或 ARC ownership 穿越 C ABI／保存在 Throwable 内等待 GC 清理。

### 8.5 Blocks、delegate 和异步 callback

Blocks 不是普通 C function pointer：它们有 descriptor、captured environment、copy/dispose 和
ARC 规则。建议分阶段处理：

1. 首版全部 Blocks 参数／结果 Unsupported；
2. 后续仅支持头文件明确 `noescape`、同步调用且有可验证调用期的 block，把它适配到
   `foreignCallback` + stack/native block facade；
3. escaping block、delegate、notification observer 和异步 completion 必须有明确的 native
   owner、copy/release、unregister/cancel、并发和 late-callback 合同；每一种 API family 通过
   窄配置或手写 adapter 接入，不能生成一个“万能 ObjC callback”。

protocol/delegate 自动实现可在后续生成一个 Objective-C adapter class：其中保存 C function
table + opaque context，Scoop 侧使用 `ForeignCallback` 管理 closure。只有确认 native 已完成
unregister 且不会再调用后才能 release callback ownership。该能力应复用现有 callback runtime，
不新增第二套 GC handle 或线程 attach 协议。

### 8.6 Objective-C 明确不做的事

- 不支持 `.mm` 和需要 C++ object model 的 header；
- 不导入 Swift-only declaration；
- 不自动把每个 Foundation object 变成 managed Scoop class hierarchy；
- 不把 API Notes、method family 或命名启发式当成绝对所有权证明；
- 不让 Objective-C exception unwind 进 Scoop；
- 不自动生成通用 KVC、KVO、runtime reflection 或 selector-by-string 调用。

## 9. 诊断要求

每个 selection root 最终必须得到一个可观察结果：

- `Direct`：生成到哪个 Scoop declaration，为什么当前 target 可直接表达；
- `Shim`：生成了哪个 native symbol，触发 shim 的首个原因；
- `Unsupported`：定位原 header，指出首个不能表示的 type/attribute/ABI，并给出可行的窄建议；
- `Excluded`：只用于未被显式 allow 的非根声明，说明命中的 filter。

诊断应展示 declaration path，例如
`function foo -> parameter callback -> function result -> long double`，避免只报“unsupported type”。
Clang warning 默认原样报告，Clang error 阻止发布；重复 declaration 合并时保留真正定义位置。

对于会改变语义的 policy，生成摘要至少列出：closed enum、GCLeaf、errno、字符串转换、owned
return、callback lifetime 和 Objective-C exception policy。摘要服务审查，不变成构建时反复
验证的授权清单。

## 10. 测试与验收

### 10.1 单元测试

- canonical C type → Scoop type；目标 `char`、`long`、`size_t` 差异；
- selection closure、typedef cycle、forward declaration 和 duplicate declaration；
- 名称转义、关键字、selector、overload 和稳定冲突诊断；
- record layout classification；
- Direct／Shim／Unsupported 决策；
- 配置条件、rename 和 policy 定位；
- emitter 的确定性与换行／排序。

### 10.2 C 集成样本

自有小 header/library 至少覆盖：

- 全部基础整数、plain char、pointer nullability、typedef；
- nested simple struct、packed/aligned struct、by-value 参数与返回；
- open/closed enum、flags；
- static inline、object/function-like macro、target-dependent constant；
- global、mutable global、TLS；
- function pointer、静态 callback、callback + context；
- configured errno、fixed vararg shim；
- union、bit-field、fixed/flexible array、long double、128-bit、complex、vector、atomic 和真正
  varargs 的代表性 Unsupported 诊断。

行为测试能放在生成 Cone 的 `scoop.test` 时就放 Cone 测试；CLI、生成目录、诊断位置、native
编译、链接、ABI 和 `check` 差异才使用 fixture。不要为每个 C API 复制冷／热完整 sysroot 构建。

### 10.3 ABI 和目标矩阵

至少在当前 target catalog 的全部目标上解析并编译 C 绑定：

- Darwin aarch64／x86_64；
- Linux x86_64 GNU／musl；
- Windows x86_64 MSVC UCRT；
- Windows x86_64 LLVM-MinGW MSVCRT／UCRT。

实际运行覆盖可由 CI 主机能力决定，但每个目标都必须让同一 target Clang 编译生成 shim，并对
关键 struct 执行 size/alignment/offset assert。重点交叉验证 Darwin aarch64 与 x86_64 的
aggregate ABI，以及 Windows／Unix 的 `long`、`wchar_t`、calling convention 差异。

### 10.4 Objective-C 验收

在 Darwin 两个架构使用一套自有 framework/header 和 Foundation 代表 API，覆盖：

- class/instance method、property、自定义 accessor、category；
- nonnull/nullable/unspecified；
- owned/borrowed return、显式 close、autorelease pool；
- Objective-C exception → Scoop exception；
- unavailable/deprecated/deployment target；
- framework 链接；
- Blocks、consumed receiver、Objective-C++ 的明确 negative 诊断。

生成结果必须连续两次字节一致；`check` 在未变化时成功，在配置/header 的真实输入变化后报告
差异。测试不通过删除、隐藏 header、libclang、工具或原产物来证明缓存复用。

## 11. 分阶段实施

### P0：两周内完成可丢弃的技术探针

- 在工具链固定的 libclang 22.1 上验证 runtime loading；
- 对七个 target 建 synthetic TU，读取整数、pointer、enum、struct layout 和函数 ABI；
- 在 Darwin 读取 Objective-C interface/method/property/category、nullability、availability 和
  ownership attribute；
- 验证 macro probe、USR 稳定使用范围和 Clang diagnostic 映射；
- 列出 CIndex 确实缺失、且首版必需的事实。没有具体缺口则不引入 LibTooling helper。

退出条件是一个小程序能打印目标相关 `HeaderIr`，不是提前冻结配置或生成器框架。

### P1：可用的 raw C MVP

- `generate`／`check`、target/toolchain、TOML、synthetic TU；
- allow/filter + transitive type closure；
- scalar、pointer、typedef、enum-as-integer、简单 struct、function、function pointer、global/TLS；
- 确定性 Scoop emitter、Cone.toml 和完整 Unsupported 诊断；
- 生成 Cone 在全部 C target 编译，代表目标运行链接样本。

### P2：C shim 和真实库能力

- static inline、常量/typed macro、configured fixed vararg、非 cdecl facade；
- opaque aggregate 和 by-value storage shim；
- layout/prototype assert；
- callback + context policy、errno、显式字符串便利层；
- 用至少两个真实、规模不同的纯 C library 验收，例如 zlib 和一个 image/crypto parser；具体库按
  当时可重复构建和许可证选择，不把通过一个库等同于完整 C 覆盖。

### P3：Objective-C 消费端

- Darwin `.m` ARC facade；
- object handle、class/instance method、property、category；
- owned/borrowed、nullability、autorelease、exception、availability、framework；
- managed `Closeable` wrapper 和清晰的显式关闭合同；
- Foundation + 自有小 framework 的两架构验收。

### P4：按真实需求扩展 Objective-C callback

- `NSError **` 的显式 convention；
- noescape synchronous Block；
- protocol/delegate adapter、escaping completion 的 owner/unregister 生命周期；
- Core Foundation bridge policy；
- 只有明确消费者出现后再决定 API Notes 的读取范围。

### P5：发行集成

- 加入工具链打包、许可证收集和开发 sysroot 安装脚本；
- 用户文档、配置 schema、升级说明和 `inspect` 示例；
- 固定生成器自身的兼容性规则：未发布阶段仍直接演进配置和输出，同一次工具链内保持 target、
  header、生成 Cone 与 native library 的真实 ABI 一致。

## 12. 实施时的仓库变更范围

P0/P1 预计涉及：

- `tooling/scoop-bindgen/`：独立 Cargo package、测试和 README；
- workspace／依赖清单：只加入实际使用的 `clang-sys`、CLI、serde/TOML 等成熟依赖；
- `scripts/build_scoop_bindgen.py` 与发行组装：P5 才加入正式工具链；
- `tests/fixtures/`：只放 CLI/ABI/诊断必须的 fixture；普通行为放生成 Cone 的测试；
- `docs/scoop-bindgen/`：配置与用户文档。

在正式发布 `scoop bindgen` 时，应先在实现规范的外部命令／配套工具章节写明命令、输入输出和
target 合同，再实现发行集成。P1/P2/P3 本身不需要改变语言或 runtime 规范，因为生成结果只
使用现有公开能力。若真实 API 证明必须新增 C union、inline array、Objective-C++ 或新的
ownership 表示，应作为独立 spec-first 设计，不能在生成器里私自编码一个后端特例。

## 13. 关键风险与待确认项

P0 必须回答以下问题，之后才能冻结实现设计：

1. `clang-sys` runtime loader 对随工具链分发的 LLVM/Clang 22.1 的实际发现方式和最小符号集；
2. CIndex 对本项目需要的 Objective-C ownership、nullability、availability、method family 和
   Blocks 属性能否完整取得，哪些需要 token/probe；
3. 当前 Scoop `@CLayout` 对真实 target pack/alignment 组合的可覆盖率；
4. 生成的 raw binding package 是否需要统一公开命名约定，避免便利 Cone 在重生成后大量改名；
5. Windows 非 cdecl facade 在 MSVC 与 LLVM-MinGW 下的 Clang source spelling；
6. Objective-C wrapper 的 owned return/close 模式在现有 `Closeable` 和异常路径下是否足够，
   哪些 Apple API family 需要消费参数而必须暂缓；
7. 工具链打包 libclang 的大小、动态依赖和许可证成本；这决定 bindgen 是否随主包安装，或成为
   使用同一固定 LLVM 分发的可选配套包，但不改变解析语义。

这些是有具体消费者的技术验证，不应扩展成通用插件、ABI 证明、来源授权或资源预算系统。

## 14. 明确排除的实现方式

- 不用正则、tree-sitter 或手写 parser 代替 Clang 解释 C preprocessor 和 target ABI；
- 不把 JSON AST dump 当稳定协议；
- 不把所有 pointer 降成整数，也不把不能表示的 by-value aggregate 偷换成 pointer API；
- 不从函数名、注释或短小函数体自动推断 GCLeaf、ownership、nonnull 或 callback 生命周期；
- 不为普通 C enum 默认生成 closed Scoop enum；
- 不从 Scoop 直接用 variadic `objc_msgSend` 调任意方法；
- 不把 bindgen 塞进 HIR/MIR、`.slib` reader 或每次 build 的 compiler stage；
- 不自动改写手写 Cone／wrapper；
- 不以 Objective-C 支持为名顺带实现 C++／Objective-C++／Swift interop；
- 不建设来源认证、header 收据、代码签名、防篡改链、逻辑成本配额或重复全量校验。

## 15. 一手资料

- Clang Tooling 接口选择与 LibClang 定位：
  <https://clang.llvm.org/docs/Tooling.html>
- LibClang tutorial 与稳定性说明：
  <https://clang.llvm.org/docs/LibClang.html>
- CIndex cursor、C／Objective-C declaration 与 attribute：
  <https://clang.llvm.org/doxygen/group__CINDEX.html>
- CIndex type、size、alignment 与 field offset：
  <https://clang.llvm.org/doxygen/group__CINDEX__TYPES.html>
- CIndex Objective-C property、selector、getter/setter 查询：
  <https://clang.llvm.org/doxygen/group__CINDEX__CURSOR__XREF.html>
- CIndex constant evaluation：
  <https://clang.llvm.org/doxygen/group__CINDEX__MISC.html>
- Clang JSON AST 不承诺稳定／完整：
  <https://clang.llvm.org/doxygen/JSONNodeDumper_8h_source.html>
- JSON Compilation Database 格式：
  <https://clang.llvm.org/docs/JSONCompilationDatabase.html>
- Clang ARC 规范：
  <https://clang.llvm.org/docs/AutomaticReferenceCounting.html>
- Clang attribute reference：
  <https://clang.llvm.org/docs/AttributeReference.html>
- Clang Blocks language spec：
  <https://clang.llvm.org/docs/BlockLanguageSpec.html>
- Apple Objective-C object/message 基础：
  <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjectiveC/Chapters/ocObjectsClasses.html>
- Apple 对 `objc_msgSend` 精确 method prototype 的要求：
  <https://developer.apple.com/documentation/uikit/managing-functions-and-function-pointers>
- Apple Objective-C exception 指南：
  <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ProgrammingWithObjectiveC/>
- rust-bindgen 的 libclang 要求、allowlist、opaque type 与 bit-field 经验：
  <https://rust-lang.github.io/rust-bindgen/requirements.html>
  <https://rust-lang.github.io/rust-bindgen/allowlisting.html>
  <https://rust-lang.github.io/rust-bindgen/opaque.html>
  <https://rust-lang.github.io/rust-bindgen/using-bitfields.html>
- Kotlin/Native cinterop definition 与 C interop，作为配置和生命周期问题的对照：
  <https://kotlinlang.org/docs/native-definition-file.html>
  <https://kotlinlang.org/docs/native-c-interop.html>
