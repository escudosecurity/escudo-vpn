use axum::extract::State;
use axum::Json;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;
use escudo_common::EscudoError;

// ---------------------------------------------------------------------------
// GetMoons API client
// ---------------------------------------------------------------------------

const GETMOONS_BASE: &str = "https://apibeta.getmoons.com/v2";

struct GetMoonsClient {
    http: Client,
    partner_id: String,
    token: String,
}

impl GetMoonsClient {
    fn new(partner_id: String, token: String) -> Self {
        Self {
            http: Client::new(),
            partner_id,
            token,
        }
    }

    async fn ramp_on_quote(&self, amount_brl: f64) -> Result<f64, EscudoError> {
        let resp = self
            .http
            .post(format!("{GETMOONS_BASE}/ramp/on/quote"))
            .header("Partner-X", &self.partner_id)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({
                "gmid": "",
                "whitelist": false,
                "amount": amount_brl,
                "asset": "USDC",
                "chain": "MATIC"
            }))
            .send()
            .await
            .map_err(|e| EscudoError::Internal(format!("GetMoons quote failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EscudoError::Internal(format!("GetMoons parse failed: {e}")))?;

        if body["success"].as_bool() != Some(true) {
            let err = body["error"]["message"].as_str().unwrap_or("unknown error");
            return Err(EscudoError::Internal(format!(
                "GetMoons quote error: {err}"
            )));
        }

        let usdt = body["data"]["payout"]["amount"]
            .as_f64()
            .ok_or_else(|| EscudoError::Internal("GetMoons: missing payout amount".into()))?;

        Ok(usdt)
    }

    async fn ramp_on_create(
        &self,
        amount_brl: f64,
        usdt_address: &str,
    ) -> Result<PixPayment, EscudoError> {
        let resp = self
            .http
            .post(format!("{GETMOONS_BASE}/ramp/on/create"))
            .header("Partner-X", &self.partner_id)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({
                "gmid": "",
                "whitelist": false,
                "amount": amount_brl,
                "asset": "USDC",
                "chain": "MATIC",
                "address": usdt_address,
                "tag": ""
            }))
            .send()
            .await
            .map_err(|e| EscudoError::Internal(format!("GetMoons create failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EscudoError::Internal(format!("GetMoons parse failed: {e}")))?;

        if body["success"].as_bool() != Some(true) {
            let err = body["error"]["message"].as_str().unwrap_or("unknown error");
            return Err(EscudoError::Internal(format!(
                "GetMoons create error: {err}"
            )));
        }

        let data = &body["data"];
        Ok(PixPayment {
            order_id: data["id"].as_str().unwrap_or("").to_string(),
            status: data["status"].as_str().unwrap_or("waiting").to_string(),
            pix_qr_code: data["payin"]["address"].as_str().unwrap_or("").to_string(),
            amount_brl: data["payin"]["amount"].as_f64().unwrap_or(amount_brl),
            usdt_amount: data["payout"]["amount"].as_f64().unwrap_or(0.0),
            expires_at: data["expiresAt"].as_str().unwrap_or("").to_string(),
        })
    }

    async fn ramp_on_status(&self, order_id: &str) -> Result<PaymentStatus, EscudoError> {
        let resp = self
            .http
            .get(format!("{GETMOONS_BASE}/ramp/on/status/{order_id}"))
            .header("Partner-X", &self.partner_id)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(|e| EscudoError::Internal(format!("GetMoons status failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EscudoError::Internal(format!("GetMoons parse failed: {e}")))?;

        if body["success"].as_bool() != Some(true) {
            let err = body["error"]["message"].as_str().unwrap_or("unknown error");
            return Err(EscudoError::Internal(format!(
                "GetMoons status error: {err}"
            )));
        }

        let data = &body["data"];
        Ok(PaymentStatus {
            order_id: data["id"].as_str().unwrap_or("").to_string(),
            status: data["status"].as_str().unwrap_or("unknown").to_string(),
            paid_at: data["paidAt"].as_str().map(|s| s.to_string()),
        })
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct PixPayment {
    order_id: String,
    status: String,
    pix_qr_code: String,
    amount_brl: f64,
    usdt_amount: f64,
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct PaymentStatus {
    order_id: String,
    status: String,
    paid_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Plan pricing (all reduce to 8 in numerology)
// ---------------------------------------------------------------------------

fn plan_price_brl(plan: &str) -> Option<f64> {
    match plan {
        "escudo" => Some(9.80),
        "pro" => Some(35.0),
        "dedicated_ip" => Some(17.0),
        _ => None,
    }
}

fn plan_price_with_discount(plan: &str, period: &str) -> Option<f64> {
    let base = plan_price_brl(plan)?;
    let months = match period {
        "quarterly" => 3.0,
        "yearly" => 12.0,
        "2year" => 24.0,
        _ => 1.0, // monthly
    };
    let discount = match period {
        "quarterly" => 0.08, // 8% off
        "yearly" => 0.17,    // 17% off
        "2year" => 0.26,     // 26% off
        _ => 0.0,
    };
    Some(base * months * (1.0 - discount))
}

// ---------------------------------------------------------------------------
// API Routes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePixRequest {
    pub plan: String,           // "escudo" | "pro" | "dedicated_ip"
    pub period: Option<String>, // "monthly" | "quarterly" | "yearly" | "2year"
}

#[derive(Serialize)]
pub struct CreatePixResponse {
    pub order_id: String,
    pub pix_qr_code: String,
    pub pix_copy_paste: String,
    pub amount_brl: f64,
    pub plan: String,
    pub period: String,
    pub expires_at: String,
}

/// POST /api/v1/billing/pix/create
/// Customer wants to subscribe — generate PIX QR code
pub async fn create_pix_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePixRequest>,
) -> escudo_common::Result<Json<CreatePixResponse>> {
    let period = req.period.unwrap_or_else(|| "monthly".to_string());

    let total_brl = plan_price_with_discount(&req.plan, &period)
        .ok_or_else(|| EscudoError::BadRequest("Invalid plan".into()))?;

    info!(
        user_id = %auth.0.sub,
        plan = %req.plan,
        period = %period,
        amount = total_brl,
        "Creating PIX payment"
    );

    let partner_id = std::env::var("GETMOONS_PARTNER_ID")
        .map_err(|_| EscudoError::Internal("GETMOONS_PARTNER_ID not set".into()))?;
    let token = std::env::var("GETMOONS_ACCESS_TOKEN")
        .map_err(|_| EscudoError::Internal("GETMOONS_ACCESS_TOKEN not set".into()))?;
    let usdt_address = std::env::var("ESCUDO_USDC_ADDRESS")
        .unwrap_or_else(|_| "YOUR_POLYGON_USDC_ADDRESS_HERE".to_string());

    let client = GetMoonsClient::new(partner_id, token);
    let payment = client.ramp_on_create(total_brl, &usdt_address).await?;

    // Store payment reference in DB for polling
    sqlx::query(
        "INSERT INTO subscriptions (user_id, plan, tier, status, stripe_customer_id) VALUES ($1, $2, $3, 'pending', $4)"
    )
    .bind(auth.0.sub)
    .bind(&req.plan)
    .bind(&req.plan)
    .bind(&payment.order_id) // reuse stripe_customer_id column for getmoons order_id
    .execute(&state.db)
    .await
    .ok(); // ignore if subscription already exists

    Ok(Json(CreatePixResponse {
        order_id: payment.order_id,
        pix_qr_code: payment.pix_qr_code.clone(),
        pix_copy_paste: payment.pix_qr_code,
        amount_brl: payment.amount_brl,
        plan: req.plan,
        period,
        expires_at: payment.expires_at,
    }))
}

#[derive(Deserialize)]
pub struct CheckPaymentRequest {
    pub order_id: String,
}

#[derive(Serialize)]
pub struct CheckPaymentResponse {
    pub status: String,
    pub paid: bool,
    pub plan_activated: bool,
}

/// POST /api/v1/billing/pix/status
/// App polls this to check if customer paid
pub async fn check_pix_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CheckPaymentRequest>,
) -> escudo_common::Result<Json<CheckPaymentResponse>> {
    let partner_id = std::env::var("GETMOONS_PARTNER_ID")
        .map_err(|_| EscudoError::Internal("GETMOONS_PARTNER_ID not set".into()))?;
    let token = std::env::var("GETMOONS_ACCESS_TOKEN")
        .map_err(|_| EscudoError::Internal("GETMOONS_ACCESS_TOKEN not set".into()))?;

    let client = GetMoonsClient::new(partner_id, token);
    let status = client.ramp_on_status(&req.order_id).await?;

    let paid = status.status == "completed";
    let mut plan_activated = false;

    if paid {
        // Activate subscription
        let updated = sqlx::query(
            "UPDATE subscriptions SET status = 'active', period_start = now(), period_end = now() + interval '30 days' WHERE user_id = $1 AND stripe_customer_id = $2 AND status = 'pending'"
        )
        .bind(auth.0.sub)
        .bind(&req.order_id)
        .execute(&state.db)
        .await;

        if let Ok(result) = updated {
            plan_activated = result.rows_affected() > 0;
            if plan_activated {
                info!(user_id = %auth.0.sub, order_id = %req.order_id, "PIX payment confirmed — subscription activated");
            }
        }
    }

    Ok(Json(CheckPaymentResponse {
        status: status.status,
        paid,
        plan_activated,
    }))
}

// ---------------------------------------------------------------------------
// GetMoons Webhook — receives payment notifications automatically
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct GetMoonsWebhook {
    pub id: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "gmid")]
    pub gmid: Option<String>,
    pub payin: Option<WebhookPayin>,
    pub payout: Option<WebhookPayout>,
}

#[derive(Deserialize)]
pub struct WebhookPayin {
    pub amount: Option<f64>,
    pub asset: Option<String>,
}

#[derive(Deserialize)]
pub struct WebhookPayout {
    pub amount: Option<f64>,
    pub asset: Option<String>,
    pub hash: Option<String>,
}

/// POST /api/v1/billing/pix/webhook
/// GetMoons calls this when a PIX payment is confirmed.
/// No JWT auth — validated by checking the order exists in our DB.
pub async fn pix_webhook(
    State(state): State<AppState>,
    Json(payload): Json<GetMoonsWebhook>,
) -> axum::http::StatusCode {
    let order_id = match &payload.id {
        Some(id) => id.clone(),
        None => {
            error!("PIX webhook: missing order ID");
            return axum::http::StatusCode::BAD_REQUEST;
        }
    };

    let status = payload.status.as_deref().unwrap_or("unknown");

    info!(
        order_id = %order_id,
        status = %status,
        amount = ?payload.payin.as_ref().and_then(|p| p.amount),
        "PIX webhook received"
    );

    if status != "completed" {
        // Not paid yet — just log and acknowledge
        info!(order_id = %order_id, status = %status, "PIX webhook: payment not yet completed");
        return axum::http::StatusCode::OK;
    }

    // Payment completed — activate the subscription
    // Find the pending subscription by getmoons order_id (stored in stripe_customer_id column)
    let result = sqlx::query(
        "UPDATE subscriptions SET status = 'active', period_start = now(), period_end = now() + interval '30 days' WHERE stripe_customer_id = $1 AND status = 'pending'"
    )
    .bind(&order_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            info!(order_id = %order_id, "PIX webhook: subscription ACTIVATED");
        }
        Ok(_) => {
            // No pending subscription found — might be a duplicate or unknown order
            info!(order_id = %order_id, "PIX webhook: no pending subscription found for this order");
        }
        Err(e) => {
            error!(order_id = %order_id, "PIX webhook: DB error activating subscription: {e}");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    axum::http::StatusCode::OK
}
