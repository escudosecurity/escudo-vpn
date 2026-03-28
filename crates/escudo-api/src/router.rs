use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::ConnectInfo;
use axum::http::header;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::routes::{
    account, anon_auth, auth, billing, billing_dlocal, billing_pix, family, favorites, internal, network, profiles,
    referral, security, settings, stats, vpn, ws,
};
use crate::state::AppState;
use crate::backend_control;

const TEST_PAGE: &str = include_str!("../../../site/test.html");
const APK_BYTES: &[u8] = include_bytes!("../../../site/escudo-vpn.apk");
const PRIVACY_PAGE: &str = include_str!("../../../site/privacy.html");

/// Simple in-memory rate limiter: 10 requests per 60 seconds per IP.
#[derive(Clone)]
struct RateLimiter {
    state: Arc<Mutex<HashMap<IpAddr, (u32, std::time::Instant)>>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

const RATE_LIMIT_MAX: u32 = 10;
const RATE_LIMIT_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);

async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    axum::extract::Extension(limiter): axum::extract::Extension<RateLimiter>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let ip = addr.ip();
    let mut map = limiter.state.lock().await;
    let now = std::time::Instant::now();
    map.retain(|_, (_, seen_at)| now.duration_since(*seen_at) <= RATE_LIMIT_WINDOW);

    let entry = map.entry(ip).or_insert((0, now));
    if now.duration_since(entry.1) > RATE_LIMIT_WINDOW {
        *entry = (0, now);
    }
    entry.0 += 1;
    if entry.0 > RATE_LIMIT_MAX {
        drop(map);
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    drop(map);

    next.run(request).await
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "https://escudovpn.com".parse().unwrap(),
            "https://www.escudovpn.com".parse().unwrap(),
            "https://api.escudovpn.com".parse().unwrap(),
        ]))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let rate_limiter = RateLimiter::new();

    // Auth routes with rate limiting
    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(axum::Extension(rate_limiter));

    let api = Router::new()
        .merge(auth_routes)
        .route("/auth/anonymous", post(anon_auth::create_anonymous_account))
        .route("/auth/login-number", post(anon_auth::login_with_number))
        .route("/account/email", put(anon_auth::add_email))
        .route("/auth/qr/generate", post(anon_auth::generate_qr_token))
        .route("/auth/qr/scan", post(anon_auth::scan_qr_token))
        .route("/auth/register-device", post(anon_auth::register_device))
        .route("/launch/status", get(backend_control::get_launch_status))
        .route("/launch/redeem-invite", post(backend_control::redeem_invite))
        .route("/servers", get(vpn::list_servers))
        .route("/connect", post(vpn::connect))
        .route("/connect/multihop", post(vpn::connect_multihop))
        .route("/connect/private-mode", post(vpn::connect_private_mode))
        .route("/disconnect/:id", delete(vpn::disconnect))
        .route("/peers", get(vpn::list_peers))
        .route("/config/:id/qr", get(vpn::get_config_qr))
        .route("/usage", get(vpn::get_usage))
        .route("/network/me", get(network::get_network_info))
        .route(
            "/account",
            get(account::get_account).delete(account::delete_account),
        )
        .route("/ws/stats", get(ws::stats_ws))
        .route("/billing/checkout", post(billing::create_checkout))
        .route("/billing/status", get(billing::get_status))
        .route("/billing/pix/create", post(billing_pix::create_pix_payment))
        .route("/billing/pix/status", post(billing_pix::check_pix_payment))
        .route("/stats/dns", get(stats::get_dns_stats))
        .route("/stats/dns/blocked", get(stats::get_blocked_domains))
        .route("/recents", get(vpn::list_recents))
        .route(
            "/favorites",
            get(favorites::list_favorites).post(favorites::add_favorite),
        )
        .route("/favorites/:server_id", delete(favorites::remove_favorite))
        .route(
            "/settings",
            get(settings::get_settings).put(settings::update_settings),
        )
        .route(
            "/profiles",
            get(profiles::list_profiles).post(profiles::create_profile),
        )
        .route(
            "/profiles/:id",
            put(profiles::update_profile).delete(profiles::delete_profile),
        )
        .route("/referral/generate", post(referral::generate_referral))
        .route("/referral/status", get(referral::get_referral_status))
        .route("/referral/redeem", post(referral::redeem_referral))
        .route(
            "/family/profile",
            get(family::get_profile).put(family::update_profile),
        )
        .route("/family/block-domain", post(family::add_blocked_domain))
        .route("/family/allow-domain", post(family::add_allowed_domain))
        .route("/family/parental/overview", get(family::get_parental_overview))
        .route(
            "/family/parental/children",
            get(family::list_parental_children).post(family::create_parental_child),
        )
        .route("/family/parental/children/:id", patch(family::update_parental_child))
        .route(
            "/family/parental/children/:id/link-device",
            post(family::link_parental_child_device),
        )
        .route(
            "/family/parental/children/:id/policy",
            put(family::upsert_parental_policy),
        )
        .route(
            "/family/parental/children/:id/schedules",
            get(family::list_parental_schedules).post(family::create_parental_schedule),
        )
        .route(
            "/family/parental/children/:id/events",
            get(family::list_parental_events),
        )
        .route("/family/parental/claim-code", post(family::claim_parental_code))
        .route("/family/parental/events", post(family::record_parental_event))
        .route("/family/parental/device-policy", get(family::get_my_device_policy))
        .route("/security/breach-check", post(security::breach_check))
        .route("/security/paste-check", post(security::paste_check))
        .route("/security/latest-breach", get(security::latest_breach))
        .route("/security/breach/:name", get(security::breach_details))
        .route(
            "/security/breach-monitors",
            get(security::list_monitors).post(security::add_monitor),
        )
        .route(
            "/security/breach-monitors/:id",
            delete(security::remove_monitor),
        );

    // Webhooks outside JWT auth (called by payment providers)
    let webhook = Router::new()
        .route("/api/v1/billing/webhook", post(billing::stripe_webhook))
        .route(
            "/api/v1/billing/pix/webhook",
            post(billing_pix::pix_webhook),
        )
        .route(
            "/api/v1/billing/dlocal/webhook",
            post(billing_dlocal::dlocal_webhook),
        );

    // Internal phone-home endpoint (authenticated via DEPLOY_SECRET bearer token)
    let internal_routes = Router::new()
        .route(
            "/internal/servers/register",
            post(internal::register_server),
        )
        .route(
            "/internal/servers/:label/proxy-credentials",
            get(internal::get_server_proxy_credentials),
        );

    Router::new()
        .nest("/api/v1", api)
        .merge(webhook)
        .merge(internal_routes)
        .route("/health", get(health))
        .route("/test", get(test_page))
        .route("/download/escudo-vpn.apk", get(download_apk))
        .route("/privacy", get(privacy_page))
        .layer(cors)
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn test_page() -> Html<&'static str> {
    Html(TEST_PAGE)
}

async fn privacy_page() -> Html<&'static str> {
    Html(PRIVACY_PAGE)
}

async fn download_apk() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.android.package-archive",
            ),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"escudo-vpn.apk\"",
            ),
        ],
        APK_BYTES,
    )
}
