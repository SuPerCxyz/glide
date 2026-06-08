use serde::{Deserialize, Serialize};

/// Input event for keyboard/mouse sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    /// Source device ID.
    pub source_device_id: String,
    /// Event timestamp (epoch millis).
    pub timestamp: i64,
    /// The actual input event payload.
    pub event: InputEventKind,
    /// Route used for this input event.
    pub route: InputRoute,
}

/// Types of input events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "input_type", content = "data")]
pub enum InputEventKind {
    /// Keyboard key press or release.
    Key {
        /// Key code (e.g. "A", "Enter", "Ctrl_L").
        key_code: String,
        /// True for press, false for release.
        pressed: bool,
        /// Modifier keys held during this event.
        modifiers: Vec<String>,
    },
    /// Mouse button press or release.
    MouseButton {
        /// Button identifier.
        button: String,
        /// True for press, false for release.
        pressed: bool,
        /// Cursor X position in pixels.
        x: i32,
        /// Cursor Y position in pixels.
        y: i32,
    },
    /// Mouse movement.
    MouseMove {
        /// Cursor X position in pixels.
        x: i32,
        /// Cursor Y position in pixels.
        y: i32,
        /// Relative movement X (for relative mode).
        dx: Option<i32>,
        /// Relative movement Y (for relative mode).
        dy: Option<i32>,
    },
    /// Mouse scroll wheel.
    MouseScroll {
        /// Horizontal scroll amount.
        dx: i32,
        /// Vertical scroll amount.
        dy: i32,
    },
    /// Media/multimedia key press or release.
    MediaKey {
        /// Media key identifier (e.g. "Play", "Stop", "Next", "Prev", "VolumeUp", "VolumeDown", "Mute").
        key: MediaKeyKind,
        /// True for press, false for release.
        pressed: bool,
    },
    /// Emergency release: disconnect all input sharing immediately.
    EmergencyRelease,
}

/// Multimedia key types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaKeyKind {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Mute,
    FastForward,
    Rewind,
    Eject,
    MediaSelect,
    PlayPause,
}

/// Route used for input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputRoute {
    /// Direct LAN connection for low latency.
    LanDirect,
    /// Server relay (higher latency, for remote nodes).
    ServerRelay,
}

/// Input sharing session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSession {
    /// Session identifier.
    pub session_id: String,
    /// Controller device ID.
    pub controller_id: String,
    /// Target device ID being controlled.
    pub target_id: String,
    /// Current route in use.
    pub route: InputRoute,
    /// Whether the session is active.
    pub active: bool,
    /// Last measured round-trip latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// Session start timestamp.
    pub started_at: i64,
}
