# Windows 支持与维护入口

Windows 的三个正式 x64 target 已接入编译器、runtime、系统库和发行流程。完整名称、默认选择与 SDK／CRT 合同以[实现规范第 1 节](../specs/SCOOP-IMPL-SPEC.md#1-范围与目标兼容性)为准；不能混用 Microsoft、LLVM-MinGW MSVCRT 和 LLVM-MinGW UCRT 的原生资源。

- **编译和运行时。** COFF／PE、Win64 ABI、SEH／statepoint、精确 GC 根、线程、初始化、Unicode 参数、subsystem 和 RC 资源的实现约定见[Windows 设计](DESIGN.md)与[运行时规范](../specs/SCOOP-RUNTIME-SPEC.md)。
- **工具链和发行。** Windows 主驱动为 `sco.exe`，调试工具使用 `scoop-` 前缀。构建 LLVM、选择 SDK 和准备开发 sysroot 的实际命令见[准备说明](../../scripts/README.md)；发行组件与目标资源见[发行设计](../distribution/DESIGN.md)。
- **系统库。** platform 使用 Windows Unicode 接口，asyncio 使用 IOCP，terminal／readline 使用 Windows console 与 ConPTY。公开 API、资源所有权和平台条件分别由[平台规范](../specs/SCOOP-PLATFORM-SPEC.md)、[异步 I/O 规范](../specs/SCOOP-ASYNCIO-SPEC.md)和[终端规范](../specs/SCOOP-TERMINAL-SPEC.md)定义，各 Cone 清单选择实际的 Windows 源码与 native 依赖。
- **调试。** CodeView／PDB、伴随文件发布、LLDB formatter 与 DAP 共用主线调试流程。可观察的值和优化限制见[调试支持范围](../debugging/IMPLEMENTATION.md)，协议维护入口见[调试验收说明](../../tooling/vscode-scoop/test/debugger/README.md)。

W0～W4 的旧分支进度、环境版本及逐轮验收保存在相邻归档 `../scoop-doc-archive/docs/windows-targets/PROGRESS.md`，也可从 [Git 历史](https://github.com/scoop-lang/scoop/blob/f93c4b46082def07aa51103567e2d6a2ea0978fc/docs/windows-targets/PROGRESS.md)查阅。该记录用于追溯历史构建，不替代当前规范或发行配置。
