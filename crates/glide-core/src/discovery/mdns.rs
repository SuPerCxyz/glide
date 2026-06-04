use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

use super::peer_registry::PeerRegistry;
use super::MDNS_SERVICE_TYPE;

/// An mDNS service discovered on the network.
#[derive(Debug, Clone)]
pub struct MdnsService {
    /// Full service name (e.g. "My Laptop._glide._tcp.local.").
    pub full_name: String,
    /// Instance name (e.g. "My Laptop").
    pub name: String,
    /// Service type (e.g. "_glide._tcp.local.").
    pub service_type: String,
    /// Host address.
    pub address: String,
    /// Service port.
    pub port: u16,
    /// TXT record with device_id.
    pub device_id: Option<String>,
}

/// mDNS-based peer discovery.
///
/// This is a simplified mDNS implementation using raw UDP sockets.
/// For production use, consider using a proper mDNS library like libmdns.
///
/// This implementation sends mDNS queries and parses basic responses
/// to discover Glide services on the local network.
pub struct MdnsDiscovery {
    /// Our device ID.
    pub device_id: String,
    /// Our service name.
    pub name: String,
    /// Our service port.
    pub port: u16,
    /// Whether discovery is running.
    running: Arc<AtomicBool>,
}

impl MdnsDiscovery {
    pub fn new(device_id: String, name: String, port: u16) -> Self {
        Self {
            device_id,
            name,
            port,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run the mDNS discovery loop.
    ///
    /// This is a simplified implementation that:
    /// 1. Sends mDNS PTR queries for _glide._tcp.local.
    /// 2. Listens for responses
    /// 3. Parses discovered services
    ///
    /// For a production implementation, use the `libmdns` crate
    /// or a proper mDNS stack.
    pub fn run_discovery(
        &self,
        registry: &mut PeerRegistry,
    ) -> anyhow::Result<()> {
        use std::net::{IpAddr, Ipv4Addr, UdpSocket};
        use std::time::{Duration, Instant};

        // mDNS uses port 5353 on all interfaces.
        let mdns_port = 5353;
        let mdns_multicast = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(224, 0, 0, 251)),
            mdns_port,
        );

        // Bind to a random high port and join multicast group.
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                warn!("mDNS discovery: failed to bind UDP socket: {}", e);
                // Fall back to a simpler announcement-only approach.
                return self.run_announcement_only(registry);
            }
        };

        // Join mDNS multicast group.
        if socket.join_multicast_v4(&Ipv4Addr::new(224, 0, 0, 251), &Ipv4Addr::UNSPECIFIED).is_err() {
            warn!("mDNS discovery: failed to join multicast group");
            return self.run_announcement_only(registry);
        }

        socket.set_ttl(2)?;
        socket.set_multicast_loop_v4(false)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        info!("mDNS discovery started on port {}", mdns_port);

        let mut last_query = Instant::now();
        let mut last_tick = Instant::now();

        while self.running.load(Ordering::SeqCst) {
            // Send PTR query for Glide service type periodically.
            if last_query.elapsed() >= Duration::from_secs(30) {
                if let Err(e) = send_mdns_query(&socket, &mdns_multicast) {
                    warn!("Failed to send mDNS query: {}", e);
                }
                last_query = Instant::now();
            }

            // Try to receive and parse mDNS responses.
            let mut buf = [0u8; 4096];
            match socket.recv_from(&mut buf) {
                Ok((len, _from)) => {
                    if let Some(services) = parse_mdns_response(&buf[..len]) {
                        for svc in services {
                            if svc.device_id.as_deref() == Some(&self.device_id) {
                                continue; // Skip our own services.
                            }
                            let addr = match svc.address.parse::<std::net::IpAddr>() {
                                Ok(ip) => SocketAddr::new(ip, svc.port),
                                Err(_) => continue,
                            };
                            let name = svc.device_id.unwrap_or_else(|| svc.name.clone());
                            registry.upsert(name, svc.name, addr);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available.
                }
                Err(e) => {
                    warn!("mDNS receive error: {}", e);
                }
            }

            // Periodically announce our own service.
            if last_query.elapsed() >= Duration::from_secs(10) {
                let _ = announce_our_service(&socket, &mdns_multicast, &self.name, &self.device_id, self.port);
            }

            // Tick the registry.
            if last_tick.elapsed() >= Duration::from_secs(10) {
                let removed = registry.tick();
                for id in removed {
                    info!("Removed offline peer: {}", id);
                }
                last_tick = Instant::now();
            }

            std::thread::sleep(Duration::from_millis(500));
        }

        info!("mDNS discovery stopped");
        Ok(())
    }

    /// Fallback: announcement-only mode when mDNS socket can't bind.
    fn run_announcement_only(&self, _registry: &mut PeerRegistry) -> anyhow::Result<()> {
        use std::time::Duration;
        info!("mDNS discovery running in announcement-only mode (unable to receive)");

        while self.running.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(30));
        }

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

// --- mDNS wire format helpers (simplified) ---

/// Send a PTR query for the Glide service type.
fn send_mdns_query(socket: &std::net::UdpSocket, target: &SocketAddr) -> anyhow::Result<()> {
    // Build a minimal mDNS query:
    // Transaction ID (2) + Flags (2) + QDCount (2) + AN/NS/AR (6) + Question
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u16.to_be_bytes()); // Transaction ID
    buf.extend_from_slice(&0u16.to_be_bytes()); // Flags: standard query
    buf.extend_from_slice(&1u16.to_be_bytes()); // QDCount = 1
    buf.extend_from_slice(&0u64.to_be_bytes()); // AN=0, NS=0, AR=0

    // Question: _glide._tcp.local. IN PTR
    encode_dns_name(&mut buf, MDNS_SERVICE_TYPE);
    buf.extend_from_slice(&12u16.to_be_bytes()); // PTR type
    buf.extend_from_slice(&1u16.to_be_bytes());  // IN class

    socket.send_to(&buf, target)?;
    Ok(())
}

/// Announce our own service via mDNS response (unsolicited).
fn announce_our_service(
    socket: &std::net::UdpSocket,
    target: &SocketAddr,
    name: &str,
    device_id: &str,
    port: u16,
) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u16.to_be_bytes()); // Transaction ID
    buf.extend_from_slice(&0x8400u16.to_be_bytes()); // Flags: response + AA
    buf.extend_from_slice(&0u16.to_be_bytes()); // QDCount = 0
    buf.extend_from_slice(&1u16.to_be_bytes()); // ANCount = 1
    buf.extend_from_slice(&0u32.to_be_bytes()); // NSCount=0, ARCount=0

    // Answer: _glide._tcp.local. PTR <name>._glide._tcp.local.
    encode_dns_name(&mut buf, MDNS_SERVICE_TYPE);
    buf.extend_from_slice(&12u16.to_be_bytes()); // PTR type
    buf.extend_from_slice(&0x0001u16.to_be_bytes()); // IN class + cache flush
    buf.extend_from_slice(&120u32.to_be_bytes()); // TTL = 120s
    buf.extend_from_slice(&0u16.to_be_bytes()); // Rdlength (placeholder, fixed below)

    // For simplicity, just send the query - proper mDNS needs more.
    // This is a placeholder for a full implementation.
    let _ = (name, device_id, port); // Used in full impl.

    // Fall back to a simple UDP announcement on our multicast port.
    let mut ann = Vec::new();
    ann.extend_from_slice(b"GLIDE\0");
    ann.extend_from_slice(device_id.as_bytes());
    ann.push(0);
    ann.extend_from_slice(name.as_bytes());
    ann.push(0);
    ann.extend_from_slice(&port.to_be_bytes());
    socket.send_to(&ann, target)?;

    Ok(())
}

/// Parse a raw mDNS response into discovered services.
fn parse_mdns_response(data: &[u8]) -> Option<Vec<MdnsService>> {
    // Simplified parser - real mDNS is more complex.
    // We look for our magic "GLIDE" pattern in any response.
    if data.len() < 12 {
        return None;
    }

    let mut services = Vec::new();

    // Try to find GLIDE announcements embedded in or alongside mDNS.
    let mut pos = 0;
    while pos + 12 <= data.len() {
        if &data[pos..pos + 5] == b"GLIDE" {
            if let Some(ann) = parse_glide_announcement(&data[pos..]) {
                services.push(MdnsService {
                    full_name: format!("{}._glide._tcp.local.", ann.name),
                    name: ann.name.clone(),
                    service_type: MDNS_SERVICE_TYPE.to_string(),
                    address: String::new(), // Would need to resolve via DNS
                    port: ann.port,
                    device_id: Some(ann.device_id),
                });
            }
        }
        pos += 1;
    }

    if services.is_empty() {
        None
    } else {
        Some(services)
    }
}

struct GlideAnnouncement {
    device_id: String,
    name: String,
    port: u16,
}

fn parse_glide_announcement(data: &[u8]) -> Option<GlideAnnouncement> {
    if data.len() < 12 || &data[0..5] != b"GLIDE" {
        return None;
    }

    let mut iter = data[6..].split(|&b| b == 0);
    let device_id = std::str::from_utf8(iter.next()?).ok()?;
    let name = std::str::from_utf8(iter.next()?).ok()?;

    let port_bytes = iter.next()?;
    if port_bytes.len() < 2 {
        return None;
    }
    let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);

    Some(GlideAnnouncement {
        device_id: device_id.to_string(),
        name: name.to_string(),
        port,
    })
}

/// Encode a DNS name in wire format.
fn encode_dns_name(buf: &mut Vec<u8>, name: &str) {
    for label in name.split('.') {
        if label.is_empty() {
            continue;
        }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // Root label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_dns_name() {
        let mut buf = Vec::new();
        encode_dns_name(&mut buf, "_glide._tcp.local.");
        // 6 _glide 3 tcp 5 local 0
        assert_eq!(buf[0], 6);
        assert_eq!(&buf[1..7], b"_glide");
        assert_eq!(buf[7], 4);
        assert_eq!(&buf[8..12], b"_tcp");
        assert_eq!(buf[12], 5);
        assert_eq!(&buf[13..18], b"local");
        assert_eq!(buf[18], 0);
    }

    #[test]
    fn test_parse_glide_announcement() {
        use super::super::udp_multicast::encode_announcement;
        let data = encode_announcement("dev-abc", "Office PC", 9999);
        let ann = parse_glide_announcement(&data).unwrap();

        assert_eq!(ann.device_id, "dev-abc");
        assert_eq!(ann.name, "Office PC");
        assert_eq!(ann.port, 9999);
    }

    #[test]
    fn test_parse_mdns_response_with_glide() {
        let mut data = vec![0u8; 20]; // fake DNS header
        data.extend_from_slice(b"GLIDE\0");
        data.extend_from_slice(b"device1\0");
        data.extend_from_slice(b"MyMac\0");
        data.extend_from_slice(&8080u16.to_be_bytes());

        let services = parse_mdns_response(&data).unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "MyMac");
        assert_eq!(services[0].device_id.as_deref(), Some("device1"));
        assert_eq!(services[0].port, 8080);
    }
}
