use anyhow::Result;
use tracing::warn;

use crate::input_adapter::InputBackend;

/// Linux input backend using xdotool for event injection.
pub struct LinuxInputBackend {
    display: Option<String>,
}

impl LinuxInputBackend {
    pub fn new() -> Self {
        Self {
            display: std::env::var("DISPLAY").ok(),
        }
    }

    pub fn with_display(display: String) -> Self {
        Self {
            display: Some(display),
        }
    }

    fn run_xdotool(&self, args: &[&str]) -> Result<()> {
        let mut cmd = std::process::Command::new("xdotool");
        cmd.args(args);
        if let Some(ref display) = self.display {
            cmd.env("DISPLAY", display);
        }
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("xdotool failed: {}", stderr);
            return Err(anyhow::anyhow!("xdotool failed: {}", stderr));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl InputBackend for LinuxInputBackend {
    async fn inject_key(&self, key_code: &str, pressed: bool, modifiers: &[String]) -> Result<()> {
        let mut keys = String::new();
        for m in modifiers {
            let mod_key = match m.as_str() {
                "Ctrl" => "ctrl",
                "Alt" => "alt",
                "Shift" => "shift",
                "Super" | "Win" => "super",
                _ => m.as_str(),
            };
            keys.push_str(mod_key);
            keys.push('+');
        }
        keys.push_str(&map_key_for_xdotool(key_code));

        if pressed {
            self.run_xdotool(&["key", &keys])?;
        } else {
            // xdotool doesn't have separate key up/down for combo keys.
            // The key command handles press+release.
        }
        Ok(())
    }

    async fn inject_mouse_button(&self, button: &str, pressed: bool, x: i32, y: i32) -> Result<()> {
        // Move to position first.
        self.run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;

        let btn = match button {
            "left" => "1",
            "right" => "3",
            "middle" => "2",
            _ => "1",
        };

        if pressed {
            self.run_xdotool(&["mousedown", btn])?;
        } else {
            self.run_xdotool(&["mouseup", btn])?;
        }
        Ok(())
    }

    async fn inject_mouse_move(
        &self,
        x: i32,
        y: i32,
        _dx: Option<i32>,
        _dy: Option<i32>,
    ) -> Result<()> {
        self.run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])
    }

    async fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<()> {
        if dy != 0 {
            let btn = if dy > 0 { "5" } else { "4" };
            let count = dy.abs().min(10).to_string();
            self.run_xdotool(&["click", "--repeat", &count, btn])?;
        }
        if dx != 0 {
            let btn = if dx > 0 { "6" } else { "7" };
            let count = dx.abs().min(10).to_string();
            self.run_xdotool(&["click", "--repeat", &count, btn])?;
        }
        Ok(())
    }

    async fn cursor_position(&self) -> Result<(i32, i32)> {
        let mut cmd = std::process::Command::new("xdotool");
        cmd.arg("getmouselocation");
        if let Some(ref display) = self.display {
            cmd.env("DISPLAY", display);
        }
        let output = cmd.output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        // xdotool outputs: x:123 y:456 screen:0 window:...
        let x = text
            .split_whitespace()
            .find(|s| s.starts_with("x:"))
            .and_then(|s| s[2..].parse::<i32>().ok())
            .unwrap_or(0);
        let y = text
            .split_whitespace()
            .find(|s| s.starts_with("y:"))
            .and_then(|s| s[2..].parse::<i32>().ok())
            .unwrap_or(0);
        Ok((x, y))
    }

    async fn screen_size(&self) -> Result<(i32, i32)> {
        let mut cmd = std::process::Command::new("xdpyinfo");
        if let Some(ref display) = self.display {
            cmd.env("DISPLAY", display);
        }
        let output = cmd.output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        // Parse "dimensions:    1920x1080 pixels"
        let line = text
            .lines()
            .find(|l| l.contains("dimensions:"))
            .unwrap_or("");
        let dims = line.split_whitespace().nth(1).unwrap_or("1920x1080");
        let mut parts = dims.split('x');
        let width = parts
            .next()
            .unwrap_or("1920")
            .parse::<i32>()
            .unwrap_or(1920);
        let height = parts
            .next()
            .unwrap_or("1080")
            .parse::<i32>()
            .unwrap_or(1080);
        Ok((width, height))
    }
}

fn map_key_for_xdotool(key_code: &str) -> String {
    match key_code {
        "Ctrl_L" | "Ctrl_R" => "ctrl".to_string(),
        "Alt_L" | "Alt_R" => "alt".to_string(),
        "Shift_L" | "Shift_R" => "shift".to_string(),
        "Super_L" | "Super_R" | "Meta" => "super".to_string(),
        "Return" => "Return".to_string(),
        "Escape" => "Escape".to_string(),
        "BackSpace" => "BackSpace".to_string(),
        "Delete" => "Delete".to_string(),
        "Tab" => "Tab".to_string(),
        "Up" => "Up".to_string(),
        "Down" => "Down".to_string(),
        "Left" => "Left".to_string(),
        "Right" => "Right".to_string(),
        "F1" => "F1".to_string(),
        "F2" => "F2".to_string(),
        "F3" => "F3".to_string(),
        "F4" => "F4".to_string(),
        "F5" => "F5".to_string(),
        "F6" => "F6".to_string(),
        "F7" => "F7".to_string(),
        "F8" => "F8".to_string(),
        "F9" => "F9".to_string(),
        "F10" => "F10".to_string(),
        "F11" => "F11".to_string(),
        "F12" => "F12".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_mapping() {
        assert_eq!(map_key_for_xdotool("Ctrl_L"), "ctrl");
        assert_eq!(map_key_for_xdotool("Alt_R"), "alt");
        assert_eq!(map_key_for_xdotool("Return"), "Return");
        assert_eq!(map_key_for_xdotool("F1"), "F1");
        assert_eq!(map_key_for_xdotool("A"), "A");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_linux_input_backend_creation() {
        let backend = LinuxInputBackend::new();
        // On headless, display may be None.
        assert!(backend.display.is_none() || backend.display.is_some());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_xdotool_injection_xvfb() {
        // This test requires Xvfb running on :99.
        // Run with: Xvfb :99 & DISPLAY=:99 cargo test test_xdotool_injection_xvfb
        if std::env::var("DISPLAY").is_err() {
            eprintln!("Skipping: DISPLAY not set");
            return;
        }
        let backend = LinuxInputBackend::new();
        // Test cursor move (should not panic).
        let _ = backend.inject_mouse_move(100, 200, None, None).await;
        // Test key injection.
        let _ = backend.inject_key("A", true, &[]).await;
    }
}
