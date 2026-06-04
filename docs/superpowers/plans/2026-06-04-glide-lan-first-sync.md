# Glide LAN-First Sync — 实施计划与进度

> **设计文档：** [docs/design.md](../design.md)
>
> **已完成任务标记 `[x]`，待完成标记 `[ ]`**

---

## 已完成任务

### 基础架构

- [x] 创建 Rust workspace (glide-core, glide-server, glide-cli, glide-desktop)
- [x] 定义核心类型：device identity, clipboard item, MIME representation, payload reference, transfer session, sync event, input event
- [x] 实现服务器数据库 schema (devices, clipboard_items, payloads, temp_tokens, input_sessions, cleanup_log)
- [x] 实现服务器 API: device registration, token validation, WebSocket sync, HTTP payload upload/download, history query, devices query, cleanup
- [x] 实现临时 token 支持 (TTL, max uses, allowed operations, max item size)
- [x] 实现 mDNS 和 UDP 多播 LAN 发现
- [x] 实现剪贴板路由选择 (LAN direct → LAN reverse pull → server fallback)
- [x] 实现输入路由选择 (LAN direct → server relay → disconnect)
- [x] 实现输入中继安全机制 (heartbeat, latency measurement, rate limiting, disconnect release, emergency release)

### CLI

- [x] 实现 CLI 持久化模式 (local config)
- [x] 实现 CLI 临时认证模式 (--server + --token)
- [x] 实现 `glide copy` (text, image, file, folder)
- [x] 实现 `glide paste` (stdout + --output)
- [x] 实现 `glide history` 和 `glide devices`

### 桌面客户端

- [x] 实现 Linux 剪贴板适配器 (xclip/xsel for X11, wl-clipboard for Wayland, headless in-memory)
- [x] 实现 Windows 剪贴板适配器 (winapi, CF_UNICODETEXT/CF_HTML/CF_DIB/CF_HDROP)
- [x] 实现 Tauri 2.x 桌面 GUI (system tray, background running, clipboard policy UI)
- [x] 实现 LAN 自动组网引擎 (UDP multicast discovery, direct WebSocket, peer-to-peer clipboard sync)
- [x] 实现 per-device 和 per-type 同步策略

### 服务端

- [x] 实现 WebSocket 同步通道 (ClipboardCaptured event relay)
- [x] 实现 Web 管理面板 (GET / → HTML dashboard)
- [x] 实现定时清理 (retention + capacity cleanup)
- [x] Dockerfile + docker-compose.yml

### 打包与 CI

- [x] GitHub Actions CI (Linux build+test, Windows build, Docker build, Package deb)
- [x] Release workflow (deb, rpm, AppImage, NSIS, MSI, zip, Docker image)
- [x] Docker 镜像标签: `dev-latest` + `YYYYMMDDHHmm`
- [x] Docker 推送到 Docker Hub + GHCR

### 测试

- [x] 核心类型序列化/反序列化测试 (26 tests)
- [x] mDNS/UDP 发现模块测试
- [x] 路由选择逻辑测试
- [x] 临时 token 过期/计数/操作限制/大小限制测试 (7 tests)
- [x] 服务端 API 测试
- [x] 剪贴板同步 WebSocket 测试 (Client A → Server → Client B)
- [x] LAN 自动组网路由测试 (12 tests)
- [x] Docker 部署验证 (server startup + API test)
- [x] 虚拟环境 GUI 启动验证 (xvfb-run)

---

## 待完成任务

### 高优先级

- [ ] CLI `glide copy` 和 `glide paste` 与服务端完整集成测试
- [ ] 键鼠共享真实端到端测试
- [ ] 多客户端并发同步压力测试
- [ ] Windows NSIS 安装器在真实 Windows 环境验证
- [ ] 服务端 TLS/HTTPS 支持文档 (反向代理配置)

### 中优先级

- [ ] macOS 剪贴板适配器 (pbcopy/pbpaste + AppKit)
- [ ] macOS Tauri 构建 (cocoa/webkit2gtk)
- [ ] 键鼠共享 Windows 输入注入 (SendInput API)
- [ ] 剪贴板变更事件监听替代轮询 (inotify/fsevents/WinAPI)
- [ ] 载荷分块传输 (大文件)
- [ ] 断点续传

### 低优先级

- [ ] Android/iOS 客户端
- [ ] Web 浏览器扩展
- [ ] 端到端加密 (E2EE)
- [ ] P2P 直传不经服务端存储
- [ ] 剪贴板历史搜索
- [ ] 剪贴板项目过期自动清理

---

## 测试矩阵

| 场景 | 平台 | 状态 |
|------|------|------|
| CLI copy/paste text | Linux headless | ✅ |
| WebSocket sync A→B | Python test | ✅ |
| GUI start under Xvfb | Linux | ✅ |
| Server Docker deploy | Linux | ✅ |
| Deb install + verify | Docker | ✅ |
| NSIS install + GUI | Windows | ✅ (CI builds, manual verify needed) |
| CLI copy/paste text | Windows | 待验证 |
| Multi-client concurrent | 待测试 | 待实现 |
| LAN auto-discovery | 待测试 | 代码已实现 |
| Keyboard/mouse sharing | 待测试 | 框架已实现 |

---

## 已知问题

1. **WebSocket 同步格式** — serde 枚举变体名必须用 PascalCase (`Text` 不是 `text`)
2. **Docker 文件权限** — 容器内以 root 运行，数据卷需要适当权限
3. **SQLite URL** — 需要 `sqlite:` 前缀 + `?mode=rwc` 参数
4. **Migration 顺序** — 必须在 cleanup 任务启动前完成 migration
5. **cargo fmt 版本** — 本地 rustfmt 1.75 与 CI 最新版格式不同，CI 使用 auto-fix
