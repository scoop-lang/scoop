# scoop-install 实施记录

实现分支均为 `codex/scoop-install`。安装器独立仓库为 [scoop-lang/scoop-install](https://github.com/scoop-lang/scoop-install)，官网在 `scoop-lang/scoop-lang.org` 维护 bootstrap。

| 大项 | 当前结果 |
| --- | --- |
| Scoop sources／vendor 拆分与共享下载目录 | 已提交 `58985050a4`；维护脚本 65 项通过，11 项按平台条件跳过 |
| 明确的组件 manifest 与 CI 合并 | 已提交 `b98cc983d0`；三宿主七目标映射与缺失组件／发行冲突测试通过 |
| 独立安装器、更新、组件增删、代理与项目选择 | 已提交 `8747fa7`、`98c4287`；macOS、Linux 9 项端到端通过，Windows 8 项通过、1 项跨宿主模拟跳过 |
| 缓存发行身份、共享外部命令、编辑器集成 | Node 24 项通过；真实 CLI 覆盖三种缓存入口及共享命令；VS Code 扩展宿主的语义功能、运行操作和多工作区隔离通过 |
| 官网 bootstrap 与发布索引 | 官网提交 `fec15e1`；本机 7 项通过、Windows 5 项通过，其余按平台跳过；完整官网与 Jekyll 指南构建通过 |
| 三平台最终安装包及真实安装验收 | 待完成 |

安装器使用静态 liblzma，macOS 已确认不依赖 Homebrew 动态库。CLI 验收使用本地 HTTP 服务与真实可执行工具，覆盖请求范围、版本和组件选择、更新失败不激活、归档路径／文件归属、并发安装、参数与退出行为。没有通过隐藏或删除原始输入验证缓存复用。

缓存／编辑器集成已提交 `513f212ccf`。安装器补充提交 `2bd5b60` 后，本机 10 项端到端通过；自定义 home 加入 PATH 后可直接使用代理，无需再次设置环境变量。发布脚本只允许覆盖未发布的草稿附件，正式 release 重打包使用新版本。

本机临时内容统一在各仓库 `tmp/`。Linux 安装器测试目录为 `~/repos/scoop-install-src-20261111`，Windows 为 `E:/scoop-install-src-20261111`。原始日志和安装产物不提交 Git，最终包位置与实际平台限制在完成后补充。
