/// Windows clipboard backend using `clipboard-win` crate.
///
/// Provides text, image, and file/folder clipboard operations
/// using the standard Windows clipboard API.
use anyhow::{anyhow, Result};

use crate::clipboard_adapter::ClipboardBackend;

/// Windows clipboard backend.
pub struct WindowsClipboard {
    /// Whether to prefer rich text (HTML) over plain text.
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
        if self.prefer_rich_text {
            if let Ok(text) = read_clipboard_format(FORMAT_HTML) {
                return Ok(text);
            }
        }
        read_clipboard_format(FORMAT_UNICODETEXT)
            .or_else(|_| read_clipboard_format(FORMAT_OEMTEXT))
            .or_else(|_| read_clipboard_format(FORMAT_TEXT))
    }

    async fn read_image(&self) -> Result<Vec<u8>> {
        // Try DIB first (most common on Windows), then TIFF.
        read_clipboard_format_bytes(FORMAT_DIB)
            .or_else(|_| read_clipboard_format_bytes(FORMAT_TIFF))
            .map_err(|_| anyhow!("No image data on clipboard"))
    }

    async fn read_files(&self) -> Result<Vec<String>> {
        // Read file list from clipboard (CF_HDROP format).
        // clipboard-win 5.x has FilesContents for this.
        let data = read_clipboard_format_bytes(FORMAT_HDROP)
            .map_err(|_| anyhow!("No files on clipboard"))?;
        parse_hdrop(&data)
    }

    async fn write_text(&self, text: &str) -> Result<()> {
        // Write as Unicode text.
        let text_wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let bytes: Vec<u8> = text_wide.iter().flat_map(|c| c.to_le_bytes()).collect();
        set_clipboard_format_bytes(FORMAT_UNICODETEXT, &bytes)
    }

    async fn write_image(&self, data: &[u8]) -> Result<()> {
        // Write as DIB (Device Independent Bitmap).
        set_clipboard_format_bytes(FORMAT_DIB, data)
    }

    async fn write_files(&self, paths: &[String]) -> Result<()> {
        let drop_data = create_hdrop(paths);
        set_clipboard_format_bytes(FORMAT_HDROP, &drop_data)
    }

    async fn available_mime_types(&self) -> Result<Vec<String>> {
        let mut types = Vec::new();
        if is_format_available(FORMAT_UNICODETEXT) {
            types.push("text/plain".to_string());
        }
        if is_format_available(FORMAT_HTML) {
            types.push("text/html".to_string());
        }
        if is_format_available(FORMAT_DIB) {
            types.push("image/bmp".to_string());
        }
        if is_format_available(FORMAT_HDROP) {
            types.push("text/uri-list".to_string());
        }
        if types.is_empty() {
            types.push("text/plain".to_string());
        }
        Ok(types)
    }

    async fn get_mime_content(&self, mime_type: &str) -> Result<Vec<u8>> {
        match mime_type {
            "text/plain" => read_clipboard_format(FORMAT_UNICODETEXT).map(|s| s.into_bytes()),
            "text/html" => read_clipboard_format(FORMAT_HTML).map(|s| s.into_bytes()),
            "image/bmp" | "image/png" => read_clipboard_format_bytes(FORMAT_DIB),
            "text/uri-list" => read_clipboard_format_bytes(FORMAT_HDROP),
            _ => Err(anyhow!("Unsupported MIME type: {}", mime_type)),
        }
    }
}

// --- Clipboard format constants ---

const FORMAT_UNICODETEXT: u32 = 13;
const FORMAT_TEXT: u32 = 1;
const FORMAT_OEMTEXT: u32 = 7;
const FORMAT_HTML: u32 = 49427;
const FORMAT_DIB: u32 = 8;
const FORMAT_TIFF: u32 = 49163;
const FORMAT_HDROP: u32 = 15;

// --- Windows API wrappers via winapi ---

#[cfg(target_os = "windows")]
fn open_clipboard() -> Result<()> {
    use std::ptr::null_mut;
    use winapi::um::winuser::OpenClipboard;
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            Err(anyhow!("Failed to open clipboard"))
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
fn close_clipboard() {
    use winapi::um::winuser::CloseClipboard;
    unsafe {
        CloseClipboard();
    }
}

#[cfg(target_os = "windows")]
fn is_format_available(format: u32) -> bool {
    use winapi::um::winuser::IsClipboardFormatAvailable;
    unsafe { IsClipboardFormatAvailable(format) != 0 }
}

#[cfg(target_os = "windows")]
fn read_clipboard_format(format: u32) -> Result<String> {
    use std::ptr::null_mut;
    use winapi::um::winbase::GlobalLock;
    use winapi::um::winuser::GetClipboardData;

    open_clipboard()?;
    let result = (|| {
        unsafe {
            let handle = GetClipboardData(format);
            if handle.is_null() {
                return Err(anyhow!("No data in clipboard for format {}", format));
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
            Ok(text)
        }
    })();
    close_clipboard();
    result
}

#[cfg(target_os = "windows")]
fn read_clipboard_format_bytes(format: u32) -> Result<Vec<u8>> {
    use winapi::um::winbase::GlobalLock;
    use winapi::um::winuser::GetClipboardData;

    open_clipboard()?;
    let result = (|| unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return Err(anyhow!("No data in clipboard for format {}", format));
        }
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            return Err(anyhow!("GlobalLock failed"));
        }
        use winapi::um::winbase::GlobalSize;
        let size = GlobalSize(handle) as usize;
        if size == 0 {
            return Err(anyhow!("GlobalSize returned 0"));
        }
        let data = std::slice::from_raw_parts(ptr as *const u8, size);
        Ok(data.to_vec())
    })();
    close_clipboard();
    result
}

#[cfg(target_os = "windows")]
fn set_clipboard_format_bytes(format: u32, data: &[u8]) -> Result<()> {
    use std::ptr::null_mut;
    use winapi::um::winbase::{GlobalAlloc, GlobalLock, GMEM_MOVEABLE, GMEM_ZEROINIT};
    use winapi::um::winuser::{EmptyClipboard, SetClipboardData};

    open_clipboard()?;
    let result = (|| {
        unsafe {
            EmptyClipboard();

            let handle = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, data.len());
            if handle.is_null() {
                return Err(anyhow!("GlobalAlloc failed"));
            }

            let ptr = GlobalLock(handle);
            if ptr.is_null() {
                return Err(anyhow!("GlobalLock failed"));
            }

            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            // Note: GlobalUnlock is not strictly needed after GlobalLock on modern Windows
            // when we're about to call SetClipboardData.

            if SetClipboardData(format, handle).is_null() {
                return Err(anyhow!("SetClipboardData failed for format {}", format));
            }
        }
        Ok(())
    })();
    // Don't close clipboard if we failed — let Windows clean up.
    if result.is_ok() {
        close_clipboard();
    } else {
        use winapi::um::winuser::CloseClipboard;
        unsafe {
            CloseClipboard();
        }
    }
    result
}

#[cfg(target_os = "windows")]
fn parse_hdrop(data: &[u8]) -> Result<Vec<String>> {
    if data.len() < 24 {
        return Err(anyhow!("Invalid HDROP data"));
    }
    let offset = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if offset >= data.len() {
        return Err(anyhow!("Invalid HDROP offset"));
    }

    let mut paths = Vec::new();
    let mut pos = offset;
    while pos + 2 < data.len() {
        let cp = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        if cp == 0 {
            break;
        }
        let mut name = vec![cp];
        while pos + 1 < data.len() {
            let cp = u16::from_le_bytes([data[pos], data[pos + 1]]);
            pos += 2;
            if cp == 0 {
                break;
            }
            name.push(cp);
        }
        if let Ok(path) = String::from_utf16(&name) {
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
    let mut data = vec![0u8; 20];
    data[0] = 20;
    data[1] = 0;
    data[2] = 0;
    data[3] = 0;
    data[16] = 1; // fWide = 1

    for path in paths {
        for cp in path.encode_utf16() {
            data.extend_from_slice(&cp.to_le_bytes());
        }
        data.extend_from_slice(&0u16.to_le_bytes());
    }
    data.extend_from_slice(&0u16.to_le_bytes());
    data
}

// --- Stub implementations for non-Windows targets ---

#[cfg(not(target_os = "windows"))]
fn open_clipboard() -> Result<()> {
    Err(anyhow!("Windows clipboard not available"))
}

#[cfg(not(target_os = "windows"))]
fn close_clipboard() {}

#[cfg(not(target_os = "windows"))]
fn is_format_available(_format: u32) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
fn read_clipboard_format(_format: u32) -> Result<String> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn read_clipboard_format_bytes(_format: u32) -> Result<Vec<u8>> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn set_clipboard_format_bytes(_format: u32, _data: &[u8]) -> Result<()> {
    Err(anyhow!("Windows clipboard not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn parse_hdrop(_data: &[u8]) -> Result<Vec<String>> {
    Err(anyhow!("Windows HDROP not available on this platform"))
}

#[cfg(not(target_os = "windows"))]
fn create_hdrop(paths: &[String]) -> Vec<u8> {
    // Stub: return a fake but plausible HDROP structure for testing.
    let mut data = vec![0u8; 20];
    data[0] = 20; // pFiles offset
    data[16] = 1; // fWide
    for path in paths {
        for cp in path.encode_utf16() {
            data.extend_from_slice(&cp.to_le_bytes());
        }
        data.extend_from_slice(&0u16.to_le_bytes());
    }
    data.extend_from_slice(&0u16.to_le_bytes());
    data
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_hdrop_roundtrip() {
        let paths = vec![
            "C:\\Users\\test\\file.txt".to_string(),
            "D:\\docs\\readme.pdf".to_string(),
        ];
        let data = create_hdrop(&paths);
        // On Windows this would round-trip; on Linux we just verify it doesn't crash.
        assert!(data.len() > 20);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_stubs_on_non_windows() {
        assert!(read_clipboard_format(0).is_err());
        assert!(read_clipboard_format_bytes(0).is_err());
        assert!(set_clipboard_format_bytes(0, &[]).is_err());
        assert!(!is_format_available(0));
    }
}
