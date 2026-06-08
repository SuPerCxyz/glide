// Disable console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gui_backend;

use gui_backend::{AppSettings, GuiBackend, MockBackend};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{info, warn};

slint::include_modules!();

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

fn run_app() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("glide-gui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    install_panic_logger();
    let _ = tracing_subscriber::fmt::try_init();
    write_diagnostic("process", "glide-gui starting");

    if args.iter().any(|arg| arg == "--diagnostics-path") {
        println!("{}", diagnostic_log_path().display());
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--diagnostics") {
        print_diagnostics()?;
        return Ok(());
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

fn run_gui() -> Result<(), Box<dyn Error>> {
    let backend = MockBackend::new();
    let window = create_window(&backend)?;

    info!("Glide GUI started");
    write_diagnostic("process", "glide-gui window running");
    window.run()?;
    Ok(())
}

fn run_smoke() -> Result<(), Box<dyn Error>> {
    let backend = MockBackend::new();
    let _window = create_window(&backend)?;

    let log_path = diagnostic_log_path();
    let status = backend.get_service_status().data;
    let settings = backend.get_settings().data.unwrap_or_else(default_settings);
    let devices = backend.list_devices().data.unwrap_or_default();

    write_diagnostic(
        "smoke",
        &format!(
            "ok version={} os={} arch={} devices={} server_url={}",
            env!("CARGO_PKG_VERSION"),
            env::consts::OS,
            env::consts::ARCH,
            devices.len(),
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
    let backend = MockBackend::new();
    let window = create_window(&backend)?;

    window.invoke_toggle_clipboard();
    window.invoke_toggle_input();
    window.invoke_connect();
    window.invoke_page_changed(3);
    window.invoke_pair_device();
    window.invoke_save_server(SharedString::from("http://192.0.2.10:8080"));
    window.invoke_save_name(SharedString::from("glide-smoke"));

    let status = backend.get_service_status().data.unwrap_or_else(|| {
        panic!("mock backend did not return service status during interaction smoke")
    });
    let settings = backend
        .get_settings()
        .data
        .unwrap_or_else(|| panic!("mock backend did not return settings during interaction smoke"));
    let logs = backend.tail_logs(20).data.unwrap_or_default();

    if status.connection_status != "已连接" {
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
    if !logs.iter().any(|line| line.contains("Pairing requested")) {
        return Err("interaction smoke failed: pair callback did not write log".into());
    }

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

    refresh_window(&window, &backend);

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
            refresh_window(&win, &backend);
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
            refresh_window(&win, &backend);
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
            let result = backend.connect_server(&url);
            if !result.success {
                warn!("connect failed: {:?}", result.error);
            }
            refresh_window(&win, &backend);
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
            refresh_window(&win, &backend);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_save_server(move |url| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let mut settings = backend.get_settings().data.unwrap_or_else(default_settings);
            settings.server_url = url.to_string();
            let result = backend.update_settings(&settings);
            if !result.success {
                warn!("failed to save server url: {:?}", result.error);
            }
            refresh_window(&win, &backend);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_save_name(move |name| {
            let Some(win) = win.upgrade() else {
                return;
            };
            let mut settings = backend.get_settings().data.unwrap_or_else(default_settings);
            settings.device_name = name.to_string();
            let result = backend.update_settings(&settings);
            if !result.success {
                warn!("failed to save device name: {:?}", result.error);
            }
            refresh_window(&win, &backend);
        });
    }

    {
        let backend = backend.clone();
        let win = window.as_weak();
        window.on_pair_device(move || {
            let Some(win) = win.upgrade() else {
                return;
            };
            let result = backend.pair_device();
            if !result.success {
                warn!("pairing failed: {:?}", result.error);
            }
            refresh_window(&win, &backend);
        });
    }

    Ok(window)
}

fn refresh_window(window: &MainWindow, backend: &MockBackend) {
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
    }

    if let Some(devices) = backend.list_devices().data {
        let rows: Vec<DeviceRow> = devices
            .into_iter()
            .map(|device| DeviceRow {
                name: SharedString::from(device.name),
                platform: SharedString::from(device.platform),
                status: SharedString::from(if device.online { "在线" } else { "离线" }),
            })
            .collect();
        window.set_devices(ModelRc::from(Rc::new(VecModel::from(rows))));
    }

    if let Some(logs) = backend.tail_logs(200).data {
        window.set_log_content(SharedString::from(logs.join("\n")));
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
