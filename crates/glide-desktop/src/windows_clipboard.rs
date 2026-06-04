/// Windows clipboard backend using native Windows API.
///
/// Uses `winapi` for clipboard access and `clipboard-win` for high-level operations.
/// Only compiled on Windows targets via cfg gating.

use anyhow::{anyhow, Result};

use crate::clipboard_adapter::ClipboardBackend;

/// Windows clipboard backend.
pub struct WindowsClipboard {
    /// Whether to use rich text as HTML.
    prefer_rich_text: bool,
}

impl WindowsClipboard {
    pub fn new() -> Self {
        Self {
            prefer_rich_text: true,
        }
    }

    pub fn with_rich_text(mut self, enabled: bool) -> Self {
        self.prefer_rich_text = enabled;
        self
    }
}

impl Default for WindowsClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ClipboardBackend for WindowsClipboard {
    async fn read_text(&self) -> Result<String> {
        // Try rich text (HTML) first if preferred, then plain text.
        if self.prefer_rich_text {
            if let Ok(data) = get_clipboard_format(CF_HTML) {
                // Strip HTML context to get the content.
                return Ok(strip_html_context(&data));
            }
        }
        let data = get_clipboard_format(CF_UNICODETEXT)?;
        Ok(data)
    }

    async fn read_image(&self) -> Result<Vec<u8>> {
        // Try PNG first, then DIB (Device Independent Bitmap).
        if let Ok(data) = get_clipboard_format_by_id(CF_DIB) {
            // DIB is a raw bitmap format. For PNG we'd need to convert.
            return Ok(data);
        }
        Err(anyhow!("No image data on clipboard"))
    }

    async fn read_files(&self) -> Result<Vec<String>> {
        get_clipboard_format_by_id(CF_HDROP)
            .and_then(|data| parse_hdrop(&data))
            .map_err(|_| anyhow!("No file URIs on clipboard"))
    }

    async fn write_text(&self, text: &str) -> Result<()> {
        set_clipboard_text(text)
    }

    async fn write_image(&self, data: &[u8]) -> Result<()> {
        // Write as DIB (bitmap) to clipboard.
        set_clipboard_data(CF_DIB, data)
    }

    async fn write_files(&self, paths: &[String]) -> Result<()> {
        // Windows uses CF_HDROP for file copies.
        let drop_data = create_hdrop(paths);
        set_clipboard_data(CF_HDROP, &drop_data)
    }

    async fn available_mime_types(&self) -> Result<Vec<String>> {
        let mut types = Vec::new();

        if is_format_available(CF_UNICODETEXT) {
            types.push("text/plain".to_string());
            types.push("text/plain;charset=utf-16".to_string());
        }
        if is_format_available(CF_HTML) {
            types.push("text/html".to_string());
        }
        if is_format_available(CF_DIB) {
            types.push("image/bmp".to_string());
            types.push("image/png".to_string());
        }
        if is_format_available(CF_HDROP) {
            types.push("text/uri-list".to_string());
        }

        if types.is_empty() {
            types.push("text/plain".to_string());
        }

        Ok(types)
    }

    async fn get_mime_content(&self, mime_type: &str) -> Result<Vec<u8>> {
        match mime_type {
            "text/plain" | "text/plain;charset=utf-16" => {
                get_clipboard_format(CF_UNICODETEXT).map(|s| s.into_bytes())
            }
            "text/html" => {
                get_clipboard_format(CF_HTML).map(|s| s.into_bytes())
            }
            "image/bmp" | "image/png" => {
                get_clipboard_format_by_id(CF_DIB)
            }
            "text/uri-list" => {
                get_clipboard_format_by_id(CF_HDROP)
            }
            _ => Err(anyhow!("Unsupported MIME type: {}", mime_type)),
        }
    }
}

// --- Windows Clipboard Format Constants ---

/// Unicode text format.
const CF_UNICODETEXT: u32 = 13;
/// HTML format.
const CF_HTML: u32 = 49427; // Registered format "HTML Format"
/// Device Independent Bitmap.
const CF_DIB: u32 = 8;
/// File drop list.
const CF_HDROP: u32 = 15;

// --- Windows API wrappers (via winapi) ---

#[cfg(target_os = "windows")]
fn open_clipboard() -> Result<()> {
    use winapi::um::winuser::{OpenClipboard, CloseClipboard};
    use std::ptr::null_mut;

    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            Err(anyhow!("Failed to open clipboard (error: {})", get_last_error()))
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
fn close_clipboard() {
    use winapi::um::winuser::CloseClipboard;
    unsafe { CloseClipboard(); }
}

#[cfg(target_os = "windows")]
fn get_last_error() -> u32 {
    unsafe { winapi::um::errhandlingapi::GetLastError() }
}

#[cfg(target_os = "windows")]
fn get_clipboard_text(format: u32) -> Result<String> {
    use winapi::um::winuser::{GetClipboardData, IsClipboardFormatAvailable};
    use winapi::um::winbase::{GlobalLock, GlobalUnlock};

    unsafe {
        if IsClipboardFormatAvailable(format) == 0 {
            return Err(anyhow!("Format not available"));
        }

        let handle = GetClipboardData(format);
        if handle.is_null() {
            return Err(anyhow!("GetClipboardData failed"));
        }

        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            return Err(anyhow!("GlobalLock failed"));
        }

        // Read until null terminator.
        let mut len = 0;
        while *ptr.offset(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len as usize);
        let text = String::from_utf16_lossy(slice);

        GlobalUnlock(handle);

        Ok(text)
    }
}

#[cfg(target_os = "windows")]
fn set_clipboard_text(text: &str) -> Result<()> {
    use winapi::um::winuser::{SetClipboardData, EmptyClipboard};
    use winapi::um::winbase::{GlobalAlloc, GlobalLock, GMEM_MOVEABLE};
    use std::ptr::null_mut;

    let utf16: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let size = utf16.len() * std::mem::size_of::<u16>();

    unsafe {
        EmptyClipboard();

        let handle = GlobalAlloc(GMEM_MOVEABLE, size);
        if handle.is_null() {
            return Err(anyhow!("GlobalAlloc failed"));
        }

        let ptr = GlobalLock(handle) as *mut u16;
        if ptr.is_null() {
            return Err(anyhow!("GlobalLock failed"));
        }

        std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
        GlobalUnlock(handle);

        if SetClipboardData(CF_UNICODETEXT, handle).is_null() {
            return Err(anyhow!("SetClipboardData failed"));
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn get_clipboard_format(format: u32) -> Result<String> {
    open_clipboard()?;
    let result = get_clipboard_text(format);
    close_clipboard();
    result
}

#[cfg(target_os = "windows")]
fn get_clipboard_format_by_id(format: u32) -> Result<Vec<u8>> {
    use winapi::um::winuser::{GetClipboardData, IsClipboardFormatAvailable};
    use winapi::um::winbase::{GlobalLock, GlobalSize};

    open_clipboard()?;

    unsafe {
        if IsClipboardFormatAvailable(format) == 0 {
            close_clipboard();
            return Err(anyhow!("Format not available"));
        }

        let handle = GetClipboardData(format);
        if handle.is_null() {
            close_clipboard();
            return Err(anyhow!("GetClipboardData failed"));
        }

        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            close_clipboard();
            return Err(anyhow!("GlobalLock failed"));
        }

        let size = GlobalSize(handle) as usize;
        let data = std::slice::from_raw_parts(ptr as *const u8, size);
        let result = data.to_vec();

        GlobalUnlock(handle);
    }

    close_clipboard();
    Ok(result)
}

#[cfg(target_os = "windows")]
fn set_clipboard_data(format: u32, data: &[u8]) -> Result<()> {
    use winapi::um::winuser::{SetClipboardData, EmptyClipboard};
    use winapi::um::winbase::{GlobalAlloc, GlobalLock, GMEM_MOVEABLE};
    use std::ptr::null_mut;

    open_clipboard()?;

    unsafe {
        EmptyClipboard();

        let handle = GlobalAlloc(GMEM_MOVEABLE, data.len());
        if handle.is_null() {
            close_clipboard();
            return Err(anyhow!("GlobalAlloc failed"));
        }

        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            close_clipboard();
            return Err(anyhow!("GlobalLock failed"));
        }

        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        GlobalUnlock(handle);

        if SetClipboardData(format, handle).is_null() {
            close_clipboard();
            return Err(anyhow!("SetClipboardData failed"));
        }
    }

    close_clipboard();
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_format_available(format: u32) -> bool {
    use winapi::um::winuser::IsClipboardFormatAvailable;
    unsafe { IsClipboardFormatAvailable(format) != 0 }
}

#[cfg(target_os = "windows")]
fn parse_hdrop(data: &[u8]) -> Result<Vec<String>> {
    // HDROP is a DROPFILES structure followed by null-terminated paths.
    use winapi::um::shellapi::{DROPFILESW, DragQueryFileW};
    use std::ptr::null_mut;

    // The first 20 bytes are the DROPFILES header.
    if data.len() < 20 {
        return Err(anyhow!("Invalid HDROP data"));
    }

    // Get file count.
    let file_count = unsafe { DragQueryFileW(null_mut(), 0xFFFFFFFF, null_mut(), 0) };

    let mut paths = Vec::new();
    for i in 0..file_count {
        let mut buf = [0u16; 4096];
        let len = unsafe { DragQueryFileW(null_mut(), i, buf.as_mut_ptr(), buf.len() as u32) };
        if len > 0 {
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            paths.push(path);
        }
    }

    if paths.is_empty() {
        Err(anyhow!("No files in HDROP"))
    } else {
        Ok(paths)
    }
}

#[cfg(target_os = "windows")]
fn create_hdrop(paths: &[String]) -> Vec<u8> {
    use winapi::um::shellapi::DROPFILESW;
    use std::mem;

    // DROPFILES structure:
    // - pFiles: offset to file list (20 = sizeof(DROPFILESW))
    // - pt: POINT (8 bytes, unused)
    // - fNC: bool (4 bytes, unused)
    // - fWide: bool (4 bytes, true for Unicode)

    let mut data = vec![0u8; mem::size_of::<DROPFILESW>()];
    let header = unsafe { &mut *(data.as_mut_ptr() as *mut DROPFILESW) };
    header.pFiles = mem::size_of::<DROPFILESW>() as u32;
    header.fWide = 1; // Unicode

    // Append null-terminated file paths, double-null at end.
    for path in paths {
        let utf16: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let bytes = unsafe {
            std::slice::from_raw_parts(
                utf16.as_ptr() as *const u8,
                utf16.len() * mem::size_of::<u16>(),
            )
        };
        data.extend_from_slice(bytes);
    }
    // Double null terminator.
    data.extend_from_slice(&[0, 0]);

    data
}

// --- Stub implementations for non-Windows targets (compile only) ---

#[cfg(not(target_os = "windows"))]
fn get_clipboard_format(_format: u32) -> Result<String> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn get_clipboard_text(_format: u32) -> Result<String> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn get_clipboard_format_by_id(_format: u32) -> Result<Vec<u8>> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn set_clipboard_text(_text: &str) -> Result<()> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn set_clipboard_data(_format: u32, _data: &[u8]) -> Result<()> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn is_format_available(_format: u32) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
fn parse_hdrop(_data: &[u8]) -> Result<Vec<String>> {
    Err(anyhow!("Windows HDROP not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn create_hdrop(_paths: &[String]) -> Vec<u8> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
fn open_clipboard() -> Result<()> {
    Err(anyhow!("Windows clipboard not available"))
}

#[cfg(not(target_os = "windows"))]
fn close_clipboard() {}

#[cfg(not(target_os = "windows"))]
fn get_last_error() -> u32 {
    0
}

/// Strip HTML context wrapper from CF_HTML data.
fn strip_html_context(html: &str) -> String {
    // CF_HTML format wraps HTML in a context header.
    // We look for the actual HTML content after the header.
    if let Some(start) = html.find("<html") {
        html[start..].to_string()
    } else {
        html.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_context() {
        let html = "Version:0.9\r\nStartHTML:0000000105\r\nEndHTML:0000000200\r\n<html><body><b>bold</b></body></html>";
        let result = strip_html_context(html);
        assert!(result.starts_with("<html"));
        assert!(result.contains("<b>bold</b>"));
    }

    #[test]
    fn test_strip_no_html_context() {
        let html = "<html><body>plain</body></html>";
        let result = strip_html_context(html);
        assert_eq!(result, html);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_windows_clipboard_stubs_on_non_windows() {
        assert!(get_clipboard_text(0).is_err());
        assert!(get_clipboard_format_by_id(0).is_err());
        assert!(set_clipboard_text("test").is_err());
        assert!(!is_format_available(0));
    }
}
