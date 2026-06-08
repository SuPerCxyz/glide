use futures::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{error, info, warn};

use glide_core::clipboard::ClipboardItem;
use glide_core::discovery::PeerRegistry;
use glide_core::discovery::{UdpMulticastConfig, UdpMulticastDiscovery};
use glide_core::route::ClipboardRouteSelector;
use glide_core::sync_event::SyncEvent;

/// Shared state between LAN sync engine and GUI.
pub struct LanSyncState {
    pub peer_registry: Arc<RwLock<PeerRegistry>>,
    pub trusted_peers: Arc<RwLock<HashSet<String>>>,
    pub peers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<SyncEvent>>>>,
    pub clipboard_tx: mpsc::UnboundedSender<ClipboardItem>,
    pub clipboard_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<ClipboardItem>>>>,
    pub running: Arc<std::sync::atomic::AtomicBool>,
}

/// LAN sync engine - enables direct peer-to-peer clipboard sync
/// on the same Layer 2 network without server dependency.
///
/// Flow:
/// 1. UDP multicast announces this device every 5 seconds
/// 2. Other devices on the same LAN are discovered automatically
/// 3. When clipboard changes, send directly to all LAN peers via WebSocket
/// 4. Receive clipboard events from LAN peers and apply locally
pub struct LanSyncEngine {
    /// Our device ID.
    pub device_id: String,
    /// Our device name.
    pub device_name: String,
    /// Our LAN service port (for incoming connections).
    pub service_port: u16,
    /// Route selector.
    pub route_selector: Arc<RwLock<ClipboardRouteSelector>>,
    /// Shared state accessible from GUI.
    pub state: Arc<LanSyncState>,
}

impl LanSyncEngine {
    pub fn new(device_id: String, device_name: String, service_port: u16) -> Self {
        let (clipboard_tx, clipboard_rx) = mpsc::unbounded_channel();
        let registry = PeerRegistry::default();

        Self {
            device_id: device_id.clone(),
            device_name,
            service_port,
            route_selector: Arc::new(RwLock::new(ClipboardRouteSelector::new(
                device_id, true, false,
            ))),
            state: Arc::new(LanSyncState {
                peer_registry: Arc::new(RwLock::new(registry)),
                trusted_peers: Arc::new(RwLock::new(HashSet::new())),
                peers: Arc::new(RwLock::new(HashMap::new())),
                clipboard_tx,
                clipboard_rx: Arc::new(Mutex::new(Some(clipboard_rx))),
                running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }),
        }
    }

    /// Start the LAN sync engine.
    /// This spawns background tasks for:
    /// - UDP multicast discovery
    /// - LAN WebSocket server (accept incoming connections)
    /// - Periodic peer connection attempts
    pub async fn start(&self) -> anyhow::Result<()> {
        self.state
            .running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        info!(
            "LAN sync engine starting: device={} port={}",
            self.device_id, self.service_port
        );

        // 1. Start UDP multicast discovery.
        let discovery_config = UdpMulticastConfig {
            device_id: self.device_id.clone(),
            name: self.device_name.clone(),
            service_port: self.service_port,
            ..Default::default()
        };

        let registry = self.state.peer_registry.clone();
        let running = self.state.running.clone();
        let device_id = self.device_id.clone();

        tokio::spawn(async move {
            let mut discovery = UdpMulticastDiscovery::new(discovery_config);
            if let Err(e) = discovery.init() {
                error!("Failed to init UDP multicast: {}", e);
                return;
            }

            info!("UDP multicast discovery started");

            while running.load(std::sync::atomic::Ordering::SeqCst) {
                if let Err(e) = discovery.send_announcement() {
                    warn!("Failed to send announcement: {}", e);
                }

                let mut reg = registry.write().await;
                loop {
                    match discovery.receive_announcement(&mut reg) {
                        Ok(true) => {
                            info!("Discovered LAN peer");
                        }
                        Ok(false) => break,
                        Err(e) => {
                            warn!("Discovery receive error: {}", e);
                            break;
                        }
                    }
                }

                let removed = reg.tick();
                for id in removed {
                    info!("Removed offline peer: {}", id);
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });

        // 2. Start LAN WebSocket server for incoming connections.
        let listen_addr = format!("0.0.0.0:{}", self.service_port);
        let clipboard_tx = self.state.clipboard_tx.clone();
        let peers = self.state.peers.clone();
        let device_id = self.device_id.clone();
        let running = self.state.running.clone();

        tokio::spawn(async move {
            use futures::{SinkExt, StreamExt};
            use tokio::net::TcpListener;
            use tokio_tungstenite::accept_async;

            let listener = match TcpListener::bind(&listen_addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!(
                        "Failed to bind LAN WebSocket server on {}: {}",
                        listen_addr, e
                    );
                    return;
                }
            };

            info!("LAN WebSocket server listening on {}", listen_addr);

            while running.load(std::sync::atomic::Ordering::SeqCst) {
                let (stream, addr) = match listener.accept().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Accept error: {}", e);
                        continue;
                    }
                };

                info!("LAN peer connected from {}", addr);

                let clipboard_tx = clipboard_tx.clone();
                let peers = peers.clone();
                let device_id = device_id.clone();

                tokio::spawn(async move {
                    let ws_stream = match accept_async(stream).await {
                        Ok(ws) => ws,
                        Err(e) => {
                            warn!("WebSocket accept error from {}: {}", addr, e);
                            return;
                        }
                    };

                    let (mut ws_tx, mut ws_rx) = ws_stream.split();
                    let (tx, mut rx) = mpsc::unbounded_channel::<SyncEvent>();

                    // Forward outgoing events to WebSocket.
                    let send_task = tokio::spawn(async move {
                        while let Some(event) = rx.recv().await {
                            if let Ok(msg) = serde_json::to_string(&event) {
                                if ws_tx
                                    .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    });

                    // Process incoming messages.
                    while let Some(Ok(msg)) = ws_rx.next().await {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            if let Ok(event) = serde_json::from_str::<SyncEvent>(&text) {
                                match &event {
                                    SyncEvent::ClipboardCaptured { item } => {
                                        info!(
                                            "LAN clipboard received: {} from {}",
                                            item.item_id, item.source_device_id
                                        );
                                        let _ = clipboard_tx.send(item.clone());
                                    }
                                    SyncEvent::DeviceJoined {
                                        device_id: did,
                                        name,
                                    } => {
                                        info!("LAN peer identified: {} ({})", name, did);
                                        peers.write().await.insert(did.clone(), tx.clone());
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    send_task.abort();
                    info!("LAN peer disconnected: {}", addr);
                });
            }
        });

        // 3. Periodic peer connection task.
        let registry = self.state.peer_registry.clone();
        let peers = self.state.peers.clone();
        let device_id = self.device_id.clone();
        let running = self.state.running.clone();

        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                let reg = registry.read().await;
                let active_peers: Vec<_> = reg.active_peers().into_iter().cloned().collect();
                drop(reg);

                for peer in active_peers {
                    let peer_id = peer.device_id.to_string();
                    let already_connected = peers.read().await.contains_key(&peer_id);
                    if already_connected {
                        continue;
                    }

                    // Try to connect to the peer.
                    let addr = peer.address;
                    info!("Connecting to LAN peer {} at {}", peer.name, addr);

                    match tokio::net::TcpStream::connect(addr).await {
                        Ok(stream) => {
                            match tokio_tungstenite::client_async(&format!("ws://{}", addr), stream)
                                .await
                            {
                                Ok((ws_stream, _)) => {
                                    let (mut ws_tx, mut ws_rx) = ws_stream.split();
                                    let (tx, mut rx) = mpsc::unbounded_channel::<SyncEvent>();

                                    // Send our identity.
                                    let identify = SyncEvent::DeviceJoined {
                                        device_id: device_id.clone(),
                                        name: "LAN Peer".to_string(),
                                    };
                                    if let Ok(msg) = serde_json::to_string(&identify) {
                                        let _ = ws_tx
                                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                                msg,
                                            ))
                                            .await;
                                    }

                                    peers.write().await.insert(peer_id.clone(), tx.clone());

                                    // Forward outgoing events.
                                    let peers_clone = peers.clone();
                                    let peer_id_clone = peer_id.clone();
                                    tokio::spawn(async move {
                                        while let Some(event) = rx.recv().await {
                                            if let Ok(msg) = serde_json::to_string(&event) {
                                                if ws_tx.send(
                                                    tokio_tungstenite::tungstenite::Message::Text(msg)
                                                ).await.is_err() {
                                                    break;
                                                }
                                            }
                                        }
                                        peers_clone.write().await.remove(&peer_id_clone);
                                    });

                                    // Handle incoming messages.
                                    let peers_for_recv = peers.clone();
                                    let peer_id_for_recv = peer_id.clone();
                                    tokio::spawn(async move {
                                        while let Some(Ok(msg)) = ws_rx.next().await {
                                            if let tokio_tungstenite::tungstenite::Message::Text(
                                                text,
                                            ) = msg
                                            {
                                                if let Ok(event) =
                                                    serde_json::from_str::<SyncEvent>(&text)
                                                {
                                                    // Forward to connected peers.
                                                    let peers = peers_for_recv.read().await;
                                                    for (pid, ptx) in peers.iter() {
                                                        if pid != &peer_id_for_recv {
                                                            let _ = ptx.send(event.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    });

                                    info!("Connected to LAN peer {} at {}", peer.name, addr);
                                }
                                Err(e) => {
                                    warn!("WebSocket handshake failed with {}: {}", addr, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to connect to LAN peer {}: {}", addr, e);
                        }
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        });

        info!("LAN sync engine started");
        Ok(())
    }

    /// Send a clipboard item to all connected LAN peers.
    pub async fn broadcast_clipboard(&self, item: &ClipboardItem) -> anyhow::Result<()> {
        let peers = self.state.peers.read().await;
        if peers.is_empty() {
            return Ok(());
        }

        let event = SyncEvent::ClipboardCaptured { item: item.clone() };

        let mut sent = 0;
        for (pid, tx) in peers.iter() {
            if pid != &self.device_id {
                if tx.send(event.clone()).is_ok() {
                    sent += 1;
                }
            }
        }

        if sent > 0 {
            info!("Broadcast clipboard to {} LAN peers", sent);
        }

        Ok(())
    }

    /// Get a receiver for incoming clipboard items from LAN peers.
    pub async fn take_clipboard_receiver(&self) -> Option<mpsc::UnboundedReceiver<ClipboardItem>> {
        self.state.clipboard_rx.lock().await.take()
    }

    /// Trust a peer. Once trusted, clipboard and input events will be shared.
    pub async fn trust_peer(&self, device_id: &str) {
        self.state.trusted_peers.write().await.insert(device_id.to_string());
        info!("Trusted LAN peer: {}", device_id);
    }

    /// Remove trust from a peer.
    pub async fn untrust_peer(&self, device_id: &str) {
        self.state.trusted_peers.write().await.remove(device_id);
        info!("Untrusted LAN peer: {}", device_id);
    }

    /// Check if a peer is trusted.
    pub async fn is_trusted(&self, device_id: &str) -> bool {
        self.state.trusted_peers.read().await.contains(device_id)
    }

    /// Send a trust request to a specific peer.
    pub async fn send_trust_request(&self, target_device_id: &str) -> Result<(), String> {
        let peers = self.state.peers.read().await;
        if let Some(tx) = peers.get(target_device_id) {
            let event = SyncEvent::TrustRequest {
                device_id: self.device_id.clone(),
                device_name: self.device_name.clone(),
            };
            tx.send(event).map_err(|e| format!("send failed: {}", e))?;
            info!("Sent trust request to: {}", target_device_id);
            Ok(())
        } else {
            Err(format!("peer not connected: {}", target_device_id))
        }
    }

    /// Accept a trust request from a peer.
    pub async fn accept_trust(&self, target_device_id: &str) -> Result<(), String> {
        self.state.trusted_peers.write().await.insert(target_device_id.to_string());
        let peers = self.state.peers.read().await;
        if let Some(tx) = peers.get(target_device_id) {
            let event = SyncEvent::TrustAccept {
                device_id: self.device_id.clone(),
            };
            tx.send(event).ok();
            info!("Accepted trust from: {}", target_device_id);
            Ok(())
        } else {
            Err(format!("peer not connected: {}", target_device_id))
        }
    }

    /// Get list of trusted peer IDs.
    pub async fn trusted_peer_ids(&self) -> Vec<String> {
        self.state.trusted_peers.read().await.iter().cloned().collect()
    }

    /// Check if we have any LAN peers connected.
    pub async fn has_lan_peers(&self) -> bool {
        !self.state.peers.read().await.is_empty()
    }

    /// Get count of connected LAN peers.
    pub async fn peer_count(&self) -> usize {
        self.state.peers.read().await.len()
    }

    /// Get list of connected peer device IDs.
    pub async fn connected_peers(&self) -> Vec<String> {
        self.state.peers.read().await.keys().cloned().collect()
    }

    /// Stop the LAN sync engine.
    pub fn stop(&self) {
        self.state.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        info!("LAN sync engine stopping");
    }
}
