# Windows Packaging Design

Windows 安装包应尽可能内置全部运行依赖，目标是在干净 Windows
环境中直接安装并运行，避免用户因为缺少运行库、DLL、WebView2、
配置文件或资源文件导致安装失败或启动失败。

当前 GUI 已迁移到 Rust + Slint。Windows GUI 不再依赖 Tauri、
WebView2、Electron、Chromium 或系统 WebView。

## 目标

1. 用户下载 Windows package 后可以直接运行 GUI、CLI 和 server。
2. 安装或解压过程不要求用户手工安装运行库、DLL 或开发工具链。
3. GUI 可以启动、显示状态、保存服务端地址并进入连接流程。
4. 剪贴板同步、键鼠共享、文件传输由后续 daemon/platform 层执行。
5. 依赖不能内置时，必须自动检测并给出清晰错误或修复提示。

## 平台

| 项 | 策略 |
|----|------|
| 支持系统 | Windows 10 x64、Windows 11 x64 |
| CPU 架构 | x64；arm64 暂不发布 |
| 第一阶段格式 | portable zip |
| 后续格式 | NSIS/MSI 或其他轻量 installer |
| GUI runtime | Slint native renderer |
| 禁止依赖 | Tauri、WebView2、Electron、Chromium、Flutter engine、Qt |
| 开发工具链依赖 | 不允许安装后依赖 Rust、Node、Python、Visual Studio |

## 安装和数据路径

| 类型 | 路径 |
|------|------|
| portable zip | zip 解压目录，包含 `glide.exe`、`glide-gui.exe`、`glide-cli.exe`、`glide-daemon.exe`、`glide-server.exe` |
| 后续 per-user 安装 | `%LOCALAPPDATA%\Programs\Glide` |
| 后续 per-machine 安装 | `%ProgramFiles%\Glide` |
| 用户配置 | `%APPDATA%\Glide\config.json` |
| 用户日志目录 | `%LOCALAPPDATA%\Glide\logs` |

GUI 首次启动需要创建用户配置目录和日志目录。服务端地址保存到用户配置，
重启后继续使用，不能依赖源码目录、开发目录或构建机路径。

## 内置依赖

| 依赖 | 打包策略 |
|------|----------|
| `glide.exe` | Slint GUI 主程序，作为用户双击入口 |
| `glide-gui.exe` | Slint GUI 主程序别名，便于诊断和与文档/CI 命名一致 |
| `glide-cli.exe` | portable zip 内置 |
| `glide-server.exe` | portable zip 内置 |
| MSVC / C runtime | Windows MSVC target 优先静态 CRT；脚本检查 VC runtime 依赖 |
| 图标资源 | 从 `crates/glide-gui/assets` 进入后续 installer |
| 默认配置 | 首次启动自动生成用户配置 |
| TLS 根证书 | 使用 Windows 系统证书存储和 Rust HTTP/TLS runtime |

## 外部依赖

| 依赖 | 是否内置 | 原因和处理 |
|------|----------|------------|
| WebView2 Runtime | 不需要 | GUI 已迁移到 Slint，不应检测或安装 WebView2 |
| Windows clipboard API | 不内置 | 操作系统能力，Windows 自带 |
| Windows tray / shell API | 不内置 | 操作系统能力，Windows 自带 |
| Windows input injection / hook API | 不内置 | 操作系统能力；如需驱动级能力必须单独设计签名和安装 |
| 防火墙规则 | 不自动创建 | 客户端主动连接服务端通常不需要入站规则；服务端模式需提示用户 |
| 开机自启动 | 暂不默认启用 | 后续启用时通过注册表/计划任务写入，并在卸载时清理 |

项目不依赖 .NET、Node/Electron runtime、Qt、GTK、wxWidgets 或外部
Python/PowerShell 模块运行。PowerShell 仅用于测试脚本，不是运行依赖。

## 安装阶段检测

第一阶段 portable zip 没有安装阶段；Windows VM 验证脚本必须检查：

1. 操作系统版本和 CPU 架构。
2. 解压目录可读写。
3. 用户配置目录和日志目录可写。
4. `glide.exe` 可以启动并保持运行。
5. 主程序不导入 `WebView2Loader.dll`。
6. 主程序不包含 Tauri 配置或开发机路径。
7. `glide-cli.exe` 和 `glide-server.exe` 存在。

后续 installer 需要补充快捷方式、卸载清理、开机自启动和防火墙提示检测。

## 连接和剪贴板能力

第一阶段 Slint GUI 通过 `GuiBackend` trait 访问 mock backend，页面包含状态、
设备、配对、日志、设置、平台能力和关于。真实网络连接、剪贴板监听、键鼠
控制和文件传输必须在 daemon/platform 层实现，再通过 Named Pipe 接入 GUI。

连接服务端只需要服务端 URL 和后续 token/pairing 配置。连接失败时 UI/命令
必须显示阶段化错误，不允许只显示“连接失败”。

## 升级和卸载

portable zip 由用户替换目录完成升级。后续 installer 覆盖安装应保留
`%APPDATA%\Glide\config.json` 和日志目录。卸载应移除安装目录、快捷方式和
卸载注册项，但不删除用户配置，除非用户明确选择清理。

当前未自动写入防火墙规则或开机自启动项，因此卸载阶段不应残留这些项目。

## 依赖完整性测试

| 脚本 | 用途 |
|------|------|
| `scripts/check-windows-package-deps.ps1` | 检查 portable zip/安装目录的 exe、DLL、无 WebView2/Tauri 依赖、无构建机路径 |
| `scripts/test-windows-installer.ps1` | 后续 installer 静默安装，并调用依赖检查和 GUI 启动 smoke test |
| `scripts/test-windows-installed-client.ps1` | 对已安装客户端做启动、目录、服务端可达和剪贴板 smoke test |

Release workflow 当前只应输出 Windows portable zip。`Glide_*_x64-setup.exe`
和 `.msi` 属于旧 Tauri/NSIS 构建路线，当前 Slint GUI 阶段不应再作为有效
Windows 下载项出现；workflow 中如果发现这些旧安装器残留，应直接失败。
后续恢复 installer 时，必须先实现新的 Slint installer，并在 Windows package
job 中运行 `check-windows-package-deps.ps1`。

## 干净 Windows 验证矩阵

| 环境 | 要求 | 当前状态 |
|------|------|----------|
| Windows 10 x64 clean VM | 解压、启动、连接、剪贴板、删除目录 | 待执行 |
| Windows 11 x64 clean VM | 解压、启动、连接、剪贴板、删除目录 | 待执行 |
| Windows x64 without Visual Studio | 不缺 VC runtime | 待执行 |
| Windows x64 without Rust/Node/Python | 不依赖开发工具链 | 待执行 |
| Windows arm64 | 暂不支持发布 | 不适用 |

portable zip 验证示例：

```powershell
Expand-Archive .\glide-0.1.0-windows-portable.zip -DestinationPath .\glide

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/check-windows-package-deps.ps1 `
  -InstallDir .\glide\glide-0.1.0-windows-portable `
  -MainExe glide.exe `
  -LaunchSmoke

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-installed-client.ps1 `
  -InstallDir .\glide\glide-0.1.0-windows-portable `
  -Server http://aicode.soocoo.xyz:8080
```

安装版端到端验证还必须覆盖 Windows GUI 与 Linux CLI/Linux GUI 的双向剪贴板
同步、服务端重启后自动恢复、安装版和开发版配置路径差异，以及日志中不泄露
token 明文。
