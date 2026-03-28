use std::sync::Arc;
use std::time::Duration;

use prometheus::{IntCounter, IntGauge, Registry};
use tracing::{debug, error};

use crate::wg::MultiWgManager;

#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub active_peers: IntGauge,
    pub total_rx_bytes: IntCounter,
    pub total_tx_bytes: IntCounter,
    pub connections_total: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let active_peers = IntGauge::new("escudo_active_peers", "Number of active WireGuard peers")
            .expect("metric creation");
        let total_rx_bytes = IntCounter::new("escudo_rx_bytes_total", "Total received bytes")
            .expect("metric creation");
        let total_tx_bytes = IntCounter::new("escudo_tx_bytes_total", "Total transmitted bytes")
            .expect("metric creation");
        let connections_total =
            IntCounter::new("escudo_connections_total", "Total connections since start")
                .expect("metric creation");

        registry.register(Box::new(active_peers.clone())).unwrap();
        registry.register(Box::new(total_rx_bytes.clone())).unwrap();
        registry.register(Box::new(total_tx_bytes.clone())).unwrap();
        registry
            .register(Box::new(connections_total.clone()))
            .unwrap();

        Self {
            registry,
            active_peers,
            total_rx_bytes,
            total_tx_bytes,
            connections_total,
        }
    }
}

pub async fn stats_collector(wg: Arc<MultiWgManager>, metrics: Metrics, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut prev_rx: i64 = 0;
    let mut prev_tx: i64 = 0;

    loop {
        interval.tick().await;

        match wg.get_aggregate_stats().await {
            Ok((peers, rx, tx)) => {
                metrics.active_peers.set(peers as i64);

                // Increment counters by delta
                let delta_rx = rx.saturating_sub(prev_rx);
                let delta_tx = tx.saturating_sub(prev_tx);
                if delta_rx > 0 {
                    metrics.total_rx_bytes.inc_by(delta_rx as u64);
                }
                if delta_tx > 0 {
                    metrics.total_tx_bytes.inc_by(delta_tx as u64);
                }
                prev_rx = rx;
                prev_tx = tx;

                debug!(
                    peers = peers,
                    rx_bytes = rx,
                    tx_bytes = tx,
                    "WireGuard stats"
                );
            }
            Err(e) => {
                error!("Failed to collect stats: {e}");
            }
        }
    }
}
