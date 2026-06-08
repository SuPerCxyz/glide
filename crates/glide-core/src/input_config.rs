/// Per-device input configuration for keyboard/mouse sharing.
use serde::{Deserialize, Serialize};

/// Mouse speed and behavior configuration for a specific device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseConfig {
    /// Device ID this config applies to.
    pub device_id: String,
    /// Mouse speed multiplier (0.1 to 5.0, default 1.0).
    pub speed_multiplier: f64,
    /// Whether to reverse horizontal scroll direction.
    pub reverse_horizontal_scroll: bool,
    /// Whether to reverse vertical scroll direction.
    pub reverse_vertical_scroll: bool,
    /// Mouse acceleration (0 = off, 1 = on).
    pub acceleration_enabled: bool,
    /// Swap left and right mouse buttons.
    pub swap_buttons: bool,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            speed_multiplier: 1.0,
            reverse_horizontal_scroll: false,
            reverse_vertical_scroll: false,
            acceleration_enabled: true,
            swap_buttons: false,
        }
    }
}

/// Complete input device configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDeviceConfig {
    pub mouse: MouseConfig,
    /// Preferred keyboard layout (if applicable).
    pub keyboard_layout: Option<String>,
    /// Custom key mappings (source -> target).
    pub key_mappings: std::collections::HashMap<String, String>,
}

impl Default for InputDeviceConfig {
    fn default() -> Self {
        Self {
            mouse: MouseConfig::default(),
            keyboard_layout: None,
            key_mappings: std::collections::HashMap::new(),
        }
    }
}

impl InputDeviceConfig {
    /// Apply mouse speed multiplier to a coordinate delta.
    pub fn apply_mouse_speed(&self, dx: i32, dy: i32) -> (i32, i32) {
        let factor = self.mouse.speed_multiplier;
        (
            (dx as f64 * factor) as i32,
            (dy as f64 * factor) as i32,
        )
    }

    /// Apply scroll direction configuration to a scroll delta.
    pub fn apply_scroll_direction(&self, dx: i32, dy: i32) -> (i32, i32) {
        (
            if self.mouse.reverse_horizontal_scroll { -dx } else { dx },
            if self.mouse.reverse_vertical_scroll { -dy } else { dy },
        )
    }

    /// Apply button swap configuration.
    pub fn apply_button_swap(&self, button: &str) -> String {
        if self.mouse.swap_buttons {
            match button {
                "left" => "right".to_string(),
                "right" => "left".to_string(),
                other => other.to_string(),
            }
        } else {
            button.to_string()
        }
    }
}
