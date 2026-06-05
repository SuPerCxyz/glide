// Disable console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gui_backend;

use gui_backend::{AppSettings, GuiBackend, MockBackend};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use tracing::{info, warn};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    tracing_subscriber::fmt::init();

    let backend = MockBackend::new();
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

    info!("Glide GUI started");
    window.run()
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
