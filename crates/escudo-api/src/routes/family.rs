use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Utc;
use escudo_common::EscudoError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use tracing::error;
use uuid::Uuid;

use crate::backend_control;
use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct FamilyProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub block_porn: bool,
    pub block_gambling: bool,
    pub block_social_media: bool,
    pub block_malware: bool,
    pub block_drugs: bool,
    pub block_violence: bool,
    pub block_dating: bool,
    pub block_gaming: bool,
    pub custom_blocked_domains: Vec<String>,
    pub custom_allowed_domains: Vec<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub block_porn: Option<bool>,
    pub block_gambling: Option<bool>,
    pub block_social_media: Option<bool>,
    pub block_malware: Option<bool>,
    pub block_drugs: Option<bool>,
    pub block_violence: Option<bool>,
    pub block_dating: Option<bool>,
    pub block_gaming: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct DomainRequest {
    pub domain: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct ChildDevice {
    pub id: Uuid,
    pub device_id: Option<Uuid>,
    pub device_install_id: Option<String>,
    pub display_name: String,
    pub platform: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub linked_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Serialize)]
pub struct ParentalPolicy {
    pub id: Uuid,
    pub child_id: Uuid,
    pub target_device_id: Option<Uuid>,
    pub block_tiktok: bool,
    pub block_youtube: bool,
    pub block_social_media: bool,
    pub block_streaming: bool,
    pub bedtime_enabled: bool,
    pub bedtime_start_minute: Option<i32>,
    pub bedtime_end_minute: Option<i32>,
    pub max_daily_minutes: Option<i32>,
    pub monitored_apps: Vec<String>,
    pub blocked_apps: Vec<String>,
    pub custom_blocked_domains: Vec<String>,
    pub custom_allowed_domains: Vec<String>,
    pub is_active: bool,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Serialize)]
pub struct ParentalSchedule {
    pub id: Uuid,
    pub child_id: Uuid,
    pub name: String,
    pub days_of_week: Vec<i32>,
    pub start_minute: i32,
    pub end_minute: i32,
    pub blocked_categories: Vec<String>,
    pub blocked_apps: Vec<String>,
    pub is_active: bool,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Serialize)]
pub struct ParentalEvent {
    pub id: Uuid,
    pub child_id: Uuid,
    pub device_id: Option<Uuid>,
    pub event_type: String,
    pub app_identifier: Option<String>,
    pub domain: Option<String>,
    pub action: Option<String>,
    pub detail: Option<String>,
    pub event_metadata: serde_json::Value,
    pub occurred_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Serialize)]
pub struct ParentalChild {
    pub id: Uuid,
    pub parent_user_id: Uuid,
    pub child_user_id: Option<Uuid>,
    pub name: String,
    pub access_code: String,
    pub tier: String,
    pub is_active: bool,
    pub linked_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub devices: Vec<ChildDevice>,
    pub policies: Vec<ParentalPolicy>,
    pub schedules: Vec<ParentalSchedule>,
}

#[derive(Serialize)]
pub struct ParentalOverview {
    pub total_children: i64,
    pub linked_children: i64,
    pub active_child_devices: i64,
    pub active_policies: i64,
    pub active_schedules: i64,
    pub recent_events: i64,
    pub children: Vec<ParentalChild>,
}

#[derive(Deserialize)]
pub struct CreateChildRequest {
    pub name: String,
    pub tier: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateChildRequest {
    pub name: Option<String>,
    pub tier: Option<String>,
    pub is_active: Option<bool>,
    pub regenerate_code: Option<bool>,
}

#[derive(Deserialize)]
pub struct LinkDeviceRequest {
    pub device_id: Option<Uuid>,
    pub device_install_id: Option<String>,
    pub display_name: Option<String>,
    pub platform: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct UpsertPolicyRequest {
    pub target_device_id: Option<Uuid>,
    pub block_tiktok: Option<bool>,
    pub block_youtube: Option<bool>,
    pub block_social_media: Option<bool>,
    pub block_streaming: Option<bool>,
    pub bedtime_enabled: Option<bool>,
    pub bedtime_start_minute: Option<i32>,
    pub bedtime_end_minute: Option<i32>,
    pub max_daily_minutes: Option<i32>,
    pub monitored_apps: Option<Vec<String>>,
    pub blocked_apps: Option<Vec<String>>,
    pub custom_blocked_domains: Option<Vec<String>>,
    pub custom_allowed_domains: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateScheduleRequest {
    pub name: String,
    pub days_of_week: Option<Vec<i32>>,
    pub start_minute: i32,
    pub end_minute: i32,
    pub blocked_categories: Option<Vec<String>>,
    pub blocked_apps: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct ClaimChildCodeRequest {
    pub access_code: String,
}

#[derive(Deserialize)]
pub struct RecordParentalEventRequest {
    pub child_id: Uuid,
    pub device_id: Option<Uuid>,
    pub event_type: String,
    pub app_identifier: Option<String>,
    pub domain: Option<String>,
    pub action: Option<String>,
    pub detail: Option<String>,
    pub event_metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct DevicePolicyQuery {
    pub device_install_id: Option<String>,
}

#[derive(Serialize)]
pub struct DevicePolicyResponse {
    pub child: Option<ParentalChild>,
    pub device_install_id: Option<String>,
    pub device_linked: bool,
    pub effective_policies: Vec<ParentalPolicy>,
    pub effective_schedules: Vec<ParentalSchedule>,
    pub recent_events: Vec<ParentalEvent>,
}

fn normalize_string_list(list: Option<Vec<String>>, max_items: usize, max_len: usize) -> Vec<String> {
    list.unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty() && value.len() <= max_len)
        .take(max_items)
        .collect()
}

fn validate_minutes(value: Option<i32>, field: &str) -> Result<Option<i32>, EscudoError> {
    match value {
        Some(v) if !(0..=1440).contains(&v) => Err(EscudoError::BadRequest(format!("{field} must be between 0 and 1440"))),
        other => Ok(other),
    }
}

fn validate_days_of_week(days: Option<Vec<i32>>) -> Result<Vec<i32>, EscudoError> {
    let days = days.unwrap_or_else(|| vec![1, 2, 3, 4, 5, 6, 7]);
    if days.is_empty() || days.iter().any(|day| !(*day >= 1 && *day <= 7)) {
        return Err(EscudoError::BadRequest("days_of_week must only contain values 1..7".into()));
    }
    Ok(days)
}

fn generate_numeric_code() -> String {
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| char::from(b'0' + rng.gen_range(0..10)))
        .collect()
}

async fn ensure_family_profile(state: &AppState, user_id: Uuid) -> Result<(), EscudoError> {
    sqlx::query(
        "INSERT INTO family_profiles (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok(())
}

fn family_profile_from_row(row: sqlx::postgres::PgRow) -> FamilyProfile {
    FamilyProfile {
        id: row.get("id"),
        user_id: row.get("user_id"),
        name: row.get("name"),
        block_porn: row.get("block_porn"),
        block_gambling: row.get("block_gambling"),
        block_social_media: row.get("block_social_media"),
        block_malware: row.get("block_malware"),
        block_drugs: row.get("block_drugs"),
        block_violence: row.get("block_violence"),
        block_dating: row.get("block_dating"),
        block_gaming: row.get("block_gaming"),
        custom_blocked_domains: row
            .get::<Option<Vec<String>>, _>("custom_blocked_domains")
            .unwrap_or_default(),
        custom_allowed_domains: row
            .get::<Option<Vec<String>>, _>("custom_allowed_domains")
            .unwrap_or_default(),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

async fn ensure_parent_owns_child(state: &AppState, parent_user_id: Uuid, child_id: Uuid) -> Result<(), EscudoError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM parental_children WHERE id = $1 AND parent_user_id = $2)",
    )
    .bind(child_id)
    .bind(parent_user_id)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(EscudoError::NotFound("Child profile not found".into()));
    }
    Ok(())
}

async fn fetch_child_devices(state: &AppState, child_id: Uuid) -> Result<Vec<ChildDevice>, EscudoError> {
    let rows = sqlx::query(
        r#"
        SELECT id, device_id, device_install_id, display_name, platform, notes, is_active, linked_at
        FROM parental_child_devices
        WHERE child_id = $1
        ORDER BY linked_at DESC
        "#,
    )
    .bind(child_id)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ChildDevice {
            id: row.get("id"),
            device_id: row.get("device_id"),
            device_install_id: row.get("device_install_id"),
            display_name: row.get("display_name"),
            platform: row.get("platform"),
            notes: row.get("notes"),
            is_active: row.get("is_active"),
            linked_at: row.get("linked_at"),
        })
        .collect())
}

async fn fetch_child_policies(state: &AppState, child_id: Uuid) -> Result<Vec<ParentalPolicy>, EscudoError> {
    let rows = sqlx::query(
        r#"
        SELECT id, child_id, target_device_id, block_tiktok, block_youtube, block_social_media,
               block_streaming, bedtime_enabled, bedtime_start_minute, bedtime_end_minute,
               max_daily_minutes, monitored_apps, blocked_apps, custom_blocked_domains,
               custom_allowed_domains, is_active, updated_at
        FROM parental_policies
        WHERE child_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(child_id)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ParentalPolicy {
            id: row.get("id"),
            child_id: row.get("child_id"),
            target_device_id: row.get("target_device_id"),
            block_tiktok: row.get("block_tiktok"),
            block_youtube: row.get("block_youtube"),
            block_social_media: row.get("block_social_media"),
            block_streaming: row.get("block_streaming"),
            bedtime_enabled: row.get("bedtime_enabled"),
            bedtime_start_minute: row.get("bedtime_start_minute"),
            bedtime_end_minute: row.get("bedtime_end_minute"),
            max_daily_minutes: row.get("max_daily_minutes"),
            monitored_apps: row
                .get::<Option<Vec<String>>, _>("monitored_apps")
                .unwrap_or_default(),
            blocked_apps: row
                .get::<Option<Vec<String>>, _>("blocked_apps")
                .unwrap_or_default(),
            custom_blocked_domains: row
                .get::<Option<Vec<String>>, _>("custom_blocked_domains")
                .unwrap_or_default(),
            custom_allowed_domains: row
                .get::<Option<Vec<String>>, _>("custom_allowed_domains")
                .unwrap_or_default(),
            is_active: row.get("is_active"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn fetch_child_schedules(state: &AppState, child_id: Uuid) -> Result<Vec<ParentalSchedule>, EscudoError> {
    let rows = sqlx::query(
        r#"
        SELECT id, child_id, name, days_of_week, start_minute, end_minute, blocked_categories, blocked_apps, is_active, updated_at
        FROM parental_schedules
        WHERE child_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(child_id)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ParentalSchedule {
            id: row.get("id"),
            child_id: row.get("child_id"),
            name: row.get("name"),
            days_of_week: row.get::<Option<Vec<i32>>, _>("days_of_week").unwrap_or_default(),
            start_minute: row.get("start_minute"),
            end_minute: row.get("end_minute"),
            blocked_categories: row
                .get::<Option<Vec<String>>, _>("blocked_categories")
                .unwrap_or_default(),
            blocked_apps: row
                .get::<Option<Vec<String>>, _>("blocked_apps")
                .unwrap_or_default(),
            is_active: row.get("is_active"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn fetch_parental_child(state: &AppState, child_id: Uuid) -> Result<ParentalChild, EscudoError> {
    let row = sqlx::query(
        r#"
        SELECT id, parent_user_id, child_user_id, name, access_code, tier, is_active, linked_at, created_at
        FROM parental_children
        WHERE id = $1
        "#,
    )
    .bind(child_id)
    .fetch_one(&state.db)
    .await?;

    Ok(ParentalChild {
        id: row.get("id"),
        parent_user_id: row.get("parent_user_id"),
        child_user_id: row.get("child_user_id"),
        name: row.get("name"),
        access_code: row.get("access_code"),
        tier: row.get("tier"),
        is_active: row.get("is_active"),
        linked_at: row.get("linked_at"),
        created_at: row.get("created_at"),
        devices: fetch_child_devices(state, child_id).await?,
        policies: fetch_child_policies(state, child_id).await?,
        schedules: fetch_child_schedules(state, child_id).await?,
    })
}

async fn fetch_recent_parental_events(state: &AppState, child_id: Uuid, limit: i64) -> Result<Vec<ParentalEvent>, EscudoError> {
    let rows = sqlx::query(
        r#"
        SELECT id, child_id, device_id, event_type, app_identifier, domain, action, detail, event_metadata, occurred_at
        FROM parental_events
        WHERE child_id = $1
        ORDER BY occurred_at DESC
        LIMIT $2
        "#,
    )
    .bind(child_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ParentalEvent {
            id: row.get("id"),
            child_id: row.get("child_id"),
            device_id: row.get("device_id"),
            event_type: row.get("event_type"),
            app_identifier: row.get("app_identifier"),
            domain: row.get("domain"),
            action: row.get("action"),
            detail: row.get("detail"),
            event_metadata: row.get("event_metadata"),
            occurred_at: row.get("occurred_at"),
        })
        .collect())
}

pub async fn get_profile(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<FamilyProfile>> {
    let user_id = auth.0.sub;
    ensure_family_profile(&state, user_id).await?;

    let row = sqlx::query(
        r#"
        SELECT id, user_id, name, block_porn, block_gambling, block_social_media,
               block_malware, block_drugs, block_violence, block_dating, block_gaming,
               custom_blocked_domains, custom_allowed_domains, is_active, created_at, updated_at
        FROM family_profiles
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to fetch family profile for user {user_id}: {e}");
        EscudoError::Internal("Falha ao buscar perfil familiar".into())
    })?;

    Ok(Json(family_profile_from_row(row)))
}

pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> escudo_common::Result<Json<FamilyProfile>> {
    let user_id = auth.0.sub;
    ensure_family_profile(&state, user_id).await?;

    let row = sqlx::query(
        r#"
        UPDATE family_profiles SET
            name = COALESCE($2, name),
            block_porn = COALESCE($3, block_porn),
            block_gambling = COALESCE($4, block_gambling),
            block_social_media = COALESCE($5, block_social_media),
            block_malware = COALESCE($6, block_malware),
            block_drugs = COALESCE($7, block_drugs),
            block_violence = COALESCE($8, block_violence),
            block_dating = COALESCE($9, block_dating),
            block_gaming = COALESCE($10, block_gaming),
            is_active = COALESCE($11, is_active),
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING id, user_id, name, block_porn, block_gambling, block_social_media,
                  block_malware, block_drugs, block_violence, block_dating, block_gaming,
                  custom_blocked_domains, custom_allowed_domains, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(req.name)
    .bind(req.block_porn)
    .bind(req.block_gambling)
    .bind(req.block_social_media)
    .bind(req.block_malware)
    .bind(req.block_drugs)
    .bind(req.block_violence)
    .bind(req.block_dating)
    .bind(req.block_gaming)
    .bind(req.is_active)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to update family profile for user {user_id}: {e}");
        EscudoError::Internal("Falha ao atualizar perfil familiar".into())
    })?;

    Ok(Json(family_profile_from_row(row)))
}

pub async fn add_blocked_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DomainRequest>,
) -> escudo_common::Result<Json<MessageResponse>> {
    let user_id = auth.0.sub;
    let domain = req.domain.trim().to_lowercase();

    if domain.is_empty() {
        return Err(EscudoError::BadRequest("Dominio invalido".into()));
    }

    ensure_family_profile(&state, user_id).await?;

    sqlx::query(
        r#"
        UPDATE family_profiles
        SET custom_blocked_domains = array_append(array_remove(custom_blocked_domains, $2::TEXT), $2::TEXT),
            updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(&domain)
    .execute(&state.db)
    .await?;

    Ok(Json(MessageResponse {
        message: format!("Dominio '{domain}' adicionado a lista de bloqueio"),
    }))
}

pub async fn add_allowed_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DomainRequest>,
) -> escudo_common::Result<Json<MessageResponse>> {
    let user_id = auth.0.sub;
    let domain = req.domain.trim().to_lowercase();

    if domain.is_empty() {
        return Err(EscudoError::BadRequest("Dominio invalido".into()));
    }

    ensure_family_profile(&state, user_id).await?;

    sqlx::query(
        r#"
        UPDATE family_profiles
        SET custom_allowed_domains = array_append(array_remove(custom_allowed_domains, $2::TEXT), $2::TEXT),
            updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(&domain)
    .execute(&state.db)
    .await?;

    Ok(Json(MessageResponse {
        message: format!("Dominio '{domain}' adicionado a lista de permissao"),
    }))
}

pub async fn get_parental_overview(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<ParentalOverview>> {
    let parent_user_id = auth.0.sub;

    let summary = sqlx::query(
        r#"
        SELECT
            COUNT(*)::BIGINT AS total_children,
            COUNT(*) FILTER (WHERE child_user_id IS NOT NULL)::BIGINT AS linked_children,
            (
                SELECT COUNT(*)::BIGINT
                FROM parental_child_devices pcd
                JOIN parental_children pc ON pc.id = pcd.child_id
                WHERE pc.parent_user_id = $1 AND pcd.is_active = TRUE
            ) AS active_child_devices,
            (
                SELECT COUNT(*)::BIGINT
                FROM parental_policies pp
                JOIN parental_children pc ON pc.id = pp.child_id
                WHERE pc.parent_user_id = $1 AND pp.is_active = TRUE
            ) AS active_policies,
            (
                SELECT COUNT(*)::BIGINT
                FROM parental_schedules ps
                JOIN parental_children pc ON pc.id = ps.child_id
                WHERE pc.parent_user_id = $1 AND ps.is_active = TRUE
            ) AS active_schedules,
            (
                SELECT COUNT(*)::BIGINT
                FROM parental_events pe
                JOIN parental_children pc ON pc.id = pe.child_id
                WHERE pc.parent_user_id = $1 AND pe.occurred_at >= NOW() - INTERVAL '24 hours'
            ) AS recent_events
        FROM parental_children
        WHERE parent_user_id = $1
        "#,
    )
    .bind(parent_user_id)
    .fetch_one(&state.db)
    .await?;

    let child_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM parental_children WHERE parent_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(parent_user_id)
    .fetch_all(&state.db)
    .await?;

    let mut children = Vec::new();
    for child_id in child_ids {
        children.push(fetch_parental_child(&state, child_id).await?);
    }

    Ok(Json(ParentalOverview {
        total_children: summary.get("total_children"),
        linked_children: summary.get("linked_children"),
        active_child_devices: summary.get("active_child_devices"),
        active_policies: summary.get("active_policies"),
        active_schedules: summary.get("active_schedules"),
        recent_events: summary.get("recent_events"),
        children,
    }))
}

pub async fn list_parental_children(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<ParentalChild>>> {
    let child_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM parental_children WHERE parent_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.0.sub)
    .fetch_all(&state.db)
    .await?;

    let mut children = Vec::new();
    for child_id in child_ids {
        children.push(fetch_parental_child(&state, child_id).await?);
    }

    Ok(Json(children))
}

pub async fn create_parental_child(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateChildRequest>,
) -> escudo_common::Result<Json<ParentalChild>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(EscudoError::BadRequest("Child name must be between 1 and 80 characters".into()));
    }
    let tier = req.tier.unwrap_or_else(|| "family".to_string());

    let child_id = {
        let mut created = None;
        for _ in 0..5 {
            let code = generate_numeric_code();
            let row = sqlx::query(
                r#"
                INSERT INTO parental_children (parent_user_id, name, access_code, tier)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (access_code) DO NOTHING
                RETURNING id
                "#,
            )
            .bind(auth.0.sub)
            .bind(name)
            .bind(code)
            .bind(&tier)
            .fetch_optional(&state.db)
            .await?;

            if let Some(row) = row {
                created = Some(row.get("id"));
                break;
            }
        }

        created.ok_or_else(|| EscudoError::Internal("Failed to generate unique child access code".into()))?
    };

    let _ = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        None,
        None,
        "parental_child_create",
        "success",
        Some(format!("Created child profile {name}")),
        json!({ "child_id": child_id, "tier": tier }),
    )
    .await;

    Ok(Json(fetch_parental_child(&state, child_id).await?))
}

pub async fn update_parental_child(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(child_id): Path<Uuid>,
    Json(req): Json<UpdateChildRequest>,
) -> escudo_common::Result<Json<ParentalChild>> {
    ensure_parent_owns_child(&state, auth.0.sub, child_id).await?;

    let name = req.name.as_deref().map(str::trim).map(str::to_string);
    if let Some(ref n) = name {
        if n.is_empty() || n.len() > 80 {
            return Err(EscudoError::BadRequest("Child name must be between 1 and 80 characters".into()));
        }
    }

    let regenerated_code = if req.regenerate_code.unwrap_or(false) {
        Some(generate_numeric_code())
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE parental_children
        SET name = COALESCE($2, name),
            tier = COALESCE($3, tier),
            is_active = COALESCE($4, is_active),
            access_code = COALESCE($5, access_code),
            updated_at = NOW()
        WHERE id = $1 AND parent_user_id = $6
        "#,
    )
    .bind(child_id)
    .bind(name)
    .bind(req.tier)
    .bind(req.is_active)
    .bind(regenerated_code)
    .bind(auth.0.sub)
    .execute(&state.db)
    .await?;

    Ok(Json(fetch_parental_child(&state, child_id).await?))
}

pub async fn claim_parental_code(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ClaimChildCodeRequest>,
) -> escudo_common::Result<Json<ParentalChild>> {
    let code = req.access_code.trim();
    if code.len() != 16 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(EscudoError::BadRequest("Access code must be a 16-digit code".into()));
    }

    let child_id: Uuid = sqlx::query_scalar(
        r#"
        UPDATE parental_children
        SET child_user_id = $2,
            linked_at = NOW(),
            updated_at = NOW()
        WHERE access_code = $1
          AND is_active = TRUE
          AND (child_user_id IS NULL OR child_user_id = $2)
        RETURNING id
        "#,
    )
    .bind(code)
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Child code not found or already linked".into()))?;

    let _ = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        None,
        None,
        "parental_child_claim",
        "success",
        Some(format!("Linked child code {code}")),
        json!({ "child_id": child_id }),
    )
    .await;

    Ok(Json(fetch_parental_child(&state, child_id).await?))
}

pub async fn link_parental_child_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(child_id): Path<Uuid>,
    Json(req): Json<LinkDeviceRequest>,
) -> escudo_common::Result<Json<ChildDevice>> {
    ensure_parent_owns_child(&state, auth.0.sub, child_id).await?;

    if req.device_id.is_none() && req.device_install_id.as_deref().unwrap_or("").trim().is_empty() {
        return Err(EscudoError::BadRequest("Either device_id or device_install_id is required".into()));
    }

    let display_name = req
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Child Device")
        .to_string();

    let row = sqlx::query(
        r#"
        INSERT INTO parental_child_devices (child_id, device_id, device_install_id, display_name, platform, notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, device_id, device_install_id, display_name, platform, notes, is_active, linked_at
        "#,
    )
    .bind(child_id)
    .bind(req.device_id)
    .bind(req.device_install_id.as_deref().map(str::trim).filter(|value| !value.is_empty()))
    .bind(display_name)
    .bind(req.platform)
    .bind(req.notes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ChildDevice {
        id: row.get("id"),
        device_id: row.get("device_id"),
        device_install_id: row.get("device_install_id"),
        display_name: row.get("display_name"),
        platform: row.get("platform"),
        notes: row.get("notes"),
        is_active: row.get("is_active"),
        linked_at: row.get("linked_at"),
    }))
}

pub async fn upsert_parental_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(child_id): Path<Uuid>,
    Json(req): Json<UpsertPolicyRequest>,
) -> escudo_common::Result<Json<ParentalPolicy>> {
    ensure_parent_owns_child(&state, auth.0.sub, child_id).await?;
    let bedtime_start_minute = validate_minutes(req.bedtime_start_minute, "bedtime_start_minute")?;
    let bedtime_end_minute = validate_minutes(req.bedtime_end_minute, "bedtime_end_minute")?;

    let row = sqlx::query(
        r#"
        INSERT INTO parental_policies (
            child_id, target_device_id, block_tiktok, block_youtube, block_social_media, block_streaming,
            bedtime_enabled, bedtime_start_minute, bedtime_end_minute, max_daily_minutes,
            monitored_apps, blocked_apps, custom_blocked_domains, custom_allowed_domains, is_active
        )
        VALUES ($1, $2, COALESCE($3, FALSE), COALESCE($4, FALSE), COALESCE($5, FALSE), COALESCE($6, FALSE),
                COALESCE($7, FALSE), $8, $9, $10, $11, $12, $13, $14, COALESCE($15, TRUE))
        RETURNING id, child_id, target_device_id, block_tiktok, block_youtube, block_social_media,
                  block_streaming, bedtime_enabled, bedtime_start_minute, bedtime_end_minute,
                  max_daily_minutes, monitored_apps, blocked_apps, custom_blocked_domains,
                  custom_allowed_domains, is_active, updated_at
        "#,
    )
    .bind(child_id)
    .bind(req.target_device_id)
    .bind(req.block_tiktok)
    .bind(req.block_youtube)
    .bind(req.block_social_media)
    .bind(req.block_streaming)
    .bind(req.bedtime_enabled)
    .bind(bedtime_start_minute)
    .bind(bedtime_end_minute)
    .bind(req.max_daily_minutes)
    .bind(normalize_string_list(req.monitored_apps, 32, 64))
    .bind(normalize_string_list(req.blocked_apps, 32, 64))
    .bind(normalize_string_list(req.custom_blocked_domains, 64, 128))
    .bind(normalize_string_list(req.custom_allowed_domains, 64, 128))
    .bind(req.is_active)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ParentalPolicy {
        id: row.get("id"),
        child_id: row.get("child_id"),
        target_device_id: row.get("target_device_id"),
        block_tiktok: row.get("block_tiktok"),
        block_youtube: row.get("block_youtube"),
        block_social_media: row.get("block_social_media"),
        block_streaming: row.get("block_streaming"),
        bedtime_enabled: row.get("bedtime_enabled"),
        bedtime_start_minute: row.get("bedtime_start_minute"),
        bedtime_end_minute: row.get("bedtime_end_minute"),
        max_daily_minutes: row.get("max_daily_minutes"),
        monitored_apps: row.get::<Option<Vec<String>>, _>("monitored_apps").unwrap_or_default(),
        blocked_apps: row.get::<Option<Vec<String>>, _>("blocked_apps").unwrap_or_default(),
        custom_blocked_domains: row
            .get::<Option<Vec<String>>, _>("custom_blocked_domains")
            .unwrap_or_default(),
        custom_allowed_domains: row
            .get::<Option<Vec<String>>, _>("custom_allowed_domains")
            .unwrap_or_default(),
        is_active: row.get("is_active"),
        updated_at: row.get("updated_at"),
    }))
}

pub async fn list_parental_schedules(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(child_id): Path<Uuid>,
) -> escudo_common::Result<Json<Vec<ParentalSchedule>>> {
    ensure_parent_owns_child(&state, auth.0.sub, child_id).await?;
    Ok(Json(fetch_child_schedules(&state, child_id).await?))
}

pub async fn create_parental_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(child_id): Path<Uuid>,
    Json(req): Json<CreateScheduleRequest>,
) -> escudo_common::Result<Json<ParentalSchedule>> {
    ensure_parent_owns_child(&state, auth.0.sub, child_id).await?;
    let name = req.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(EscudoError::BadRequest("Schedule name must be between 1 and 80 characters".into()));
    }
    if !(0..=1440).contains(&req.start_minute) || !(0..=1440).contains(&req.end_minute) {
        return Err(EscudoError::BadRequest("Schedule minutes must be between 0 and 1440".into()));
    }
    let days = validate_days_of_week(req.days_of_week)?;

    let row = sqlx::query(
        r#"
        INSERT INTO parental_schedules (child_id, name, days_of_week, start_minute, end_minute, blocked_categories, blocked_apps, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, TRUE))
        RETURNING id, child_id, name, days_of_week, start_minute, end_minute, blocked_categories, blocked_apps, is_active, updated_at
        "#,
    )
    .bind(child_id)
    .bind(name)
    .bind(days)
    .bind(req.start_minute)
    .bind(req.end_minute)
    .bind(normalize_string_list(req.blocked_categories, 16, 64))
    .bind(normalize_string_list(req.blocked_apps, 32, 64))
    .bind(req.is_active)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ParentalSchedule {
        id: row.get("id"),
        child_id: row.get("child_id"),
        name: row.get("name"),
        days_of_week: row.get::<Option<Vec<i32>>, _>("days_of_week").unwrap_or_default(),
        start_minute: row.get("start_minute"),
        end_minute: row.get("end_minute"),
        blocked_categories: row
            .get::<Option<Vec<String>>, _>("blocked_categories")
            .unwrap_or_default(),
        blocked_apps: row.get::<Option<Vec<String>>, _>("blocked_apps").unwrap_or_default(),
        is_active: row.get("is_active"),
        updated_at: row.get("updated_at"),
    }))
}

pub async fn list_parental_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(child_id): Path<Uuid>,
) -> escudo_common::Result<Json<Vec<ParentalEvent>>> {
    ensure_parent_owns_child(&state, auth.0.sub, child_id).await?;
    Ok(Json(fetch_recent_parental_events(&state, child_id, 200).await?))
}

pub async fn record_parental_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RecordParentalEventRequest>,
) -> escudo_common::Result<Json<MessageResponse>> {
    let child_parent: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT parent_user_id, child_user_id FROM parental_children WHERE id = $1 AND is_active = TRUE",
    )
    .bind(req.child_id)
    .fetch_optional(&state.db)
    .await?;

    let (parent_user_id, child_user_id) =
        child_parent.ok_or_else(|| EscudoError::NotFound("Child profile not found".into()))?;

    if auth.0.sub != parent_user_id && Some(auth.0.sub) != child_user_id {
        return Err(EscudoError::Forbidden("Not authorized to record child event".into()));
    }

    let event_type = req.event_type.trim().to_ascii_lowercase();
    if event_type.is_empty() || event_type.len() > 64 {
        return Err(EscudoError::BadRequest("event_type must be between 1 and 64 characters".into()));
    }

    sqlx::query(
        r#"
        INSERT INTO parental_events (child_id, device_id, event_type, app_identifier, domain, action, detail, event_metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(req.child_id)
    .bind(req.device_id)
    .bind(&event_type)
    .bind(req.app_identifier.as_deref().map(str::trim).filter(|value| !value.is_empty()))
    .bind(req.domain.as_deref().map(str::trim).filter(|value| !value.is_empty()))
    .bind(req.action.as_deref().map(str::trim).filter(|value| !value.is_empty()))
    .bind(req.detail.as_deref().map(str::trim).filter(|value| !value.is_empty()))
    .bind(req.event_metadata.unwrap_or_else(|| json!({})))
    .execute(&state.db)
    .await?;

    Ok(Json(MessageResponse {
        message: "Child supervision event recorded".into(),
    }))
}

pub async fn get_my_device_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<DevicePolicyQuery>,
) -> escudo_common::Result<Json<DevicePolicyResponse>> {
    let device_install_id = query
        .device_install_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let child_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM parental_children WHERE child_user_id = $1 AND is_active = TRUE ORDER BY linked_at DESC NULLS LAST, created_at DESC LIMIT 1",
    )
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?;

    let Some(child_id) = child_id else {
        return Ok(Json(DevicePolicyResponse {
            child: None,
            device_install_id,
            device_linked: false,
            effective_policies: Vec::new(),
            effective_schedules: Vec::new(),
            recent_events: Vec::new(),
        }));
    };

    let child = fetch_parental_child(&state, child_id).await?;
    let device_linked = if let Some(ref install_id) = device_install_id {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1 FROM parental_child_devices
                WHERE child_id = $1 AND device_install_id = $2 AND is_active = TRUE
            )",
        )
        .bind(child_id)
        .bind(install_id)
        .fetch_one(&state.db)
        .await?
    } else {
        false
    };

    let effective_policies = child
        .policies
        .iter()
        .filter(|policy| policy.is_active)
        .filter(|policy| policy.target_device_id.is_none() || device_linked)
        .cloned()
        .collect();

    let effective_schedules = child
        .schedules
        .iter()
        .filter(|schedule| schedule.is_active)
        .cloned()
        .collect();

    let recent_events = fetch_recent_parental_events(&state, child_id, 25).await?;

    Ok(Json(DevicePolicyResponse {
        child: Some(child),
        device_install_id,
        device_linked,
        effective_policies,
        effective_schedules,
        recent_events,
    }))
}
