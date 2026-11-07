# scoop-bundle

将已经链接的 Scoop 可执行程序和附加文件打包为应用。外部命令安装在 sysroot 的 `commands/`，通过 `scoop bundle`（Windows 为 `sco bundle`）调用。

设计与阶段范围见 [设计文档](../../docs/scoop-bundle/DESIGN.md)，验证进度见 [实施记录](../../docs/scoop-bundle/PROGRESS.md)。当前阶段交付 macOS app；Linux 和 Windows 后端在同一首期分支继续实施。

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

工具不会下载或安装打包依赖。macOS 使用系统 `install_name_tool`、`codesign`，PNG 转 ICNS 需要 `iconutil`；可通过 `[bundle.tools]` 指定工具路径。Developer ID 公证、Flatpak 与 MSIX 尚不属于首期。

维护验证：先 `cargo fmt --manifest-path tooling/scoop-bundle/Cargo.toml` 和对应 `cargo clippy --all-targets -- -D warnings`，再运行对应 `cargo test`。macOS 集成测试实际构建含 framework 和传递 dylib 的程序，覆盖资源、签名、更新复用和失败保留旧包；临时工程自动清理。
