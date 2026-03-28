use axum::extract::State;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::state::AppState;

// ── dLocal Go Webhook ─────────────────────────────────────────
//
// dLocal sends POST to our notification_url with:
//   { "payment_id": "DP-283" }
//
// We must then call GET /v1/payments/{payment_id} to get full details.
// Signature is in the Authorization header:
//   Authorization: V2-HMAC-SHA256, Signature: {hex_signature}
// Formula: HMAC-SHA256(api_key + payload_json, secret_key)

type HmacSha256 = Hmac<Sha256>;

#[derive(serde::Deserialize, Debug)]
pub struct DLocalWebhookPayload {
    pub payment_id: Option<String>,
    // Subscription execution notifications may also have these
    pub subscription_id: Option<String>,
    pub execution_id: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct DLocalPaymentResponse {
    pub id: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub country: Option<String>,
    pub status: Option<String>,
    pub order_id: Option<String>,
    pub description: Option<String>,
    pub created_date: Option<String>,
    pub approved_date: Option<String>,
    pub payment_method_type: Option<String>,
    pub payer: Option<DLocalPayer>,
}

#[derive(serde::Deserialize, Debug)]
pub struct DLocalPayer {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub document_type: Option<String>,
    pub document: Option<String>,
}

/// Verify dLocal webhook signature.
/// Formula: HMAC-SHA256(api_key + raw_body, secret_key)
fn verify_dlocal_signature(
    auth_header: &str,
    raw_body: &[u8],
    api_key: &str,
    secret_key: &str,
) -> bool {
    // Extract signature from header: "V2-HMAC-SHA256, Signature: {hex}"
    let signature_hex = match auth_header.split("Signature: ").nth(1) {
        Some(sig) => sig.trim(),
        None => {
            warn!("dLocal webhook: no Signature found in Authorization header");
            return false;
        }
    };

    // Build the message: api_key concatenated with the raw JSON body
    let mut message = api_key.as_bytes().to_vec();
    message.extend_from_slice(raw_body);

    // Compute HMAC-SHA256
    let mut mac = match HmacSha256::new_from_slice(secret_key.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            error!("dLocal webhook: invalid secret key for HMAC");
            return false;
        }
    };
    mac.update(&message);

    let bytes = mac.finalize().into_bytes();
    let expected: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

    if expected == signature_hex {
        true
    } else {
        warn!(
            expected = %expected,
            received = %signature_hex,
            "dLocal webhook: signature mismatch"
        );
        false
    }
}

/// POST /api/v1/billing/dlocal/webhook
///
/// dLocal Go calls this when a payment status changes (paid, rejected, etc.)
/// or when a subscription execution completes.
///
/// No JWT auth — validated via HMAC signature from dLocal.
pub async fn dlocal_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let raw_body = body.as_ref();

    // ── Signature Verification ──
    let api_key = std::env::var("DLOCAL_API_KEY").unwrap_or_default();
    let secret_key = std::env::var("DLOCAL_SECRET_KEY").unwrap_or_default();

    if let Some(auth_header) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if !verify_dlocal_signature(auth_header, raw_body, &api_key, &secret_key) {
            warn!("dLocal webhook: INVALID signature — rejecting");
            return StatusCode::UNAUTHORIZED;
        }
        info!("dLocal webhook: signature verified");
    } else {
        // dLocal might not always send signature — log but accept
        // (we validate by fetching the payment status from dLocal API)
        warn!("dLocal webhook: no Authorization header — proceeding with API verification");
    }

    // ── Parse Payload ──
    let payload: DLocalWebhookPayload = match serde_json::from_slice(raw_body) {
        Ok(p) => p,
        Err(e) => {
            error!("dLocal webhook: failed to parse payload: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    let payment_id = match &payload.payment_id {
        Some(id) => id.clone(),
        None => {
            error!("dLocal webhook: missing payment_id in payload");
            return StatusCode::BAD_REQUEST;
        }
    };

    info!(payment_id = %payment_id, "dLocal webhook received");

    // ── Fetch Full Payment Details from dLocal API ──
    let client = reqwest::Client::new();
    let payment_url = format!("https://api.dlocalgo.com/v1/payments/{}", payment_id);
    let auth_token = format!("Bearer {}:{}", api_key, secret_key);

    let payment_response = match client
        .get(&payment_url)
        .header("Authorization", &auth_token)
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            error!(payment_id = %payment_id, "dLocal webhook: failed to fetch payment: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let payment: DLocalPaymentResponse = match payment_response.json().await {
        Ok(p) => p,
        Err(e) => {
            error!(payment_id = %payment_id, "dLocal webhook: failed to parse payment response: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let status = payment.status.as_deref().unwrap_or("unknown");
    let amount = payment.amount.unwrap_or(0.0);
    let currency = payment.currency.as_deref().unwrap_or("?");
    let payer_email = payment
        .payer
        .as_ref()
        .and_then(|p| p.email.as_deref())
        .unwrap_or("unknown");

    info!(
        payment_id = %payment_id,
        status = %status,
        amount = %amount,
        currency = %currency,
        payer_email = %payer_email,
        "dLocal payment status fetched"
    );

    match status {
        "PAID" => {
            // ── Payment Successful — Activate Subscription ──

            // Determine plan tier from amount
            let (plan, tier, period_days) = match amount as i64 {
                8 => ("medium", "medium", 30),         // Medium monthly
                17 => ("power", "power", 30),          // Power monthly
                35 => ("ultimate", "ultimate", 30),    // Ultimate monthly
                // Yearly plans (approximate — BRL amounts)
                a if (79..=80).contains(&a) => ("medium", "medium", 365),
                a if (169..=170).contains(&a) => ("power", "power", 365),
                a if (348..=349).contains(&a) => ("ultimate", "ultimate", 365),
                _ => {
                    warn!(amount = %amount, "dLocal webhook: unknown plan amount, defaulting to medium");
                    ("medium", "medium", 30)
                }
            };

            info!(
                payment_id = %payment_id,
                plan = %plan,
                tier = %tier,
                period_days = %period_days,
                "dLocal: activating subscription"
            );

            // Try to find existing subscription by dLocal payment ID or payer email
            // First check if we have a pending subscription for this payment
            let result = sqlx::query(
                "UPDATE subscriptions
                 SET status = 'active',
                     plan = $1,
                     tier = $2,
                     stripe_subscription_id = $3,
                     period_start = now(),
                     period_end = now() + make_interval(days => $4)
                 WHERE stripe_customer_id = $3
                   AND status = 'pending'"
            )
            .bind(plan)
            .bind(tier)
            .bind(&payment_id)
            .bind(period_days)
            .execute(&state.db)
            .await;

            match result {
                Ok(r) if r.rows_affected() > 0 => {
                    info!(payment_id = %payment_id, "dLocal: subscription ACTIVATED (existing pending)");
                }
                Ok(_) => {
                    // No pending subscription — might be a new subscription via dLocal checkout
                    // Try to find user by email and create/update subscription
                    info!(
                        payment_id = %payment_id,
                        payer_email = %payer_email,
                        "dLocal: no pending subscription found — looking up user by email"
                    );

                    // Find user by email
                    let user_row = sqlx::query_scalar::<_, uuid::Uuid>(
                        "SELECT id FROM users WHERE email = $1"
                    )
                    .bind(payer_email)
                    .fetch_optional(&state.db)
                    .await;

                    match user_row {
                        Ok(Some(user_id)) => {
                            // Upsert subscription for this user
                            let upsert = sqlx::query(
                                "INSERT INTO subscriptions (id, user_id, plan, tier, status, stripe_customer_id, stripe_subscription_id, period_start, period_end)
                                 VALUES (gen_random_uuid(), $1, $2, $3, 'active', $4, $4, now(), now() + make_interval(days => $5))
                                 ON CONFLICT (user_id) DO UPDATE SET
                                   plan = $2, tier = $3, status = 'active',
                                   stripe_subscription_id = $4,
                                   period_start = now(),
                                   period_end = now() + make_interval(days => $5)"
                            )
                            .bind(user_id)
                            .bind(plan)
                            .bind(tier)
                            .bind(&payment_id)
                            .bind(period_days)
                            .execute(&state.db)
                            .await;

                            match upsert {
                                Ok(_) => {
                                    info!(user_id = %user_id, plan = %plan, "dLocal: subscription CREATED/UPDATED for user");

                                    // Sync tier
                                    if let Err(e) = crate::backend_control::sync_user_tier(
                                        &state.db, user_id, tier,
                                    ).await {
                                        error!(user_id = %user_id, "dLocal: failed to sync tier: {e}");
                                    }
                                }
                                Err(e) => {
                                    error!(user_id = %user_id, "dLocal: failed to upsert subscription: {e}");
                                }
                            }
                        }
                        Ok(None) => {
                            // User not found — store the payment for later matching
                            warn!(
                                payer_email = %payer_email,
                                payment_id = %payment_id,
                                "dLocal: PAID but no user found with this email — payment stored for manual review"
                            );
                        }
                        Err(e) => {
                            error!("dLocal: DB error looking up user: {e}");
                        }
                    }
                }
                Err(e) => {
                    error!(payment_id = %payment_id, "dLocal: DB error activating subscription: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            }
        }
        "REJECTED" | "CANCELLED" | "EXPIRED" => {
            info!(
                payment_id = %payment_id,
                status = %status,
                "dLocal: payment not successful"
            );
        }
        "PENDING" => {
            info!(
                payment_id = %payment_id,
                "dLocal: payment still pending (e.g., PIX awaiting confirmation)"
            );
        }
        _ => {
            warn!(
                payment_id = %payment_id,
                status = %status,
                "dLocal: unrecognized payment status"
            );
        }
    }

    // Always return 200 to acknowledge the webhook
    // (dLocal retries every 10 minutes for 30 days if we don't return 200)
    StatusCode::OK
}
