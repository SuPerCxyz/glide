# Glide Project Context

## 设计文档

- 产品需求与设计：[docs/design.md](docs/design.md)
- 实施计划与进度：[docs/superpowers/plans/2026-06-04-glide-lan-first-sync.md](docs/superpowers/plans/2026-06-04-glide-lan-first-sync.md)

## 核心需求

1. **LAN 优先同步** — 同一二层网络内自动发现、直连、同步，不依赖服务端
2. **服务端回退** — 跨网段时通过 glide-server 中继
3. **全节点同步** — 任一可信节点复制，同步到所有可信节点
4. **临时 CLI 会话** — `--server` + `--token`，不写配置、不加入可信网络
5. **系统托盘后台运行** — 关闭窗口隐藏到托盘，不退出
6. **多平台 GUI** — Linux (webkit2gtk) + Windows (WebView2)
7. **Web 管理面板** — 服务端 GET / 展示状态、设备、历史
8. **Docker 一键部署** — docker-compose 启动，带数据清理
9. **按设备/按类型同步策略** — 细粒度控制
10. **键鼠共享** — 默认关闭，LAN 直连优先，服务端中继回退

## 架构

```
crates/glide-core/      共享类型、协议、发现、路由
crates/glide-server/    中央服务端 (Axum + SQLite + WebSocket)
crates/glide-cli/       无头 CLI 工具
crates/glide-desktop/   桌面库 (剪贴板适配、LAN 同步、输入共享)
crates/glide-tauri/     Tauri 2.x GUI 应用
```

## 关键技术决策

- Tauri 2.x（兼容 Ubuntu 24.04 webkit2gtk-4.1）
- UDP 多播 239.255.0.1:9998 发现（不依赖系统 mDNS）
- SQLite（单机部署简单）
- `cargo tauri build`（自动处理 WebView2 引导）
- Cargo.lock 提交到仓库（CI 确定性构建）
- Docker 标签：`dev-latest` + `YYYYMMDDHHmm`

## 已知问题

- serde 枚举变体名必须 PascalCase（`Text` 不是 `text`）
- Docker 容器以 root 运行，数据卷需要权限
- SQLite URL 需要 `sqlite:` 前缀 + `?mode=rwc`
- Migration 必须在 cleanup 前完成
- 本地 rustfmt 1.75 与 CI 最新版格式不同

## CI/CD

- GitHub Actions：Linux + Windows + Docker + Package
- Release：deb + rpm + AppImage + NSIS + MSI + zip + Docker 镜像
- 仓库同步：Gitea → GitHub

## 部署

- 服务端：`docker run -p 8080:8080 -e GLIDE_DATA_DIR=/data glide-server:dev-latest`
- 管理面板：http://server:8080/
- 测试服务器：http://aicode.soocoo.xyz:8080/

## 代码规范

- 当前分支内可默认修改应用代码、测试、局部文档
- 删除文件、大规模重构、shared contract/schema 需确认
- Commit 信息：祈使句、50 字符首行、72 字符正文
- 首次实现时阅读 plan 文件并按任务执行
