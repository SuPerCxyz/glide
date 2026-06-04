# Glide 测试计划

> 最后更新：2026-06-04
> 总测试：72 通过 / 0 失败

---

## 一、测试矩阵

| 场景 | 平台 | 测试方式 | 状态 |
|------|------|----------|------|
| 服务端健康检查 | Linux | 自动化 | ✅ 通过 |
| 设备注册（正确 token） | Linux | 自动化 | ✅ 通过 |
| 设备注册（错误 token） | Linux | 自动化 | ✅ 通过 |
| 设备注册（无 token） | Linux | 自动化 | ✅ 通过 |
| 纯文本同步 A→B | Linux | 自动化 | ✅ 通过 |
| 中文文本同步 | Linux | 自动化 | ✅ 通过 |
| Emoji 同步 | Linux | 自动化 | ✅ 通过 |
| 空文本同步 | Linux | 自动化 | ✅ 通过 |
| 大文本同步 (100KB) | Linux | 自动化 | ✅ 通过 |
| 多行文本同步 | Linux | 自动化 | ✅ 通过 |
| 特殊字符同步 | Linux | 自动化 | ✅ 通过 |
| 双向同步 B→A | Linux | 自动化 | ✅ 通过 |
| 循环抑制 | Linux | 自动化 | ✅ 通过 |
| 多设备广播 | Linux | 自动化 | ✅ 通过 |
| 键盘事件序列化 | 跨平台 | 单元测试 | ✅ 通过 |
| 鼠标移动事件 | 跨平台 | 单元测试 | ✅ 通过 |
| 鼠标按键事件 | 跨平台 | 单元测试 | ✅ 通过 |
| 鼠标滚轮事件 | 跨平台 | 单元测试 | ✅ 通过 |
| 紧急释放事件 | 跨平台 | 单元测试 | ✅ 通过 |
| 错误端口连接 | Linux | 自动化 | ✅ 通过 |
| 无效 token 验证 | Linux | 自动化 | ✅ 通过 |
| 历史记录分页 | Linux | 自动化 | ✅ 通过 |
| 重连恢复 | Linux | 自动化 | ✅ 通过 |
| Windows 设备注册 | Linux 模拟 | 自动化 | ✅ 通过 |
| Linux→Windows 同步 | Linux 模拟 | 自动化 | ✅ 通过 |
| 网络：IPv4 连接 | Linux | 自动化 | ✅ 通过 |
| 网络：IPv6 监听 | Linux | 自动化 | ✅ 通过 |
| 网络：0.0.0.0 绑定 | Linux | 自动化 | ✅ 通过 |
| 网络：错误端口拒绝 | Linux | 自动化 | ✅ 通过 |
| mDNS/UDP 发现模块 | 跨平台 | 单元测试 | ✅ 通过 (12 tests) |
| 路由选择逻辑 | 跨平台 | 单元测试 | ✅ 通过 (11 tests) |
| 临时 token 逻辑 | 跨平台 | 单元测试 | ✅ 通过 (7 tests) |
| 核心类型序列化 | 跨平台 | 单元测试 | ✅ 通过 (26 tests) |
| LAN 同步引擎 | 跨平台 | 单元测试 | ✅ 通过 (12 tests) |
| 服务端数据库 | Linux | 集成测试 | ✅ 通过 (13 tests) |

---

## 二、已知 Bug 和修复

| Bug | 根因 | 修复 | 测试覆盖 |
|-----|------|------|----------|
| 循环抑制失败 | 服务端 WebSocket 广播未过滤发送者 | ws.rs 添加 device_id 过滤 | ✅ test-e2e.py Phase 3 |

---

## 三、待 Windows VM 验证项

以下测试需要在真实 Windows 环境中执行：

| 测试项 | 脚本 | 验证步骤 |
|--------|------|----------|
| Windows GUI 启动 | `test-windows-connect.ps1` | 运行 `glide.exe`，输入服务端地址，点击连接 |
| Windows 剪贴板同步 | `test-windows-clipboard.ps1` | Notepad 复制 → Linux 接收 |
| Linux → Windows 同步 | `test-windows-connect.ps1` | Linux 复制 → Windows 粘贴 |
| Windows 服务端重连 | 手动 | 重启服务端，观察客户端自动重连 |
| Windows DPI 缩放 | 手动 | 设置 125%/150%，观察 GUI 是否正常 |
| Windows 防火墙 | 手动 | 阻止端口，观察错误提示 |

---

## 四、测试命令

```bash
# 全部单元测试
cargo test --package glide-core --package glide-server --package glide-desktop

# E2E 测试（需要服务端运行）
python3 scripts/test-e2e.py

# 网络测试
bash scripts/test-network.sh

# Windows 连接测试（PowerShell）
.\scripts\test-windows-connect.ps1 -Server http://aicode.soocoo.xyz:8080

# Windows 剪贴板测试（PowerShell）
.\scripts\test-windows-clipboard.ps1 -Server http://aicode.soocoo.xyz:8080
```

---

## 五、测试环境

| 组件 | 方式 | 状态 |
|------|------|------|
| 服务端 | Docker 容器 | ✅ 运行中 |
| Linux CLI | 直接运行 | ✅ 可用 |
| Linux GUI | Xvfb + xclip | ⏭ 需安装 xclip |
| Windows GUI | PowerShell 脚本 | 📝 已生成，待 Windows VM 执行 |
| 网络模拟 | Docker + ss | ✅ 已验证 |
