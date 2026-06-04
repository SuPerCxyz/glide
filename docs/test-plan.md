# Glide 测试计划

> 最后更新：2026-06-04
> 测试总数：124 通过 / 0 失败

---

## 一、测试矩阵

### A. 单元测试 (86 tests)

| 测试 | 数量 | 状态 |
|------|------|------|
| 核心类型序列化/反序列化 | 26 | ✅ |
| 服务端临时 token | 7 | ✅ |
| 路由选择逻辑 | 11 | ✅ |
| mDNS/UDP 发现 | 4 | ✅ |
| 剪贴板适配器 | 3 | ✅ |
| LAN 同步引擎 | 12 | ✅ |
| PeerRegistry | 5 | ✅ |
| 服务端数据库 | 13 | ✅ |
| 延迟追踪器 | 5 | ✅ |

### B. E2E 集成测试 (34 tests)

| 场景 | 状态 |
|------|------|
| 健康检查 | ✅ |
| 设备注册（正确/错误/无 token） | ✅ |
| 纯文本/中文/Emoji/空文本/大文本/多行/特殊字符同步 | ✅ |
| 循环抑制 | ✅ |
| 双向同步 B→A | ✅ |
| 多设备广播 | ✅ |
| 键盘/鼠标/滚轮/紧急释放事件 | ✅ |
| 错误端口连接 | ✅ |
| 历史记录分页 | ✅ |
| 重连恢复 | ✅ |
| Windows 设备注册+同步 | ✅ |

### C. 网络测试 (11 tests)

| 场景 | 状态 |
|------|------|
| 健康检查端点 | ✅ |
| IPv4 连接 | ✅ |
| 错误端口拒绝 | ✅ |
| 0.0.0.0 绑定 | ✅ |
| IPv6 监听 | ✅ |
| 设备注册 | ✅ |
| 错误 token 拒绝 | ✅ |
| API 端点 | ✅ |
| WebSocket 连接/断开 | ✅ |

### D. 网络异常测试 (10 tests)

| 场景 | 状态 |
|------|------|
| 服务端重启恢复 | ✅ |
| 连接超时到错误 IP | ✅ |
| 重连到正常服务器 | ✅ |
| 快速连接/断开 10 次 | ✅ |
| 1MB 大载荷发送 | ✅ |
| IPv4 localhost | ✅ |
| IPv4 0.0.0.0 | ✅ |
| IPv6 localhost | ✅ |
| 端口绑定验证 (IPv4) | ✅ |
| 端口绑定验证 (IPv6) | ✅ |

### E. CLI 剪贴板测试 (7 tests)

| 场景 | 状态 |
|------|------|
| 纯文本同步 | ✅ |
| 中文同步 | ✅ |
| Emoji 同步 | ✅ |
| 空文本同步 | ✅ |
| 多行文本同步 | ✅ |
| 特殊字符同步 | ✅ |
| 大文本 (500KB) 同步 | ✅ |

### F. 键盘/鼠标协议测试 (34 tests)

| 场景 | 状态 |
|------|------|
| 键盘事件编码（A/Ctrl+C/Ctrl+Alt+Del/Shift/Win/F1/Enter/Arrow） | ✅ |
| 鼠标移动（0,0/1920,1080/负增量） | ✅ |
| 鼠标按键（左/右/中/释放） | ✅ |
| 滚轮（上/下/水平） | ✅ |
| 紧急释放 | ✅ |
| 输入路由（LAN Direct/Server Relay） | ✅ |
| 坐标映射（跨分辨率） | ✅ |
| 屏幕边缘检测 | ✅ |
| DPI 缩放（100%/125%/150%） | ✅ |

### G. 认证与重连测试 (7 tests)

| 场景 | 状态 |
|------|------|
| 正确 token 认证 | ✅ |
| 错误 token 拒绝 | ✅ |
| 无 token 拒绝 | ✅ |
| WebSocket 关闭后重连 | ✅ |
| 5 个客户端同时连接 | ✅ |
| 5 客户端间同步 | ✅ |
| 设备注册表验证 | ✅ |

### H. Linux GUI 无头测试 (6 tests)

| 场景 | 状态 |
|------|------|
| Xvfb 启动 | ✅ |
| xclip 写入/读取 | ✅ |
| xclip 中文文本 | ✅ |
| xdotool 鼠标移动 | ✅ |
| xdotool 键盘组合键 | ✅ |
| xdotool 点击 | ✅ |

---

## 二、已修复的 Bug

| Bug | 根因 | 修复 |
|-----|------|------|
| 循环抑制失败 | WebSocket 广播未过滤发送者 | ws.rs 添加 device_id 过滤 |
| WebSocket 客户端 FK 约束失败 | 客户端未注册就发事件 | ws.rs 自动注册设备 |

---

## 三、测试命令

```bash
# 运行全部测试
make test-all

# 单独测试
make test            # E2E 测试
make test-network    # 网络测试
make test-gui        # Linux GUI 无头测试
make test-clipboard  # CLI 剪贴板测试
make test-keyboard   # 键鼠协议测试
make test-auth       # 认证重连测试
make test-network-anomaly  # 网络异常测试

# 单元测试
cargo test --package glide-core --package glide-server --package glide-desktop
```

---

## 四、Windows VM 待验证

| 测试项 | 脚本 |
|--------|------|
| Windows 连接测试 | `scripts/test-windows-connect.ps1` |
| Windows 剪贴板测试 | `scripts/test-windows-clipboard.ps1` |
| Windows GUI 自动化 | `scripts/test-windows-gui.ahk` |

---

## 五、竞品参考

| 产品 | 借鉴功能 | 测试点 |
|------|----------|--------|
| Barrier/Input Leap | 屏幕边缘切换 | 边缘检测、坐标映射 |
| Synergy | 跨屏键鼠 | 组合键、断线释放 |
| KDE Connect | 剪贴板同步 | 循环抑制、离线重连 |
| LocalSend | 局域网传输 | mDNS 发现、大文件 |
| RustDesk | 远程桌面 | WebSocket 协议、重连退避 |
| CopyQ | 剪贴板历史 | 多类型、去重 |

---

## 六、当前限制

1. Windows GUI 无法在 Linux 本地真实执行
2. Wayland GUI 测试需 weston headless（未安装）
3. mDNS 真实网络发现需多机环境
4. 跨屏键鼠真实测试需多显示器

## 七、结论

当前虚拟环境下 **124 个测试全部通过**，0 失败。
