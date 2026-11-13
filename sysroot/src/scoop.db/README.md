# scoop.db

数据库公共资源、异步结果流，以及可选的 `scoop.db.row` 数据与 record 映射。公共 Connection 不假定 SQL、事务、socket 或物理连接；SQL 能力位于 [scoop.db.sql](../scoop.db.sql/README.md)，首个驱动为 [scoop.db.sqlite](../scoop.db.sqlite/README.md)。

`ResultStream<T>.next()` 按需交付稳定值，外层 None 表示 EOF。同一流不允许并发 next；close 停止接纳并等待本地在途工作收尾。通过现有 `useAsync` 管理资源，关闭早于所属 EventLoop/executor 退出。`DbCallOptions.timeout` 作用于单次调用，包含排队；取消和超时不证明写入已回滚。

`Rows.encodeToRow(value, codec)` 与 `Rows.decodeFromRow(row, codec)` 使用现有 ValueEncoder/ValueDecoder。record 是普通 struct/class，不需要持久化基类、反射或宏。`ResultStream<Row>.decodeRecords(codec)` 惰性映射并接管源流的消费与关闭；映射失败关闭源流。

Row 是不可变快照。缺列与显式 Null 不同；重复列名用 `select` 按零基 Long 序号投影并重命名。数值转换保持精确，不自动将文本、整数和 Boolean 互转。嵌套 Record/Sequence 可用于映射，但不自动成为 SQL 标量。二进制使用显式 RowBytesCodec，默认 Array<Byte> codec 仍表示整数序列。

完整规则见[数据库规范](../../../docs/specs/SCOOP-DB-SPEC.md)与[设计](../../../docs/database/DESIGN.md)。使用示例见 [SQLite record 往返](../scoop.db.sqlite/examples/README.md)。
