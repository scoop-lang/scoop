# Scoop 教程大纲

本大纲规划 Scoop 教程的章节结构，覆盖语法、常见用法与基础库。内容以 [语言规范](../specs/SCOOP-SPEC.md)、[标准基础库规范](../specs/SCOOP-STDLIB-SPEC.md)、[平台库规范](../specs/SCOOP-PLATFORM-SPEC.md) 等文档为准；教程不另立规则，与规范冲突时以规范为准。

标记 ★ 的条目是 Scoop 与 Kotlin 差异较大、读者最容易误用的要点，正文应重点讲解。

编写约定：

- 每章至少配一个可直接运行的完整短程序，并给出预期输出；第 2～13 章优先复用 `sysroot/share/scoop/examples/programs` 中的已有示例。
- 第 7～9 章（值类型、模式匹配、Option）是与 Kotlin 差异最大的部分，篇幅应最多。
- 每章末尾整理"常见误用"，可以参考示例 topics 中的同名小节。

---

## 第一部分 入门

### 第 1 章 认识 Scoop

- 定位：以 Kotlin 核心语法为基础，静态类型，编译到原生代码；有真正的值类型、单态化泛型、LLVM 后端和分代 Immix GC。
- 与 Kotlin 的主要差异速览（细节在后续各章展开）：
  - 没有 `null`，可选值使用 `Option<T>`；
  - 用 `struct` / `enum` 代替 `data class` / `enum class`；
  - 只有 f-string 支持插值；
  - 没有运行期反射；
  - 泛型没有型变。
- 安装工具链，`scoop` 命令总览：`run`、`test`、`new`、`example`、格式化、文档、LSP。

### 第 2 章 第一个程序

- 编写 `hello.scoop`，用 `scoop run --release hello.scoop` 运行。
- 单文件程序：
  - 不需要 `Cone.toml`；
  - 用 `//! dep: scoop:scoop.platform:0.1.0` 声明依赖；
  - 首行可以写 `#!` 解释器注释。
- 项目：`scoop new --group examples.hello hello` 创建项目，再用 `scoop run .` / `scoop test .` 运行和测试。
- 用 `scoop example --list` 浏览离线示例。

## 第二部分 语言核心

### 第 3 章 基础语法与基本类型

- `val` / `var`、类型推断、`const val`。
- 基本类型：
  - 8 种定宽整数，有符号与无符号（无符号字面量带 `u` 后缀）；
  - `Float` / `Double`、`Boolean`、`Char`、`Unit`；
  - 顶层类型 `Any` 与底层类型 `Nothing`。
- ★ 整数运算默认回绕（wrapping）；size、index、计数统一使用 `Long`（例如 `bytes[0L]`）。
- ★ 浮点规则：所有 NaN 彼此相等，正负零相等；`Float` / `Double` 不实现 `Hash`。
- 标识符规则（Unicode、反引号名称）、注释、文档注释。

### 第 4 章 字符串与文本

- `String` 采用 UTF-8：`charLength` 是 Unicode 标量个数，`toByteArray().size` 是字节数，两者可能不同。
- ★ 只有 `f"Hello, ${name}"` 会插值，普通字符串中的 `"${x}"` 原样输出。
- `Char` 与 Unicode 标量、遍历字符串、`StringBuilder`。

### 第 5 章 控制流

- `if` / `when` / `try` 都是表达式；值位置的 `if` 必须带 `else`。
- `for` 与区间 `1 .. n`、`while` / `do-while`、带标签的 `break` / `continue` / `return`。
- 普通 `when`：值分支、`is` 分支、`in` 分支、guard（`is String if ...`）、`else if`。

### 第 6 章 函数

- 默认参数、命名参数、`vararg`、`infix`、单表达式函数。
- 扩展函数与扩展属性。
- 函数类型、lambda、尾随 lambda、带标签的 `return@label`。
- 闭包捕获规则（★ 不能通过 `var` 的 smart cast 视图隐式捕获）、局部函数、callable reference。
- `inline` / `reified`；`tailrec` 等兼容修饰符只解析，没有效果。

### 第 7 章 值类型 ★

- `struct`：主构造、方法、companion、结构相等 `==`。
- 副本更新：`p.{x: 8}`。
- `enum`：
  - 无 payload 变体；
  - 位置 payload 变体；
  - 命名字段 payload 变体；
  - 整型 enum。
- tuple：`(1, "a")`，位置名从 `_1` 开始。
- 解构：`val (x, y) = p`、字段解构 `{ f1, .. }`、用 `..` 跳过其余字段。
- 值语义：
  - 不可变；
  - 没有 identity，不能用 `===`；
  - 赋值和传参复制整个值；
  - 转换为接口或 `Any` 时装箱。
- 零尺寸值类型（ZST）。
- 选型指南：何时用 struct、enum，何时用 class。

### 第 8 章 模式匹配

- `when` 中的 `case` 模式：
  - enum 变体、tuple、struct（字段模式与位置模式）；
  - 可以嵌套，可以包含字面量；
  - 用 `_` 和 `..` 作通配。
- guard 与 first-match 求值顺序。
- ★ 穷尽性检查：未穷尽时编译器给出未覆盖的示例；带 guard 的分支不计入覆盖。
- `when (val x = expr)`，smart cast 的作用范围。

### 第 9 章 Option 与空安全

- ★ `T?` 是 `Option<T>` 的简写，取值为 `Some(v)` 或 `None`，没有 `null`。
- `?.`、`?:`、`!!` 的语义。
- ★ 判断非空之后不会自动收窄类型，取值要用 `case Some(v)`。
- `Some(None)` 与 `None` 是两个不同的值。
- std 中的组合函数：`map`、`getOrNone`、`mapNotNone`、`filterNotNone`；`OrNone` 系列 API 的命名约定。
- `Result<T, E>`：需要携带失败载荷时使用。

### 第 10 章 类、接口与对象

- class：
  - 主构造与次构造、`init` 块；
  - 属性及其 getter / setter；
  - 可见性修饰符。
- 继承：
  - `open` / `abstract` / `override`；
  - 单继承，可以实现多个接口；
  - `sealed` 继承及其穷尽性。
- interface：默认实现、条件接口实现；struct 也可以实现接口。
- `object`、`companion object`、嵌套类、`inner class`、匿名 object。
- 委托：属性委托（`by`）、接口实现委托。

### 第 11 章 运算符与核心协议

- `operator` 约定：算术运算、下标 `[]`、`invoke`、比较、复合赋值、`++` / `--`。
- 相等：`==` 与 `Equals<T>` 接口；`===` 只适用于引用类型。
- ★ `ToString` 与 `Hash` 必须显式声明实现，不会自动派生；`Compare` 的比较结果是 `Long`。
- 迭代协议：实现 `Iterable` / `Iterator`，让自定义类型支持 `for`。

### 第 12 章 泛型

- 泛型函数与泛型类型；单态化意味着运行期没有类型擦除。
- 上界约束 `<T: Named>`、`where` 子句、调用处类型推断。
- ★ 所有泛型都是不变的（invariant），没有 `in` / `out` / `*` 投影。
- `typealias`、companion bound（泛型中通过 `T.Companion` 调用）、条件接口实现。

### 第 13 章 异常与资源管理

- `try` / `catch` / `finally` / `throw`，core 异常层级。
- ★ 异常构造接收 `Option`：`IllegalArgumentException(Some("msg"))`；读取消息用 `error.message ?: "..."`。
- 确定性资源释放：
  - `Disposable` / `Closeable` 接口；
  - 用 `x.use { ... }` 确保释放；
  - 资源 API 的 `withX` / `takeX` 命名约定。

## 第三部分 工程化

### 第 14 章 包、导入与 Cone

- `package`、`import`、用 `public import` 再导出、块级 import。
- 可见性：`internal` 的作用范围是 Cone。
- `Cone.toml`：
  - `group` / `name` / `version` / `kind`；
  - `[dependencies]` 与测试依赖。
- workspace、`Cone.lock`、依赖来源（registry、Git、本地路径）。
- 编译期输入：`env`、`includeStr`、`includeBytes`、`includeDir`。

### 第 15 章 测试

- `scoop.test`：`@Test`、`assertEquals`、`assertFailsWith`。
- `scoop test --list` / `--filter`；测试源码放在 `tests/`。
- 推荐的库项目结构：库代码放 `src/`，测试放 `tests/`，`scoop.test` 只写在测试依赖中。

### 第 16 章 序列化与 codec

- `ValueEncoder<T>` / `ValueDecoder<T>`，由 companion 或普通对象提供 codec。
- 自动合成实现的条件，以及缺省数据模型。
- 实例：`scoop.json` 的 `Json.encodeToString` / `Json.decodeFromString<User>`；值为 `None` 的字段如何编码。

## 第四部分 基础库

### 第 17 章 core（无需声明依赖）

- `String` / `StringBuilder`。
- `Array`：数组字面量、按长度初始化。
- `List` / `MutableList` / `ArrayList`，快照与迭代。
- 区间与迭代、`println` 与文本 writer、只读调试输出。
- 原子类型与内存序（在第 22 章展开）。

### 第 18 章 scoop.std

- `scoop.collections`：
  - HashMap、LinkedHashMap、Set、Deque、优先队列；
  - `mapOf` / `listOf` 等工厂函数；
  - filter、map、sortedWith 等即时求值的集合操作。
- `scoop.comparisons`：Comparator 及其组合。
- `scoop.sequences`：惰性序列。
- `scoop.text`：文本操作，`toInt` 等数值解析，`NumberFormatException`。
- `scoop.bytes`（字节构建、读取与字节序）、`scoop.io.encoding`（Base64）、Hex。
- `scoop.random`（显式种子 PRNG）、`scoop.uuid`。
- `scoop.time`：Duration、Instant、Clock、TimeMark。
- `scoop.datetime`：LocalDate 等日期时间类型、固定 UTC 偏移。

### 第 19 章 scoop.platform（系统交互）

- `OsString`、`Path`。
- `File` / `OpenOptions`、目录操作、临时目录。
- 同步字节流、缓冲、UTF-8 读写。
- 命令行参数、环境变量、系统信息、时钟与阻塞 `sleep`、系统熵。
- 线程与同步原语、匿名管道与子进程。
- 阻塞式 DNS、TCP、UDP。

### 第 20 章 常用工具库

- `scoop.cli`：
  - 用注解声明位置参数与选项；
  - `parseCommandLine` 解析命令行；
  - 返回退出码。
- `scoop.log` 与 `scoop.log.console`。
- `scoop.json`、`scoop.regex`、`scoop.math`（BigInteger / BigDecimal）、`scoop.url`、`scoop.charset`、`scoop.compression`、`scoop.crypto`。

## 第五部分 进阶

### 第 21 章 协程与异步 I/O

- `suspend` 函数；在普通 `main` 中先创建 `EventLoop`，再用 `loop.run { ... }` 进入挂起上下文。
- 结构化并发：
  - `scope.async` / `await`；
  - `launch` / `join`；
  - 取消，以及在 `finally` 中清理；
  - `Job` / `Deferred`。
- ★ 挂起不等于开线程；不要用延时代替同步。
- 上下文参数 `context(name: T)`（task-local）。
- `scoop.asyncio`：网络、文件、定时等待。
- `scoop.concurrent`：Channel、AsyncMutex、AsyncConditionVariable、多线程 executor。

### 第 22 章 线程、原子与内存模型

- ★ Scoop 不保证并发安全，数据竞争是未定义行为。
- happens-before 关系、原子操作与内存序。
- 如何把对象安全地发布给其他线程。

### 第 23 章 网络应用实战

- `scoop.http.client`：发送 GET / POST 请求，读取并关闭响应。
- `scoop.http.server`：编写一个小型服务。
- TLS 与 WebSocket 简介。

### 第 24 章 FFI 与底层（选读）

- `@Extern`、`@CallingConvention`、`@CLayout`。
- `Ptr<T>`、`FunPtr`、`sizeOf` / `alignOf`。
- `@Unsafe` / `@Safe`、`MaybeUninit`、`@NoGC`、`@Global` / `@ThreadLocal`。
- 在 Cone 中携带 C / Objective-C 源码，以及链接系统库。

## 附录

- A. Kotlin 开发者速查表：逐项对照两者的差异。
- B. 常见误用汇总：从各章及示例 topics 的"常见误用"小节整理。
- C. 库 API 命名风格：见 [Scoop 库 API 风格](../specs/SCOOP-API-STYLE.md)。
- D. 选读：终端、readline、UI、窗口、图形库的简介与入口。
