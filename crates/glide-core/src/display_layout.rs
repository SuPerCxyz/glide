use serde::{Deserialize, Serialize};

/// Monitor layout configuration for multi-monitor input sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLayout {
    /// Unique identifier for this layout.
    pub layout_id: String,
    /// Human-readable name for this layout.
    pub name: String,
    /// List of monitors in this layout.
    pub monitors: Vec<MonitorInfo>,
    /// Layout creation timestamp (epoch millis).
    pub created_at: i64,
    /// Last modified timestamp (epoch millis).
    pub modified_at: i64,
}

impl DisplayLayout {
    pub fn new(name: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            layout_id: uuid::Uuid::new_v4().to_string(),
            name,
            monitors: Vec::new(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Add a monitor to this layout.
    pub fn add_monitor(&mut self, monitor: MonitorInfo) {
        self.monitors.push(monitor);
        self.modified_at = chrono::Utc::now().timestamp_millis();
    }

    /// Remove a monitor by ID.
    pub fn remove_monitor(&mut self, monitor_id: &str) -> bool {
        let initial_len = self.monitors.len();
        self.monitors.retain(|m| m.monitor_id != monitor_id);
        if self.monitors.len() != initial_len {
            self.modified_at = chrono::Utc::now().timestamp_millis();
            true
        } else {
            false
        }
    }

    /// Find a monitor by ID.
    pub fn get_monitor(&self, monitor_id: &str) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.monitor_id == monitor_id)
    }

    /// Get the primary monitor.
    pub fn get_primary_monitor(&self) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.is_primary)
    }

    /// Calculate total virtual screen dimensions.
    pub fn total_bounds(&self) -> (i32, i32, i32, i32) {
        if self.monitors.is_empty() {
            return (0, 0, 0, 0);
        }

        let min_x = self.monitors.iter().map(|m| m.x_offset).min().unwrap_or(0);
        let min_y = self.monitors.iter().map(|m| m.y_offset).min().unwrap_or(0);
        let max_x = self
            .monitors
            .iter()
            .map(|m| m.x_offset + m.width)
            .max()
            .unwrap_or(0);
        let max_y = self
            .monitors
            .iter()
            .map(|m| m.y_offset + m.height)
            .max()
            .unwrap_or(0);

        (min_x, min_y, max_x, max_y)
    }

    /// Find which monitor contains the given coordinates.
    pub fn monitor_at_position(&self, x: i32, y: i32) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| {
            x >= m.x_offset
                && x < m.x_offset + m.width
                && y >= m.y_offset
                && y < m.y_offset + m.height
        })
    }

    /// Get the device ID that owns a monitor.
    pub fn device_for_monitor(&self, monitor_id: &str) -> Option<String> {
        self.get_monitor(monitor_id).map(|m| m.device_id.clone())
    }
}

/// Information about a single monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Unique identifier for this monitor.
    pub monitor_id: String,
    /// Device ID that owns this monitor.
    pub device_id: String,
    /// Human-readable name (e.g., "Dell U2415").
    pub name: String,
    /// Monitor width in pixels.
    pub width: i32,
    /// Monitor height in pixels.
    pub height: i32,
    /// X offset in virtual screen coordinates.
    pub x_offset: i32,
    /// Y offset in virtual screen coordinates.
    pub y_offset: i32,
    /// Whether this is the primary monitor.
    pub is_primary: bool,
    /// DPI scale factor (e.g., 1.0, 1.25, 1.5, 2.0).
    pub dpi_scale: f32,
    /// Refresh rate in Hz.
    pub refresh_rate: i32,
    /// Whether this monitor is currently active.
    pub is_active: bool,
}

impl MonitorInfo {
    pub fn new(
        device_id: String,
        name: String,
        width: i32,
        height: i32,
        x_offset: i32,
        y_offset: i32,
        is_primary: bool,
    ) -> Self {
        Self {
            monitor_id: uuid::Uuid::new_v4().to_string(),
            device_id,
            name,
            width,
            height,
            x_offset,
            y_offset,
            is_primary,
            dpi_scale: 1.0,
            refresh_rate: 60,
            is_active: true,
        }
    }

    /// Get the right edge X coordinate.
    pub fn right_edge(&self) -> i32 {
        self.x_offset + self.width
    }

    /// Get the bottom edge Y coordinate.
    pub fn bottom_edge(&self) -> i32 {
        self.y_offset + self.height
    }

    /// Check if a point is within this monitor.
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x_offset
            && x < self.right_edge()
            && y >= self.y_offset
            && y < self.bottom_edge()
    }

    /// Check if this monitor is adjacent to another monitor.
    pub fn is_adjacent_to(&self, other: &MonitorInfo) -> Adjacency {
        let mut adjacency = Adjacency::empty();

        // Check left edge adjacency
        if self.x_offset == other.right_edge() {
            adjacency.left = true;
        }

        // Check right edge adjacency
        if self.right_edge() == other.x_offset {
            adjacency.right = true;
        }

        // Check top edge adjacency
        if self.y_offset == other.bottom_edge() {
            adjacency.top = true;
        }

        // Check bottom edge adjacency
        if self.bottom_edge() == other.y_offset {
            adjacency.bottom = true;
        }

        adjacency
    }
}

/// Describes which edges of a monitor are adjacent to another monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adjacency {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Adjacency {
    pub fn empty() -> Self {
        Self {
            left: false,
            right: false,
            top: false,
            bottom: false,
        }
    }

    pub fn has_any(&self) -> bool {
        self.left || self.right || self.top || self.bottom
    }
}

/// Edge crossing configuration for input sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCrossingConfig {
    /// Enable automatic edge crossing.
    pub enabled: bool,
    /// Delay before crossing (milliseconds).
    pub crossing_delay_ms: u64,
    /// Threshold distance from edge to trigger crossing (pixels).
    pub edge_threshold_px: i32,
    /// Enable diagonal crossing.
    pub allow_diagonal: bool,
    /// Hotkey to manually switch monitors.
    pub switch_hotkey: Option<String>,
}

impl Default for EdgeCrossingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            crossing_delay_ms: 100,
            edge_threshold_px: 5,
            allow_diagonal: true,
            switch_hotkey: Some("Ctrl+Alt+Right".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_layout_creation() {
        let mut layout = DisplayLayout::new("Test Layout".to_string());
        assert_eq!(layout.name, "Test Layout");
        assert_eq!(layout.monitors.len(), 0);

        let monitor = MonitorInfo::new(
            "device-1".to_string(),
            "Monitor 1".to_string(),
            1920,
            1080,
            0,
            0,
            true,
        );
        layout.add_monitor(monitor);
        assert_eq!(layout.monitors.len(), 1);
    }

    #[test]
    fn test_monitor_at_position() {
        let mut layout = DisplayLayout::new("Test".to_string());
        layout.add_monitor(MonitorInfo::new(
            "device-1".to_string(),
            "Left".to_string(),
            1920,
            1080,
            0,
            0,
            true,
        ));
        layout.add_monitor(MonitorInfo::new(
            "device-2".to_string(),
            "Right".to_string(),
            1920,
            1080,
            1920,
            0,
            false,
        ));

        let monitor = layout.monitor_at_position(500, 500);
        assert!(monitor.is_some());
        assert_eq!(monitor.unwrap().name, "Left");

        let monitor = layout.monitor_at_position(2000, 500);
        assert!(monitor.is_some());
        assert_eq!(monitor.unwrap().name, "Right");

        let monitor = layout.monitor_at_position(4000, 500);
        assert!(monitor.is_none());
    }

    #[test]
    fn test_total_bounds() {
        let mut layout = DisplayLayout::new("Test".to_string());
        layout.add_monitor(MonitorInfo::new(
            "device-1".to_string(),
            "Left".to_string(),
            1920,
            1080,
            0,
            0,
            true,
        ));
        layout.add_monitor(MonitorInfo::new(
            "device-2".to_string(),
            "Right".to_string(),
            1920,
            1080,
            1920,
            0,
            false,
        ));

        let (min_x, min_y, max_x, max_y) = layout.total_bounds();
        assert_eq!(min_x, 0);
        assert_eq!(min_y, 0);
        assert_eq!(max_x, 3840);
        assert_eq!(max_y, 1080);
    }

    #[test]
    fn test_monitor_adjacency() {
        let left = MonitorInfo::new(
            "device-1".to_string(),
            "Left".to_string(),
            1920,
            1080,
            0,
            0,
            true,
        );
        let right = MonitorInfo::new(
            "device-2".to_string(),
            "Right".to_string(),
            1920,
            1080,
            1920,
            0,
            false,
        );

        let adjacency = left.is_adjacent_to(&right);
        assert!(adjacency.right);
        assert!(!adjacency.left);
        assert!(!adjacency.top);
        assert!(!adjacency.bottom);
    }
}
