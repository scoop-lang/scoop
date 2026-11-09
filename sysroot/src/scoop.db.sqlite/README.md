# scoop.db.sqlite

SQLite 驱动，实现 `scoop.db.sql.TransactionalSqlConnection`。依赖目标环境的线程安全 SQLite 3.37+ 开发库，构建时链接 `sqlite3`；不内嵌数据库引擎。本轮只完成本机验证。

在 Cone.toml 的 dependencies 中声明程序直接使用的库：

```toml
"scoop:scoop.core" = "0.1.0"
"scoop:scoop.std" = "0.1.0"
"scoop:scoop.asyncio" = "0.1.0"
"scoop:scoop.db" = "0.1.0"
"scoop:scoop.db.sql" = "0.1.0"
"scoop:scoop.db.sqlite" = "0.1.0"
```

可运行程序见 [examples/records.scoop](examples/records.scoop)，从普通 record 编码参数，在事务中写入，再按字段名惰性解码查询结果。

`SqliteDataSource(SqliteLocation.Memory)` 每次 connect 创建独立内存库。文件使用 `SqliteLocation.File(Path.fromString(...))`，读取文件还需直接依赖 scoop.platform。File 拒绝空路径和裸 `:memory:`，不把路径当 URI。`SqliteOptions(readOnly, busyTimeout)` 不改变 journal mode 或其他应用 PRAGMA；busyTimeout 默认为零。

connect 要求活动 EventLoop。打开、prepare、step、reset/finalize、事务完成及 close 都在借用的 managed blocking workers 上运行，恢复回调用方 executor。连接内串行，活动结果独占会话，竞争报 DbBusyException；需要独立并行操作时创建不同连接。关闭全部数据库资源后再退出所属 loop。

运行中取消结合 interrupt、progress handler 和可取消的锁等待。排队取消不执行请求；已开始写入交付真实 SQLite 结果，不能用取消断言“没有写入”。超时覆盖排队，且要等 native work 收尾后才返回。rollback/close 在已取消任务中仍能完成，旧取消通知不会传给下一条操作。

| SQLite 值 | RowValue |
| --- | --- |
| NULL | Null |
| INTEGER | Signed |
| REAL | Float64 |
| TEXT | Text，严格 UTF-8 |
| BLOB | Bytes，空值与 NULL 分开 |

Boolean 参数绑定为 0/1，读取时由显式 codec 选择解释。Unsigned 超过 Long.MAX_VALUE、NaN、Record/Sequence 和非空 nativeTypeName 提示在执行前拒绝；时间、Decimal、JSON 等须先显式编码。TEXT 支持内嵌 NUL，SQL 文本和文件路径仍遵守各自字符串/路径规则。

具名参数保留 `:x`、`@x`、`$x` 等精确名称，重复名称共用一个 slot；`?NNN` 稀疏编号使用 positional 并提供到最大 slot 的所有值。结果列可重名，按名称解码前用 Row.select 投影重命名。

Update.affectedRows 是本次执行 total_changes64 的差值，包括 SQLite 计入的触发器/外键修改；无修改的 DDL 为零。RETURNING 属于 Rows，首次 step 或提前 close 不保证回滚；需要原子性时用显式事务。SQLite 自动回滚会更新 transaction.state 并使旧事务 Statement 失效。SqliteException 保留 extendedCode 和 primaryCode。

完整合同见[数据库规范](../../../docs/specs/SCOOP-DB-SPEC.md)，验证范围见[实施记录](../../../docs/database/PROGRESS.md)。
