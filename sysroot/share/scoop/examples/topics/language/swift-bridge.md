## 运行

仅在 macOS 上运行。按显示路径保存全部文件，在 Cone.toml 所在目录执行：

```sh
scoop run --release .
```

需要指定 Swift 工具链时，用 `--swift-toolchain /path/to/Toolchain.xctoolchain`。工具链必须支持正式 `@c` 和 Swift Build 后端。

## 预期结果

```text
true
true
```

第一个函数创建 Helvetica 字体并读取字号；第二个函数读取该字体的 glyph 数量。它们分别演示 Double 和 Int32 的 C ABI 返回值。

## 关键说明

SwiftPM 将 SystemBridge product 构建为 `.a`。Scoop 使用已有 `@Extern` 和 NativeSafe 调用边界；`Int` 对应这里的 Swift Int32，Swift Double 对应 Scoop Double。Swift 对象留在桥接函数内部，由 Swift／系统管理；接口仅传递标量。

最终 framework 依赖属于 Cone，通过 native.libraries 显式声明。Package.swift 只负责 Swift 包自身构建，Scoop 不分析它来生成另一套依赖。包目录加入缓存；如果将支持代码移到包外，或引入第三方 framework，须在 Cone.toml 的 native.swift.inputs 中列出相应文件／目录。

只改 Fonts.swift 时，driver 更新 Swift 库和最终链接，复用 Scoop `.slib`。无变化的构建直接命中，不调用 SwiftPM。offline／frozen 可以复用已有 Swift 结果；未命中需先正常构建。

Cone 中附带最小 bundle metadata。本例只用系统 framework，不需要私有 framework 清单；bundle 不复制或修改 CoreText／CoreFoundation。引入第三方动态 framework 时，在 bundle metadata 的 files 中显式列出 from、to 和 kind = framework，打包器在副本上调整加载路径再签名；链接仍使用正常库搜索目录。

## 常见误用

- Swift product 使用显式 `.static`，automatic library 不保证生成独立 `.a`。
- `@Extern` 的 lib 与 product 名一致，name 与 `@c` 导出的 C symbol 一致。
- 不把 Swift String、Array、class 或原生 Swift calling convention 直接写成 Scoop 的 C ABI 声明。
- `.a` 不代表 Swift runtime 全静态；实际 runtime／compatibility 库按机器码引用解析。
- 跨 Cone 暴露声明时给 Scoop 的 extern 函数加 public。自动 Swift 包装和 bindgen 属于下一阶段。
