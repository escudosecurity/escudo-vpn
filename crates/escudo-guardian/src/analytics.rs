use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;
use tracing::info;

/// Burn-rate analytics for residential proxy IPs.
pub struct BurnAnalytics {
    db: PgPool,
}

impl BurnAnalytics {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Query ip_health_logs for the last 24 hours and log the burn rate.
    ///
    /// Burn rate = (blocked checks / total checks) * 100%.
    pub async fn analyze(&self) -> Result<()> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) AS total,
                COUNT(*) FILTER (WHERE status = 'blocked') AS blocked
            FROM ip_health_logs
            WHERE checked_at >= now() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.db)
        .await?;

        let total: i64 = row.get("total");
        let blocked: i64 = row.get("blocked");

        if total == 0 {
            info!("burn analytics: no health checks recorded in the last 24h");
            return Ok(());
        }

        let burn_rate = (blocked as f64 / total as f64) * 100.0;

        info!(
            total_checks = total,
            blocked_checks = blocked,
            burn_rate_pct = format!("{burn_rate:.1}"),
            "IP burn rate (last 24h)"
        );

        Ok(())
    }
}
