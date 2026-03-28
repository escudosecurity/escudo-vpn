use boringtun::noise::{Tunn, TunnResult};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Configuration for tunnel features (DAITA, multihop).
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Enable Defence Against AI-guided Traffic Analysis (DAITA).
    /// When true, the tunnel pads packets to uniform sizes and injects
    /// dummy traffic to resist traffic fingerprinting.
    /// Full DAITA support requires the GotaTun backend; this flag
    /// controls the placeholder padding logic until the dependency swap.
    pub daita_enabled: bool,

    /// Enable multihop routing through an entry and exit relay.
    /// When true, traffic is encapsulated twice: first for the exit
    /// relay, then for the entry relay, so the entry never sees
    /// plaintext and the exit never sees the client IP.
    pub multihop_enabled: bool,

    /// Target padded packet size for DAITA mode (bytes).
    /// Packets smaller than this are padded; larger packets pass through.
    pub daita_pad_to: usize,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            daita_enabled: false,
            multihop_enabled: false,
            daita_pad_to: 1420,
        }
    }
}

/// Traffic counters and connection metadata.
pub struct ConnectionInfo {
    pub connected: bool,
    pub daita_enabled: bool,
    pub multihop_enabled: bool,
    pub uptime_secs: u64,
    pub bytes_rx: u64,
    pub bytes_tx: u64,
    pub server_ip: Option<String>,
    pub protocol: String,
}

/// A single WireGuard tunnel instance.
struct WgTunnel {
    tunnel: Tunn,
}

impl WgTunnel {
    fn new(
        private_key: &[u8; 32],
        peer_public_key: &[u8; 32],
        preshared_key: Option<[u8; 32]>,
    ) -> Self {
        let static_private = boringtun::x25519::StaticSecret::from(*private_key);
        let peer_static_public = boringtun::x25519::PublicKey::from(*peer_public_key);

        let tunnel = Tunn::new(
            static_private,
            peer_static_public,
            preshared_key,
            None, // persistent_keepalive
            0,    // index
            None, // rate_limiter
        );

        Self { tunnel }
    }

    fn encapsulate<'a>(&mut self, src: &[u8], dst: &'a mut [u8]) -> TunnResult<'a> {
        self.tunnel.encapsulate(src, dst)
    }

    fn decapsulate<'a>(
        &mut self,
        src_addr: Option<IpAddr>,
        datagram: &[u8],
        dst: &'a mut [u8],
    ) -> TunnResult<'a> {
        self.tunnel.decapsulate(src_addr, datagram, dst)
    }
}

pub struct VpnTunnel {
    /// Primary tunnel (entry relay in multihop, or the only tunnel in standard mode).
    entry: WgTunnel,
    /// Exit tunnel, present only in multihop mode.
    exit: Option<WgTunnel>,
    config: TunnelConfig,
    connected_at: Instant,
    bytes_rx: AtomicU64,
    bytes_tx: AtomicU64,
    daita_active: AtomicBool,
    server_ip: Option<String>,
}

impl VpnTunnel {
    /// Create a standard single-hop tunnel.
    pub fn new(
        private_key: &[u8; 32],
        peer_public_key: &[u8; 32],
        preshared_key: Option<[u8; 32]>,
    ) -> Self {
        Self {
            entry: WgTunnel::new(private_key, peer_public_key, preshared_key),
            exit: None,
            config: TunnelConfig::default(),
            connected_at: Instant::now(),
            bytes_rx: AtomicU64::new(0),
            bytes_tx: AtomicU64::new(0),
            daita_active: AtomicBool::new(false),
            server_ip: None,
        }
    }

    /// Create a tunnel with custom configuration (DAITA / multihop).
    pub fn with_config(
        private_key: &[u8; 32],
        peer_public_key: &[u8; 32],
        preshared_key: Option<[u8; 32]>,
        config: TunnelConfig,
    ) -> Self {
        Self {
            entry: WgTunnel::new(private_key, peer_public_key, preshared_key),
            exit: None,
            config: config.clone(),
            connected_at: Instant::now(),
            bytes_rx: AtomicU64::new(0),
            bytes_tx: AtomicU64::new(0),
            daita_active: AtomicBool::new(config.daita_enabled),
            server_ip: None,
        }
    }

    /// Create a multihop tunnel with separate entry and exit relays.
    /// Traffic is double-encapsulated: inner layer for exit, outer layer for entry.
    pub fn new_multihop(
        entry_private_key: &[u8; 32],
        entry_peer_public_key: &[u8; 32],
        entry_preshared_key: Option<[u8; 32]>,
        exit_private_key: &[u8; 32],
        exit_peer_public_key: &[u8; 32],
        exit_preshared_key: Option<[u8; 32]>,
        config: TunnelConfig,
    ) -> Self {
        Self {
            entry: WgTunnel::new(
                entry_private_key,
                entry_peer_public_key,
                entry_preshared_key,
            ),
            exit: Some(WgTunnel::new(
                exit_private_key,
                exit_peer_public_key,
                exit_preshared_key,
            )),
            config: config.clone(),
            connected_at: Instant::now(),
            bytes_rx: AtomicU64::new(0),
            bytes_tx: AtomicU64::new(0),
            daita_active: AtomicBool::new(config.daita_enabled),
            server_ip: None,
        }
    }

    /// Set the server IP for connection info reporting.
    pub fn set_server_ip(&mut self, ip: String) {
        self.server_ip = Some(ip);
    }

    /// Enable or disable DAITA at runtime.
    pub fn set_daita_enabled(&mut self, enabled: bool) {
        self.config.daita_enabled = enabled;
        self.daita_active.store(enabled, Ordering::Relaxed);
    }

    /// Return current connection metadata and traffic counters.
    pub fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            connected: true,
            daita_enabled: self.daita_active.load(Ordering::Relaxed),
            multihop_enabled: self.exit.is_some(),
            uptime_secs: self.connected_at.elapsed().as_secs(),
            bytes_rx: self.bytes_rx.load(Ordering::Relaxed),
            bytes_tx: self.bytes_tx.load(Ordering::Relaxed),
            server_ip: self.server_ip.clone(),
            protocol: "WireGuard".to_string(),
        }
    }

    /// Encapsulate an outbound packet.
    /// In multihop mode, the packet is first encapsulated for the exit relay,
    /// then the result is encapsulated again for the entry relay.
    /// When DAITA is enabled, the source packet is padded before encapsulation.
    pub fn encapsulate<'a>(&mut self, src: &[u8], dst: &'a mut [u8]) -> TunnResult<'a> {
        self.bytes_tx.fetch_add(src.len() as u64, Ordering::Relaxed);

        // When DAITA is enabled, pad the packet to a uniform size.
        // Real DAITA (with GotaTun) will replace this with proper
        // constant-size cells and chaff injection.
        let padded;
        let data = if self.config.daita_enabled {
            padded = Self::pad_packet(src, self.config.daita_pad_to);
            &padded
        } else {
            src
        };

        if let Some(ref mut exit_tunnel) = self.exit {
            // Multihop: first encapsulate for exit relay into a temp buffer,
            // then encapsulate that for the entry relay.
            let mut intermediate = vec![0u8; data.len() + 256];
            let inner_bytes = match exit_tunnel.encapsulate(data, &mut intermediate) {
                TunnResult::WriteToNetwork(inner) => inner.to_vec(),
                _ => return TunnResult::Done,
            };
            // Now wrap the exit-encapsulated packet for the entry relay
            self.entry.encapsulate(&inner_bytes, dst)
        } else {
            self.entry.encapsulate(data, dst)
        }
    }

    /// Decapsulate an inbound packet.
    /// In multihop mode, the packet is decapsulated by the entry relay first,
    /// then the inner payload is decapsulated by the exit relay.
    pub fn decapsulate<'a>(
        &mut self,
        src_addr: Option<IpAddr>,
        datagram: &[u8],
        dst: &'a mut [u8],
    ) -> TunnResult<'a> {
        self.bytes_rx
            .fetch_add(datagram.len() as u64, Ordering::Relaxed);

        if let Some(ref mut exit_tunnel) = self.exit {
            // Multihop: strip entry layer first
            let mut intermediate = vec![0u8; datagram.len() + 256];
            let inner_bytes = match self
                .entry
                .decapsulate(src_addr, datagram, &mut intermediate)
            {
                TunnResult::WriteToTunnelV4(inner, _) => inner.to_vec(),
                TunnResult::WriteToTunnelV6(inner, _) => inner.to_vec(),
                _ => return TunnResult::Done,
            };
            // Now strip the exit layer
            exit_tunnel.decapsulate(None, &inner_bytes, dst)
        } else {
            self.entry.decapsulate(src_addr, datagram, dst)
        }
    }

    /// Pad a packet to the target size with zeroes.
    /// Returns the original data if it already meets or exceeds the target.
    fn pad_packet(src: &[u8], target_size: usize) -> Vec<u8> {
        if src.len() >= target_size {
            return src.to_vec();
        }
        let mut padded = vec![0u8; target_size];
        padded[..src.len()].copy_from_slice(src);
        padded
    }
}
