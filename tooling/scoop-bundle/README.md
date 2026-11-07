# scoop-bundle

将已经链接的 Scoop 可执行程序和附加文件打包为应用。外部命令安装在 sysroot 的 `commands/`，通过 `scoop bundle`（Windows 为 `sco bundle`）调用。

设计与阶段范围见 [设计文档](../../docs/scoop-bundle/DESIGN.md)，验证进度见 [实施记录](../../docs/scoop-bundle/PROGRESS.md)。当前交付 macOS app 与 Linux deb、rpm、AppImage；Windows 后端在同一首期分支继续实施。

```sh
python3 scripts/build_scoop_bundle.py --sysroot ./sysroot

# Installed toolchain.
scoop bundle ./my-application --format app

# Development checkout with explicitly paired tools.
SCOOP_SYSROOT="$PWD/sysroot" target/release/scoop bundle \
  tooling/scoop-bundle/examples/resources \
  --scoop "$PWD/target/release/scoop" --format app

# Existing program: no PM or compiler invocation.
scoop bundle --executable ./dist/editor --target aarch64-apple-darwin \
  --config packaging/Bundle.toml --format app
```

项目配置位于 `Cone.toml` 的 `[bundle]`，也可以仅声明 `config = "packaging/Bundle.toml"`。独立文件仍使用 `[bundle]` 表；已有程序模式必须提供 name、version、identifier。命令行 `--config` 选择完整外置配置，相对文件路径以该配置文件所在目录为基准。

资源示例见 [Cone.toml](examples/resources/Cone.toml) 与 [main.scoop](examples/resources/src/main.scoop)。程序通过 `executablePath()` 访问固定相对布局，在其他 cwd 启动同样可用。

macOS 默认执行 ad-hoc codesign；真实证书在 `[bundle.macos.signing]` 显式选择 `mode = "identity"` 与 `identity`。主应用和辅助 executable 的 entitlements 分别声明。`--force` 强制重新部署和签名；普通命中不查询钥匙串或读取已签产物。

工具不会下载或安装打包依赖。macOS 使用系统 `install_name_tool`、`codesign`，PNG 转 ICNS 需要 `iconutil`；可通过 `[bundle.tools]` 指定工具路径。Linux 使用 `patchelf`、`ldconfig`、`dpkg-deb`；RPM 直接通过锁定的格式库生成。AppImage 需要预先准备 `appimagetool` 和 type-2 runtime（命名为 `appimage-runtime` 放入 PATH，或在 `[bundle.tools]` 指定路径），每次调用都显式传入 runtime，避免上游在线下载。

[GUI 示例](examples/gui/Cone.toml) 打开真实窗口、读取资源并在两秒后退出，适合验证打包后的事件循环。Linux GUI 使用 GTK 4.18 以上、libadwaita 1.4 以上；deb/rpm 的发行版依赖必须由应用声明，AppImage 的构建系统须满足最低运行系统的 glibc 基线。动态加载模块、GSettings schema 等非 ELF imports 输入使用 files 显式加入。

Developer ID 公证、Flatpak 与 MSIX 尚不属于首期。

维护验证：先 `cargo fmt --manifest-path tooling/scoop-bundle/Cargo.toml` 和对应 `cargo clippy --all-targets -- -D warnings`，再运行对应 `cargo test`。macOS 集成测试实际构建含 framework 和传递 dylib 的程序，覆盖资源、签名、更新复用和失败保留旧包；临时工程自动清理。

Linux 集成测试需要 `cc`、`dpkg`、`rpm`、`patchelf`、`ldconfig`、`appimagetool` 与 `appimage-runtime`。它在临时目录中安装、升级和卸载测试包，不修改宿主的包数据库，并通过普通 FUSE 入口运行 AppImage。
