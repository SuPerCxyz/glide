use thiserror::Error;

/// Top-level error type for glide-core operations.
#[derive(Debug, Error)]
pub enum GlideError {
    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("device not trusted: {0}")]
    DeviceNotTrusted(String),

    #[error("invalid token: {0}")]
    InvalidToken(String),

    #[error("token expired")]
    TokenExpired,

    #[error("token max uses exceeded")]
    TokenMaxUsesExceeded,

    #[error("operation not allowed with this token: {0}")]
    TokenOperationNotAllowed(String),

    #[error("item too large: {size} bytes exceeds limit of {limit}")]
    ItemTooLarge { size: u64, limit: u64 },

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("payload not found: {0}")]
    PayloadNotFound(String),

    #[error("transfer failed: {0}")]
    TransferFailed(String),

    #[error("connection error: {0}")]
    ConnectionError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("input relay disabled")]
    InputRelayDisabled,

    #[error("input relay latency too high: {0}ms")]
    InputRelayLatencyHigh(u64),

    #[error("rate limit exceeded")]
    RateLimitExceeded,
}
