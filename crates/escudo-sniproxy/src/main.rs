mod config;
mod proxy;
mod sni;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::SniProxyConfig;
use crate::proxy::ProxyHandler;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "config/sniproxy.toml")]
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
    let config_str = std::fs::read_to_string(&args.config)?;
    let config: SniProxyConfig = toml::from_str(&config_str)?;

    let listen_addr = format!("{}:{}", config.listen.addr, config.listen.port);
    let handler = Arc::new(ProxyHandler::new(
        config.streaming.domains,
        config.streaming.vpn_bind_ip,
    ));

    let listener = TcpListener::bind(&listen_addr).await?;
    info!("SNI proxy listening on {listen_addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        let handler = handler.clone();
        tokio::spawn(async move {
            handler.handle_connection(stream, addr).await;
        });
    }
}
