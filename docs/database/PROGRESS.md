# 数据库框架实施记录

分支：`codex/database-sqlite`。状态：本机实现与验证完成。范围：公共资源与 Row/codec、SQL 能力、SQLite 完整异步驱动；HBase/Cosmos 保持设计检验。

## 批次与状态

1. 设计与规范：已建立设计、库规范和本记录。
2. 公共层与 Row/codec：已实现 `scoop.db` 的公共声明、稳定 Row/RowSchema、精确标量与 binary codec、Encoder/Decoder、惰性 record 映射和关闭协调。
3. SQL 接口与参数/事务组合：已实现 `scoop.db.sql` 的会话、Statement/ResultSet、参数快照与 resolve、事务接口和结构化 helper；事务执行行为已通过真实 SQLite 测试。
4. SQLite 原生驱动、完整异步、取消和资源生命周期：已实现 `scoop.db.sqlite`，覆盖内存/文件/只读、单语句准备与重复绑定、逐项读取、取消/超时、事务与确定性关闭；复用 managed blocking workers 和 EventLoop。
5. SQLite record 闭环、关键竞态、示例和收尾：已验证 record → 参数 → 数据库 → Row → record，交付[可运行示例](../../sysroot/src/scoop.db.sqlite/examples/README.md)及三层库说明。本机预编译库已构建；源码目录的临时 check 产物与 Cone.lock 已清理，保留增量构建缓存。

每个实现批次只运行相关 Cone 的 release 测试；完成后提交、推送并合并最新 origin/main。重复验证仅针对新变化或未解决问题。临时日志/探查文件不提交，批次完成后清理不再需要的临时文件，保留后续复用的构建缓存。

## 验证记录

- 文档批次：本地链接、章节编号、代码围栏和 whitespace 检查通过；没有实现或运行验证。
- 公共层批次：本机 aarch64-apple-darwin release 的 `scoop.db` 16 项 Cone 测试通过，覆盖嵌套/缺失/NULL/二进制/数值、encoder 提交与 decoder 复读、映射异常和并发 next/close。使用现有工具和 sysroot，仅显式指定本机已安装的 LLD；未重建工具链或运行跨平台测试。
- SQL 能力批次：同一本机 release 的 `scoop.db.sql` 5 项参数测试通过，覆盖位置/名称/完整性、record 显式字段顺序、缺失与 NULL、原生类型提示边界。公共层未变化，复用其已验证产物。
- SQLite 批次：本机 release 的 22 项 Cone 测试全部通过。覆盖文件重连/只读、内存隔离、完整绑定、稳定值/空 BLOB/非法 UTF-8、RETURNING、事务提交/正文失败/取消回滚/自动回滚、会话独占、运行与排队取消、超时、锁等待、关闭在途读取，以及已确认写入的结果和 executor 恢复。
- 收尾竞态：真实文件锁阻塞 SQLite finalize 时，并发 ResultSet.close 等待 native 收尾；修复了过早解除 Statement 资源归属的问题。旧取消退休后连续查询可复用连接，自动回滚后的旧 Statement 立即失效。
- 示例以三个生产 `.slib` 编译运行，输出 `7: Ada (no note)`。合计 43 项本地 Cone 测试通过；C 编译使用 `-std=c11 -Wall -Wextra -Werror`，Scoop 格式与文档链接检查通过。一次非法 UTF-8 测试的异常类型断言已按语言规范修正并复测闭合。

## 范围限制

本轮仅验证 aarch64-apple-darwin 和本机 SQLite 3.54.0；未运行跨平台、全仓或 GC stress 验收，不声明 HBase/Cosmos 驱动可用。驱动依赖目标环境的线程安全 SQLite 3.37+ 开发库，未新增 vendor、runtime ABI、编译器机制、ORM 或统一查询 AST。
