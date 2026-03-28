use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ConnectionProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub server_id: Option<Uuid>,
    pub protocol: String,
    pub kill_switch: bool,
    pub dns_over_https: bool,
    pub is_preset: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

type ProfileRow = (
    Uuid,
    Uuid,
    String,
    Option<Uuid>,
    String,
    bool,
    bool,
    bool,
    chrono::DateTime<chrono::Utc>,
);

fn row_to_profile(row: ProfileRow) -> ConnectionProfile {
    ConnectionProfile {
        id: row.0,
        user_id: row.1,
        name: row.2,
        server_id: row.3,
        protocol: row.4,
        kill_switch: row.5,
        dns_over_https: row.6,
        is_preset: row.7,
        created_at: row.8,
    }
}

#[derive(Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    pub server_id: Option<Uuid>,
    pub protocol: Option<String>,
    pub kill_switch: Option<bool>,
    pub dns_over_https: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub server_id: Option<Uuid>,
    pub protocol: Option<String>,
    pub kill_switch: Option<bool>,
    pub dns_over_https: Option<bool>,
}

const PROFILE_COLUMNS: &str =
    "id, user_id, name, server_id, protocol, kill_switch, dns_over_https, is_preset, created_at";

pub async fn list_profiles(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<ConnectionProfile>>> {
    let rows = sqlx::query_as::<_, ProfileRow>(
        &format!("SELECT {PROFILE_COLUMNS} FROM connection_profiles WHERE user_id = $1 ORDER BY created_at ASC"),
    )
    .bind(auth.0.sub)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(row_to_profile).collect()))
}

pub async fn create_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateProfileRequest>,
) -> escudo_common::Result<Json<ConnectionProfile>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(EscudoError::BadRequest(
            "Profile name cannot be empty".into(),
        ));
    }
    if name.len() > 64 {
        return Err(EscudoError::BadRequest("Profile name is too long".into()));
    }

    let row = sqlx::query_as::<_, ProfileRow>(
        &format!(
            r#"INSERT INTO connection_profiles (user_id, name, server_id, protocol, kill_switch, dns_over_https, is_preset)
               VALUES ($1, $2, $3, $4, $5, $6, false)
               RETURNING {PROFILE_COLUMNS}"#
        ),
    )
    .bind(auth.0.sub)
    .bind(&name)
    .bind(body.server_id)
    .bind(body.protocol.unwrap_or_else(|| "wireguard".to_string()))
    .bind(body.kill_switch.unwrap_or(false))
    .bind(body.dns_over_https.unwrap_or(false))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row_to_profile(row)))
}

pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProfileRequest>,
) -> escudo_common::Result<Json<ConnectionProfile>> {
    // Verify ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM connection_profiles WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(auth.0.sub)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(EscudoError::NotFound("Profile not found".into()));
    }

    let row = sqlx::query_as::<_, ProfileRow>(&format!(
        r#"UPDATE connection_profiles SET
                name = COALESCE($3, name),
                server_id = COALESCE($4, server_id),
                protocol = COALESCE($5, protocol),
                kill_switch = COALESCE($6, kill_switch),
                dns_over_https = COALESCE($7, dns_over_https)
            WHERE id = $1 AND user_id = $2
            RETURNING {PROFILE_COLUMNS}"#
    ))
    .bind(id)
    .bind(auth.0.sub)
    .bind(body.name)
    .bind(body.server_id)
    .bind(body.protocol)
    .bind(body.kill_switch)
    .bind(body.dns_over_https)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row_to_profile(row)))
}

pub async fn delete_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> escudo_common::Result<StatusCode> {
    // Check ownership and preset status
    let profile = sqlx::query_as::<_, (bool,)>(
        "SELECT is_preset FROM connection_profiles WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?;

    match profile {
        None => return Err(EscudoError::NotFound("Profile not found".into())),
        Some((true,)) => {
            return Err(EscudoError::BadRequest(
                "Cannot delete a preset profile".into(),
            ));
        }
        _ => {}
    }

    sqlx::query("DELETE FROM connection_profiles WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.0.sub)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
