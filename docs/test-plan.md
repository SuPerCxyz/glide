# Glide 测试计划

> 最后更新：2026-06-05
> 当前虚拟环境执行结果：198 通过 / 0 失败
> Windows GUI 真实桌面执行：当前环境无 Windows VM / Wine / PowerShell，已生成 VM 脚本，需在 Windows VM 执行

## 1. 测试环境

| 环境 | 方式 | 状态 |
|------|------|------|
| Linux server | `scripts/test-lib.sh` 启动隔离 `target/debug/glide-server`，随机 `127.0.0.1:<port>`，临时 `GLIDE_DATA_DIR`，`GLIDE_REGISTRATION_TOKEN=reg123` | 已真实执行 |
| Linux CLI/WebSocket | Python `websockets` + HTTP API | 已真实执行 |
| Linux GUI X11 | Xvfb + xclip + xdotool | 已真实执行 |
| Windows platform 分支 | `platform=windows` 注册、WebSocket 同协议模拟、Windows 路径/注册 payload 单元测试 | 已虚拟/单元验证 |
| Windows GUI | PowerShell / AutoHotkey / pywinauto 脚本 | 已生成，需 Windows VM |
| Wine | 本机无 `wine/wine64` | 未执行 |
| 本地 Windows VM | 本机无 `qemu-system-x86_64/virsh/VirtualBox/pwsh` | 未执行 |

## 2. 竞品参考与测试点

| 产品 | 参考点 | 转化为 Glide 测试 |
|------|--------|-------------------|
| Barrier / Input Leap | 跨平台键鼠共享、剪贴板共享；Input Leap 文档说明 Linux Wayland 剪贴板受限，配置含 SSL/fingerprint/profile 目录 | X11/Wayland 能力分支、剪贴板权限不足提示、配置目录和证书路径检查、token/指纹不泄露 |
| Deskflow / Synergy | server 捕获键鼠/剪贴板并转发到 client；协议包含键鼠、剪贴板、TLS | 输入事件编码/解码、跨屏坐标映射、断线紧急释放、协议版本/认证失败日志 |
| Microsoft Mouse Without Borders | 安全 key 配对、最多多机、共享剪贴板、单文件和 100MB 限制、防火墙规则、连接状态分色和错误状态 | token 错误/缺失、端口/DNS/防火墙提示、文件/大 payload 限制、连接阶段化状态 |
| KDE Connect | 配对、共享剪贴板、远程输入；移动端和 Wayland 权限会影响剪贴板/输入 | 配对/token 测试、Wayland 降级或提示、远程输入普通键/组合键/输入法限制 |
| LocalSend | LAN 内免云传输、跨平台、端到端加密、附近设备发现 | LAN 优先发现失败后手动地址连接、大文件、离线/无互联网测试 |
| RustDesk | rendezvous/relay、直连失败回退、剪贴板/文件传输、日志包含配置路径和连接阶段 | 服务端回退、重连、错误 IP/端口、日志脱敏、服务端重启恢复 |
| CopyQ | 多格式剪贴板历史、图片/HTML/URL/自定义格式、命令行自动化 | 文本/中文/Emoji/多行/大文本、图片/文件待扩展、历史分页、去重/回环抑制 |

参考来源：Input Leap GitHub/Wiki、Deskflow Wiki、Microsoft Learn PowerToys Mouse Without Borders、KDE Connect 文档、LocalSend 官网、RustDesk 文档、CopyQ 文档。

## 3. 自动化命令

```bash
cargo test --package glide-core --package glide-server --package glide-desktop
cargo test --package glide-gui
cargo check --workspace
cargo build --release --package glide-gui --package glide-cli --package glide-server
cargo clippy --package glide-gui --no-deps -- -D warnings
VERSION=0.1.0 DIST_DIR=dist-test ./scripts/package-linux.sh
bash scripts/test-e2e-linux.sh
bash scripts/test-network.sh
bash scripts/test-clipboard-cli.sh
bash scripts/test-keyboard-mouse-protocol.sh
bash scripts/test-reconnect.sh
bash scripts/test-gui-linux.sh
bash scripts/test-tc-network.sh
```

`make test` 现在调用 `scripts/test-e2e-linux.sh`，默认启动隔离服务端，不再依赖本机已有 `localhost:8080`。

## 4. 当前已通过测试

| 分组 | 命令 | 覆盖 | 结果 |
|------|------|------|------|
| Rust 单元/集成 | `cargo test --package glide-core --package glide-server --package glide-desktop` | core 53、desktop 23、server 13；含 Windows clipboard stub、headless clipboard、路由、token、LAN sync | 89/89 通过 |
| E2E | `bash scripts/test-e2e-linux.sh` | health、设备注册、错误/缺失 token、剪贴板 10 类文本、回环抑制、双向同步、输入事件、错误端口、Windows platform 注册、Linux→Windows 模拟同步、重连 | 34/34 通过 |
| 网络基础 | `bash scripts/test-network.sh` | health、IPv4、错误端口、隔离端口绑定、注册、坏 token、API、WebSocket | 11/11 通过 |
| CLI 剪贴板 | `bash scripts/test-clipboard-cli.sh` | 纯文本、中文、Emoji、空文本、多行、特殊字符、500KB 大文本 | 7/7 通过 |
| 键鼠协议 | `bash scripts/test-keyboard-mouse-protocol.sh` | 键盘、组合键、鼠标移动/点击/滚轮、紧急释放、路由、坐标映射、边缘检测、DPI 100/125/150 | 34/34 通过 |
| 认证重连 | `bash scripts/test-reconnect.sh` | 正确/错误/缺失 token、WebSocket 重连、5 客户端、5 客户端同步、设备注册表 | 7/7 通过 |
| Linux GUI | `bash scripts/test-gui-linux.sh` | Xvfb、xclip 写读、中文、xdotool 鼠标/键盘/点击 | 6/6 通过 |
| Slint GUI | `cargo test --package glide-gui` | GUI backend trait mock、连接状态、空 URL 拒绝、剪贴板/键鼠开关 | 3/3 通过 |
| Slint 构建 | `cargo check --workspace` | 全 workspace 编译检查；GUI 不依赖 Tauri/WebView2 | 通过，既有 crate 有 unused warnings |
| Slint clippy | `cargo clippy --package glide-gui --no-deps -- -D warnings` | 新 GUI crate 自身 lint | 通过 |
| Rust workspace | `cargo test --workspace` | core、desktop、server、cli、gui 单元/集成/doc tests | 107/107 通过，既有 crate 有 unused warnings |
| Linux 打包 | `VERSION=0.1.0 DIST_DIR=dist-test ./scripts/package-linux.sh` | deb/AppImage 生成、root owner、GUI/CLI/server 内容 | deb 11MB，AppImage 15MB |
| 网络异常 | `bash scripts/test-tc-network.sh` | 服务端重启、坏 IP 超时后恢复、快速连接/断开、1MB payload、IPv4、端口绑定 | 7/7 通过 |

## 5. 已修复问题

| Bug | 最小复现 | 根因 | 修复 | 回归 |
|-----|----------|------|------|------|
| Windows GUI 连接启用 token 的服务端失败 | 启动 `GLIDE_REGISTRATION_TOKEN=reg123` 的服务端，GUI 只输入 URL 后连接 | GUI 注册请求没有字段 `registration_token`，服务端返回 401；UI 也没有 token 输入框 | GUI 增加注册 Token 输入；`connect_to_server` 保存并传入 token；`SyncEngine::connect` 注册 payload 带 token；日志输出连接阶段和脱敏 token | `client_connection` 单元测试 3/3，E2E 错误/缺失/正确 token 通过，Windows platform 注册模拟通过 |
| `ttl_secs=0` 临时 token 偶发未过期 | `cargo test --package glide-server --test server_tests test_temp_token_expired` | 过期判断使用 `now > expires_at`，同毫秒创建和校验会误判有效 | 改为 `now >= expires_at`；清理也改为 `expires_at <= now` | server token 测试 8/8，完整 server 测试 13/13 |
| 测试脚本被本机已有 8080 服务污染 | 本机 8080 已有无 token 服务端时运行 `test-e2e.py/test-network.sh/test-reconnect.sh` | 脚本硬编码 `localhost:8080`，导致坏 token 用例打到错误服务端 | 新增 `scripts/test-lib.sh` 托管隔离服务端；更新 E2E/network/clipboard/reconnect/network anomaly 脚本 | E2E 34/34，network 11/11，reconnect 7/7 |
| `test-clipboard-cli.sh` 失败仍退出 0 | 大文本失败时脚本只打印结果 | Python 未按失败数 `sys.exit(1)` | 根据 failed 数退出；默认使用隔离服务端 | CLI clipboard 7/7 |
| `test-tc-network.sh` 会重启外部 Docker 容器且超时 | 运行脚本时执行 `docker restart glide-server` | 脚本假设存在固定 Docker 容器，且不统计 Python 子测试退出码 | 改为受控本地服务端重启和独立网络异常测试 | network anomaly 7/7 |

## 6. Windows VM 验证脚本

| 脚本 | 覆盖 | 当前状态 |
|------|------|----------|
| `scripts/check-windows-package-deps.ps1` | 安装包产物、DLL、资源、构建机路径、无 WebView2/Tauri 依赖、VC runtime 检查 | CI/Windows runner 可执行 |
| `scripts/test-windows-installer.ps1` | NSIS/MSI 安装、卸载、快捷方式、目录 | 需干净 Windows VM |
| `scripts/test-windows-installed-client.ps1` | 安装后 GUI 启动、配置/日志目录 | 需干净 Windows VM |
| `scripts/test-windows-connect.ps1` | `Test-NetConnection`、DNS、health、Windows device 注册、坏 token、WebSocket | 需 Windows VM；已与 token 参数兼容 |
| `scripts/test-windows-clipboard.ps1` | Windows 剪贴板读写、中文、空文本、大文本、Notepad 粘贴 | 需 Windows VM |
| `scripts/test-windows-gui.ahk` | Notepad 输入、复制、中文、快速剪贴板变化 | 需 Windows VM |
| `scripts/test-windows-notepad-clipboard.py` | pywinauto Notepad 自动化 | 需 Windows VM |

Windows VM 建议执行：

```powershell
.\scripts\test-windows-connect.ps1 -Server http://<linux-server-ip>:8080 -Token reg123
.\scripts\test-windows-clipboard.ps1 -Server http://<linux-server-ip>:8080
AutoHotkey.exe .\scripts\test-windows-gui.ahk
pip install pywinauto
python .\scripts\test-windows-notepad-clipboard.py
```

同时检查：

```powershell
Test-NetConnection -ComputerName <linux-server-ip> -Port 8080
Resolve-DnsName <server-host>
Get-NetTCPConnection | ? RemotePort -eq 8080
Get-Process | ? ProcessName -match "glide"
```

## 7. 测试用例矩阵

| 编号 | 名称 | 前置条件 | 步骤 | 预期 | 自动化 | 状态 |
|------|------|----------|------|------|--------|------|
| CONN-001 | 正确 token 注册 | 隔离服务端启用 `reg123` | POST `/devices/register` 携带 token | 200 registered | 是 | 通过 |
| CONN-002 | 错误 token 拒绝 | 同上 | 携带 wrong token | 401 | 是 | 通过 |
| CONN-003 | 缺失 token 拒绝 | 同上 | 不携带 token | 401 | 是 | 通过 |
| CONN-004 | 错误端口 | 隔离服务端运行 | 连接未监听端口 | 连接失败 | 是 | 通过 |
| CONN-005 | Windows platform 注册 | 隔离服务端运行 | `platform=windows` 注册并连 WS | 注册和 WS 成功 | 是，模拟 | 通过 |
| CLIP-001 | 文本多类型同步 | 两个 WS 客户端 | 发送文本/中文/Emoji/空/多行/大文本 | 接收端内容一致 | 是 | 通过 |
| CLIP-002 | 回环抑制 | 两个 WS 客户端 | 接收端发送自己的 item | 不回传给自己 | 是 | 通过 |
| CLIP-003 | 5 客户端广播 | 5 个 WS 客户端 | 1 个发送剪贴板 | 其他客户端收到 | 是 | 通过 |
| GUI-001 | Linux X11 剪贴板 | Xvfb | xclip 写读中文 | 内容一致 | 是 | 通过 |
| GUI-002 | Windows Notepad 剪贴板 | Windows VM | AHK/PowerShell 操作 Notepad | 复制粘贴一致 | 是，需 VM | 待执行 |
| INPUT-001 | 键盘事件 | 协议脚本 | Ctrl+C、Ctrl+Alt+Del、Win、F1 等编码 | JSON 正确 | 是 | 通过 |
| INPUT-002 | 鼠标事件 | 协议脚本 | 移动/点击/滚轮 | JSON 正确 | 是 | 通过 |
| INPUT-003 | 跨屏坐标/DPI | 协议脚本 | 100/125/150% 映射 | 坐标符合预期 | 是 | 通过 |
| INPUT-004 | 断线安全释放 | 协议脚本 | EmergencyRelease 编码 | 可被传输 | 是 | 通过 |
| NET-001 | 服务端重启 | 隔离服务端 | kill 后重启同端口 | health 恢复 | 是 | 通过 |
| NET-002 | 坏 IP | 虚拟不可达地址 | 连接 `10.255.255.1` | 超时后正常服务可重连 | 是 | 通过 |
| NET-003 | 大 payload | WS | 发送 1MB 文本 | 发送成功 | 是 | 通过 |
| SEC-001 | token 脱敏 | 单元测试 | `mask_secret` | 不输出明文 | 是 | 通过 |
| GUI-003 | Slint GUI mock backend | 本地 Rust 环境 | `cargo test --package glide-gui` | 状态、连接、开关 mock 逻辑通过 | 是 | 通过 |
| GUI-004 | Slint GUI 构建 | 本地 Rust 环境 | `cargo check --package glide-gui` | 无 Tauri/WebView2 依赖，UI 代码生成成功 | 是 | 通过 |

## 8. 当前无法执行项

| 项目 | 原因 | 替代验证 |
|------|------|----------|
| Windows NSIS/MSI 干净安装 | 本机无 Windows VM | GitHub Windows runner 构建 + PowerShell VM 脚本 |
| Windows GUI 实际连接/托盘 | 本机无 Windows 桌面/PowerShell/Wine | Windows platform 注册和 WebSocket 同协议模拟；VM 脚本待执行 |
| Windows ↔ Linux 真实系统剪贴板 | 本机无 Windows VM | Linux CLI/GUI 真实执行 + Windows PowerShell/AHK 脚本 |
| Wayland 真实剪贴板 | 本机未安装 weston/wl-clipboard 组合 | X11 Xvfb 已执行；Wayland backend detect 单元测试 |
| 真实跨屏键鼠 | 当前仅单机虚拟环境 | 协议、坐标、DPI、边缘、紧急释放自动化覆盖；真实多屏需 VM/多显示器 |

## 9. 结论

当前 Linux 虚拟环境、Headless GUI、Mock/协议层、Windows platform 模拟下没有已知失败。Windows 客户端无法连接服务端的直接修复是：GUI 现在支持输入并提交 registration token，并在日志中输出连接阶段、目标地址和脱敏 token。安装包模式和 Windows GUI 实际剪贴板/键鼠仍需在干净 Windows 10/11 VM 中执行已生成脚本验证。
