#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    use glide_core::clipboard::{ClipboardItem, ClipboardKind, DeliveryPolicy, SessionType};
    use glide_core::discovery::{DiscoveredPeer, PeerRegistry, PeerState};
    use glide_core::route::ClipboardRouteSelector;
    use glide_core::transfer::TransferRoute;

    fn make_test_item(device_id: &str, text: &str) -> ClipboardItem {
        ClipboardItem {
            item_id: uuid::Uuid::new_v4().to_string(),
            source_device_id: device_id.to_string(),
            source_session_type: SessionType::Persistent,
            kind: ClipboardKind::Text,
            representations: vec![glide_core::mime_rep::MimeRepresentation {
                mime_type: "text/plain".to_string(),
                content: glide_core::mime_rep::RepresentationContent::Text(text.to_string()),
            }],
            size: text.len() as u64,
            created_at: chrono::Utc::now().timestamp_millis(),
            payload_refs: vec![],
            checksum: "test".to_string(),
            delivery_policy: DeliveryPolicy::Broadcast,
        }
    }

    // --- LAN Sync Tests ---

    #[test]
    fn test_lan_sync_route_lan_direct() {
        let mut registry = PeerRegistry::default();
        let addr: std::net::SocketAddr = "192.168.1.100:9998".parse().unwrap();
        registry.upsert("peer-1".to_string(), "Laptop".to_string(), addr);

        let selector = ClipboardRouteSelector::new(
            "my-device".to_string(),
            true,  // LAN available
            false, // No server
        )
        .with_registry(registry);

        let result = selector.select_route("peer-1");
        assert_eq!(result.route, TransferRoute::LanDirect);
        assert!(result.found_via_lan);
        assert!(result.target_address.is_some());
    }

    #[test]
    fn test_lan_sync_route_server_fallback() {
        let registry = PeerRegistry::default(); // No peers.

        let selector = ClipboardRouteSelector::new(
            "my-device".to_string(),
            true,
            true, // Server available as fallback.
        )
        .with_registry(registry);

        let result = selector.select_route("unknown-peer");
        assert_eq!(result.route, TransferRoute::ServerFallback);
        assert!(!result.found_via_lan);
    }

    #[test]
    fn test_lan_sync_loop_prevention() {
        let selector = ClipboardRouteSelector::new("my-device".to_string(), true, true);

        let result = selector.select_route("my-device");
        // Should not sync to self.
        assert!(!result.found_via_lan);
    }

    // --- Peer Registry Tests ---

    #[test]
    fn test_peer_registry_discovery_and_removal() {
        let mut registry = PeerRegistry::default();
        let addr: std::net::SocketAddr = "192.168.1.100:9998".parse().unwrap();

        // Discover a peer.
        registry.upsert("peer-1".to_string(), "Laptop".to_string(), addr);
        assert!(registry.is_active("peer-1"));
        assert_eq!(registry.active_count(), 1);

        // Remove it.
        registry.remove("peer-1");
        assert!(!registry.is_active("peer-1"));
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_peer_registry_update_existing() {
        let mut registry = PeerRegistry::default();
        let addr1: std::net::SocketAddr = "192.168.1.100:9998".parse().unwrap();
        let addr2: std::net::SocketAddr = "192.168.1.101:9998".parse().unwrap();

        registry.upsert("peer-1".to_string(), "Laptop".to_string(), addr1);
        registry.upsert("peer-1".to_string(), "Desktop".to_string(), addr2);

        let peer = registry.get("peer-1").unwrap();
        assert_eq!(peer.name, "Desktop");
        assert_eq!(peer.address.port(), 9998);
    }

    #[test]
    fn test_peer_registry_multiple_peers() {
        let mut registry = PeerRegistry::default();

        registry.upsert(
            "peer-1".to_string(),
            "Laptop".to_string(),
            "192.168.1.100:9998".parse().unwrap(),
        );
        registry.upsert(
            "peer-2".to_string(),
            "Desktop".to_string(),
            "192.168.1.101:9998".parse().unwrap(),
        );
        registry.upsert(
            "peer-3".to_string(),
            "Phone".to_string(),
            "192.168.1.102:9998".parse().unwrap(),
        );

        assert_eq!(registry.active_count(), 3);
        assert_eq!(registry.all_peers().len(), 3);
    }

    // --- Clipboard Item Tests ---

    #[test]
    fn test_clipboard_item_creation() {
        let item = make_test_item("dev-1", "hello world");
        assert_eq!(item.kind, ClipboardKind::Text);
        assert_eq!(item.size, 11);
        assert_eq!(item.source_device_id, "dev-1");
    }

    #[test]
    fn test_clipboard_item_serialization_roundtrip() {
        let item = make_test_item("dev-1", "test content");
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: ClipboardItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.item_id, item.item_id);
        assert_eq!(deserialized.kind, ClipboardKind::Text);
    }

    // --- Route Selection with Multiple Peers ---

    #[test]
    fn test_route_select_all_routes() {
        let mut registry = PeerRegistry::default();
        registry.upsert(
            "peer-1".to_string(),
            "Laptop".to_string(),
            "192.168.1.100:9998".parse().unwrap(),
        );
        registry.upsert(
            "peer-2".to_string(),
            "Desktop".to_string(),
            "192.168.1.101:9998".parse().unwrap(),
        );
        registry.upsert(
            "my-device".to_string(),
            "Self".to_string(),
            "192.168.1.102:9998".parse().unwrap(),
        );

        let selector = ClipboardRouteSelector::new("my-device".to_string(), true, true)
            .with_registry(registry);

        let routes = selector.select_all_routes(&[
            "my-device".to_string(),
            "peer-1".to_string(),
            "peer-2".to_string(),
        ]);

        // Should have 2 routes (skipping self).
        assert_eq!(routes.len(), 2);
        assert!(routes.iter().all(|(id, _)| id != "my-device"));
        assert!(routes
            .iter()
            .all(|(_, r)| r.route == TransferRoute::LanDirect));
    }

    // --- Delivery Policy Tests ---

    #[test]
    fn test_delivery_policy_broadcast() {
        let item = make_test_item("dev-1", "test");
        assert_eq!(item.delivery_policy, DeliveryPolicy::Broadcast);
    }

    #[test]
    fn test_delivery_policy_targeted() {
        let mut item = make_test_item("dev-1", "test");
        item.delivery_policy =
            DeliveryPolicy::Targeted(vec!["dev-2".to_string(), "dev-3".to_string()]);

        match &item.delivery_policy {
            DeliveryPolicy::Targeted(ids) => assert_eq!(ids.len(), 2),
            _ => panic!("Expected Targeted policy"),
        }
    }

    // --- Latency Tracker Tests ---

    #[test]
    fn test_latency_tracker() {
        let mut tracker = glide_core::route::LatencyTracker::new(0.5);
        tracker.record(100);
        assert_eq!(tracker.estimate(), Some(100));

        tracker.record(50);
        // EMA: 0.5 * 50 + 0.5 * 100 = 75
        assert_eq!(tracker.estimate(), Some(75));

        assert!(tracker.exceeds(60));
        assert!(!tracker.exceeds(100));
    }
}
