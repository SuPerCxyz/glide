/// LAN discovery module for Glide.
///
/// Provides peer discovery via:
/// - mDNS (multicast DNS) service advertisement and resolution
/// - UDP multicast heartbeat/announcement protocol
///
/// Peers discovered via either method are tracked in a [`PeerRegistry`].

mod mdns;
mod udp_multicast;
mod peer_registry;

pub use mdns::{MdnsDiscovery, MdnsService};
pub use peer_registry::{DiscoveredPeer, PeerRegistry, PeerState};
pub use udp_multicast::{UdpMulticastDiscovery, UdpMulticastConfig};

/// Default mDNS service type for Glide.
pub const MDNS_SERVICE_TYPE: &str = "_glide._tcp.local.";
/// Default UDP multicast group for Glide peer discovery.
pub const MULTICAST_GROUP: &str = "239.255.0.1";
/// Default UDP multicast port.
pub const MULTICAST_PORT: u16 = 9998;
