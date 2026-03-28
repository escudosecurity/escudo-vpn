use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct LaunchControls {
    pub maintenance_mode: bool,
    pub allow_public_signup: bool,
    pub allow_anonymous_signup: bool,
    pub allow_connect: bool,
    pub allow_paid_checkout: bool,
    pub healthy_only_routing: bool,
    pub expose_paid_tiers: bool,
    pub free_beta_label: String,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LaunchStatusResponse {
    pub controls: LaunchControls,
    pub effective_tier: String,
    pub active_invites: i64,
}

#[derive(Debug, Deserialize)]
pub struct RedeemInviteRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RedeemInviteResponse {
    pub code: String,
    pub tier: String,
    pub plan: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub async fn get_launch_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<LaunchStatusResponse>> {
    let controls = fetch_launch_controls(&state.db).await?;
    let active_invites: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM invite_code_redemptions WHERE user_id = $1",
    )
    .bind(auth.0.sub)
    .fetch_one(&state.db)
    .await?;
    let effective_tier = effective_user_tier(&state.db, auth.0.sub).await?;

    Ok(Json(LaunchStatusResponse {
        controls,
        effective_tier,
        active_invites,
    }))
}

pub async fn redeem_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<RedeemInviteRequest>,
) -> escudo_common::Result<Json<RedeemInviteResponse>> {
    let code = body.code.trim().to_uppercase();
    if code.is_empty() {
        return Err(EscudoError::BadRequest("Invite code cannot be empty".into()));
    }

    let mut tx = state.db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT id, tier, plan, duration_days, max_uses, used_count, expires_at
        FROM invite_codes
        WHERE code = $1
          AND active = true
          AND (expires_at IS NULL OR expires_at > NOW())
        FOR UPDATE
        "#,
    )
    .bind(&code)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Invite code not found or expired".into()))?;

    let invite_id: Uuid = row.get("id");
    let tier: String = row.get("tier");
    let plan: String = row.get("plan");
    let duration_days: i32 = row.get("duration_days");
    let max_uses: i32 = row.get("max_uses");
    let used_count: i32 = row.get("used_count");
    let expires_at: Option<chrono::DateTime<Utc>> = row.get("expires_at");

    if used_count >= max_uses {
        return Err(EscudoError::Forbidden("Invite code has no uses remaining".into()));
    }

    let already_redeemed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM invite_code_redemptions WHERE invite_code_id = $1 AND user_id = $2)",
    )
    .bind(invite_id)
    .bind(auth.0.sub)
    .fetch_one(&mut *tx)
    .await?;

    if already_redeemed {
        return Err(EscudoError::Conflict("Invite code already redeemed on this account".into()));
    }

    let now = Utc::now();
    let sub_expires_at = now + Duration::days(i64::from(duration_days.max(1)));

    let synthetic_subscription_id = format!("invite:{code}:{}", auth.0.sub);
    let synthetic_customer_id = format!("invite:{}", auth.0.sub);

    sqlx::query(
        r#"
        INSERT INTO invite_code_redemptions (invite_code_id, user_id)
        VALUES ($1, $2)
        "#,
    )
    .bind(invite_id)
    .bind(auth.0.sub)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE invite_codes
        SET used_count = used_count + 1,
            updated_at = NOW(),
            active = CASE WHEN used_count + 1 >= max_uses THEN FALSE ELSE active END
        WHERE id = $1
        "#,
    )
    .bind(invite_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            user_id, stripe_customer_id, stripe_subscription_id, plan, status, period_start, period_end, bandwidth_limit_bytes, tier
        )
        VALUES ($1, $2, $3, $4, 'active', NOW(), $5, 0, $6)
        ON CONFLICT (stripe_subscription_id) DO UPDATE SET
            plan = EXCLUDED.plan,
            status = 'active',
            period_end = EXCLUDED.period_end,
            tier = EXCLUDED.tier,
            updated_at = NOW()
        "#,
    )
    .bind(auth.0.sub)
    .bind(&synthetic_customer_id)
    .bind(&synthetic_subscription_id)
    .bind(&plan)
    .bind(sub_expires_at)
    .bind(&tier)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE users SET subscription_plan = $2, updated_at = NOW() WHERE id = $1")
        .bind(auth.0.sub)
        .bind(&tier)
        .execute(&mut *tx)
        .await?;

    sync_account_tier(&mut tx, auth.0.sub, &tier).await?;
    record_journey_event_tx(
        &mut tx,
        Some(auth.0.sub),
        None,
        None,
        "invite_redeem",
        "success",
        Some(format!("Redeemed invite code {code}")),
        json!({ "code": code, "tier": tier, "plan": plan }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(RedeemInviteResponse {
        code,
        tier,
        plan,
        expires_at: expires_at.unwrap_or(sub_expires_at),
    }))
}

pub async fn fetch_launch_controls(db: &PgPool) -> Result<LaunchControls, EscudoError> {
    let row = sqlx::query(
        r#"
        SELECT maintenance_mode, allow_public_signup, allow_anonymous_signup, allow_connect,
               allow_paid_checkout, healthy_only_routing, expose_paid_tiers, free_beta_label, updated_at
        FROM launch_controls
        WHERE singleton = TRUE
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(LaunchControls {
        maintenance_mode: row.get("maintenance_mode"),
        allow_public_signup: row.get("allow_public_signup"),
        allow_anonymous_signup: row.get("allow_anonymous_signup"),
        allow_connect: row.get("allow_connect"),
        allow_paid_checkout: row.get("allow_paid_checkout"),
        healthy_only_routing: row.get("healthy_only_routing"),
        expose_paid_tiers: row.get("expose_paid_tiers"),
        free_beta_label: row.get("free_beta_label"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn effective_user_tier(db: &PgPool, user_id: Uuid) -> Result<String, EscudoError> {
    let tier = sqlx::query_scalar(
        r#"
        SELECT COALESCE(
            (SELECT s.tier
             FROM subscriptions s
             WHERE s.user_id = $1
               AND s.status = 'active'
               AND s.period_end > NOW()
             ORDER BY s.period_end DESC
             LIMIT 1),
            NULLIF((SELECT u.subscription_plan FROM users u WHERE u.id = $1), ''),
            (SELECT a.tier
             FROM accounts a
             WHERE a.email = ('user:' || $1::text)
                OR a.email = (SELECT email FROM users WHERE id = $1)
             ORDER BY a.created_at DESC
             LIMIT 1),
            'free'
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;

    Ok(tier)
}

pub async fn enforce_signup_allowed(
    db: &PgPool,
    anonymous: bool,
) -> Result<LaunchControls, EscudoError> {
    let controls = fetch_launch_controls(db).await?;
    if controls.maintenance_mode {
        return Err(EscudoError::Forbidden("Service is in maintenance mode".into()));
    }
    if anonymous && !controls.allow_anonymous_signup {
        return Err(EscudoError::Forbidden("Anonymous signup is disabled".into()));
    }
    if !anonymous && !controls.allow_public_signup {
        return Err(EscudoError::Forbidden("Public signup is disabled".into()));
    }
    Ok(controls)
}

pub async fn enforce_connect_allowed(db: &PgPool) -> Result<LaunchControls, EscudoError> {
    let controls = fetch_launch_controls(db).await?;
    if controls.maintenance_mode {
        return Err(EscudoError::Forbidden("Service is in maintenance mode".into()));
    }
    if !controls.allow_connect {
        return Err(EscudoError::Forbidden("New connects are temporarily disabled".into()));
    }
    Ok(controls)
}

pub async fn enforce_paid_checkout_allowed(db: &PgPool) -> Result<LaunchControls, EscudoError> {
    let controls = fetch_launch_controls(db).await?;
    if controls.maintenance_mode {
        return Err(EscudoError::Forbidden("Service is in maintenance mode".into()));
    }
    if !controls.allow_paid_checkout || !controls.expose_paid_tiers {
        return Err(EscudoError::Forbidden("Paid checkout is not enabled yet".into()));
    }
    Ok(controls)
}

pub async fn record_journey_event(
    db: &PgPool,
    user_id: Option<Uuid>,
    device_id: Option<Uuid>,
    server_id: Option<Uuid>,
    event_type: &str,
    outcome: &str,
    detail: Option<String>,
    metadata: serde_json::Value,
) -> Result<(), EscudoError> {
    sqlx::query(
        r#"
        INSERT INTO journey_events (user_id, device_id, server_id, event_type, outcome, detail, event_metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .bind(server_id)
    .bind(event_type)
    .bind(outcome)
    .bind(detail)
    .bind(metadata)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn record_journey_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Option<Uuid>,
    device_id: Option<Uuid>,
    server_id: Option<Uuid>,
    event_type: &str,
    outcome: &str,
    detail: Option<String>,
    metadata: serde_json::Value,
) -> Result<(), EscudoError> {
    sqlx::query(
        r#"
        INSERT INTO journey_events (user_id, device_id, server_id, event_type, outcome, detail, event_metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .bind(server_id)
    .bind(event_type)
    .bind(outcome)
    .bind(detail)
    .bind(metadata)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn open_vpn_session(
    db: &PgPool,
    user_id: Uuid,
    device_id: Uuid,
    server_id: Uuid,
    tier: &str,
    connect_country: Option<String>,
    session_metadata: serde_json::Value,
) -> Result<(), EscudoError> {
    sqlx::query("UPDATE vpn_sessions SET ended_at = NOW(), disconnect_reason = 'reconnect' WHERE device_id = $1 AND ended_at IS NULL")
        .bind(device_id)
        .execute(db)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO vpn_sessions (user_id, device_id, server_id, tier, connect_country, session_metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .bind(server_id)
    .bind(tier)
    .bind(connect_country)
    .bind(session_metadata)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn close_vpn_session(
    db: &PgPool,
    device_id: Uuid,
    disconnect_reason: &str,
) -> Result<(), EscudoError> {
    let usage = sqlx::query(
        r#"
        SELECT COALESCE(SUM(u.rx_bytes), 0)::BIGINT AS rx_bytes,
               COALESCE(SUM(u.tx_bytes), 0)::BIGINT AS tx_bytes
        FROM usage_logs u
        WHERE u.device_id = $1
        "#,
    )
    .bind(device_id)
    .fetch_one(db)
    .await?;

    let rx_bytes: i64 = usage.get("rx_bytes");
    let tx_bytes: i64 = usage.get("tx_bytes");

    sqlx::query(
        r#"
        UPDATE vpn_sessions
        SET ended_at = NOW(),
            disconnect_reason = $2,
            bytes_in = $3,
            bytes_out = $4
        WHERE device_id = $1
          AND ended_at IS NULL
        "#,
    )
    .bind(device_id)
    .bind(disconnect_reason)
    .bind(rx_bytes)
    .bind(tx_bytes)
    .execute(db)
    .await?;

    Ok(())
}

async fn sync_account_tier(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    tier: &str,
) -> Result<(), EscudoError> {
    sqlx::query(
        r#"
        UPDATE accounts
        SET tier = $2
        WHERE email = ('user:' || $1::text)
           OR email = (SELECT email FROM users WHERE id = $1)
        "#,
    )
    .bind(user_id)
    .bind(tier)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn sync_user_tier(
    db: &PgPool,
    user_id: Uuid,
    tier: &str,
) -> Result<(), EscudoError> {
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE users SET subscription_plan = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .bind(tier)
        .execute(&mut *tx)
        .await?;
    sync_account_tier(&mut tx, user_id, tier).await?;
    tx.commit().await?;
    Ok(())
}
