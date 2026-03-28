use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::config::BlocklistSource;

pub type SharedBlocklist = Arc<RwLock<HashSet<String>>>;

pub fn new_blocklist() -> SharedBlocklist {
    Arc::new(RwLock::new(HashSet::new()))
}

fn parse_domains(body: &str) -> HashSet<String> {
    body.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Handle hostfile format: "0.0.0.0 domain" or "127.0.0.1 domain"
            if trimmed.starts_with("0.0.0.0") || trimmed.starts_with("127.0.0.1") {
                trimmed
                    .split_whitespace()
                    .nth(1)
                    .map(|d| d.to_lowercase())
                    .filter(|d| d != "localhost")
            } else {
                // Plain domain format
                Some(trimmed.to_lowercase())
            }
        })
        .collect()
}

async fn download_source(
    client: &reqwest::Client,
    source: &BlocklistSource,
) -> anyhow::Result<HashSet<String>> {
    info!(
        "Downloading blocklist '{}' from {}",
        source.name, source.url
    );
    let body = client.get(&source.url).send().await?.text().await?;
    let domains = parse_domains(&body);
    info!("Loaded {} domains from '{}'", domains.len(), source.name);
    Ok(domains)
}

pub async fn download_all_sources(sources: &[BlocklistSource]) -> HashSet<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("failed to build HTTP client");

    let futures: Vec<_> = sources
        .iter()
        .map(|src| download_source(&client, src))
        .collect();

    let results = futures::future::join_all(futures).await;

    let mut merged = HashSet::new();
    for result in results {
        match result {
            Ok(domains) => merged.extend(domains),
            Err(e) => warn!("Failed to download blocklist source: {e}"),
        }
    }

    info!("Total merged blocklist: {} unique domains", merged.len());
    merged
}

pub async fn refresh_loop(
    blocklist: SharedBlocklist,
    sources: Vec<BlocklistSource>,
    interval_hours: u64,
) {
    let interval = std::time::Duration::from_secs(interval_hours * 3600);

    // Initial load
    let domains = download_all_sources(&sources).await;
    if !domains.is_empty() {
        let mut bl = blocklist.write().await;
        *bl = domains;
    } else {
        error!("Initial blocklist load returned zero domains");
    }

    loop {
        tokio::time::sleep(interval).await;
        let domains = download_all_sources(&sources).await;
        if !domains.is_empty() {
            let mut bl = blocklist.write().await;
            *bl = domains;
        }
    }
}
