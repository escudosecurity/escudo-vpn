use std::net::SocketAddr;
use std::time::Duration;

use hickory_server::ServerFuture;
use tokio::net::{TcpListener, UdpSocket};
use tracing::info;

use crate::handler::EscudoHandler;

pub async fn run_dns_server(handler: EscudoHandler, addr: SocketAddr) -> anyhow::Result<()> {
    let mut server = ServerFuture::new(handler);

    let udp_socket = UdpSocket::bind(addr).await?;
    info!("DNS UDP listening on {addr}");
    server.register_socket(udp_socket);

    let tcp_listener = TcpListener::bind(addr).await?;
    info!("DNS TCP listening on {addr}");
    server.register_listener(tcp_listener, Duration::from_secs(30));

    server.block_until_done().await?;
    Ok(())
}
