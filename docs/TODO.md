# 待办事项

本清单记录仍有具体实现或验证缺口的工作。新增语言行为、公开接口或 ABI 时，先更新所属规范。领域项目的进度见[实现路线图](ROADMAP.md)。

- [ ] **条件依赖与测试进程。** PM 按目标筛选生产和测试依赖。生成的测试入口按 `@Test(useProcess = true)` 逐例启动子进程并收集结果。两项可分别实施。[设计与验收](conditional-dependencies-and-test-processes/DESIGN.md)
- [ ] **协程调试。** 建立挂起位置与源代码行的映射，让调试器能识别恢复入口、显示逻辑协程栈并定位保存的局部变量。现有物理栈和原生调试信息可作为起点。[调试实施记录](debugging/IMPLEMENTATION.md)
- [ ] **增量编译的查询依赖。** 缩小重编译范围前，需要跟踪空查询结果、被过滤的候选、星号导入和再导出，以及实际使用的默认参数与模板依赖。以新增声明能够改变原调用绑定的工程验证失效行为。[编译器与产物规范](specs/SCOOP-IMPL-SPEC.md)
- [ ] **运行时入口文档。** 按实际导出入口核对参数、返回值、线程状态、所有权和失败路径。优先补充头文件与规范中仍只描述功能的接口。[公开头文件](../runtime/include/scoop_rt.h) · [运行时规范第 4 节](specs/SCOOP-RUNTIME-SPEC.md#4-scoop-abi-ffi-runtime-functions)
- [ ] **编译和执行性能。** 以现有工程与基准复测已记录的 CPU、峰值内存和 GC 热点，记录改动的收益及回退。[编译性能记录](compiler-performance/PROGRESS.md) · [运行基准](../benchmark/README.md)

原清单中的带标签控制流、源码级调试行表、受检 UTF-8 解码和 native 字节缓冲均已有实现。对应合同可从[语言规范](specs/SCOOP-SPEC.md)、[编译器规范](specs/SCOOP-IMPL-SPEC.md)和[平台库规范](specs/SCOOP-PLATFORM-SPEC.md)查阅。
