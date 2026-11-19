# macOS 可执行文件体积实施记录

## Mach-O 最终链接

普通发布 executable 已启用 LLD dead strip。以同一份发行 sysroot 和相同输入验证，Hello 为 760,448 字节，file-report 为 1,013,424 字节，自绘 Counter 为 17,685,856 字节，真实段落为 16,218,272 字节。Counter 的 ICU 数据进入 dead 列表，段落程序的 ICU 数据仍被引用。文件读取成功和失败、Objective-C、C++ 静态构造、移动 GC、异常栈、native-app 动态库和独立调试文件均经正式入口验证。[基线](FINDINGS.md)

## Tagged enum 的整字存储

以相同机器布局发射非引用区间和静态枚举常量。重编 Counter 的全部传递依赖后，最终 Mach-O 从 17,685,856 降至 13,062,480 字节。`SurfaceHost.onEvent` 为 102,096 字节、25,524 条 ARM64 指令。七项定向 codegen 单测通过，跨 Cone 大枚举、嵌套枚举返回和移动 GC 共三个 fixture、五种配置通过。真实 `SurfaceHostTest` 通过了帧回调、输入视图与退出检查。

## 按值传递与 GC 返回路径

从已有栈槽直接读取枚举字段并复用栈聚合复制后，`onEvent` 的 `__text` 从 102,096 降至 100,000 字节。另有 12 对 volatile load/store 在正常或异常返回后重写整个事件。沿已发布的扫描描述只重载托管引用槽后，正式构建得到 21,680 字节、5,420 条指令，其中 `ldr/str` 合计 4,707 条。重编 Counter 的传递依赖后，Mach-O 从 13,062,480 降至 12,550,608 字节。

使用真实 `SurfaceHost.onEvent` 向同一 handler 转发 `Window(Focused(true))`，在无窗口绘制的事件循环中每组计时 20,000 次。两个版本交错运行，各排除预热、取 10 组中位数，每事件从 703 降至 413 纳秒。397 项 codegen 测试通过；大枚举、嵌套返回、移动 GC 与抛错栈四项正式 fixture 共十种配置及真实 `SurfaceHostTest` 通过。
