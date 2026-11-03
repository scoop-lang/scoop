# scoop.url

独立 URI／IRI 解析、相对引用、组件拼装、百分号和表单 codec。依赖 core、std、idna，使用固定 uriparser 1.0.2；没有网络或文件 I/O，也没有需要用户关闭的 parser。

```scoop
import scoop.url.*

val base = Url.parse("https://bücher.example/a/b?x=1")
val next = base.resolve("../路径?x=2&x=&flag#片段")
val origin = next.httpOrigin()
val target = next.httpRequestTarget()
val edited = next.copy {
    appendPathSegment("a/b")
    queryParameters(UrlQuery().add("q", Some("x+y")))
}
val created = url {
    scheme = Some("https")
    host("bücher.example")
    path("/api")
    encodedQuery = Some("q=%2F&q=&flag")
}
```

`Url` 是不可变值，按规范序列化结果比较与 hash。`toString` 给出 ASCII URI，`toIriString` 给出展示文本；`%2F` 等已编码保留字符继续保留。HTTP 网络 scheme 的域名转换为 A-label，origin 按去掉根点的 DNS 名或 IP 字节及有效端口比较。

`Url.parse` 接受相对引用和非 HTTP URI；它不从 `example.com` 猜测 scheme。`httpOrigin` 验证实际网络目标，拒绝空／非法 DNS host、数字地址的兼容写法、零端口与 IPvFuture。采用严格 RFC 3986／3987 profile；反斜杠、裸空白、坏百分号和 IPv6 zone identifier 明确报错，不提供浏览器容错。

`url { ... }` 从空引用开始配置，`base.copy { ... }` 从已有 Url 开始配置；两者同步执行配置一次并返回独立快照，修改被捕获的 builder 不改变结果。`url()` 是空相对引用，`base.copy()` 保留原组件。构造器和最终快照方法由库内部使用。

`scheme`、`port` 及 `encodedHost`／`encodedUserInfo`／`encodedPath`／`encodedQuery`／`encodedFragment` 是可写配置属性。`host`、`userInfo`、`path`／`appendPathSegment`、`query`、`fragment` 接收原文并编码；整体 `encodedAuthority` 操作同时替换 host、userinfo 和 port。所有配置操作返回 Unit，完整属性、默认值和清除规则见规范。

`appendEncodedPathSegment` 不接受裸结构分隔符。原文单段 `.`／`..` 编码成数据；已有编码不会被解码后当点段。整体 query 与参数操作不同，`encodedQuery` 不要求 query 是键值对；None 与空 query／fragment 分别保留。

`UrlQuery.parse` 保留重复项、顺序、无值与空值；`entries` 提供独立字节快照，文本 getter 严格按 UTF-8 解码。`add` 追加，`remove` 删除所有同名项，`set` 删除同名项后在末尾追加。`PercentEncoding` 的字节接口允许任意数据，只有 `decodeUtf8` 要求 UTF-8。普通 query 中 `+` 不代表空格，`FormUrlEncoded.parse`／`encode` 才采用表单规则。

`httpRequestTarget` 返回 path 与 query，`httpAbsoluteTarget` 提供代理所需的绝对形式，两者都排除 userinfo 与 fragment。URL 可以保留 userinfo，HTTP client 另行要求显式认证。日志可调用 `redacted` 去掉 userinfo 并按指定参数名遮盖值。

`UrlParseException` 保存组件、可选原输入 UTF-8 字节位置、说明及 host 的 IDNA 原因；`parseOrNone` 仅转换正常格式失败。后端／分配失败单独报告。完整合同见 [URL 规范](../../../docs/specs/SCOOP-URL-SPEC.md)。
