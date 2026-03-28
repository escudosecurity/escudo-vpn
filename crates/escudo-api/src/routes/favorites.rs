use axum::extract::{Path, State};
use axum::Json;
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct FavoriteServer {
    pub id: Uuid,
    pub name: String,
    pub location: String,
    pub country_code: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub city: Option<String>,
    pub country_name: Option<String>,
    pub is_virtual: bool,
}

#[derive(Deserialize)]
pub struct AddFavoriteRequest {
    pub server_id: Uuid,
}

pub async fn list_favorites(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<FavoriteServer>>> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
            Option<String>,
            bool,
        ),
    >(
        r#"
        SELECT s.id, s.name, s.location, s.country_code,
               s.latitude, s.longitude, s.city, s.country_name,
               COALESCE(s.is_virtual, false) as is_virtual
        FROM favorites f
        JOIN servers s ON s.id = f.server_id
        WHERE f.user_id = $1
        ORDER BY f.created_at DESC
        "#,
    )
    .bind(auth.0.sub)
    .fetch_all(&state.db)
    .await?;

    let favorites = rows
        .into_iter()
        .map(
            |(
                id,
                name,
                location,
                country_code,
                latitude,
                longitude,
                city,
                country_name,
                is_virtual,
            )| {
                FavoriteServer {
                    id,
                    name,
                    location,
                    country_code,
                    latitude,
                    longitude,
                    city,
                    country_name,
                    is_virtual,
                }
            },
        )
        .collect();

    Ok(Json(favorites))
}

pub async fn add_favorite(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<AddFavoriteRequest>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    // Verify server exists
    let server_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM servers WHERE id = $1 AND is_active = true)",
    )
    .bind(body.server_id)
    .fetch_one(&state.db)
    .await?;

    if !server_exists {
        return Err(EscudoError::NotFound("Server not found".into()));
    }

    // Check for duplicate
    let already_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM favorites WHERE user_id = $1 AND server_id = $2)",
    )
    .bind(auth.0.sub)
    .bind(body.server_id)
    .fetch_one(&state.db)
    .await?;

    if already_exists {
        return Err(EscudoError::Conflict("Server is already a favorite".into()));
    }

    sqlx::query("INSERT INTO favorites (user_id, server_id) VALUES ($1, $2)")
        .bind(auth.0.sub)
        .bind(body.server_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "status": "added" })))
}

pub async fn remove_favorite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM favorites WHERE user_id = $1 AND server_id = $2")
        .bind(auth.0.sub)
        .bind(server_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(EscudoError::NotFound("Favorite not found".into()));
    }

    Ok(Json(serde_json::json!({ "status": "removed" })))
}
