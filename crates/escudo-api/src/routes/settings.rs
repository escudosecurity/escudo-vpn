use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub kill_switch: bool,
    pub auto_connect: bool,
    pub auto_connect_wifi_only: bool,
    pub protocol: String,
    pub lan_discovery: bool,
    pub split_tunnel_apps: Vec<String>,
    pub preferred_server_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub kill_switch: Option<bool>,
    pub auto_connect: Option<bool>,
    pub auto_connect_wifi_only: Option<bool>,
    pub protocol: Option<String>,
    pub lan_discovery: Option<bool>,
    pub split_tunnel_apps: Option<Vec<String>>,
    pub preferred_server_id: Option<Uuid>,
}

type SettingsRow = (
    Uuid,
    bool,
    bool,
    bool,
    String,
    bool,
    Vec<String>,
    Option<Uuid>,
);

fn row_to_settings(row: SettingsRow) -> UserSettings {
    UserSettings {
        user_id: row.0,
        kill_switch: row.1,
        auto_connect: row.2,
        auto_connect_wifi_only: row.3,
        protocol: row.4,
        lan_discovery: row.5,
        split_tunnel_apps: row.6,
        preferred_server_id: row.7,
    }
}

pub async fn get_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<UserSettings>> {
    let row = sqlx::query_as::<_, SettingsRow>(
        r#"SELECT user_id, kill_switch, auto_connect, auto_connect_wifi_only,
                  protocol, lan_discovery, split_tunnel_apps, preferred_server_id
           FROM user_settings WHERE user_id = $1"#,
    )
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?;

    if let Some(row) = row {
        return Ok(Json(row_to_settings(row)));
    }

    let row = sqlx::query_as::<_, SettingsRow>(
        r#"INSERT INTO user_settings (user_id)
           VALUES ($1)
           RETURNING user_id, kill_switch, auto_connect, auto_connect_wifi_only,
                     protocol, lan_discovery, split_tunnel_apps, preferred_server_id"#,
    )
    .bind(auth.0.sub)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row_to_settings(row)))
}

pub async fn update_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UpdateSettingsRequest>,
) -> escudo_common::Result<Json<UserSettings>> {
    sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(auth.0.sub)
        .execute(&state.db)
        .await?;

    let row = sqlx::query_as::<_, SettingsRow>(
        r#"UPDATE user_settings SET
            kill_switch = COALESCE($2, kill_switch),
            auto_connect = COALESCE($3, auto_connect),
            auto_connect_wifi_only = COALESCE($4, auto_connect_wifi_only),
            protocol = COALESCE($5, protocol),
            lan_discovery = COALESCE($6, lan_discovery),
            split_tunnel_apps = COALESCE($7, split_tunnel_apps),
            preferred_server_id = COALESCE($8, preferred_server_id),
            updated_at = now()
        WHERE user_id = $1
        RETURNING user_id, kill_switch, auto_connect, auto_connect_wifi_only,
                  protocol, lan_discovery, split_tunnel_apps, preferred_server_id"#,
    )
    .bind(auth.0.sub)
    .bind(body.kill_switch)
    .bind(body.auto_connect)
    .bind(body.auto_connect_wifi_only)
    .bind(body.protocol)
    .bind(body.lan_discovery)
    .bind(body.split_tunnel_apps)
    .bind(body.preferred_server_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row_to_settings(row)))
}
