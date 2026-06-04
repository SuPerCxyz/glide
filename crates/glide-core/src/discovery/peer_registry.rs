use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// State of a discovered peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    /// Peer was just discovered.
    New,
    /// Peer is actively responding to heartbeats.
    Active,
    /// Peer hasn't responded recently but was recently active.
    Stale,
    /// Peer is considered offline.
    Offline,
}

/// A discovered peer with metadata.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    /// Unique device identifier.
    pub device_id: String,
    /// Human-readable name.
    pub name: String,
    /// Address to reach this peer.
    pub address: SocketAddr,
    /// Current state.
    pub state: PeerState,
    /// Last time we heard from this peer.
    pub last_seen: Instant,
    /// When this peer was first discovered.
    pub first_seen: Instant,
    /// Number of consecutive heartbeat failures.
    pub missed_heartbeats: u32,
}

impl DiscoveredPeer {
    pub fn new(device_id: String, name: String, address: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            device_id,
            name,
            address,
            state: PeerState::New,
            last_seen: now,
            first_seen: now,
            missed_heartbeats: 0,
        }
    }
}

/// Registry tracking all discovered peers.
#[derive(Debug)]
pub struct PeerRegistry {
    peers: HashMap<String, DiscoveredPeer>,
    /// Time after which a peer is considered stale.
    stale_timeout: Duration,
    /// Time after which a stale peer is considered offline.
    offline_timeout: Duration,
}

impl Default for PeerRegistry {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(30),  // stale after 30s
            Duration::from_secs(120), // offline after 120s
        )
    }
}

impl PeerRegistry {
    pub fn new(stale_timeout: Duration, offline_timeout: Duration) -> Self {
        Self {
            peers: HashMap::new(),
            stale_timeout,
            offline_timeout,
        }
    }

    /// Register or update a discovered peer.
    pub fn upsert(&mut self, device_id: String, name: String, address: SocketAddr) -> &DiscoveredPeer {
        let entry = self.peers.entry(device_id.clone()).or_insert_with(|| {
            DiscoveredPeer::new(device_id, name.clone(), address)
        });
        entry.name = name;
        entry.address = address;
        entry.state = PeerState::Active;
        entry.last_seen = Instant::now();
        entry.missed_heartbeats = 0;
        entry
    }

    /// Mark a peer as missed heartbeat.
    pub fn mark_missed(&mut self, device_id: &str) {
        if let Some(peer) = self.peers.get_mut(device_id) {
            peer.missed_heartbeats += 1;
            let elapsed = peer.last_seen.elapsed();
            if elapsed > self.offline_timeout {
                peer.state = PeerState::Offline;
            } else if elapsed > self.stale_timeout {
                peer.state = PeerState::Stale;
            }
        }
    }

    /// Tick the registry, updating peer states based on time elapsed.
    pub fn tick(&mut self) -> Vec<String> {
        let mut removed = Vec::new();
        let now = Instant::now();

        for (id, peer) in self.peers.iter_mut() {
            let elapsed = now.duration_since(peer.last_seen);
            if elapsed > self.offline_timeout {
                peer.state = PeerState::Offline;
                // Remove peers that have been offline for too long.
                if elapsed > self.offline_timeout * 4 {
                    removed.push(id.clone());
                }
            } else if elapsed > self.stale_timeout {
                if peer.state != PeerState::Offline {
                    peer.state = PeerState::Stale;
                }
            }
        }

        for id in &removed {
            self.peers.remove(id);
        }

        removed
    }

    /// Get all active peers.
    pub fn active_peers(&self) -> Vec<&DiscoveredPeer> {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Active)
            .collect()
    }

    /// Get all peers (active + stale).
    pub fn all_peers(&self) -> Vec<&DiscoveredPeer> {
        self.peers
            .values()
            .filter(|p| p.state != PeerState::Offline)
            .collect()
    }

    /// Get a specific peer.
    pub fn get(&self, device_id: &str) -> Option<&DiscoveredPeer> {
        self.peers.get(device_id)
    }

    /// Check if a peer is known and active.
    pub fn is_active(&self, device_id: &str) -> bool {
        self.peers
            .get(device_id)
            .map(|p| p.state == PeerState::Active)
            .unwrap_or(false)
    }

    /// Remove a peer.
    pub fn remove(&mut self, device_id: &str) -> Option<DiscoveredPeer> {
        self.peers.remove(device_id)
    }

    /// Get count of active peers.
    pub fn active_count(&self) -> usize {
        self.active_peers().len()
    }

    /// Clear all peers.
    pub fn clear(&mut self) {
        self.peers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn make_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), port)
    }

    #[test]
    fn test_upsert_new_peer() {
        let mut registry = PeerRegistry::default();
        registry.upsert("dev-1".to_string(), "Laptop".to_string(), make_addr(9999));

        assert!(registry.is_active("dev-1"));
        assert_eq!(registry.active_count(), 1);
        assert_eq!(registry.all_peers().len(), 1);
    }

    #[test]
    fn test_upsert_updates_existing() {
        let mut registry = PeerRegistry::default();
        registry.upsert("dev-1".to_string(), "Laptop".to_string(), make_addr(9999));
        registry.upsert("dev-1".to_string(), "Laptop Pro".to_string(), make_addr(8888));

        let peer = registry.get("dev-1").unwrap();
        assert_eq!(peer.name, "Laptop Pro");
        assert_eq!(peer.address.port(), 8888);
    }

    #[test]
    fn test_mark_missed() {
        let mut registry = PeerRegistry::default();
        registry.upsert("dev-1".to_string(), "Laptop".to_string(), make_addr(9999));

        registry.mark_missed("dev-1");
        assert_eq!(registry.get("dev-1").unwrap().missed_heartbeats, 1);
    }

    #[test]
    fn test_clear() {
        let mut registry = PeerRegistry::default();
        registry.upsert("dev-1".to_string(), "Laptop".to_string(), make_addr(9999));
        registry.upsert("dev-2".to_string(), "Desktop".to_string(), make_addr(9999));

        registry.clear();
        assert_eq!(registry.active_count(), 0);
        assert!(registry.get("dev-1").is_none());
    }

    #[test]
    fn test_remove() {
        let mut registry = PeerRegistry::default();
        registry.upsert("dev-1".to_string(), "Laptop".to_string(), make_addr(9999));

        let removed = registry.remove("dev-1");
        assert!(removed.is_some());
        assert!(registry.get("dev-1").is_none());
    }
}
