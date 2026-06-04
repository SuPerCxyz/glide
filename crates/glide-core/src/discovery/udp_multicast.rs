use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

use super::peer_registry::PeerRegistry;

/// Configuration for UDP multicast discovery.
#[derive(Debug, Clone)]
pub struct UdpMulticastConfig {
    /// Multicast group address (e.g. "239.255.0.1").
    pub multicast_group: String,
    /// Multicast port.
    pub multicast_port: u16,
    /// Local bind port (usually same as multicast_port).
    pub bind_port: u16,
    /// How often to send announcements.
    pub announcement_interval: Duration,
    /// Device ID to announce.
    pub device_id: String,
    /// Human-readable name to announce.
    pub name: String,
    /// Service port (the actual Glide service port).
    pub service_port: u16,
}

impl Default for UdpMulticastConfig {
    fn default() -> Self {
        Self {
            multicast_group: "239.255.0.1".to_string(),
            multicast_port: 9998,
            bind_port: 9998,
            announcement_interval: Duration::from_secs(5),
            device_id: String::new(),
            name: "unknown".to_string(),
            service_port: 9999,
        }
    }
}

/// UDP multicast-based peer discovery.
///
/// Sends periodic announcements to the multicast group and listens
/// for announcements from other peers.
pub struct UdpMulticastDiscovery {
    config: UdpMulticastConfig,
    socket: Option<UdpSocket>,
    running: Arc<AtomicBool>,
}

/// Wire format for announcements:
/// GLIDE\0<device_id>\0<name>\0<service_port>\0
///
/// Magic header: 5 bytes "GLIDE"
/// Null-terminated fields for simplicity.
const MAGIC: &[u8; 5] = b"GLIDE";

impl UdpMulticastDiscovery {
    pub fn new(config: UdpMulticastConfig) -> Self {
        Self {
            config,
            socket: None,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the UDP socket for multicast.
    pub fn init(&mut self) -> anyhow::Result<()> {
        let bind_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            self.config.bind_port,
        );

        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_ttl(2)?;

        // Join multicast group on all interfaces.
        let multicast_addr = Ipv4Addr::new(239, 255, 0, 1);
        socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;

        socket.set_broadcast(true)?;

        info!(
            "UDP multicast discovery initialized on {} group {} port {}",
            bind_addr, self.config.multicast_group, self.config.multicast_port
        );

        self.socket = Some(socket);
        Ok(())
    }

    /// Send an announcement to the multicast group.
    pub fn send_announcement(&self) -> anyhow::Result<()> {
        if let Some(ref socket) = self.socket {
            let multicast_addr = SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(239, 255, 0, 1)),
                self.config.multicast_port,
            );

            let announcement = encode_announcement(
                &self.config.device_id,
                &self.config.name,
                self.config.service_port,
            );

            socket.send_to(&announcement, multicast_addr)?;
        }
        Ok(())
    }

    /// Receive a single announcement and update the peer registry.
    pub fn receive_announcement(&self, registry: &mut PeerRegistry) -> anyhow::Result<bool> {
        if let Some(ref socket) = self.socket {
            let mut buf = [0u8; 1024];

            // Use a short timeout so we don't block forever.
            socket.set_read_timeout(Some(Duration::from_millis(100)))?;

            match socket.recv_from(&mut buf) {
                Ok((len, from)) => {
                    if let Some(announcement) = decode_announcement(&buf[..len]) {
                        // Ignore our own announcements.
                        if announcement.device_id == self.config.device_id {
                            return Ok(false);
                        }

                        let addr = SocketAddr::new(from.ip(), announcement.service_port);
                        registry.upsert(
                            announcement.device_id,
                            announcement.name,
                            addr,
                        );
                        return Ok(true);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available within timeout.
                    return Ok(false);
                }
                Err(e) => {
                    warn!("UDP multicast receive error: {}", e);
                    return Err(e.into());
                }
            }
        }
        Ok(false)
    }

    /// Run the discovery loop until stopped.
    /// This blocks the calling thread. For async use, call from a blocking task.
    pub fn run_discovery(
        &self,
        registry: &mut PeerRegistry,
    ) -> anyhow::Result<()> {
        if self.socket.is_none() {
            anyhow::bail!("UDP multicast discovery not initialized. Call init() first.");
        }

        info!("UDP multicast discovery loop started");

        let mut last_announcement = std::time::Instant::now();
        let mut last_tick = std::time::Instant::now();

        while self.running.load(Ordering::SeqCst) {
            // Send announcement at configured interval.
            if last_announcement.elapsed() >= self.config.announcement_interval {
                if let Err(e) = self.send_announcement() {
                    warn!("Failed to send announcement: {}", e);
                }
                last_announcement = std::time::Instant::now();
            }

            // Receive any incoming announcements.
            loop {
                match self.receive_announcement(registry) {
                    Ok(true) => {} // Found a peer, try to receive more.
                    Ok(false) => break, // No more data or our own announcement.
                    Err(e) => {
                        warn!("Discovery receive error: {}", e);
                        break;
                    }
                }
            }

            // Tick the registry to update peer states.
            if last_tick.elapsed() >= Duration::from_secs(10) {
                let removed = registry.tick();
                for id in removed {
                    info!("Removed offline peer: {}", id);
                }
                last_tick = std::time::Instant::now();
            }

            // Small sleep to avoid busy-waiting.
            std::thread::sleep(Duration::from_millis(500));
        }

        info!("UDP multicast discovery loop stopped");
        Ok(())
    }

    /// Signal the discovery loop to stop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get a handle to start/stop the discovery loop.
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}

/// Announcement data.
struct Announcement {
    device_id: String,
    name: String,
    service_port: u16,
}

/// Encode an announcement to wire format.
pub(crate) fn encode_announcement(device_id: &str, name: &str, service_port: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(MAGIC);
    buf.push(0); // null separator
    buf.extend_from_slice(device_id.as_bytes());
    buf.push(0);
    buf.extend_from_slice(name.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&service_port.to_be_bytes());
    buf
}

/// Decode an announcement from wire format.
fn decode_announcement(data: &[u8]) -> Option<Announcement> {
    // Minimum: MAGIC(5) + null(1) + device_id(1) + null(1) + name(1) + null(1) + port(2) = 12
    if data.len() < 12 {
        return None;
    }

    // Check magic header.
    if &data[0..5] != MAGIC {
        return None;
    }

    // Find null separators.
    let mut iter = data[6..].split(|&b| b == 0);

    let device_id = std::str::from_utf8(iter.next()?).ok()?;
    let name = std::str::from_utf8(iter.next()?).ok()?;

    let port_bytes = iter.next()?;
    if port_bytes.len() < 2 {
        return None;
    }
    let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);

    Some(Announcement {
        device_id: device_id.to_string(),
        name: name.to_string(),
        service_port: port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_announcement() {
        let data = encode_announcement("dev-123", "My Laptop", 9999);
        let ann = decode_announcement(&data).unwrap();

        assert_eq!(ann.device_id, "dev-123");
        assert_eq!(ann.name, "My Laptop");
        assert_eq!(ann.service_port, 9999);
    }

    #[test]
    fn test_decode_invalid_magic() {
        let mut data = encode_announcement("dev-1", "test", 8080);
        data[0] = b'X'; // corrupt magic

        assert!(decode_announcement(&data).is_none());
    }

    #[test]
    fn test_decode_too_short() {
        let data = b"GLIDE\x00ab";
        assert!(decode_announcement(data).is_none());
    }

    #[test]
    fn test_decode_empty_name() {
        let data = encode_announcement("dev-1", "", 8080);
        let ann = decode_announcement(&data).unwrap();
        assert_eq!(ann.device_id, "dev-1");
        assert_eq!(ann.name, "");
    }
}
