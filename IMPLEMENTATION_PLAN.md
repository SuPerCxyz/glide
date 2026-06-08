# Glide 功能实现计划

## 已完成
- ✅ 显示器管理器 - 基础检测和显示
- ✅ 客户端管理界面 - 设备列表增强、信任/取消信任/移除按钮、设备统计
- ✅ 多媒体键支持 - 添加 MediaKey 事件类型，Linux xdotool 和 Windows API 实现
- ✅ 鼠标速度自定义 - input_config.rs 数据结构，速度倍率、滚轮方向、按钮交换
- ✅ 网络安全设置界面 - 设置页增加加密传输开关（TLS 切换按钮）
- ⏳ 系统托盘集成 - 跳过（环境库冲突）

## 待实现功能（按优先级）

### 阶段 1: 高优先级（核心竞争力）

#### 1.1 系统托盘集成
**目标**: 在系统托盘添加图标，提供快速访问和控制
**实现内容**:
- 托盘图标（使用 `tray-icon` crate）
- 右键菜单：打开/隐藏、启用/禁用共享、退出
- 状态提示：连接状态、活动设备数
- 快捷键支持

**涉及文件**:
- `crates/glide-gui/Cargo.toml` - 添加 `tray-icon` 依赖
- `crates/glide-gui/src/main.rs` - 初始化托盘
- `crates/glide-gui/src/tray.rs` - 新建托盘管理模块

**预计工作量**: 2-3小时

---

#### 1.2 富文本/图片剪贴板支持
**目标**: 支持富文本（RTF/HTML）和图片的跨设备同步
**实现内容**:
- 扩展现有剪贴板模型支持多种 MIME 类型
- Linux: 使用 `xclip`/`xsel` 支持图片
- Windows: 使用 Win32 API 支持富文本和图片
- GUI 显示剪贴板内容类型

**涉及文件**:
- `crates/glide-core/src/clipboard.rs` - 扩展 ClipboardItem
- `crates/glide-desktop/src/clipboard_adapter.rs` - 添加富文本/图片处理
- `crates/glide-desktop/src/linux_backends.rs` - Linux 实现
- `crates/glide-desktop/src/windows_clipboard.rs` - Windows 实现

**预计工作量**: 4-5小时

---

#### 1.3 客户端管理界面
**目标**: 提供详细的设备管理功能
**实现内容**:
- 设备详情页：连接信息、最后活动、信任状态
- 设备操作：断开连接、移除设备、修改信任状态
- 连接历史：显示最近的连接记录
- 批量操作：全选、批量信任/移除

**涉及文件**:
- `crates/glide-gui/ui/device_detail.slint` - 新建详情页
- `crates/glide-gui/ui/app.slint` - 添加导航
- `crates/glide-gui/src/gui_backend.rs` - 添加管理接口

**预计工作量**: 3-4小时

---

### 阶段 2: 中优先级（用户体验）

#### 2.1 鼠标速度自定义
**目标**: 为每个设备设置不同的鼠标速度和滚轮方向
**实现内容**:
- 设备配置：速度倍率、滚轮方向、灵敏度
- 配置持久化：保存到本地文件
- GUI 设置界面

**涉及文件**:
- `crates/glide-core/src/device_config.rs` - 新建配置结构
- `crates/glide-desktop/src/input_adapter.rs` - 应用速度配置
- `crates/glide-gui/ui/device_settings.slint` - 设置界面

**预计工作量**: 2-3小时

---

#### 2.2 多媒体键支持
**目标**: 支持播放/暂停、音量等多媒体按键
**实现内容**:
- 扩展 InputEventKind 支持多媒体键
- Linux: xdotool 多媒体键映射
- Windows: 虚拟键码映射

**涉及文件**:
- `crates/glide-core/src/input_event.rs` - 添加多媒体键类型
- `crates/glide-desktop/src/linux_backends/linux_input.rs`
- `crates/glide-desktop/src/windows_input.rs`

**预计工作量**: 2小时

---

#### 2.3 远程登录支持
**目标**: Windows 冷启动后支持远程登录
**实现内容**:
- 开机自启动服务
- 登录界面注入
- UAC 弹窗支持

**涉及文件**:
- `crates/glide-daemon/` - 服务化改造
- Windows 特定实现

**预计工作量**: 4-5小时

---

#### 2.4 网络安全设置界面
**目标**: 可视化配置加密和认证
**实现内容**:
- TLS/SSL 配置界面
- 设备认证证书管理
- 连接加密状态显示

**涉及文件**:
- `crates/glide-gui/ui/security_settings.slint` - 新建
- `crates/glide-gui/src/gui_backend.rs` - 安全配置接口

**预计工作量**: 3小时

---

### 阶段 3: 低优先级（锦上添花）

#### 3.1 屏幕保护同步
**目标**: 主设备激活屏保时同步到所有设备
**实现内容**:
- 屏保状态检测
- 事件广播机制
- 各平台屏保控制 API

**预计工作量**: 3小时

---

#### 3.2 同步锁定
**目标**: 锁定主设备时同时锁定所有从设备
**实现内容**:
- 锁定事件广播
- 各平台锁屏 API
- 快捷键支持

**预计工作量**: 2小时

---

#### 3.3 显示器变暗
**目标**: 非活动显示器自动变暗
**实现内容**:
- 亮度控制 API
- 活动设备追踪
- 渐变动画效果

**预计工作量**: 2-3小时

---

#### 3.4 位置配置文件
**目标**: 笔记本在不同位置自动切换布局
**实现内容**:
- 网络环境检测（WiFi SSID、IP 范围）
- 配置切换逻辑
- 配置文件管理界面

**预计工作量**: 3-4小时

---

## 执行顺序

1. **系统托盘集成** (2-3h) - 基础交互体验
2. **富文本/图片剪贴板** (4-5h) - 核心功能增强
3. **客户端管理界面** (3-4h) - 管理体验优化
4. **鼠标速度自定义** (2-3h) - 个性化配置
5. **多媒体键支持** (2h) - 功能完善
6. **网络安全设置** (3h) - 安全增强
7. **远程登录支持** (4-5h) - Windows 特性
8. **屏幕保护同步** (3h) - 便利功能
9. **同步锁定** (2h) - 便利功能
10. **显示器变暗** (2-3h) - 视觉反馈
11. **位置配置文件** (3-4h) - 笔记本优化

**总预计工作量**: 30-38 小时
