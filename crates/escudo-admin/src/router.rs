use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::routes::{dashboard, ops, servers, stats, tenants, users};
use crate::state::AdminState;

pub fn create_router(state: AdminState) -> Router {
    let admin = Router::new()
        .route("/users", get(users::list_users))
        .route("/users/:id/suspend", patch(users::suspend_user))
        .route("/users/:id", delete(users::delete_user))
        .route("/servers", get(servers::list_servers))
        .route("/servers", post(servers::create_server))
        .route("/servers/:id", patch(servers::update_server))
        .route("/ops/overview", get(ops::get_overview))
        .route("/ops/alerts", get(ops::get_alerts))
        .route("/ops/snapshot", get(ops::get_snapshot))
        .route("/ops/nodes", get(ops::list_nodes))
        .route("/ops/launch-controls", get(ops::get_launch_controls))
        .route("/ops/launch-controls", patch(ops::update_launch_controls))
        .route("/ops/invite-codes", get(ops::list_invite_codes))
        .route("/ops/invite-codes", post(ops::create_invite_code))
        .route("/ops/sessions", get(ops::list_sessions))
        .route("/ops/journey-events", get(ops::list_journey_events))
        .route("/ops/parental-overview", get(ops::get_parental_overview))
        .route("/tenants", get(tenants::list_tenants))
        .route("/tenants", post(tenants::create_tenant))
        .route("/stats", get(stats::get_stats))
        .route("/dashboard", get(dashboard::dashboard_page));

    Router::new()
        .nest("/admin/v1", admin)
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}
