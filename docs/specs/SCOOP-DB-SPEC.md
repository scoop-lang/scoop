# Scoop 数据库基础库规范

本规范规定 `scoop.db`、`scoop.db.sql` 和首个驱动 `scoop.db.sqlite` 的公开行为。具体声明及三种后端的抽象检验见[设计](../database/DESIGN.md)，实施状态见[记录](../database/PROGRESS.md)。这些是普通 Cone，不增加语言、编译器或 runtime ABI。

## 1. 范围与依赖

公共层只规定逻辑 Connection、DataSource、ResultStream、异步生命周期和可选的表格 Row/codec。SQL Statement、参数与事务属于 `scoop.db.sql`；SQLite 原生连接和调用属于 `scoop.db.sqlite`。HBase cells 和 Cosmos 文档不必转换为 Row，本期不实现这两个驱动。

record 指普通 struct/class。映射复用语言规范 9.5、11.13 的 ValueEncoder/ValueDecoder、方法合成和容器协议；不引入 ORM、反射、数据库宏、查询生成或持久化身份。普通库行为由所属 Cone 的 `scoop.test` 验证。

## 2. 公共资源与异步调用

- Connection 和 ResultStream 实现现有 AsyncCloseable；DataSource.connect 每次交付一个由调用者关闭的逻辑连接。工厂不自动拥有连接。
- 所有数据库 I/O、准备、锁等待、执行、取项、事务完成和资源释放均为 suspend；读取快照与 codec 为普通同步调用。
- 异步入口要求当前 Job 和可用 executor。关闭/回滚允许在已取消 Job 中完成，使用既有 nonCancellableScope。不得用 GC release hook 完成 I/O 或等待。
- 参数与可变容器在提交前取得稳定快照；编码、参数检查、快照失败不发出请求。
- 取消选择和原生工作退休是两件事：原生借用、callback 和未交付资源收尾后才恢复调用者，并恢复到原 execution/executor。排队取消避免开始操作；已运行操作按后端能力请求停止并等待本地真实结束。
- 对只读请求，取消、关闭和成功选择一次结果；先获胜的结果不被后续取消覆盖。可能已提交的写操作交付能够确定的真实成功/失败；超时、断链或取消不证明未写入，不自动重试。
- DbCallOptions.timeout 是每次调用的相对时限，接纳时转换为单调截止时间，包含排队与本次内部续页；不逐页重置。None 不设时限，负值是参数错误，零不提交并立即超时。超时请求停止，仍等待本地退休；不能强制中断同步 codec。
- close 停止接纳，协调在途工作并按拥有关系关闭子资源。并发 close 等同一次收尾；后续重复 close 不重报历史错误。清理不受正文已取消/到期影响，多个失败用 ResourceUseException 保留。
- close 不主动 COMMIT，也不保证撤销写入。原生隐式事务仍按其语义结束。借用的 EventLoop、executor、pool 由外层在数据库资源之后关闭。

DbException 保留 operation、message、cause；Closed/Busy/Timeout 是库能够确定的状态。驱动错误保留原始错误码。取消使用现有 CancellationException；用户 codec/default/constructor 的异常不改写为数据库异常。

## 3. ResultStream

`next(options): T?` 的外层 None 表示结束，Some(None) 可作为可空 T 的数据。同一流最多一个在途 next，竞争时报 DbBusyException；不同流能否并行由专属接口规定。

默认按需求拉取，不自动读完整个查询。允许原生页/批次缓存；空页带续页信息时继续，直到取得一项或明确结束。EOF 退休读取资源，此后 next 继续返回 None；关闭后报 DbClosedException。读取/映射失败使流终止，不暗中重开请求。

交付值在下次 next、挂起和流/连接关闭后仍有效。native 临时指针不能逃逸；不可变存储可以共享。失败 next 在恢复前退休本地读取/游标借用，并使连接恢复明确可用状态或失效。

mapRecords 与 decodeRecords 每次同步映射一项，接管源流的消费和关闭责任。映射失败关闭源流，双失败用 ResourceUseException；调用者不得并行消费已交给 wrapper 的源流。

## 4. Row 与 codec

Row/RowSchema 是不可变快照，列和值数量一致；Long 索引从零开始。列名可以重复，按名字读取歧义时报 AmbiguousColumnException，缺列时报 MissingColumnException；getOrNone 仅对缺列返回 None。select 按显式序号投影并重命名，目标名唯一。

RowValue 的值域为 Null、Boolean、Signed(Long)、Unsigned(ULong)、Float32、Float64、Text、Bytes、Record(Row)、Sequence。它不是所有数据库的共同 wire 类型。Column.nativeTypeName 是实际元数据，不决定当前值类型或非空性。

Rows.encodeToRow 要求 keyed 根，按实际 codec 字段输出；遵守根提交、pending 子值、重复 key 和只写一次的既有规则。Rows.decodeFromRow 使用 keyed 根，Rows.valueDecoder 显式适配单个值。Decoder 稳定、可复读，形状/类型失败不消费值；每个 unkeyed 视图有独立游标。

未知列默认 Ignore；Reject 在当前 keyed.end 检查未通过 required/optional 取得的字段，keys 枚举不计访问。path 使用语言规范的 JSON Pointer 规则。

数值只允许 Signed/Unsigned 范围内精确转换和 Float32→Float64 提升；其他数值种类转换须显式 codec。Boolean 不隐式读取整数，Text 不自动解析数值。BinaryEncodingContainer/BinaryDecodingContainer 以普通接口扩展单值容器；RowBytesCodec 使用它们，默认 Array<Byte> codec 仍为整数序列。

字段缺失、Null、有值和删除/DEFAULT 分开。Serialize、默认值、Option 省略沿用语言规范；显式 Null 不使用构造默认值。Row 不产生数据库写操作，字段省略不自动变成 NULL 或删除。嵌套值不自动绑定成 SQL scalar；JSON、时间、Decimal 等使用明确 codec。

## 5. SQL 能力

SqlSession.prepare 接收后端方言，不改写通用 SQL。Statement 一次对应一个可执行语句和一个结果，支持完整重新绑定后重复执行。SqlResult 为 Rows(ResultSet) 或 Update(affectedRows?)；Rows 只表示结果形状，不表示只读或已经提交。空查询也有 schema。

execute 启动本次执行；Update 表示执行完成，Rows 后续仍可发生读取/执行失败。ResultSet 的列顺序固定、值类型可变。关闭/耗尽释放会话占用；关闭 Statement 先关闭它的结果，关闭 Connection 清理所有子资源。

SqlArguments 是快照：positional 按公开的零基 slot 顺序，named 按驱动的精确名称。缺少、重复、多余或歧义参数在执行前失败。fromRow 按明确 fieldOrder 选择列，参数类型提示为 None，不把 Column 元数据当写入提示。类型提示由驱动决定，不能拼接 SQL。SQL DEFAULT 由 SQL 文本表达。

`SqlArguments.resolve(parameters: List<SqlParameterInfo>): Array<SqlParameter>` 是驱动的普通公开绑定入口：元数据按连续零基 slot 排列，返回同序快照并检查完整性。named 要求每个 slot 有唯一精确名称；因此首版不接受包含匿名或未命名空隙 slot 的 named 绑定，稀疏 SQLite 参数使用 positional。

首版 SQL 会话单路执行，活动结果占用会话；冲突报 Busy，不隐式等待自己释放结果。事务取得会话执行权，事务外连接和旧 Statement 不得混入；事务中只执行该事务的 Statement。

SqlTransaction.state 为 Active/Committed/RolledBack/Invalid。begin 的迟到成功在未交付时先回滚。commit 只接受 Active 且没有活动结果；可能已经提交后交付实际结果，未知结果使连接失效。确认提交先更新状态，再清理 Statement，清理错误不改写已提交事实。

rollback/close 可在取消后使用；关闭 Active 事务回滚，已完成事务幂等，Invalid 只收尾。后端自动回滚同步到 state。transaction helper 向正文提供 SqlSession，先等待正文结构化子任务，再检查取消并提交；提交后不追加取消检查。原失败与回滚失败均保留。手动事务控制 SQL 不得改变受管理事务的归属。

本版 SQL Row/参数值域不能无损表示的原生类型报转换错误，专属 API 可以保留自己的类型和 Decoder；不通过隐式 Text/Double 转换伪装支持。

## 6. SQLite

SqliteDataSource 配置 Memory/File、readOnly、busyTimeout。每次 Memory 连接独立，Memory/readOnly 冲突报参数错误；默认不改变 journal mode 或应用 PRAGMA。busyTimeout 非负，零按原生锁竞争错误返回。

连接借用 connect 时的 EventLoop。所有普通 SQLite 调用在 managed blocking workers 串行执行，worker 可以迁移；定时器和取消只协调停止与恢复。pool/loop 存活到连接关闭。使用线程安全 SQLite C API，不新增 runtime 功能。

prepare 由 SQLite 解析并拒绝第二条可执行语句。execute 完成绑定和首次 step，缓存首行或 EOF；next 继续推进，reset/finalize/close 也在 worker 执行。TEXT/BLOB 在交付前形成独立快照；空 BLOB 与 NULL 区分。

NULL/INTEGER/REAL/TEXT/BLOB 分别映射 Null/Signed/Float64/Text/Bytes；TEXT 严格 UTF-8，Boolean 参数为 0/1，Unsigned 超过 Long.MAX_VALUE 时拒绝，Float32 提升为 REAL。SqlParameter.nativeTypeName 必须为 None。

SQLite 原生 slot 减一作为公开索引，稀疏 ?NNN 按最大 slot 数绑定，具名重复占一个 slot；named 不猜匿名 slot。实际结果类型/列名来自 SQLite，不根据 SQL 前缀猜测。

运行中取消连接 sqlite3_interrupt、progress handler 和可取消 busy 等待。interrupt/close 正确同步，旧 work 与取消通知退休后才允许新操作，空闲 interrupt 不能代替排队/执行前取消检查。busy callback 只读取 native 状态，不执行用户 Scoop 代码。

step 失败后根据原生事务状态同步对象，必要时使连接失效。RETURNING 首次 step 产生修改但不表示提交完成；结束语句可能完成隐式事务，close 不承诺回滚，最终 step/reset/finalize 错误必须交付。

本轮只验证本机 SQLite 和 Cone 测试，不据此声明跨平台验证或其他数据库支持。
