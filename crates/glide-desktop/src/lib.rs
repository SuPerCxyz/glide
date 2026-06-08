/// Glide Desktop — desktop tray application and clipboard adapters.
///
/// This crate provides:
/// - Linux clipboard adapter (X11/Wayland/headless)
/// - Windows clipboard adapter (winapi/clipboard-win)
/// - LAN sync engine (direct peer-to-peer without server)
/// - Desktop-facing sync policy helpers used by CLI/GUI backends
/// - Monitor detection for multi-monitor input sharing
pub mod clipboard_adapter;
pub mod input_adapter;
pub mod lan_input;
pub mod lan_sync;
pub mod linux_backends;
pub mod monitor_detect;
pub mod platform_input;
pub mod policy_ui;
pub mod windows_clipboard;
#[cfg(target_os = "windows")]
pub mod windows_input;
