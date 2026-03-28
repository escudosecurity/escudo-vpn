mod blocklist;
mod config;
mod handler;
mod policy;
mod server;
mod stats;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use tracing::info;

use sqlx::postgres::PgPoolOptions;

use crate::blocklist::{new_blocklist, refresh_loop};
use crate::config::DnsConfig;
use crate::handler::{DnsMetrics, EscudoHandler};
use crate::policy::PolicyResolver;
use crate::stats::StatsRecorder;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "config/dns.toml")]
    config: PathBuf,
}

async fn metrics_handler(State(metrics): State<Arc<DnsMetrics>>) -> String {
    let queries = metrics.queries_total.load(Ordering::Relaxed);
    let blocked = metrics.blocked_total.load(Ordering::Relaxed);
    format!(
        "# HELP escudo_dns_queries_total Total DNS queries\n\
         # TYPE escudo_dns_queries_total counter\n\
         escudo_dns_queries_total {queries}\n\
         # HELP escudo_dns_blocked_total Total blocked DNS queries\n\
         # TYPE escudo_dns_blocked_total counter\n\
         escudo_dns_blocked_total {blocked}\n"
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let config: DnsConfig = escudo_common::config::load_config(&args.config)?;

    let blocklist = new_blocklist();
    let metrics = Arc::new(DnsMetrics::new());

    // Spawn blocklist refresh task
    let bl_clone = blocklist.clone();
    let sources = config.blocklist.sources.clone();
    let bl_interval = config.blocklist.refresh_interval_hours;
    tokio::spawn(async move {
        refresh_loop(bl_clone, sources, bl_interval).await;
    });

    // Spawn metrics HTTP server on port 9153
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .with_state(metrics_clone);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:9153")
            .await
            .expect("bind metrics port 9153");
        info!("DNS metrics server on 127.0.0.1:9153");
        axum::serve(listener, app)
            .await
            .expect("metrics server failed");
    });

    // Connect to database and start per-client stats recorder (if configured)
    let (stats_recorder, policy_resolver) = if let Some(db_config) = &config.database {
        let db = PgPoolOptions::new()
            .max_connections(3)
            .connect(&db_config.url)
            .await?;
        info!("DNS stats database connected");

        let recorder = StatsRecorder::new(db.clone());
        recorder.clone().spawn_flush_loop();
        (Some(recorder), Some(PolicyResolver::new(db)))
    } else {
        info!("No database configured; per-client DNS stats disabled");
        (None, None)
    };

    // Wait briefly for initial blocklist load
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let handler = EscudoHandler::new(blocklist, metrics, stats_recorder, policy_resolver)
        .map_err(|e| anyhow::anyhow!("Failed to create DNS handler: {e}"))?;
    let addr: SocketAddr = format!(
        "{}:{}",
        config.server.listen_addr, config.server.listen_port
    )
    .parse()?;

    info!("Starting Escudo DNS server");
    server::run_dns_server(handler, addr).await?;

    Ok(())
}
