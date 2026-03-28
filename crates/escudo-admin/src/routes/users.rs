use axum::extract::{Path, State};
use axum::Json;
use escudo_common::EscudoError;
use serde::Serialize;
use uuid::Uuid;

use crate::middleware::AdminUser;
use crate::state::AdminState;

#[derive(Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_users(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<UserSummary>>> {
    let users = sqlx::query_as::<_, (Uuid, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, email, role, is_active, created_at FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, email, role, is_active, created_at)| UserSummary {
        id,
        email,
        role,
        is_active,
        created_at,
    })
    .collect();

    Ok(Json(users))
}

pub async fn suspend_user(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let result =
        sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(EscudoError::NotFound("User not found".into()));
    }

    Ok(Json(serde_json::json!({ "message": "User suspended" })))
}

pub async fn delete_user(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(EscudoError::NotFound("User not found".into()));
    }

    Ok(Json(serde_json::json!({ "message": "User deleted" })))
}
