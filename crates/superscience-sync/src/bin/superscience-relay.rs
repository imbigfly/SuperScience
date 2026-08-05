use anyhow::{Context, Result};
use superscience_sync::{relay_router, FileRelay, RelayHttpState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "superscience_sync=info,superscience_relay=info".into()),
        )
        .init();
    let root = std::env::var_os("SUPERSCIENCE_RELAY_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("./superscience-relay-data"));
    let token = std::env::var("SUPERSCIENCE_RELAY_TOKEN")
        .context("SUPERSCIENCE_RELAY_TOKEN must be set to a strong random bearer token")?;
    let bind = std::env::var("SUPERSCIENCE_RELAY_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let relay = FileRelay::open(root).await?;
    let state = RelayHttpState::new(relay, token)?;
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "SuperScience relay listening");
    axum::serve(listener, relay_router(state)).await?;
    Ok(())
}
