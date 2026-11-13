# scoop.markdown

纯 Scoop 的独立 Markdown 文档库，不依赖 UI、window、graphics 或 native Markdown 后端。完整语法与编辑器按[设计](../../../docs/markdown/DESIGN.md)实施，已验证范围见[实施记录](../../../docs/markdown/PROGRESS.md)。

当前源码模型支持持久化快照、UTF-8 范围、CR／LF／CRLF 行索引、原子多范围替换、位置映射、逆向修改和内容订阅：

```scoop
import scoop.markdown.*

val document = MarkdownDocument.fromText("# Title\n\nText\n")
val before = document.snapshot()
val change = document.replace(MarkdownRange(2L, 7L), "Updated")
document.apply(MarkdownEdit(document.revision, change.inverseReplacements))
```

`snapshot()` 不复制全文；`read(range)` 和 `toMarkdown()` 显式生成字符串。`writeTo(TextWriter)` 顺序输出分块。回调接收差量，不能重入修改；不编辑时输出保留所有源码字节。

完整的独立消费示例见 [examples/source](examples/source/src/main.scoop)，其 Cone 通过 path 依赖当前源码库，不要求先重新打包 sysroot。未安装本库时，应用在 `Cone.toml` 中为 `scoop:scoop.markdown` 指定实际源码目录。

本机使用已准备的工具链与系统库：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop fmt sysroot/src/scoop.markdown
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.markdown --tests --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown --release
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.markdown/examples/source --release
```
