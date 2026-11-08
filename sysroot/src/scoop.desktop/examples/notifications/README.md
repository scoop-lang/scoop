# 系统桌面通知示例

此独立示例只依赖 desktop／window 与基础库。main 在 Application 之前准备通知激活入口；收到默认点击或 “Open & 你好” 动作后打开原生窗口，标题显示完整 payload。Clear 动作明确请求撤回。示例不自行安装注册或修改系统权限。

先准备当前目标的预编译库，再构建：

```sh
scoop build sysroot/src/scoop.desktop/examples/notifications --release --sysroot sysroot -o /tmp/notification-demo
scoop bundle --executable /tmp/notification-demo --target aarch64-apple-darwin --config sysroot/src/scoop.desktop/examples/notifications/Bundle.toml
```

上例选择 macOS Apple Silicon；其他平台应将 `--target` 改为实际完整目标。Windows 使用 `sco` 和对应 `.exe` 路径；MSVC 版本在示例配置中选择系统 VC runtime，因此运行机器需要已安装对应运行库。打包工具、GUI 链接依赖及打包格式按当前平台配置。macOS 应运行生成的 `.app`，Windows 应先安装生成的 NSIS／MSI，Linux 需要会话通知服务。

| 参数 | 行为 |
| --- | --- |
| `--check` | 查询设置后退出，不发送通知或请求授权 |
| `--permission` | 显式请求系统权限；无独立授权 API 的平台仅查询设置 |
| `--send` | 发送后保持运行，接收原生动作 |
| `--send-and-exit` | 提交后退出，用 macOS／Windows 的系统通知点击检查冷启动 |
| `--replace` | 连续提交同一 ID，第二次更换正文 |
| `--replace-and-withdraw` | 在同一进程中发送、替换、撤回后退出 |
| `--withdraw` | 请求撤回；Linux 新进程没有旧服务 ID 映射时返回 NotFound |
| `--bounded` | 最多运行 45 秒，适用于有界桌面验证 |

事件追加到系统临时目录的 `scoop-notification-demo.log`。通知包含 XML 特殊字符、引号与中文；动作 ID 为 `open&你好`，payload 为 `payload "<>&" 你好`，用于检查原文传递。系统接受请求不代表通知已经显示；静音、系统专注模式和用户通知设置仍由 OS 决定。

macOS 的冷启动验证需经系统授权并点击实际通知；Windows 的冷启动需安装期 COM 注册和正常 main 入口配合。Linux 此后端只在发送进程存活时接收动作。关闭示例窗口会正常退出应用。
