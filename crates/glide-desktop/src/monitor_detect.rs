/// Monitor detection and management for multi-monitor input sharing.
///
/// This module provides platform-specific monitor detection:
/// - Linux: Uses xrandr for X11, limited support for Wayland
/// - Windows: Uses EnumDisplayDevices and EnumDisplayMonitors APIs

use anyhow::Result;
use glide_core::display_layout::{DisplayLayout, MonitorInfo};

/// Detect the current system's monitor layout.
pub fn detect_monitor_layout(device_id: &str) -> Result<DisplayLayout> {
    #[cfg(target_os = "linux")]
    {
        return detect_linux_monitors(device_id);
    }

    #[cfg(target_os = "windows")]
    {
        return detect_windows_monitors(device_id);
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("Monitor detection is not supported on this platform")
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_monitors(device_id: &str) -> Result<DisplayLayout> {
    use std::process::Command;

    let mut layout = DisplayLayout::new("Auto-detected Layout".to_string());

    // Try xrandr first (X11)
    let output = Command::new("xrandr")
        .arg("--query")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run xrandr: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("xrandr failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse xrandr output to find connected monitors
    let mut current_monitor: Option<MonitorInfo> = None;
    let mut x_offset = 0;

    for line in stdout.lines() {
        // Look for connected monitors (lines with "connected" and resolution)
        if line.contains(" connected") && line.contains("x") {
            // Parse monitor name and resolution
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];

                // Find resolution (format: WxH+X+Y or WxH)
                for part in parts {
                    if part.contains('x') && part.contains('+') {
                        // Format: 1920x1080+0+0
                        let res_parts: Vec<&str> = part.split('+').collect();
                        if res_parts.len() >= 3 {
                            let dims: Vec<&str> = res_parts[0].split('x').collect();
                            if dims.len() == 2 {
                                let width = dims[0].parse::<i32>().unwrap_or(1920);
                                let height = dims[1].parse::<i32>().unwrap_or(1080);
                                let x = res_parts[1].parse::<i32>().unwrap_or(x_offset);
                                let y = res_parts[2].parse::<i32>().unwrap_or(0);

                                let is_primary = line.contains("primary");
                                let mut monitor = MonitorInfo::new(
                                    device_id.to_string(),
                                    name.to_string(),
                                    width,
                                    height,
                                    x,
                                    y,
                                    is_primary,
                                );
                                monitor.is_active = true;

                                current_monitor = Some(monitor);
                                x_offset = x + width;
                                break;
                            }
                        }
                    }
                }
            }
        } else if line.contains(" connected") && current_monitor.is_none() {
            // Connected but no resolution info yet
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0];
                current_monitor = Some(MonitorInfo::new(
                    device_id.to_string(),
                    name.to_string(),
                    1920,
                    1080,
                    x_offset,
                    0,
                    false,
                ));
            }
        }

        // Add monitor if we have one
        if let Some(monitor) = current_monitor.take() {
            layout.add_monitor(monitor);
        }
    }

    // If no monitors detected, create a default one
    if layout.monitors.is_empty() {
        layout.add_monitor(MonitorInfo::new(
            device_id.to_string(),
            "Default Monitor".to_string(),
            1920,
            1080,
            0,
            0,
            true,
        ));
    }

    Ok(layout)
}

#[cfg(target_os = "windows")]
fn detect_windows_monitors(device_id: &str) -> Result<DisplayLayout> {
    let mut layout = DisplayLayout::new("Auto-detected Layout".to_string());

    unsafe {
        use winapi::um::winuser::{EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO, MONITORINFOEXW};
        use winapi::shared::windef::{HMONITOR, HDC, RECT};
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::ptr;

        struct Monitors(Vec<MONITORINFOEXW>);

        let monitors = std::sync::Mutex::new(Monitors(Vec::new()));

        unsafe extern "system" fn enum_proc(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _lprc: *mut RECT,
            dw_data: isize,
        ) -> i32 {
            let monitors = &mut *(dw_data as *mut std::sync::Mutex<Monitors>);
            let mut info: MONITORINFOEXW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            // Cast MONITORINFOEXW* to MONITORINFO* as GetMonitorInfoW expects the base type
            let info_ptr = &mut info as *mut MONITORINFOEXW as *mut MONITORINFO;
            if GetMonitorInfoW(hmonitor, info_ptr) != 0 {
                if let Ok(mut m) = monitors.lock() {
                    m.0.push(info);
                }
            }
            1
        }

        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(enum_proc),
            &monitors as *const _ as isize,
        );

        let info_list = monitors.into_inner().unwrap_or_else(|_| Monitors(Vec::new()));
        let mut index = 0;

        for info in &info_list.0 {
            let name_wide: Vec<u16> = info.szDevice.iter()
                .take_while(|&&c| c != 0).copied().collect();
            let name = OsString::from_wide(&name_wide)
                .to_string_lossy().to_string();
            let width = info.rcMonitor.right - info.rcMonitor.left;
            let height = info.rcMonitor.bottom - info.rcMonitor.top;

            let mut monitor = MonitorInfo::new(
                device_id.to_string(),
                if name.is_empty() { format!("Monitor {}", index + 1) } else { name },
                width,
                height,
                info.rcMonitor.left,
                info.rcMonitor.top,
                index == 0,
            );
            monitor.is_active = true;
            layout.add_monitor(monitor);
            index += 1;
        }
    }

    if layout.monitors.is_empty() {
        layout.add_monitor(MonitorInfo::new(
            device_id.to_string(),
            "Default Monitor".to_string(),
            1920, 1080, 0, 0, true,
        ));
    }

    Ok(layout)
}

/// Save a display layout to a JSON file.
pub fn save_display_layout(layout: &DisplayLayout, path: &std::path::Path) -> Result<()> {
    let json = serde_json::to_string_pretty(layout)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a display layout from a JSON file.
pub fn load_display_layout(path: &std::path::Path) -> Result<DisplayLayout> {
    let json = std::fs::read_to_string(path)?;
    let layout: DisplayLayout = serde_json::from_str(&json)?;
    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_monitor_layout() {
        // This test may fail in headless environments
        match detect_monitor_layout("test-device") {
            Ok(layout) => {
                assert!(!layout.monitors.is_empty());
                println!("Detected {} monitor(s)", layout.monitors.len());
                for monitor in &layout.monitors {
                    println!(
                        "  {} ({}x{} at {},{})",
                        monitor.name,
                        monitor.width,
                        monitor.height,
                        monitor.x_offset,
                        monitor.y_offset
                    );
                }
            }
            Err(e) => {
                println!("Monitor detection failed (expected in headless): {}", e);
            }
        }
    }

    #[test]
    fn test_save_load_layout() {
        let mut layout = DisplayLayout::new("Test Layout".to_string());
        layout.add_monitor(MonitorInfo::new(
            "device-1".to_string(),
            "Monitor 1".to_string(),
            1920,
            1080,
            0,
            0,
            true,
        ));

        let temp_path = std::env::temp_dir().join("test_layout.json");
        assert!(save_display_layout(&layout, &temp_path).is_ok());
        assert!(temp_path.exists());

        let loaded = load_display_layout(&temp_path);
        assert!(loaded.is_ok());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, "Test Layout");
        assert_eq!(loaded.monitors.len(), 1);

        std::fs::remove_file(temp_path).ok();
    }
}
