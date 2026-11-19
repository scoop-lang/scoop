# macOS 可执行文件体积实施记录

## Mach-O 最终链接

普通发布 executable 已启用 LLD dead strip。以同一份发行 sysroot 和相同输入验证，Hello 为 760,448 字节，file-report 为 1,013,424 字节，自绘 Counter 为 17,685,856 字节，真实段落为 16,218,272 字节。Counter 的 ICU 数据进入 dead 列表，段落程序的 ICU 数据仍被引用。文件读取成功和失败、Objective-C、C++ 静态构造、移动 GC、异常栈、native-app 动态库和独立调试文件均经正式入口验证。[基线](FINDINGS.md)
