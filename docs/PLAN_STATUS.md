# Glide LAN-First Plan 状态核对

> 对照 `docs/superpowers/plans/2026-06-04-glide-lan-first-sync.md`。
> 更新日期：2026-06-06。状态以当前代码、脚本、CI 和稳定文档为准。

| Plan 任务 | 当前状态 | 代码/脚本位置 | 已测试 | 产品文档记录 | 未完成细节 / 后续验证 |
|-----------|----------|---------------|--------|--------------|------------------------|
| Rust workspace | 已实现 | `Cargo.toml` | `cargo check --workspace` | 是 | 无 |
| 核心类型：设备、剪贴板、MIME、payload、sync/input event | 已实现 | `crates/glide-core` | `cargo test --workspace` | 是 | 协议变化需补跨 crate 测试 |
| server schema/API/WebSocket/history/payload | 已实现 | `crates/glide-server` | `cargo test --package glide-server` | 是 | history SQL 拼接后续应参数化 |
| 临时 token TTL/次数/操作/大小 | 已实现 | `temp_token.rs` | server token tests | 是 | CLI 临时 token 与注册 token 产品语义需统一 |
| mDNS/UDP 多播发现 | 部分实现 | `glide-core::discovery`、`glide-desktop::lan_sync` | 单元/脚本 | 是 | 未接 daemon/GUI 产品链路 |
| 剪贴板路由选择 | 已实现模型 | `crates/glide-core/src/route.rs` | route tests | 是 | 真实多节点路由需 daemon 验证 |
| 输入路由和安全机制 | 部分实现 | `route.rs`、`input_adapter.rs`、`lan_input.rs` | 协议/单元测试 | 是 | 真实跨屏/断线释放需桌面环境 |
| CLI 持久化模式 | 部分实现 | `glide-cli/src/config.rs` | CLI smoke、Windows CLI Wine smoke | 是 | 配置初始化命令未实现 |
| CLI 临时认证模式 | 部分实现 | `glide-cli/src/main.rs`、`commands.rs` | 基础测试 | 是 | 临时 token 未完整贯穿所有 API |
| CLI text copy/paste/history/devices | 部分实现 | `glide-cli/src/commands.rs` | clipboard scripts、`test-windows-cli-wine.sh` | 是 | 实际系统剪贴板未接入 CLI |
| CLI file/image/folder copy | 部分实现 | `commands.rs` | `test-cli-payload.sh` 和 `test-windows-cli-wine.sh` 覆盖单文件 | 是 | 目录恢复、图片剪贴板、大文件/断点续传待做；真实 Win11 桌面 CLI 待验证 |
| Linux clipboard adapter | 部分实现 | `glide-desktop/src/linux_backends` | headless/X11 tests | 是 | Wayland 实际桌面待验证 |
| Windows clipboard adapter | 部分实现 | `windows_clipboard.rs` | 非 Windows stub tests | 是 | Windows VM 实测待执行 |
| Slint GUI 第一阶段 | 已实现页面，mock 后端 | `crates/glide-gui` | `cargo test --package glide-gui` | 是 | 未接真实 daemon IPC |
| LAN 自动组网引擎 | 部分实现 | `lan_sync.rs` | LAN sync tests | 是 | 多机/namespace 实际验证待扩展 |
| per-device/per-type 策略 | 已实现模型 | `policy.rs` | core tests | 是 | GUI 配置入口待做 |
| 跨节点键鼠共享 | 部分实现 | `lan_input.rs`、`input_adapter.rs` | 协议/坐标测试 | 是 | Windows SendInput、真实捕获、跨屏实测待做 |
| 屏幕边缘和显示器布局 | 已实现模型 | `display.rs`、`input_adapter.rs` | display/input tests | 是 | GUI 布局编辑待做 |
| Web 管理面板 | 已实现 | `crates/glide-server/static/index.html` | Docker/health smoke | 是 | 登录/安全体验需加强 |
| Docker runtime-only image | 已实现 | `Dockerfile` | CI Docker verify | 是 | 镜像只包含 server/CLI |
| CI Linux/Windows/Docker/Package | 已实现 | `.github/workflows/ci.yml` | CI 配置 | 是 | Windows GUI 启动不是 CI 真 GUI 验证 |
| Release deb/rpm/AppImage/Windows zip/Docker | 已实现配置 | `.github/workflows/release.yml` | release 配置、本地 package smoke | 是 | Windows installer 待规划 |
| Windows NSIS/MSI installer | 未实现/已废弃旧 Tauri 路线 | 无当前 installer | 否 | 是 | 需新 Slint installer 和 Windows VM |
| macOS clipboard/build/权限 | 未实现 | 仅 core 平台枚举 | 否 | 是 | 需要 macOS 环境 |
| Windows SendInput | 未实现 | 无 Windows input backend | 否 | 是 | 需要 Windows VM |
| 断点续传 | 未实现 | 无 | 否 | 是 | 文件传输 P2 |
| Android/iOS/Web 扩展 | 规划中 | 无 | 否 | 是 | 暂不纳入近期版本 |
| E2EE | 规划中 | 无 | 否 | 是 | 需要密钥/配对设计 |
