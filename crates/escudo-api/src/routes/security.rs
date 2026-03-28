use axum::extract::{Path, State};
use axum::Json;
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response / request types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct BreachInfo {
    pub name: String,
    pub date: Option<String>,
    pub description: Option<String>,
    pub data_types: Vec<String>,
}

#[derive(Serialize)]
pub struct BreachCheckResponse {
    pub email: String,
    pub found: bool,
    pub breach_count: usize,
    pub breaches: Vec<BreachInfo>,
}

#[derive(Deserialize)]
pub struct BreachCheckRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct BreachMonitor {
    pub id: Uuid,
    pub email: String,
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
    pub breach_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct AddMonitorRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ---------------------------------------------------------------------------
// Have I Been Pwned API integration (HIBP v3)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct HibpBreach {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Title")]
    title: Option<String>,
    #[serde(rename = "BreachDate")]
    breach_date: Option<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "DataClasses")]
    data_classes: Option<Vec<String>>,
    #[serde(rename = "PwnCount")]
    pwn_count: Option<u64>,
    #[serde(rename = "LogoPath")]
    logo_path: Option<String>,
}

async fn check_breach_hibp(email: &str) -> Result<Vec<BreachInfo>, String> {
    let api_key = std::env::var("HIBP_API_KEY").map_err(|_| "HIBP_API_KEY not set".to_string())?;

    let client = reqwest::Client::builder()
        .user_agent("EscudoVPN/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let url = format!(
        "https://haveibeenpwned.com/api/v3/breachedaccount/{}?truncateResponse=false",
        urlencoding::encode(email)
    );

    let resp = client
        .get(&url)
        .header("hibp-api-key", &api_key)
        .send()
        .await
        .map_err(|e| format!("HIBP request failed: {e}"))?;

    let status = resp.status();

    // 404 = no breaches found
    if status.as_u16() == 404 {
        return Ok(vec![]);
    }

    // 429 = rate limited
    if status.as_u16() == 429 {
        return Err("Taxa de requisicoes excedida. Tente novamente em alguns segundos.".into());
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HIBP API retornou status {status}: {body}"));
    }

    let hibp_breaches: Vec<HibpBreach> = resp
        .json()
        .await
        .map_err(|e| format!("Falha ao parsear resposta HIBP: {e}"))?;

    let breaches = hibp_breaches
        .into_iter()
        .map(|b| BreachInfo {
            name: b.title.unwrap_or(b.name),
            date: b.breach_date,
            description: b.description,
            data_types: b.data_classes.unwrap_or_default(),
        })
        .collect();

    Ok(breaches)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/security/breach-check
/// Immediate one-off check — does NOT persist the email as a monitor.
pub async fn breach_check(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<BreachCheckRequest>,
) -> escudo_common::Result<Json<BreachCheckResponse>> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(EscudoError::BadRequest("E-mail invalido".into()));
    }

    let breaches = check_breach_hibp(&email).await.map_err(|e| {
        error!("Breach check failed for {email}: {e}");
        EscudoError::Internal("Falha ao verificar vazamentos".into())
    })?;

    let count = breaches.len();
    Ok(Json(BreachCheckResponse {
        email,
        found: count > 0,
        breach_count: count,
        breaches,
    }))
}

/// GET /api/v1/security/breach-monitors
pub async fn list_monitors(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<BreachMonitor>>> {
    let user_id = auth.0.sub;

    let rows = sqlx::query(
        r#"
        SELECT id, email, last_checked, breach_count, created_at
        FROM breach_monitors
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to list breach monitors for user {user_id}: {e}");
        EscudoError::Internal("Falha ao listar monitoramentos".into())
    })?;

    use sqlx::Row;
    let monitors = rows
        .iter()
        .map(|row| BreachMonitor {
            id: row.get("id"),
            email: row.get("email"),
            last_checked: row.get("last_checked"),
            breach_count: row.get::<Option<i32>, _>("breach_count").unwrap_or(0),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(monitors))
}

/// POST /api/v1/security/breach-monitors
/// Add an email to monitor and run an initial check.
pub async fn add_monitor(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AddMonitorRequest>,
) -> escudo_common::Result<Json<BreachMonitor>> {
    let user_id = auth.0.sub;
    let email = req.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return Err(EscudoError::BadRequest("E-mail invalido".into()));
    }

    // Check if already monitored (unique index handles the race, but give a friendly message)
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM breach_monitors WHERE user_id=$1 AND email=$2)",
    )
    .bind(user_id)
    .bind(&email)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to check existing monitor for user {user_id}: {e}");
        EscudoError::Internal("Falha ao verificar monitoramento".into())
    })?;

    if exists {
        return Err(EscudoError::Conflict(
            "Este e-mail ja esta sendo monitorado".into(),
        ));
    }

    // Initial breach check
    let breaches = check_breach_hibp(&email).await.unwrap_or_default();
    let breach_count = breaches.len() as i32;

    // Insert monitor record
    let row = sqlx::query(
        r#"
        INSERT INTO breach_monitors (user_id, email, last_checked, breach_count)
        VALUES ($1, $2, now(), $3)
        RETURNING id, email, last_checked, breach_count, created_at
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(breach_count)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to insert breach monitor for user {user_id}: {e}");
        EscudoError::Internal("Falha ao adicionar monitoramento".into())
    })?;

    use sqlx::Row;
    let monitor_id: Uuid = row.get("id");

    // Persist breach results
    for breach in &breaches {
        let _ = sqlx::query(
            r#"
            INSERT INTO breach_results (monitor_id, breach_name, breach_date, breach_description, data_types)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(monitor_id)
        .bind(&breach.name)
        .bind(&breach.date)
        .bind(&breach.description)
        .bind(&breach.data_types)
        .execute(&state.db)
        .await;
    }

    Ok(Json(BreachMonitor {
        id: monitor_id,
        email: row.get("email"),
        last_checked: row.get("last_checked"),
        breach_count: row.get::<Option<i32>, _>("breach_count").unwrap_or(0),
        created_at: row.get("created_at"),
    }))
}

/// DELETE /api/v1/security/breach-monitors/:id
pub async fn remove_monitor(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> escudo_common::Result<Json<MessageResponse>> {
    let user_id = auth.0.sub;

    // Verify ownership before deleting
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM breach_monitors WHERE id=$1 AND user_id=$2)",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to check breach monitor ownership for user {user_id}: {e}");
        EscudoError::Internal("Falha ao verificar monitoramento".into())
    })?;

    if !exists {
        return Err(EscudoError::NotFound("Monitoramento nao encontrado".into()));
    }

    // Delete results first (FK constraint)
    sqlx::query("DELETE FROM breach_results WHERE monitor_id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to delete breach results for monitor {id}: {e}");
            EscudoError::Internal("Falha ao remover monitoramento".into())
        })?;

    sqlx::query("DELETE FROM breach_monitors WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to delete breach monitor {id} for user {user_id}: {e}");
            EscudoError::Internal("Falha ao remover monitoramento".into())
        })?;

    Ok(Json(MessageResponse {
        message: "Monitoramento removido com sucesso".into(),
    }))
}

// ---------------------------------------------------------------------------
// HIBP Paste monitoring
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PasteInfo {
    pub source: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub date: Option<String>,
    pub email_count: Option<u64>,
}

#[derive(Serialize)]
pub struct PasteCheckResponse {
    pub email: String,
    pub found: bool,
    pub paste_count: usize,
    pub pastes: Vec<PasteInfo>,
}

/// POST /api/v1/security/paste-check
/// Check if an email appeared in public pastes (Pastebin, etc.)
pub async fn paste_check(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<BreachCheckRequest>,
) -> escudo_common::Result<Json<PasteCheckResponse>> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(EscudoError::BadRequest("E-mail inválido".into()));
    }

    let api_key = std::env::var("HIBP_API_KEY")
        .map_err(|_| EscudoError::Internal("HIBP_API_KEY não configurada".into()))?;

    let client = reqwest::Client::builder()
        .user_agent("EscudoVPN/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| EscudoError::Internal(format!("Falha ao criar cliente HTTP: {e}")))?;

    let url = format!(
        "https://haveibeenpwned.com/api/v3/pasteaccount/{}",
        urlencoding::encode(&email)
    );

    let resp = client
        .get(&url)
        .header("hibp-api-key", &api_key)
        .send()
        .await
        .map_err(|e| EscudoError::Internal(format!("Falha na requisição HIBP: {e}")))?;

    if resp.status().as_u16() == 404 {
        return Ok(Json(PasteCheckResponse {
            email,
            found: false,
            paste_count: 0,
            pastes: vec![],
        }));
    }

    if resp.status().as_u16() == 429 {
        return Err(EscudoError::Internal(
            "Taxa de requisições excedida. Tente em alguns segundos.".into(),
        ));
    }

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(EscudoError::Internal(format!(
            "HIBP retornou status: {body}"
        )));
    }

    #[derive(Deserialize)]
    struct HibpPaste {
        #[serde(rename = "Source")]
        source: String,
        #[serde(rename = "Id")]
        id: Option<String>,
        #[serde(rename = "Title")]
        title: Option<String>,
        #[serde(rename = "Date")]
        date: Option<String>,
        #[serde(rename = "EmailCount")]
        email_count: Option<u64>,
    }

    let hibp_pastes: Vec<HibpPaste> = resp
        .json()
        .await
        .map_err(|e| EscudoError::Internal(format!("Falha ao parsear resposta: {e}")))?;

    let count = hibp_pastes.len();
    let pastes = hibp_pastes
        .into_iter()
        .map(|p| PasteInfo {
            source: p.source,
            id: p.id,
            title: p.title,
            date: p.date,
            email_count: p.email_count,
        })
        .collect();

    Ok(Json(PasteCheckResponse {
        email,
        found: count > 0,
        paste_count: count,
        pastes,
    }))
}

// ---------------------------------------------------------------------------
// Latest breach feed (free, no auth needed on HIBP side)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct LatestBreachResponse {
    pub name: String,
    pub title: String,
    pub domain: String,
    pub breach_date: String,
    pub pwn_count: u64,
    pub description: String,
    pub data_classes: Vec<String>,
}

/// GET /api/v1/security/latest-breach
/// Returns the most recently added breach to HIBP — free endpoint, good for news feed
pub async fn latest_breach(
    State(_state): State<AppState>,
) -> escudo_common::Result<Json<LatestBreachResponse>> {
    let client = reqwest::Client::builder()
        .user_agent("EscudoVPN/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| EscudoError::Internal(format!("Falha ao criar cliente HTTP: {e}")))?;

    let resp = client
        .get("https://haveibeenpwned.com/api/v3/latestbreach")
        .send()
        .await
        .map_err(|e| EscudoError::Internal(format!("Falha na requisição HIBP: {e}")))?;

    if !resp.status().is_success() {
        return Err(EscudoError::Internal(
            "Falha ao buscar último vazamento".into(),
        ));
    }

    let breach: HibpBreach = resp
        .json()
        .await
        .map_err(|e| EscudoError::Internal(format!("Falha ao parsear: {e}")))?;

    Ok(Json(LatestBreachResponse {
        name: breach.name,
        title: breach.title.unwrap_or_default(),
        domain: "".to_string(),
        breach_date: breach.breach_date.unwrap_or_default(),
        pwn_count: breach.pwn_count.unwrap_or(0),
        description: breach.description.unwrap_or_default(),
        data_classes: breach.data_classes.unwrap_or_default(),
    }))
}

// ---------------------------------------------------------------------------
// Full breach details for a specific breach (free, no auth)
// ---------------------------------------------------------------------------

/// GET /api/v1/security/breach/:name
/// Get details of a specific breach by name
pub async fn breach_details(Path(name): Path<String>) -> escudo_common::Result<Json<BreachInfo>> {
    let client = reqwest::Client::builder()
        .user_agent("EscudoVPN/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| EscudoError::Internal(format!("Falha ao criar cliente HTTP: {e}")))?;

    let resp = client
        .get(format!(
            "https://haveibeenpwned.com/api/v3/breach/{}",
            urlencoding::encode(&name)
        ))
        .send()
        .await
        .map_err(|e| EscudoError::Internal(format!("Falha na requisição: {e}")))?;

    if resp.status().as_u16() == 404 {
        return Err(EscudoError::NotFound("Vazamento não encontrado".into()));
    }

    let breach: HibpBreach = resp
        .json()
        .await
        .map_err(|e| EscudoError::Internal(format!("Falha ao parsear: {e}")))?;

    Ok(Json(BreachInfo {
        name: breach.title.unwrap_or(breach.name),
        date: breach.breach_date,
        description: breach.description,
        data_types: breach.data_classes.unwrap_or_default(),
    }))
}
