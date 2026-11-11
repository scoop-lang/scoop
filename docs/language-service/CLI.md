# 语言服务命令行查询

`scoop-lsp query` 将一次查询交给同一可执行文件的 `--stdio` 服务器。它复用已有 workspace、前端、诊断与 LSP 结果，不构建应用、准备 sysroot 或创建另一套类型判断。原有无参数/`--stdio` 启动方式保持不变。

## 查询

在含 Cone.toml 的项目或 workspace 目录中运行，文件路径相对当前目录；`--project` 可选择另一个项目目录或 Cone.toml。所选工具链须具有配套 PM 和目标库。库查找相关路径选项相对当前目录；查询不把单文件模式的 `//! dep:` 当成 Cone 依赖。

```sh
scoop-lsp query hover src/main.scoop --line 8 --column 12
scoop-lsp query signature src/main.scoop --line 8 --column 20 --format json
scoop-lsp query completion src/main.scoop --line 9 --column 10 --purpose test --format json
scoop-lsp query definition src/main.scoop --line 8 --column 12
scoop-lsp query references src/main.scoop --line 8 --column 12
scoop-lsp query symbols src/main.scoop
scoop-lsp query workspace-symbols Greeting
scoop-lsp query diagnostics src/main.scoop --format json
scoop-lsp query capabilities --format json
```

hover 包含当前位置可确定的类型、签名和文档；signature 针对调用参数位置。completion 使用实际接收者、可见性和约束。type-definition、implementation 分别查询类型定义和实现。symbols 与 workspace-symbols 检索当前文档/工作区源码声明，不宣称已经检索全部已安装库；按名字发现尚未使用的库 API 由文档查询负责。

共用选项为 `--project`、`--sysroot`、`--target`、`--purpose production|test`、`--package-manager`、`--package-cache-dir`、`--cone-path`、`--lockfile`、`--position-encoding utf-8|utf-16|utf-32` 以及 `--format text|json`。capabilities 报告实际服务器能力，不加载项目。

文档查询可加 `--stdin`，将 stdin 的完整 UTF-8 正文作为该文件的编辑 buffer；文件名仍用于确定项目归属和位置，正文不写回磁盘。每次 CLI 调用建立新会话，使用当前磁盘/显式 buffer；不复用上一次 CLI 的旧快照。引用与定义继续通过原有 typed 实体和源码映射查询。

## 位置、结果与失败

- `--line` 与 `--column` 从 1 开始，默认列以 Unicode 标量计数（utf-32），不是字节或字素簇；LF、CRLF、CR 的分行沿用 LSP。编辑器提供 UTF-16 位置时显式选择 utf-16 并将其零基行列各加 1。
- JSON 为 `schema: 1, kind: "query"`，包含 query、status、positionEncoding、rangeBase、result、diagnostics 和 messages。result 保留原 LSP 数据，rangeBase 为 0；diagnostics 保留 URI、版本及对应诊断。文本格式显示声明/文档与一基位置。
- status 为 ok 时取得正常结果；empty 表示没有匹配；partial 表示本次工作区或正文存在错误诊断，或服务器报告了分析问题，仍返回已确定的信息。普通源码 warning 不将结果标记为 partial。不要仅因退出码 0 就假定源码能够完整编译。
- 查询成功（包括 empty/partial）退出 0。无法读取输入、启动/通信失败或 LSP 请求失败退出 1，用法错误退出 2；失败不发布部分 stdout，stderr 按所选格式报告错误。实际构建/编译验收继续使用 `scoop check/build/test`。

客户端发送 initialize/initialized，必要时发送 didOpen，再执行查询。diagnostics 使用已有 publishDiagnostics；文档查询等待同一版本的诊断，防止将缺失项目误报为“没有类型”。读取服务器通知和响应使用已有 lsp-server 协议实现。结束时发送 shutdown/exit 并回收自己启动的子进程，异常路径也释放管道与进程。

缺少 lock 时由现有 PM 解析入口初始化；之后离线取得已锁定的依赖 metadata。缺失源码、目标库或无效配置会以实际诊断说明，不下载/构建 sysroot，不通过更换工具链或清缓存隐藏错误。
