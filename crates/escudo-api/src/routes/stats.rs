use axum::extract::{Query, State};
use axum::Json;
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};

use crate::middleware::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct DnsStatsResponse {
    pub blocked_today: i64,
    pub queries_today: i64,
    pub blocked_all_time: i64,
}

#[derive(Deserialize)]
pub struct DnsStatsQuery {
    pub range: Option<String>,
}

fn range_to_interval(range: &str) -> Result<&str, EscudoError> {
    match range {
        "1d" => Ok("1 day"),
        "7d" => Ok("7 days"),
        "30d" => Ok("30 days"),
        _ => Err(EscudoError::BadRequest(
            "Invalid range. Use 1d, 7d, or 30d".into(),
        )),
    }
}

pub async fn get_dns_stats(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<DnsStatsQuery>,
) -> escudo_common::Result<Json<DnsStatsResponse>> {
    let range_filter = if let Some(ref range) = params.range {
        let interval = range_to_interval(range)?;
        format!("AND date >= CURRENT_DATE - INTERVAL '{interval}'")
    } else {
        "AND date = CURRENT_DATE".to_string()
    };

    let query_str = format!(
        r#"
        SELECT
            COALESCE(SUM(queries_total), 0)::BIGINT,
            COALESCE(SUM(blocked_total), 0)::BIGINT
        FROM dns_stats
        WHERE client_ip IN (
            SELECT assigned_ip
            FROM devices
            WHERE user_id = $1
        )
        {range_filter}
        "#,
    );

    let today_row: Option<(i64, i64)> = sqlx::query_as(&query_str)
        .bind(auth.0.sub)
        .fetch_optional(&state.db)
        .await?;

    let (queries_today, blocked_today) = today_row.unwrap_or((0, 0));

    let blocked_all_time: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(blocked_total), 0)::BIGINT
        FROM dns_stats
        WHERE client_ip IN (
            SELECT assigned_ip
            FROM devices
            WHERE user_id = $1
        )
        "#,
    )
    .bind(auth.0.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to query all-time stats: {e}")))?;

    Ok(Json(DnsStatsResponse {
        blocked_today,
        queries_today,
        blocked_all_time,
    }))
}

#[derive(Serialize)]
pub struct BlockedDomain {
    pub domain: String,
    pub category: String,
    pub blocked_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_blocked_domains(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<BlockedDomain>>> {
    let rows = sqlx::query_as::<_, (String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT domain, category, created_at
           FROM blocked_domains
           ORDER BY created_at DESC
           LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await?;

    let domains = rows
        .into_iter()
        .map(|(domain, category, created_at)| BlockedDomain {
            domain,
            category: category.unwrap_or_default(),
            blocked_at: created_at,
        })
        .collect();

    Ok(Json(domains))
}
