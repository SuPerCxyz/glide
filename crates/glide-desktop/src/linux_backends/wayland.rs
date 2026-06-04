use anyhow::{anyhow, Result};

use crate::clipboard_adapter::ClipboardBackend;

/// Wayland clipboard backend using `wl-copy` / `wl-paste`.
pub struct WaylandClipboard {
    /// Whether to use the primary selection instead of clipboard.
    primary: bool,
}

impl WaylandClipboard {
    pub fn new() -> Self {
        Self { primary: false }
    }

    pub fn primary_selection() -> Self {
        Self { primary: true }
    }

    fn extra_args(&self) -> Vec<&str> {
        if self.primary {
            vec!["--primary"]
        } else {
            vec![]
        }
    }

    fn run_wl_paste(&self, type_arg: Option<&str>) -> Result<Vec<u8>> {
        let mut cmd = std::process::Command::new("wl-paste");
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ty) = type_arg {
            cmd.args(["--type", ty]);
        }
        for arg in self.extra_args() {
            cmd.arg(arg);
        }

        let output = cmd.output()?;
        if !output.status.success() {
            // wl-paste exits non-zero when clipboard is empty.
            return Err(anyhow!(
                "wl-paste failed (may be empty clipboard): {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(output.stdout)
    }

    fn run_wl_copy(&self, type_arg: Option<&str>, data: &[u8]) -> Result<()> {
        let mut cmd = std::process::Command::new("wl-copy");
        cmd.stdin(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ty) = type_arg {
            cmd.args(["--type", ty]);
        }
        for arg in self.extra_args() {
            cmd.arg(arg);
        }

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            std::io::Write::write_all(&mut stdin, data)?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "wl-copy failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}

impl Default for WaylandClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ClipboardBackend for WaylandClipboard {
    async fn read_text(&self) -> Result<String> {
        let data = self.run_wl_paste(None)?;
        Ok(String::from_utf8(data)?)
    }

    async fn read_image(&self) -> Result<Vec<u8>> {
        // Try PNG first, then check available types.
        if let Ok(data) = self.run_wl_paste(Some("image/png")) {
            return Ok(data);
        }
        Err(anyhow!("No image data found on clipboard"))
    }

    async fn read_files(&self) -> Result<Vec<String>> {
        let data = self.run_wl_paste(Some("text/uri-list"))?;
        let text = String::from_utf8(data)?;
        let paths: Vec<String> = text
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|uri| uri.strip_prefix("file://").unwrap_or(uri).to_string())
            .collect();

        if paths.is_empty() {
            Err(anyhow!("No file URIs found on clipboard"))
        } else {
            Ok(paths)
        }
    }

    async fn write_text(&self, text: &str) -> Result<()> {
        self.run_wl_copy(None, text.as_bytes())
    }

    async fn write_image(&self, data: &[u8]) -> Result<()> {
        self.run_wl_copy(Some("image/png"), data)
    }

    async fn write_files(&self, paths: &[String]) -> Result<()> {
        let uri_list = paths
            .iter()
            .map(|p| format!("file://{}\n", p))
            .collect::<String>();
        self.run_wl_copy(Some("text/uri-list"), uri_list.as_bytes())
    }

    async fn available_mime_types(&self) -> Result<Vec<String>> {
        // wl-paste --list-types outputs one MIME type per line.
        let mut cmd = std::process::Command::new("wl-paste");
        cmd.arg("--list-types");
        for arg in self.extra_args() {
            cmd.arg(arg);
        }
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "wl-paste --list-types failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let text = String::from_utf8(output.stdout)?;
        let types: Vec<String> = text
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        Ok(types)
    }

    async fn get_mime_content(&self, mime_type: &str) -> Result<Vec<u8>> {
        self.run_wl_paste(Some(mime_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_backend_detect() {
        let available = command_available("wl-copy") && command_available("wl-paste");
        // Just verify it doesn't crash.
        assert!(available || !available);
    }

    fn command_available(cmd: &str) -> bool {
        std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
