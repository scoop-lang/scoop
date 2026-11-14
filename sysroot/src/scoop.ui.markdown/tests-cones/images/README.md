# 图像端到端验收

此 Cone 使用 `scoop test`、真实工作线程、原生窗口与图片像素，覆盖普通资源、自定义行内图像、Mermaid 和 MathJax。队列用例还验证跨视图共享、版本命中、失败重试、旧快照、关闭时不等待，以及图像等待期间 Markdown 解析仍能完成。

预先准备 mmdc、Node、`@mathjax/src` 与 rsvg-convert。当前本机验收配置为 mmdc 12.2.0、MathJax 4.1.3、rsvg-convert 2.63.4。mmdc 需要 Puppeteer 浏览器；已有 Chrome 时可用 `PUPPETEER_EXECUTABLE_PATH` 显式指定，其临时浏览器由 Puppeteer 管理。Scoop 库和测试不自动安装工具；例如宿主可先显式运行：

```sh
npm install --prefix /absolute/tool-directory --save-exact @mathjax/src@4.1.3
```

从仓库根运行：

```sh
~/.scoop/bin/scoop --sysroot ~/.scoop fmt sysroot/src/scoop.ui.markdown sysroot/src/scoop.ui.markdown/tests-cones/images
~/.scoop/bin/scoop --sysroot ~/.scoop check sysroot/src/scoop.ui.markdown/tests-cones/images --tests --release
PUPPETEER_EXECUTABLE_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
SCOOP_TEST_MMDC=/opt/homebrew/bin/mmdc \
SCOOP_TEST_NODE=/opt/homebrew/bin/node \
SCOOP_TEST_MATHJAX=/absolute/tool-directory/node_modules/@mathjax/src \
SCOOP_TEST_RSVG=/opt/homebrew/bin/rsvg-convert \
~/.scoop/bin/scoop --sysroot ~/.scoop test sysroot/src/scoop.ui.markdown/tests-cones/images --release
```

测试打开一个原生窗口，读取实际光栅像素，并比较公式的原生占位宽度与 MathJax 返回的 advance。相对图片位于临时 include 文件旁，URI 含空格与中文的百分号转义。临时资源在 Scoop 测试退出时删除；外部工具正常输出和中间图片也由适配器清理。

库、调度与测试均由 Scoop 实现。`tools/mathjax/render.mjs` 是随 UI Cone 嵌入的外部 MathJax 进程入口，使用 MathJax 的 SVG 排版结果；测试与示例直接消费公开的 Scoop 适配器。
