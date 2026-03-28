use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use escudo_common::EscudoError;
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::backend_control;
use crate::middleware::AuthUser;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
}

pub async fn create_checkout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<CheckoutResponse>> {
    backend_control::enforce_paid_checkout_allowed(&state.db).await?;

    let stripe_config = state
        .config
        .stripe
        .as_ref()
        .ok_or_else(|| EscudoError::Internal("Stripe not configured".into()))?;

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .header(
            "Authorization",
            format!("Bearer {}", stripe_config.secret_key),
        )
        .form(&[
            ("mode", "subscription"),
            (
                "success_url",
                &format!("{}/billing/success", stripe_config.app_url),
            ),
            (
                "cancel_url",
                &format!("{}/billing/cancel", stripe_config.app_url),
            ),
            ("line_items[0][price]", &stripe_config.price_id),
            ("line_items[0][quantity]", "1"),
            ("client_reference_id", &auth.0.sub.to_string()),
        ])
        .send()
        .await
        .map_err(|e| EscudoError::Internal(format!("Stripe request failed: {e}")))?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        error!("Stripe checkout creation failed: {body}");
        return Err(EscudoError::Internal(
            "Failed to create checkout session".into(),
        ));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| EscudoError::Internal(format!("Invalid Stripe response: {e}")))?;

    let url = body["url"]
        .as_str()
        .ok_or_else(|| EscudoError::Internal("Missing checkout URL".into()))?
        .to_string();

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        None,
        None,
        "checkout_start",
        "success",
        Some("Stripe checkout session created".into()),
        serde_json::json!({ "provider": "stripe" }),
    )
    .await
    {
        warn!(
            "Failed to record checkout_start journey event for user {}: {}",
            auth.0.sub,
            error
        );
    }

    Ok(Json(CheckoutResponse { checkout_url: url }))
}

#[derive(Serialize)]
pub struct BillingStatus {
    pub plan: String,
    pub status: String,
    pub period_end: Option<String>,
    pub bandwidth_limit_bytes: i64,
}

pub async fn get_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<BillingStatus>> {
    let sub = sqlx::query_as::<_, (String, String, chrono::DateTime<chrono::Utc>, i64)>(
        "SELECT plan, status, period_end, bandwidth_limit_bytes FROM subscriptions WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?;

    match sub {
        Some((plan, status, period_end, bandwidth_limit)) => Ok(Json(BillingStatus {
            plan,
            status,
            period_end: Some(period_end.to_rfc3339()),
            bandwidth_limit_bytes: bandwidth_limit,
        })),
        None => Ok(Json(BillingStatus {
            plan: "free".to_string(),
            status: "none".to_string(),
            period_end: None,
            bandwidth_limit_bytes: 0,
        })),
    }
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> axum::response::Response {
    let stripe_config = match &state.config.stripe {
        Some(c) => c,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Validate Stripe signature
    let sig_header = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    if !verify_stripe_signature(&body, &sig_header, &stripe_config.webhook_secret) {
        warn!("Invalid Stripe webhook signature");
        return StatusCode::BAD_REQUEST.into_response();
    }

    let event: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let event_type = event["type"].as_str().unwrap_or("");
    info!("Stripe webhook: {event_type}");

    match event_type {
        "checkout.session.completed" => {
            let session = &event["data"]["object"];
            let customer_id = session["customer"].as_str().unwrap_or("");
            let subscription_id = session["subscription"].as_str().unwrap_or("");
            let user_id_str = session["client_reference_id"].as_str().unwrap_or("");

            let user_id: uuid::Uuid = match user_id_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    error!("Invalid user_id in checkout session: {user_id_str}");
                    return StatusCode::OK.into_response();
                }
            };

            if let Err(e) = sqlx::query(
                r#"INSERT INTO subscriptions (user_id, stripe_customer_id, stripe_subscription_id, plan, status)
                   VALUES ($1, $2, $3, 'pro', 'active')
                   ON CONFLICT (stripe_subscription_id) DO UPDATE SET status = 'active', updated_at = NOW()"#,
            )
            .bind(user_id)
            .bind(customer_id)
            .bind(subscription_id)
            .execute(&state.db)
            .await
            {
                error!("Webhook DB error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            if let Err(e) = backend_control::sync_user_tier(&state.db, user_id, "pro").await {
                error!("Failed to sync user/account tier after checkout completion: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            if let Err(e) = backend_control::record_journey_event(
                &state.db,
                Some(user_id),
                None,
                None,
                "checkout_complete",
                "success",
                Some("Stripe checkout session completed".into()),
                serde_json::json!({ "provider": "stripe" }),
            )
            .await
            {
                error!("Failed to record checkout_complete journey event: {e}");
            }

            info!("Subscription activated for user {user_id}");
        }
        "invoice.paid" => {
            let invoice = &event["data"]["object"];
            let subscription_id = invoice["subscription"].as_str().unwrap_or("");
            let period_end = invoice["period_end"].as_i64().unwrap_or(0);

            let period_end_dt =
                chrono::DateTime::from_timestamp(period_end, 0).unwrap_or_else(chrono::Utc::now);

            if let Err(e) = sqlx::query(
                "UPDATE subscriptions SET status = 'active', period_end = $1, updated_at = NOW() WHERE stripe_subscription_id = $2",
            )
            .bind(period_end_dt)
            .bind(subscription_id)
            .execute(&state.db)
            .await
            {
                error!("Webhook DB error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
        "customer.subscription.deleted" => {
            let sub = &event["data"]["object"];
            let subscription_id = sub["id"].as_str().unwrap_or("");

            let user_id: Option<uuid::Uuid> = match sqlx::query_scalar(
                "UPDATE subscriptions SET status = 'cancelled', updated_at = NOW() WHERE stripe_subscription_id = $1 RETURNING user_id",
            )
            .bind(subscription_id)
            .fetch_optional(&state.db)
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("Webhook DB error: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            if let Some(uid) = user_id {
                if let Err(e) = backend_control::sync_user_tier(&state.db, uid, "free").await {
                    error!("Failed to sync cancelled subscription tier: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                if let Err(e) = backend_control::record_journey_event(
                    &state.db,
                    Some(uid),
                    None,
                    None,
                    "subscription_cancelled",
                    "success",
                    Some("Stripe subscription cancelled".into()),
                    serde_json::json!({ "provider": "stripe" }),
                )
                .await
                {
                    error!("Failed to record subscription_cancelled journey event: {e}");
                }
                info!("Subscription cancelled for user {uid}");
            }
        }
        _ => {
            info!("Unhandled webhook event: {event_type}");
        }
    }

    StatusCode::OK.into_response()
}

fn verify_stripe_signature(payload: &str, sig_header: &str, secret: &str) -> bool {
    // Parse signature header: "t=timestamp,v1=signature"
    let mut timestamp = "";
    let mut signature = "";

    for part in sig_header.split(',') {
        if let Some(val) = part.strip_prefix("t=") {
            timestamp = val;
        } else if let Some(val) = part.strip_prefix("v1=") {
            signature = val;
        }
    }

    if timestamp.is_empty() || signature.is_empty() {
        return false;
    }

    // Reject replayed webhooks: timestamp must be within 5 minutes
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = chrono::Utc::now().timestamp();
        if (now - ts).abs() > 300 {
            warn!("Webhook timestamp too old: {ts} vs now {now}");
            return false;
        }
    } else {
        return false;
    }

    let signed_payload = format!("{timestamp}.{payload}");

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());

    let expected = hex::encode(&mac.finalize().into_bytes());
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
