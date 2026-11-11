# 命令行 API 查询

状态：已实现。`scoop api` 是安装在 `<SYSROOT>/commands/scoop-api` 的独立阅读器，直接读取 `scoop doc` 发布的 API 数据。无需启动编译器或语言服务器。

```sh
scoop api search "Option orElse"
scoop api show assertEquals --cone scoop.test --format json
scoop api show orElse --receiver Option
scoop api search Process --target aarch64-apple-darwin
scoop api show parse --docs ./api-docs
```

`--docs` 指向包含 `api/index.json` 的具体站点；多根文档选择对应根的子站点。默认目录是安装根下的 `docs/<宿主默认 target>`；指定 `--target` 时选择对应安装文档，显式项目站点则核对记录中的目标。找不到数据时，显式生成项目文档或更新安装文档；阅读器不代为准备。sysroot 的选择沿用 `scoop --sysroot PATH api ...`，直接运行阅读器也可传 `--sysroot PATH`。

search 按空白分词，所有词都必须命中该条目的名称、限定名、坐标、package、receiver、文档、别名或公开导入路径。精确名称优先，其余按坐标和限定名稳定排序。默认显示 20 项，`--limit` 和 `--offset` 控制显示并返回 total；这是显示分页，不改变程序是否合法。show 精确匹配名称、限定名、公开导入路径或文档键，返回全部相关重载；`--cone` 接受完整坐标或其中的库名，`--receiver` 对显式扩展 receiver 或成员所属类型的限定名筛选。例如 `show communicate --receiver ChildProcess` 可以找到实例方法。

JSON envelope 为 schema 1、kind（search/show）、status（ok/empty）、target、total、offset、items。items 含声明事实和真实可导入路径；成员的 declaringType 从已发布 parent 取得 key 和 qualifiedName，表示声明所在类型，不推断调用处的泛型实参。多个匹配不自动当作唯一候选。正常查询包括 empty 退出 0；错误为 stderr 上的 schema 1/kind error/message/code，读取或格式错误退出 1，非法参数退出 2。stdout 不混入进度或提示。

每个目标站点发布：

- `api/index.json`：schema、target、cones（文档 key、完整 coordinate、相对数据文件）。
- `api/<key>.json`：schema、coordinate、declarations、bindings。声明包含 key、alternateKeys、parent、package、name、qualifiedName、kind、visibility、property（getter/setter 各自的可见性）、signature、receiver、equalityRequirements、aliases、source、url、documentation、relations。
- bindings 保存 package、name、target、reexport；阅读器用这些公开绑定构造导入信息，成员使用其所属顶层声明的绑定。跨 Cone 的 re-export 仍列出实际提供该导入路径的依赖坐标。未发布目标声明的绑定保持可检索，并明确声明详情缺失。导入信息同时保留 package/name 和可复制的 path；path 的每一段使用 Scoop 反引号，正确表达关键字、空格或含点的名称。限定名只是显示/查询名称，有歧义时使用文档 key。

签名与 receiver 来自文档投影，包含普通／挂起函数、泛型和成员要求，沿用文档对默认表达式的 `…` 显示；需要默认值正文时查看给出的源码。source 保存可用路径、UTF-8 byte range、一基行号及文档源码链接；源码未提供时不伪造路径。documentation 只转写原注释和标签，包括已记录的 throws、资源责任与示例引用；null 表示未记录。目标字段表示这份数据的实际选择，不还原未记录的源码条件，也不承诺其他目标有相同接口。

查询结果只说明文档中有哪些 API。使用前仍需把对应库加入 Cone 依赖并 import；当前表达式的类型、可见候选和精确重载使用 `scoop-lsp query`，行为和链接使用正式 CLI 验证。内部文档缓存、HTML 内容与 `.slib` 格式都不是本命令的读取协议。
