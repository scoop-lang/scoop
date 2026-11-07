# `scoop-bundle` 实施记录

范围遵循 [设计](DESIGN.md)：首期为 app、deb、rpm、AppImage、NSIS、MSI；Flatpak/MSIX 后续单独实施。原始日志、上游源码调研和临时验证工程放在仓库外。

## 2026-11-08：公共接口

- Cone `[bundle]` 保留为工具 metadata；PM 成员协议版本 6 交付原始表、来源 span 与展开的发布 metadata。普通编译不解释打包配置。
- build JSON 提供实际 canonical target；`scoop.platform.env.executablePath()` 使用三平台原生接口返回主程序真实位置，资源定位不依赖 cwd 或 argv[0]。
- `cargo fmt --all`、完整 workspace clippy 与相关 manifest/protocol/PM 的 198 项 Rust 测试通过。
- macOS 的实际 Scoop 平台测试运行 5 项，全部通过；原生路径探针覆盖直接启动、符号链接、其他 cwd 和不同 argv[0]。
- 实际 Scoop CLI 构建验证 JSON target，并确认只改 bundle 显示名称后没有编译子进程调用；源码、编译器和输出始终保留。
- 本轮临时验证工程已清理；Linux/Windows 原生实现等待各平台阶段验证。打包器后端尚未交付，真实 Developer ID 签名尚未验收。

## 2026-11-08：公共打包器与 macOS

- 独立 Rust 命令完成内嵌/引用/显式配置、PM/build 交接、已有程序入口、目标条件、统一文件清单、更新复用与按格式暂存发布。target facts 直接使用仓库同一静态目录。
- macOS 完成 Info.plist、PNG/ICNS 图标、framework 链接保留、实际传递 dylib 部署、加载路径修正和由内到外签名。全部修改只发生在副本。
- 独立 Cargo 格式化、clippy 与 6 项公共测试、1 项实际 macOS CLI 集成测试通过。集成覆盖传递原生依赖、空格/中文路径、资源更新、目录新增文件、无变化不再调用签名工具，以及签名失败保留旧包。
- 通过开发 sysroot 的真实 `scoop bundle` 运行资源示例；从 `/tmp` 启动读取资源成功。本机 Developer ID 专项验证通过，签名包含可信时间戳及 hardened runtime；该记录不代表完成公证。
- 维护构建入口和完整工具链发行已接入新命令，依赖许可收集覆盖其独立 Cargo workspace。定向发行脚本测试通过，其他宿主专属检查留待相应平台。
- 本轮临时 C/framework 工程与签名配置已清理；签名样包和必要构建缓存保留在仓库外供后续验收。Linux/Windows 格式继续实施。
