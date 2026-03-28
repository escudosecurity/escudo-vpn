use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike, Utc};
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct EffectiveDnsPolicy {
    pub child_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub blocked_domains: HashSet<String>,
    pub allowed_domains: HashSet<String>,
    pub blocked_categories: HashSet<String>,
    pub blocked_apps: HashSet<String>,
}

struct CachedPolicy {
    policy: EffectiveDnsPolicy,
    cached_at: Instant,
}

#[derive(Clone)]
pub struct PolicyResolver {
    db: PgPool,
    cache: Arc<Mutex<HashMap<IpAddr, CachedPolicy>>>,
}

impl PolicyResolver {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn resolve_for_ip(&self, client_ip: IpAddr) -> Option<EffectiveDnsPolicy> {
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&client_ip) {
                if cached.cached_at.elapsed() < Duration::from_secs(60) {
                    return Some(cached.policy.clone());
                }
            }
        }

        let policy = match self.query_policy(client_ip).await {
            Ok(policy) => policy,
            Err(error) => {
                warn!("Failed to resolve parental DNS policy for {client_ip}: {error}");
                None
            }
        };

        if let Some(policy) = policy.clone() {
            let mut cache = self.cache.lock().await;
            cache.insert(
                client_ip,
                CachedPolicy {
                    policy,
                    cached_at: Instant::now(),
                },
            );
        }

        policy
    }

    pub async fn record_blocked_event(
        &self,
        policy: &EffectiveDnsPolicy,
        domain: &str,
        reason: &str,
    ) {
        let Some(child_id) = policy.child_id else {
            return;
        };

        if let Err(error) = sqlx::query(
            "INSERT INTO parental_events (child_id, device_id, event_type, domain, action, detail, event_metadata)
             VALUES ($1, $2, 'dns_block', $3, 'blocked', $4, $5)",
        )
        .bind(child_id)
        .bind(policy.device_id)
        .bind(domain)
        .bind(reason)
        .bind(serde_json::json!({ "source": "escudo-dns", "domain": domain, "reason": reason }))
        .execute(&self.db)
        .await
        {
            warn!("Failed to record parental DNS block event for {domain}: {error}");
        }
    }

    async fn query_policy(&self, client_ip: IpAddr) -> Result<Option<EffectiveDnsPolicy>, sqlx::Error> {
        let client_ip = client_ip.to_string();
        let device_row = sqlx::query(
            r#"
            SELECT d.id AS device_id, d.user_id, d.device_install_id, pc.id AS child_id
            FROM devices d
            LEFT JOIN parental_children pc
              ON pc.child_user_id = d.user_id
             AND pc.is_active = TRUE
            WHERE d.assigned_ip = $1
              AND d.is_active = TRUE
            ORDER BY d.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&client_ip)
        .fetch_optional(&self.db)
        .await?;

        let Some(device_row) = device_row else {
            return Ok(None);
        };

        let device_id: Option<Uuid> = device_row.get("device_id");
        let user_id: Uuid = device_row.get("user_id");
        let child_id: Option<Uuid> = device_row.get("child_id");

        let mut policy = EffectiveDnsPolicy {
            child_id,
            device_id,
            ..Default::default()
        };

        if let Some(profile_row) = sqlx::query(
            r#"
            SELECT block_social_media, block_gaming, custom_blocked_domains, custom_allowed_domains
            FROM family_profiles
            WHERE user_id = $1 AND is_active = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        {
            if profile_row.get::<bool, _>("block_social_media") {
                policy.blocked_categories.insert("social_media".into());
            }
            if profile_row.get::<bool, _>("block_gaming") {
                policy.blocked_categories.insert("gaming".into());
            }
            for domain in profile_row
                .get::<Option<Vec<String>>, _>("custom_blocked_domains")
                .unwrap_or_default()
            {
                policy.blocked_domains.insert(domain.to_ascii_lowercase());
            }
            for domain in profile_row
                .get::<Option<Vec<String>>, _>("custom_allowed_domains")
                .unwrap_or_default()
            {
                policy.allowed_domains.insert(domain.to_ascii_lowercase());
            }
        }

        let Some(child_id) = child_id else {
            return Ok(Some(policy));
        };

        for row in sqlx::query(
            r#"
            SELECT target_device_id, block_tiktok, block_youtube, block_social_media, block_streaming,
                   bedtime_enabled, bedtime_start_minute, bedtime_end_minute, blocked_apps,
                   custom_blocked_domains, custom_allowed_domains
            FROM parental_policies
            WHERE child_id = $1 AND is_active = TRUE
            ORDER BY updated_at DESC
            "#,
        )
        .bind(child_id)
        .fetch_all(&self.db)
        .await?
        {
            let target_device_id: Option<Uuid> = row.get("target_device_id");
            if target_device_id.is_some() && target_device_id != device_id {
                continue;
            }

            if row.get::<bool, _>("block_tiktok") {
                policy.blocked_categories.insert("tiktok".into());
            }
            if row.get::<bool, _>("block_youtube") {
                policy.blocked_categories.insert("youtube".into());
            }
            if row.get::<bool, _>("block_social_media") {
                policy.blocked_categories.insert("social_media".into());
            }
            if row.get::<bool, _>("block_streaming") {
                policy.blocked_categories.insert("streaming".into());
            }

            let bedtime_enabled: bool = row.get("bedtime_enabled");
            let bedtime_start: Option<i32> = row.get("bedtime_start_minute");
            let bedtime_end: Option<i32> = row.get("bedtime_end_minute");
            if bedtime_enabled && bedtime_active(bedtime_start, bedtime_end) {
                policy.blocked_categories.insert("social_media".into());
                policy.blocked_categories.insert("streaming".into());
                policy.blocked_categories.insert("youtube".into());
                policy.blocked_categories.insert("tiktok".into());
            }

            for app in row.get::<Option<Vec<String>>, _>("blocked_apps").unwrap_or_default() {
                policy.blocked_apps.insert(app.to_ascii_lowercase());
            }
            for domain in row
                .get::<Option<Vec<String>>, _>("custom_blocked_domains")
                .unwrap_or_default()
            {
                policy.blocked_domains.insert(domain.to_ascii_lowercase());
            }
            for domain in row
                .get::<Option<Vec<String>>, _>("custom_allowed_domains")
                .unwrap_or_default()
            {
                policy.allowed_domains.insert(domain.to_ascii_lowercase());
            }
        }

        let now = Utc::now();
        let weekday = now.weekday().number_from_monday() as i32;
        let minute = (now.hour() as i32) * 60 + (now.minute() as i32);
        for row in sqlx::query(
            r#"
            SELECT days_of_week, start_minute, end_minute, blocked_categories, blocked_apps
            FROM parental_schedules
            WHERE child_id = $1 AND is_active = TRUE
            ORDER BY updated_at DESC
            "#,
        )
        .bind(child_id)
        .fetch_all(&self.db)
        .await?
        {
            let days = row.get::<Option<Vec<i32>>, _>("days_of_week").unwrap_or_default();
            if !days.contains(&weekday) {
                continue;
            }
            let start_minute: i32 = row.get("start_minute");
            let end_minute: i32 = row.get("end_minute");
            if !time_window_active(minute, start_minute, end_minute) {
                continue;
            }

            for category in row
                .get::<Option<Vec<String>>, _>("blocked_categories")
                .unwrap_or_default()
            {
                policy.blocked_categories.insert(category.to_ascii_lowercase());
            }
            for app in row.get::<Option<Vec<String>>, _>("blocked_apps").unwrap_or_default() {
                policy.blocked_apps.insert(app.to_ascii_lowercase());
            }
        }

        Ok(Some(policy))
    }
}

fn bedtime_active(start: Option<i32>, end: Option<i32>) -> bool {
    match (start, end) {
        (Some(start), Some(end)) => {
            let now = Utc::now();
            let minute = (now.hour() as i32) * 60 + (now.minute() as i32);
            time_window_active(minute, start, end)
        }
        _ => false,
    }
}

fn time_window_active(now: i32, start: i32, end: i32) -> bool {
    if start == end {
        return true;
    }
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

pub fn match_policy_block(policy: &EffectiveDnsPolicy, domain: &str) -> Option<&'static str> {
    let name = domain.trim_end_matches('.').to_ascii_lowercase();

    if domain_matches(&policy.allowed_domains, &name) {
        return None;
    }

    if domain_matches(&policy.blocked_domains, &name) {
        return Some("custom_domain");
    }

    let category = classify_domain(&name);
    if let Some(category) = category {
        if policy.blocked_categories.contains(category) {
            return Some(category);
        }
    }

    if domain_matches_app(&policy.blocked_apps, &name) {
        return Some("blocked_app");
    }

    None
}

fn domain_matches(domains: &HashSet<String>, candidate: &str) -> bool {
    domains.iter().any(|domain| {
        candidate == domain || candidate.ends_with(&format!(".{domain}"))
    })
}

fn domain_matches_app(apps: &HashSet<String>, domain: &str) -> bool {
    apps.iter().any(|app| match app.as_str() {
        "tiktok" => matches_patterns(domain, &["tiktokcdn.com", "tiktokv.com", "tiktok.com", "byteoversea.com"]),
        "youtube" => matches_patterns(domain, &["youtube.com", "youtu.be", "ytimg.com", "googlevideo.com"]),
        other => domain.contains(other),
    })
}

fn matches_patterns(domain: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| domain == *pattern || domain.ends_with(&format!(".{pattern}")))
}

fn classify_domain(domain: &str) -> Option<&'static str> {
    if matches_patterns(domain, &["tiktokcdn.com", "tiktokv.com", "tiktok.com", "byteoversea.com"]) {
        return Some("tiktok");
    }
    if matches_patterns(domain, &["youtube.com", "youtu.be", "ytimg.com", "googlevideo.com"]) {
        return Some("youtube");
    }
    if matches_patterns(
        domain,
        &[
            "facebook.com",
            "fbcdn.net",
            "instagram.com",
            "cdninstagram.com",
            "snapchat.com",
            "twitter.com",
            "x.com",
            "tiktok.com",
            "tiktokcdn.com",
        ],
    ) {
        return Some("social_media");
    }
    if matches_patterns(
        domain,
        &[
            "netflix.com",
            "nflxvideo.net",
            "disneyplus.com",
            "dssott.com",
            "spotify.com",
            "scdn.co",
            "primevideo.com",
            "hbomax.com",
            "max.com",
        ],
    ) {
        return Some("streaming");
    }
    if matches_patterns(
        domain,
        &["roblox.com", "epicgames.com", "steamcontent.com", "steampowered.com"],
    ) {
        return Some("gaming");
    }
    None
}
