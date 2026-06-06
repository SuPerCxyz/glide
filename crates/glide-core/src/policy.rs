use serde::{Deserialize, Serialize};

use super::clipboard::ClipboardKind;
use super::device::DeviceId;

/// Sync policy for a specific device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePolicy {
    /// Target device ID.
    pub device_id: DeviceId,
    /// Whether sync is allowed with this device.
    pub sync_enabled: bool,
    /// Whether input sharing is allowed with this device.
    pub input_enabled: bool,
}

/// Sync policy for clipboard content types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePolicy {
    /// Clipboard type this policy applies to.
    pub kind: ClipboardKind,
    /// Whether this type is synced.
    pub sync_enabled: bool,
    /// Maximum item size in bytes for this type.
    pub max_size_bytes: Option<u64>,
}

/// Complete policy configuration for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Default behavior when no explicit rule exists.
    pub default_action: PolicyAction,
    /// Per-device overrides.
    pub device_policies: Vec<DevicePolicy>,
    /// Per-type restrictions.
    pub type_policies: Vec<TypePolicy>,
}

/// Default policy action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow sync by default.
    Allow,
    /// Deny sync by default.
    Deny,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            default_action: PolicyAction::Allow,
            device_policies: Vec::new(),
            type_policies: Vec::new(),
        }
    }
}

impl Policy {
    /// Check whether a clipboard item should be synced to a target device.
    pub fn allows_sync(&self, device_id: &DeviceId, kind: ClipboardKind) -> bool {
        // Check per-device policy first.
        for dp in &self.device_policies {
            if dp.device_id == *device_id && !dp.sync_enabled {
                return false;
            }
        }

        // Check per-type policy.
        for tp in &self.type_policies {
            if tp.kind == kind {
                return tp.sync_enabled;
            }
        }

        // Fall back to default.
        matches!(self.default_action, PolicyAction::Allow)
    }

    /// Check whether input sharing is allowed with a target device.
    pub fn allows_input(&self, device_id: &DeviceId) -> bool {
        for dp in &self.device_policies {
            if dp.device_id == *device_id {
                return dp.input_enabled;
            }
        }
        false // Input sharing defaults to denied.
    }

    /// Get the max size allowed for a clipboard kind.
    pub fn max_size_for_kind(&self, kind: ClipboardKind) -> Option<u64> {
        self.type_policies
            .iter()
            .find(|tp| tp.kind == kind)
            .and_then(|tp| tp.max_size_bytes)
    }
}
