# Glide LAN-First Sync — 实施计划与进度

> **设计文档：** [docs/design.md](../design.md)
> **测试计划：** [docs/test-plan.md](../test-plan.md)
>
> **最后更新：** 2026-06-05
> **总测试：** 92 单元 + 32 E2E = 124+ 通过

---

## 已完成任务

### 基础架构

- [x] 创建 Rust workspace (glide-core, glide-server, glide-cli, glide-desktop, glide-tauri)
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
- [x] 实现跨节点键盘/鼠标共享 (LanInputEngine, xdotool Linux backend, WebSocket input relay)
- [x] 实现屏幕边缘穿越检测
- [x] 实现紧急释放控制

### 服务端

- [x] 实现 WebSocket 同步通道 (ClipboardCaptured event relay)
- [x] 实现 Web 管理面板 (GET / → HTML dashboard with login)
- [x] 实现用户名密码登录 (GLIDE_USERNAME / GLIDE_PASSWORD)
- [x] 实现临时 token 创建 (POST /api/v1/tokens/create)
- [x] 实现定时清理 (retention + capacity cleanup)
- [x] 实现 WebSocket 循环抑制 (过滤发送者 device_id)
- [x] 实现自动设备注册 (WebSocket 连接时自动注册)
- [x] Dockerfile (runtime-only image, 无编译)
- [x] docker-compose.yml

### 打包与 CI

- [x] GitHub Actions CI (Linux build+test, Windows build, Docker build verification)
- [x] Release workflow (deb, rpm, AppImage, MSI, zip, Docker image)
- [x] Docker 镜像标签: `dev-latest` + `YYYYMMDDHHmm`
- [x] Tauri 2.x 构建流程 (cargo tauri build)
- [x] NSIS/MSI 安装器构建
- [x] WebView2 检测 (启动前诊断)

### 测试

- [x] 核心类型序列化/反序列化测试 (27 tests)
- [x] mDNS/UDP 发现模块测试 (5 tests)
- [x] 路由选择逻辑测试 (11 tests)
- [x] 临时 token 过期/计数/操作限制/大小限制测试 (13 tests)
- [x] 服务端数据库测试 (13 tests)
- [x] LAN 同步引擎测试 (12 tests)
- [x] E2E 集成测试 (32 tests: 服务器、剪贴板、输入、错误、重连)
- [x] 网络测试 (11 tests: 可达性、端口绑定、IPv4/IPv6)
- [x] 网络异常测试 (10 tests: 重启、超时、快速连接、大载荷)
- [x] 剪贴板 CLI 测试 (7 tests: 英文/中文/Emoji/多行/特殊字符/大文本)
- [x] 键鼠协议测试 (34 tests: 键盘/鼠标/滚轮/DPI/边缘检测)
- [x] 认证重连测试 (7 tests: token/重连/多客户端)
- [x] GUI 无头测试 (6 tests: Xvfb/xclip/xdotool)
- [x] Docker 网络测试 (4 tests: 跨容器连接/同步)
- [x] Windows 平台分支测试 (mock)

---

## 待完成任务

### 高优先级

- [ ] CLI `glide copy` 和 `glide paste` 与服务端完整集成测试
- [ ] 键鼠共享真实端到端测试 (Xvfb 上用 xdotool 模拟)
- [ ] 多客户端并发同步压力测试
- [ ] Windows NSIS/MSI 安装器在真实 Windows 环境验证
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
| Multi-client concurrent sync | Python test | ✅ |
| LAN auto-discovery | 代码实现 | ✅ |
| Keyboard/mouse protocol | Python test | ✅ (34 tests) |
| Input relay WebSocket | Python test | ✅ |
| User/password login | curl + Python test | ✅ |
| Temp token creation | curl + Python test | ✅ |
| Docker cross-container sync | Python test | ✅ |

---

## 已知问题与修复记录

| 问题 | 根因 | 修复 |
|------|------|------|
| 循环抑制失败 | WS 广播未过滤发送者 | ws.rs 添加 device_id 过滤 |
| FK 约束失败 | 客户端未注册就发事件 | ws.rs 自动注册设备 |
| SQLite 无法打开 | 缺少 `sqlite:` 前缀 | 添加 `?mode=rwc` |
| NSIS 按钮无效 | Tauri 2.x API 路径变更 | `__TAURI__.core.invoke` |
| Tauri lifetime panic | `tokio::spawn` 在 setup 前调用 | `tauri::async_runtime::spawn` |
| Windows exe 闪退 | WebView2 未安装 | NSIS 含自动引导 |
| Docker build 慢 | 每次全量重编译 | runtime-only image |
| MSI 中文语言缺失 | CI runner 无 zh-CN.nlf | 使用 en-US |
| NSIS language file 缺失 | NSIS runner 不含语言文件 | 改用 MSI only |
