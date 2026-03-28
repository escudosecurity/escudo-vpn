use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::middleware::AdminUser;
use crate::state::AdminState;

#[derive(Serialize)]
pub struct OpsOverview {
    pub total_nodes: i64,
    pub healthy_nodes: i64,
    pub warm_nodes: i64,
    pub degraded_nodes: i64,
    pub blocked_nodes: i64,
    pub avg_health_score: f64,
    pub total_assigned_user_cap: i64,
    pub total_active_sessions: i64,
    pub total_assigned_users: i64,
}

#[derive(Serialize)]
pub struct NodeOpsInfo {
    pub id: Uuid,
    pub name: String,
    pub location: String,
    pub public_ip: String,
    pub service_class: String,
    pub provider: Option<String>,
    pub lifecycle_state: String,
    pub health_score: i32,
    pub routing_weight: f64,
    pub assigned_user_cap: i32,
    pub active_session_soft_cap: i32,
    pub active_session_hard_cap: i32,
    pub assigned_users: i64,
    pub active_sessions: i32,
    pub cpu_pct: f64,
    pub ram_pct: f64,
    pub nic_in_mbps: f64,
    pub nic_out_mbps: f64,
    pub connect_success_pct: f64,
    pub median_connect_ms: i32,
}

#[derive(Serialize)]
pub struct LabeledCount {
    pub label: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct TelemetryCoverage {
    pub signup_country_users: i64,
    pub latest_login_country_users: i64,
    pub device_platforms: i64,
    pub device_install_ids: i64,
    pub abuse_flagged_users: i64,
}

#[derive(Serialize)]
pub struct OpsSnapshot {
    pub total_users: i64,
    pub active_users: i64,
    pub paid_users: i64,
    pub total_devices: i64,
    pub active_devices: i64,
    pub active_device_sessions: i64,
    pub dedicated_devices: i64,
    pub sensitive_route_devices: i64,
    pub flagged_users: i64,
    pub high_risk_users: i64,
    pub avg_abuse_score: f64,
    pub total_rx_bytes: i64,
    pub total_tx_bytes: i64,
    pub plan_mix: Vec<LabeledCount>,
    pub user_country_mix: Vec<LabeledCount>,
    pub node_country_mix: Vec<LabeledCount>,
    pub device_platform_mix: Vec<LabeledCount>,
    pub usage_bucket_mix: Vec<LabeledCount>,
    pub preferred_class_mix: Vec<LabeledCount>,
    pub abuse_bucket_mix: Vec<LabeledCount>,
    pub telemetry_coverage: TelemetryCoverage,
}

#[derive(Serialize)]
pub struct OpsAlert {
    pub severity: String,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct ParentalEventOpsRow {
    pub child_id: Uuid,
    pub child_name: String,
    pub event_type: String,
    pub app_identifier: Option<String>,
    pub action: Option<String>,
    pub detail: Option<String>,
    pub occurred_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ParentalOpsOverview {
    pub total_children: i64,
    pub linked_children: i64,
    pub active_child_devices: i64,
    pub active_policies: i64,
    pub active_schedules: i64,
    pub recent_events: i64,
    pub latest_events: Vec<ParentalEventOpsRow>,
}

#[derive(Serialize)]
pub struct LaunchControlsView {
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

#[derive(Deserialize)]
pub struct UpdateLaunchControlsRequest {
    pub maintenance_mode: Option<bool>,
    pub allow_public_signup: Option<bool>,
    pub allow_anonymous_signup: Option<bool>,
    pub allow_connect: Option<bool>,
    pub allow_paid_checkout: Option<bool>,
    pub healthy_only_routing: Option<bool>,
    pub expose_paid_tiers: Option<bool>,
    pub free_beta_label: Option<String>,
}

#[derive(Serialize)]
pub struct InviteCodeView {
    pub id: Uuid,
    pub code: String,
    pub tier: String,
    pub plan: String,
    pub duration_days: i32,
    pub max_uses: i32,
    pub used_count: i32,
    pub active: bool,
    pub cohort: Option<String>,
    pub notes: Option<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateInviteCodeRequest {
    pub code: Option<String>,
    pub tier: Option<String>,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub max_uses: Option<i32>,
    pub cohort: Option<String>,
    pub notes: Option<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Serialize)]
pub struct SessionLedgerRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub server_id: Uuid,
    pub tier: String,
    pub connect_country: Option<String>,
    pub started_at: chrono::DateTime<Utc>,
    pub ended_at: Option<chrono::DateTime<Utc>>,
    pub disconnect_reason: Option<String>,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub session_metadata: serde_json::Value,
}

#[derive(Serialize)]
pub struct JourneyEventRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub server_id: Option<Uuid>,
    pub event_type: String,
    pub outcome: String,
    pub detail: Option<String>,
    pub event_metadata: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
}

pub async fn get_overview(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<OpsOverview>> {
    let overview = sqlx::query_as::<_, (i64, i64, i64, i64, i64, f64, i64, i64, i64)>(
        r#"
        WITH latest_metrics AS (
            SELECT DISTINCT ON (nm.server_id)
                nm.server_id,
                nm.active_sessions,
                nm.assigned_users
            FROM node_metrics nm
            ORDER BY nm.server_id, nm.collected_at DESC
        )
        SELECT
            COUNT(*)::BIGINT AS total_nodes,
            COUNT(*) FILTER (WHERE s.lifecycle_state = 'healthy')::BIGINT AS healthy_nodes,
            COUNT(*) FILTER (WHERE s.lifecycle_state = 'warm')::BIGINT AS warm_nodes,
            COUNT(*) FILTER (WHERE s.lifecycle_state = 'degraded')::BIGINT AS degraded_nodes,
            COUNT(*) FILTER (WHERE s.lifecycle_state = 'blocked')::BIGINT AS blocked_nodes,
            COALESCE(AVG(s.health_score), 0)::DOUBLE PRECISION AS avg_health_score,
            COALESCE(SUM(s.assigned_user_cap), 0)::BIGINT AS total_assigned_user_cap,
            COALESCE(SUM(lm.active_sessions), 0)::BIGINT AS total_active_sessions,
            COALESCE(SUM(lm.assigned_users), 0)::BIGINT AS total_assigned_users
        FROM servers s
        LEFT JOIN latest_metrics lm ON lm.server_id = s.id
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let (
        total_nodes,
        healthy_nodes,
        warm_nodes,
        degraded_nodes,
        blocked_nodes,
        avg_health_score,
        total_assigned_user_cap,
        total_active_sessions,
        total_assigned_users,
    ) = overview;

    Ok(Json(OpsOverview {
        total_nodes,
        healthy_nodes,
        warm_nodes,
        degraded_nodes,
        blocked_nodes,
        avg_health_score,
        total_assigned_user_cap,
        total_active_sessions,
        total_assigned_users,
    }))
}

pub async fn list_nodes(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<NodeOpsInfo>>> {
    let rows = sqlx::query(
        r#"
        WITH latest_metrics AS (
            SELECT DISTINCT ON (nm.server_id)
                nm.server_id,
                nm.active_sessions,
                nm.cpu_pct,
                nm.ram_pct,
                nm.nic_in_mbps,
                nm.nic_out_mbps,
                nm.connect_success_pct,
                nm.median_connect_ms
            FROM node_metrics nm
            ORDER BY nm.server_id, nm.collected_at DESC
        )
        SELECT
            s.id,
            s.name,
            s.location,
            s.public_ip,
            ps.node_class,
            ps.provider,
            s.lifecycle_state,
            s.health_score,
            s.routing_weight,
            s.assigned_user_cap,
            s.active_session_soft_cap,
            s.active_session_hard_cap,
            COUNT(d.id)::BIGINT AS assigned_users,
            lm.active_sessions,
            lm.cpu_pct,
            lm.ram_pct,
            lm.nic_in_mbps,
            lm.nic_out_mbps,
            lm.connect_success_pct,
            lm.median_connect_ms
        FROM servers s
        LEFT JOIN provider_servers ps ON ps.server_id = s.id
        LEFT JOIN devices d ON d.server_id = s.id AND d.is_active = true
        LEFT JOIN latest_metrics lm ON lm.server_id = s.id
        GROUP BY
            s.id,
            s.name,
            s.location,
            s.public_ip,
            ps.node_class,
            ps.provider,
            s.lifecycle_state,
            s.health_score,
            s.routing_weight,
            s.assigned_user_cap,
            s.active_session_soft_cap,
            s.active_session_hard_cap,
            lm.active_sessions,
            lm.cpu_pct,
            lm.ram_pct,
            lm.nic_in_mbps,
            lm.nic_out_mbps,
            lm.connect_success_pct,
            lm.median_connect_ms
        ORDER BY s.health_score DESC, assigned_users ASC, s.created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let nodes = rows
        .into_iter()
        .map(|row| NodeOpsInfo {
            id: row.get("id"),
            name: row.get("name"),
            location: row.get("location"),
            public_ip: row.get("public_ip"),
            service_class: classify_service_class(
                row.get::<Option<String>, _>("node_class").as_deref(),
            ),
            provider: row.get("provider"),
            lifecycle_state: row.get("lifecycle_state"),
            health_score: row.get("health_score"),
            routing_weight: row.get("routing_weight"),
            assigned_user_cap: row.get("assigned_user_cap"),
            active_session_soft_cap: row.get("active_session_soft_cap"),
            active_session_hard_cap: row.get("active_session_hard_cap"),
            assigned_users: row.get("assigned_users"),
            active_sessions: row
                .get::<Option<i32>, _>("active_sessions")
                .unwrap_or_default(),
            cpu_pct: row.get::<Option<f64>, _>("cpu_pct").unwrap_or_default(),
            ram_pct: row.get::<Option<f64>, _>("ram_pct").unwrap_or_default(),
            nic_in_mbps: row.get::<Option<f64>, _>("nic_in_mbps").unwrap_or_default(),
            nic_out_mbps: row
                .get::<Option<f64>, _>("nic_out_mbps")
                .unwrap_or_default(),
            connect_success_pct: row
                .get::<Option<f64>, _>("connect_success_pct")
                .unwrap_or(100.0),
            median_connect_ms: row
                .get::<Option<i32>, _>("median_connect_ms")
                .unwrap_or_default(),
        })
        .collect();

    Ok(Json(nodes))
}

pub async fn get_snapshot(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<OpsSnapshot>> {
    let summary = sqlx::query(
        r#"
        SELECT
            (SELECT COUNT(*)::BIGINT FROM users) AS total_users,
            (SELECT COUNT(*)::BIGINT FROM users WHERE is_active = true) AS active_users,
            (SELECT COUNT(*)::BIGINT FROM users WHERE subscription_plan <> 'free') AS paid_users,
            (SELECT COUNT(*)::BIGINT FROM devices) AS total_devices,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE is_active = true) AS active_devices,
            (SELECT COALESCE(SUM(current_active_sessions), 0)::BIGINT FROM devices WHERE is_active = true) AS active_device_sessions,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE dedicated_required = true) AS dedicated_devices,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE sensitive_route = true) AS sensitive_route_devices,
            (SELECT COUNT(*)::BIGINT FROM users WHERE abuse_score >= 20) AS flagged_users,
            (SELECT COUNT(*)::BIGINT FROM users WHERE abuse_score >= 50) AS high_risk_users,
            (SELECT COALESCE(AVG(abuse_score), 0)::DOUBLE PRECISION FROM users) AS avg_abuse_score,
            (SELECT COALESCE(SUM(rx_bytes), 0)::BIGINT FROM usage_logs) AS total_rx_bytes,
            (SELECT COALESCE(SUM(tx_bytes), 0)::BIGINT FROM usage_logs) AS total_tx_bytes,
            (SELECT COUNT(*)::BIGINT FROM users WHERE signup_country IS NOT NULL AND signup_country <> '') AS signup_country_users,
            (SELECT COUNT(*)::BIGINT FROM users WHERE latest_login_country IS NOT NULL AND latest_login_country <> '') AS latest_login_country_users,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE platform IS NOT NULL AND platform <> '') AS device_platforms,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE device_install_id IS NOT NULL AND device_install_id <> '') AS device_install_ids,
            (SELECT COUNT(*)::BIGINT FROM users WHERE abuse_score > 0) AS abuse_flagged_users
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let plan_mix = query_labeled_counts(
        &state,
        r#"
        SELECT subscription_plan AS label, COUNT(*)::BIGINT AS count
        FROM users
        GROUP BY subscription_plan
        ORDER BY count DESC, label ASC
        "#,
    )
    .await?;

    let user_country_mix = query_labeled_counts(
        &state,
        r#"
        SELECT COALESCE(NULLIF(latest_login_country, ''), NULLIF(signup_country, ''), 'Unknown') AS label,
               COUNT(*)::BIGINT AS count
        FROM users
        GROUP BY 1
        ORDER BY count DESC, label ASC
        LIMIT 8
        "#,
    )
    .await?;

    let node_country_mix = query_labeled_counts(
        &state,
        r#"
        SELECT COALESCE(NULLIF(country_code, ''), location, 'Unknown') AS label,
               COUNT(*)::BIGINT AS count
        FROM servers
        GROUP BY 1
        ORDER BY count DESC, label ASC
        LIMIT 8
        "#,
    )
    .await?;

    let device_platform_mix = query_labeled_counts(
        &state,
        r#"
        SELECT COALESCE(NULLIF(platform, ''), 'Unknown') AS label,
               COUNT(*)::BIGINT AS count
        FROM devices
        GROUP BY 1
        ORDER BY count DESC, label ASC
        LIMIT 8
        "#,
    )
    .await?;

    let usage_bucket_mix = query_labeled_counts(
        &state,
        r#"
        SELECT COALESCE(NULLIF(usage_bucket, ''), 'Unknown') AS label,
               COUNT(*)::BIGINT AS count
        FROM devices
        GROUP BY 1
        ORDER BY count DESC, label ASC
        LIMIT 8
        "#,
    )
    .await?;

    let preferred_class_mix = query_labeled_counts(
        &state,
        r#"
        SELECT COALESCE(NULLIF(preferred_class, ''), 'Unspecified') AS label,
               COUNT(*)::BIGINT AS count
        FROM devices
        GROUP BY 1
        ORDER BY count DESC, label ASC
        LIMIT 8
        "#,
    )
    .await?;

    let abuse_bucket_mix = query_labeled_counts(
        &state,
        r#"
        SELECT
            CASE
                WHEN abuse_score >= 50 THEN '50+ high risk'
                WHEN abuse_score >= 20 THEN '20-49 flagged'
                WHEN abuse_score > 0 THEN '1-19 watch'
                ELSE '0 clean'
            END AS label,
            COUNT(*)::BIGINT AS count
        FROM users
        GROUP BY 1
        ORDER BY count DESC, label ASC
        "#,
    )
    .await?;

    Ok(Json(OpsSnapshot {
        total_users: summary.get("total_users"),
        active_users: summary.get("active_users"),
        paid_users: summary.get("paid_users"),
        total_devices: summary.get("total_devices"),
        active_devices: summary.get("active_devices"),
        active_device_sessions: summary.get("active_device_sessions"),
        dedicated_devices: summary.get("dedicated_devices"),
        sensitive_route_devices: summary.get("sensitive_route_devices"),
        flagged_users: summary.get("flagged_users"),
        high_risk_users: summary.get("high_risk_users"),
        avg_abuse_score: summary.get("avg_abuse_score"),
        total_rx_bytes: summary.get("total_rx_bytes"),
        total_tx_bytes: summary.get("total_tx_bytes"),
        plan_mix,
        user_country_mix,
        node_country_mix,
        device_platform_mix,
        usage_bucket_mix,
        preferred_class_mix,
        abuse_bucket_mix,
        telemetry_coverage: TelemetryCoverage {
            signup_country_users: summary.get("signup_country_users"),
            latest_login_country_users: summary.get("latest_login_country_users"),
            device_platforms: summary.get("device_platforms"),
            device_install_ids: summary.get("device_install_ids"),
            abuse_flagged_users: summary.get("abuse_flagged_users"),
        },
    }))
}

pub async fn get_alerts(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<OpsAlert>>> {
    let telemetry = sqlx::query(
        r#"
        SELECT
            (SELECT COUNT(*)::BIGINT FROM users) AS total_users,
            (SELECT COUNT(*)::BIGINT FROM devices) AS total_devices,
            (SELECT COUNT(*)::BIGINT FROM users WHERE latest_login_country IS NOT NULL AND latest_login_country <> '') AS login_country_users,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE platform IS NOT NULL AND platform <> '') AS device_platforms,
            (SELECT COUNT(*)::BIGINT FROM devices WHERE device_install_id IS NOT NULL AND device_install_id <> '') AS device_install_ids,
            (SELECT COUNT(*)::BIGINT FROM users WHERE abuse_score >= 20) AS flagged_users,
            (SELECT COUNT(*)::BIGINT FROM users WHERE abuse_score >= 50) AS high_risk_users
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let fleet = sqlx::query(
        r#"
        WITH latest_metrics AS (
            SELECT DISTINCT ON (nm.server_id)
                nm.server_id,
                nm.cpu_pct,
                nm.ram_pct,
                nm.connect_success_pct,
                nm.collected_at
            FROM node_metrics nm
            ORDER BY nm.server_id, nm.collected_at DESC
        )
        SELECT
            COUNT(*) FILTER (WHERE s.lifecycle_state IN ('degraded', 'blocked'))::BIGINT AS unhealthy_nodes,
            COUNT(*) FILTER (WHERE s.lifecycle_state = 'warm')::BIGINT AS warm_nodes,
            COUNT(*) FILTER (
                WHERE lm.server_id IS NULL OR lm.collected_at < NOW() - INTERVAL '15 minutes'
            )::BIGINT AS stale_metric_nodes,
            COUNT(*) FILTER (
                WHERE COALESCE(lm.cpu_pct, 0) >= 85
                   OR COALESCE(lm.ram_pct, 0) >= 85
                   OR COALESCE(lm.connect_success_pct, 100) < 98
            )::BIGINT AS stressed_nodes
        FROM servers s
        LEFT JOIN latest_metrics lm ON lm.server_id = s.id
        WHERE s.is_active = true
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let total_users: i64 = telemetry.get("total_users");
    let total_devices: i64 = telemetry.get("total_devices");
    let login_country_users: i64 = telemetry.get("login_country_users");
    let device_platforms: i64 = telemetry.get("device_platforms");
    let device_install_ids: i64 = telemetry.get("device_install_ids");
    let flagged_users: i64 = telemetry.get("flagged_users");
    let high_risk_users: i64 = telemetry.get("high_risk_users");
    let unhealthy_nodes: i64 = fleet.get("unhealthy_nodes");
    let warm_nodes: i64 = fleet.get("warm_nodes");
    let stale_metric_nodes: i64 = fleet.get("stale_metric_nodes");
    let stressed_nodes: i64 = fleet.get("stressed_nodes");

    let mut alerts = Vec::new();

    if stale_metric_nodes > 0 {
        alerts.push(OpsAlert {
            severity: "critical".into(),
            category: "telemetry".into(),
            title: "Nodes missing fresh metrics".into(),
            detail: "Some servers have no node_metrics sample in the last 15 minutes.".into(),
            count: stale_metric_nodes,
        });
    }

    if unhealthy_nodes > 0 {
        alerts.push(OpsAlert {
            severity: "critical".into(),
            category: "fleet".into(),
            title: "Nodes are degraded or blocked".into(),
            detail: "Routing should avoid these nodes until health recovers.".into(),
            count: unhealthy_nodes,
        });
    }

    if stressed_nodes > 0 {
        alerts.push(OpsAlert {
            severity: "warning".into(),
            category: "fleet".into(),
            title: "Nodes under load stress".into(),
            detail: "Latest metrics show high CPU, high RAM, or low connect success.".into(),
            count: stressed_nodes,
        });
    }

    if warm_nodes > 0 {
        alerts.push(OpsAlert {
            severity: "warning".into(),
            category: "fleet".into(),
            title: "Warm nodes need watching".into(),
            detail: "These nodes are still routable but have reduced health headroom.".into(),
            count: warm_nodes,
        });
    }

    if total_users > 0 && login_country_users * 100 < total_users * 80 {
        alerts.push(OpsAlert {
            severity: "warning".into(),
            category: "telemetry".into(),
            title: "User country telemetry is incomplete".into(),
            detail: "Latest login country coverage is below the 80% target.".into(),
            count: total_users - login_country_users,
        });
    }

    if total_devices > 0 && device_platforms * 100 < total_devices * 80 {
        alerts.push(OpsAlert {
            severity: "warning".into(),
            category: "telemetry".into(),
            title: "Device platform telemetry is incomplete".into(),
            detail: "Platform coverage is below the 80% target.".into(),
            count: total_devices - device_platforms,
        });
    }

    if total_devices > 0 && device_install_ids * 100 < total_devices * 80 {
        alerts.push(OpsAlert {
            severity: "warning".into(),
            category: "telemetry".into(),
            title: "Device install IDs are missing".into(),
            detail: "Install IDs are needed for abuse tracing and concurrency analysis.".into(),
            count: total_devices - device_install_ids,
        });
    }

    if high_risk_users > 0 {
        alerts.push(OpsAlert {
            severity: "critical".into(),
            category: "abuse".into(),
            title: "High-risk users detected".into(),
            detail: "Abuse scores at or above 50 need manual review.".into(),
            count: high_risk_users,
        });
    } else if flagged_users > 0 {
        alerts.push(OpsAlert {
            severity: "warning".into(),
            category: "abuse".into(),
            title: "Flagged users detected".into(),
            detail: "Abuse scores at or above 20 are being tracked.".into(),
            count: flagged_users,
        });
    }

    if alerts.is_empty() {
        alerts.push(OpsAlert {
            severity: "info".into(),
            category: "ops".into(),
            title: "No active launch alerts".into(),
            detail: "Fleet health, abuse thresholds, and telemetry checks are currently within defined thresholds.".into(),
            count: 0,
        });
    }

    Ok(Json(alerts))
}

async fn load_launch_controls(state: &AdminState) -> Result<LaunchControlsView, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT maintenance_mode, allow_public_signup, allow_anonymous_signup, allow_connect,
               allow_paid_checkout, healthy_only_routing, expose_paid_tiers, free_beta_label, updated_at
        FROM launch_controls
        WHERE singleton = TRUE
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(LaunchControlsView {
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

pub async fn get_launch_controls(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<LaunchControlsView>> {
    Ok(Json(load_launch_controls(&state).await?))
}

pub async fn get_parental_overview(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<ParentalOpsOverview>> {
    let summary = sqlx::query(
        r#"
        SELECT
            (SELECT COUNT(*)::BIGINT FROM parental_children) AS total_children,
            (SELECT COUNT(*)::BIGINT FROM parental_children WHERE child_user_id IS NOT NULL) AS linked_children,
            (SELECT COUNT(*)::BIGINT FROM parental_child_devices WHERE is_active = TRUE) AS active_child_devices,
            (SELECT COUNT(*)::BIGINT FROM parental_policies WHERE is_active = TRUE) AS active_policies,
            (SELECT COUNT(*)::BIGINT FROM parental_schedules WHERE is_active = TRUE) AS active_schedules,
            (SELECT COUNT(*)::BIGINT FROM parental_events WHERE occurred_at >= NOW() - INTERVAL '24 hours') AS recent_events
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let latest_rows = sqlx::query(
        r#"
        SELECT pc.id AS child_id, pc.name AS child_name, pe.event_type, pe.app_identifier, pe.action, pe.detail, pe.occurred_at
        FROM parental_events pe
        JOIN parental_children pc ON pc.id = pe.child_id
        ORDER BY pe.occurred_at DESC
        LIMIT 25
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ParentalOpsOverview {
        total_children: summary.get("total_children"),
        linked_children: summary.get("linked_children"),
        active_child_devices: summary.get("active_child_devices"),
        active_policies: summary.get("active_policies"),
        active_schedules: summary.get("active_schedules"),
        recent_events: summary.get("recent_events"),
        latest_events: latest_rows
            .into_iter()
            .map(|row| ParentalEventOpsRow {
                child_id: row.get("child_id"),
                child_name: row.get("child_name"),
                event_type: row.get("event_type"),
                app_identifier: row.get("app_identifier"),
                action: row.get("action"),
                detail: row.get("detail"),
                occurred_at: row.get("occurred_at"),
            })
            .collect(),
    }))
}

pub async fn update_launch_controls(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Json(body): Json<UpdateLaunchControlsRequest>,
) -> escudo_common::Result<Json<LaunchControlsView>> {
    let free_beta_label = body
        .free_beta_label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    sqlx::query(
        r#"
        UPDATE launch_controls
        SET maintenance_mode = COALESCE($1, maintenance_mode),
            allow_public_signup = COALESCE($2, allow_public_signup),
            allow_anonymous_signup = COALESCE($3, allow_anonymous_signup),
            allow_connect = COALESCE($4, allow_connect),
            allow_paid_checkout = COALESCE($5, allow_paid_checkout),
            healthy_only_routing = COALESCE($6, healthy_only_routing),
            expose_paid_tiers = COALESCE($7, expose_paid_tiers),
            free_beta_label = COALESCE($8, free_beta_label),
            updated_at = NOW()
        WHERE singleton = TRUE
        "#,
    )
    .bind(body.maintenance_mode)
    .bind(body.allow_public_signup)
    .bind(body.allow_anonymous_signup)
    .bind(body.allow_connect)
    .bind(body.allow_paid_checkout)
    .bind(body.healthy_only_routing)
    .bind(body.expose_paid_tiers)
    .bind(free_beta_label)
    .execute(&state.db)
    .await?;

    Ok(Json(load_launch_controls(&state).await?))
}

pub async fn list_invite_codes(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<InviteCodeView>>> {
    let rows = sqlx::query(
        r#"
        SELECT id, code, tier, plan, duration_days, max_uses, used_count, active, cohort, notes, expires_at, created_at
        FROM invite_codes
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows
        .into_iter()
        .map(|row| InviteCodeView {
            id: row.get("id"),
            code: row.get("code"),
            tier: row.get("tier"),
            plan: row.get("plan"),
            duration_days: row.get("duration_days"),
            max_uses: row.get("max_uses"),
            used_count: row.get("used_count"),
            active: row.get("active"),
            cohort: row.get("cohort"),
            notes: row.get("notes"),
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
        })
        .collect()))
}

pub async fn create_invite_code(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Json(body): Json<CreateInviteCodeRequest>,
) -> escudo_common::Result<Json<InviteCodeView>> {
    let generated = Uuid::new_v4().simple().to_string();
    let code = body
        .code
        .unwrap_or_else(|| generated[..12].to_string())
        .trim()
        .to_uppercase();
    if code.is_empty() {
        return Err(escudo_common::EscudoError::BadRequest(
            "Invite code cannot be empty".into(),
        ));
    }

    let tier = body
        .tier
        .unwrap_or_else(|| "pro".to_string())
        .trim()
        .to_ascii_lowercase();
    let plan = body
        .plan
        .unwrap_or_else(|| tier.clone())
        .trim()
        .to_ascii_lowercase();
    let duration_days = body.duration_days.unwrap_or(30).max(1);
    let max_uses = body.max_uses.unwrap_or(1).max(1);
    let expires_at = body
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days.max(1)));

    let row = sqlx::query(
        r#"
        INSERT INTO invite_codes (code, tier, plan, duration_days, max_uses, cohort, notes, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, code, tier, plan, duration_days, max_uses, used_count, active, cohort, notes, expires_at, created_at
        "#,
    )
    .bind(code)
    .bind(tier)
    .bind(plan)
    .bind(duration_days)
    .bind(max_uses)
    .bind(body.cohort)
    .bind(body.notes)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(InviteCodeView {
        id: row.get("id"),
        code: row.get("code"),
        tier: row.get("tier"),
        plan: row.get("plan"),
        duration_days: row.get("duration_days"),
        max_uses: row.get("max_uses"),
        used_count: row.get("used_count"),
        active: row.get("active"),
        cohort: row.get("cohort"),
        notes: row.get("notes"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
    }))
}

pub async fn list_sessions(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<SessionLedgerRow>>> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, device_id, server_id, tier, connect_country, started_at, ended_at, disconnect_reason, bytes_in, bytes_out, session_metadata
        FROM vpn_sessions
        ORDER BY started_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows
        .into_iter()
        .map(|row| SessionLedgerRow {
            id: row.get("id"),
            user_id: row.get("user_id"),
            device_id: row.get("device_id"),
            server_id: row.get("server_id"),
            tier: row.get("tier"),
            connect_country: row.get("connect_country"),
            started_at: row.get("started_at"),
            ended_at: row.get("ended_at"),
            disconnect_reason: row.get("disconnect_reason"),
            bytes_in: row.get("bytes_in"),
            bytes_out: row.get("bytes_out"),
            session_metadata: row.get("session_metadata"),
        })
        .collect()))
}

pub async fn list_journey_events(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> escudo_common::Result<Json<Vec<JourneyEventRow>>> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, device_id, server_id, event_type, outcome, detail, event_metadata, created_at
        FROM journey_events
        ORDER BY created_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows
        .into_iter()
        .map(|row| JourneyEventRow {
            id: row.get("id"),
            user_id: row.get("user_id"),
            device_id: row.get("device_id"),
            server_id: row.get("server_id"),
            event_type: row.get("event_type"),
            outcome: row.get("outcome"),
            detail: row.get("detail"),
            event_metadata: row.get("event_metadata"),
            created_at: row.get("created_at"),
        })
        .collect()))
}

async fn query_labeled_counts(
    state: &AdminState,
    sql: &str,
) -> Result<Vec<LabeledCount>, sqlx::Error> {
    let rows = sqlx::query(sql).fetch_all(&state.db).await?;
    Ok(rows
        .into_iter()
        .map(|row| LabeledCount {
            label: row.get("label"),
            count: row.get("count"),
        })
        .collect())
}

fn classify_service_class(node_class: Option<&str>) -> String {
    let node_class = node_class.unwrap_or_default().to_ascii_lowercase();
    if node_class.contains("pro")
        || node_class.contains("dedicated")
        || node_class.contains("power")
    {
        return "Power".to_string();
    }

    if node_class.contains("shared")
        || node_class.contains("medium")
        || node_class.contains("cpx")
        || node_class.contains("cx33")
    {
        return "Medium".to_string();
    }

    "Free".to_string()
}
