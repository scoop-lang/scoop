## 运行

在保存文件的目录执行：

```sh
scoop run --release main.scoop
```

## 预期结果

程序 stdout：

```text
Hello, Ada!
Hello, guest!
3
84
present, scalars=0
fallback
```

## 关键说明

`T?` 是 `Option<T>` 的简写。`Some(value)` 携带数据，`None` 表示缺失；没有 null 值。`when` 显式解构后，value 的类型是 String。

`name?.charLength` 得到 `Long?`，`?: 0L` 提供缺失时的 Long。需要不同缺失行为时，修改 None 分支或 Elvis 的右侧即可。Option 泛型不变；嵌套 `T??` 表示两层 Option。安全调用不会把返回值原有的 Option 自动展平。

`map` 保留映射结果的层数；回调返回 Option 时，`flatMap` 直接连接两层操作。`fold` 分别处理 Some/None，Some(空文本) 仍是有值。`getOrElse` 的回调产生普通值，`orElse` 的回调产生另一个 Option；两者都仅在缺失时求值。这些扩展来自 std 的 `scoop` 包，源码显式导入。调用次数验证见 test/assertions。

## 常见误用

程序末尾的 `when` 把缺失值视为已处理的无操作分支，写成 `case None -> Unit`，因此不增加输出。`()` 与 `Unit` 等价，也可独立用作无操作语句。

不要写 `if (name != null)`；Scoop 不提供这种 null 判空或 Option 判空智能转换。需要值时用 case 解构，不要假设检查过 None 后变量本身就变成 String。
