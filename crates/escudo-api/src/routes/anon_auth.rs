use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{Duration, Utc};
use escudo_common::jwt::{encode_jwt, Role};
use escudo_common::EscudoError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::backend_control;
use crate::middleware::AuthUser;
use crate::routes::auth::AuthResponse;
use crate::state::AppState;
use crate::telemetry::{normalize_country, resolve_request_telemetry};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AccountInfo {
    pub account_number: String,
    pub tier: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct LoginNumberRequest {
    pub account_number: String,
}

#[derive(Deserialize)]
pub struct AddEmailRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct QrTokenResponse {
    pub qr_token: Uuid,
    pub qr_url: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ScanQrRequest {
    pub qr_token: Uuid,
}

#[derive(Deserialize)]
pub struct DeviceRegistration {
    pub android_id: String,
    pub advertising_id: Option<String>,
    pub device_model: Option<String>,
    pub os_version: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a 16-digit account number formatted as XXXX-XXXX-XXXX-XXXX.
fn generate_account_number() -> String {
    let mut rng = rand::thread_rng();
    let digits: Vec<u8> = (0..16).map(|_| rng.gen_range(0..10)).collect();
    format!(
        "{}{}{}{}-{}{}{}{}-{}{}{}{}-{}{}{}{}",
        digits[0],
        digits[1],
        digits[2],
        digits[3],
        digits[4],
        digits[5],
        digits[6],
        digits[7],
        digits[8],
        digits[9],
        digits[10],
        digits[11],
        digits[12],
        digits[13],
        digits[14],
        digits[15],
    )
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/anonymous
// ---------------------------------------------------------------------------

pub async fn create_anonymous_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> escudo_common::Result<Json<AccountInfo>> {
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    backend_control::enforce_signup_allowed(&state.db, true).await?;

    // Retry up to 5 times in case of account number collision
    let mut attempts = 0;
    let (account_number, created_at) = loop {
        let number = generate_account_number();
        let result = sqlx::query_as::<_, (String, chrono::DateTime<Utc>)>(
            "INSERT INTO accounts (account_number) VALUES ($1) \
             ON CONFLICT DO NOTHING \
             RETURNING account_number, created_at",
        )
        .bind(&number)
        .fetch_optional(&state.db)
        .await?;

        if let Some(row) = result {
            break row;
        }

        attempts += 1;
        if attempts >= 5 {
            return Err(EscudoError::Internal(
                "Failed to generate unique account number".into(),
            ));
        }
    };

    // Also create a row in the users table so that JWT auth works
    // (the AuthUser middleware checks users.id).
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (
            email, password_hash, role, signup_ip, signup_country, latest_login_ip, latest_login_country
         ) VALUES ($1, '', 'user', CAST($2 AS inet), $3, CAST($2 AS inet), $3) RETURNING id",
    )
    .bind(&format!("anon+{}@escudovpn.com", account_number))
    .bind(telemetry.ip.map(|ip| ip.to_string()))
    .bind(normalize_country(
        telemetry
            .country_code
            .as_deref()
            .or(telemetry.country.as_deref()),
    ))
    .fetch_one(&state.db)
    .await?;

    // Link the account to the user
    sqlx::query("UPDATE accounts SET email = $1 WHERE account_number = $2")
        .bind(&format!("user:{}", user_id))
        .bind(&account_number)
        .execute(&state.db)
        .await?;

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(user_id),
        None,
        None,
        "anonymous_register",
        "success",
        Some("Anonymous account created".into()),
        serde_json::json!({
            "account_number": account_number,
            "signup_country": normalize_country(
                telemetry.country_code.as_deref().or(telemetry.country.as_deref())
            ),
        }),
    )
    .await
    {
        tracing::warn!(
            "Failed to record anonymous register journey event for user {user_id}: {error}"
        );
    }

    Ok(Json(AccountInfo {
        account_number,
        tier: "free".to_string(),
        created_at,
    }))
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/login-number
// ---------------------------------------------------------------------------

pub async fn login_with_number(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<LoginNumberRequest>,
) -> escudo_common::Result<Json<AuthResponse>> {
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    let account_number = body.account_number.trim().to_string();

    // Validate format: XXXX-XXXX-XXXX-XXXX (digits and dashes)
    if account_number.len() != 19
        || account_number.chars().enumerate().any(|(i, c)| {
            if i == 4 || i == 9 || i == 14 {
                c != '-'
            } else {
                !c.is_ascii_digit()
            }
        })
    {
        return Err(EscudoError::BadRequest(
            "Invalid account number format. Expected XXXX-XXXX-XXXX-XXXX".into(),
        ));
    }

    // Look up the account
    let account = sqlx::query_as::<_, (String, String)>(
        "SELECT account_number, COALESCE(email, '') FROM accounts \
         WHERE account_number = $1 AND status = 'active'",
    )
    .bind(&account_number)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::Unauthorized("Account not found".into()))?;

    let (_acct_num, email_or_ref) = account;

    // Resolve the user_id.  The email column stores "user:<uuid>" for anonymous accounts.
    let user_id: Uuid = if let Some(uid_str) = email_or_ref.strip_prefix("user:") {
        uid_str
            .parse()
            .map_err(|_| EscudoError::Internal("Corrupt account link".into()))?
    } else {
        // Fallback: look up by the synthetic email
        let synthetic = format!("anon+{}@escudovpn.com", account_number);
        sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&synthetic)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| EscudoError::Internal("Account user record missing".into()))?
    };

    let token = encode_jwt(
        user_id,
        &format!("anon+{}@escudovpn.com", account_number),
        Role::User,
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
        "login_number",
        "success",
        Some("Anonymous number login completed".into()),
        serde_json::json!({
            "account_number": account_number,
            "login_country": normalize_country(
                telemetry.country_code.as_deref().or(telemetry.country.as_deref())
            ),
        }),
    )
    .await
    {
        tracing::warn!("Failed to record login-number journey event for user {user_id}: {error}");
    }

    Ok(Json(AuthResponse { token, user_id }))
}

// ---------------------------------------------------------------------------
// PUT /api/v1/account/email
// ---------------------------------------------------------------------------

pub async fn add_email(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<AddEmailRequest>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let email = body.email.trim().to_lowercase();

    // Basic email validation (same approach as auth.rs)
    if email.len() > 254 {
        return Err(EscudoError::BadRequest("Email too long".into()));
    }
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2
        || parts[0].is_empty()
        || parts[1].is_empty()
        || !parts[1].contains('.')
        || parts[1].starts_with('.')
        || parts[1].ends_with('.')
    {
        return Err(EscudoError::BadRequest("Invalid email".into()));
    }

    // Find the account linked to this user
    let current_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(auth.0.sub)
        .fetch_one(&state.db)
        .await?;

    // Derive account number from synthetic email
    let account_number = current_email
        .strip_prefix("anon+")
        .and_then(|s| s.strip_suffix("@escudovpn.com"))
        .ok_or_else(|| {
            EscudoError::BadRequest("This account does not support email linking".into())
        })?
        .to_string();

    // Check the email is not already taken by another user
    let taken: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND id != $2)")
            .bind(&email)
            .bind(auth.0.sub)
            .fetch_one(&state.db)
            .await?;

    if taken {
        return Err(EscudoError::Conflict("Email already in use".into()));
    }

    // Update accounts table
    sqlx::query("UPDATE accounts SET email = $1 WHERE account_number = $2")
        .bind(&email)
        .bind(&account_number)
        .execute(&state.db)
        .await?;

    // Update users table email as well
    sqlx::query("UPDATE users SET email = $1 WHERE id = $2")
        .bind(&email)
        .bind(auth.0.sub)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "message": "Email added successfully",
        "email": email,
    })))
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/qr/generate
// ---------------------------------------------------------------------------

pub async fn generate_qr_token(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<QrTokenResponse>> {
    let expires_at = Utc::now() + Duration::minutes(5);

    // Derive account number from synthetic email (may be null for non-anon users)
    let user_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(auth.0.sub)
        .fetch_one(&state.db)
        .await?;

    let account_number: Option<String> = user_email
        .strip_prefix("anon+")
        .and_then(|s| s.strip_suffix("@escudovpn.com"))
        .map(|s| s.to_string());

    let token: Uuid = sqlx::query_scalar(
        "INSERT INTO qr_tokens (user_id, account_number, expires_at) \
         VALUES ($1, $2, $3) RETURNING token",
    )
    .bind(auth.0.sub)
    .bind(&account_number)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    let qr_url = format!("escudo://pair?token={}", token);

    Ok(Json(QrTokenResponse {
        qr_token: token,
        qr_url,
        expires_at,
    }))
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/qr/scan
// ---------------------------------------------------------------------------

pub async fn scan_qr_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<ScanQrRequest>,
) -> escudo_common::Result<Json<AuthResponse>> {
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    // Fetch the QR token if it exists, is unused, and not expired
    let row = sqlx::query_as::<_, (Uuid, Uuid, Option<String>)>(
        "SELECT token, user_id, account_number FROM qr_tokens \
         WHERE token = $1 AND used = false AND expires_at > NOW()",
    )
    .bind(body.qr_token)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        EscudoError::Unauthorized("QR token is invalid, expired, or already used".into())
    })?;

    let (_token, user_id, _account_number) = row;

    // Mark token as used (one-time)
    sqlx::query("UPDATE qr_tokens SET used = true WHERE token = $1")
        .bind(body.qr_token)
        .execute(&state.db)
        .await?;

    // Look up the user's email for the JWT
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    let token = encode_jwt(
        user_id,
        &email,
        Role::User,
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

    Ok(Json(AuthResponse { token, user_id }))
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/register-device
// ---------------------------------------------------------------------------

pub async fn register_device(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<DeviceRegistration>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    if body.android_id.is_empty() || body.android_id.len() > 64 {
        return Err(EscudoError::BadRequest(
            "android_id is required and must be at most 64 characters".into(),
        ));
    }

    // Resolve account number for this user
    let user_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(auth.0.sub)
        .fetch_one(&state.db)
        .await?;

    let account_number = user_email
        .strip_prefix("anon+")
        .and_then(|s| s.strip_suffix("@escudovpn.com"))
        .map(|s| s.to_string());

    let account_number = match account_number {
        Some(n) => n,
        None => {
            // Non-anonymous user -- look up their account by email
            let acct: Option<String> =
                sqlx::query_scalar("SELECT account_number FROM accounts WHERE email = $1")
                    .bind(&user_email)
                    .fetch_optional(&state.db)
                    .await?;
            match acct {
                Some(n) => n,
                None => {
                    return Err(EscudoError::BadRequest(
                        "No account linked to this user".into(),
                    ));
                }
            }
        }
    };

    // Check the tier
    let tier: String = sqlx::query_scalar("SELECT tier FROM accounts WHERE account_number = $1")
        .bind(&account_number)
        .fetch_one(&state.db)
        .await?;

    // For free-tier accounts, check if android_id is already used on a different account
    if tier == "free" {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT account_number FROM device_fingerprints \
             WHERE android_id = $1 AND account_number != $2 \
             LIMIT 1",
        )
        .bind(&body.android_id)
        .bind(&account_number)
        .fetch_optional(&state.db)
        .await?;

        if existing.is_some() {
            return Err(EscudoError::Forbidden(
                "This device is already registered to another free account".into(),
            ));
        }
    }

    // Upsert device fingerprint
    sqlx::query(
        "INSERT INTO device_fingerprints (account_number, android_id, advertising_id, device_model, os_version) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (android_id, account_number) DO UPDATE SET \
           advertising_id = EXCLUDED.advertising_id, \
           device_model = EXCLUDED.device_model, \
           os_version = EXCLUDED.os_version",
    )
    .bind(&account_number)
    .bind(&body.android_id)
    .bind(&body.advertising_id)
    .bind(&body.device_model)
    .bind(&body.os_version)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE device_fingerprints
         SET ip_address = COALESCE($3, ip_address)
         WHERE account_number = $1 AND android_id = $2",
    )
    .bind(&account_number)
    .bind(&body.android_id)
    .bind(telemetry.ip.map(|ip| ip.to_string()))
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE users
         SET latest_login_ip = COALESCE(CAST($2 AS inet), latest_login_ip),
             latest_login_country = COALESCE($3, latest_login_country),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(auth.0.sub)
    .bind(telemetry.ip.map(|ip| ip.to_string()))
    .bind(normalize_country(
        telemetry
            .country_code
            .as_deref()
            .or(telemetry.country.as_deref()),
    ))
    .execute(&state.db)
    .await?;

    // Update device count
    sqlx::query(
        "UPDATE accounts SET devices_count = \
         (SELECT COUNT(*) FROM device_fingerprints WHERE account_number = $1) \
         WHERE account_number = $1",
    )
    .bind(&account_number)
    .execute(&state.db)
    .await?;

    refresh_user_abuse_score(&state, auth.0.sub, &account_number).await?;

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        None,
        None,
        "register_device",
        "success",
        Some("Device fingerprint registered".into()),
        serde_json::json!({
            "account_number": account_number,
            "android_id_present": true,
            "device_model": body.device_model,
            "os_version": body.os_version,
            "platform": telemetry.inferred_platform,
        }),
    )
    .await
    {
        tracing::warn!(
            "Failed to record device registration journey event for user {}: {}",
            auth.0.sub,
            error
        );
    }

    Ok(Json(serde_json::json!({
        "message": "Device registered",
        "account_number": account_number,
    })))
}

fn device_limit_for_tier(tier: &str) -> i64 {
    match tier {
        "escudo" => 5,
        "pro" => 10,
        "dedicated" => 10,
        _ => 1,
    }
}

async fn refresh_user_abuse_score(
    state: &AppState,
    user_id: Uuid,
    account_number: &str,
) -> Result<(), EscudoError> {
    let tier: String = sqlx::query_scalar("SELECT tier FROM accounts WHERE account_number = $1")
        .bind(account_number)
        .fetch_one(&state.db)
        .await?;
    let device_limit = device_limit_for_tier(&tier);

    let row = sqlx::query(
        r#"
        WITH active_devices AS (
            SELECT COUNT(*)::BIGINT AS count
            FROM devices
            WHERE user_id = $1 AND is_active = true
        ),
        shared_android AS (
            SELECT COUNT(DISTINCT df2.account_number)::BIGINT AS count
            FROM device_fingerprints df1
            JOIN device_fingerprints df2
              ON df1.android_id = df2.android_id
             AND df1.account_number <> df2.account_number
            WHERE df1.account_number = $2
        ),
        shared_ad AS (
            SELECT COUNT(DISTINCT df2.account_number)::BIGINT AS count
            FROM device_fingerprints df1
            JOIN device_fingerprints df2
              ON df1.advertising_id IS NOT NULL
             AND df1.advertising_id <> ''
             AND df2.advertising_id = df1.advertising_id
             AND df1.account_number <> df2.account_number
            WHERE df1.account_number = $2
        ),
        geo_mismatch AS (
            SELECT CASE
                WHEN signup_country IS NOT NULL
                 AND latest_login_country IS NOT NULL
                 AND signup_country <> latest_login_country
                THEN 1 ELSE 0
            END AS mismatch
            FROM users
            WHERE id = $1
        )
        SELECT
            COALESCE((SELECT count FROM active_devices), 0) AS active_devices,
            COALESCE((SELECT count FROM shared_android), 0) AS shared_android_accounts,
            COALESCE((SELECT count FROM shared_ad), 0) AS shared_ad_accounts,
            COALESCE((SELECT mismatch FROM geo_mismatch), 0) AS geo_mismatch
        "#,
    )
    .bind(user_id)
    .bind(account_number)
    .fetch_one(&state.db)
    .await?;

    let active_devices: i64 = row.get("active_devices");
    let shared_android_accounts: i64 = row.get("shared_android_accounts");
    let shared_ad_accounts: i64 = row.get("shared_ad_accounts");
    let geo_mismatch: i32 = row.get("geo_mismatch");

    let mut score = 0_i32;
    if active_devices > device_limit {
        score += ((active_devices - device_limit) as i32 * 10).min(40);
    }
    if shared_android_accounts > 0 {
        score += (shared_android_accounts as i32 * 35).min(70);
    }
    if shared_ad_accounts > 0 {
        score += (shared_ad_accounts as i32 * 20).min(40);
    }
    if geo_mismatch > 0 {
        score += 5;
    }
    score = score.min(100);

    sqlx::query("UPDATE users SET abuse_score = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .bind(score)
        .execute(&state.db)
        .await?;

    Ok(())
}
