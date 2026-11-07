# scoop-bundle

将已经链接的 Scoop 可执行程序和附加文件打包为应用。外部命令安装在 sysroot 的 `commands/`，通过 `scoop bundle`（Windows 为 `sco bundle`）调用。

设计与阶段范围见 [设计文档](../../docs/scoop-bundle/DESIGN.md)，验证进度见 [实施记录](../../docs/scoop-bundle/PROGRESS.md)。首期格式为 macOS app、Linux deb/rpm/AppImage 和 Windows NSIS/MSI。各宿主的实际验证范围以实施记录为准。

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

Windows 使用 NSIS 3.13 与 WiX 5.0.2 验收，WiX UI 扩展须与 CLI 同版本。维护环境预先准备 `makensis.exe`、`wix.exe` 和 `WixToolset.UI.wixext.dll`，放到 PATH 或用 `bundle.tools` 指定路径；工具本身不会下载或安装它们。WiX 6/7 的 OSMF 协议由使用者自行处理，工具不自动接受协议。生成 MSI 还须提供 `bundle.publisher` 和稳定的 `bundle.windows.msi.upgrade-code`。

WiX 5 的 CAB 中间目录受系统 ANSI 代码页限制。后端使用系统临时目录，需将 `TEMP` 配置为该代码页可表示的路径；应用输入、metadata 和最终产物继续支持 Unicode。

两种 Windows 安装器默认 `install-mode = "per-user"`，也支持 `per-machine`。同格式、同范围可以升级；切换格式/范围需要先卸载。安装器只删除自己安装的文件，保留用户新增的数据。Windows 内嵌 exe 图标和版本仍使用正常构建的 RC 输入。

GUI 示例通过 `[windows]` 选择 GUI subsystem 和 RC manifest，在进程启动时声明窗口库要求的 Per-Monitor V2 DPI 模式；这些设置属于正常编译输入。窗口运行验收需要交互桌面，Windows SSH 的非交互会话不提供相同的 DPI 环境。

VC runtime 依赖需要显式选择 `bundle.windows.vc-runtime = "system"` 或 `"app-local"`；后者从 files/library-search-paths 提供可分发 DLL。已有程序的 DLL 搜索不使用 PATH，导入、延迟加载和导出转发中的私有依赖随包部署。API-set 和系统 UCRT 由 Windows 提供。

可选 `[bundle.windows.signing]` 通过 `certificate-thumbprint` 引用证书存储，并提供 `timestamp-url`；`certificate-store` 默认 `current-user`，也可以为 `local-machine`。签名先覆盖 payload，再覆盖 NSIS 内部卸载器与最终安装包。配置只保存证书引用，私钥和密码不进入配置。

Windows 集成测试需要 Visual C++ 构建环境、`clang-cl`、`lld-link`、上述安装工具和 PowerShell。测试使用独立的当前用户安装标识，覆盖自定义目录、DLL/资源运行、NSIS/MSI 升级、MSI 修复和正常卸载；结束后清理测试安装。

设置 `SCOOP_BUNDLE_TEST_SIGNING_THUMBPRINT` 和 `SCOOP_BUNDLE_TEST_TIMESTAMP_URL` 可运行同一套安装生命周期的签名专项，同时检查安装器、安装后的 exe/DLL 与 NSIS 卸载器的 Authenticode 签名。证书需由维护环境预先准备并受该环境信任；普通测试不创建证书或修改信任存储。
