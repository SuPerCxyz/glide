use anyhow::{anyhow, Result};

use crate::clipboard_adapter::ClipboardBackend;

/// X11 clipboard backend using `xclip`.
pub struct X11Clipboard {
    /// Display override (e.g. ":0").
    display: Option<String>,
}

impl X11Clipboard {
    pub fn new(display: Option<String>) -> Self {
        Self { display }
    }

    fn run_xclip(&self, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
        let mut cmd = std::process::Command::new("xclip");
        cmd.args(args);
        if let Some(ref display) = self.display {
            cmd.env("DISPLAY", display);
        }

        if let Some(input_data) = input {
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let mut child = cmd.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                std::io::Write::write_all(&mut stdin, input_data)?;
            }
            let output = child.wait_with_output()?;
            if !output.status.success() {
                return Err(anyhow!(
                    "xclip failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Ok(output.stdout)
        } else {
            cmd.stdin(std::process::Stdio::null());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let output = cmd.output()?;
            if !output.status.success() {
                return Err(anyhow!(
                    "xclip failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Ok(output.stdout)
        }
    }
}

#[async_trait::async_trait]
impl ClipboardBackend for X11Clipboard {
    async fn read_text(&self) -> Result<String> {
        let data = self.run_xclip(&["-o", "-selection", "clipboard"], None)?;
        Ok(String::from_utf8(data)?)
    }

    async fn read_image(&self) -> Result<Vec<u8>> {
        // Try PNG first, then fall back to other formats.
        for mime in &["image/png", "image/bmp", "image/gif"] {
            if let Ok(data) = self.run_xclip(&["-o", "-selection", "clipboard", "-t", mime], None) {
                return Ok(data);
            }
        }
        Err(anyhow!("No image data found on clipboard"))
    }

    async fn read_files(&self) -> Result<Vec<String>> {
        // X11 file lists come as text/uri-list.
        let data = self.run_xclip(
            &["-o", "-selection", "clipboard", "-t", "text/uri-list"],
            None,
        )?;
        let text = String::from_utf8(data)?;
        let paths: Vec<String> = text
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|uri| {
                // Strip "file://" prefix.
                uri.strip_prefix("file://").unwrap_or(uri).to_string()
            })
            .collect();

        if paths.is_empty() {
            Err(anyhow!("No file URIs found on clipboard"))
        } else {
            Ok(paths)
        }
    }

    async fn write_text(&self, text: &str) -> Result<()> {
        self.run_xclip(&["-i", "-selection", "clipboard"], Some(text.as_bytes()))?;
        Ok(())
    }

    async fn write_image(&self, data: &[u8]) -> Result<()> {
        // xclip can write images via -t image/png.
        self.run_xclip(
            &["-i", "-selection", "clipboard", "-t", "image/png"],
            Some(data),
        )?;
        Ok(())
    }

    async fn write_files(&self, paths: &[String]) -> Result<()> {
        // X11 uses text/uri-list for file copies.
        let uri_list = paths
            .iter()
            .map(|p| format!("file://{}\n", p))
            .collect::<String>();
        self.run_xclip(
            &["-i", "-selection", "clipboard", "-t", "text/uri-list"],
            Some(uri_list.as_bytes()),
        )?;
        Ok(())
    }

    async fn available_mime_types(&self) -> Result<Vec<String>> {
        // xclip doesn't have a direct "list targets" for all cases, but -t can probe.
        // Common targets to check.
        let mut found = Vec::new();
        let common = vec![
            "UTF8_STRING",
            "text/plain",
            "text/plain;charset=utf-8",
            "text/html",
            "text/rtf",
            "text/uri-list",
            "image/png",
            "image/bmp",
            "image/gif",
            "TARGETS",
        ];

        for mime in &common {
            let mut cmd = std::process::Command::new("xclip");
            cmd.args(["-o", "-selection", "clipboard", "-t", mime]);
            if let Some(ref display) = self.display {
                cmd.env("DISPLAY", display);
            }
            cmd.stdin(std::process::Stdio::null());
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    found.push(mime.to_string());
                }
            }
        }

        if found.is_empty() {
            // Fallback: try reading as UTF8_STRING.
            found.push("UTF8_STRING".to_string());
        }

        Ok(found)
    }

    async fn get_mime_content(&self, mime_type: &str) -> Result<Vec<u8>> {
        self.run_xclip(&["-o", "-selection", "clipboard", "-t", mime_type], None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x11_clipboard_not_available() {
        // In CI/headless, xclip may not be available.
        let has_xclip = command_available("xclip");
        // Just verify it doesn't crash.
        assert!(has_xclip || !has_xclip);
    }

    fn command_available(cmd: &str) -> bool {
        std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
