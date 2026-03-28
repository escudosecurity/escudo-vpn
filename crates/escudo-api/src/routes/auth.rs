use std::net::SocketAddr;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use escudo_common::jwt::{encode_jwt, Role};
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::backend_control;
use crate::state::AppState;
use crate::telemetry::{normalize_country, resolve_request_telemetry};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
}

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(req): Json<RegisterRequest>,
) -> escudo_common::Result<Json<AuthResponse>> {
    let email = req.email.trim().to_lowercase();
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    backend_control::enforce_signup_allowed(&state.db, false).await?;

    // Validate email
    if email.len() > 254 {
        return Err(EscudoError::BadRequest("Email too long".into()));
    }
    if email.contains('<')
        || email.contains('>')
        || email.contains('"')
        || email.contains('\'')
        || email.contains('&')
    {
        return Err(EscudoError::BadRequest(
            "Email contains invalid characters".into(),
        ));
    }
    // Must match: something@something.something
    let email_parts: Vec<&str> = email.splitn(2, '@').collect();
    if email_parts.len() != 2
        || email_parts[0].is_empty()
        || email_parts[1].is_empty()
        || !email_parts[1].contains('.')
        || email_parts[1].starts_with('.')
        || email_parts[1].ends_with('.')
    {
        return Err(EscudoError::BadRequest("Invalid email".into()));
    }

    // Validate password
    if req.password.len() < 8 {
        return Err(EscudoError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }
    if req.password.len() > 128 {
        return Err(EscudoError::BadRequest("Password too long".into()));
    }

    // Check if email exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
        .bind(&email)
        .fetch_one(&state.db)
        .await?;

    if exists {
        return Err(EscudoError::Conflict("Email already registered".into()));
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| EscudoError::Internal(e.to_string()))?
        .to_string();

    // Insert user
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (
            email, password_hash, role, signup_ip, signup_country, latest_login_ip, latest_login_country
         ) VALUES ($1, $2, 'user', CAST($3 AS inet), $4, CAST($3 AS inet), $4) RETURNING id",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(telemetry.ip.map(|ip| ip.to_string()))
    .bind(normalize_country(
        telemetry
            .country_code
            .as_deref()
            .or(telemetry.country.as_deref()),
    ))
    .fetch_one(&state.db)
    .await?;

    let token = encode_jwt(
        user_id,
        &email,
        Role::User,
        &state.config.jwt.secret,
        state.config.jwt.expiration_hours,
    )?;

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(user_id),
        None,
        None,
        "register",
        "success",
        Some("Email/password registration completed".into()),
        serde_json::json!({
            "method": "password",
            "signup_country": normalize_country(
                telemetry.country_code.as_deref().or(telemetry.country.as_deref())
            ),
        }),
    )
    .await
    {
        tracing::warn!("Failed to record register journey event for user {user_id}: {error}");
    }

    Ok(Json(AuthResponse { token, user_id }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(req): Json<LoginRequest>,
) -> escudo_common::Result<Json<AuthResponse>> {
    let email = req.email.trim().to_lowercase();
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;

    let user = sqlx::query_as::<_, (Uuid, String, String, bool)>(
        "SELECT id, password_hash, role, is_active FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::Unauthorized("Invalid credentials".into()))?;

    let (user_id, password_hash, role, is_active) = user;

    if !is_active {
        return Err(EscudoError::Forbidden("Account is suspended".into()));
    }

    let parsed_hash =
        PasswordHash::new(&password_hash).map_err(|e| EscudoError::Internal(e.to_string()))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| EscudoError::Unauthorized("Invalid credentials".into()))?;

    let role = match role.as_str() {
        "admin" => Role::Admin,
        _ => Role::User,
    };

    let token = encode_jwt(
        user_id,
        &email,
        role,
        &state.config.jwt.secret,
        state.config.jwt.expiration_hours,
    )?;

    sqlx::query(
        "UPDATE users
         SET latest_login_ip = COALESCE(CAST($2 AS inet), latest_login_ip),
             latest_login_country = COALESCE($3, latest_login_country),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(telemetry.ip.map(|ip| ip.to_string()))
    .bind(normalize_country(
        telemetry
            .country_code
            .as_deref()
            .or(telemetry.country.as_deref()),
    ))
    .execute(&state.db)
    .await?;

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(user_id),
        None,
        None,
        "login",
        "success",
        Some("Email/password login completed".into()),
        serde_json::json!({
            "method": "password",
            "login_country": normalize_country(
                telemetry.country_code.as_deref().or(telemetry.country.as_deref())
            ),
        }),
    )
    .await
    {
        tracing::warn!("Failed to record login journey event for user {user_id}: {error}");
    }

    Ok(Json(AuthResponse { token, user_id }))
}
