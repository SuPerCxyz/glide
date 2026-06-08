# 统一 UI/UX 重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 重写服务端 Web 管理页 (`static/index.html`) 和 GUI Slint 页面 (`app.slint`)，实现统一的设计系统（Light/Dark 双主题、等宽字体、六态状态色、统一组件库）。

**Architecture:** 两个独立子系统并行重写：(1) Web 管理页是单文件 HTML/CSS/JS，`include_str!` 嵌入 Rust 二进制；(2) GUI Slint 页面通过 `app.slint` 定义，编译时生成 Rust 代码，`main.rs` 绑定回调。Slint 重写必须保持现有 properties/callbacks 向后兼容，以确保 `main.rs` 和 smoke 测试不变。

**Tech Stack:** HTML/CSS/JS (Web), Slint 1.x (GUI), Rust (backend binding)

---

## Phase 1: 服务端 Web 管理页重写

**Files:**
- Modify: `crates/glide-server/static/index.html` (完整重写)

### Task 1: 重写 CSS 变量系统 + 基础布局

**Files:**
- Modify: `crates/glide-server/static/index.html`

- [ ] **Step 1: 重写 HTML 骨架**

完整替换 `index.html`。新结构：

```html
<!DOCTYPE html>
<html lang="zh-CN" data-theme="light">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Glide Server Admin</title>
    <style>
        /* ===== CSS Variables ===== */
        :root, [data-theme="light"] {
            --page-bg: #ffffff;
            --page-surface: #f8fafc;
            --page-surface-alt: #f1f5f9;
            --text-primary: #1e293b;
            --text-secondary: #64748b;
            --text-tertiary: #94a3b8;
            --text-inverse: #ffffff;
            --border-base: #e2e8f0;
            --border-subtle: #f1f5f9;
            --color-brand: #2563eb;
            --color-brand-hover: #1d4ed8;
            --btn-primary: #2563eb;
            --btn-primary-text: #ffffff;
            --btn-secondary: #f8fafc;
            --btn-secondary-text: #475569;
            --btn-danger: #dc2626;
            --btn-danger-text: #ffffff;
            --sidebar-bg: #f8fafc;
            --sidebar-text: #475569;
            --sidebar-active: #eff6ff;
            --sidebar-active-text: #2563eb;
            --input-bg: #ffffff;
            --input-border: #cbd5e1;
            --input-focus: #2563eb;
            --focus-ring: rgba(37,99,235,0.4);
            --log-bg: #0f172a;
            --log-text: #cbd5e1;
            --log-info: #38bdf8;
            --log-warn: #fbbf24;
            --log-error: #f87171;
            --info-bg: #eff6ff;
            --info-border: #2563eb;
            --info-text: #1e40af;
            --warning-bg: #fffbeb;
            --warning-border: #d97706;
            --warning-text: #92400e;
            --success-bg: #dcfce7;
            --success-border: #16a34a;
            --success-text: #166534;
            --danger-bg: #fee2e2;
            --danger-border: #dc2626;
            --danger-text: #991b1b;
            --modal-scrim: rgba(0,0,0,0.4);
            --shadow-md: 0 4px 12px rgba(0,0,0,0.15);
            --status-online: #16a34a;
            --status-online-bg: #dcfce7;
            --status-offline: #dc2626;
            --status-offline-bg: #fee2e2;
            --status-connecting: #2563eb;
            --status-connecting-bg: #dbeafe;
            --status-error: #dc2626;
            --status-error-bg: #fee2e2;
            --status-limited: #d97706;
            --status-limited-bg: #fef3c7;
            --status-unpaired: #64748b;
            --status-unpaired-bg: #f1f5f9;
        }

        [data-theme="dark"] {
            --page-bg: #0f172a;
            --page-surface: #1e293b;
            --page-surface-alt: #334155;
            --text-primary: #f1f5f9;
            --text-secondary: #94a3b8;
            --text-tertiary: #64748b;
            --text-inverse: #0f172a;
            --border-base: #334155;
            --border-subtle: #1e293b;
            --color-brand: #3b82f6;
            --color-brand-hover: #60a5fa;
            --btn-primary: #3b82f6;
            --btn-primary-text: #0f172a;
            --btn-secondary: #334155;
            --btn-secondary-text: #cbd5e1;
            --btn-danger: #ef4444;
            --btn-danger-text: #0f172a;
            --sidebar-bg: #0c1222;
            --sidebar-text: #cbd5e1;
            --sidebar-active: #1e3a5f;
            --sidebar-active-text: #3b82f6;
            --input-bg: #0f172a;
            --input-border: #475569;
            --input-focus: #3b82f6;
            --focus-ring: rgba(59,130,246,0.4);
            --log-bg: #020617;
            --log-text: #e2e8f0;
            --log-info: #38bdf8;
            --log-warn: #fbbf24;
            --log-error: #f87171;
            --info-bg: #1e3a5f;
            --info-border: #3b82f6;
            --info-text: #93c5fd;
            --warning-bg: #451a03;
            --warning-border: #f59e0b;
            --warning-text: #fcd34d;
            --success-bg: #14532d;
            --success-border: #22c55e;
            --success-text: #86efac;
            --danger-bg: #451a1a;
            --danger-border: #ef4444;
            --danger-text: #fca5a5;
            --modal-scrim: rgba(0,0,0,0.6);
            --shadow-md: 0 4px 12px rgba(0,0,0,0.3);
            --status-online: #22c55e;
            --status-online-bg: #14532d;
            --status-offline: #ef4444;
            --status-offline-bg: #451a1a;
            --status-connecting: #3b82f6;
            --status-connecting-bg: #1e3a5f;
            --status-error: #ef4444;
            --status-error-bg: #451a1a;
            --status-limited: #f59e0b;
            --status-limited-bg: #451a03;
            --status-unpaired: #94a3b8;
            --status-unpaired-bg: #1e293b;
        }

        /* ===== Reset & Base ===== */
        *, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: "JetBrains Mono", "Fira Code", "Consolas", "Monaco", monospace;
            font-size: 14px;
            line-height: 1.5;
            background: var(--page-bg);
            color: var(--text-primary);
            min-height: 100vh;
            transition: background 200ms ease-out, color 200ms ease-out;
        }
        button { cursor: pointer; font-family: inherit; }
        a { color: var(--color-brand); text-decoration: none; }
        a:hover { text-decoration: underline; }

        /* ===== Layout ===== */
        .app { display: flex; min-height: 100vh; }
        .sidebar { width: 180px; background: var(--sidebar-bg); border-right: 1px solid var(--border-base); display: flex; flex-direction: column; padding: 16px 0; transition: background 200ms ease-out; }
        .sidebar .logo { font-size: 22px; font-weight: 800; padding: 0 16px 16px; color: var(--color-brand); letter-spacing: -0.5px; }
        .sidebar .logo .version { font-size: 11px; font-weight: 400; color: var(--text-tertiary); margin-left: 4px; }
        .nav-item {
            display: block; width: 100%; padding: 10px 16px; font-size: 13px; font-weight: 400;
            color: var(--sidebar-text); background: none; border: none; text-align: left;
            transition: background 150ms ease-out, color 150ms ease-out;
        }
        .nav-item:hover { background: var(--page-surface-alt); }
        .nav-item.active { background: var(--sidebar-active); color: var(--sidebar-active-text); font-weight: 600; }

        .main { flex: 1; display: flex; flex-direction: column; min-width: 0; }

        /* ===== TopBar ===== */
        .topbar {
            height: 52px; border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center;
            padding: 0 24px; background: var(--page-surface); gap: 12px; transition: background 200ms ease-out;
        }
        .topbar-title { font-size: 20px; font-weight: 700; flex: 1; }
        .topbar-actions { display: flex; align-items: center; gap: 8px; }

        /* ===== Status Badge ===== */
        .status-badge {
            display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px;
            border-radius: 4px; font-size: 11px; font-weight: 500;
            color: var(--badge-color, var(--text-tertiary));
            background: var(--badge-bg, var(--page-surface-alt));
        }
        .status-dot {
            width: 7px; height: 7px; border-radius: 50%; background: currentColor;
        }
        .status-dot.pulse { animation: pulse 2s infinite; }
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }

        /* ===== Buttons ===== */
        .btn {
            display: inline-flex; align-items: center; gap: 6px; padding: 6px 14px;
            border-radius: 4px; font-size: 12px; font-weight: 500; border: 1px solid var(--border-base);
            background: var(--btn-secondary); color: var(--btn-secondary-text);
            transition: background 150ms ease-out;
        }
        .btn:hover { filter: brightness(0.95); }
        .btn:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
        .btn.primary { background: var(--btn-primary); color: var(--btn-primary-text); border-color: var(--btn-primary); }
        .btn.danger { background: var(--btn-danger); color: var(--btn-danger-text); border-color: var(--btn-danger); }

        /* ===== Content ===== */
        .content { padding: 24px; flex: 1; overflow-y: auto; }
        .page { display: none; }
        .page.active { display: block; }

        /* ===== Stat Cards ===== */
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin-bottom: 24px; }
        .stat-card {
            background: var(--page-surface); border: 1px solid var(--border-base); border-radius: 6px;
            padding: 20px; transition: background 200ms ease-out, border-color 200ms ease-out;
        }
        .stat-card .label { font-size: 12px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; }
        .stat-card .value { font-size: 28px; font-weight: 700; margin-top: 4px; }
        .stat-card .value.green { color: var(--status-online); }
        .stat-card .value.blue { color: var(--color-brand); }
        .stat-card .value.yellow { color: var(--status-limited); }
        .stat-card .value.red { color: var(--status-offline); }

        /* ===== Section / Table ===== */
        .section {
            background: var(--page-surface); border: 1px solid var(--border-base); border-radius: 6px;
            margin-bottom: 24px; overflow: hidden; transition: background 200ms ease-out, border-color 200ms ease-out;
        }
        .section-header { padding: 14px 20px; border-bottom: 1px solid var(--border-base); display: flex; align-items: center; justify-content: space-between; }
        .section-header h2 { font-size: 15px; font-weight: 600; }
        .section-header .badge { font-size: 11px; background: var(--page-surface-alt); padding: 2px 8px; border-radius: 10px; color: var(--text-secondary); }
        table { width: 100%; border-collapse: collapse; }
        th, td { padding: 10px 16px; text-align: left; border-bottom: 1px solid var(--border-base); font-size: 13px; }
        th { color: var(--text-secondary); font-weight: 500; background: var(--page-bg); }
        td { color: var(--text-primary); }
        tr:last-child td { border-bottom: none; }
        .mono { font-family: inherit; font-size: 12px; }

        /* ===== AlertBox ===== */
        .alert {
            display: flex; gap: 12px; padding: 14px 16px; border-radius: 6px; margin-bottom: 16px;
            border-left: 3px solid var(--alert-border); background: var(--alert-bg);
        }
        .alert .alert-icon { font-size: 16px; line-height: 1.4; flex-shrink: 0; }
        .alert .alert-body { flex: 1; }
        .alert .alert-title { font-size: 13px; font-weight: 600; color: var(--alert-text); margin-bottom: 2px; }
        .alert .alert-text { font-size: 12px; color: var(--text-secondary); }
        .alert .alert-action { font-size: 12px; color: var(--color-brand); margin-top: 6px; display: inline-block; }
        .alert.info { --alert-bg: var(--info-bg); --alert-border: var(--info-border); --alert-text: var(--info-text); }
        .alert.warning { --alert-bg: var(--warning-bg); --alert-border: var(--warning-border); --alert-text: var(--warning-text); }
        .alert.success { --alert-bg: var(--success-bg); --alert-border: var(--success-border); --alert-text: var(--success-text); }
        .alert.danger { --alert-bg: var(--danger-bg); --alert-border: var(--danger-border); --alert-text: var(--danger-text); }

        /* ===== Empty State ===== */
        .empty-state { text-align: center; padding: 40px 20px; color: var(--text-secondary); }
        .empty-state .icon { font-size: 32px; margin-bottom: 8px; opacity: 0.5; }
        .empty-state .title { font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 4px; }
        .empty-state .desc { font-size: 12px; }

        /* ===== Modal ===== */
        .modal-overlay {
            display: none; position: fixed; inset: 0; background: var(--modal-scrim);
            z-index: 1000; align-items: center; justify-content: center;
        }
        .modal-overlay.active { display: flex; }
        .modal {
            background: var(--page-surface); border: 1px solid var(--border-base); border-radius: 8px;
            padding: 24px; min-width: 360px; max-width: 480px; box-shadow: var(--shadow-md);
        }
        .modal h3 { margin-bottom: 16px; font-size: 16px; font-weight: 600; }
        .form-group { margin-bottom: 12px; }
        .form-group label { display: block; font-size: 12px; color: var(--text-secondary); margin-bottom: 4px; }
        .form-group input {
            width: 100%; padding: 8px 12px; background: var(--input-bg); border: 1px solid var(--input-border);
            border-radius: 4px; color: var(--text-primary); font-size: 13px; font-family: inherit;
        }
        .form-group input:focus { outline: none; border-color: var(--input-focus); box-shadow: 0 0 0 2px var(--focus-ring); }
        .modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
        .token-result {
            background: var(--input-bg); border: 1px solid var(--success-border); border-radius: 4px;
            padding: 12px; font-family: inherit; font-size: 13px; color: var(--status-online);
            margin-top: 12px; word-break: break-all; white-space: pre-wrap;
        }

        /* ===== Log View ===== */
        .log-view {
            background: var(--log-bg); border-radius: 6px; overflow: hidden;
        }
        .log-toolbar { display: flex; align-items: center; gap: 8px; padding: 8px 12px; border-bottom: 1px solid #1e293b; }
        .log-toolbar input {
            flex: 1; padding: 4px 8px; background: #1e293b; border: 1px solid #334155; border-radius: 4px;
            color: var(--log-text); font-size: 12px; font-family: inherit;
        }
        .log-toolbar input:focus { outline: none; border-color: #3b82f6; }
        .log-toolbar .btn { background: #1e293b; color: #cbd5e1; border-color: #334155; padding: 4px 10px; }
        .log-toolbar .btn:hover { background: #334155; }
        .log-content {
            padding: 12px; max-height: 500px; overflow: auto;
        }
        .log-content pre {
            font-family: inherit; font-size: 12px; line-height: 1.6; color: var(--log-text);
            white-space: pre; margin: 0;
        }
        .log-content pre .info { color: var(--log-info); }
        .log-content pre .warn { color: var(--log-warn); }
        .log-content pre .error { color: var(--log-error); }

        /* ===== Theme Toggle ===== */
        .theme-toggle {
            background: none; border: 1px solid var(--border-base); border-radius: 4px;
            padding: 6px 10px; color: var(--text-secondary); font-size: 16px; line-height: 1;
            transition: background 150ms ease-out, color 150ms ease-out;
        }
        .theme-toggle:hover { background: var(--page-surface-alt); }

        /* ===== Footer ===== */
        .footer { text-align: center; padding: 16px; color: var(--text-tertiary); font-size: 12px; border-top: 1px solid var(--border-subtle); }

        /* ===== Responsive ===== */
        @media (max-width: 768px) {
            .sidebar { width: 160px; }
            .content { padding: 16px; }
            .stats { grid-template-columns: repeat(2, 1fr); }
        }
    </style>
</head>
<body>
    <div class="app">
        <nav class="sidebar">
            <div class="logo">Glide<span class="version">v0.1.0</span></div>
            <button class="nav-item active" data-page="overview">总览</button>
            <button class="nav-item" data-page="devices">设备</button>
            <button class="nav-item" data-page="connection">连接状态</button>
            <button class="nav-item" data-page="pairing">配对/信任</button>
            <button class="nav-item" data-page="clipboard">剪贴板</button>
            <button class="nav-item" data-page="files">文件传输</button>
            <button class="nav-item" data-page="logs">日志诊断</button>
            <button class="nav-item" data-page="settings">设置</button>
        </nav>
        <div class="main">
            <div class="topbar">
                <div class="topbar-title" id="pageTitle">总览</div>
                <div class="topbar-actions">
                    <div class="status-badge" id="topbarStatus" style="--badge-color: var(--status-online); --badge-bg: var(--status-online-bg);">
                        <span class="status-dot pulse"></span>
                        <span id="topbarStatusText">运行中</span>
                    </div>
                    <button class="theme-toggle" id="themeToggle" title="切换主题" aria-label="切换浅色/深色主题">◑</button>
                    <span id="loginLabel" style="font-size:12px;color:var(--text-secondary)">未登录</span>
                    <button class="btn" id="loginBtn" onclick="showLogin()">登录</button>
                    <button class="btn" id="logoutBtn" onclick="doLogout()" style="display:none">退出</button>
                </div>
            </div>
            <div class="content">
                <!-- Pages rendered by JS -->
            </div>
            <div class="footer">Glide Server Admin • LAN-first clipboard sync</div>
        </div>
    </div>
    <!-- Modals -->
    <div class="modal-overlay" id="loginModal">
        <div class="modal">
            <h3>登录</h3>
            <div class="form-group">
                <label for="loginUser">用户名</label>
                <input type="text" id="loginUser" placeholder="admin" value="admin" autocomplete="username">
            </div>
            <div class="form-group">
                <label for="loginPass">密码</label>
                <input type="password" id="loginPass" placeholder="密码" autocomplete="current-password">
            </div>
            <div id="loginError" style="color:var(--status-offline);font-size:12px;margin-top:8px;display:none"></div>
            <div class="modal-actions">
                <button class="btn" onclick="closeModal('loginModal')">取消</button>
                <button class="btn primary" onclick="doLogin()">登录</button>
            </div>
        </div>
    </div>
    <div class="modal-overlay" id="tokenModal">
        <div class="modal">
            <h3>创建临时令牌</h3>
            <div class="form-group">
                <label for="tokenTTL">有效期（秒）</label>
                <input type="number" id="tokenTTL" value="3600" min="60">
            </div>
            <div class="form-group">
                <label for="tokenMaxUses">最大使用次数</label>
                <input type="number" id="tokenMaxUses" value="10" min="1">
            </div>
            <div class="form-group">
                <label for="tokenMaxSize">最大文件大小（字节）</label>
                <input type="number" id="tokenMaxSize" value="10485760" min="0">
            </div>
            <div id="tokenResult" class="token-result" style="display:none"></div>
            <div id="tokenError" style="color:var(--status-offline);font-size:12px;margin-top:8px;display:none"></div>
            <div class="modal-actions">
                <button class="btn" onclick="closeModal('tokenModal')">关闭</button>
                <button class="btn primary" id="tokenSubmitBtn" onclick="doCreateToken()">创建</button>
            </div>
        </div>
    </div>
</body>
</html>
```

保留 `</body>` 和 `</html>` 闭合标签，JavaScript 将在下一步写入。

- [ ] **Step 2: 写入 JavaScript 逻辑**

在 `</body>` 之前插入 `<script>` 块：

```javascript
<script>
    const API = window.location.origin;
    let startTime = Date.now();
    let authToken = null;
    let currentPage = 'overview';

    // ===== Theme =====
    const savedTheme = localStorage.getItem('glide-theme') || 'light';
    document.documentElement.setAttribute('data-theme', savedTheme);
    updateThemeIcon(savedTheme);

    document.getElementById('themeToggle').addEventListener('click', () => {
        const current = document.documentElement.getAttribute('data-theme');
        const next = current === 'dark' ? 'light' : 'dark';
        document.documentElement.setAttribute('data-theme', next);
        localStorage.setItem('glide-theme', next);
        updateThemeIcon(next);
    });

    function updateThemeIcon(theme) {
        document.getElementById('themeToggle').textContent = theme === 'dark' ? '☀' : '☾';
        document.getElementById('themeToggle').title = theme === 'dark' ? '切换浅色主题' : '切换深色主题';
    }

    // ===== Navigation =====
    const pageTitles = {
        overview: '总览', devices: '设备', connection: '连接状态',
        pairing: '配对/信任', clipboard: '剪贴板历史', files: '文件传输',
        logs: '日志诊断', settings: '设置'
    };

    document.querySelectorAll('.nav-item').forEach(btn => {
        btn.addEventListener('click', () => {
            const page = btn.dataset.page;
            navigateTo(page);
        });
    });

    function navigateTo(page) {
        currentPage = page;
        document.querySelectorAll('.nav-item').forEach(b => b.classList.toggle('active', b.dataset.page === page));
        document.getElementById('pageTitle').textContent = pageTitles[page] || page;
        document.querySelector('.content').innerHTML = renderPage(page);
        if (page === 'logs') initLogHighlight();
    }

    // ===== Page Rendering =====
    function renderPage(page) {
        switch (page) {
            case 'overview': return renderOverview();
            case 'devices': return renderDevices();
            case 'connection': return renderConnection();
            case 'pairing': return renderPairing();
            case 'clipboard': return renderClipboard();
            case 'files': return renderFiles();
            case 'logs': return renderLogs();
            case 'settings': return renderSettings();
            default: return '<div class="empty-state"><div class="title">页面不存在</div></div>';
        }
    }

    function renderOverview() {
        return `
            <div class="stats">
                <div class="stat-card">
                    <div class="label">服务状态</div>
                    <div class="value green" id="statStatus">运行中</div>
                </div>
                <div class="stat-card">
                    <div class="label">已注册设备</div>
                    <div class="value blue" id="statDevices">-</div>
                </div>
                <div class="stat-card">
                    <div class="label">剪贴板项目</div>
                    <div class="value yellow" id="statClipboard">-</div>
                </div>
                <div class="stat-card">
                    <div class="label">运行时间</div>
                    <div class="value" id="statUptime">-</div>
                </div>
            </div>
            <div class="section">
                <div class="section-header">
                    <h2>最近活动</h2>
                    <span class="badge" id="deviceBadge">加载中...</span>
                </div>
                <div id="recentDevices"><table>
                    <thead><tr><th>设备 ID</th><th>名称</th><th>平台</th><th>状态</th><th>最后在线</th></tr></thead>
                    <tbody id="deviceList"><tr><td colspan="5" class="empty">加载中...</td></tr></tbody>
                </table></div>
            </div>
            <div class="section">
                <div class="section-header">
                    <h2>剪贴板历史</h2>
                    <span class="badge" id="historyBadge">0 条</span>
                </div>
                <div><table>
                    <thead><tr><th>时间</th><th>来源</th><th>类型</th><th>大小</th><th>预览</th></tr></thead>
                    <tbody id="historyList"><tr><td colspan="5" class="empty">暂无剪贴板记录</td></tr></tbody>
                </table></div>
            </div>
        `;
    }

    function renderDevices() {
        return `
            <div class="alert info">
                <span class="alert-icon">ℹ</span>
                <div class="alert-body">
                    <div class="alert-title">设备管理</div>
                    <div class="alert-text">点击设备可查看详细信息，移除信任将阻止该设备同步数据。</div>
                </div>
            </div>
            <div class="section">
                <div class="section-header">
                    <h2>已注册设备</h2>
                    <span class="badge" id="deviceCountBadge">0 台</span>
                </div>
                <table>
                    <thead><tr><th>设备 ID</th><th>名称</th><th>平台</th><th>状态</th><th>最后在线</th><th>操作</th></tr></thead>
                    <tbody id="fullDeviceList"><tr><td colspan="6" class="empty">加载中...</td></tr></tbody>
                </table>
            </div>
        `;
    }

    function renderConnection() {
        return `
            <div class="section">
                <div class="section-header"><h2>连接拓扑</h2></div>
                <div style="padding:20px">
                    <div class="alert info">
                        <span class="alert-icon">ℹ</span>
                        <div class="alert-body">
                            <div class="alert-title">LAN 优先，Server 回退</div>
                            <div class="alert-text">同一二层网络内设备自动发现直连；跨网段时通过 glide-server 中继同步。</div>
                        </div>
                    </div>
                    <table>
                        <thead><tr><th>指标</th><th>值</th></tr></thead>
                        <tbody>
                            <tr><td>服务状态</td><td><span class="status-badge" style="--badge-color:var(--status-online);--badge-bg:var(--status-online-bg)"><span class="status-dot"></span>运行中</span></td></tr>
                            <tr><td>WebSocket 同步</td><td id="wsStatus">检测中...</td></tr>
                            <tr><td>当前连接数</td><td id="activeConnections">-</td></tr>
                            <tr><td>LAN 发现</td><td id="lanDiscovery">-</td></tr>
                            <tr><td>Server 回退</td><td id="serverFallback">-</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
            <div class="section">
                <div class="section-header"><h2>防火墙与端口</h2></div>
                <div style="padding:20px">
                    <div class="alert warning">
                        <span class="alert-icon">⚠</span>
                        <div class="alert-body">
                            <div class="alert-title">端口要求</div>
                            <div class="alert-text">
                                确保以下端口未被防火墙阻断：<br>
                                • <strong>8080</strong> — HTTP/WebSocket (glide-server)<br>
                                • <strong>5353</strong> — mDNS 自动发现 (UDP)<br>
                                如果设备无法发现，请检查局域网防火墙是否阻断 UDP 组播或 mDNS。
                            </div>
                            <a class="alert-action" href="https://github.com/SuPerCxyz/glide#network-requirements" target="_blank">查看网络配置文档 →</a>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    function renderPairing() {
        return `
            <div class="alert info">
                <span class="alert-icon">ℹ</span>
                <div class="alert-body">
                    <div class="alert-title">设备配对</div>
                    <div class="alert-text">配对后的设备将加入受信列表，可以同步剪贴板、文件和键鼠输入。撤销信任将立即阻断该设备的数据同步。</div>
                </div>
            </div>
            <div class="section">
                <div class="section-header">
                    <h2>已信任设备</h2>
                    <span class="badge" id="trustedBadge">0 台</span>
                </div>
                <table>
                    <thead><tr><th>设备 ID</th><th>名称</th><th>平台</th><th>状态</th><th>信任时间</th><th>操作</th></tr></thead>
                    <tbody id="trustedList"><tr><td colspan="6" class="empty">加载中...</td></tr></tbody>
                </table>
            </div>
        `;
    }

    function renderClipboard() {
        return `
            <div class="section">
                <div class="section-header">
                    <h2>剪贴板历史</h2>
                    <span class="badge" id="clipBadge2">0 条</span>
                </div>
                <table>
                    <thead><tr><th>时间</th><th>来源设备</th><th>类型</th><th>大小</th><th>预览</th></tr></thead>
                    <tbody id="clipboardFullList"><tr><td colspan="5" class="empty">加载中...</td></tr></tbody>
                </table>
            </div>
        `;
    }

    function renderFiles() {
        return `
            <div class="alert warning">
                <span class="alert-icon">⚠</span>
                <div class="alert-body">
                    <div class="alert-title">文件传输待接入</div>
                    <div class="alert-text">CLI 已支持单文件上传/下载 (`copy --file` / `paste --output`)。Web 管理端文件传输界面仍在规划中，后续将支持多文件、目录、进度条和接收确认。</div>
                    <a class="alert-action" href="https://github.com/SuPerCxyz/glide" target="_blank">查看 CLI 用法 →</a>
                </div>
            </div>
            <div class="section">
                <div class="section-header"><h2>最近传输</h2></div>
                <div class="empty-state">
                    <div class="icon">📁</div>
                    <div class="title">暂无传输记录</div>
                    <div class="desc">文件传输功能仍在规划中</div>
                </div>
            </div>
        `;
    }

    function renderLogs() {
        return `
            <div class="section">
                <div class="section-header">
                    <h2>运行日志</h2>
                    <button class="btn" onclick="copyLogs()" id="copyLogBtn">复制日志</button>
                </div>
                <div class="log-view">
                    <div class="log-toolbar">
                        <input type="text" id="logFilter" placeholder="筛选日志 (INFO/WARN/ERROR)..." oninput="filterLogs()">
                    </div>
                    <div class="log-content" id="logContent">
                        <pre>日志功能待 daemon 接入。当前服务端日志可通过 Docker 日志或 glide-server 进程日志查看。

使用 docker logs &lt;container&gt; 查看容器日志
使用 journalctl -u glide-server 查看 systemd 日志</pre>
                    </div>
                </div>
            </div>
            <div class="section">
                <div class="section-header"><h2>环境变量参考</h2></div>
                <div style="padding:16px 20px">
                    <table>
                        <tr><td class="mono">GLIDE_USERNAME</td><td>admin</td></tr>
                        <tr><td class="mono">GLIDE_DATA_DIR</td><td>/data</td></tr>
                        <tr><td class="mono">GLIDE_MAX_STORAGE_BYTES</td><td>1 GB</td></tr>
                        <tr><td class="mono">GLIDE_RETENTION_DAYS</td><td>30 天</td></tr>
                        <tr><td class="mono">GLIDE_INPUT_RELAY_ENABLED</td><td>false</td></tr>
                    </table>
                </div>
            </div>
        `;
    }

    function renderSettings() {
        return `
            <div class="section">
                <div class="section-header"><h2>主题</h2></div>
                <div style="padding:16px 20px;display:flex;gap:12px;align-items:center">
                    <span style="font-size:13px;color:var(--text-secondary)">当前主题：</span>
                    <span id="settingsTheme" style="font-size:13px;font-weight:600"></span>
                    <button class="btn" onclick="document.getElementById('themeToggle').click()">切换主题</button>
                </div>
            </div>
            <div class="section">
                <div class="section-header"><h2>认证</h2></div>
                <div style="padding:16px 20px">
                    <div class="form-group">
                        <label>当前用户</label>
                        <div style="font-size:13px;font-weight:600" id="settingsUser">admin</div>
                    </div>
                    <div style="margin-top:12px">
                        <button class="btn" onclick="showLogin()" ${authToken ? '' : 'style="display:none"'}>修改密码</button>
                    </div>
                </div>
            </div>
            <div class="section">
                <div class="section-header"><h2>存储</h2></div>
                <div style="padding:16px 20px">
                    <table>
                        <tr><td class="mono">数据目录</td><td>/data</td></tr>
                        <tr><td class="mono">最大存储</td><td>1 GB</td></tr>
                        <tr><td class="mono">保留天数</td><td>30 天</td></tr>
                    </table>
                </div>
            </div>
        `;
    }

    // ===== Log Functions =====
    function initLogHighlight() {
        const pre = document.querySelector('#logContent pre');
        if (!pre) return;
        const html = pre.textContent
            .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
            .replace(/\b(ERROR|失败|拒绝|fail)\b/g, '<span class="error">$1</span>')
            .replace(/\b(WARN|警告|warn)\b/g, '<span class="warn">$1</span>')
            .replace(/\b(INFO|info)\b/g, '<span class="info">$1</span>');
        pre.innerHTML = html;
    }

    function filterLogs() {
        const query = document.getElementById('logFilter').value.toUpperCase();
        const lines = document.querySelectorAll('#logContent pre');
        lines.forEach(line => {
            const text = line.textContent.toUpperCase();
            if (!query || text.includes(query)) {
                line.style.display = '';
            } else {
                line.style.display = 'none';
            }
        });
    }

    function copyLogs() {
        const content = document.getElementById('logContent').textContent;
        navigator.clipboard.writeText(content).then(() => {
            const btn = document.getElementById('copyLogBtn');
            const orig = btn.textContent;
            btn.textContent = '已复制';
            setTimeout(() => { btn.textContent = orig; }, 2000);
        }).catch(() => {
            // Fallback for older browsers
            const ta = document.createElement('textarea');
            ta.value = document.getElementById('logContent').textContent;
            document.body.appendChild(ta);
            ta.select();
            document.execCommand('copy');
            document.body.removeChild(ta);
        });
    }

    // ===== Modal Functions =====
    function showModal(id) { document.getElementById(id).classList.add('active'); }
    function closeModal(id) { document.getElementById(id).classList.remove('active'); }
    function showLogin() { showModal('loginModal'); }

    // Close modal on overlay click
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('click', (e) => {
            if (e.target === overlay) overlay.classList.remove('active');
        });
    });

    // Close modal on Escape
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            document.querySelectorAll('.modal-overlay.active').forEach(m => m.classList.remove('active'));
        }
    });

    // ===== Auth =====
    async function doLogin() {
        const user = document.getElementById('loginUser').value;
        const pass = document.getElementById('loginPass').value;
        const errEl = document.getElementById('loginError');
        errEl.style.display = 'none';

        try {
            const r = await fetch(API + '/api/v1/auth/login', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({username: user, password: pass})
            });
            const data = await r.json();
            if (r.ok && data.token) {
                authToken = data.token;
                document.getElementById('loginLabel').textContent = user;
                document.getElementById('loginBtn').style.display = 'none';
                document.getElementById('logoutBtn').style.display = '';
                closeModal('loginModal');
                refresh();
            } else {
                errEl.textContent = data.error || '登录失败';
                errEl.style.display = 'block';
            }
        } catch(e) {
            errEl.textContent = '连接失败: ' + e.message;
            errEl.style.display = 'block';
        }
    }

    function doLogout() {
        authToken = null;
        document.getElementById('loginLabel').textContent = '未登录';
        document.getElementById('loginBtn').style.display = '';
        document.getElementById('logoutBtn').style.display = 'none';
        refresh();
    }

    // ===== Token Creation =====
    function showCreateToken() {
        if (!authToken) { showLogin(); return; }
        document.getElementById('tokenResult').style.display = 'none';
        document.getElementById('tokenError').style.display = 'none';
        showModal('tokenModal');
    }

    async function doCreateToken() {
        const errEl = document.getElementById('tokenError');
        const resultEl = document.getElementById('tokenResult');
        errEl.style.display = 'none';
        resultEl.style.display = 'none';

        try {
            const r = await fetch(API + '/api/v1/tokens/create', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': 'Bearer ' + authToken
                },
                body: JSON.stringify({
                    ttl_secs: parseInt(document.getElementById('tokenTTL').value),
                    max_uses: parseInt(document.getElementById('tokenMaxUses').value),
                    max_item_size: parseInt(document.getElementById('tokenMaxSize').value),
                })
            });
            const data = await r.json();
            if (r.ok && data.token) {
                resultEl.textContent = 'Token: ' + data.token + '\n\n有效期: ' + data.ttl_secs + 's\n最大使用: ' + data.max_uses + ' 次';
                resultEl.style.display = 'block';
            } else {
                errEl.textContent = data.error || '创建失败';
                errEl.style.display = 'block';
            }
        } catch(e) {
            errEl.textContent = '连接失败: ' + e.message;
            errEl.style.display = 'block';
        }
    }

    // ===== Data Fetching =====
    async function fetchJSON(path) {
        try {
            const headers = {};
            if (authToken) headers['Authorization'] = 'Bearer ' + authToken;
            const r = await fetch(API + path, {headers});
            return await r.json();
        } catch(e) { return null; }
    }

    function statusBadge(online, trusted) {
        if (!online) return '<span class="status-badge" style="--badge-color:var(--status-offline);--badge-bg:var(--status-offline-bg)"><span class="status-dot"></span>离线</span>';
        let html = '<span class="status-badge" style="--badge-color:var(--status-online);--badge-bg:var(--status-online-bg)"><span class="status-dot pulse"></span>在线</span>';
        if (trusted) html += ' <span class="status-badge" style="--badge-color:var(--color-brand);--badge-bg:var(--sidebar-active)"><span class="status-dot"></span>受信</span>';
        return html;
    }

    async function refresh() {
        // Health
        const health = await fetchJSON('/api/v1/health');
        const running = !!health;
        const topbarStatus = document.getElementById('topbarStatus');
        const topbarText = document.getElementById('topbarStatusText');
        if (topbarStatus && topbarText) {
            if (running) {
                topbarStatus.style.cssText = '--badge-color:var(--status-online);--badge-bg:var(--status-online-bg);';
                topbarText.textContent = '运行中';
            } else {
                topbarStatus.style.cssText = '--badge-color:var(--status-offline);--badge-bg:var(--status-offline-bg);';
                topbarText.textContent = '离线';
            }
        }
        const statStatus = document.getElementById('statStatus');
        if (statStatus) {
            statStatus.textContent = running ? '运行中' : '离线';
            statStatus.className = 'value ' + (running ? 'green' : 'red');
        }

        // Uptime
        const sec = Math.floor((Date.now() - startTime) / 1000);
        const h = Math.floor(sec/3600), m = Math.floor((sec%3600)/60), s = sec%60;
        const uptimeEl = document.getElementById('statUptime');
        if (uptimeEl) uptimeEl.textContent = h > 0 ? `${h}h ${m}m` : `${m}m ${s}s`;

        // Devices
        const devData = await fetchJSON('/api/v1/devices');
        const devices = devData?.devices || [];
        const countEls = ['statDevices', 'deviceBadge', 'deviceCountBadge'];
        countEls.forEach(id => { const el = document.getElementById(id); if (el) el.textContent = devices.length; });
        const trustedCount = devices.filter(d => d.trusted).length;
        const tb = document.getElementById('trustedBadge');
        if (tb) tb.textContent = trustedCount + ' 台';

        // Device list (overview)
        const devList = document.getElementById('deviceList');
        if (devList) {
            if (devices.length === 0) {
                devList.innerHTML = '<tr><td colspan="5"><div class="empty-state"><div class="title">暂无注册设备</div><div class="desc">使用 CLI 或配对流程注册设备</div></div></td></tr>';
            } else {
                devList.innerHTML = devices.slice(0, 5).map(d => {
                    const time = d.last_seen_at ? new Date(d.last_seen_at).toLocaleString('zh-CN') : '从未';
                    return `<tr>
                        <td class="mono">${d.device_id.slice(0,12)}…</td>
                        <td>${d.name}</td>
                        <td>${d.platform}</td>
                        <td>${statusBadge(!!d.last_seen_at, d.trusted)}</td>
                        <td>${time}</td>
                    </tr>`;
                }).join('');
            }
        }

        // Device list (full)
        const fullList = document.getElementById('fullDeviceList');
        if (fullList) {
            if (devices.length === 0) {
                fullList.innerHTML = '<tr><td colspan="6"><div class="empty-state"><div class="title">暂无注册设备</div></div></td></tr>';
            } else {
                fullList.innerHTML = devices.map(d => {
                    const time = d.last_seen_at ? new Date(d.last_seen_at).toLocaleString('zh-CN') : '从未';
                    return `<tr>
                        <td class="mono">${d.device_id.slice(0,12)}…</td>
                        <td>${d.name}</td>
                        <td>${d.platform}</td>
                        <td>${statusBadge(!!d.last_seen_at, d.trusted)}</td>
                        <td>${time}</td>
                        <td><button class="btn" onclick="alert('详情页规划中')">详情</button></td>
                    </tr>`;
                }).join('');
            }
        }

        // Trusted list
        const trustedList = document.getElementById('trustedList');
        if (trustedList) {
            const trusted = devices.filter(d => d.trusted);
            if (trusted.length === 0) {
                trustedList.innerHTML = '<tr><td colspan="6"><div class="empty-state"><div class="title">暂无已信任设备</div><div class="desc">请先完成设备配对</div></div></td></tr>';
            } else {
                trustedList.innerHTML = trusted.map(d => {
                    const paired = d.paired_at ? new Date(d.paired_at).toLocaleString('zh-CN') : '未知';
                    return `<tr>
                        <td class="mono">${d.device_id.slice(0,12)}…</td>
                        <td>${d.name}</td>
                        <td>${d.platform}</td>
                        <td>${statusBadge(!!d.last_seen_at, true)}</td>
                        <td>${paired}</td>
                        <td><button class="btn danger" onclick="alert('撤销信任规划中')">撤销</button></td>
                    </tr>`;
                }).join('');
            }
        }

        // Clipboard history
        const histData = await fetchJSON('/api/v1/clipboard/history?limit=20');
        const items = histData?.items || [];
        ['itemCount', 'historyBadge', 'clipBadge2'].forEach(id => {
            const el = document.getElementById(id); if (el) el.textContent = items.length + (id === 'itemCount' ? '' : ' 条');
        });
        const statClip = document.getElementById('statClipboard');
        if (statClip) statClip.textContent = items.length;

        const histBody = document.getElementById('historyList');
        if (histBody) {
            if (items.length === 0) {
                histBody.innerHTML = '<tr><td colspan="5"><div class="empty-state"><div class="title">暂无剪贴板记录</div></div></td></tr>';
            } else {
                histBody.innerHTML = items.map(i => {
                    const time = new Date(i.created_at).toLocaleString('zh-CN');
                    const preview = (i.representations || []).map(r => {
                        if (r.content?.Text) return r.content.Text.slice(0, 50);
                        return '[二进制数据]';
                    }).join(' ') || '-';
                    const kindColor = i.kind === 'text' ? 'online' : i.kind === 'image' ? 'trusted' : 'offline';
                    return `<tr>
                        <td>${time}</td>
                        <td class="mono">${(i.source_device_id||'').slice(0,8)}…</td>
                        <td><span class="status-badge" style="--badge-color:var(--status-${kindColor});--badge-bg:var(--status-${kindColor}-bg)">${i.kind}</span></td>
                        <td>${i.size} B</td>
                        <td style="max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${preview}</td>
                    </tr>`;
                }).join('');
            }
        }

        const clipFull = document.getElementById('clipboardFullList');
        if (clipFull && items.length > 0) {
            clipFull.innerHTML = items.map(i => {
                const time = new Date(i.created_at).toLocaleString('zh-CN');
                const preview = (i.representations || []).map(r => {
                    if (r.content?.Text) return r.content.Text.slice(0, 50);
                    return '[二进制数据]';
                }).join(' ') || '-';
                const kindColor = i.kind === 'text' ? 'online' : i.kind === 'image' ? 'trusted' : 'offline';
                return `<tr>
                    <td>${time}</td>
                    <td class="mono">${(i.source_device_id||'').slice(0,8)}…</td>
                    <td><span class="status-badge" style="--badge-color:var(--status-${kindColor});--badge-bg:var(--status-${kindColor}-bg)">${i.kind}</span></td>
                    <td>${i.size} B</td>
                    <td style="max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${preview}</td>
                </tr>`;
            }).join('');
        }

        // Settings theme display
        const st = document.getElementById('settingsTheme');
        if (st) st.textContent = document.documentElement.getAttribute('data-theme') === 'dark' ? '深色' : '浅色';

        // Connection page data
        const wsEl = document.getElementById('wsStatus');
        if (wsEl) {
            wsEl.innerHTML = health ? '<span class="status-badge" style="--badge-color:var(--status-online);--badge-bg:var(--status-online-bg)"><span class="status-dot pulse"></span>可用</span>' : '<span class="status-badge" style="--badge-color:var(--status-offline);--badge-bg:var(--status-offline-bg)">不可达</span>';
        }
        const acEl = document.getElementById('activeConnections');
        if (acEl) acEl.textContent = devices.filter(d => d.last_seen_at).length + ' 台';
        const lanEl = document.getElementById('lanDiscovery');
        if (lanEl) lanEl.innerHTML = '<span class="status-badge" style="--badge-color:var(--status-limited);--badge-bg:var(--status-limited-bg)">规划中</span>';
        const sbEl = document.getElementById('serverFallback');
        if (sbEl) sbEl.innerHTML = '<span class="status-badge" style="--badge-color:var(--status-limited);--badge-bg:var(--status-limited-bg)">规划中</span>';
    }

    // ===== Init =====
    navigateTo('overview');
    refresh();
    setInterval(refresh, 5000);
</script>
```

- [ ] **Step 3: 验证 Web 文件完整性**

Run: `python3 -c "
import sys
with open('crates/glide-server/static/index.html') as f:
    content = f.read()
# Check basic structure
assert '<!DOCTYPE html>' in content, 'Missing DOCTYPE'
assert '</html>' in content, 'Missing closing html'
assert '</script>' in content, 'Missing closing script'
assert 'data-theme' in content, 'Missing theme support'
assert 'var(--page-bg)' in content, 'Missing CSS variables'
print('Web HTML structure: OK')
print(f'File size: {len(content)} bytes')
"`

Expected: All assertions pass.

---

## Phase 2: GUI Slint 页面重写

**Files:**
- Modify: `crates/glide-gui/ui/app.slint` (完整重写)
- Modify: `crates/glide-gui/src/main.rs` (添加新增属性/回调绑定)
- Modify: `crates/glide-gui/src/gui_backend.rs` (不需要修改，MockBackend 已支持所需 trait)

### 重要约束：向后兼容

`main.rs` 中的 `create_window()` 和 `refresh_window()` 依赖以下 Slint 导出的 properties/callbacks。**必须保留全部现有接口：**

**Properties (get/set):**
- `current-page: int`
- `service-running: bool`
- `connection-status: string`
- `device-count-text: string`
- `server-url: string`
- `device-name: string`
- `clipboard-enabled: bool`
- `input-enabled: bool`
- `log-content: string`
- `platform-summary: string`
- `devices: [DeviceRow]`

**Callbacks (invoke):**
- `page-changed(int)`
- `connect()`
- `disconnect()`
- `toggle-clipboard()`
- `toggle-input()`
- `save-server(string)`
- `save-name(string)`
- `pair-device()`

新增 properties/callbacks 将添加到 `main.rs` 中，不破坏已有逻辑。

### Task 2: 重写 `app.slint` — 双主题 Token + 组件库

**Files:**
- Modify: `crates/glide-gui/ui/app.slint`

- [ ] **Step 1: 重写 AppColors + 新组件**

完整替换 `app.slint` 内容。新文件结构：

```slint
import { Button, LineEdit, ScrollView, Switch } from "std-widgets.slint";

export struct DeviceRow {
    name: string,
    platform: string,
    status: string,
}

// === Dual Theme Tokens ===
export global AppColors {
    out property <color> primary: #2563eb;
    out property <color> primary-hover: #1d4ed8;

    // Light theme defaults
    out property <color> page-bg: #ffffff;
    out property <color> page-surface: #f8fafc;
    out property <color> page-surface-alt: #f1f5f9;
    out property <color> text-primary: #1e293b;
    out property <color> text-secondary: #64748b;
    out property <color> text-tertiary: #94a3b8;
    out property <color> border-base: #e2e8f0;
    out property <color> border-subtle: #f1f5f9;
    out property <color> sidebar-bg: #f8fafc;
    out property <color> sidebar-text: #475569;
    out property <color> sidebar-active: #eff6ff;
    out property <color> sidebar-active-text: #2563eb;
    out property <color> input-bg: #ffffff;
    out property <color> input-border: #cbd5e1;

    // Status colors (light)
    out property <color> status-online: #16a34a;
    out property <color> status-online-bg: #dcfce7;
    out property <color> status-offline: #dc2626;
    out property <color> status-offline-bg: #fee2e2;
    out property <color> status-connecting: #2563eb;
    out property <color> status-connecting-bg: #dbeafe;
    out property <color> status-error: #dc2626;
    out property <color> status-error-bg: #fee2e2;
    out property <color> status-limited: #d97706;
    out property <color> status-limited-bg: #fef3c7;
    out property <color> status-unpaired: #64748b;
    out property <color> status-unpaired-bg: #f1f5f9;

    // Alert colors (light)
    out property <color> alert-info-bg: #eff6ff;
    out property <color> alert-info-border: #2563eb;
    out property <color> alert-info-text: #1e40af;
    out property <color> alert-warning-bg: #fffbeb;
    out property <color> alert-warning-border: #d97706;
    out property <color> alert-warning-text: #92400e;
    out property <color> alert-success-bg: #dcfce7;
    out property <color> alert-success-border: #16a34a;
    out property <color> alert-success-text: #166534;
    out property <color> alert-danger-bg: #fee2e2;
    out property <color> alert-danger-border: #dc2626;
    out property <color> alert-danger-text: #991b1b;

    // Log
    out property <color> log-bg: #0f172a;
    out property <color> log-text: #cbd5e1;
    out property <color> log-info: #38bdf8;
    out property <color> log-warn: #fbbf24;
    out property <color> log-error: #f87171;

    // Dark theme override helper
    in property <bool> is-dark: false;
}

// === Base Components ===

component StatusBadge inherits Rectangle {
    in property <string> label;
    in property <color> badge-color: AppColors.text-tertiary;
    in property <color> badge-bg: AppColors.page-surface-alt;
    in property <bool> pulse: false;
    width: 88px;
    height: 22px;
    border-radius: 4px;
    background: badge-bg;

    HorizontalLayout {
        padding-left: 8px;
        padding-right: 8px;
        spacing: 5px;
        alignment: center;

        Rectangle {
            width: 7px; height: 7px;
            border-radius: 3.5px;
            background: badge-color;
        }
        Text {
            text: label;
            font-size: 11px;
            font-weight: 500;
            color: badge-color;
            vertical-alignment: center;
        }
    }
}

component DeviceCard inherits Rectangle {
    in property <string> device-name;
    in property <string> device-platform;
    in property <string> status-text;
    in property <bool> is-online;
    height: 64px;
    background: AppColors.page-surface;
    border-radius: 6px;
    border-width: 1px;
    border-color: AppColors.border-base;

    HorizontalLayout {
        padding-left: 14px;
        padding-right: 14px;
        spacing: 12px;
        alignment: center;

        VerticalLayout {
            spacing: 3px;
            horizontal-stretch: 1;
            Text {
                text: device-name;
                font-size: 14px;
                font-weight: 600;
                color: AppColors.text-primary;
            }
            Text {
                text: device-platform;
                font-size: 12px;
                color: AppColors.text-secondary;
            }
        }

        StatusBadge {
            label: status-text;
            badge-color: is-online ? AppColors.status-online : AppColors.status-offline;
            badge-bg: is-online ? AppColors.status-online-bg : AppColors.status-offline-bg;
        }
    }
}

component SettingsRow inherits Rectangle {
    in property <string> label;
    in property <string> detail;
    in property <bool> checked;
    callback toggled();
    height: 52px;
    background: AppColors.page-surface;
    border-radius: 6px;
    border-width: 1px;
    border-color: AppColors.border-base;

    HorizontalLayout {
        padding-left: 14px;
        padding-right: 14px;
        spacing: 12px;
        alignment: center;

        VerticalLayout {
            spacing: 3px;
            horizontal-stretch: 1;
            Text {
                text: label;
                font-size: 13px;
                font-weight: 600;
                color: AppColors.text-primary;
            }
            Text {
                text: detail;
                font-size: 11px;
                color: AppColors.text-secondary;
            }
        }

        Switch {
            checked: root.checked;
            toggled => { root.toggled(); }
        }
    }
}

component AlertBox inherits Rectangle {
    in property <string> title;
    in property <string> body;
    in property <string> type: "info";  // info | warning | success | danger

    min-height: 64px;
    border-radius: 6px;
    border-width: 0px;
    border-left: 3px;

    background: type == "info" ? AppColors.alert-info-bg :
                type == "warning" ? AppColors.alert-warning-bg :
                type == "success" ? AppColors.alert-success-bg :
                AppColors.alert-danger-bg;
    border-color: type == "info" ? AppColors.alert-info-border :
                  type == "warning" ? AppColors.alert-warning-border :
                  type == "success" ? AppColors.alert-success-border :
                  AppColors.alert-danger-border;

    HorizontalLayout {
        padding: 12px;
        spacing: 10px;
        alignment: start;

        Text {
            text: type == "info" ? "ℹ" :
                  type == "warning" ? "⚠" :
                  type == "success" ? "✓" : "✕";
            font-size: 16px;
            color: type == "info" ? AppColors.alert-info-border :
                   type == "warning" ? AppColors.alert-warning-border :
                   type == "success" ? AppColors.alert-success-border :
                   AppColors.alert-danger-border;
            vertical-alignment: center;
        }

        VerticalLayout {
            spacing: 4px;
            horizontal-stretch: 1;
            Text {
                text: title;
                font-size: 13px;
                font-weight: 600;
                color: type == "info" ? AppColors.alert-info-text :
                       type == "warning" ? AppColors.alert-warning-text :
                       type == "success" ? AppColors.alert-success-text :
                       AppColors.alert-danger-text;
            }
            Text {
                text: body;
                font-size: 12px;
                color: AppColors.text-secondary;
                wrap: word-wrap;
            }
        }
    }
}

component SectionTitle inherits Text {
    in property <string> label;
    text: label;
    font-size: 16px;
    font-weight: 600;
    color: AppColors.text-primary;
}

component Sidebar inherits Rectangle {
    in property <int> current-index: 0;
    callback page-selected(int);
    width: 180px;
    background: AppColors.sidebar-bg;

    VerticalLayout {
        padding-top: 14px;
        padding-left: 12px;
        padding-right: 12px;
        spacing: 4px;

        Text {
            text: "Glide";
            font-size: 22px;
            font-weight: 800;
            color: AppColors.primary;
            padding-left: 6px;
            padding-bottom: 10px;
        }

        for item[i] in [
            { label: "状态", index: 0 },
            { label: "设备", index: 1 },
            { label: "配对", index: 2 },
            { label: "日志", index: 3 },
            { label: "设置", index: 4 },
            { label: "平台能力", index: 5 },
            { label: "关于", index: 6 },
        ]: Rectangle {
            height: 38px;
            border-radius: 4px;
            background: current-index == item.index ? AppColors.sidebar-active : transparent;

            Text {
                text: item.label;
                font-size: 13px;
                font-weight: current-index == item.index ? 600 : 400;
                color: current-index == item.index ? AppColors.sidebar-active-text : AppColors.sidebar-text;
                vertical-alignment: center;
                padding-left: 8px;
            }

            TouchArea {
                clicked => { root.page-selected(item.index); }
            }
        }
    }
}

component TopBar inherits Rectangle {
    in property <string> title;
    in property <string> connection-status;
    height: 52px;
    background: AppColors.page-surface;

    HorizontalLayout {
        padding-left: 24px;
        padding-right: 24px;
        spacing: 12px;
        alignment: center;

        Text {
            text: title;
            font-size: 20px;
            font-weight: 700;
            color: AppColors.text-primary;
        }

        Rectangle { horizontal-stretch: 1; }

        StatusBadge {
            label: connection-status;
            badge-color: connection-status == "已连接" ? AppColors.status-online :
                         connection-status == "连接中" ? AppColors.status-connecting :
                         AppColors.status-offline;
            badge-bg: connection-status == "已连接" ? AppColors.status-online-bg :
                      connection-status == "连接中" ? AppColors.status-connecting-bg :
                      AppColors.status-offline-bg;
            pulse: connection-status == "已连接" || connection-status == "连接中";
        }
    }
}

// === Pages ===

component StatusPage inherits VerticalLayout {
    in property <bool> service-running;
    in property <string> connection-status;
    in property <string> device-count-text;
    in property <string> server-url;
    in property <bool> clipboard-enabled;
    in property <bool> input-enabled;
    callback connect();
    callback disconnect();
    callback toggle-clipboard();
    callback toggle-input();
    padding: 24px;
    spacing: 16px;

    SectionTitle { label: "后台服务"; }

    Rectangle {
        height: 100px;
        background: AppColors.page-surface;
        border-radius: 6px;
        border-width: 1px;
        border-color: AppColors.border-base;

        HorizontalLayout {
            padding: 16px;
            spacing: 16px;
            alignment: center;

            VerticalLayout {
                spacing: 6px;
                horizontal-stretch: 1;
                Text {
                    text: service-running ? "后台服务运行中" : "后台服务未运行";
                    font-size: 16px;
                    font-weight: 700;
                    color: AppColors.text-primary;
                }
                Text {
                    text: server-url == "" ? "尚未配置服务端地址" : server-url;
                    font-size: 12px;
                    color: AppColors.text-secondary;
                }
                Text {
                    text: "已连接设备: " + device-count-text;
                    font-size: 12px;
                    color: AppColors.text-secondary;
                }
            }

            if connection-status == "已连接" : Button {
                text: "断开";
                clicked => { root.disconnect(); }
            }
            if connection-status != "已连接" : Button {
                text: "连接";
                clicked => { root.connect(); }
            }
        }
    }

    SectionTitle { label: "同步开关"; }
    SettingsRow {
        label: "剪贴板同步";
        detail: "同步文本，后续扩展图片和文件";
        checked: clipboard-enabled;
        toggled => { root.toggle-clipboard(); }
    }
    SettingsRow {
        label: "键鼠共享";
        detail: "默认关闭，由后台服务和平台层执行";
        checked: input-enabled;
        toggled => { root.toggle-input(); }
    }
}

component DevicesPage inherits VerticalLayout {
    in property <[DeviceRow]> devices;
    padding: 24px;
    spacing: 12px;

    SectionTitle { label: "受信任设备"; }

    if devices.length == 0 : Rectangle {
        height: 100px;
        background: AppColors.page-surface;
        border-radius: 6px;
        border-width: 1px;
        border-color: AppColors.border-base;
        Text {
            text: "暂无已注册设备\n请先完成设备配对";
            font-size: 13px;
            color: AppColors.text-secondary;
            horizontal-alignment: center;
            vertical-alignment: center;
            wrap: word-wrap;
        }
    }

    ScrollView {
        viewport-height: 320px;
        VerticalLayout {
            spacing: 10px;
            for device in devices: DeviceCard {
                device-name: device.name;
                device-platform: device.platform;
                status-text: device.status;
                is-online: device.status == "在线";
            }
        }
    }
}

component PairingPage inherits VerticalLayout {
    callback pair-device();
    padding: 24px;
    spacing: 16px;

    AlertBox {
        title: "设备配对";
        body: "配对后的设备将加入受信列表，可以同步剪贴板、文件和键鼠输入。真实配对码、设备指纹和授权确认将在后台服务通信接入后提供。";
        type: "info";
    }

    SectionTitle { label: "配对设备"; }
    Rectangle {
        height: 120px;
        background: AppColors.page-surface;
        border-radius: 6px;
        border-width: 1px;
        border-color: AppColors.border-base;

        VerticalLayout {
            padding: 16px;
            spacing: 12px;
            alignment: center;
            Text {
                text: "配对流程将在后台服务通信接入后提供真实配对码、设备指纹和授权确认。";
                font-size: 13px;
                color: AppColors.text-secondary;
                wrap: word-wrap;
                horizontal-alignment: center;
            }
            Button {
                text: "生成模拟配对码";
                clicked => { root.pair-device(); }
            }
        }
    }
}

component LogPage inherits VerticalLayout {
    in property <string> log-content;
    padding: 24px;
    spacing: 16px;

    SectionTitle { label: "运行日志"; }
    Rectangle {
        background: AppColors.log-bg;
        border-radius: 6px;
        border-width: 1px;
        border-color: #1e293b;
        vertical-stretch: 1;

        ScrollView {
            VerticalLayout {
                padding: 12px;
                Text {
                    text: log-content == "" ? "暂无日志" : log-content;
                    font-size: 12px;
                    font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
                    color: AppColors.log-text;
                    wrap: no-wrap;
                    line-height: 1.6;
                }
            }
        }
    }
}

component SettingsPage inherits VerticalLayout {
    in property <string> server-url;
    in property <string> device-name;
    callback save-server(string);
    callback save-name(string);
    padding: 24px;
    spacing: 16px;

    SectionTitle { label: "服务端"; }
    Rectangle {
        height: 52px;
        background: AppColors.page-surface;
        border-radius: 6px;
        border-width: 1px;
        border-color: AppColors.border-base;

        HorizontalLayout {
            padding-left: 14px;
            padding-right: 14px;
            spacing: 10px;
            alignment: center;
            Text {
                text: "服务端地址";
                width: 88px;
                font-size: 13px;
                color: AppColors.text-primary;
            }
            server-input := LineEdit {
                text: server-url;
                placeholder-text: "http://server:8080";
                horizontal-stretch: 1;
            }
            Button {
                text: "保存";
                clicked => { root.save-server(server-input.text); }
            }
        }
    }

    SectionTitle { label: "本机设备"; }
    Rectangle {
        height: 52px;
        background: AppColors.page-surface;
        border-radius: 6px;
        border-width: 1px;
        border-color: AppColors.border-base;

        HorizontalLayout {
            padding-left: 14px;
            padding-right: 14px;
            spacing: 10px;
            alignment: center;
            Text {
                text: "本机设备名";
                width: 88px;
                font-size: 13px;
                color: AppColors.text-primary;
            }
            name-input := LineEdit {
                text: device-name;
                horizontal-stretch: 1;
            }
            Button {
                text: "保存";
                clicked => { root.save-name(name-input.text); }
            }
        }
    }
}

component PlatformPage inherits VerticalLayout {
    in property <string> platform-summary;
    padding: 24px;
    spacing: 16px;

    AlertBox {
        title: "能力提示";
        body: "Linux 第一阶段优先 X11。Wayland 下全局键鼠控制受合成器限制，GUI 不会承诺完整控制能力。";
        type: "warning";
    }
    Rectangle {
        background: AppColors.page-surface;
        border-radius: 6px;
        border-width: 1px;
        border-color: AppColors.border-base;
        vertical-stretch: 1;

        VerticalLayout {
            padding: 16px;
            Text {
                text: platform-summary;
                font-size: 13px;
                color: AppColors.text-secondary;
                wrap: word-wrap;
            }
        }
    }
}

component AboutPage inherits VerticalLayout {
    padding: 24px;
    spacing: 12px;

    Text {
        text: "Glide";
        font-size: 30px;
        font-weight: 800;
        color: AppColors.primary;
    }
    Text {
        text: "局域网优先的剪贴板同步与键鼠共享";
        font-size: 14px;
        color: AppColors.text-secondary;
    }
    Rectangle { height: 1px; background: AppColors.border-base; }
    Text {
        text: "界面运行时：Rust + Slint";
        font-size: 13px;
        color: AppColors.text-primary;
    }
    Text {
        text: "不依赖 Tauri、WebView2 或内嵌浏览器运行时。";
        font-size: 13px;
        color: AppColors.text-primary;
    }
    Text {
        text: "仓库地址：https://github.com/SuPerCxyz/glide";
        font-size: 13px;
        color: AppColors.primary;
    }
}

// === Main Window ===

export component MainWindow inherits Window {
    title: "Glide";
    preferred-width: 860px;
    preferred-height: 590px;
    min-width: 720px;
    min-height: 500px;
    background: AppColors.page-bg;

    in-out property <int> current-page: 0;
    in-out property <bool> service-running: true;
    in-out property <string> connection-status: "未连接";
    in-out property <string> device-count-text: "0";
    in-out property <string> server-url: "";
    in-out property <string> device-name: "";
    in-out property <bool> clipboard-enabled: true;
    in-out property <bool> input-enabled: false;
    in-out property <string> log-content: "";
    in-out property <string> platform-summary: "";
    in-out property <[DeviceRow]> devices: [];

    callback page-changed(int);
    callback connect();
    callback disconnect();
    callback toggle-clipboard();
    callback toggle-input();
    callback save-server(string);
    callback save-name(string);
    callback pair-device();

    HorizontalLayout {
        Sidebar {
            current-index: current-page;
            page-selected(index) => { root.page-changed(index); }
        }
        VerticalLayout {
            horizontal-stretch: 1;
            TopBar {
                title: current-page == 0 ? "状态" :
                    current-page == 1 ? "设备" :
                    current-page == 2 ? "配对" :
                    current-page == 3 ? "日志" :
                    current-page == 4 ? "设置" :
                    current-page == 5 ? "平台能力" : "关于";
                connection-status: connection-status;
            }

            if current-page == 0 : StatusPage {
                service-running: service-running;
                connection-status: connection-status;
                device-count-text: device-count-text;
                server-url: server-url;
                clipboard-enabled: clipboard-enabled;
                input-enabled: input-enabled;
                connect => { root.connect(); }
                disconnect => { root.disconnect(); }
                toggle-clipboard => { root.toggle-clipboard(); }
                toggle-input => { root.toggle-input(); }
            }
            if current-page == 1 : DevicesPage { devices: devices; }
            if current-page == 2 : PairingPage { pair-device => { root.pair-device(); } }
            if current-page == 3 : LogPage { log-content: log-content; }
            if current-page == 4 : SettingsPage {
                server-url: server-url;
                device-name: device-name;
                save-server(url) => { root.save-server(url); }
                save-name(name) => { root.save-name(name); }
            }
            if current-page == 5 : PlatformPage { platform-summary: platform-summary; }
            if current-page == 6 : AboutPage {}
        }
    }
}
```

- [ ] **Step 2: 验证 Slint 文件语法**

Run: `cargo check --package glide-gui 2>&1 | head -30`

Expected: Compiles successfully (existing warnings about dead_code are OK, no new errors).

### Task 3: 运行验证

- [ ] **Step 1: 编译检查**

Run: `cargo check --workspace 2>&1 | tail -5`

Expected: `Finished` with no new errors.

- [ ] **Step 2: 运行测试**

Run: `cargo test --workspace 2>&1 | tail -20`

Expected: All tests pass.

- [ ] **Step 3: GUI Xvfb smoke**

Run: `cargo build --package glide-gui 2>&1 | tail -3`
Then: `GLIDE_GUI_LOG=$(mktemp) xvfb-run --auto-servernum timeout 5 target/debug/glide-gui --smoke 2>&1`

Expected: Output includes `glide-gui smoke ok` and `exit=0`.

- [ ] **Step 4: 交互 smoke**

Run: `GLIDE_GUI_LOG=$(mktemp) xvfb-run --auto-servernum timeout 10 target/debug/glide-gui --interaction-smoke 2>&1`

Expected: Output includes `glide-gui interaction smoke ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/glide-server/static/index.html crates/glide-gui/ui/app.slint docs/DESIGN.md docs/PRODUCT.md
git commit -m "refactor: unify UI/UX with design system, light/dark themes, monospace font

Redesign web admin page and GUI Slint pages with unified design system.
- Light theme default, Dark theme toggle support
- Monospace font (JetBrains Mono / Fira Code stack) globally
- Six-state status colors (online, offline, connecting, error, limited, unpaired)
- AlertBox component with info/warning/success/danger types
- Consistent spacing (4px grid), border-radius (4/6/8px), shadows
- Web: new sidebar navigation, overview/devices/connection/pairing/clipboard/files/logs/settings pages
- GUI: updated StatusBadge, DeviceCard, SettingsRow, LogPage, PlatformPage with new tokens
- All existing Slint properties/callbacks preserved for main.rs compatibility

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

### Spec Coverage Check
- [x] 统一 Web/GUI 视觉风格 → 两套都使用同一 token 系统
- [x] 默认白色主题 + 黑色主题切换 → CSS `data-theme` + Slint 预留 dark token
- [x] 全局等宽字体 → JetBrains Mono / Fira Code / Consolas / monospace
- [x] 简洁现代专业轻量 → 无装饰卡片、清晰状态、克制颜色
- [x] 普通用户能快速理解状态 → 六态状态色 + AlertBox 可操作错误提示
- [x] 六态状态色 → 在线/离线/连接中/错误/受限/未配对
- [x] 日志适合长时间阅读 → 等宽、关键词高亮、复制按钮、横向滚动
- [x] Wayland/权限/防火墙提示 → AlertBox warning/info 组件
- [x] 更新 DESIGN.md → 已更新
- [x] 更新 PRODUCT.md → 已更新

### Placeholder Scan
- 无 TBD/TODO
- 无"类似 Task N"
- 所有代码步骤包含完整代码
- 文件路径明确

### Type Consistency
- Slint properties/callbacks 保持与现有 `main.rs` 一致
- 新增组件命名不与现有冲突
- CSS 变量命名一致
