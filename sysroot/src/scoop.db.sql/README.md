# scoop.db.sql

在 `scoop.db` 之上提供可选 SQL 能力：SqlSession、Statement、ResultSet、SqlArguments，以及 TransactionalSqlConnection/SqlTransaction。非 SQL 驱动无需实现这些接口。

`prepare(sql)` 使用后端方言，一次准备一条可执行语句。`execute(arguments)` 返回 `SqlResult.Rows` 或 `SqlResult.Update`；带 RETURNING 的写语句也属于 Rows。空查询保留 schema。Statement 可在前次结果耗尽或关闭后重新绑定执行，每次必须提供完整参数，不继承旧绑定。

首版 SQL 会话单路执行，活动结果占用会话，竞争报 DbBusyException。用 `useAsync` 关闭结果和 Statement；事务通过 `connection.transaction { session -> ... }` 管理，正文及其结构化子任务结束后提交，失败或提交前取消则回滚。手动控制使用 `beginTransaction`，不要另发改变事务归属的 BEGIN/COMMIT/ROLLBACK SQL。

参数支持 positional 和 named；名称按后端精确匹配。`SqlArguments.fromRow(row, fieldOrder)` 按显式字段顺序绑定。缺字段不等于 NULL，DEFAULT 由 SQL 文本表达，类型提示由驱动决定。独立驱动用公开 `resolve(parameters)` 得到经过完整性检查的参数快照。

可运行示例见 [SQLite record 往返](../scoop.db.sqlite/examples/README.md)，完整接口和异步规则见[数据库规范](../../../docs/specs/SCOOP-DB-SPEC.md)。
