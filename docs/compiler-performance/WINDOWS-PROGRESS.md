# Windows 编译性能实施记录

工作分支：`codex/windows-compile-performance`。起点：`f27eda5ee1`。工作范围与验收条件见[任务单](WINDOWS-TASKS.md)。

每个大项先完成本地实现和定向验证，再提交、推送并合并 `origin/main`。Linux/Windows 按大项集中复验，全部完成后分别制作安装包；不逐小改动重复全量测试。

## 当前状态

| 项目 | 状态 |
| --- | --- |
| W0 Windows 基线 | 两台远程主机已连通并建立独立工作目录，测量待进行 |
| W2 检查成功缓存 | 实现及 macOS 定向验证完成，Linux/Windows 待集中复验 |
| W10 编译进度 | 下一项 |
| W1/W3～W9 | 待实施 |
| W11 三平台安装包 | 待全部实现和验证完成后执行 |

Linux 工作目录为 `~/repos/scoop-windows-performance-20261110`，Windows 为 `E:/scoop-windows-performance-20261110`；均从同一基线建立，未改动已有 checkout。原始报告和日志保存在各自主机的仓库外工作目录。

## 2026-11-10：每次检查根 Cone

删除独立的 `checks-v4` 记录、专用锁、整图输入 key、结果序列化与 `cache_hit` 状态。每次调用直接完成当前根的前端检查；依赖继续按普通构建缓存复用。完成节点不再为检查缓存额外携带 generator 输入副本，检查也不再收集 native 工具状态。同步实现规范、fmt/check 设计、CLI 展示和正式用例。

用例通过实际 child 调用验证每次根检查、源码依赖复用、真实头文件更新、Production/Test 选择、generator 重执行、并发请求、当前 warning 及错误修复。无平台依赖的基础、依赖和测试用途用例开放到 Windows；依赖 POSIX shebang 的 generator 用例保留平台条件。

macOS AArch64 本地结果：

- `cargo fmt --all`、workspace all-targets Clippy 和 Ruff 0.16.10 通过；Clippy 仍报告既有代码在当前 Rust 版本下的警告。
- release 配套 CLI 构建通过；`scoop` CLI 的 40 个单元测试通过。
- fixture runner 规则测试运行 84 项，4 项平台条件跳过，其余通过。
- 12 个检查命令用例通过；另一个缓存/clean 组合用例在更新测试 runtime 后通过，合计 13 个正式用例完成验证。

首次组合用例的链接失败来自已有开发 sysroot 的旧 runtime 索引（10 字段，当前 reader 要求 11 字段）。使用现有准备脚本，仅在本任务的独立 sysroot 更新四种 runtime 配置和头文件，复用已有系统 `.slib`，随后只重跑该失败用例。没有通过修改编译器兼容旧索引或放宽断言处理此环境问题。

本轮结束后清理 fixture 工作目录及不再需要的测试缓存；报告另存仓库外，配套工具、Cargo 缓存和已准备的测试 sysroot 留供后续大项复用。跨平台结果与性能数据尚未取得，不将本地通过写成三平台验收完成。
