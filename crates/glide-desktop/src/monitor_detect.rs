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
    use winapi::shared::windef::{HDC, HMONITOR, RECT};
    use winapi::um::wingdi::{DEVMODEW, DM_DISPLAYFREQUENCY};
    use winapi::um::winuser::{
        EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetDC, GetDeviceCaps,
        ReleaseDC, MONITORINFOEXW, CCHDEVICENAME, CCHFORMNAME, DEVICENAMEW, DM_PELSWIDTH,
        DM_PELSHEIGHT, EDD_GET_DEVICE_INTERFACE_NAME, ENUM_CURRENT_SETTINGS, HORZRES, VERTRES,
    };

    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;

    let mut layout = DisplayLayout::new("Auto-detected Layout".to_string());

    // Use EnumDisplayMonitors to get all monitors
    let device_id_clone = device_id.to_string();
    let monitors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let monitors_clone = monitors.clone();

    unsafe {
        let _ = EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(monitor_enum_proc),
            monitors_clone.as_ref().unwrap().as_mut_ptr() as isize,
        );
    }

    // Extract monitors from the closure
    let monitor_list = monitors.lock().unwrap();

    for (i, monitor_info) in monitor_list.iter().enumerate() {
        let mut devmode: DEVMODEW = unsafe { std::mem::zeroed() };
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u32;

        let device_name: Vec<u16> = monitor_info
            .szDevice
            .iter()
            .take_while(|&&c| c != 0)
            .copied()
            .collect();
        let device_name_str = OsString::from_wide(&device_name).to_string_lossy().to_string();

        if unsafe {
            EnumDisplaySettingsW(
                device_name.as_ptr(),
                ENUM_CURRENT_SETTINGS,
                &mut devmode,
            )
        } != 0
        {
            let name = if !device_name_str.is_empty() {
                device_name_str
            } else {
                format!("Monitor {}", i + 1)
            };

            let mut monitor = MonitorInfo::new(
                device_id.to_string(),
                name,
                devmode.dmPelsWidth as i32,
                devmode.dmPelsHeight as i32,
                monitor_info.rcMonitor.left,
                monitor_info.rcMonitor.top,
                i == 0, // First monitor is primary
            );

            monitor.refresh_rate = devmode.dmDisplayFrequency as i32;
            monitor.is_active = true;

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
unsafe extern "system" fn monitor_enum_proc(
    h_monitor: HMONITOR,
    hdc_monitor: HDC,
    lprc_monitor: *mut RECT,
    dw_data: isize,
) -> i32 {
    use winapi::um::winuser::GetMonitorInfoW;
    use winapi::um::winuser::MONITORINFOEXW;

    let mut monitor_info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
    monitor_info.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(h_monitor, &mut monitor_info) != 0 {
        let monitors = &mut *(dw_data as *mut Vec<MONITORINFOEXW>);
        monitors.push(monitor_info);
    }

    1 // Continue enumeration
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
