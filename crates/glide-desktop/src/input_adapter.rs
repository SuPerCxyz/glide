use std::sync::Arc;
use tokio::sync::RwLock;

use glide_core::input_event::{InputEvent, InputEventKind, InputRoute, InputSession};
use glide_core::policy::Policy;

/// Trait for platform-specific input injection.
#[async_trait::async_trait]
pub trait InputBackend: Send + Sync {
    /// Inject a keyboard event.
    async fn inject_key(
        &self,
        key_code: &str,
        pressed: bool,
        modifiers: &[String],
    ) -> anyhow::Result<()>;
    /// Inject a mouse button event.
    async fn inject_mouse_button(
        &self,
        button: &str,
        pressed: bool,
        x: i32,
        y: i32,
    ) -> anyhow::Result<()>;
    /// Inject a mouse move event.
    async fn inject_mouse_move(
        &self,
        x: i32,
        y: i32,
        dx: Option<i32>,
        dy: Option<i32>,
    ) -> anyhow::Result<()>;
    /// Inject a mouse scroll event.
    async fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> anyhow::Result<()>;
    /// Get current cursor position.
    async fn cursor_position(&self) -> anyhow::Result<(i32, i32)>;
    /// Get screen dimensions.
    async fn screen_size(&self) -> anyhow::Result<(i32, i32)>;
}

/// Edge crossing detection for multi-monitor input sharing.
pub struct EdgeCrossingDetector {
    pub target_edge: EdgeDirection,
    pub current_screen: (i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeCrossingDetector {
    pub fn new(target_edge: EdgeDirection, screen_size: (i32, i32)) -> Self {
        Self {
            target_edge,
            current_screen: screen_size,
        }
    }

    /// Check if the cursor position crosses the edge to the target screen.
    pub fn is_crossing(&self, x: i32, y: i32) -> bool {
        match self.target_edge {
            EdgeDirection::Left => x <= 0,
            EdgeDirection::Right => x >= self.current_screen.0,
            EdgeDirection::Top => y <= 0,
            EdgeDirection::Bottom => y >= self.current_screen.1,
        }
    }
}

/// Input sharing module with safeguard controls.
pub struct InputSharing {
    pub session: RwLock<Option<InputSession>>,
    pub policy: Arc<Policy>,
    pub backend: Arc<dyn InputBackend>,
    pub heartbeat_interval_ms: u64,
    pub max_latency_ms: u64,
    pub rate_limit: Arc<RwLock<RateLimiter>>,
}

/// Rate limiter for input events.
pub struct RateLimiter {
    max_events_per_second: u64,
    events: Vec<std::time::Instant>,
}

impl RateLimiter {
    pub fn new(max_events_per_second: u64) -> Self {
        Self {
            max_events_per_second,
            events: Vec::with_capacity(max_events_per_second as usize),
        }
    }

    pub fn allow(&mut self) -> bool {
        let now = std::time::Instant::now();
        // Remove events older than 1 second.
        self.events.retain(|t| now.duration_since(*t).as_secs() < 1);
        if self.events.len() < self.max_events_per_second as usize {
            self.events.push(now);
            true
        } else {
            false
        }
    }
}

impl InputSharing {
    pub fn new(policy: Arc<Policy>, backend: Arc<dyn InputBackend>) -> Self {
        Self {
            session: RwLock::new(None),
            policy,
            backend,
            heartbeat_interval_ms: 1000,
            max_latency_ms: 200,
            rate_limit: Arc::new(RwLock::new(RateLimiter::new(1000))), // 1000 events/sec
        }
    }

    /// Start an input sharing session.
    pub async fn start_session(
        &self,
        controller_id: &str,
        target_id: &str,
        route: InputRoute,
    ) -> anyhow::Result<String> {
        let target_uuid = target_id.parse().unwrap_or_else(|_| uuid::Uuid::nil());
        if !self.policy.allows_input(&target_uuid) {
            anyhow::bail!("Input sharing not allowed with device {}", target_id);
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let session = InputSession {
            session_id: session_id.clone(),
            controller_id: controller_id.to_string(),
            target_id: target_id.to_string(),
            route,
            active: true,
            latency_ms: None,
            started_at: chrono::Utc::now().timestamp_millis(),
        };

        let mut lock = self.session.write().await;
        *lock = Some(session);

        Ok(session_id)
    }

    /// Process an incoming input event.
    pub async fn process_event(&self, event: InputEvent) -> anyhow::Result<()> {
        let session = self.session.read().await;
        let session = session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active input session"))?;

        if !session.active {
            anyhow::bail!("Input session is not active");
        }

        // Rate limiting check.
        let mut rl = self.rate_limit.write().await;
        if !rl.allow() {
            anyhow::bail!("Rate limit exceeded");
        }

        // Latency check for relay mode.
        if let InputRoute::ServerRelay = event.route {
            if let Some(latency) = session.latency_ms {
                if latency > self.max_latency_ms {
                    anyhow::bail!("Input relay latency too high: {}ms", latency);
                }
            }
        }

        // Inject the event.
        match event.event {
            InputEventKind::Key {
                key_code,
                pressed,
                modifiers,
            } => {
                self.backend
                    .inject_key(&key_code, pressed, &modifiers)
                    .await?;
            }
            InputEventKind::MouseButton {
                button,
                pressed,
                x,
                y,
            } => {
                self.backend
                    .inject_mouse_button(&button, pressed, x, y)
                    .await?;
            }
            InputEventKind::MouseMove { x, y, dx, dy } => {
                self.backend.inject_mouse_move(x, y, dx, dy).await?;
            }
            InputEventKind::MouseScroll { dx, dy } => {
                self.backend.inject_mouse_scroll(dx, dy).await?;
            }
            InputEventKind::EmergencyRelease => {
                self.emergency_release().await?;
            }
        }

        Ok(())
    }

    /// Emergency release: disconnect all input sharing.
    pub async fn emergency_release(&self) -> anyhow::Result<()> {
        let mut lock = self.session.write().await;
        if let Some(ref mut session) = *lock {
            session.active = false;
        }
        Ok(())
    }

    /// Update measured latency.
    pub async fn update_latency(&self, latency_ms: u64) {
        let mut lock = self.session.write().await;
        if let Some(ref mut session) = *lock {
            session.latency_ms = Some(latency_ms);
        }
    }

    /// Stop the session.
    pub async fn stop_session(&self) {
        let mut lock = self.session.write().await;
        if let Some(ref mut session) = *lock {
            session.active = false;
        }
    }
}
