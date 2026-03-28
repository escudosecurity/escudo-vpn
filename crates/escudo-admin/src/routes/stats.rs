use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::middleware::AdminUser;
use crate::state::AdminState;

#[derive(Serialize)]
pub struct AggregateStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_devices: i64,
    pub active_devices: i64,
    pub total_rx_bytes: i64,
    pub total_tx_bytes: i64,
}

pub async fn get_stats(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<AggregateStats>> {
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    let active_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db)
        .await?;

    let total_devices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices")
        .fetch_one(&state.db)
        .await?;

    let active_devices: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM devices WHERE is_active = true")
            .fetch_one(&state.db)
            .await?;

    let total_rx: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(rx_bytes), 0)::BIGINT FROM usage_logs")
            .fetch_one(&state.db)
            .await?;

    let total_tx: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(tx_bytes), 0)::BIGINT FROM usage_logs")
            .fetch_one(&state.db)
            .await?;

    Ok(Json(AggregateStats {
        total_users,
        active_users,
        total_devices,
        active_devices,
        total_rx_bytes: total_rx,
        total_tx_bytes: total_tx,
    }))
}
