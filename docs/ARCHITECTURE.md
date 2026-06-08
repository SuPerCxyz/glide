# Glide 架构文档

> 本文档描述当前模块关系、运行流程和后续演进边界。更新日期：2026-06-06。

## 1. 总体架构

```text
                         ┌────────────────────┐
                         │    glide-server     │
                         │ Axum + SQLite + WS  │
                         └─────────▲──────────┘
                                   │ HTTP / WebSocket
                                   │
┌──────────────┐        ┌──────────┴───────────┐
│  glide-cli   │        │     glide-daemon      │ 规划中：真实后台常驻服务
│ headless CLI │        │ status/settings now   │
└──────▲───────┘        └──────────▲───────────┘
       │                            │ planned IPC
       │                            │
┌──────┴────────┐       ┌───────────┴──────────┐
│ glide-core    │       │      glide-gui        │
│ shared model  │       │ Rust + Slint + mock   │
└──────▲────────┘       └──────────────────────┘
       │
┌──────┴────────┐
│ glide-desktop │
│ platform libs │
└───────────────┘
```

当前真实状态：

- `glide-server`、`glide-cli`、`glide-core` 有实际可运行链路。
- `glide-gui` 是 Slint GUI 第一阶段，使用模拟后端。
- `glide-daemon` 是 skeleton。
- `glide-desktop` 是库级平台能力集合，尚未由 daemon 统一驱动。

## 2. 模块边界

### glide-core

只放共享模型和纯逻辑：

- 设备身份和平台类型。
- 剪贴板 item、MIME 表示、payload 引用。
- 同步事件和输入事件。
- 路由选择、策略、发现、显示器布局。
- token 脱敏辅助。

### glide-server

负责跨网段回退和中心化存储/中继：

- 设备注册和 token。
- SQLite 数据库。
- 管理页面。
- 剪贴板历史和 payload。
- WebSocket 同步。
- 输入中继 endpoint。

### glide-cli

负责无 GUI 的调试和自动化入口：

- 读取 CLI 参数或持久配置。
- 调用 server HTTP API。
- 通过 WebSocket 发送 sync event。
- 不依赖 Slint GUI。

### glide-desktop

负责平台能力库：

- 剪贴板 backend trait。
- Linux/Windows clipboard 分支。
- 输入注入 backend trait。
- LAN sync engine。
- LAN input engine。

该 crate 当前不是独立运行进程。

### glide-daemon

目标是后台常驻服务。当前只实现：

- daemon settings/status。
- connect/disconnect 状态模型。
- 剪贴板/键鼠开关。
- 日志 tail。
- `--print-status`。

规划中：接管网络连接、剪贴板监听、键鼠事件、文件传输、LAN discovery 和本地 IPC。

### glide-gui

负责显示和配置，不直接执行核心能力：

- Slint UI。
- `GuiBackend` trait。
- `MockBackend`。
- 状态、设备、配对、日志、设置、平台能力、关于页面。

## 3. 数据流

### 3.1 设备发现

已存在模型/库：

- `glide-core::discovery`
- `glide-desktop::lan_sync`

设计流：

```text
本机 daemon
  -> UDP multicast announcement
  -> PeerRegistry 更新在线设备
  -> route selector 优先选择 LAN direct
```

当前产品状态：LAN discovery 库存在，但未由 daemon/GUI 串成完整产品流程。

### 3.2 配对

当前实现：

- server 支持 `GLIDE_REGISTRATION_TOKEN` 设备注册校验。
- GUI 配对页是 mock。

规划流：

```text
GUI 请求配对
  -> daemon 生成/接收配对码或 token
  -> server 或 LAN peer 校验
  -> 保存可信设备和 token/指纹
  -> GUI 刷新设备状态
```

### 3.3 连接

当前实现：

- CLI 可以指定 server URL/token。
- server 默认监听 `0.0.0.0:8080`。
- GUI connect 只改变模拟后端状态。

规划流：

```text
GUI 保存 server URL
  -> daemon 解析地址
  -> HTTP health / device register
  -> WebSocket sync connect
  -> auth/pairing
  -> daemon 状态推送给 GUI
```

### 3.4 剪贴板同步

当前 CLI/server 文本流：

```text
glide-cli copy
  -> register device
  -> build ClipboardItem
  -> WebSocket /ws/sync
  -> server 存入 SQLite 并广播
  -> 其他客户端接收或通过 history/paste 拉取
```

规划 GUI/daemon 流：

```text
platform clipboard watcher
  -> daemon 去重和策略检查
  -> route selector
  -> LAN direct 或 server fallback
  -> 接收端 daemon apply 到系统剪贴板
```

### 3.5 键鼠控制

当前实现：

- `glide-core::input_event` 定义键盘/鼠标事件。
- `glide-desktop::input_adapter` 实现输入 backend trait、边缘检测、速率限制、紧急释放。
- `glide-desktop::linux_backends::linux_input` 提供 X11/xdotool 输入注入后端。
- `glide-desktop::windows_input` 提供 Windows SendInput 输入注入后端。
- `glide-desktop::platform_input` 根据目标平台选择 Linux/Windows 输入后端。
- `glide-desktop::lan_input` 有 LAN input skeleton，目标端 consumer 已使用平台选择器，不再写死 Linux 后端。

当前缺口：

- GUI 的键鼠开关仍是模拟状态。
- daemon 尚未建立真实 input session，也没有捕获本机输入事件、处理设备布局或执行断线释放。
- Windows SendInput 后端已通过交叉构建，但真实 Win11 注入还需要 VM/物理机安全测试。

规划流：

```text
本机边缘触发
  -> daemon 建立 input session
  -> 捕获输入事件
  -> LAN direct 或 server relay
  -> 目标平台 backend 注入输入
  -> 断线/紧急释放时停止控制
```

### 3.6 文件传输

当前实现：

- core 有 payload ref。
- server 有 upload/download API。
- CLI 有 file/dir/image 参数和 payload ref 生成。
- CLI 单文件会上传 payload，并由另一设备通过 `paste --output` 下载。

当前缺口：

- 目录会归档为临时 tar.gz 后上传，但还缺恢复目录结构的产品级下载体验。
- 图片路径复用文件 payload 上传，仍缺跨平台系统剪贴板图片粘贴验证。
- GUI 没有发送文件页面。

### 3.7 日志诊断

当前实现：

- server 使用 tracing 输出启动、监听、清理、WebSocket、解析失败等日志。
- daemon skeleton 有内存日志。
- GUI 显示模拟后端日志，并提供 `--smoke` 诊断入口。
- GUI 启动和 panic hook 会写入本地诊断日志，Windows 默认路径为 `%APPDATA%\Glide\logs\glide-gui.log`。

规划中：

- daemon 写持久日志。
- GUI `tail_logs()` 通过 IPC 获取日志。
- token 和密钥统一脱敏。

## 4. 本地 IPC 设计

当前没有真实 IPC。`crates/glide-gui/src/gui_backend.rs` 定义了第一阶段接口：

- `get_service_status()`
- `start_service()` / `stop_service()`
- `list_devices()` / `get_device_detail()`
- `pair_device()`
- `connect_device()` / `disconnect_device()`
- `connect_server()` / `disconnect_server()`
- `get_clipboard_status()` / `set_clipboard_enabled()`
- `get_input_status()` / `set_input_enabled()`
- `get_file_transfer_status()` / `send_file()`
- `get_settings()` / `update_settings()`
- `tail_logs()` / `export_diagnostics()`
- `get_platform_capabilities()`

规划：

- Windows：Named Pipe。
- Linux/macOS：Unix Domain Socket。
- 消息格式：建议 JSON 或 bincode，需版本字段和错误阶段字段。
- GUI 不应直接链接底层 platform 执行逻辑。

## 5. 跨平台适配层

当前项目还没有单独 `glide-platform` crate，平台适配主要在 `glide-desktop` 内。

后续演进方向：

- 将系统剪贴板、输入注入、托盘、开机启动、权限检测、服务安装拆到明确 platform 层。
- Linux 分离 X11 和 Wayland 能力。
- macOS 增加权限检测和引导。
- Windows 增加 portable/installer 依赖检查、Named Pipe、开机启动和防火墙提示策略。

## 6. 后续演进路线

1. 将 `glide-daemon` 从 skeleton 扩展为真实后台服务。
2. 把 `glide-desktop` 能力接入 daemon。
3. 实现本地 IPC，并让 `glide-gui` 替换模拟后端。
4. 补齐 CLI 文件 payload 上传和下载完整链路。
5. 加入真实配对流程和设备信任模型。
6. 完善 Windows portable/installer 验证。
7. 增加 Linux tray 和 X11/Wayland 权限提示。
8. 规划 macOS 打包和权限引导。
