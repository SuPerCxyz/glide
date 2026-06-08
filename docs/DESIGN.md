# Glide 稳定设计文档

> 本文档记录当前 GUI、交互、视觉和安装体验设计事实。更新日期：2026-06-06。

## 1. 产品视觉方向

Glide 当前 GUI 使用 Rust + Slint 实现。视觉目标是轻量、清晰、跨平台一致，不追求系统原生控件外观，也不依赖 WebView。

当前第一阶段 UI 是管理型桌面工具界面：

- 左侧导航。
- 顶部状态栏。
- 主内容区展示状态、设备、配对、日志、设置、平台能力和关于。
- 颜色克制，优先传达连接状态、能力限制和可操作配置。

## 2. 统一样式原则

当前样式在 `crates/glide-gui/ui/app.slint` 中定义，主要颜色如下：

| 变量 | 当前值 | 用途 |
|------|--------|------|
| `primary` | `#1f7a8c` | 品牌色、选中导航 |
| `background` | `#f6f8fa` | 页面背景 |
| `surface` | `#ffffff` | 卡片/表单背景 |
| `border` | `#d0d7de` | 边框 |
| `text-primary` | `#24292f` | 主文本 |
| `text-secondary` | `#57606a` | 次文本 |
| `success` | `#1a7f37` | 成功/在线 |
| `warning` | `#9a6700` | 警告/限制 |
| `danger` | `#cf222e` | 失败/未连接 |

当前圆角以 4px 到 6px 为主，页面间距主要为 10px 到 22px。

## 3. 基础组件

当前已在 Slint 文件中实现或等价实现：

- `StatusBadge`：连接状态、在线/离线状态。
- `DeviceCard`：设备名称、平台和在线状态。
- `SettingsRow`：带说明和 Switch 的设置行。
- `PermissionNotice`：能力限制和权限提示。
- `Sidebar`：页面导航。
- `TopBar`：页面标题和连接状态。
- `LogPage` 中的日志视图。

当前未单独抽象但应保持设计一致的组件：

- `Button`：使用 Slint 标准 `Button`。
- `Switch`：使用 Slint 标准 `Switch`。
- `Dialog`：未实现。
- `EmptyState`：设备列表空状态已有局部实现，尚未抽象。

## 4. 当前页面

| 页面 | 设计目标 | 当前状态 |
|------|----------|----------|
| 状态页 | 快速查看后台服务、连接状态、服务端地址、在线设备数、剪贴板/键鼠开关 | 已实现 |
| 设备页 | 展示受信任设备和在线状态 | 已实现，数据为 mock |
| 配对页 | 进入配对流程 | 已实现占位，真实配对未接入 |
| 日志页 | 查看近期诊断日志 | 已实现，页面日志为 mock；进程启动诊断写入本地日志 |
| 设置页 | 编辑服务端地址和本机设备名 | 已实现，保存到 mock backend |
| 平台能力页 | 告知 X11/Wayland/平台能力限制 | 已实现基础提示 |
| 关于页 | 展示技术方案和仓库信息 | 已实现，明确无 Tauri/WebView2 |

## 5. 错误提示原则

后续错误提示必须满足：

- 说明失败发生在哪个阶段，例如地址解析、端口连接、WebSocket 握手、注册、认证、配对、权限检测。
- 给出用户可执行的下一步，例如检查服务端地址、端口、防火墙、token、权限或日志。
- 不输出 token、密钥或私钥明文。
- 不把平台限制伪装成普通失败。例如 Wayland 全局键鼠受限应明确说明。
- Windows 双击后闪退类问题需要提示用户运行 `glide.exe --smoke`、`glide.exe --diagnostics`，或查看 `%APPDATA%\Glide\logs\glide-gui.log`。

当前 GUI 只对空服务端地址有 mock 错误，真实错误提示仍需 daemon IPC 接入。

## 6. 平台限制提示原则

Linux：

- X11 是第一阶段优先目标。
- Wayland 下必须显示全局键鼠能力受限，不承诺完整控制。

Windows：

- 不提示安装 WebView2。
- 后续如果缺少运行库或权限，安装/启动阶段必须说明依赖和修复方式。

macOS：

- 规划中，需要辅助功能、输入监控、剪贴板和局域网权限引导。

## 7. 安装体验原则

Windows：

- 当前是 portable zip。
- portable zip 中 `glide.exe` 和 `glide-gui.exe` 都是 Slint GUI 入口；旧 `Glide_*_x64-setup.exe` / `.msi` 不属于当前 GUI 方案。
- 后续 installer 需要保证干净 Windows 10/11 x64 可直接安装运行。
- 不能依赖用户手动安装 WebView2、Node、Rust、Python 或开发工具链。

Linux：

- 当前提供 `.deb`、`.rpm`、`.AppImage`。
- 包含 GUI、daemon、CLI、server、desktop entry 和图标。
- X11 依赖和 Wayland 限制应在文档中清楚说明。

## 8. 后续 UI 设计待办

- 将 mock backend 替换为 daemon IPC 后，补真实错误状态和 loading 状态。
- 增加真实配对码/设备指纹确认界面。
- 增加 token/认证配置和脱敏展示。
- 增加文件传输入口、接收确认、保存目录、进度、取消和失败重试。
- 增加诊断导出入口。
- 增加系统托盘和后台运行交互。
- 增加权限检查向导，特别是 Windows 输入权限、Linux Wayland 限制和 macOS 权限。
- 抽象 `Dialog`、`EmptyState`、`LogView` 等复用组件。
