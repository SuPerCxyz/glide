use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use glide_core::input_event::InputEvent;
use glide_core::policy::Policy;

use crate::input_adapter::{EdgeCrossingDetector, InputSharing};
use crate::platform_input::create_platform_input_backend;

/// LAN input sharing engine - handles cross-node keyboard/mouse sharing.
///
/// Flow:
/// 1. Controller device captures local input events
/// 2. Events sent via WebSocket to target device
/// 3. Target device injects events via InputBackend
/// 4. Edge crossing detection switches control between devices
/// 5. Emergency release disconnects all input sharing
pub struct LanInputEngine {
    pub device_id: String,
    pub device_name: String,
    pub service_port: u16,
    /// Input sharing session.
    pub input_sharing: Option<Arc<InputSharing>>,
    /// Connected input peers.
    pub peers: Arc<RwLock<std::collections::HashMap<String, mpsc::UnboundedSender<InputEvent>>>>,
    /// Channel for incoming input events.
    pub incoming_rx: Arc<tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<InputEvent>>>>,
    pub incoming_tx: mpsc::UnboundedSender<InputEvent>,
    /// Whether we are currently controlling another device.
    pub is_controlling: Arc<RwLock<bool>>,
    /// Edge crossing detector.
    pub edge_detector: Option<EdgeCrossingDetector>,
    /// Whether the engine is running.
    pub running: Arc<std::sync::atomic::AtomicBool>,
}

impl LanInputEngine {
    pub fn new(device_id: String, device_name: String, service_port: u16) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        Self {
            device_id,
            device_name,
            service_port,
            input_sharing: None,
            peers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            incoming_rx: Arc::new(tokio::sync::Mutex::new(Some(incoming_rx))),
            incoming_tx,
            is_controlling: Arc::new(RwLock::new(false)),
            edge_detector: None,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the LAN input engine.
    pub async fn start(&self) -> anyhow::Result<()> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        info!("LAN input engine starting for device {}", self.device_id);

        // Start input event server.
        let listen_addr = format!("0.0.0.0:{}", self.service_port + 1);
        let incoming_tx = self.incoming_tx.clone();
        let running = self.running.clone();
        let device_id = self.device_id.clone();
        tokio::spawn(async move {
            use tokio::net::TcpListener;

            let listener = match TcpListener::bind(&listen_addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to bind input server on {}: {}", listen_addr, e);
                    return;
                }
            };

            info!("Input event server listening on {}", listen_addr);

            while running.load(std::sync::atomic::Ordering::SeqCst) {
                let (stream, addr) = match listener.accept().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Input accept error: {}", e);
                        continue;
                    }
                };

                info!("Input peer connected from {}", addr);
                let incoming_tx = incoming_tx.clone();
                let device_id = device_id.clone();

                tokio::spawn(async move {
                    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                        Ok(ws) => ws,
                        Err(e) => {
                            warn!("Input WebSocket accept error: {}", e);
                            return;
                        }
                    };

                    let (_, mut ws_rx) = ws_stream.split();

                    while let Some(Ok(msg)) = ws_rx.next().await {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            if let Ok(event) = serde_json::from_str::<InputEvent>(&text) {
                                // Don't echo our own events.
                                if event.source_device_id != device_id {
                                    let _ = incoming_tx.send(event);
                                }
                            }
                        }
                    }

                    info!("Input peer disconnected: {}", addr);
                });
            }
        });

        // Start input event consumer.
        let incoming_rx = self.incoming_rx.clone();

        tokio::spawn(async move {
            let mut rx = {
                let mut guard = incoming_rx.lock().await;
                guard.take()
            };

            if let Some(ref mut rx) = rx {
                let backend = match create_platform_input_backend() {
                    Ok(backend) => backend,
                    Err(e) => {
                        warn!("Input backend unavailable: {}", e);
                        return;
                    }
                };
                let input_sharing =
                    Arc::new(InputSharing::new(Arc::new(Policy::default()), backend));

                while let Some(event) = rx.recv().await {
                    if let Err(e) = input_sharing.process_event(event).await {
                        warn!("Failed to process input event: {}", e);
                    }
                }
            }
        });

        info!("LAN input engine started");
        Ok(())
    }

    /// Send an input event to a target peer.
    pub async fn send_input(
        &self,
        target_device_id: &str,
        event: InputEvent,
    ) -> anyhow::Result<()> {
        let peers = self.peers.read().await;
        if let Some(tx) = peers.get(target_device_id) {
            tx.send(event)
                .map_err(|_| anyhow::anyhow!("Failed to send to peer"))?;
        } else {
            return Err(anyhow::anyhow!("Peer {} not found", target_device_id));
        }
        Ok(())
    }

    /// Emergency release: disconnect all input sharing.
    pub async fn emergency_release(&self) -> anyhow::Result<()> {
        info!("Emergency release triggered");
        let mut is_ctrl = self.is_controlling.write().await;
        *is_ctrl = false;

        if let Some(ref sharing) = self.input_sharing {
            sharing.emergency_release().await?;
        }

        // Clear all peers.
        self.peers.write().await.clear();
        Ok(())
    }

    /// Take the incoming input event receiver.
    pub async fn take_incoming(&self) -> Option<mpsc::UnboundedReceiver<InputEvent>> {
        self.incoming_rx.lock().await.take()
    }

    /// Check if we are currently controlling another device.
    pub async fn is_controlling(&self) -> bool {
        *self.is_controlling.read().await
    }
}
