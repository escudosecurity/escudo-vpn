use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::AdminUser;
use crate::state::AdminState;

#[derive(Serialize)]
pub struct ServerInfo {
    pub id: Uuid,
    pub name: String,
    pub location: String,
    pub public_ip: String,
    pub endpoint_port: i32,
    pub capacity_max: i32,
    pub assigned_user_cap: i32,
    pub active_session_soft_cap: i32,
    pub active_session_hard_cap: i32,
    pub routing_weight: f64,
    pub health_score: i32,
    pub lifecycle_state: String,
    pub is_active: bool,
}

pub async fn list_servers(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<ServerInfo>>> {
    let servers = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            i32,
            i32,
            i32,
            i32,
            i32,
            f64,
            i32,
            String,
            bool,
        ),
    >(
        "SELECT id, name, location, public_ip, endpoint_port, capacity_max,
                assigned_user_cap, active_session_soft_cap, active_session_hard_cap,
                routing_weight, health_score, lifecycle_state, is_active
         FROM servers",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(
        |(
            id,
            name,
            location,
            public_ip,
            endpoint_port,
            capacity_max,
            assigned_user_cap,
            active_session_soft_cap,
            active_session_hard_cap,
            routing_weight,
            health_score,
            lifecycle_state,
            is_active,
        )| {
            ServerInfo {
                id,
                name,
                location,
                public_ip,
                endpoint_port,
                capacity_max,
                assigned_user_cap,
                active_session_soft_cap,
                active_session_hard_cap,
                routing_weight,
                health_score,
                lifecycle_state,
                is_active,
            }
        },
    )
    .collect();

    Ok(Json(servers))
}

#[derive(Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub location: String,
    pub public_ip: String,
    pub public_key: String,
    pub endpoint_port: i32,
    pub capacity_max: i32,
}

pub async fn create_server(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Json(req): Json<CreateServerRequest>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO servers (name, location, public_ip, public_key, endpoint_port, capacity_max)
           VALUES ($1, $2, $3, $4, $5, $6) RETURNING id"#,
    )
    .bind(&req.name)
    .bind(&req.location)
    .bind(&req.public_ip)
    .bind(&req.public_key)
    .bind(req.endpoint_port)
    .bind(req.capacity_max)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct UpdateServerRequest {
    pub capacity_max: Option<i32>,
    pub assigned_user_cap: Option<i32>,
    pub active_session_soft_cap: Option<i32>,
    pub active_session_hard_cap: Option<i32>,
    pub routing_weight: Option<f64>,
    pub health_score: Option<i32>,
    pub lifecycle_state: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn update_server(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(server_id): Path<Uuid>,
    Json(req): Json<UpdateServerRequest>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    if let Some(capacity) = req.capacity_max {
        sqlx::query("UPDATE servers SET capacity_max = $1, updated_at = NOW() WHERE id = $2")
            .bind(capacity)
            .bind(server_id)
            .execute(&state.db)
            .await?;
    }

    if let Some(capacity) = req.assigned_user_cap {
        sqlx::query("UPDATE servers SET assigned_user_cap = $1, updated_at = NOW() WHERE id = $2")
            .bind(capacity)
            .bind(server_id)
            .execute(&state.db)
            .await?;
    }

    if let Some(capacity) = req.active_session_soft_cap {
        sqlx::query(
            "UPDATE servers SET active_session_soft_cap = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(capacity)
        .bind(server_id)
        .execute(&state.db)
        .await?;
    }

    if let Some(capacity) = req.active_session_hard_cap {
        sqlx::query(
            "UPDATE servers SET active_session_hard_cap = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(capacity)
        .bind(server_id)
        .execute(&state.db)
        .await?;
    }

    if let Some(weight) = req.routing_weight {
        sqlx::query("UPDATE servers SET routing_weight = $1, updated_at = NOW() WHERE id = $2")
            .bind(weight)
            .bind(server_id)
            .execute(&state.db)
            .await?;
    }

    if let Some(score) = req.health_score {
        sqlx::query(
            "UPDATE servers SET health_score = $1, last_health_at = NOW(), updated_at = NOW() WHERE id = $2",
        )
        .bind(score)
        .bind(server_id)
        .execute(&state.db)
        .await?;
    }

    if let Some(state_value) = req.lifecycle_state.as_deref() {
        sqlx::query("UPDATE servers SET lifecycle_state = $1, updated_at = NOW() WHERE id = $2")
            .bind(state_value)
            .bind(server_id)
            .execute(&state.db)
            .await?;
    }

    if let Some(active) = req.is_active {
        sqlx::query("UPDATE servers SET is_active = $1, updated_at = NOW() WHERE id = $2")
            .bind(active)
            .bind(server_id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(serde_json::json!({ "message": "Server updated" })))
}
