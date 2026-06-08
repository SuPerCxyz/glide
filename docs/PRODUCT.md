# Glide 稳定产品文档

> 本文档是 Glide 当前产品能力、功能边界和交互设计的事实来源。更新日期：2026-06-08。
> 历史 plan 的逐项状态核对见 [PLAN_STATUS.md](PLAN_STATUS.md)。

## 1. 产品定位

Glide 面向需要在多台可信设备之间共享剪贴板、后续共享键鼠和文件的用户。典型场景包括：

- 桌面 Linux 与 Windows 设备之间复制文本。
- 局域网内多设备同步剪贴板。
- 跨网段时通过自建 `glide-server` 回退。
- 使用 CLI 做自动化、调试或临时一次性复制粘贴。

Glide 不是远程桌面产品，不传输完整屏幕画面。它的方向是轻量的剪贴板同步、设备配对、文件传输和跨屏输入控制。

## 2. 核心能力状态

| 能力 | 产品状态 |
|------|----------|
| 多端文本复制粘贴 | 部分实现：CLI/server 路径可测，GUI 仍是模拟后端 |
| 图片剪贴板 | 规划中/部分模型实现：核心 MIME 和 payload 模型存在，端到端仍需补齐 |
| 文件/目录传输 | 部分实现：类型、server payload API、CLI 单文件上传/下载 smoke 已完成；GUI、目录恢复、图片剪贴板仍需补齐 |
| 跨设备键鼠控制 | 部分实现：协议、输入模型、Linux xdotool 注入、Windows SendInput 注入和 LAN input 平台选择存在，真实 GUI/daemon 产品链路未完成 |
| 设备连接与配对 | 部分实现：server 设备注册和 token 存在，GUI 配对页为模拟状态 |
| 本机设备管理 | 部分实现：GUI 可编辑模拟设备名，CLI 支持 `GLIDE_CONFIG_PATH`、Windows `%APPDATA%\Glide\config.json` 和非 Windows `$HOME/.config/glide/config.json` |
| 日志与诊断 | 部分实现：GUI 显示模拟日志，server/daemon 有 tracing/log 模型 |
| CLI 能力 | 部分实现：copy/paste/history/devices 命令存在 |
| 后台运行能力 | 规划中/部分实现：daemon skeleton 存在，尚未作为真实后台服务接管能力 |

## 3. 平台支持

| 平台 | 当前目标 | 当前状态 |
|------|----------|----------|
| Windows 10/11 x64 | Slint GUI、daemon、CLI、server portable zip；不依赖 WebView2 | 构建已配置；Windows 11 Enterprise Evaluation 25H2 QEMU VM 中 `glide-gui.exe --smoke` 已通过 |
| Linux x64 | Slint GUI、daemon、CLI、server；`.deb/.rpm/.AppImage` | 构建和包脚本存在，X11 优先 |
| Linux Wayland | GUI 启动和有限能力提示 | 全局键鼠控制受限制，不承诺完整能力 |
| macOS | 后续 Slint GUI 和 daemon 架构预留 | 未实现发布和权限引导 |

权限要求：

- Windows：键鼠注入使用系统 SendInput API，不安装驱动；真实使用时仍可能被安全软件或权限策略影响。
- Linux X11：剪贴板和键鼠控制依赖 X11 工具/权限。
- Linux Wayland：合成器通常限制全局输入监听/注入。
- macOS：规划中，需要 Accessibility、Input Monitoring、Clipboard、Local Network 等权限设计。

## 4. 用户流程

### 4.1 首次启动

当前 Slint GUI 可以打开状态页、设备页、配对页、日志页、设置页、平台能力页和关于页。第一阶段显示的是模拟状态，用于替代 Tauri GUI 的最小界面框架。

规划中：首次启动应检测 daemon 状态、配置目录、日志目录、平台权限和服务端连接配置。

### 4.2 设备配对

当前 server 支持设备注册和 `GLIDE_REGISTRATION_TOKEN`。GUI 配对页目前只生成模拟配对码。真实配对码、设备指纹、用户确认和 token 保存仍需 daemon IPC / 后台服务通信接入。

### 4.3 连接设备/服务端

当前 GUI 可输入服务端地址并切换模拟连接状态。真实连接应由 daemon 负责，并向 GUI 返回阶段化状态：解析地址、连接 TCP/WebSocket、认证、配对、同步就绪、失败原因。

CLI 可通过 `--server` 和 `--token` 进行临时连接。

### 4.4 开启复制粘贴同步

GUI 有剪贴板同步开关，但当前只更新模拟状态。真实同步应由 daemon/platform 监听系统剪贴板，再通过 LAN 或 server 中继同步。

### 4.5 开启键鼠跨屏控制

GUI 有键鼠共享开关和平台能力提示。默认应关闭。当前库级输入注入后端已包含 Linux X11 xdotool 和 Windows SendInput，但 GUI 开关仍只更新模拟状态。真实跨屏控制必须由 daemon 接入平台后端，并考虑平台权限、断线紧急释放、DPI/多屏、输入法和 Wayland 限制。

### 4.6 发送文件

当前 GUI 没有文件发送界面。CLI 和 server 已有部分文件/payload 模型，但端到端仍需补齐。

### 4.7 查看状态和日志

GUI 状态页和日志页已存在。当前日志来自模拟后端。后续应 tail daemon 日志，并保证 token 脱敏。

## 5. GUI 页面设计状态

### 5.1 GUI 客户端页面

| 页面 | 当前状态 |
|------|----------|
| 首页/状态页 | 已实现 Slint 页面，显示服务状态、服务端地址、连接按钮、剪贴板/键鼠开关；待按统一设计系统重写 |
| 设备列表页 | 已实现 Slint 页面，显示模拟受信任设备；待按统一设计系统重写 |
| 设备详情页 | 规划中：单设备信息、同步策略、活动历史 |
| 配对页 | 已实现 Slint 页面占位，真实配对流程未接入；待按统一设计系统重写 |
| 剪贴板同步页 | 规划中：同步开关、按类型策略、最近记录 |
| 键鼠控制页 | 规划中：共享开关、设备布局、平台限制提示 |
| 文件传输页 | 规划中：发送/接收文件、传输历史 |
| 连接状态页 | 规划中：连接拓扑、各设备状态、诊断信息 |
| 日志页 | 已实现 Slint 页面，显示模拟日志；待按统一设计系统重写 |
| 设置页 | 已实现 Slint 页面，服务端地址和本机设备名保存到模拟后端；待按统一设计系统重写，新增主题切换 |
| 平台能力页 | 已实现基础提示，Wayland 限制说明已存在；待按统一设计系统重写 |
| 关于页 | 已实现 Slint 页面，明确不依赖 Tauri/WebView2 |

### 5.2 服务端 Web 管理页面

| 页面 | 当前状态 |
|------|----------|
| 总览页 | 已实现基础统计卡片、设备列表、剪贴板历史、临时令牌；待按统一设计系统重写 |
| 设备列表页 | 已实现基础表格展示；待按统一设计系统重写 |
| 设备详情页 | 规划中 |
| 连接状态页 | 规划中 |
| 配对/信任管理页 | 规划中 |
| 剪贴板历史页 | 已实现基础列表；待按统一设计系统重写 |
| 文件传输页 | 规划中 |
| 日志诊断页 | 规划中 |
| 设置页 | 规划中 |

### 5.3 统一设计系统

GUI 客户端和服务端 Web 管理页将使用统一设计系统：

- **Light 为默认主题**，支持切换 Dark 主题。
- **全局等宽字体**：`JetBrains Mono`, `Fira Code`, `Consolas`, `monospace`。
- **六态状态色**：在线、离线、连接中、错误、受限、未配对，每态独立文字色和背景色。
- **AlertBox 四类型**：info / warning / success / danger，左边框 3px 标识类型。
- **详细设计系统规范**见 `docs/DESIGN.md`。

### 5.4 文案国际化

当前 GUI 可见文案以简体中文为主，保留 Glide、Rust、Slint、Tauri、WebView2、URL、Windows、Linux、Wayland 等必要品牌名和技术名词。

## 6. 产品边界

已实现：

- Rust workspace 基础分层。
- Slint GUI 第一阶段页面。
- 不依赖 Tauri/WebView2 的 GUI 构建。
- Server API、WebSocket、SQLite、管理页、Docker runtime 镜像。
- CLI 基础命令。
- Linux `.deb/.rpm/.AppImage` 打包脚本和 CI artifact。
- Windows portable zip 构建流程。
- GUI 主要可见文案中文化。
- Windows SendInput 和 Linux xdotool 输入注入库级后端。
- 统一设计系统规范（DESIGN.md），定义 Light/Dark 双主题、六态状态色、组件库。

部分实现：

- daemon 只提供 skeleton。
- GUI backend 是模拟后端。
- desktop crate 有平台能力库，但没有接入真实 daemon。
- 键鼠共享具备事件模型和平台注入后端，但缺真实捕获、设备布局、daemon 会话控制和 GUI 联动。
- 文件/图片 payload 模型存在但端到端未完成。

规划中：

- daemon IPC / 后台服务通信。
- Windows installer。
- 系统托盘和开机自启动。
- macOS 支持。
- 真实 GUI 配对、连接、剪贴板、键鼠、文件传输。
- GUI 新增页面：设备详情、剪贴板同步、键鼠控制、文件传输、连接状态。
- Web 管理端新增页面：设备详情、连接状态、配对/信任管理、日志诊断、设置、文件传输。
- GUI/Web 按统一设计系统实现 Light/Dark 双主题和组件库。

不承诺：

- Wayland 下完整全局键鼠控制。
- 不安装任何系统权限/安全提示即可完成所有输入控制。
- 当前 GUI 模拟状态等同真实连接成功。
- 当前 Windows portable zip 已在所有干净 Win11 机器上验证无闪退。

## 7. 打包和分发策略

Windows：

- 当前产物：portable zip，包含 `glide.exe`、`glide-gui.exe`、`glide-daemon.exe`、`glide-cli.exe`、`glide-server.exe` 和 README。
- GUI 不依赖 WebView2。
- 安装器为规划中；旧 `Glide_*_x64-setup.exe` / `.msi` 属于 Tauri 时代产物，不是当前 Slint GUI 下载项。

Linux：

- 当前产物：`.deb`、`.rpm`、`.AppImage`。
- 包含 `glide-gui`、`glide-daemon`、`glide-cli`、`glide-server`、desktop entry 和图标。

CLI/server：

- Windows release 还会上传独立 server/CLI exe。
- Docker 镜像包含 server 和 CLI。

离线依赖策略：

- Windows 不再内置 WebView2 offline installer。
- Linux 包声明系统动态库依赖，不内置所有系统库。

## 8. 功能状态总览

### 8.1 已实现功能

| 模块 | 功能 | 平台 | 入口 | 代码位置 | 已验证场景 | 已知限制 |
|------|------|------|------|----------|------------|----------|
| 服务端 | 健康检查、设备注册、设备列表、临时 token、history、WebSocket sync、payload upload/download、管理页 | Linux/Windows 构建目标 | `glide-server`、HTTP API、`/ws/sync` | `crates/glide-server` | `cargo test --package glide-server`，E2E/network 脚本，Docker verify | TLS 主要依赖反向代理文档；history 查询仍有 SQL 拼接需要后续参数化 |
| CLI | 文本 copy/history/paste/devices | Linux/Windows 构建目标 | `glide-cli` | `crates/glide-cli` | `cargo test --package glide-cli`，`scripts/test-clipboard-cli.sh`，`scripts/test-windows-cli-wine.sh` | 临时 token 和注册 token 的产品语义仍需统一；真实 Win11 桌面 CLI 待补测 |
| CLI 文件 payload | 单文件 `copy --file` 上传，另一设备 `paste --output` 下载 | Linux 已验证，Windows GNU/Wine 已验证 | `glide-cli copy --file` / `paste --output` | `crates/glide-cli/src/commands.rs`、`crates/glide-server/src/handlers.rs`、`crates/glide-server/src/database.rs` | `scripts/test-cli-payload.sh`，`scripts/test-windows-cli-wine.sh` | 目录恢复、图片系统剪贴板、大文件、断点续传仍未完成；真实 Win11 桌面 CLI 待补测 |
| Core | 剪贴板、payload、设备、策略、输入事件、显示器布局、路由模型 | 跨平台 Rust | crate API | `crates/glide-core` | `cargo test --workspace` | 只是模型/纯逻辑，不代表 GUI 产品链路完成 |
| GUI | Slint 第一阶段页面 | Linux/Windows 构建目标 | `glide-gui` | `crates/glide-gui` | `cargo test --package glide-gui`，Linux Xvfb smoke，Wine smoke，QEMU Win11 25H2 `glide-gui.exe --smoke` | 当前使用模拟后端；Win11 smoke 不等同真实连接/剪贴板 |
| Linux 包 | `.deb/.rpm/.AppImage` | Linux x64 | CI artifact / `scripts/package-linux.sh` | `scripts/package-linux.sh` | 本地和 CI package 脚本 | Wayland 能力和发行版依赖仍需真实桌面验证 |
| Docker | runtime-only server 镜像 | Linux container | `Dockerfile` / `docker-compose.yml` | `Dockerfile` | CI 启动 10 秒后 curl 管理页 | 镜像不包含 GUI |

### 8.2 部分实现功能

| 功能 | 已完成 | 还缺 | 当前风险 | 下一步 |
|------|--------|------|----------|--------|
| GUI 连接服务端 | 页面和 backend trait 存在，模拟后端可切换状态 | daemon IPC、真实 health/register/ws、token 输入和错误阶段 | 用户可能误以为 GUI 已真实连接 | 接入 daemon IPC 前，UI 文案继续标注模拟/待接入 |
| 后台 daemon | status/settings/connect 开关和日志 skeleton | 常驻服务、IPC server、网络、剪贴板、键鼠、文件传输 | GUI 退出后核心能力不能继续运行 | 先实现本地 IPC，再接入 server/desktop |
| LAN 直连同步 | UDP discovery、LAN sync engine、route selector | daemon 集成、真实多节点桌面验证、认证/加密 | 库级完成不等于产品完成 | 用 network namespace/多进程集成测试验证 |
| 键鼠共享 | 输入事件、路由、边缘检测、速率限制、紧急释放、Linux xdotool backend、Windows SendInput backend、LAN input 平台 selector | 真实捕获、设备布局、跨屏 GUI、daemon 会话控制、断线释放实测 | 平台权限复杂，误触发风险高 | X11/Windows 后端先接 daemon，再用 VM/物理机验证 SendInput 和 xdotool |
| 文件传输 | server payload、CLI 单文件上传/下载 | 多文件/目录恢复、拖拽、进度、取消、重试、接收确认 | 大文件内存上传可能高占用 | 引入分块/streaming 和传输状态模型 |
| 诊断日志 | server tracing、daemon 内存日志、GUI 模拟日志 | 持久日志、导出诊断包、用户可读错误 | Windows 闪退缺少足够日志 | 增加 `export_diagnostics()` 真实实现 |

### 8.3 未实现功能

| 功能 | 来源 | 优先级 | 依赖条件 | 建议实现方式 |
|------|------|--------|----------|--------------|
| Windows NSIS/MSI installer | plan / 安装体验 | P0/P1 | Windows runner 或 VM、依赖检查脚本 | 先 portable zip 稳定，再做轻量 installer |
| Windows GUI 真实连接和剪贴板 | 用户反馈 / plan | P0 | Windows VM 日志、daemon IPC | 使用 GitHub Actions Windows + VM 手工脚本验证 |
| Windows SendInput 真实注入验证 | plan / 竞品 | P1 | Windows VM、权限策略 | 用 Notepad/PowerShell 安全脚本验证鼠标、键盘、滚轮和断线释放 |
| macOS 剪贴板和输入 | plan | P2 | macOS runner/设备、权限引导 | pbcopy/pbpaste 起步，后续 AppKit/CGEvent |
| GUI 文件传输 | 竞品 / 产品规划 | P2 | daemon IPC、payload 传输状态 | 文件选择/拖拽、进度、取消、接收确认 |
| 系统托盘/开机启动 | 产品规划 | P2 | platform 层 | Windows shell、Linux tray、macOS LaunchAgent 分支 |
| 端到端加密 E2EE | plan / 竞品 | P2 | 密钥管理、配对指纹 | 先 TLS/反向代理，再做设备间 E2EE |

### 8.4 规划中功能

| 功能 | 用户价值 | 推荐交互 | 技术依赖 | 风险 | 近期版本 |
|------|----------|----------|----------|------|----------|
| 配对码/指纹确认 | 防止误连和中间人 | 配对页显示本机指纹、输入/扫描配对码、双方确认 | daemon IPC、设备信任存储 | 安全模型复杂 | 是 |
| 设备布局编辑 | 支持跨屏移动 | 拖拽设备卡片排列左右/上下，支持多显示器 | DisplayLayout、daemon 状态 | DPI/Wayland 差异 | 是 |
| 权限检查页 | 降低启动失败困惑 | 平台能力页列出缺失权限和修复按钮 | platform capability probe | 平台 API 差异 | 是 |
| 诊断包导出 | 便于排查 Windows/Linux 连接问题 | 日志页一键导出 zip/json | 日志路径、配置脱敏 | 泄露 token 风险 | 是 |
| 接收确认 | 防误收文件 | 文件传输弹窗显示来源、大小、保存目录 | daemon/file transfer state | 打扰用户 | 中期 |

### 8.5 暂不承诺功能

- Wayland 下完整全局键鼠控制：合成器和门户能力差异大，当前只承诺明确限制提示。
- macOS 完整输入控制：未完成权限引导和真实设备验证。
- 跨公网自动穿透：当前定位是 LAN first + 自建 server 回退，不承诺自动 NAT 穿透。
- 移动端和浏览器扩展：当前没有对应 crate 和 CI。
- 当前 GUI 模拟状态不等于真实连接、真实剪贴板或真实键鼠控制。

## 9. 平台功能矩阵

| 功能 | Windows | Linux X11 | Linux Wayland | macOS |
|------|---------|-----------|---------------|-------|
| CLI 构建 | 已配置 CI | 已验证 | 同 Linux CLI | 架构预留 |
| CLI 文本同步 | Wine 中 Windows GNU CLI 已验证；真实 Win11 桌面待补测 | 已验证 | 已验证命令行路径 | 待验证 |
| CLI 单文件 payload | Wine 中 Windows GNU CLI 已验证；真实 Win11 桌面待补测 | 已验证 `scripts/test-cli-payload.sh` | 已验证命令行路径 | 待验证 |
| Slint GUI 启动 | QEMU Win11 25H2 `glide-gui.exe --smoke` 通过；Wine smoke 通过 | Xvfb smoke 可测 | 可启动性待桌面验证，限制需提示 | 未验证 |
| 系统剪贴板监听 | 代码分支存在，需 VM | X11/headless 已测部分 | 受 compositor 限制 | 未实现 |
| 键鼠注入 | SendInput 库级后端已实现，真实 Win11 注入待验证 | xdotool/backend 部分可测 | 不承诺完整 | 未实现 |
| 安装包 | portable zip | deb/rpm/AppImage | deb/rpm/AppImage | 未实现 |

## 10. CLI / GUI / daemon 功能矩阵

| 功能 | CLI | GUI | daemon |
|------|-----|-----|--------|
| 服务端地址配置 | `--server` 或 config | 模拟设置页 | settings skeleton |
| 注册 token | config 支持；`GLIDE_CONFIG_PATH` 可覆盖配置文件，Windows 默认 `%APPDATA%\Glide\config.json` | 未接真实输入 | 未接入 |
| 文本同步 | 可用 | 模拟 | 未接入 |
| 文件 payload | 单文件 smoke 通过 | 未实现 | 未接入 |
| 设备列表 | 可调用 server API | 模拟设备列表 | 未接入 |
| 剪贴板开关 | 未作为本地 watcher 开关 | 模拟开关 | settings skeleton |
| 键鼠开关 | 未作为真实输入控制 | 模拟开关，文案已中文化 | settings skeleton |
| 日志 | 命令输出和 server 日志 | 模拟日志 | 内存日志 |
| 诊断导出 | 未实现 | trait 占位 | 未实现 |

## 11. 竞品调研补充

调研对象：Deskflow、Barrier、Input Leap、Synergy、Lan Mouse、Mouse Without Borders、ShareMouse、Logitech Flow、LocalSend、KDE Connect、Warpinator/Winpinator、PairDrop/Snapdrop、Croc。

参考来源：

- Deskflow 官方文档：软件 KVM、剪贴板同步、Windows/macOS/Linux/BSD、多机 TCP 架构。
- Barrier README：边缘切换、手动服务端 IP、Bonjour/auto configuration、Wayland 和 UTF-8 限制。
- Input Leap README：剪贴板支持但 Linux/Wayland 下不支持、AltGr 等跨平台键盘差异。
- Lan Mouse README：Rust 软件 KVM、Wayland/Windows/macOS 支持目标、DTLS 加密。
- Microsoft PowerToys Mouse Without Borders 文档：security key、设备布局、服务模式、剪贴板、单文件 100MB 限制、防火墙提示。
- KDE Connect 官网：文件传输、远程输入、浏览器集成、VPN 场景和设备互联。
- LocalSend 官网：无账号、无互联网、局域网内加密文件/文本传输、默认下载目录和 PIN 验证。
- Warpinator README：局域网文件发送/接收和保存目录限制。
- PairDrop README：免安装、免注册、WebRTC/WebSocket 跨平台文件传输。
- Croc 文档：code phrase、relay、PAKE/E2EE、可恢复传输和本地直连优先。

适合 Glide 吸收的产品细节：

- 设备发现：保留自动发现，也必须支持手动 IP/端口连接；局域网发现失败时 UI 要给出“手动连接”入口。
- 配对信任：借鉴 Mouse Without Borders 的 security key、Lan Mouse 的 fingerprint 授权、KDE Connect 的 pair/unpair，Glide 应实现配对码 + 指纹 + 受信设备列表 + 撤销信任。
- 键鼠控制：借鉴 Barrier/ShareMouse 的屏幕边缘切换、延迟防误触、屏幕角阻挡、相对鼠标移动、多显示器布局；Glide 应加入紧急释放快捷键和断线自动释放修饰键。
- 剪贴板同步：需要文本、中文、Emoji、多行、大文本、HTML/RTF、图片和文件路径测试；必须支持关闭剪贴板同步，防止敏感内容自动传播。
- 文件传输：借鉴 LocalSend/Warpinator 的接收确认、保存目录、进度、取消、失败重试、同名文件处理；借鉴 Mouse Without Borders 的文件大小限制提示。
- 网络状态：显示连接状态、延迟、心跳、重连退避、防火墙/端口提示；明确 NAT/跨三层网络需要手动 server 回退。
- 平台差异：Lan Mouse 对 Wayland 后端差异的说明值得参考，Glide 必须按 X11/Wayland/Windows/macOS 分别声明能力。
- 诊断体验：需要日志导出、配置路径、服务端地址、解析 IP、端口、token 脱敏、最近错误阶段。

## 12. 当前测试验证摘要

本轮新增/确认：

- `cargo check --workspace` 通过，保留既有 unused/dead_code warning。
- `cargo test --workspace` 通过。
- `cargo build --release --workspace` 通过。
- `scripts/test-cli-payload.sh` 通过，覆盖两 CLI 设备经 server 上传/下载单文件 payload。
- `cargo test -p glide-cli config::tests` 通过，覆盖显式配置路径、Windows `%APPDATA%` 和非 Windows `$HOME` 配置路径。
- `scripts/test-windows-cli-wine.sh` 通过，Wine 9.0 中运行 Windows GNU `glide-cli.exe`，连接真实本机 server 完成文本 copy/paste 和单文件 payload upload/download。
- `xvfb-run --auto-servernum timeout 5 target/release/glide-gui` 通过：GUI 在 Xvfb 下保持运行 5 秒，未崩溃。
- `GLIDE_GUI_LOG=<tmp> xvfb-run --auto-servernum target/debug/glide-gui --smoke` 通过：GUI smoke 输出版本、平台、服务状态和诊断日志路径。
- `cargo build --package glide-gui --target x86_64-pc-windows-gnu` 通过，产物为 `target/x86_64-pc-windows-gnu/debug/glide-gui.exe`。
- `bash scripts/test-windows-gui-wine.sh` 通过：Wine 9.0 中运行 Windows GNU `glide-gui.exe --smoke`，输出 `glide-gui smoke ok`。
- QEMU Win11 Enterprise Evaluation 25H2 VM smoke 通过：在安装后的 Win11 OOBE 命令行执行 `D:\glide-test.ps1`，`glide-gui.exe --smoke` 退出码 0，并回传诊断报告。
- `VERSION=0.1.0 DIST_DIR=dist-verify ./scripts/package-linux.sh` 通过，生成 `.deb/.rpm/.AppImage`。
- `APPIMAGE_EXTRACT_AND_RUN=1 xvfb-run --auto-servernum timeout 5 dist-verify/glide-0.1.0-x86_64.AppImage` 通过：AppImage 在 Xvfb 下保持运行 5 秒。
- 发现并修复 history API enum 变体名、`payload_refs` 缺失、Axum payload download route 和 CLI payload upload 缺口。
- Linux 包体积实测：`glide_0.1.0_amd64.deb` 约 11M，`glide-0.1.0-1.x86_64.rpm` 约 15M，`glide-0.1.0-x86_64.AppImage` 约 15M。

仍需验证：

- Windows 完整安装包/portable zip 在 OOBE 完成后的普通桌面双击启动。
- Windows VM 中运行 `scripts/test-windows-gui-smoke.ps1`，收集 `%APPDATA%\Glide\logs\glide-gui.log` 或 `GLIDE_GUI_LOG` 指定日志；也可运行 `glide-gui.exe --diagnostics-path` 查看日志路径，运行 `glide-gui.exe --diagnostics` 直接打印日志；当前已用同等 smoke 命令在 QEMU Win11 OOBE 命令行执行通过。
- Windows CLI 文件 payload smoke 已通过 Wine；真实 Win11 桌面 CLI 仍需补测。
- Linux GUI 真实桌面截图和页面遍历。
- Wayland 下 GUI 启动和限制提示。
- 跨设备真实键鼠控制。

Windows 真实虚拟环境补充验证：

- 已安装并使用 QEMU/KVM Windows 11 Enterprise Evaluation 25H2 x64，配置为 4G RAM、2 vCPU、OVMF Secure Boot、swtpm、USB vvfat 测试盘。
- 已在该 Win11 VM 的 OOBE/Shift+F10 环境运行 `D:\glide-test.ps1`，脚本执行 `D:\glide-gui.exe --smoke` 并回传报告到宿主机。
- 回传结果：`glide-gui smoke ok`，`version=0.1.0`，`os=windows`，`arch=x86_64`，`exit=0`，诊断日志包含 `[process] glide-gui starting` 和 `[smoke] ok ... server_url=http://127.0.0.1:8080`。
- 已在 Wine 9.0 中运行交叉编译的 `target/x86_64-pc-windows-gnu/debug/glide-gui.exe --smoke`，退出码 0，并生成 `/tmp/glide-gui-wine.log`。

当前本机虚拟环境能力：

- 可用：Docker、Xvfb、`xvfb-run`、`rpmbuild`、`qemu-system-x86_64`、`virsh`、Wine、PowerShell、MinGW Windows GNU target。
- `VBoxManage` 命令存在，但 VirtualBox DKMS 在当前 6.17 kernel 上构建失败，`/dev/vboxdrv` 不可用，因此 VirtualBox 不能作为当前 VM 后端。
- Windows 11 VM 可运行，但宿主机内存只有 8G，测试时启用了临时 12G swap 并限制 VM 为 4G/2 vCPU；`Autounattend.xml` 尚未完全接管 OOBE，需要手工辅助。

## 13. 产品设计原则

- 核心能力优先于视觉复杂度。
- GUI 只展示状态、配置、配对、开关、日志和权限提示。
- 错误提示必须可操作：说明失败阶段、地址、端口、权限或 token 问题。
- 平台差异必须明确提示，不隐藏 Wayland/macOS/Windows 权限限制。
- 日志不得泄露 token、密钥或隐私路径。
- 新能力进入产品文档前必须确认代码或脚本中有对应实现。
