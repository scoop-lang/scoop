# 代码高亮规则

默认高亮器使用 scoop.regex 的 PCRE2 实现，以一个有序正则扫描完整代码块；结果是 UTF-8 字节范围，不依赖 UI、JavaScript 引擎或外部进程。支持 Scoop、Kotlin、C／C++、Rust、Shell、JSON、TOML、Python、JavaScript／TypeScript、YAML 和 SQL，以及 rules.scoop 中列出的别名。没有已知规则时返回空范围，正文继续完整显示。

词法规则改编自 [Prism](https://github.com/PrismJS/prism/tree/298b75f1764c4abfe76d997394b7b149845f685a/components) 的 clike、c、kotlin、rust、bash、json、toml、python、javascript、yaml 和 sql 组件；固定参考提交为 `298b75f1764c4abfe76d997394b7b149845f685a`。MIT 许可见 [licenses/prism.txt](../../licenses/prism.txt)。Scoop 关键字按本仓库语言规范调整；正则转换为 PCRE2，使用 Unicode 单词边界，并允许编辑中的未闭合字符串和注释。

此实现只提供平坦词法颜色，不移植 Prism 的递归 token 引擎或语言语义分析。字符串插值、嵌套语言、复杂 Shell 展开和上下文相关类型名称不细分；宿主可通过 MarkdownCodeHighlighter 提供自己的范围。高亮失败只产生诊断并回退为普通文字，不改写代码。
