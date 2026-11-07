# `scoop-bundle` 实施记录

范围遵循 [设计](DESIGN.md)：首期为 app、deb、rpm、AppImage、NSIS、MSI；Flatpak/MSIX 后续单独实施。原始日志、上游源码调研和临时验证工程放在仓库外。

## 2026-11-08：公共接口

- Cone `[bundle]` 保留为工具 metadata；PM 成员协议版本 6 交付原始表、来源 span 与展开的发布 metadata。普通编译不解释打包配置。
- build JSON 提供实际 canonical target；`scoop.platform.env.executablePath()` 使用三平台原生接口返回主程序真实位置，资源定位不依赖 cwd 或 argv[0]。
- `cargo fmt --all`、完整 workspace clippy 与相关 manifest/protocol/PM 的 198 项 Rust 测试通过。
- macOS 的实际 Scoop 平台测试运行 5 项，全部通过；原生路径探针覆盖直接启动、符号链接、其他 cwd 和不同 argv[0]。
- 实际 Scoop CLI 构建验证 JSON target，并确认只改 bundle 显示名称后没有编译子进程调用；源码、编译器和输出始终保留。
- 本轮临时验证工程已清理；Linux/Windows 原生实现等待各平台阶段验证。打包器后端尚未交付，真实 Developer ID 签名尚未验收。
