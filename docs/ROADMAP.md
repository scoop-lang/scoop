# Scoop 实现路线图

Scoop 尚未发布首个版本。语言、库和工具随主线一起演进。开发按纵向功能推进，每项能力从源码写法贯通到产物消费、链接与执行。公开行为、运行时边界及编译阶段的输入输出分别由三份规范定义。[语言规范](specs/SCOOP-SPEC.md) · [运行时规范](specs/SCOOP-RUNTIME-SPEC.md) · [编译器与产物规范](specs/SCOOP-IMPL-SPEC.md)

## 当前能力

| 领域 | 已有能力与维护入口 |
| --- | --- |
| 语言与编译 | 类型检查、值类型、模式匹配、泛型单态化、协程、异常、C FFI。编译器支持多 Cone 工程和跨产物链接。[语言规范](specs/SCOOP-SPEC.md) · [编译器与产物规范](specs/SCOOP-IMPL-SPEC.md) |
| 运行时 | 分代 Immix、精确栈根、线程协作、异常与原生栈，以及 runtime image 注册。[运行时规范](specs/SCOOP-RUNTIME-SPEC.md) · [实现入口](../runtime/README.md) |
| 系统库 | core 与 std 提供类型、容器和编码协议。平台、异步 I/O、网络及图形系统以独立 Cone 交付。各库规范列出公开合同与平台条件。[系统库源码](../sysroot/src/) · [领域规范](specs/) |
| 开发工具 | 构建、包管理、测试、格式化、文档生成、LSP、编辑器和原生调试工具可从安装包使用。[包管理规范](specs/SCOOP-PACKAGE-SPEC.md) · [编辑器工具](../tooling/README.md) · [调试交付范围](debugging/IMPLEMENTATION.md) |
| 目标与发行 | 编译器登记了 macOS、iOS、Linux 和 Windows 目标。具体 CPU、ABI、native 后端和发行资源按目标配置。[目标合同](specs/SCOOP-IMPL-SPEC.md) · [发行配置](../scripts/release.toml) · [构建与验证](../scripts/README.md) |

## 后续工作

- 编译器性能优先处理真实工程中耗时或内存占用最大的阶段。冷编译、缓存命中和最终链接分别测量。[编译性能记录与后续优先级](compiler-performance/PROGRESS.md)
- 程序执行性能按工作负载比较，GC、代码生成和内置库的改动分别保留正确性覆盖。[基准入口](../benchmark/README.md) · [已启用的优化](program-performance/ENABLED-OPTIMIZATIONS.md)
- 编辑器、调试器、桌面及控件库分别记录其功能边界。DataGrid 在独立仓库实施，各平台的实际支持与验收以所属文档为准。[语言服务](language-service/PROGRESS.md) · [桌面与控件规范](specs/SCOOP-UI-COMPONENT-SPEC.md) · [DataGrid 设计](ui/data-grid/DESIGN.md)
- 仍待实施或明确的工作集中记录在[待办清单](TODO.md)。

## 历史记录

M1 至 M62 的设计、阶段验收、实验数据及原完整路线图保存在相邻的 `../scoop-doc-archive/docs/`。历史版本也可从 Git 查阅。仍在使用的生成器协议和 JSON 消息定义保留在当前仓库。[归档时的文档目录](https://github.com/scoop-lang/scoop/tree/aa937c52e02120837e78c9a0c8b9572fd318e3a1/docs) · [生成器协议](generator-protocol/PROTOCOL.md)
