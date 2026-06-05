# GUI Migration to Slint

## Decision

Glide no longer keeps the Tauri GUI as the primary desktop client. The desktop
GUI is moving to Rust + Slint so the Windows client does not depend on WebView2
and Linux does not depend on a WebKit runtime.

## Why Slint

- It does not embed Chromium, WebView2, Electron or a web runtime.
- It integrates directly with Rust and Cargo.
- It supports Windows, Linux and macOS with one UI language.
- It gives Glide a consistent visual system instead of platform-specific native
  controls.
- It is a better fit for a small status/configuration GUI than a browser shell.

## Boundary

`glide-gui` must stay thin. It owns windows, pages, settings forms, status
display, pairing entry points, logs and permission notices. It must not directly
own network connections, clipboard listeners, input hooks, file transfer or LAN
discovery.

The first phase adds a `GuiBackend` trait with mock data. The trait mirrors the
future daemon IPC API:

- `get_service_status()`
- `start_service()` / `stop_service()`
- `list_devices()` / `get_device_detail()`
- `pair_device()`
- `connect_device()` / `disconnect_device()`
- `connect_server()` / `disconnect_server()`
- `get_clipboard_status()` / `set_clipboard_enabled()`
- `get_input_status()` / `set_input_enabled()`
- `get_file_transfer_status()` / `send_file()`
- `get_settings()` / `update_settings()`
- `tail_logs()` / `export_diagnostics()`

Windows should use Named Pipe for the real daemon IPC. Linux and macOS should
use Unix domain sockets.

## First Phase Scope

Implemented in `crates/glide-gui`:

- Status page
- Service status display
- Device list
- Connect/disconnect buttons
- Clipboard sync switch
- Input sharing switch
- Device name setting
- Pairing page
- Logs page
- Basic settings page
- About page
- Platform capability notice page

The backend is currently `MockBackend`. It is deliberately isolated from
network, clipboard and input internals so it can be replaced with daemon IPC.

## Platform Notes

Windows:

- GUI must not depend on WebView2.
- Current artifact is a portable zip containing GUI, CLI and server binaries.
- Installer work remains a later step.

Linux:

- First phase prioritizes X11.
- Wayland may start the GUI, but global input control is limited by compositor
  permissions and must be shown as limited.
- Linux packages are generated as `.deb` and `.AppImage`.

macOS:

- Architecture is reserved.
- Later work must cover Accessibility, Input Monitoring, Clipboard, Local
  Network and LaunchAgent permissions.

## Packaging Size Goal

The old offline Windows package could include a WebView2 offline installer and
grow past 200 MB. The Slint GUI should be distributed as a native executable
without WebView2. The expected first-stage artifact is the portable zip; final
size must be measured in CI and compared against the previous Tauri package.

Local Linux build result on 2026-06-05:

| Artifact | Size |
|----------|------|
| `target/release/glide-gui` | 24 MB |
| `dist-test/glide_0.1.0_amd64.deb` | 11 MB |
| `dist-test/glide-0.1.0-x86_64.AppImage` | 15 MB |

The previous Windows offline package size is not available in this workspace;
CI/release artifacts should record the old Tauri package size when comparing the
Windows portable zip.

## Follow-up Roadmap

1. Add `glide-daemon` as the long-running service.
2. Move real connection, clipboard, input and file transfer control behind
   daemon IPC.
3. Add a lightweight Windows installer without WebView2.
4. Add Linux tray integration through `glide-platform`.
5. Add macOS permission onboarding.
6. Replace mock GUI data with daemon-provided data and integration tests.
