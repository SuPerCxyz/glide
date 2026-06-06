mod headless;
pub mod linux_input;
mod wayland;
/// Linux clipboard backends using system CLI tools.
///
/// Supports:
/// - X11 via `xclip`
/// - Wayland via `wl-copy` / `wl-paste`
/// - Fallback via `xsel`
/// - Headless fallback (no clipboard, log only)
mod x11;

pub use headless::HeadlessClipboard;
pub use wayland::WaylandClipboard;
pub use x11::X11Clipboard;

/// Detect which clipboard backend is available on this system.
pub fn detect_backend() -> BackendType {
    if command_available("wl-copy") && command_available("wl-paste") {
        return BackendType::Wayland;
    }
    if command_available("xclip") {
        return BackendType::X11;
    }
    if command_available("xsel") {
        return BackendType::Xsel;
    }
    BackendType::Headless
}

/// Available clipboard backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Wayland via wl-clipboard (wl-copy/wl-paste).
    Wayland,
    /// X11 via xclip.
    X11,
    /// X11 via xsel (fallback).
    Xsel,
    /// No graphical clipboard available (headless).
    Headless,
}

fn command_available(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
