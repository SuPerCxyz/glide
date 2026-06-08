# Glide 稳定技术文档

> 本文档是 Glide 当前技术实现的事实来源。更新日期：2026-06-08。
> 文档只记录当前仓库可从代码、配置、脚本或已有文档确认的内容；规划内容会明确标注。

## 1. 项目技术概览

Glide 是一个 Rust workspace，用于实现局域网优先的跨设备剪贴板同步、服务端回退、CLI 调试工具、轻量桌面 GUI 和后续后台 daemon。

当前主要技术栈：

- 语言：Rust。
- GUI：`crates/glide-gui` 使用 Slint，不依赖 Tauri、WebView2、Electron、Chromium 或系统 WebView。
- 服务端：`crates/glide-server` 使用 Axum、Tokio、SQLite/sqlx、HTTP API 和 WebSocket。
- CLI：`crates/glide-cli` 使用 clap、reqwest、tokio-tungstenite。
- 核心类型：`crates/glide-core` 定义协议、设备、剪贴板、payload、路由、输入事件、策略、发现和显示器模型。
- 桌面适配：`crates/glide-desktop` 提供剪贴板、输入、LAN sync 和 LAN input 的库级实现。
- 后台服务：`crates/glide-daemon` 当前是第一阶段 skeleton，只提供状态、设置、连接状态、开关和日志模型。
- 打包：Linux 通过 `scripts/package-linux.sh` 构建 `.deb`、`.rpm`、`.AppImage`；Windows 当前 release 产物是 portable zip。
- Docker：`Dockerfile` 是 runtime-only 镜像，包含预构建的 `glide-server` 和 `glide-cli`。

## 2. 模块结构

| 模块 | 当前用途 | 当前状态 |
|------|----------|----------|
| `crates/glide-core` | 共享协议和数据模型：设备、剪贴板、MIME、payload、同步事件、输入事件、策略、发现、路由、显示器布局 | 已实现核心模型和单元测试 |
| `crates/glide-server` | 中央服务端：设备注册、临时 token、WebSocket 同步、payload、历史、管理页面、清理任务 | 已实现 |
| `crates/glide-cli` | 无头命令行工具：`copy`、`paste`、`history`、`devices`，支持 `--server` + `--token` 临时模式 | 部分实现；文本和基础文件 payload 上传/下载可用，Windows GNU CLI 已通过 Wine smoke，完整目录/图片体验仍需更多端到端验证 |
| `crates/glide-desktop` | 桌面库：剪贴板 backend trait、Linux/Windows clipboard、Linux xdotool/Windows SendInput input backend、LAN sync、LAN input | 部分实现；未接入新 GUI/daemon 常驻链路 |
| `crates/glide-daemon` | 后台服务 skeleton：状态、设置、连接/断开状态、剪贴板/键鼠开关、日志 tail | 部分实现；尚未执行真实网络、剪贴板、键鼠或文件传输 |
| `crates/glide-gui` | Slint GUI：页面、状态展示、设置、模拟后端、未来 IPC trait 边界 | 第一阶段已实现；当前使用 `MockBackend` |
| `scripts/` | Linux/Windows/Docker/网络/GUI/剪贴板测试和打包脚本 | 部分脚本已在 Linux 环境验证，Windows 脚本需 Windows VM |

仓库中已移除 `crates/glide-tauri`。后续不应恢复 Tauri 作为主 GUI。

## 3. 运行架构

### 3.1 服务端

`glide-server` 默认读取：

- `GLIDE_LISTEN_ADDR`，默认 `0.0.0.0:8080`。
- `GLIDE_DATA_DIR`，默认 `./data`。

启动时会创建数据目录、初始化 SQLite 数据库、运行 migration、启动定时清理任务，并通过 Axum 提供：

- `GET /` 管理页面。
- `GET /api/v1/health` 健康检查。
- `POST /api/v1/devices/register` 设备注册。
- `GET /api/v1/devices` 设备列表。
- `POST /api/v1/tokens/create`、`POST /api/v1/tokens/validate` 临时 token。
- `GET /api/v1/clipboard/history` 剪贴板历史。
- `POST /api/v1/payload/upload`、`GET /api/v1/payload/{payload_id}` payload。
- `GET /ws/sync` 剪贴板同步 WebSocket。
- `GET /ws/input` 输入中继 WebSocket。

### 3.2 CLI

`glide-cli` 支持：

```bash
glide --server http://host:8080 --token TOKEN copy "hello"
glide paste
glide history --limit 20
glide devices
```

配置文件路径：

- `GLIDE_CONFIG_PATH` 可显式指定配置文件，供测试、portable 包和虚拟环境使用。
- Windows 目标默认读取 `%APPDATA%\Glide\config.json`。
- 非 Windows 目标默认读取 `$HOME/.config/glide/config.json`。

临时模式使用 `--server` + `--token`，不写配置、不加入持久可信网络。

### 3.3 GUI

`glide-gui` 启动 Slint `MainWindow`，通过 `GuiBackend` trait 获取状态、设备、设置、日志和平台能力。第一阶段使用 `MockBackend`，但已支持：

- **真实 HTTP 服务端连接**：点击连接按钮时，GUI 会通过 HTTP POST 到 `/api/v1/devices/register` 注册本机设备；状态页会调用 `/api/v1/health` 检查连接状态；设备列表会从 `/api/v1/devices` 拉取服务端设备列表（与 LAN 发现设备合并展示）。服务端不可达时回退到模拟连接模式。
- **终端实时日志**：增加 `--verbose` /`-v` 命令行参数，启用后 tracing 输出到 stderr；同时日志页每 3 秒自动刷新。
- **mock 列表中的示例设备**（Linux CLI / Windows VM）在网络设备列表中仍保留，但在真实服务端连接后会替换为服务端设备数据。

GUI 可见文案以简体中文为主，保留 Glide、Rust、Slint、Tauri、WebView2、URL、平台名等必要技术名词。

当前 GUI 页面：

- 状态页
- 设备页
- 配对页
- 日志页
- 设置页
- 平台能力页
- 关于页

### 3.4 Daemon

`glide-daemon` 当前可执行：

```bash
glide-daemon --print-status
```

输出 JSON 状态。无参数运行时会等待 Ctrl+C。它尚未作为系统服务安装，也没有真实 IPC server。

规划中：GUI 通过本地 IPC 与 daemon 通信，Windows 使用 Named Pipe，Linux/macOS 使用 Unix Domain Socket。

## 4. 网络通信方案

已实现或已有模型：

- 服务端 HTTP API + WebSocket 同步。
- `glide-core` 中有 `SyncEvent`、`ClipboardItem`、`InputEvent`、`TransferRoute` 等协议模型。
- `glide-desktop::lan_sync` 中有 UDP 多播发现和 LAN WebSocket 直连引擎。
- `glide-desktop::lan_input` 中有输入 WebSocket server 和事件处理 skeleton。

需要注意：

- 新 Slint GUI 当前没有接入真实 server/desktop/daemon 网络链路。
- CLI WebSocket 发送文本剪贴板事件到 server 的路径存在。
- CLI 文件 payload 会通过 `/api/v1/payload/upload` multipart 上传；history API 返回 `payload_refs` 后，其他 CLI 设备可通过 `paste --output` 下载。

## 5. 跨平台技术细节

### Windows

当前目标：

- 构建 `glide-gui.exe`、`glide-daemon.exe`、`glide-cli.exe`、`glide-server.exe`。
- GUI 使用 Slint，不依赖 WebView2。
- Release workflow 产出 Windows portable zip，zip 内同时包含 `glide.exe` 和 `glide-gui.exe` 两个 GUI 启动名。
- `glide-desktop` 已有 Windows SendInput 输入注入后端，可执行键盘、鼠标移动、点击、滚轮和屏幕尺寸/光标位置读取的库级能力。
- GUI 默认使用自动渲染选择：先以 `SLINT_BACKEND=winit-femtovg`
  启动子进程，物理机可用时使用 GPU renderer；如果日志显示 OpenGL
  初始化失败或 `glCreateShader` 缺失，则回退到
  `SLINT_BACKEND=winit-software`。用户显式设置 `SLINT_BACKEND` 时以用户设置为准。

当前限制：

- 没有当前 Slint 版 NSIS/MSI 安装器；旧 `Glide_*_x64-setup.exe` / `.msi` 是 Tauri 时代产物，不应继续使用。
- Windows 11 Enterprise Evaluation 25H2 QEMU VM 中 `glide-gui.exe --smoke` 已真实执行通过。
- 安装包模式、普通桌面双击启动、真实连接、剪贴板、键鼠验证仍需执行。
- GUI 当前模拟后端不代表真实 Windows 剪贴板和输入链路；SendInput 后端尚未由 daemon/GUI 串成完整跨设备产品链路。

### Linux

当前目标：

- 第一阶段优先 X11。
- `.deb`、`.rpm`、`.AppImage` 包含 GUI、daemon、CLI、server。
- GUI 可以在 X11/Xvfb 下 smoke 启动。

Wayland 限制：

- GUI 可启动能力需要按发行版和桌面环境验证。
- 全局键鼠控制受 Wayland compositor 权限限制，不能承诺完整支持。

### macOS

当前仅架构预留。代码中有 `Platform::MacOs` 类型，但没有 macOS 打包、权限引导、LaunchAgent、剪贴板/输入实测流程。

## 6. 剪贴板、键鼠和文件能力

| 能力 | 当前实现状态 |
|------|--------------|
| 文本剪贴板模型 | 已实现 |
| CLI 文本 copy/paste/history | 部分实现并有自动化测试 |
| 图片/文件/目录模型 | 已实现类型和 payload 引用 |
| 图片/文件/目录端到端传输 | 部分实现；CLI 单文件 payload 上传/下载 smoke 已通过，目录/图片仍需扩展验证 |
| Linux clipboard backend | 部分实现，支持 X11/Wayland/headless 分支 |
| Windows clipboard backend | 代码存在，需 Windows VM 验证 |
| 键鼠事件模型 | 已实现 |
| 输入注入抽象和速率/紧急释放 | 已实现库级逻辑 |
| Linux 输入注入 backend | 部分实现，X11 下通过 xdotool 执行，Wayland 不承诺完整全局输入 |
| Windows 输入注入 backend | 已实现库级 SendInput 后端，并通过 Windows GNU 交叉构建验证；真实 Win11 注入仍需 VM/物理机安全测试 |
| LAN input 平台选择 | 已实现 Linux/Windows backend selector，LAN input consumer 不再写死 Linux 后端 |
| 真实跨屏键鼠端到端 | 未实现完整产品链路，需 daemon/platform 接入 |
| 文件传输 GUI | 未实现 |

## 7. 构建与打包

常用本地命令：

```bash
cargo check --workspace
cargo test --workspace
cargo build --release --package glide-gui --package glide-daemon --package glide-cli --package glide-server
VERSION=0.1.0 ./scripts/package-linux.sh
```

Linux package 脚本要求 release binaries 已存在，并需要 `dpkg-deb`、`rpmbuild`、`curl` 和 AppImage 工具。它会生成：

- `glide_<version>_amd64.deb`
- `glide-<version>-1.x86_64.rpm`
- `glide-<version>-x86_64.AppImage`

已记录的本地打包结果：2026-06-06 本地 `dist-verify` 结果为 deb 11 MB、rpm 15 MB、AppImage 15 MB；`target/release/glide-gui` 为 24 MB。该目录不是长期产物。

CI 状态：

- 普通 CI：Linux build/test、Docker verify、Linux package artifact、Windows native build artifact。
- Release workflow：按输入或 tag 构建 Linux package、Windows portable zip、Docker image。
- Windows GUI smoke：普通 CI 和 Release 都执行 `glide-gui.exe --smoke`
  与 `--interaction-smoke`。Windows release GUI 使用 GUI subsystem，
  PowerShell 直接调用时不会稳定等待和填充 `$LASTEXITCODE`，workflow
  必须使用 `Start-Process -Wait -PassThru` 并检查进程 `ExitCode`。
- Docker verify：启动容器 10 秒后通过 `curl http://127.0.0.1:18080/` 验证管理页响应。

## 8. 测试与验证

当前已有测试入口包括：

```bash
cargo check --workspace
cargo test --workspace
cargo build --release --workspace
cargo test --package glide-gui
cargo test --package glide-daemon
bash scripts/test-e2e-linux.sh
bash scripts/test-network.sh
bash scripts/test-clipboard-cli.sh
bash scripts/test-gui-linux.sh
bash scripts/test-keyboard-mouse-protocol.sh
bash scripts/test-reconnect.sh
bash scripts/test-tc-network.sh
bash scripts/test-cli-payload.sh
bash scripts/test-windows-cli-wine.sh
xvfb-run --auto-servernum timeout 5 target/release/glide-gui
APPIMAGE_EXTRACT_AND_RUN=1 xvfb-run --auto-servernum timeout 5 dist-verify/glide-0.1.0-x86_64.AppImage
```

2026-06-06 本地验证结果：

- `cargo check --workspace` 通过。
- `cargo test --workspace` 通过。
- `cargo build --release --workspace` 通过。
- `bash scripts/test-cli-payload.sh` 通过，覆盖双 CLI 设备单文件 payload 上传/下载。
- `cargo test -p glide-cli config::tests` 通过，覆盖 CLI 显式配置路径、Windows `%APPDATA%` 和非 Windows `$HOME` 路径选择。
- `bash scripts/test-windows-cli-wine.sh` 通过，Wine 9.0 中 Windows GNU `glide-cli.exe` 连接真实本机 server，完成文本 copy/paste 和单文件 payload upload/download。
- `xvfb-run --auto-servernum timeout 5 target/release/glide-gui` 通过，GUI 保持运行 5 秒。
- `VERSION=0.1.0 DIST_DIR=dist-verify ./scripts/package-linux.sh` 通过。
- `APPIMAGE_EXTRACT_AND_RUN=1 xvfb-run --auto-servernum timeout 5 dist-verify/glide-0.1.0-x86_64.AppImage` 通过。
- `GLIDE_GUI_LOG=<tmp> xvfb-run --auto-servernum target/debug/glide-gui --smoke` 通过，输出版本、平台、服务状态和诊断日志路径。
- `cargo build --package glide-gui --target x86_64-pc-windows-gnu` 通过。
- `bash scripts/test-windows-gui-wine.sh` 通过，Wine 9.0 中 Windows GNU `glide-gui.exe --smoke` 输出 `glide-gui smoke ok`。
- QEMU/KVM Windows 11 Enterprise Evaluation 25H2 x64 真实 VM smoke 通过：`D:\glide-test.ps1` 执行 `D:\glide-gui.exe --smoke`，回传 `exit=0`、`os=windows`、`arch=x86_64` 和诊断日志。

Windows VM 脚本：

- `scripts/check-windows-package-deps.ps1`
- `scripts/test-windows-connect.ps1`
- `scripts/test-windows-clipboard.ps1`
- `scripts/test-windows-gui.ahk`
- `scripts/test-windows-gui-smoke.ps1`
- `scripts/test-windows-installed-client.ps1`
- `scripts/test-windows-installer.ps1`
- `scripts/test-windows-notepad-clipboard.py`

Windows GUI smoke 已在真实 Win11 QEMU VM 中执行通过。安装包模式、普通桌面会话双击启动、真实连接、剪贴板和键鼠仍需 Windows VM 继续验证。不能把 smoke/mock/协议层测试通过表述为 Windows GUI 真实连接或真实剪贴板通过。

Windows GUI 诊断入口：

```powershell
$env:GLIDE_GUI_LOG="$env:TEMP\glide-gui.log"
.\glide.exe --smoke
Get-Content $env:GLIDE_GUI_LOG
```

直接查看默认日志：

```powershell
.\glide.exe --diagnostics-path
.\glide.exe --diagnostics
Get-Content "$env:APPDATA\Glide\logs\glide-gui.log"
```

注意：Windows release GUI 是 GUI subsystem 程序，PowerShell 直接执行
`--diagnostics` 或 `--diagnostics-path` 时可能看不到 stdout。排错时优先设置
`GLIDE_GUI_LOG`，再读取该日志文件。

或运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test-windows-gui-smoke.ps1 -GuiExe .\glide.exe
```

`glide-gui` 普通启动、启动失败和 panic hook 都会写入诊断日志。默认日志路径为 `%APPDATA%\Glide\logs\glide-gui.log`，也可通过 `GLIDE_GUI_LOG` 覆盖。若普通启动失败，终端会输出 `glide-gui failed: ...` 和 `diagnostics=<path>`。

## 9. 开发注意事项

- 修改 GUI：保持 `glide-gui` 薄层，只做状态显示、配置、配对入口、日志和提示；不要把网络、剪贴板、键鼠、文件传输直接写入 GUI。
- 修改 daemon：需要保持未来 IPC 边界；真实能力应下沉到 daemon/platform/desktop，而不是 GUI。
- 修改 core：协议和共享模型会影响 server、CLI、desktop、daemon、GUI，必须补跨 crate 测试。
- 修改 server：注意 SQLite migration、token 校验、WebSocket 事件格式和日志脱敏。
- 修改 CLI：注意临时模式不写配置，token 不得明文输出。
- 修改跨平台能力：必须明确 Windows/Linux/macOS 差异，Wayland 不要虚假承诺全局输入控制。
- 修改打包流程：需要同步 CI/release、`scripts/package-linux.sh`、Windows 依赖检查脚本和产品分发文档。

## 10. 已知限制

- GUI 第一阶段使用模拟后端，已支持真实 HTTP 服务端连接（注册/健康检查/设备列表），服务端不可达时回退到离线模式。未集成真实 WebSocket 同步和完整 daemon IPC。
- daemon 当前是 skeleton，没有真实 IPC、服务安装、网络、剪贴板、键鼠或文件传输执行。
- Windows `glide-gui.exe --smoke` 在 QEMU Win11 25H2 已通过；Windows GNU `glide-cli.exe` 在 Wine 中已通过文本和单文件 payload smoke。用户报告的 GUI 双击弹窗后消失仍需在普通桌面会话/portable zip 模式复测。
- Windows installer 仍是规划中。
- macOS 支持仍是规划中。
- 文件 payload 已有 CLI 单文件上传/下载 smoke；目录、图片、大文件、断点续传和 GUI 文件传输仍未完成。
- `docs/design.md` 是历史产品需求文档，可能包含已过期状态；稳定事实以本文档、`docs/PRODUCT.md`、`docs/ARCHITECTURE.md`、`docs/DESIGN.md` 为准。
