# scoop.db.sqlite 示例

先完成[运行准备](../../../README.md#运行准备)，并准备线程安全的 SQLite 3.37+ 开发库。普通 sysroot 准备流程会按依赖发现 scoop.db、scoop.db.sql 和 scoop.db.sqlite。

在本目录执行：

```sh
scoop run --release records.scoop
```

[records.scoop](records.scoop) 创建独立内存库，在事务中写入普通 struct，再把查询 ResultSet 惰性映射为同一个类型，输出 `7: Ada (no note)`。所有资源在 EventLoop 退出前通过 useAsync 关闭，不创建文件。

record companion 复用语言已有的 ValueEncoder/ValueDecoder 合成。`emitNull = true` 使 note=None 编码为显式 NULL，以满足三个 SQL 参数；查询故意使用不同列顺序，验证按名字解码。SQL 和字段顺序由程序给出，不生成 ORM 查询。
