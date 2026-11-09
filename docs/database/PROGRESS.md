# 数据库框架实施记录

分支：`codex/database-sqlite`。范围：公共资源与 Row/codec、SQL 能力、SQLite 完整异步驱动；HBase/Cosmos 保持设计检验。

## 批次与状态

1. 设计与规范：已建立设计、库规范和本记录。
2. 公共层与 Row/codec：已实现 `scoop.db` 的公共声明、稳定 Row/RowSchema、精确标量与 binary codec、Encoder/Decoder、惰性 record 映射和关闭协调。
3. SQL 接口与参数/事务组合：已实现 `scoop.db.sql` 的会话、Statement/ResultSet、参数快照与 resolve、事务接口和结构化 helper；事务执行行为将在真实 SQLite 测试中验收。
4. SQLite 原生驱动、完整异步、取消和资源生命周期：待实施。
5. SQLite record 闭环、关键竞态、示例和收尾：待实施。

每个实现批次只运行相关 Cone 的 release 测试；完成后提交、推送并合并最新 origin/main。重复验证仅针对新变化或未解决问题。临时日志/探查文件不提交，批次完成后清理不再需要的临时文件，保留后续复用的构建缓存。

## 验证记录

- 文档批次：本地链接、章节编号、代码围栏和 whitespace 检查通过；没有实现或运行验证。
- 公共层批次：本机 aarch64-apple-darwin release 的 `scoop.db` 16 项 Cone 测试通过，覆盖嵌套/缺失/NULL/二进制/数值、encoder 提交与 decoder 复读、映射异常和并发 next/close。使用现有工具和 sysroot，仅显式指定本机已安装的 LLD；未重建工具链或运行跨平台测试。
- SQL 能力批次：同一本机 release 的 `scoop.db.sql` 5 项参数测试通过，覆盖位置/名称/完整性、record 显式字段顺序、缺失与 NULL、原生类型提示边界。公共层未变化，复用其已验证产物。
