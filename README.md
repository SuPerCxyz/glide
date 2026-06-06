# 🚀 Glide

> **局域网优先、服务端回退**的跨设备剪贴板同步工具，可选键盘/鼠标共享

当前稳定技术、产品、架构和设计状态以 `docs/TECHNICAL.md`、
`docs/PRODUCT.md`、`docs/ARCHITECTURE.md`、`docs/DESIGN.md` 为准。

```
  ┌─────────────┐      局域网直连       ┌─────────────┐
  │  📋 电脑 A   │◄═══════════════════►│  📋 电脑 B   │
  │ Linux/Win  │   mDNS / UDP 发现     │  Windows    │
  └──────┬──────┘                      └──────┬──────┘
         │  TLS 加密                           │
         ▼                                     ▼
  ╔═══════════════════════════════════════════════════╗
  ║           🖥️  Glide Server (Docker)               ║
  ║                                                   ║
  ║  设备注册 │ 剪贴板中继 │ 临时令牌 │ 输入中继(可选)  ║
  ║  SQLite │ 文件存储 │ WebSocket 同步               ║
  ╚═══════════════════════════════════════════════════╝
         ▲
         │  一次性临时会话 (不写配置文件)
         │
  ┌──────┴──────┐
  │  🖥️  终端   │  glide copy "你好世界"
  │  无头模式   │
  └─────────────┘
```

## ✨ 特性

| 特性 | 说明 |
|------|------|
| 🌐 **局域网优先路由** | mDNS/UDP 多播发现，点对点直传，最低延迟 |
| ☁️ **服务端回退** | 跨网段时通过 WebSocket 同步 + HTTP 中继传输 |
| 🔑 **临时 CLI 会话** | `--server` + `--token` 一次性认证，不写本地配置 |
| 🔄 **全节点同步** | 任一受信任节点复制，自动同步到所有可信节点 |
| 🖱️ **可选键鼠共享** | 局域网直连模式，服务端回退，紧急释放控制 |
| 🎛️ **细粒度同步策略** | 按设备、按类型控制同步行为 |
| 🐳 **Docker 一键部署** | docker-compose 启动，自带数据清理 |

## 📦 快速开始

### 启动服务端

```bash
# Docker Compose 一键启动
docker compose up -d

# 或手动构建运行
cargo build --release --package glide-server
GLIDE_LISTEN_ADDR=0.0.0.0:8080 cargo run --package glide-server
```

### 使用 CLI

**持久化模式**（需要配置文件 `~/.config/glide/config.json`）：

```bash
# 复制文本
glide copy "你好世界"

# 复制文件
glide copy --file ./文档.zip
glide copy --dir ./项目目录
glide copy --image ./截图.png

# 粘贴
glide paste
glide paste --output ./收到的文件

# 查看历史和设备
glide history --limit 50
glide devices
```

**临时会话**（不写配置文件，适合临时使用）：

```bash
glide --server https://glide.example.com --token 临时令牌 copy "你好"
glide --server https://glide.example.com --token 临时令牌 paste --output ./收到
```

## 🏗️ 架构详情

### 传输路由优先级

```
  设备 A 上复制
       │
       ▼
  ┌──────────────┐
  │ 是自己发的？  │ ── 是 ──► 丢弃（防止回环）
  └──────┬───────┘
         │ 否
         ▼
  ┌──────────────────┐
  │ 发现局域网设备？  │ ── 是 ──► 🟢 局域网直传（最低延迟）
  └──────┬───────────┘
         │ 否
         ▼
  ┌──────────────────────┐
  │ 服务端可达？          │ ── 是 ──► 🟡 服务端回退中继
  └──────┬───────────────┘
         │ 否
         ▼
    📦 加入队列，稍后重试
```

### 键鼠共享路由

```
  控制设备 A ──► 目标设备 B
       │
       ├── 🟢 局域网直连（延迟 < 10ms）
       │
       ├── 🟡 服务端中继（延迟较高，有状态提示）
       │
       └── 🔴 断开并释放（两者都不可用时）
```

### 项目结构

| 组件 | 路径 | 说明 |
|------|------|------|
| 🧩 **glide-core** | `crates/glide-core` | 共享类型：设备身份、剪贴板项、MIME 表示、载荷引用、传输会话、同步事件、输入事件、策略 |
| 🖥️ **glide-server** | `crates/glide-server` | 中央服务端：Axum HTTP/WebSocket API、SQLite 设备注册、文件系统载荷存储、临时令牌认证、输入中继、定时清理 |
| 💻 **glide-cli** | `crates/glide-cli` | 无头 CLI 工具：`glide copy` / `paste` / `history` / `devices` |
| 🪟 **glide-desktop** | `crates/glide-desktop` | 桌面库：X11/Wayland/无头剪贴板适配、键鼠共享模块、边缘穿越检测、速率限制、同步策略 UI 状态 |
| ⚙️ **glide-daemon** | `crates/glide-daemon` | 后台服务 skeleton：状态、设置、开关和日志，后续接真实 IPC |
| 🖼️ **glide-gui** | `crates/glide-gui` | Rust + Slint GUI 第一阶段，不依赖 Tauri/WebView2，当前使用 mock backend |

## 🔧 配置

### 服务端环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `GLIDE_DATA_DIR` | `./data` | 数据目录（SQLite + 载荷文件） |
| `GLIDE_MAX_STORAGE_BYTES` | `1073741824` | 最大存储容量（1 GB） |
| `GLIDE_RETENTION_DAYS` | `30` | 剪贴板保留天数 |
| `GLIDE_MAX_ITEM_BYTES` | `10485760` | 单条项目最大体积（10 MB） |
| `GLIDE_PUBLIC_URL` | `http://localhost:8080` | 公网访问地址 |
| `GLIDE_ADMIN_TOKEN` | — | 管理员认证令牌 |
| `GLIDE_REGISTRATION_TOKEN` | — | 受信任设备注册所需令牌 |
| `GLIDE_TEMP_TOKEN_DEFAULT_TTL` | `3600` | 临时令牌有效期（秒） |
| `GLIDE_INPUT_RELAY_ENABLED` | `false` | 启用键鼠输入中继 |
| `GLIDE_INPUT_RELAY_MAX_LATENCY_MS` | `200` | 中继延迟告警阈值（毫秒） |
| `GLIDE_LISTEN_ADDR` | `0.0.0.0:8080` | 监听地址 |

### API 参考

**HTTP 端点**

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/health` | 健康检查 |
| `POST` | `/api/v1/devices/register` | 注册设备 |
| `GET` | `/api/v1/devices` | 列出所有设备 |
| `POST` | `/api/v1/tokens/validate` | 验证临时令牌 |
| `GET` | `/api/v1/clipboard/history` | 查询剪贴板历史 |
| `POST` | `/api/v1/payload/upload` | 上传载荷（multipart） |
| `GET` | `/api/v1/payload/{id}` | 下载载荷 |
| `POST` | `/api/v1/cleanup` | 触发清理任务 |

**WebSocket 端点**

| 路径 | 说明 |
|------|------|
| `/ws/sync` | 剪贴板同步通道（受信任设备） |
| `/ws/input` | 输入中继通道（仅受信任设备，需开启 `GLIDE_INPUT_RELAY_ENABLED`） |

## 🔒 安全模型

- 🔐 **传输加密** — 所有客户端-服务端、客户端-客户端通信均使用 TLS/WSS
- 📁 **服务端存储** — 服务端明文存储剪贴板；二进制数据存文件系统
- 🏷️ **持久化设备** — 使用 `GLIDE_REGISTRATION_TOKEN` 注册，本地存储凭证
- ⏱️ **临时令牌** — 有 TTL、最大使用次数、允许操作类型、最大体积限制
- 🔒 **临时会话** — 永不加入受信任设备网络，默认仅通过服务端传输
- 🖱️ **输入中继** — 仅限受信任持久设备；紧急释放控制始终可用
- 🛡️ **路径遍历保护** — 载荷 ID 校验禁止 `/` 和 `..`

## 🛠️ 构建与测试

```bash
# 构建全部组件
cargo build --workspace

# 运行全部测试
cargo test --workspace

# 快速检查（不构建）
cargo check --workspace

# 运行服务端
cargo run --package glide-server

# 运行 CLI
cargo run --package glide-cli -- copy "测试文本"

# GUI 启动诊断（Windows 可把 glide-gui 替换为 glide.exe）
cargo run --package glide-gui -- --smoke
```

### Windows GUI 日志和启动排错

如果在 Windows 11 下双击 `glide-gui.exe` 只看到终端或窗口一闪而过，请在
PowerShell 中进入 exe 所在目录执行下面命令查看日志：

```powershell
# 查看诊断日志路径
.\glide-gui.exe --diagnostics-path

# 直接打印当前诊断日志内容
.\glide-gui.exe --diagnostics

# 默认日志位置
Get-Content "$env:APPDATA\Glide\logs\glide-gui.log"
```

也可以指定临时日志文件后启动，便于把日志发给开发者：

```powershell
$env:GLIDE_GUI_LOG="$env:TEMP\glide-gui.log"
.\glide-gui.exe
Get-Content $env:GLIDE_GUI_LOG
```

快速自检命令：

```powershell
.\glide-gui.exe --smoke
.\glide-gui.exe --diagnostics
```

说明：

- 普通启动、启动失败和 panic 都会写入诊断日志。
- 默认日志路径是 `%APPDATA%\Glide\logs\glide-gui.log`。
- 如果你下载的是 GitHub Actions 里的 `windows-binaries`，当前是 debug
  artifact，显示终端窗口是正常现象；release 构建会隐藏控制台窗口。
- 当前 GUI 第一阶段仍使用 mock backend，窗口能启动不代表真实连接、剪贴板和
  键鼠链路已经接入。

### CI 状态

[![CI](https://github.com/SuPerCxyz/glide/actions/workflows/ci.yml/badge.svg)](https://github.com/SuPerCxyz/glide/actions)

CI 流程包含：Linux 构建+测试、Windows 原生构建和 `glide-gui --smoke` 诊断、Docker 镜像验证、Linux `.deb/.rpm/.AppImage` 打包。

## 📋 剪贴板类型

**用户可见类型：**

- 📝 **文本** — 纯文本、HTML 富文本、RTF、URL、颜色值
- 🖼️ **图片** — PNG、JPEG、GIF 等栅格图像
- 📁 **文件/文件夹** — 单文件或文件列表

富文本、URL、颜色作为文本的内部 MIME 表示保留粘贴保真度，不单独暴露为产品级分类。

## 🎮 键鼠共享

默认关闭。启用后支持：

| 功能 | 说明 |
|------|------|
| 🟢 局域网直连 | 最低延迟，优先模式 |
| 🟡 服务端中继 | 延迟较高，有降级状态提示 |
| 🔴 紧急释放 | 任一节点立即断开控制 |
| ⌨️ 快捷键切换 | 在局域网模式和服务端模式间切换 |
| 🖱️ 边缘穿越 | 光标跨越屏幕边缘自动切换到另一设备的屏幕 |
| 📊 速率限制 | 限制每秒输入事件数防止过载 |
| ⚠️ 延迟告警 | 中继延迟超过阈值时发出警告 |

## 🐳 Docker Compose

```yaml
services:
  glide-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      - GLIDE_DATA_DIR=/data
      - GLIDE_RETENTION_DAYS=30
      - GLIDE_INPUT_RELAY_ENABLED=false
    volumes:
      - glide-data:/data
    restart: unless-stopped

volumes:
  glide-data:
```

## 📄 许可证

MIT
