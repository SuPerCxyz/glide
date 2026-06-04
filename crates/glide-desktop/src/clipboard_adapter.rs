use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::Digest;

use glide_core::clipboard::{ClipboardItem, ClipboardKind};
use glide_core::mime_rep::{MimeRepresentation, RepresentationContent, mime_types};
use glide_core::payload::PayloadRef;

/// Trait for platform-specific clipboard access.
#[async_trait::async_trait]
pub trait ClipboardBackend: Send + Sync {
    /// Read the current clipboard content as text.
    async fn read_text(&self) -> anyhow::Result<String>;
    /// Read the current clipboard content as an image (PNG bytes).
    async fn read_image(&self) -> anyhow::Result<Vec<u8>>;
    /// Read the current clipboard as file list.
    async fn read_files(&self) -> anyhow::Result<Vec<String>>;
    /// Write text to the clipboard.
    async fn write_text(&self, text: &str) -> anyhow::Result<()>;
    /// Write an image to the clipboard.
    async fn write_image(&self, data: &[u8]) -> anyhow::Result<()>;
    /// Write files to the clipboard.
    async fn write_files(&self, paths: &[String]) -> anyhow::Result<()>;
    /// Get available MIME types for the current clipboard content.
    async fn available_mime_types(&self) -> anyhow::Result<Vec<String>>;
    /// Get content for a specific MIME type.
    async fn get_mime_content(&self, mime_type: &str) -> anyhow::Result<Vec<u8>>;
}

/// Clipboard adapter that bridges platform clipboard with the Glide sync layer.
#[derive(Clone)]
pub struct ClipboardAdapter<B: ClipboardBackend> {
    backend: Arc<RwLock<Option<B>>>,
}

impl<B: ClipboardBackend> ClipboardAdapter<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(RwLock::new(Some(backend))),
        }
    }

    /// Capture the current clipboard and return a ClipboardItem.
    pub async fn capture(&self) -> anyhow::Result<ClipboardItem> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| anyhow::anyhow!("No clipboard backend"))?;

        let mime_types = backend.available_mime_types().await?;

        let mut representations = Vec::new();
        let mut kind = ClipboardKind::Text;
        let mut total_size: u64 = 0;

        for mt in &mime_types {
            let content_bytes = backend.get_mime_content(mt).await?;
            total_size += content_bytes.len() as u64;

            if mt.starts_with("image/") && content_bytes.len() > 1024 {
                // Large images use payload reference.
                kind = ClipboardKind::Image;
                let pid = uuid::Uuid::new_v4().to_string();
                let checksum = format!("{:x}", sha2::Sha256::digest(&content_bytes));
                let _pref = PayloadRef::new(pid.clone(), format!("/api/v1/payload/{}", pid), content_bytes.len() as u64, checksum.clone());
                representations.push(MimeRepresentation {
                    mime_type: mt.clone(),
                    content: RepresentationContent::PayloadRef(pid),
                });
                continue;
            }

            let content_str = String::from_utf8_lossy(&content_bytes);
            if mt.starts_with("text/") && content_str.len() == content_bytes.len() {
                // Text content.
                if let Ok(text) = String::from_utf8(content_bytes.clone()) {
                    representations.push(MimeRepresentation {
                        mime_type: mt.clone(),
                        content: RepresentationContent::Text(text),
                    });
                    if mt == &mime_types::TEXT_HTML.to_string() || mt == &mime_types::TEXT_RTF.to_string() {
                        kind = ClipboardKind::Text;
                    }
                    continue;
                }
            }
            // Fallback: inline base64.
            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &content_bytes);
            representations.push(MimeRepresentation {
                mime_type: mt.clone(),
                content: RepresentationContent::InlineBase64(encoded),
            });
        }

        let checksum = if !representations.is_empty() {
            match &representations[0].content {
                RepresentationContent::Text(t) => format!("{:x}", sha2::Sha256::digest(t.as_bytes())),
                RepresentationContent::InlineBase64(b) => format!("{:x}", sha2::Sha256::digest(b.as_bytes())),
                RepresentationContent::PayloadRef(pid) => pid.clone(),
            }
        } else {
            String::new()
        };

        Ok(ClipboardItem {
            item_id: uuid::Uuid::new_v4().to_string(),
            source_device_id: String::new(), // Set by caller.
            source_session_type: glide_core::clipboard::SessionType::Persistent,
            kind,
            representations,
            size: total_size,
            created_at: chrono::Utc::now().timestamp_millis(),
            payload_refs: Vec::new(),
            checksum,
            delivery_policy: glide_core::clipboard::DeliveryPolicy::default(),
        })
    }

    /// Apply a ClipboardItem to the local system clipboard.
    pub async fn apply(&self, item: &ClipboardItem) -> anyhow::Result<()> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| anyhow::anyhow!("No clipboard backend"))?;

        match item.kind {
            ClipboardKind::Text => {
                for rep in &item.representations {
                    if let RepresentationContent::Text(text) = &rep.content {
                        if rep.mime_type == mime_types::TEXT_PLAIN {
                            backend.write_text(text).await?;
                            return Ok(());
                        }
                    }
                }
            }
            ClipboardKind::Image => {
                for rep in &item.representations {
                    match &rep.content {
                        RepresentationContent::InlineBase64(data) => {
                            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)?;
                            backend.write_image(&bytes).await?;
                            return Ok(());
                        }
                        RepresentationContent::PayloadRef(_pid) => {
                            // Would need to fetch payload first.
                        }
                        _ => {}
                    }
                }
            }
            ClipboardKind::File => {
                let paths: Vec<String> = item.representations
                    .iter()
                    .filter_map(|r| match &r.content {
                        RepresentationContent::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();
                if !paths.is_empty() {
                    backend.write_files(&paths).await?;
                    return Ok(());
                }
            }
        }

        anyhow::bail!("No suitable representation found for clipboard item")
    }
}
