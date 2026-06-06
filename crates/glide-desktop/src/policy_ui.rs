use glide_core::clipboard::ClipboardKind;
use glide_core::device::DeviceId;
use glide_core::policy::{DevicePolicy, Policy, TypePolicy};

/// Policy UI state for the desktop app.
#[derive(Debug, Clone)]
pub struct PolicyState {
    pub global: Policy,
    pub device_sync_enabled: bool,
    pub input_sharing_enabled: bool,
    pub pause_sync: bool,
}

impl Default for PolicyState {
    fn default() -> Self {
        Self {
            global: Policy::default(),
            device_sync_enabled: true,
            input_sharing_enabled: false, // Disabled by default.
            pause_sync: false,
        }
    }
}

impl PolicyState {
    /// Add a per-device sync policy.
    pub fn set_device_policy(
        &mut self,
        device_id: DeviceId,
        sync_enabled: bool,
        input_enabled: bool,
    ) {
        // Remove existing policy for this device.
        self.global
            .device_policies
            .retain(|dp| dp.device_id != device_id);

        self.global.device_policies.push(DevicePolicy {
            device_id,
            sync_enabled,
            input_enabled,
        });
    }

    /// Add a per-type policy.
    pub fn set_type_policy(
        &mut self,
        kind: ClipboardKind,
        sync_enabled: bool,
        max_size: Option<u64>,
    ) {
        self.global.type_policies.retain(|tp| tp.kind != kind);

        self.global.type_policies.push(TypePolicy {
            kind,
            sync_enabled,
            max_size_bytes: max_size,
        });
    }

    /// Check if sync should proceed for a given device and type.
    pub fn should_sync(&self, device_id: &DeviceId, kind: ClipboardKind) -> bool {
        if self.pause_sync {
            return false;
        }
        if !self.device_sync_enabled {
            return false;
        }
        self.global.allows_sync(device_id, kind)
    }

    /// Check if input sharing should be allowed.
    pub fn should_allow_input(&self, device_id: &DeviceId) -> bool {
        if !self.input_sharing_enabled {
            return false;
        }
        self.global.allows_input(device_id)
    }
}

/// Connection status for display in the tray.
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected { peer_count: usize },
    LanDirect { peers: Vec<String> },
    ServerFallback,
    Error { message: String },
}

impl ConnectionStatus {
    pub fn is_connected(&self) -> bool {
        matches!(
            self,
            Self::Connected { .. } | Self::LanDirect { .. } | Self::ServerFallback
        )
    }
}
