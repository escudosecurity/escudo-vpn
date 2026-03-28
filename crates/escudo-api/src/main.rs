mod backend_control;
mod config;
mod middleware;
mod qr;
mod router;
mod routes;
mod state;
mod telemetry;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::config::ApiConfig;
use crate::state::{gateway::gateway_service_client::GatewayServiceClient, AppState};

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "config/api.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let config: ApiConfig = escudo_common::config::load_config(&args.config)?;
    config.validate_runtime()?;

    // Connect to database
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database.url)
        .await?;

    info!("Connected to database");

    // Run migrations
    let skip_migrations = std::env::var("ESCUDO_SKIP_MIGRATIONS")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    if skip_migrations {
        info!("Skipping automatic migrations due to ESCUDO_SKIP_MIGRATIONS");
    } else {
        let migrator = Migrator::new(std::path::Path::new("../../migrations")).await?;
        migrator.run(&db).await?;
        info!("Migrations applied");
    }

    // Connect to gateway gRPC
    let gateway = GatewayServiceClient::connect(config.gateway.grpc_addr.clone()).await?;
    info!("Connected to gateway gRPC at {}", config.gateway.grpc_addr);

    let state = AppState {
        db,
        gateway,
        config: Arc::new(config.clone()),
    };

    let app = router::create_router(state);

    let listener = tokio::net::TcpListener::bind(&config.server.addr).await?;
    info!("API server listening on {}", config.server.addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
