use convex_autobackup_core::{Error, Result, ResultContext};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                "info".parse().map_err(|error| {
                    Error::with_source("default log directive is invalid", error)
                })?,
            ),
        )
        .init();

    let addr: SocketAddr = std::env::var("CONVEX_AUTOBACKUP_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8976".to_string())
        .parse()
        .context("CONVEX_AUTOBACKUP_BIND must be host:port")?;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "starting ConvexAutoBackup server");
    axum::serve(listener, convex_autobackup_server::router()?).await?;
    Ok(())
}
