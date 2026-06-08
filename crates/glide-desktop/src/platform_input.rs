use std::sync::Arc;

use crate::input_adapter::InputBackend;

/// Create the default input injection backend for the current platform.
pub fn create_platform_input_backend() -> anyhow::Result<Arc<dyn InputBackend>> {
    #[cfg(target_os = "linux")]
    {
        return Ok(Arc::new(
            crate::linux_backends::linux_input::LinuxInputBackend::new(),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        return Ok(Arc::new(crate::windows_input::WindowsInputBackend::new()));
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("input injection is not implemented on this platform")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn platform_input_backend_is_available_on_supported_platforms() {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        assert!(super::create_platform_input_backend().is_ok());
    }
}
