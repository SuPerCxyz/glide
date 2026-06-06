use anyhow::{anyhow, Result};
use tokio::sync::RwLock;

use crate::clipboard_adapter::ClipboardBackend;

/// Headless clipboard backend for servers and CLI-only environments.
///
/// This stores clipboard content in memory rather than interacting with
/// a display server. Useful for testing and headless Linux servers.
pub struct HeadlessClipboard {
    /// In-memory clipboard storage.
    clipboard: RwLock<HeadlessClipboardStore>,
}

#[derive(Debug, Default)]
struct HeadlessClipboardStore {
    text: Option<String>,
    image: Option<Vec<u8>>,
    files: Vec<String>,
}

impl HeadlessClipboard {
    pub fn new() -> Self {
        Self {
            clipboard: RwLock::new(HeadlessClipboardStore::default()),
        }
    }
}

impl Default for HeadlessClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ClipboardBackend for HeadlessClipboard {
    async fn read_text(&self) -> Result<String> {
        let store = self.clipboard.read().await;
        store
            .text
            .clone()
            .ok_or_else(|| anyhow!("No text on clipboard"))
    }

    async fn read_image(&self) -> Result<Vec<u8>> {
        let store = self.clipboard.read().await;
        store
            .image
            .clone()
            .ok_or_else(|| anyhow!("No image on clipboard"))
    }

    async fn read_files(&self) -> Result<Vec<String>> {
        let store = self.clipboard.read().await;
        if store.files.is_empty() {
            Err(anyhow!("No files on clipboard"))
        } else {
            Ok(store.files.clone())
        }
    }

    async fn write_text(&self, text: &str) -> Result<()> {
        let mut store = self.clipboard.write().await;
        store.text = Some(text.to_string());
        Ok(())
    }

    async fn write_image(&self, data: &[u8]) -> Result<()> {
        let mut store = self.clipboard.write().await;
        store.image = Some(data.to_vec());
        Ok(())
    }

    async fn write_files(&self, paths: &[String]) -> Result<()> {
        let mut store = self.clipboard.write().await;
        store.files = paths.to_vec();
        Ok(())
    }

    async fn available_mime_types(&self) -> Result<Vec<String>> {
        let store = self.clipboard.read().await;
        let mut types = Vec::new();
        if store.text.is_some() {
            types.push("text/plain".to_string());
        }
        if store.image.is_some() {
            types.push("image/png".to_string());
        }
        if !store.files.is_empty() {
            types.push("text/uri-list".to_string());
        }
        Ok(types)
    }

    async fn get_mime_content(&self, mime_type: &str) -> Result<Vec<u8>> {
        let store = self.clipboard.read().await;
        match mime_type {
            "text/plain" | "UTF8_STRING" => store
                .text
                .clone()
                .map(|t| t.into_bytes())
                .ok_or_else(|| anyhow!("No text")),
            "image/png" => store.image.clone().ok_or_else(|| anyhow!("No image")),
            "text/uri-list" => {
                if store.files.is_empty() {
                    Err(anyhow!("No files"))
                } else {
                    let uri = store
                        .files
                        .iter()
                        .map(|f| format!("file://{}\n", f))
                        .collect::<String>();
                    Ok(uri.into_bytes())
                }
            }
            _ => Err(anyhow!("Unsupported MIME type: {}", mime_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_headless_write_read_text() {
        let clipboard = HeadlessClipboard::new();
        clipboard.write_text("hello world").await.unwrap();
        let text = clipboard.read_text().await.unwrap();
        assert_eq!(text, "hello world");
    }

    #[tokio::test]
    async fn test_headless_write_read_image() {
        let clipboard = HeadlessClipboard::new();
        clipboard
            .write_image(&[0x89, 0x50, 0x4E, 0x47])
            .await
            .unwrap();
        let data = clipboard.read_image().await.unwrap();
        assert_eq!(data, vec![0x89, 0x50, 0x4E, 0x47]);
    }

    #[tokio::test]
    async fn test_headless_write_read_files() {
        let clipboard = HeadlessClipboard::new();
        clipboard
            .write_files(&["/tmp/a.txt".to_string(), "/tmp/b.zip".to_string()])
            .await
            .unwrap();
        let files = clipboard.read_files().await.unwrap();
        assert_eq!(files, vec!["/tmp/a.txt", "/tmp/b.zip"]);
    }

    #[tokio::test]
    async fn test_headless_empty_read_fails() {
        let clipboard = HeadlessClipboard::new();
        assert!(clipboard.read_text().await.is_err());
        assert!(clipboard.read_image().await.is_err());
        assert!(clipboard.read_files().await.is_err());
    }

    #[tokio::test]
    async fn test_headless_available_mime_types() {
        let clipboard = HeadlessClipboard::new();
        // Empty clipboard.
        let types = clipboard.available_mime_types().await.unwrap();
        assert!(types.is_empty());

        // After writing text.
        clipboard.write_text("test").await.unwrap();
        let types = clipboard.available_mime_types().await.unwrap();
        assert_eq!(types, vec!["text/plain"]);
    }

    #[tokio::test]
    async fn test_headless_get_mime_content() {
        let clipboard = HeadlessClipboard::new();
        clipboard.write_text("hello").await.unwrap();

        let data = clipboard.get_mime_content("text/plain").await.unwrap();
        assert_eq!(data, b"hello");

        let data = clipboard.get_mime_content("UTF8_STRING").await.unwrap();
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn test_headless_uri_list() {
        let clipboard = HeadlessClipboard::new();
        clipboard
            .write_files(&["/home/user/doc.pdf".to_string()])
            .await
            .unwrap();

        let data = clipboard.get_mime_content("text/uri-list").await.unwrap();
        assert!(String::from_utf8_lossy(&data).contains("file:///home/user/doc.pdf"));
    }
}
