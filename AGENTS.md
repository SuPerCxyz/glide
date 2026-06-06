# Glide Project Context

## 稳定文档入口

后续代理必须优先使用以下稳定文档理解项目当前状态：

- 技术事实来源：[docs/TECHNICAL.md](docs/TECHNICAL.md)
- 产品事实来源：[docs/PRODUCT.md](docs/PRODUCT.md)
- 架构事实来源：[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- 设计事实来源：[docs/DESIGN.md](docs/DESIGN.md)

历史和专项文档仍可作为参考，但如果与稳定文档或代码冲突，必须以当前代码为准，并同步修正稳定文档。

## 修改前阅读稳定文档

任何涉及技术实现、产品能力、架构设计、GUI、CLI、daemon、core、platform、打包、安装、交互、视觉、协议、配置、日志或测试的任务，在修改代码前必须先阅读：

1. `docs/TECHNICAL.md`
2. `docs/PRODUCT.md`

如果任务涉及模块关系、进程关系、接口边界、IPC、平台分层或系统运行流程，必须同时阅读：

3. `docs/ARCHITECTURE.md`

如果任务涉及 UI、交互、视觉、页面、组件、图标、安装体验或用户提示，必须同时阅读：

4. `docs/DESIGN.md`

## 修改后同步稳定文档

完成修改后必须检查是否需要同步稳定文档：

- 技术方案、依赖、构建、测试、配置、协议、日志、CI、打包变化：更新 `docs/TECHNICAL.md`。
- 产品能力、用户流程、功能边界、平台支持、分发策略变化：更新 `docs/PRODUCT.md`。
- 模块关系、进程关系、接口边界、IPC、daemon/core/platform 分层变化：更新 `docs/ARCHITECTURE.md`。
- UI、交互、视觉、页面、组件、错误提示、安装体验变化：更新 `docs/DESIGN.md`。

不允许把规划内容写成已完成。已实现、部分实现、规划中、未实现必须明确区分。

## 文档真实性规则

- 文档只能记录当前仓库真实实现和明确规划。
- 已实现内容必须能从代码、配置、脚本、CI 或已有验证记录中找到依据。
- 不确定内容必须写“不确定”或“需要后续确认”。
- 如果代码与文档冲突，必须检查实际代码，并在本次任务中同步修正文档。
- 如果发现稳定文档过期，必须在本次任务中更新。

## 最终输出要求

每次涉及技术、产品或设计变化的任务，最终回复必须说明：

- 是否阅读了稳定文档。
- 修改了哪些代码。
- 修改了哪些文档。
- 是否存在未同步的文档项。
- 运行了哪些测试或检查，以及真实结果。

## 当前核心需求

1. LAN 优先同步：同一二层网络内自动发现、直连、同步；当前库级能力存在，产品链路仍在接入。
2. 服务端回退：跨网段时通过 `glide-server` 中继。
3. 全节点同步：任一可信节点复制后同步到其他可信节点；CLI/server 文本路径已覆盖，GUI/daemon 待接入。
4. 临时 CLI 会话：`--server` + `--token`，不写配置、不加入可信网络。
5. Slint GUI：不依赖 Tauri、WebView2、Electron、Chromium 或系统 WebView。
6. Web 管理面板：服务端 `GET /` 展示状态和管理入口。
7. Docker 一键部署：`docker-compose` 启动 server，带数据目录和 healthcheck。
8. 按设备/按类型同步策略：核心模型存在，产品配置入口仍需完善。
9. 键鼠共享：默认关闭，协议和库级 skeleton 存在，真实产品链路仍在规划/接入。

## 当前架构

```text
crates/glide-core/      共享类型、协议、发现、路由、策略、输入/显示模型
crates/glide-server/    中央服务端 (Axum + SQLite + WebSocket)
crates/glide-cli/       无头 CLI 工具
crates/glide-desktop/   桌面库 (剪贴板适配、LAN 同步、输入共享)
crates/glide-daemon/    后台服务 skeleton，后续承接真实能力
crates/glide-gui/       Rust + Slint GUI，当前使用 MockBackend
```

仓库不再保留 `crates/glide-tauri` 作为主 GUI。不要恢复 Tauri/WebView2 方案，除非用户明确要求重新评估并更新稳定文档。

## 关键技术决策

- GUI 使用 Rust + Slint。
- GUI 必须保持薄层，只负责状态、配置、配对入口、开关、日志和提示。
- 后续真实网络、剪贴板、键鼠、文件传输应由 daemon/platform/desktop 层执行。
- Windows 后续 IPC 规划为 Named Pipe；Linux/macOS 后续 IPC 规划为 Unix Domain Socket。
- SQLite 用于 server 单机数据存储。
- Docker 镜像为 runtime-only，CI 先构建二进制再打镜像。
- Cargo.lock 提交到仓库，保证 CI 构建确定性。

## CI/CD

- 普通 CI：Linux build/test、Docker verify、Linux `.deb/.rpm/.AppImage` package artifact、Windows native build artifact。
- Release workflow：按输入或 tag 构建 Linux package、Windows portable zip、Docker image。
- Windows 当前发布形态是 portable zip；NSIS/MSI installer 仍是后续规划。
- 文档变更通常不触发 CI，代码或构建相关变更完成后需要按任务要求运行本地验证，必要时手动触发 CI。

## 部署

- 服务端：`docker run -p 8080:8080 -e GLIDE_DATA_DIR=/data glide-server:dev-latest`
- Docker Compose：`docker compose up --build`
- 管理面板：http://server:8080/
- 测试服务器：http://aicode.soocoo.xyz:8080/

## 代码规范

- 当前分支内可默认修改应用代码、测试、局部文档。
- 删除文件、大规模重构、shared contract/schema/root CI/依赖变更需确认，除非用户已明确授权。
- 不要提交 token、密钥、隐私路径或明文凭据。
- Commit 信息：祈使句、50 字符首行、72 字符正文。
- 没有真实验证证据，不得声称构建、测试、GUI 启动、Windows 安装或跨平台能力已通过。
