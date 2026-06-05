# Windows Packaging Design

Windows 安装包应尽可能内置全部运行依赖，目标是在干净 Windows
环境中直接安装并运行，避免用户因为缺少运行库、DLL、WebView2、
配置文件或资源文件导致安装失败或启动失败。

## 目标

1. 用户下载 Windows 安装包后可以直接安装。
2. 安装过程不要求用户手工安装运行库、DLL、开发工具链或 WebView2。
3. 安装后 GUI 客户端可以直接启动、保存服务端地址并连接服务端。
4. 安装后文本剪贴板同步、托盘后台运行和键鼠共享入口可用。
5. 依赖不能内置时，必须自动检测并给出清晰错误或修复提示。

## 平台

| 项 | 策略 |
|----|------|
| 支持系统 | Windows 10 x64、Windows 11 x64 |
| CPU 架构 | x64；arm64 暂不发布，Cargo 已预留静态 CRT 配置 |
| 安装包格式 | NSIS `.exe`、MSI `.msi`、portable zip |
| 打包工具 | Tauri 2 bundler (`cargo tauri build --bundles nsis,msi`) |
| GUI runtime | WebView2 |
| 开发工具链依赖 | 不允许安装后依赖 Rust、Node、Python、Visual Studio |

## 安装和数据路径

| 类型 | 路径 |
|------|------|
| per-user 安装 | `%LOCALAPPDATA%\Programs\Glide` 或 Tauri NSIS 默认 per-user 目录 |
| per-machine 安装 | `%ProgramFiles%\Glide` |
| 用户配置 | `%APPDATA%\Glide\config.json` |
| 用户日志目录 | `%LOCALAPPDATA%\Glide\logs` |
| portable zip | zip 解压目录，包含 `glide.exe`、`glide-cli.exe`、`glide-server.exe` |

GUI 首次启动会创建用户配置目录和日志目录。服务端地址保存到
`%APPDATA%\Glide\config.json`，重启后继续使用，避免依赖源码目录或
构建机路径。

## 内置依赖

| 依赖 | 打包策略 |
|------|----------|
| `Glide.exe` / `glide.exe` | Tauri GUI 主程序 |
| `glide-cli.exe` | Windows package artifact 和 portable zip 内置 |
| `glide-server.exe` | Windows package artifact 和 portable zip 内置 |
| WebView2 Runtime | Tauri `webviewInstallMode = offlineInstaller` 内置离线安装器 |
| MSVC / C runtime | Windows MSVC targets 使用 `crt-static` 静态链接 |
| Tauri frontend | `crates/glide-tauri/public` 随 Tauri bundle 内置 |
| 图标/托盘图标 | `icons/*.png` 和 `icons/icon.ico` 随 Tauri bundle 内置 |
| 默认配置 | 首次启动自动生成用户配置 |
| TLS 根证书 | 使用 Windows 系统证书存储和 Rust HTTP/TLS runtime |

## 外部依赖

| 依赖 | 是否内置 | 原因和处理 |
|------|----------|------------|
| Windows WebView2 system runtime | 安装包内置离线安装器 | 系统可能已存在；不存在时安装器自动安装 |
| Windows clipboard API | 不内置 | 操作系统能力，Windows 自带 |
| Windows tray / shell API | 不内置 | 操作系统能力，Windows 自带 |
| Windows input injection / hook API | 不内置 | 操作系统能力；未来如需驱动级能力必须单独设计签名和安装 |
| 防火墙规则 | 不自动创建 | 客户端主动连接服务端通常不需要入站规则；服务端模式需提示用户 |
| 开机自启动 | 暂不默认启用 | 后续启用时通过 Tauri/注册表写入，并在卸载时清理 |

项目不依赖 .NET、Node/Electron runtime、Qt、GTK、wxWidgets 或外部
Python/PowerShell 模块运行。PowerShell 仅用于测试脚本，不是安装后运行依赖。

## 安装阶段检测

NSIS/MSI 由 Tauri 生成，安装包负责：

1. 校验 WebView2 Runtime，缺失时运行内置离线安装器。
2. 校验目标目录可写。
3. 写入卸载信息和快捷方式。
4. 安装 GUI 主程序、frontend 资源、图标和 Tauri 资源。

首次启动负责：

1. 创建 `%APPDATA%\Glide`。
2. 创建 `%LOCALAPPDATA%\Glide\logs`。
3. 检测 WebView2 Runtime；如果仍缺失，弹出清晰错误提示。
4. 读取或生成 `config.json`。

## 连接和剪贴板能力

连接服务端只需要服务端 URL。GUI 会保存 URL 并通过 HTTP 注册设备，
再建立 WebSocket 同步连接。连接失败时 UI/命令返回错误文本。

剪贴板依赖 Windows 原生 clipboard API，文本使用 Unicode clipboard。
图片和文件剪贴板依赖当前 `glide-desktop` Windows backend 能力；完整
安装版验证必须覆盖文本、中文、Emoji、多行、大文本、图片和文件路径。

键鼠共享默认关闭。当前协议和 UI 入口已存在，Windows 安装版验证必须覆盖
输入共享开关、连接服务端、输入事件发送/接收、紧急释放和断线释放。

## 升级和卸载

升级覆盖安装应保留 `%APPDATA%\Glide\config.json` 和日志目录。卸载应移除
安装目录、快捷方式和卸载注册项，但不删除用户配置，除非用户明确选择清理。

当前未自动写入防火墙规则或开机自启动项，因此卸载阶段不应残留这些项目。
未来如果新增，必须在卸载验证中检查并清理。

## 依赖完整性测试

新增脚本：

| 脚本 | 用途 |
|------|------|
| `scripts/check-windows-package-deps.ps1` | 检查安装目录/portable zip 的 exe、依赖、WebView2 offline 配置和构建机路径泄露 |
| `scripts/test-windows-installer.ps1` | 静默安装 NSIS/MSI，并调用依赖检查和 GUI 启动 smoke test |
| `scripts/test-windows-installed-client.ps1` | 对已安装客户端做启动、目录、服务端可达和剪贴板 smoke test |

Release workflow 在 Windows package job 中运行
`check-windows-package-deps.ps1`，至少保证 portable zip 和 installer 输出完整。

## 干净 Windows 验证矩阵

| 环境 | 要求 | 当前状态 |
|------|------|----------|
| Windows 10 x64 clean VM | 安装、启动、连接、剪贴板、卸载 | 待执行 |
| Windows 11 x64 clean VM | 安装、启动、连接、剪贴板、卸载 | 待执行 |
| Windows x64 without Visual Studio | 不缺 VC runtime | 待执行 |
| Windows x64 without Rust/Node/Python | 不依赖开发工具链 | 待执行 |
| Windows arm64 | 暂不支持发布 | 不适用 |

干净 VM 验证命令：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-installer.ps1 `
  -InstallerPath .\Glide_0.1.0_x64-setup.exe

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-installed-client.ps1 `
  -Server http://aicode.soocoo.xyz:8080
```

安装版端到端验证还必须覆盖 Windows GUI 与 Linux CLI/Linux GUI 的双向剪贴板
同步、服务端重启后自动恢复、安装版和开发版配置路径差异，以及日志中不泄露
token 明文。
