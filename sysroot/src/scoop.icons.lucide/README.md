# scoop.icons.lucide

提供 `lucide-static 1.70.0` 中选定的 31 个原始 SVG 图标。每个 lowerCamelCase 入口返回不可变 SvgAsset；读取资源不会解析 SVG，也不会创建 native 对象。清单见 [icons.txt](icons.txt)，ISC 和适用的 Feather MIT 声明完整保存在 [LICENSE](LICENSE)。

```scoop
import scoop.icons.lucide.search
import scoop.ui.component.*

val view = icon(search) {
    size = 32.0
    absoluteStrokeWidth = true
}

val action = button({ style -> style.icon(search) }, { println("Search") })
```

图标库本身只依赖 graphics.svg，不依赖 UI。离屏或自定义绘图可以 `SvgDocument.parse(search.source)` 后调用 Canvas.drawSvg。普通 icon 由挂载作用域管理文档，更新颜色、尺寸和描边时复用，来源改变或卸载时释放。`svgView` 则借用调用者已经持有的文档。

使用新版编译器重新构建本库及依赖后，debug／release 的最终链接均按实际引用裁剪资源。未使用图标的 SVG、独立 getter、callable 登记和 immortal 登记不会进入成品；新增未使用图标也不增加这些内容。已内联的 getter 可以消失，所用资源继续保留。应用或 UI 组件实际引用的默认图标仍会保留，主动构建全量资产数组或运行时图标索引也会保留它引用的图标。

macOS aarch64 release 的真实离屏消费者已验证零／单／全部引用分别只保留 0／1／31 份 SVG；仅用 search 时为 352 字节原始 SVG，新增一个未引用图标后保持不变。完整文件还含 Skia、runtime 等共享代码，不能将其大小全部归为图标开销。实现、构建成本和各平台验收见[静态常量与图标资源裁剪](../../../docs/program-performance/IMMORTAL-PRUNING.md)。

维护入口为 `python3 scripts/update_lucide.py --archive /path/to/lucide-static-1.70.0.tgz`；不指定归档时从仓库已有资产重新生成独立声明，`--check` 检查声明和资产基线。维护脚本不下载文件，应用构建也不需要 npm。所有入选资源由本 Cone 的 scoop.test 实际解析并绘制。
