use std::net::SocketAddr;
use std::time::Instant;

use crate::discovery::PeerRegistry;
use crate::input_event::InputRoute;
use crate::transfer::TransferRoute;

/// Result of a route selection for clipboard transfer.
#[derive(Debug, Clone)]
pub struct ClipboardRouteResult {
    /// Selected route.
    pub route: TransferRoute,
    /// Target address (if LAN route).
    pub target_address: Option<SocketAddr>,
    /// Whether the target was found via LAN discovery.
    pub found_via_lan: bool,
}

/// Result of a route selection for input sharing.
#[derive(Debug, Clone)]
pub struct InputRouteResult {
    /// Selected route.
    pub route: InputRoute,
    /// Target address (if LAN route).
    pub target_address: Option<SocketAddr>,
    /// Whether the target was found via LAN discovery.
    pub found_via_lan: bool,
}

/// Clipboard route selector.
///
/// Priority:
/// 1. Local loop prevention (don't sync to self)
/// 2. LAN direct (peer found via mDNS/UDP multicast)
/// 3. LAN reverse pull (peer found but can't push directly)
/// 4. Server fallback (no LAN peer found)
pub struct ClipboardRouteSelector {
    /// Our device ID.
    pub local_device_id: String,
    /// Whether we have LAN discovery running.
    pub lan_available: bool,
    /// Whether server is reachable.
    pub server_available: bool,
    /// Reference to the peer registry from discovery.
    pub registry: Option<PeerRegistry>,
}

impl ClipboardRouteSelector {
    pub fn new(local_device_id: String, lan_available: bool, server_available: bool) -> Self {
        Self {
            local_device_id,
            lan_available,
            server_available,
            registry: None,
        }
    }

    pub fn with_registry(mut self, registry: PeerRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Select the best route to a target device.
    pub fn select_route(&self, target_device_id: &str) -> ClipboardRouteResult {
        // 1. Local loop prevention.
        if target_device_id == self.local_device_id {
            return ClipboardRouteResult {
                route: TransferRoute::LanDirect, // Local, no network needed.
                target_address: None,
                found_via_lan: false,
            };
        }

        // 2. Check LAN discovery for the target.
        if self.lan_available {
            if let Some(ref registry) = self.registry {
                if let Some(peer) = registry.get(target_device_id) {
                    if peer.missed_heartbeats < 3 {
                        return ClipboardRouteResult {
                            route: TransferRoute::LanDirect,
                            target_address: Some(peer.address),
                            found_via_lan: true,
                        };
                    } else {
                        // Peer found but stale - try reverse pull.
                        return ClipboardRouteResult {
                            route: TransferRoute::LanReversePull,
                            target_address: Some(peer.address),
                            found_via_lan: true,
                        };
                    }
                }
            }
        }

        // 3. Server fallback.
        if self.server_available {
            return ClipboardRouteResult {
                route: TransferRoute::ServerFallback,
                target_address: None,
                found_via_lan: false,
            };
        }

        // 4. No route available - queue for later retry.
        ClipboardRouteResult {
            route: TransferRoute::ServerFallback, // Best effort.
            target_address: None,
            found_via_lan: false,
        }
    }

    /// Select routes to all available targets (broadcast).
    pub fn select_all_routes(&self, target_ids: &[String]) -> Vec<(String, ClipboardRouteResult)> {
        target_ids
            .iter()
            .filter(|id| **id != self.local_device_id) // Skip self.
            .map(|id| (id.clone(), self.select_route(id)))
            .collect()
    }
}

/// Input route selector.
///
/// Priority:
/// 1. LAN direct (lowest latency)
/// 2. Server relay (when enabled, higher latency)
/// 3. Disconnect and release input (both fail)
pub struct InputRouteSelector {
    /// Our device ID.
    pub local_device_id: String,
    /// Whether LAN is available.
    pub lan_available: bool,
    /// Whether server relay is enabled.
    pub server_relay_enabled: bool,
    /// Reference to the peer registry.
    pub registry: Option<PeerRegistry>,
}

impl InputRouteSelector {
    pub fn new(local_device_id: String, lan_available: bool, server_relay_enabled: bool) -> Self {
        Self {
            local_device_id,
            lan_available,
            server_relay_enabled,
            registry: None,
        }
    }

    pub fn with_registry(mut self, registry: PeerRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Select the best route for input sharing to a target device.
    pub fn select_route(&self, target_device_id: &str) -> Option<InputRouteResult> {
        // Check if target is on LAN.
        if self.lan_available {
            if let Some(ref registry) = self.registry {
                if let Some(peer) = registry.get(target_device_id) {
                    if peer.missed_heartbeats < 3 {
                        return Some(InputRouteResult {
                            route: InputRoute::LanDirect,
                            target_address: Some(peer.address),
                            found_via_lan: true,
                        });
                    }
                }
            }
        }

        // Fall back to server relay.
        if self.server_relay_enabled {
            return Some(InputRouteResult {
                route: InputRoute::ServerRelay,
                target_address: None,
                found_via_lan: false,
            });
        }

        // No route available - disconnect and release.
        None
    }
}

/// Latency measurement tracker for input relay.
pub struct LatencyTracker {
    /// Last measured round-trip time.
    pub last_rtt_ms: Option<u64>,
    /// Moving average of RTT (exponential).
    pub avg_rtt_ms: Option<f64>,
    /// When the last measurement was taken.
    pub last_measurement: Option<Instant>,
    /// Smoothing factor for EMA (0.0 - 1.0).
    pub alpha: f64,
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self {
            last_rtt_ms: None,
            avg_rtt_ms: None,
            last_measurement: None,
            alpha: 0.3,
        }
    }
}

impl LatencyTracker {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Record a new RTT measurement.
    pub fn record(&mut self, rtt_ms: u64) {
        self.last_rtt_ms = Some(rtt_ms);
        self.last_measurement = Some(Instant::now());

        let rtt = rtt_ms as f64;
        self.avg_rtt_ms = Some(match self.avg_rtt_ms {
            Some(avg) => self.alpha * rtt + (1.0 - self.alpha) * avg,
            None => rtt,
        });
    }

    /// Get the current estimated latency.
    pub fn estimate(&self) -> Option<u64> {
        self.avg_rtt_ms.map(|v| v.round() as u64)
    }

    /// Check if latency exceeds the threshold.
    pub fn exceeds(&self, threshold_ms: u64) -> bool {
        self.estimate()
            .map(|est| est > threshold_ms)
            .unwrap_or(false)
    }

    /// Check if the last measurement is stale.
    pub fn is_stale(&self, max_age: std::time::Duration) -> bool {
        self.last_measurement
            .map(|t| t.elapsed() > max_age)
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::PeerRegistry;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    fn make_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), port)
    }

    // --- Clipboard Route Selection Tests ---

    #[test]
    fn test_local_loop_prevention() {
        let selector = ClipboardRouteSelector::new("my-device".to_string(), true, true);

        let result = selector.select_route("my-device");
        // Should route to self (local, no network).
        assert!(!result.found_via_lan);
        assert!(result.target_address.is_none());
    }

    #[test]
    fn test_lan_direct_when_peer_found() {
        let mut registry = PeerRegistry::default();
        registry.upsert("peer-1".to_string(), "Laptop".to_string(), make_addr(9999));

        let selector = ClipboardRouteSelector::new("my-device".to_string(), true, true)
            .with_registry(registry);

        let result = selector.select_route("peer-1");
        assert_eq!(result.route, TransferRoute::LanDirect);
        assert!(result.found_via_lan);
        assert!(result.target_address.is_some());
    }

    #[test]
    fn test_server_fallback_when_no_lan() {
        let selector = ClipboardRouteSelector::new(
            "my-device".to_string(),
            false, // No LAN.
            true,
        );

        let result = selector.select_route("unknown-peer");
        assert_eq!(result.route, TransferRoute::ServerFallback);
        assert!(!result.found_via_lan);
    }

    #[test]
    fn test_server_fallback_when_peer_not_in_registry() {
        let registry = PeerRegistry::default(); // Empty.
        let selector = ClipboardRouteSelector::new("my-device".to_string(), true, true)
            .with_registry(registry);

        let result = selector.select_route("unknown-peer");
        assert_eq!(result.route, TransferRoute::ServerFallback);
    }

    #[test]
    fn test_select_all_routes_skips_self() {
        let mut registry = PeerRegistry::default();
        registry.upsert("peer-1".to_string(), "Laptop".to_string(), make_addr(9999));
        registry.upsert("peer-2".to_string(), "Desktop".to_string(), make_addr(9999));

        let selector = ClipboardRouteSelector::new("my-device".to_string(), true, true)
            .with_registry(registry);

        let routes = selector.select_all_routes(&[
            "my-device".to_string(),
            "peer-1".to_string(),
            "peer-2".to_string(),
        ]);

        // Should have 2 routes (skipping self).
        assert_eq!(routes.len(), 2);
        assert!(routes.iter().any(|(id, _)| id == "peer-1"));
        assert!(routes.iter().any(|(id, _)| id == "peer-2"));
    }

    // --- Input Route Selection Tests ---

    #[test]
    fn test_input_lan_direct() {
        let mut registry = PeerRegistry::default();
        registry.upsert(
            "target".to_string(),
            "Target PC".to_string(),
            make_addr(9999),
        );

        let selector =
            InputRouteSelector::new("my-device".to_string(), true, true).with_registry(registry);

        let result = selector.select_route("target").unwrap();
        assert_eq!(result.route, InputRoute::LanDirect);
        assert!(result.found_via_lan);
    }

    #[test]
    fn test_input_server_relay_fallback() {
        let registry = PeerRegistry::default(); // No peers.
        let selector = InputRouteSelector::new(
            "my-device".to_string(),
            false, // No LAN.
            true,  // Server relay enabled.
        )
        .with_registry(registry);

        let result = selector.select_route("target").unwrap();
        assert_eq!(result.route, InputRoute::ServerRelay);
        assert!(!result.found_via_lan);
    }

    #[test]
    fn test_input_disconnect_when_no_route() {
        let registry = PeerRegistry::default();
        let selector = InputRouteSelector::new(
            "my-device".to_string(),
            false, // No LAN.
            false, // No server relay.
        )
        .with_registry(registry);

        let result = selector.select_route("target");
        assert!(result.is_none()); // Should return None = disconnect.
    }

    // --- Latency Tracker Tests ---

    #[test]
    fn test_latency_tracker_first_measurement() {
        let mut tracker = LatencyTracker::default();
        assert!(tracker.estimate().is_none());

        tracker.record(50);
        assert_eq!(tracker.estimate(), Some(50));
        assert_eq!(tracker.last_rtt_ms, Some(50));
    }

    #[test]
    fn test_latency_tracker_exponential_moving_average() {
        let mut tracker = LatencyTracker::new(0.5);
        tracker.record(100); // First: 100
        tracker.record(50); // EMA: 0.5 * 50 + 0.5 * 100 = 75

        let est = tracker.estimate().unwrap();
        assert_eq!(est, 75);
    }

    #[test]
    fn test_latency_tracker_exceeds_threshold() {
        let mut tracker = LatencyTracker::default();
        tracker.record(250);
        assert!(tracker.exceeds(200));
        assert!(!tracker.exceeds(300));
    }

    #[test]
    fn test_latency_tracker_stale_detection() {
        use std::thread::sleep;

        let mut tracker = LatencyTracker::default();
        tracker.record(50);
        assert!(!tracker.is_stale(Duration::from_secs(1)));

        sleep(Duration::from_millis(100));
        assert!(tracker.is_stale(Duration::from_millis(50)));
    }
}
