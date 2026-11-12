## 运行

```sh
scoop run --release main.scoop
```

再分别将 `SCOOP_EXAMPLE_LABEL` 设为空字符串和 `Ada` 后运行。环境变量设置方式由当前 shell 决定。

## 预期结果

变量缺失时：

```text
missing
effective: 'guest'
```

存在且为空时输出 `present: ''` 与 `effective: ''`；值为 Ada 时两行都保留 Ada。

## 关键说明

`environmentVariable` 返回 `OsString?`。先区分 None 与 Some，再依照应用的文本协议解码。此例明确要求 UTF-8；不合法的编码应向调用方报告，不当作变量缺失。

`getOrElse` 只在 None 时执行默认值回调。Some(空字符串) 不触发 fallback。需要回退到另一个可选来源时使用 `orElse`，不要先求值备用来源再传入急切求值的普通实参。

## 常见误用

运行期的 `scoop.platform.env.environmentVariable` 与编译期的 `scoop.compiler.env` 用途不同；后者属于编译输入。不要用空文本、"0" 或 "false" 统一代表缺失，解析和默认策略应由配置规则决定。
