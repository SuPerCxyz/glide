# Glide 产品需求与设计文档

> 注意：本文是历史产品需求与设计记录，部分状态可能已经过期。
> 当前稳定事实来源请优先阅读：
> [TECHNICAL.md](TECHNICAL.md)、[PRODUCT.md](PRODUCT.md)、
> [ARCHITECTURE.md](ARCHITECTURE.md)、[DESIGN.md](DESIGN.md)。
> 如果本文与当前代码或稳定文档冲突，以当前代码和稳定文档为准。

> 本文档记录 Glide 产品的所有需求、设计决策和技术实现细节。

---

## 1. 产品定位

Glide 是一个局域网优先、服务端回退的跨设备剪贴板同步工具，支持可选的键盘/鼠标共享。

**核心价值：** 在可信设备之间无缝同步剪贴板内容，同一二层网络内自动组网，无需依赖服务端。

---

## 2. 核心需求

### 2.1 剪贴板同步

| 需求 | 优先级 | 状态 |
|------|--------|------|
| 文本同步 | P0 | ✅ 已实现 |
| 图片同步 | P0 | ✅ 类型已定义，MIME 模型已实现 |
| 文件/文件夹同步 | P0 | ✅ 类型已定义，载荷引用已实现 |
| 富文本内部保留 (HTML/RTF) | P1 | ✅ MIME 表示模型已实现 |
| URL 内部保留 | P1 | ✅ text/uri-list 支持 |
| 颜色值内部保留 | P2 | ✅ 文本变体 |

**设计原则：** 用户可见的类型只有三种：文本、图片、文件/文件夹。富文本、URL、颜色作为文本的内部 MIME 表示，不单独暴露为产品级分类。

### 2.2 路由优先级

**剪贴板路由：**
1. **本地回环检测** — 防止自己的事件回传
2. **LAN 直连** — 同一二层网络内 UDP 多播发现后 WebSocket 直传
3. **LAN 反向拉取** — 设备在线但直连失败时
4. **服务端回退** — 跨网段时通过 glide-server 中继

**输入路由（键盘/鼠标共享）：**
1. LAN 直连（最低延迟）
2. 服务端中继（延迟较高，有降级状态提示）
3. 两者都失败时断开并释放输入控制

### 2.3 设备管理

| 需求 | 状态 |
|------|------|
| 持久化设备注册（受信任设备） | ✅ |
| 临时 CLI 会话（不写配置、不加入可信网络） | ✅ |
| 设备注册令牌验证 | ✅ |
| 按设备同步策略 | ✅ |
| 按类型同步策略 | ✅ |

### 2.4 客户端形态

| 形态 | 平台 | 状态 |
|------|------|------|
| 桌面 GUI（Slint） | Linux X11 / Wayland 启动 | ✅ 第一阶段 |
| 桌面 GUI（Slint） | Windows x64 | ✅ 第一阶段 |
| 桌面 GUI（Slint） | macOS | ⏳ 架构预留 |
| CLI 工具 | Linux/Windows/macOS | ✅ |
| 无头模式 | Linux 服务器 | ✅ |

### 2.5 系统托盘与后台运行

| 需求 | 状态 |
|------|------|
| 关闭窗口隐藏到系统托盘（不退出） | ✅ |
| 托盘菜单：显示窗口、暂停/恢复同步、键鼠共享开关、退出 | ✅ |
| 单击托盘图标显示窗口 | ✅ |
| Windows 从托盘恢复窗口 | ✅ |

---

## 3. LAN 自动组网

### 3.1 需求

多个客户端在同一二层网络内时，必须能够自动发现彼此并直接同步剪贴板，不依赖服务端。

### 3.2 实现方案

- **UDP 多播公告** — 每 5 秒向 `239.255.0.1:9998` 发送设备公告
- **自动发现** — 监听多播组，发现同网段设备
- **WebSocket 直连** — 发现后自动建立点对点 WebSocket 连接
- **剪贴板直传** — 变更直接发给所有 LAN 节点，不经过服务端
- **PeerRegistry** — 跟踪在线设备，心跳超时自动移除

### 3.3 排除范围

- 临时 CLI 会话（`--server` + `--token`）永远不加入可信设备网络
- 键鼠共享默认关闭

---

## 4. 服务端

### 4.1 功能清单

| 功能 | 状态 |
|------|------|
| 设备注册 API | ✅ |
| WebSocket 同步通道 | ✅ |
| HTTP 载荷上传/下载 | ✅ |
| 剪贴板历史查询 | ✅ |
| 设备列表查询 | ✅ |
| 临时令牌验证 | ✅ |
| 定时清理（保留期 + 容量） | ✅ |
| Web 管理面板 | ✅ |
| 输入中继（可选） | ✅ 框架已实现 |

### 4.2 Web 管理面板

- 地址：`http://server:8080/`
- 功能：服务状态、设备列表、剪贴板历史、环境配置
- 自动刷新：每 5 秒
- 暗色主题 UI

### 4.3 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `GLIDE_DATA_DIR` | `/data` | 数据目录 |
| `GLIDE_MAX_STORAGE_BYTES` | `1073741824` | 最大存储 1GB |
| `GLIDE_RETENTION_DAYS` | `30` | 保留天数 |
| `GLIDE_MAX_ITEM_BYTES` | `10485760` | 单项最大 10MB |
| `GLIDE_PUBLIC_URL` | `http://localhost:8080` | 公网地址 |
| `GLIDE_ADMIN_TOKEN` | — | 管理令牌 |
| `GLIDE_REGISTRATION_TOKEN` | — | 设备注册令牌 |
| `GLIDE_TEMP_TOKEN_DEFAULT_TTL` | `3600` | 临时令牌有效期 |
| `GLIDE_INPUT_RELAY_ENABLED` | `false` | 输入中继开关 |
| `GLIDE_INPUT_RELAY_MAX_LATENCY_MS` | `200` | 延迟告警阈值 |

---

## 5. CLI 工具

### 5.1 命令

```bash
glide copy "文本内容"                    # 复制文本
glide copy --file ./document.zip         # 复制文件
glide copy --dir ./project               # 复制目录
glide copy --image ./screenshot.png      # 复制图片
glide paste                              # 粘贴到 stdout
glide paste --output ./received          # 粘贴到文件
glide history --limit 50                 # 查看历史
glide devices                            # 列出设备
```

### 5.2 认证模式

- **持久化模式** — 使用 `~/.config/glide/config.json` 配置文件
- **临时会话** — `--server URL --token TOKEN`，不写配置文件

---

## 6. 打包与部署

Windows 安装包依赖策略详见 [Windows Packaging Design](windows-packaging.md)。

### 6.1 安装包

| 平台 | 格式 | 包含内容 | 状态 |
|------|------|----------|------|
| Linux | deb | GUI + Daemon + CLI + Server | ✅ |
| Linux | rpm | GUI + Daemon + CLI + Server | ✅ |
| Linux | AppImage | GUI + Daemon + CLI + Server | ✅ |
| Windows | portable zip | Slint GUI + CLI + Server | ✅ |
| Windows | NSIS/MSI | 后续安装器阶段 | ⏳ |
| Windows | zip | GUI + CLI + Server | ✅ |
| Docker | 镜像 | Server + CLI | ✅ |

### 6.2 Docker 镜像标签

- `dev-latest` — 最新构建
- `YYYYMMDDHHmm` — 构建时间（如 `202606042118`）

### 6.3 GitHub Actions CI

普通 CI 只做快速验证；Linux/Windows 安装包和 Docker 发布镜像在
Release workflow 中构建。手动触发 Release 时可通过 `build_linux`、
`build_windows`、`build_docker` 按需选择产物；tag `v*` 发布时全量构建。

| Job | 说明 |
|-----|------|
| Build & Test (Linux) | 构建 + 测试 + Clippy |
| Build & Test (Windows) | 构建 glide-core/server/cli/gui |
| Verify Docker Build | 构建并验证 Docker 镜像 |
| Linux Packages (Release) | deb + rpm + AppImage |
| Windows Packages (Release) | portable zip |
| Docker Image (Release) | 构建并验证发布镜像 |

---

## 7. 安全模型

- 所有传输使用 TLS/WSS
- 服务端明文存储剪贴板
- 持久化设备使用注册令牌注册
- 临时令牌有 TTL、最大使用次数、操作限制、大小限制
- 临时会话永不加入可信设备网络
- 输入中继仅限受信任设备
- 路径遍历保护（payload ID 校验）

---

## 8. 键盘/鼠标共享

- **默认关闭**
- LAN 直连模式（最低延迟）
- 服务端中继模式（延迟较高，有降级提示）
- 紧急释放控制（任一节点可立即断开）
- 边缘穿越检测（光标跨屏幕边缘切换设备）
- 速率限制（每秒事件数上限）
- 延迟告警（超过阈值时通知）

---

## 9. 已知限制与后续计划

- GUI 使用 Slint，不依赖 Tauri、WebView2、Electron、Chromium 或系统 WebView。
- macOS/iOS/Android 为未来计划
- 键鼠共享的 Windows 实现需要额外的输入注入 API
- Linux 第一阶段优先 X11；Wayland 下全局键鼠控制受合成器权限限制，必须展示限制提示。
- 服务端 TLS 需自行配置反向代理

---

## 10. 关键设计决策记录

| 决策 | 原因 |
|------|------|
| Slint 替代 Tauri | 移除 WebView2/WebKit 运行时依赖，降低 Windows 离线包体积并统一跨平台样式 |
| GUI 仅访问 backend trait/daemon IPC | 网络、剪贴板、键鼠和文件传输不散落在 GUI，后续可切换 Named Pipe/Unix socket |
| UDP 多播而非纯 mDNS | 更可靠，不依赖系统 mDNS 服务 |
| SQLite 而非 PostgreSQL | 单机部署简单，Docker 友好 |
| 普通 Cargo 构建 + 轻量打包 | Linux 生成 deb/AppImage，Windows 先发布 portable zip，后续再补轻量 installer |
| 单一服务端二进制 | 简化部署，server + CLI 合一 |
| 关闭窗口隐藏到托盘 | 后台同步需要持续运行 |
| Cargo.lock 提交到仓库 | 确保 CI 使用确定性依赖版本 |
