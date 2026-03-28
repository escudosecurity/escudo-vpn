use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::AdminUser;
use crate::state::AdminState;

#[derive(Serialize)]
pub struct TenantInfo {
    pub id: Uuid,
    pub name: String,
    pub max_users: i32,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_tenants(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<TenantInfo>>> {
    let tenants = sqlx::query_as::<_, (Uuid, String, i32, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, max_users, is_active, created_at FROM tenants ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, name, max_users, is_active, created_at)| TenantInfo {
        id,
        name,
        max_users,
        is_active,
        created_at,
    })
    .collect();

    Ok(Json(tenants))
}

#[derive(Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub max_users: i32,
}

pub async fn create_tenant(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Json(req): Json<CreateTenantRequest>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO tenants (name, max_users) VALUES ($1, $2) RETURNING id")
            .bind(&req.name)
            .bind(req.max_users)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(serde_json::json!({ "id": id })))
}
