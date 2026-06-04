/// Glide server — Docker-deployable central server for clipboard relay,
/// device registration, and temporary token authentication.

pub mod database;
pub mod handlers;
pub mod models;
pub mod state;
pub mod temp_token;
pub mod cleanup;
pub mod input_relay;
pub mod ws;

use anyhow::Result;
use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::database::create_pool;
use crate::state::ServerState;

/// Run the Glide server.
pub async fn run(listen_addr: &str) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("glide_server=info".parse()?),
        )
        .init();

    info!("Starting Glide server on {}", listen_addr);

    let data_dir = std::env::var("GLIDE_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    std::fs::create_dir_all(&data_dir)?;

    let db_url = format!("{}/glide.db", data_dir);
    let pool = create_pool(&db_url).await?;
    database::migrate(&pool).await?;

    let state = ServerState::new(pool, data_dir);

    let app = Router::new()
        .merge(handlers::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    info!("Listening on {}", listen_addr);

    axum::serve(listener, app).await?;
    Ok(())
}
