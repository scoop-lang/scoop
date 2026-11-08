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

独立入口不保证最终逐图标剔除。当前 macOS aarch64 release 消费者实际保留全部 31 份资源，原始 SVG 合计 12,159 字节；链接测量见 [GUI 扩展实施进度](../../../docs/desktop-ui/GUI-EXTENSIONS-PROGRESS.md)。

维护入口为 `python3 scripts/update_lucide.py --archive /path/to/lucide-static-1.70.0.tgz`；不指定归档时从仓库已有资产重新生成独立声明，`--check` 检查声明和资产基线。维护脚本不下载文件，应用构建也不需要 npm。所有入选资源由本 Cone 的 scoop.test 实际解析并绘制。
