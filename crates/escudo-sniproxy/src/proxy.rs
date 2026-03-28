use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use crate::sni::extract_sni;

pub struct ProxyHandler {
    streaming_domains: HashSet<String>,
    vpn_bind_ip: String,
}

impl ProxyHandler {
    pub fn new(domains: Vec<String>, vpn_bind_ip: String) -> Self {
        let streaming_domains: HashSet<String> = domains.into_iter().collect();
        Self {
            streaming_domains,
            vpn_bind_ip,
        }
    }

    fn matches_streaming(&self, hostname: &str) -> bool {
        let lower = hostname.to_lowercase();
        self.streaming_domains
            .iter()
            .any(|domain| lower == *domain || lower.ends_with(&format!(".{domain}")))
    }

    pub async fn handle_connection(self: &Arc<Self>, client: TcpStream, client_addr: SocketAddr) {
        // Peek at the TLS ClientHello to extract SNI
        let mut buf = vec![0u8; 4096];
        let n = match client.peek(&mut buf).await {
            Ok(n) if n > 0 => n,
            _ => {
                debug!("Failed to peek from {client_addr}");
                return;
            }
        };

        let sni = extract_sni(&buf[..n]);
        let hostname = match &sni {
            Some(h) => h.as_str(),
            None => {
                debug!("No SNI from {client_addr}, passing through directly");
                return;
            }
        };

        let via_vpn = self.matches_streaming(hostname);
        debug!(
            "SNI: {hostname} from {client_addr} -> {}",
            if via_vpn { "VPN" } else { "direct" }
        );

        // Resolve the hostname to connect upstream
        let upstream_addr = format!("{hostname}:443");

        let upstream = if via_vpn {
            // Bind to VPN interface IP
            let bind_addr: SocketAddr = format!("{}:0", self.vpn_bind_ip)
                .parse()
                .expect("valid bind addr");
            let socket = match tokio::net::TcpSocket::new_v4() {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create socket: {e}");
                    return;
                }
            };
            if let Err(e) = socket.bind(bind_addr) {
                error!("Failed to bind to VPN IP {}: {e}", self.vpn_bind_ip);
                return;
            }

            // Resolve and connect
            let addrs: Vec<SocketAddr> = match tokio::net::lookup_host(&upstream_addr).await {
                Ok(addrs) => addrs.collect(),
                Err(e) => {
                    error!("DNS lookup failed for {hostname}: {e}");
                    return;
                }
            };
            let target = match addrs.first() {
                Some(a) => *a,
                None => {
                    error!("No addresses for {hostname}");
                    return;
                }
            };

            match socket.connect(target).await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("Failed to connect to {hostname} via VPN: {e}");
                    return;
                }
            }
        } else {
            match TcpStream::connect(&upstream_addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("Failed to connect to {hostname}: {e}");
                    return;
                }
            }
        };

        // Bidirectional copy
        let (mut client_read, mut client_write) = client.into_split();
        let (mut upstream_read, mut upstream_write) = upstream.into_split();

        let c2u = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut client_read, &mut upstream_write).await;
            let _ = upstream_write.shutdown().await;
        });
        let u2c = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut upstream_read, &mut client_write).await;
            let _ = client_write.shutdown().await;
        });

        let _ = tokio::join!(c2u, u2c);
        info!("Connection closed: {client_addr} -> {hostname}");
    }
}
