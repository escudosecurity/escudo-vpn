use axum::extract::State;
use axum::Json;
use escudo_common::EscudoError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ReferralInfo {
    pub id: Uuid,
    pub code: String,
    pub referrer_id: Uuid,
}

#[derive(Serialize)]
pub struct ReferralStatus {
    pub code: Option<String>,
    pub total_sent: i64,
    pub total_redeemed: i64,
    pub months_earned: i64,
}

#[derive(Deserialize)]
pub struct RedeemRequest {
    pub code: String,
}

fn generate_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn generate_referral(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<ReferralInfo>> {
    // Check if user already has an unused referral code
    let existing = sqlx::query_as::<_, (Uuid, String, Uuid)>(
        "SELECT id, code, referrer_id FROM referrals WHERE referrer_id = $1 AND referred_id IS NULL LIMIT 1",
    )
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?;

    if let Some((id, code, referrer_id)) = existing {
        return Ok(Json(ReferralInfo {
            id,
            code,
            referrer_id,
        }));
    }

    let code = generate_code();

    let (id,): (Uuid,) =
        sqlx::query_as("INSERT INTO referrals (referrer_id, code) VALUES ($1, $2) RETURNING id")
            .bind(auth.0.sub)
            .bind(&code)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(ReferralInfo {
        id,
        code,
        referrer_id: auth.0.sub,
    }))
}

pub async fn get_referral_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<ReferralStatus>> {
    let code: Option<String> =
        sqlx::query_scalar("SELECT code FROM referrals WHERE referrer_id = $1 LIMIT 1")
            .bind(auth.0.sub)
            .fetch_optional(&state.db)
            .await?;

    let total_sent: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM referrals WHERE referrer_id = $1")
            .bind(auth.0.sub)
            .fetch_one(&state.db)
            .await?;

    let total_redeemed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM referrals WHERE referrer_id = $1 AND referred_id IS NOT NULL",
    )
    .bind(auth.0.sub)
    .fetch_one(&state.db)
    .await?;

    let months_earned = total_redeemed;

    Ok(Json(ReferralStatus {
        code,
        total_sent,
        total_redeemed,
        months_earned,
    }))
}

pub async fn redeem_referral(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<RedeemRequest>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let code = body.code.trim().to_uppercase();

    if code.is_empty() {
        return Err(EscudoError::BadRequest(
            "Referral code cannot be empty".into(),
        ));
    }

    let referral = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id, referrer_id FROM referrals WHERE code = $1 AND referred_id IS NULL",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await?;

    let (referral_id, referrer_id) = match referral {
        Some(r) => r,
        None => {
            return Err(EscudoError::NotFound(
                "Invalid or already redeemed referral code".into(),
            ))
        }
    };

    if referrer_id == auth.0.sub {
        return Err(EscudoError::BadRequest(
            "Cannot redeem your own referral code".into(),
        ));
    }

    sqlx::query("UPDATE referrals SET referred_id = $1, redeemed_at = NOW() WHERE id = $2")
        .bind(auth.0.sub)
        .bind(referral_id)
        .execute(&state.db)
        .await?;

    // Extend referrer subscription by 1 month
    sqlx::query(
        r#"
        UPDATE subscriptions
        SET period_end = period_end + INTERVAL '1 month'
        WHERE user_id = $1 AND status = 'active'
        "#,
    )
    .bind(referrer_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "status": "redeemed" })))
}
