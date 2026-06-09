// Disable console window on Windows (all builds).
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod gui_backend;

use gui_backend::{AppSettings, GuiBackend, MockBackend};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::{self, Command};
use std::rc::Rc;
use std::sync::Mutex;
use tracing::{info, warn};
use tracing::Event;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use tracing_subscriber::prelude::*;

slint::include_modules!();

// === GUI 日志桥接层 ===
// 将 tracing 事件同步写入 GUI 日志缓冲区，供日志页展示和复制
static GUI_LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());
const MAX_GUI_LOG_ENTRIES: usize = 200; // GUI 日志最大保留条数

// 需要过滤掉的第三方库模块前缀
const FILTERED_TARGETS: &[&str] = &[
    "hyper::",
    "reqwest::",
    "rustls::",
    "tungstenite::",
    "mio::",
    "tokio::",
    "want::",
    "h2::",
];

struct GuiLogLayer;

impl<S> Layer<S> for GuiLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // 过滤掉第三方库的 debug/trace 级别日志
        let target = event.metadata().target();
        let level = event.metadata().level();
        if (*level == tracing::Level::DEBUG || *level == tracing::Level::TRACE)
            && FILTERED_TARGETS.iter().any(|t| target.starts_with(t))
        {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        // 不带时间戳——backend.log() 会自动添加
        let entry = format!("{} {}", target, visitor.message);
        if let Ok(mut logs) = GUI_LOG_BUFFER.lock() {
            logs.push(entry);
            let excess = logs.len().saturating_sub(500);
            if excess > 0 {
                logs.drain(0..excess);
            }
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !self.message.is_empty() {
            self.message.push(' ');
        }
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            self.message.push_str(field.name());
            self.message.push('=');
            self.message.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !self.message.is_empty() {
            self.message.push(' ');
        }
        if field.name() == "message" {
            self.message.push_str(&format!("{value:?}"));
        } else {
            self.message.push_str(&format!("{}={value:?}", field.name()));
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Err(error) = run_app() {
        let message = format!("{error}");
        write_diagnostic("error", &message);
        eprintln!("glide-gui failed: {message}");
        eprintln!("diagnostics={}", diagnostic_log_path().display());
        return Err(error);
    }
    Ok(())
}

use tracing_subscriber::EnvFilter;

fn run_app() -> Result<(), Box<dyn Error>> {
    let _verbose = env::args().any(|arg| arg == "--verbose" || arg == "-v");
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("glide-gui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    install_panic_logger();

    // Suppress ICU4X segmentation warnings for CJK text rendering
    // (upstream issue: https://github.com/slint-ui/slint/issues/11638)
    // Bridge log crate -> tracing so ICU4X messages are caught by our filter
    let _ = tracing_log::LogTracer::init();
    let filter = EnvFilter::try_from_env("GLIDE_GUI_LOG").unwrap_or_else(|_| {
        EnvFilter::new(
            "info,glide_core=debug,glide_cli=debug,glide_server=debug,glide_desktop=debug,glide_gui=debug,glide_daemon=debug,icu_provider=error,icu_segmenter=error,icu_locid=error,icu_list=error,icu_locale=error",
        )
    });
    // 组合日志层：GuiLogLayer 桥接事件到 GUI 日志页，fmt 层输出到 stderr
    let _ = tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(GuiLogLayer)
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr).with_filter(filter)),
    );
    write_diagnostic("process", "glide-gui starting");
    write_diagnostic(
        "renderer",
        &format!(
            "SLINT_BACKEND={}",
            env::var("SLINT_BACKEND").unwrap_or_else(|_| "default".to_string())
        ),
    );

    if args.iter().any(|arg| arg == "--diagnostics-path") {
        println!("{}", diagnostic_log_path().display());
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--diagnostics") {
        print_diagnostics()?;
        return Ok(());
    }

    if should_run_windows_renderer_auto(&args) {
        return run_windows_renderer_auto(&args);
    }

    if args.iter().any(|arg| arg == "--smoke") {
        run_smoke()?;
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--interaction-smoke") {
        run_interaction_smoke()?;
        return Ok(());
    }

    run_gui()
}

fn default_device_id() -> String {
    env::var("GLIDE_DEVICE_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn default_device_name() -> String {
    env::var("GLIDE_DEVICE_NAME")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "glide-device".to_string()))
}

fn run_gui() -> Result<(), Box<dyn Error>> {
    let mut backend = MockBackend::with_id(default_device_id());

    // Start LAN engines in background threads with their own tokio runtimes
    let device_id = default_device_id();
    let device_name = default_device_name();
    let lan_sync_port: u16 = env::var("GLIDE_LAN_SYNC_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9999);

    let engine = glide_desktop::lan_sync::LanSyncEngine::new(
        device_id.clone(),
        device_name.clone(),
        lan_sync_port,
    );

    // Share LAN state with backend so GUI can see discovered peers
    let lan_state = engine.state.clone();
    backend.lan_state = Some(lan_state);

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                warn!("failed to create tokio runtime for LAN sync: {}", e);
                return;
            }
        };
        rt.block_on(async {
            info!("Starting LAN sync engine on port {}", lan_sync_port);
            if let Err(e) = engine.start().await {
                warn!("LAN sync engine failed: {}", e);
            }
        });
    });

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                warn!("failed to create tokio runtime for LAN input: {}", e);
                return;
            }
        };
        let engine = glide_desktop::lan_input::LanInputEngine::new(
            device_id,
            device_name,
            lan_sync_port + 1,
        );
        rt.block_on(async {
            info!("Starting LAN input engine on port {}", lan_sync_port + 1);
            if let Err(e) = engine.start().await {
                warn!("LAN input engine failed: {}", e);
            }
        });
    });

    // Give engines a moment to start before opening window
    std::thread::sleep(std::time::Duration::from_millis(500));

    let window = create_window(&backend)?;

    info!("Glide GUI started");
    write_diagnostic("process", "glide-gui window running");

    // 每 5 秒刷新状态（HTTP 调用在后端有 10 秒缓存，大多数时候直接返回缓存）
    let timer_weak = window.as_weak();
    let timer_backend = backend.clone();
    let timer = Timer::default();
    let mut refresh_count = 0u32;
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(5),
        move || {
            if let Some(win) = timer_weak.upgrade() {
                refresh_count = refresh_count.wrapping_add(1);
                // 日志每 15 秒刷新一次
                let refresh_logs = refresh_count % 3 == 0;
                refresh_window(&win, &timer_backend, refresh_logs);
            }
        },
    );
    std::mem::forget(timer);
    window.run()?;
    Ok(())
}

fn run_smoke() -> Result<(), Box<dyn Error>> {
    let backend = MockBackend::with_id(default_device_id());
    let _window = create_window(&backend)?;

    let log_path = diagnostic_log_path();
    let status = backend.get_service_status().data;
    let settings = backend.get_settings().data.unwrap_or_else(default_settings);
    let devices_raw = backend.list_devices().data.unwrap_or_default();
    // Convert to DeviceRow format for smoke
    let _device_rows: Vec<_> = devices_raw.iter().map(|d| DeviceRow {
        name: d.name.clone().into(),
        platform: d.platform.clone().into(),
        status: if d.online { SharedString::from("在线") } else { SharedString::from("离线") },
        device_id: d.device_id.clone().into(),
        trusted: d.trusted,
        is_lan: true,
    }).collect();
    // Old method kept for backward compat
    let _ = devices_raw;

    write_diagnostic(
        "smoke",
        &format!(
            "ok version={} os={} arch={} devices={} server_url={}",
            env!("CARGO_PKG_VERSION"),
            env::consts::OS,
            env::consts::ARCH,
            devices_raw.len(),
            settings.server_url
        ),
    );

    println!("glide-gui smoke ok");
    println!("version={}", env!("CARGO_PKG_VERSION"));
    println!("os={}", env::consts::OS);
    println!("arch={}", env::consts::ARCH);
    println!(
        "service_running={}",
        status.as_ref().map(|value| value.running).unwrap_or(false)
    );
    println!(
        "connection_status={}",
        status
            .as_ref()
            .map(|value| value.connection_status.as_str())
            .unwrap_or("unknown")
    );
    println!("diagnostics={}", log_path.display());

    Ok(())
}

fn run_interaction_smoke() -> Result<(), Box<dyn Error>> {
    let backend = MockBackend::with_id(default_device_id());
    let window = create_window(&backend)?;

    window.invoke_toggle_clipboard();
    window.invoke_toggle_input();
    window.invoke_connect();
    window.invoke_page_changed(3);
    window.invoke_save_server(SharedString::from("http://192.0.2.10:8080"));
    window.invoke_save_name(SharedString::from("glide-smoke"));
    let status = backend
        .get_service_status()
        .data
        .unwrap_or_else(|| panic!("交互 smoke 未从模拟后端拿到服务状态"));
    let settings = backend
        .get_settings()
        .data
        .unwrap_or_else(|| panic!("交互 smoke 未从模拟后端拿到设置"));
    let _logs = backend.tail_logs(20).data.unwrap_or_default();

    // Accept both "已连接" (server present) and "连接断开" (offline/CI fallback)
    if status.connection_status == "未连接" {
        return Err("interaction smoke failed: connect callback did not update status".into());
    }
    if status.clipboard_enabled {
        return Err("interaction smoke failed: clipboard toggle did not update status".into());
    }
    if !status.input_enabled {
        return Err("interaction smoke failed: input toggle did not update status".into());
    }
    if settings.server_url != "http://192.0.2.10:8080" {
        return Err(
            "interaction smoke failed: save-server callback did not update settings".into(),
        );
    }
    if settings.device_name != "glide-smoke" {
        return Err("interaction smoke failed: save-name callback did not update settings".into());
    }
    // pair_device removed - trust/device management is now LAN-based
    // and handled via the GUI trust buttons

    write_diagnostic(
        "interaction-smoke",
        &format!(
            "ok connected={} clipboard={} input={} page={}",
            status.connection_status,
            status.clipboard_enabled,
            status.input_enabled,
            window.get_current_page()
        ),
    );

    println!("glide-gui interaction smoke ok");
    println!("current_page={}", window.get_current_page());
    println!("connection_status={}", window.get_connection_status());
    println!("clipboard_enabled={}", window.get_clipboard_enabled());
    println!("input_enabled={}", window.get_input_enabled());
    println!("server_url={}", settings.server_url);
    println!("device_name={}", settings.device_name);
    println!("diagnostics={}", diagnostic_log_path().display());

    Ok(())
}

fn create_window(backend: &MockBackend) -> Result<MainWindow, slint::PlatformError> {
    let window = MainWindow::new()?;

    refresh_window(&window, &backend, true);

    {
        let win = window.as_weak();
        window.on_page_changed(move |idx| {
            if let Some(win) = win.upgrade() {
                win.set_current_page(idx);
            }
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_toggle_clipboard(move || {
            let Some(win) = win.upgrade() else {
                return;
            };
            let next = !win.get_clipboard_enabled();
            if !backend.set_clipboard_enabled(next).success {
                warn!("failed to toggle clipboard");
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_toggle_input(move || {
            let Some(win) = win.upgrade() else {
                return;
            };
            let next = !win.get_input_enabled();
            if !backend.set_input_enabled(next).success {
                warn!("failed to toggle input sharing");
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_connect(move || {
            let Some(win) = win.upgrade() else {
                return;
            };
            let url = win.get_server_url().to_string();
            // 立即更新 UI 显示"连接中..."
            win.set_connection_status(SharedString::from("连接中..."));
            // 异步执行 HTTP 连接，不阻塞 UI 线程
            let rx = backend.connect_server_async(&url);
            let weak = win.as_weak();
            let thread_backend = backend.clone();
            std::thread::spawn(move || {
                // 等待连接结果（最长 8 秒超时）
                match rx.recv_timeout(std::time::Duration::from_secs(8)) {
                    Ok(result) => {
                        if !result.success {
                            warn!("connect failed: {:?}", result.error);
                        }
                        if let Some(win) = weak.upgrade() {
                            refresh_window(&win, &thread_backend, true);
                        }
                    }
                    Err(_) => {
                        warn!("connect timed out");
                        if let Some(win) = weak.upgrade() {
                            win.set_connection_status(SharedString::from("连接超时"));
                        }
                    }
                }
            });
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_disconnect(move || {
            let Some(win) = win.upgrade() else {
                return;
            };
            let result = backend.disconnect_server();
            if !result.success {
                warn!("disconnect failed: {:?}", result.error);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_save_server(move |url| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let result = backend.save_server_url(&url);
            if !result.success {
                warn!("failed to save server url: {:?}", result.error);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_save_name(move |name| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let result = backend.save_device_name(&name);
            if !result.success {
                warn!("failed to save device name: {:?}", result.error);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_trust_device(move |device_id| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let id = device_id.to_string();
            if backend.trust_device(&id).success {
                info!("Trusted LAN device: {}", id);
            } else {
                warn!("Failed to trust LAN device: {}", id);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_untrust_device(move |device_id| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let id = device_id.to_string();
            if backend.untrust_device(&id).success {
                info!("Untrusted LAN device: {}", id);
            } else {
                warn!("Failed to untrust LAN device: {}", id);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let win = window.as_weak();
        window.on_remove_device(move |device_id| {
            let Some(_win) = win.upgrade() else {
                return;
            };
            info!("Remove request for device: {}", device_id);
            // TODO: Implement actual device removal
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_save_auth(move |username, password| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let result = backend.save_auth(&username, &password);
            if !result.success {
                warn!("failed to save auth: {:?}", result.error);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_save_registration_token(move |token| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let result = backend.save_registration_token(&token);
            if !result.success {
                warn!("failed to save registration token: {:?}", result.error);
            }
            refresh_window_local(&win, &backend, false);
        });
    }

    {
        let win = window.as_weak();
        window.on_toggle_tls(move || {
            let Some(win) = win.upgrade() else {
                return;
            };
            let next = !win.get_enable_tls();
            win.set_enable_tls(next);
            info!("TLS encryption set to: {}", next);
        });
    }

    Ok(window)
}

/// 仅刷新本地状态（设置、日志、平台能力），不触发任何 HTTP 调用。
/// 用于 UI 回调（开关、保存等），确保操作即时响应。
fn refresh_window_local(window: &MainWindow, backend: &MockBackend, refresh_logs: bool) {
    if let Some(settings) = backend.get_settings().data {
        window.set_device_name(SharedString::from(settings.device_name));
        window.set_auth_username(SharedString::from(settings.auth_username));
        window.set_auth_password(SharedString::from(settings.auth_password));
        window.set_registration_token(SharedString::from(settings.registration_token));
    }

    if refresh_logs {
        if let Ok(buf) = GUI_LOG_BUFFER.lock() {
            let content = buf.join("\n");
            window.set_log_content(SharedString::from(content));
        }
    }

    if let Some(capabilities) = backend.get_platform_capabilities().data {
        let notes = if capabilities.notes.is_empty() {
            "暂无额外限制".to_string()
        } else {
            capabilities.notes.join("\n")
        };
        window.set_platform_summary(SharedString::from(format!(
            "平台: {}\n剪贴板: {}\n键鼠: {}\n文件传输: {}\n{}",
            capabilities.platform,
            capabilities.clipboard,
            capabilities.input,
            capabilities.file_transfer,
            notes
        )));
    }
}

fn refresh_window(window: &MainWindow, backend: &MockBackend, refresh_logs: bool) {
    if let Some(status) = backend.get_service_status().data {
        window.set_service_running(status.running);
        window.set_connection_status(SharedString::from(status.connection_status));
        window.set_clipboard_enabled(status.clipboard_enabled);
        window.set_input_enabled(status.input_enabled);
        window.set_device_count_text(SharedString::from(status.device_count.to_string()));
        window.set_server_url(SharedString::from(status.server_url));
    }

    if let Some(settings) = backend.get_settings().data {
        window.set_device_name(SharedString::from(settings.device_name));
        window.set_auth_username(SharedString::from(settings.auth_username));
        window.set_auth_password(SharedString::from(settings.auth_password));
        window.set_registration_token(SharedString::from(settings.registration_token));
    }

    if let Some(devices) = backend.list_devices().data {
        let rows: Vec<DeviceRow> = devices
            .into_iter()
            .map(|device| {
                let is_lan = device.platform.starts_with("LAN ");
                DeviceRow {
                name: SharedString::from(device.name),
                platform: SharedString::from(device.platform),
                status: SharedString::from(if device.online { "在线" } else { "离线" }),
                device_id: SharedString::from(device.device_id),
                trusted: device.trusted,
                is_lan,
            }})
            .collect();
        let total = rows.len();
        let online = rows.iter().filter(|r| r.status == "在线").count();
        window.set_devices(ModelRc::from(Rc::new(VecModel::from(rows))));
        window.set_total_devices(total as i32);
        window.set_online_devices(online as i32);
    }

    // 日志每 10 秒刷新一次（减少卡顿）
    if refresh_logs {
        // 直接从全局缓冲区读取，不通过 backend.log() 中转
        if let Ok(buf) = GUI_LOG_BUFFER.lock() {
            let content = buf.join("\n");
            window.set_log_content(SharedString::from(content));
        }
    }

    if let Some(capabilities) = backend.get_platform_capabilities().data {
        let notes = if capabilities.notes.is_empty() {
            "暂无额外限制".to_string()
        } else {
            capabilities.notes.join("\n")
        };
        window.set_platform_summary(SharedString::from(format!(
            "平台: {}\n剪贴板: {}\n键鼠: {}\n文件传输: {}\n{}",
            capabilities.platform,
            capabilities.clipboard,
            capabilities.input,
            capabilities.file_transfer,
            notes
        )));
    }
}

fn default_settings() -> AppSettings {
    AppSettings {
        server_url: String::new(),
        device_name: "glide-device".to_string(),
        auto_connect: false,
        clipboard_enabled: true,
        input_enabled: false,
        auth_username: String::new(),
        auth_password: String::new(),
        registration_token: String::new(),
    }
}

fn install_panic_logger() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown".to_string());
        write_diagnostic("panic", &format!("{panic_info} at {location}"));
    }));
}

fn should_run_windows_renderer_auto(args: &[String]) -> bool {
    #[cfg(target_os = "windows")]
    {
        env::var_os("SLINT_BACKEND").is_none()
            && env::var_os("GLIDE_GUI_RENDERER_CHILD").is_none()
            && args.iter().any(|arg| {
                arg == "--smoke" || arg == "--interaction-smoke" || !arg.starts_with('-')
            })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = args;
        false
    }
}

#[cfg(target_os = "windows")]
fn run_windows_renderer_auto(args: &[String]) -> Result<(), Box<dyn Error>> {
    let current_exe = env::current_exe()?;
    let child_args: Vec<String> = args.iter().skip(1).cloned().collect();

    write_diagnostic("renderer", "auto trying winit-femtovg");
    let log_start = diagnostic_log_len();
    let gpu_status = Command::new(&current_exe)
        .args(&child_args)
        .env("GLIDE_GUI_RENDERER_CHILD", "1")
        .env("SLINT_BACKEND", "winit-femtovg")
        .status()?;

    if gpu_status.success() {
        write_diagnostic("renderer", "auto selected winit-femtovg");
        return Ok(());
    }

    if !diagnostics_indicate_renderer_failure(log_start) {
        write_diagnostic(
            "renderer",
            &format!(
                "winit-femtovg failed with status {:?}; not a renderer failure",
                gpu_status.code()
            ),
        );
        process::exit(gpu_status.code().unwrap_or(1));
    }

    write_diagnostic(
        "renderer",
        "winit-femtovg failed; falling back to winit-software",
    );
    let software_status = Command::new(current_exe)
        .args(child_args)
        .env("GLIDE_GUI_RENDERER_CHILD", "1")
        .env("SLINT_BACKEND", "winit-software")
        .status()?;

    if software_status.success() {
        write_diagnostic("renderer", "auto selected winit-software");
        Ok(())
    } else {
        write_diagnostic(
            "renderer",
            &format!(
                "winit-software failed with status {:?}",
                software_status.code()
            ),
        );
        process::exit(software_status.code().unwrap_or(1));
    }
}

#[cfg(not(target_os = "windows"))]
fn run_windows_renderer_auto(_args: &[String]) -> Result<(), Box<dyn Error>> {
    unreachable!("renderer auto is only enabled on Windows")
}

#[cfg(target_os = "windows")]
fn diagnostic_log_len() -> u64 {
    fs::metadata(diagnostic_log_path())
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn diagnostics_indicate_renderer_failure(start_at: u64) -> bool {
    fs::read(diagnostic_log_path())
        .ok()
        .and_then(|bytes| {
            let start = usize::try_from(start_at).ok()?;
            Some(bytes.get(start..).unwrap_or(&[]).to_vec())
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|content| {
            content.contains("Failed to initialize OpenGL driver")
                || content.contains("Could not locate glCreateShader symbol")
        })
        .unwrap_or(false)
}

fn write_diagnostic(stage: &str, message: &str) {
    let path = diagnostic_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "{} [{}] {}",
            chrono::Utc::now().to_rfc3339(),
            stage,
            message
        );
    }
}

fn print_diagnostics() -> Result<(), Box<dyn Error>> {
    let path = diagnostic_log_path();
    println!("diagnostics={}", path.display());
    if !path.exists() {
        println!("diagnostics log does not exist yet");
        return Ok(());
    }
    println!("--- diagnostics ---");
    print!("{}", fs::read_to_string(path)?);
    Ok(())
}

fn diagnostic_log_path() -> PathBuf {
    if let Ok(path) = env::var("GLIDE_GUI_LOG") {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("Glide")
                .join("logs")
                .join("glide-gui.log");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(state_home) = env::var("XDG_STATE_HOME") {
            return PathBuf::from(state_home)
                .join("glide")
                .join("glide-gui.log");
        }
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("glide")
                .join("glide-gui.log");
        }
    }

    env::temp_dir().join("glide-gui.log")
}

#[cfg(test)]
mod tests {
    use super::diagnostic_log_path;

    #[test]
    fn diagnostics_path_uses_override() {
        std::env::set_var("GLIDE_GUI_LOG", "/tmp/glide-test.log");
        assert_eq!(
            diagnostic_log_path().to_string_lossy(),
            "/tmp/glide-test.log"
        );
        std::env::remove_var("GLIDE_GUI_LOG");
    }
}
