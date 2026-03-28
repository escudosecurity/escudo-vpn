use axum::extract::State;
use axum::Json;
use serde::Serialize;
use tracing::error;
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct AccountInfo {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub device_count: i64,
}

pub async fn get_account(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<AccountInfo>> {
    let user = sqlx::query_as::<_, (Uuid, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, email, role, is_active, created_at FROM users WHERE id = $1",
    )
    .bind(auth.0.sub)
    .fetch_one(&state.db)
    .await?;

    let (id, email, role, is_active, created_at) = user;

    let device_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM devices WHERE user_id = $1 AND is_active = true")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(AccountInfo {
        id,
        email,
        role,
        is_active,
        created_at,
        device_count,
    }))
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub message: String,
}

pub async fn delete_account(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<DeleteResponse>> {
    let user_id = auth.0.sub;
    let mut tx = state.db.begin().await.map_err(|e| {
        error!("Failed to begin account deletion transaction for user {user_id}: {e}");
        escudo_common::EscudoError::Internal("Failed to delete account".into())
    })?;

    sqlx::query(
        "DELETE FROM usage_logs WHERE device_id IN (SELECT id FROM devices WHERE user_id = $1)",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to delete usage_logs for user {user_id}: {e}");
        escudo_common::EscudoError::Internal("Failed to delete account".into())
    })?;

    sqlx::query("DELETE FROM devices WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Failed to delete devices for user {user_id}: {e}");
            escudo_common::EscudoError::Internal("Failed to delete account".into())
        })?;

    sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Failed to delete subscriptions for user {user_id}: {e}");
            escudo_common::EscudoError::Internal("Failed to delete account".into())
        })?;

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Failed to delete user {user_id}: {e}");
            escudo_common::EscudoError::Internal("Failed to delete account".into())
        })?;

    tx.commit().await.map_err(|e| {
        error!("Failed to commit account deletion for user {user_id}: {e}");
        escudo_common::EscudoError::Internal("Failed to delete account".into())
    })?;

    Ok(Json(DeleteResponse {
        message: "Conta excluída com sucesso".to_string(),
    }))
}
