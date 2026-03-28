mod config;
mod middleware;
mod router;
mod routes;
mod state;

use std::path::PathBuf;

use clap::Parser;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::config::AdminConfig;
use crate::state::AdminState;

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
    let config: AdminConfig = escudo_common::config::load_config(&args.config)?;
    config.validate_runtime()?;

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database.url)
        .await?;

    info!("Connected to database");

    let skip_migrations = std::env::var("ESCUDO_SKIP_MIGRATIONS")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    if skip_migrations {
        info!("Skipping automatic migrations due to ESCUDO_SKIP_MIGRATIONS");
    } else {
        let migrator = Migrator::new(std::path::Path::new("../../migrations")).await?;
        migrator.run(&db).await?;
    }

    let state = AdminState {
        db,
        jwt_secret: config.jwt.secret.clone(),
    };

    let app = router::create_router(state);

    // Admin on port 3001, bind to localhost only
    let listen_addr = "127.0.0.1:3001";
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    info!("Admin server listening on {listen_addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
