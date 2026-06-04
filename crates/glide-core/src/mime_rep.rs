use serde::{Deserialize, Serialize};

/// A single MIME representation of clipboard content.
///
/// Rich text, URLs, and colors are internally represented as text variants,
/// not exposed as separate user-visible clipboard types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimeRepresentation {
    /// MIME type string (e.g. "text/plain", "image/png", "text/uri-list").
    pub mime_type: String,
    /// The actual content bytes or text.
    pub content: RepresentationContent,
}

/// Content of a MIME representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepresentationContent {
    /// Inline text content. Used for plain text, HTML, RTF, URLs, colors.
    Text(String),
    /// Inline base64-encoded binary (small images).
    InlineBase64(String),
    /// Reference to a payload file (large images, files, folders).
    PayloadRef(String),
}

/// Well-known MIME type constants.
pub mod mime_types {
    pub const TEXT_PLAIN: &str = "text/plain";
    pub const TEXT_HTML: &str = "text/html";
    pub const TEXT_RTF: &str = "text/rtf";
    pub const TEXT_URI_LIST: &str = "text/uri-list";
    pub const IMAGE_PNG: &str = "image/png";
    pub const IMAGE_JPEG: &str = "image/jpeg";
    pub const IMAGE_GIF: &str = "image/gif";
    pub const FILE_URI_LIST: &str = "text/uri-list";
}
