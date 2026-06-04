#[cfg(test)]
mod tests {
    use glide_core::clipboard::{ClipboardItem, ClipboardKind, DeliveryPolicy, SessionType};
    use glide_core::mime_rep::{MimeRepresentation, RepresentationContent, mime_types};
    use glide_core::payload::PayloadRef;
    use glide_core::device::{Device, Platform, RegistrationType};
    use glide_core::transfer::{TransferRoute, TransferState, TransferSession};
    use glide_core::sync_event::SyncEvent;
    use glide_core::input_event::{InputEvent, InputEventKind, InputRoute, InputSession};
    use glide_core::policy::{Policy, PolicyAction, DevicePolicy, TypePolicy};

    // --- Serialization/Deserialization Tests ---

    #[test]
    fn test_clipboard_item_serialize_deserialize() {
        let item = ClipboardItem {
            item_id: "test-item-1".to_string(),
            source_device_id: "device-1".to_string(),
            source_session_type: SessionType::Persistent,
            kind: ClipboardKind::Text,
            representations: vec![MimeRepresentation {
                mime_type: mime_types::TEXT_PLAIN.to_string(),
                content: RepresentationContent::Text("hello world".to_string()),
            }],
            size: 11,
            created_at: 1700000000000,
            payload_refs: vec![],
            checksum: "abc123".to_string(),
            delivery_policy: DeliveryPolicy::Broadcast,
        };

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: ClipboardItem = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.item_id, "test-item-1");
        assert_eq!(deserialized.source_device_id, "device-1");
        assert_eq!(deserialized.kind, ClipboardKind::Text);
        assert_eq!(deserialized.representations.len(), 1);
    }

    #[test]
    fn test_clipboard_item_image_serialize() {
        let item = ClipboardItem {
            item_id: "img-1".to_string(),
            source_device_id: "dev-1".to_string(),
            source_session_type: SessionType::Persistent,
            kind: ClipboardKind::Image,
            representations: vec![MimeRepresentation {
                mime_type: mime_types::IMAGE_PNG.to_string(),
                content: RepresentationContent::InlineBase64("iVBORw0KGgo=".to_string()),
            }],
            size: 1024,
            created_at: 1700000000000,
            payload_refs: vec![],
            checksum: "def456".to_string(),
            delivery_policy: DeliveryPolicy::default(),
        };

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: ClipboardItem = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.kind, ClipboardKind::Image);
    }

    #[test]
    fn test_device_serialize() {
        let device = Device {
            device_id: uuid::Uuid::new_v4(),
            name: "Test Laptop".to_string(),
            platform: Platform::Linux,
            trusted: true,
            public_key_fingerprint: Some("sha256:abc".to_string()),
            lan_address: Some("192.168.1.100:9999".to_string()),
            last_seen_at: Some(1700000000000),
            created_at: 1700000000000,
        };

        let json = serde_json::to_string(&device).unwrap();
        let deserialized: Device = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Test Laptop");
        assert_eq!(deserialized.platform, Platform::Linux);
        assert!(deserialized.trusted);
    }

    #[test]
    fn test_transfer_session_serialize() {
        let session = TransferSession {
            session_id: "sess-1".to_string(),
            source_device_id: "dev-1".to_string(),
            target_device_id: "dev-2".to_string(),
            item_id: "item-1".to_string(),
            route: TransferRoute::LanDirect,
            state: TransferState::InProgress,
            started_at: 1700000000000,
            completed_at: None,
            error: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: TransferSession = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.route, TransferRoute::LanDirect);
        assert_eq!(deserialized.state, TransferState::InProgress);
    }

    #[test]
    fn test_sync_event_serialize() {
        let item = ClipboardItem {
            item_id: "evt-1".to_string(),
            source_device_id: "dev-1".to_string(),
            source_session_type: SessionType::Persistent,
            kind: ClipboardKind::Text,
            representations: vec![],
            size: 0,
            created_at: 1700000000000,
            payload_refs: vec![],
            checksum: "".to_string(),
            delivery_policy: DeliveryPolicy::Broadcast,
        };

        let event = SyncEvent::ClipboardCaptured { item };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: SyncEvent = serde_json::from_str(&json).unwrap();

        assert!(deserialized.clipboard_item_id().is_some());
        assert_eq!(deserialized.clipboard_item_id().unwrap(), "evt-1");
    }

    #[test]
    fn test_input_event_serialize() {
        let event = InputEvent {
            source_device_id: "dev-1".to_string(),
            timestamp: 1700000000000,
            event: InputEventKind::Key {
                key_code: "A".to_string(),
                pressed: true,
                modifiers: vec!["Ctrl".to_string()],
            },
            route: InputRoute::LanDirect,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: InputEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.route, InputRoute::LanDirect);
        match deserialized.event {
            InputEventKind::Key { key_code, pressed, .. } => {
                assert_eq!(key_code, "A");
                assert!(pressed);
            }
            _ => panic!("Expected Key event"),
        }
    }

    #[test]
    fn test_input_session_serialize() {
        let session = InputSession {
            session_id: "input-sess-1".to_string(),
            controller_id: "ctrl-1".to_string(),
            target_id: "target-1".to_string(),
            route: InputRoute::ServerRelay,
            active: true,
            latency_ms: Some(45),
            started_at: 1700000000000,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: InputSession = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.route, InputRoute::ServerRelay);
        assert_eq!(deserialized.latency_ms, Some(45));
    }

    // --- MIME Representation Selection Tests ---

    #[test]
    fn test_mime_text_plain() {
        let rep = MimeRepresentation {
            mime_type: mime_types::TEXT_PLAIN.to_string(),
            content: RepresentationContent::Text("hello".to_string()),
        };
        assert_eq!(rep.mime_type, "text/plain");
        match &rep.content {
            RepresentationContent::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_mime_html_rich_text() {
        let rep = MimeRepresentation {
            mime_type: mime_types::TEXT_HTML.to_string(),
            content: RepresentationContent::Text("<b>bold</b>".to_string()),
        };
        assert_eq!(rep.mime_type, "text/html");
    }

    #[test]
    fn test_mime_url() {
        let rep = MimeRepresentation {
            mime_type: mime_types::TEXT_URI_LIST.to_string(),
            content: RepresentationContent::Text("https://example.com".to_string()),
        };
        assert_eq!(rep.mime_type, "text/uri-list");
    }

    #[test]
    fn test_mime_image_png() {
        let rep = MimeRepresentation {
            mime_type: mime_types::IMAGE_PNG.to_string(),
            content: RepresentationContent::InlineBase64("iVBORw0KGgo=".to_string()),
        };
        assert_eq!(rep.mime_type, "image/png");
        match &rep.content {
            RepresentationContent::InlineBase64(_) => {}
            _ => panic!("Expected base64 content"),
        }
    }

    #[test]
    fn test_mime_payload_ref() {
        let rep = MimeRepresentation {
            mime_type: "application/octet-stream".to_string(),
            content: RepresentationContent::PayloadRef("payload-123".to_string()),
        };
        match &rep.content {
            RepresentationContent::PayloadRef(pid) => assert_eq!(pid, "payload-123"),
            _ => panic!("Expected payload ref"),
        }
    }

    // --- Policy Tests ---

    #[test]
    fn test_policy_default_allow() {
        let policy = Policy::default();
        let device_id = uuid::Uuid::new_v4();
        assert!(policy.allows_sync(&device_id, ClipboardKind::Text));
        assert!(policy.allows_sync(&device_id, ClipboardKind::Image));
        assert!(!policy.allows_input(&device_id)); // Input defaults to deny.
    }

    #[test]
    fn test_policy_device_override_deny() {
        let mut policy = Policy::default();
        let device_id = uuid::Uuid::new_v4();
        policy.device_policies.push(DevicePolicy {
            device_id,
            sync_enabled: false,
            input_enabled: false,
        });

        assert!(!policy.allows_sync(&device_id, ClipboardKind::Text));
    }

    #[test]
    fn test_policy_type_restriction() {
        let mut policy = Policy::default();
        let device_id = uuid::Uuid::new_v4();
        policy.type_policies.push(TypePolicy {
            kind: ClipboardKind::Image,
            sync_enabled: false,
            max_size_bytes: None,
        });

        assert!(policy.allows_sync(&device_id, ClipboardKind::Text));
        assert!(!policy.allows_sync(&device_id, ClipboardKind::Image));
        assert!(policy.allows_sync(&device_id, ClipboardKind::File));
    }

    #[test]
    fn test_policy_default_deny() {
        let mut policy = Policy {
            default_action: PolicyAction::Deny,
            device_policies: Vec::new(),
            type_policies: Vec::new(),
        };

        let device_id = uuid::Uuid::new_v4();
        assert!(!policy.allows_sync(&device_id, ClipboardKind::Text));
    }

    #[test]
    fn test_policy_max_size() {
        let mut policy = Policy::default();
        policy.type_policies.push(TypePolicy {
            kind: ClipboardKind::Image,
            sync_enabled: true,
            max_size_bytes: Some(5_000_000),
        });

        assert_eq!(policy.max_size_for_kind(ClipboardKind::Image), Some(5_000_000));
        assert_eq!(policy.max_size_for_kind(ClipboardKind::Text), None);
    }

    // --- Payload Ref Tests ---

    #[test]
    fn test_payload_ref_new() {
        let pref = PayloadRef::new(
            "pid-1".to_string(),
            "/api/v1/payload/pid-1".to_string(),
            1024,
            "sha256:abc".to_string(),
        );

        assert_eq!(pref.payload_id, "pid-1");
        assert_eq!(pref.size, 1024);
    }

    // --- SyncEvent clipboard_item_id Tests ---

    #[test]
    fn test_sync_event_clipboard_item_id() {
        let captured = SyncEvent::ClipboardCaptured {
            item: ClipboardItem {
                item_id: "item-x".to_string(),
                source_device_id: "dev-1".to_string(),
                source_session_type: SessionType::Persistent,
                kind: ClipboardKind::Text,
                representations: vec![],
                size: 0,
                created_at: 0,
                payload_refs: vec![],
                checksum: "".to_string(),
                delivery_policy: DeliveryPolicy::Broadcast,
            },
        };
        assert_eq!(captured.clipboard_item_id(), Some("item-x"));

        let heartbeat = SyncEvent::Heartbeat {
            device_id: "dev-1".to_string(),
            timestamp: 0,
        };
        assert!(heartbeat.clipboard_item_id().is_none());
    }

    // --- Delivery Policy Tests ---

    #[test]
    fn test_delivery_policy_targeted() {
        let targets = DeliveryPolicy::Targeted(vec!["dev-1".to_string(), "dev-2".to_string()]);
        let json = serde_json::to_string(&targets).unwrap();
        let deserialized: DeliveryPolicy = serde_json::from_str(&json).unwrap();
        match deserialized {
            DeliveryPolicy::Targeted(ids) => {
                assert_eq!(ids.len(), 2);
                assert_eq!(ids[0], "dev-1");
            }
            _ => panic!("Expected Targeted"),
        }
    }

    #[test]
    fn test_delivery_policy_local_only() {
        let policy = DeliveryPolicy::LocalOnly;
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: DeliveryPolicy = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, DeliveryPolicy::LocalOnly));
    }

    // --- Transfer Route Tests ---

    #[test]
    fn test_transfer_route_variants() {
        // Verify all route variants can be serialized.
        let routes = vec![
            TransferRoute::LanDirect,
            TransferRoute::LanReversePull,
            TransferRoute::ServerFallback,
        ];

        for route in &routes {
            let json = serde_json::to_string(route).unwrap();
            let deserialized: TransferRoute = serde_json::from_str(&json).unwrap();
            assert_eq!(*route, deserialized);
        }
    }

    // --- Input Route Tests ---

    #[test]
    fn test_input_route_variants() {
        let routes = vec![
            InputRoute::LanDirect,
            InputRoute::ServerRelay,
        ];

        for route in &routes {
            let json = serde_json::to_string(route).unwrap();
            let deserialized: InputRoute = serde_json::from_str(&json).unwrap();
            assert_eq!(*route, deserialized);
        }
    }

    // --- Input Event Kind Tests ---

    #[test]
    fn test_input_event_mouse_move() {
        let event = InputEventKind::MouseMove {
            x: 100,
            y: 200,
            dx: Some(10),
            dy: Some(-5),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: InputEventKind = serde_json::from_str(&json).unwrap();
        match deserialized {
            InputEventKind::MouseMove { x, y, dx, dy } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(dx, Some(10));
                assert_eq!(dy, Some(-5));
            }
            _ => panic!("Expected MouseMove"),
        }
    }

    #[test]
    fn test_input_event_mouse_scroll() {
        let event = InputEventKind::MouseScroll { dx: 0, dy: -3 };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: InputEventKind = serde_json::from_str(&json).unwrap();
        match deserialized {
            InputEventKind::MouseScroll { dx, dy } => {
                assert_eq!(dx, 0);
                assert_eq!(dy, -3);
            }
            _ => panic!("Expected MouseScroll"),
        }
    }

    #[test]
    fn test_input_event_emergency_release() {
        let event = InputEventKind::EmergencyRelease;
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: InputEventKind = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, InputEventKind::EmergencyRelease));
    }
}
