use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let listen_addr = std::env::var("GLIDE_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let data_dir = std::env::var("GLIDE_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let db_url = format!("{}/glide.db", data_dir);

    // Start periodic cleanup task.
    let pool = glide_server::database::create_pool(&db_url).await?;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            match glide_server::cleanup::run_cleanup(&pool).await {
                Ok(result) => {
                    tracing::info!(
                        "Cleanup: deleted {} items, freed {} bytes",
                        result.items_deleted,
                        result.bytes_freed
                    );
                }
                Err(e) => {
                    tracing::error!("Cleanup failed: {}", e);
                }
            }
        }
    });

    glide_server::run(&listen_addr).await
}
