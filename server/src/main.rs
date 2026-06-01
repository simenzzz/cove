use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

use nexus_server::config::AppConfig;
use nexus_server::{build_router, build_state};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structured JSON logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    // Initialize Prometheus metrics exporter (installs the global recorder —
    // must happen exactly once per process).
    let prometheus_handle =
        metrics_exporter_prometheus::PrometheusBuilder::new().install_recorder()?;

    // Load .env if present (no-op in production where env is provided).
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;

    let state = build_state(config, prometheus_handle).await?;
    let addr = format!("{}:{}", state.config.server_host, state.config.server_port);
    let app = build_router(state);

    tracing::info!("Nexus server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
