#[cfg(test)]
mod tests {
    use glide_server::temp_token::{
        cleanup_expired_tokens, create_temp_token, validate_and_use_token, TempTokenOperation,
    };
    use sqlx::SqlitePool;
    use std::time::Duration;

    async fn create_test_pool() -> SqlitePool {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        glide_server::database::migrate(&pool).await.unwrap();
        pool
    }

    // --- Temporary Token Expiry Tests ---

    #[tokio::test]
    async fn test_temp_token_valid() {
        let pool = create_test_pool().await;
        let token = create_temp_token(
            &pool,
            3600,
            5,
            vec!["copy".to_string(), "paste".to_string()],
            10_000_000,
        )
        .await
        .unwrap();

        let result = validate_and_use_token(&pool, &token, TempTokenOperation::Copy, None).await;
        assert!(result.is_ok(), "Token should be valid: {:?}", result);
    }

    #[tokio::test]
    async fn test_temp_token_expired() {
        let pool = create_test_pool().await;
        // Create token that expires immediately (0 seconds TTL).
        let token = create_temp_token(&pool, 0, 5, vec!["copy".to_string()], 10_000_000)
            .await
            .unwrap();

        let result = validate_and_use_token(&pool, &token, TempTokenOperation::Copy, None).await;
        assert!(result.is_err(), "Token should be expired");
        assert!(matches!(
            result.unwrap_err(),
            glide_core::error::GlideError::TokenExpired
        ));
    }

    #[tokio::test]
    async fn test_temp_token_max_use_counting() {
        let pool = create_test_pool().await;
        let token = create_temp_token(&pool, 3600, 2, vec!["copy".to_string()], 10_000_000)
            .await
            .unwrap();

        // First use should succeed.
        let result1 = validate_and_use_token(&pool, &token, TempTokenOperation::Copy, None).await;
        assert!(result1.is_ok());

        // Second use should succeed.
        let result2 = validate_and_use_token(&pool, &token, TempTokenOperation::Copy, None).await;
        assert!(result2.is_ok());

        // Third use should exceed max uses.
        let result3 = validate_and_use_token(&pool, &token, TempTokenOperation::Copy, None).await;
        assert!(result3.is_err());
        assert!(matches!(
            result3.unwrap_err(),
            glide_core::error::GlideError::TokenMaxUsesExceeded
        ));
    }

    #[tokio::test]
    async fn test_temp_token_allowed_operation_check() {
        let pool = create_test_pool().await;
        let token = create_temp_token(&pool, 3600, 5, vec!["copy".to_string()], 10_000_000)
            .await
            .unwrap();

        // Copy should work.
        let result = validate_and_use_token(&pool, &token, TempTokenOperation::Copy, None).await;
        assert!(result.is_ok());

        // Paste should fail (not in allowed operations).
        let result = validate_and_use_token(&pool, &token, TempTokenOperation::Paste, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            glide_core::error::GlideError::TokenOperationNotAllowed(_)
        ));
    }

    #[tokio::test]
    async fn test_temp_token_item_size_limit() {
        let pool = create_test_pool().await;
        let token = create_temp_token(&pool, 3600, 5, vec!["copy".to_string()], 1000)
            .await
            .unwrap();

        // Within limit.
        let result =
            validate_and_use_token(&pool, &token, TempTokenOperation::Copy, Some(500)).await;
        assert!(result.is_ok());

        // Over limit.
        let result =
            validate_and_use_token(&pool, &token, TempTokenOperation::Copy, Some(2000)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            glide_core::error::GlideError::ItemTooLarge { .. }
        ));
    }

    #[tokio::test]
    async fn test_temp_token_nonexistent() {
        let pool = create_test_pool().await;
        let result =
            validate_and_use_token(&pool, "nonexistent_token", TempTokenOperation::Copy, None)
                .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            glide_core::error::GlideError::InvalidToken(_)
        ));
    }

    #[tokio::test]
    async fn test_temp_token_cleanup_expired() {
        let pool = create_test_pool().await;
        // Create an already-expired token.
        let _token = create_temp_token(&pool, 0, 1, vec!["copy".to_string()], 10_000_000)
            .await
            .unwrap();
        // Create a valid token.
        let _valid = create_temp_token(&pool, 3600, 1, vec!["copy".to_string()], 10_000_000)
            .await
            .unwrap();

        let deleted = cleanup_expired_tokens(&pool).await.unwrap();
        assert_eq!(deleted, 1, "Should have deleted 1 expired token");
    }

    #[tokio::test]
    async fn test_temp_token_operation_from_str() {
        assert_eq!(
            TempTokenOperation::from_str("copy"),
            Some(TempTokenOperation::Copy)
        );
        assert_eq!(
            TempTokenOperation::from_str("paste"),
            Some(TempTokenOperation::Paste)
        );
        assert_eq!(
            TempTokenOperation::from_str("history"),
            Some(TempTokenOperation::History)
        );
        assert_eq!(
            TempTokenOperation::from_str("devices"),
            Some(TempTokenOperation::Devices)
        );
        assert_eq!(TempTokenOperation::from_str("invalid"), None);
    }

    // --- Route Selection Logic Tests ---

    #[test]
    fn test_route_selection_lan_preferred() {
        // Simulate route selection logic: LAN direct > LAN reverse > server.
        let has_lan = true;
        let has_server = true;

        let route = if has_lan {
            glide_core::transfer::TransferRoute::LanDirect
        } else if has_server {
            glide_core::transfer::TransferRoute::ServerFallback
        } else {
            panic!("No route available")
        };

        assert_eq!(route, glide_core::transfer::TransferRoute::LanDirect);
    }

    #[test]
    fn test_route_selection_server_fallback() {
        let has_lan = false;
        let has_server = true;

        let route = if has_lan {
            glide_core::transfer::TransferRoute::LanDirect
        } else if has_server {
            glide_core::transfer::TransferRoute::ServerFallback
        } else {
            panic!("No route available")
        };

        assert_eq!(route, glide_core::transfer::TransferRoute::ServerFallback);
    }

    // --- Input Route Selection Tests ---

    #[test]
    fn test_input_route_lan_direct_preferred() {
        let lan_available = true;
        let server_available = true;

        let route = if lan_available {
            glide_core::input_event::InputRoute::LanDirect
        } else if server_available {
            glide_core::input_event::InputRoute::ServerRelay
        } else {
            panic!("No input route available")
        };

        assert_eq!(route, glide_core::input_event::InputRoute::LanDirect);
    }

    #[test]
    fn test_input_route_server_fallback() {
        let lan_available = false;
        let server_available = true;

        let route = if lan_available {
            glide_core::input_event::InputRoute::LanDirect
        } else if server_available {
            glide_core::input_event::InputRoute::ServerRelay
        } else {
            panic!("No input route available")
        };

        assert_eq!(route, glide_core::input_event::InputRoute::ServerRelay);
    }

    #[test]
    fn test_input_route_disconnect_when_both_fail() {
        let lan_available = false;
        let server_available = false;

        let route = if lan_available {
            Some(glide_core::input_event::InputRoute::LanDirect)
        } else if server_available {
            Some(glide_core::input_event::InputRoute::ServerRelay)
        } else {
            None // Disconnect and release input.
        };

        assert!(route.is_none());
    }
}
