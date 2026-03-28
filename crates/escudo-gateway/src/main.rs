mod config;
mod grpc;
mod health;
mod proxy;
mod stats;
mod wg;

use std::future::pending;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use tonic::transport::Server as TonicServer;
use tracing::info;

use crate::config::GatewayConfig;
use crate::grpc::gateway::gateway_service_server::GatewayServiceServer;
use crate::grpc::GatewayServiceImpl;
use crate::proxy::ProxyManager;
use crate::stats::Metrics;
use crate::wg::MultiWgManager;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "config/gateway.toml")]
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
    let config: GatewayConfig = escudo_common::config::load_config(&args.config)?;

    let wg = Arc::new(MultiWgManager::new(
        &config.wireguard.interface,
        &config.wireguard.wg1_interface,
        &config.wireguard.wg2_interface,
    ));
    let proxy = match config.proxy.clone() {
        Some(proxy_cfg) if proxy_cfg.enabled => Some(Arc::new(ProxyManager::new(proxy_cfg)?)),
        _ => None,
    };
    let start_time = Instant::now();
    let metrics = Metrics::new();

    let grpc_addr: std::net::SocketAddr = config.server.grpc_addr.parse()?;
    let health_addr: std::net::SocketAddr = config.server.health_addr.parse()?;

    let gateway_service = GatewayServiceImpl {
        wg: wg.clone(),
        start_time,
        metrics: metrics.clone(),
        proxy: proxy.clone(),
    };

    info!("Starting gRPC server on {}", config.server.grpc_addr);
    info!("Starting health server on {}", config.server.health_addr);

    let grpc_server = TonicServer::builder()
        .add_service(GatewayServiceServer::new(gateway_service))
        .serve(grpc_addr);

    let health_server = axum::serve(
        tokio::net::TcpListener::bind(health_addr).await?,
        health::health_router(metrics.clone()),
    );

    let stats_task = tokio::spawn(stats::stats_collector(
        wg.clone(),
        metrics,
        config.stats.collection_interval_secs,
    ));
    let proxy_task = proxy
        .and_then(|manager| manager.spawn_poller())
        .unwrap_or_else(|| tokio::spawn(async { pending::<()>().await }));

    tokio::select! {
        r = grpc_server => { r?; }
        r = health_server => { r?; }
        _ = stats_task => {}
        _ = proxy_task => {}
    }

    Ok(())
}
