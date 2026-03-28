use std::sync::Arc;
use std::{collections::HashMap, net::IpAddr};

use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::{debug, error};

/// Per-client counters accumulated in memory between flushes.
#[derive(Debug, Default)]
struct ClientCounts {
    queries: u64,
    blocked: u64,
}

/// Buffered stats collector that periodically flushes to the database.
///
/// Every DNS query calls `record()` which updates in-memory counters.
/// A background task calls `flush()` every 30 seconds.
#[derive(Clone)]
pub struct StatsRecorder {
    inner: Arc<Mutex<HashMap<IpAddr, ClientCounts>>>,
    db: PgPool,
}

impl StatsRecorder {
    pub fn new(db: PgPool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            db,
        }
    }

    /// Record a DNS query. If `blocked` is true, also increment the blocked
    /// counter for the source IP.
    pub async fn record(&self, client_ip: IpAddr, blocked: bool) {
        let mut counts = self.inner.lock().await;
        let entry = counts.entry(client_ip).or_default();
        entry.queries += 1;
        if blocked {
            entry.blocked += 1;
        }
    }

    /// Drain the in-memory buffer and upsert accumulated counts into `dns_stats`.
    pub async fn flush(&self) {
        let snapshot = {
            let mut counts = self.inner.lock().await;
            std::mem::take(&mut *counts)
        };

        if snapshot.is_empty() {
            return;
        }

        debug!("Flushing DNS stats for {} clients", snapshot.len());

        let mut tx = match self.db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to begin DNS stats flush transaction: {e}");
                return;
            }
        };

        for (client_ip, counts) in snapshot {
            if let Err(e) = sqlx::query(
                r#"
                INSERT INTO dns_stats (client_ip, date, queries_total, blocked_total, updated_at)
                VALUES ($1, CURRENT_DATE, $2, $3, NOW())
                ON CONFLICT (client_ip, date) DO UPDATE SET
                    queries_total = dns_stats.queries_total + EXCLUDED.queries_total,
                    blocked_total = dns_stats.blocked_total + EXCLUDED.blocked_total,
                    updated_at = NOW()
                "#,
            )
            .bind(client_ip.to_string())
            .bind(counts.queries as i64)
            .bind(counts.blocked as i64)
            .execute(&mut *tx)
            .await
            {
                error!("Failed to flush DNS stats for {client_ip}: {e}");
                return;
            }
        }

        if let Err(e) = tx.commit().await {
            error!("Failed to commit DNS stats flush: {e}");
        }
    }

    /// Spawn the periodic flush loop. Should be called once at startup.
    pub fn spawn_flush_loop(self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            let mut purge_ticks: u64 = 0;
            loop {
                interval.tick().await;
                self.flush().await;
                purge_ticks += 1;

                // Purge once per hour instead of every flush.
                if purge_ticks >= 120 {
                    purge_ticks = 0;
                    if let Err(e) = sqlx::query(
                        "DELETE FROM dns_stats WHERE date < CURRENT_DATE - INTERVAL '30 days'",
                    )
                    .execute(&self.db)
                    .await
                    {
                        error!("Failed to purge old dns_stats: {e}");
                    }
                }
            }
        });
    }
}
