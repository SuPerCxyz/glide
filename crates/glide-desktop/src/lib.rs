/// Glide Desktop — desktop tray application and clipboard adapters.
///
/// This crate provides:
/// - Linux clipboard adapter (X11/Wayland/headless)
/// - Windows clipboard adapter (winapi/clipboard-win)
/// - LAN sync engine (direct peer-to-peer without server)
/// - Tauri desktop GUI
/// - Sync policy UI

pub mod clipboard_adapter;
pub mod input_adapter;
pub mod policy_ui;
pub mod linux_backends;
pub mod windows_clipboard;
pub mod lan_sync;
