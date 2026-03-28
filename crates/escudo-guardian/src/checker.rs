use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::config::StreamingService;

/// Result of a single health check against one streaming service.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The service name (e.g. "netflix").
    pub service: String,
    /// One of: "healthy", "blocked", "timeout", "error".
    pub status: String,
    /// Round-trip time in milliseconds, if a response was received.
    pub response_time_ms: Option<i32>,
    /// Human-readable detail for non-healthy results.
    pub error_detail: Option<String>,
}

/// Tests a single proxy IP against streaming services.
pub struct HealthChecker;

impl HealthChecker {
    /// Check a proxy URL against a single streaming service.
    ///
    /// Creates a new reqwest::Client per check, each configured with its own
    /// SOCKS5 proxy so they are fully isolated.
    pub async fn check(socks5_url: &str, service: &StreamingService) -> Result<CheckResult> {
        let proxy = match reqwest::Proxy::all(socks5_url) {
            Ok(p) => p,
            Err(e) => {
                return Ok(CheckResult {
                    service: service.name.clone(),
                    status: "error".to_string(),
                    response_time_ms: None,
                    error_detail: Some(format!("invalid proxy URL: {e}")),
                });
            }
        };

        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (compatible; EscudoGuardian/1.0)")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return Ok(CheckResult {
                    service: service.name.clone(),
                    status: "error".to_string(),
                    response_time_ms: None,
                    error_detail: Some(format!("failed to build HTTP client: {e}")),
                });
            }
        };

        let start = Instant::now();

        let response = match client.get(&service.url).send().await {
            Ok(r) => r,
            Err(e) => {
                let elapsed_ms = start.elapsed().as_millis() as i32;
                if e.is_timeout() {
                    warn!(
                        service = %service.name,
                        proxy = %socks5_url,
                        "health check timed out"
                    );
                    return Ok(CheckResult {
                        service: service.name.clone(),
                        status: "timeout".to_string(),
                        response_time_ms: Some(elapsed_ms),
                        error_detail: Some("request timed out after 15s".to_string()),
                    });
                }
                warn!(
                    service = %service.name,
                    proxy = %socks5_url,
                    error = %e,
                    "health check request error"
                );
                return Ok(CheckResult {
                    service: service.name.clone(),
                    status: "error".to_string(),
                    response_time_ms: Some(elapsed_ms),
                    error_detail: Some(format!("{e}")),
                });
            }
        };

        let elapsed_ms = start.elapsed().as_millis() as i32;
        let status_code = response.status();

        let body = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                return Ok(CheckResult {
                    service: service.name.clone(),
                    status: "error".to_string(),
                    response_time_ms: Some(elapsed_ms),
                    error_detail: Some(format!("failed to read response body: {e}")),
                });
            }
        };

        let body_lower = body.to_lowercase();

        // Check for block indicators (case-insensitive).
        for indicator in &service.block_indicators {
            if body_lower.contains(&indicator.to_lowercase()) {
                debug!(
                    service = %service.name,
                    indicator = %indicator,
                    "block indicator found in response"
                );
                return Ok(CheckResult {
                    service: service.name.clone(),
                    status: "blocked".to_string(),
                    response_time_ms: Some(elapsed_ms),
                    error_detail: Some(format!(
                        "block indicator found: \"{indicator}\" (HTTP {status_code})"
                    )),
                });
            }
        }

        // Non-2xx responses without block indicators count as an error.
        if !status_code.is_success() {
            return Ok(CheckResult {
                service: service.name.clone(),
                status: "error".to_string(),
                response_time_ms: Some(elapsed_ms),
                error_detail: Some(format!("HTTP {status_code}")),
            });
        }

        debug!(
            service = %service.name,
            response_ms = elapsed_ms,
            "health check passed"
        );
        Ok(CheckResult {
            service: service.name.clone(),
            status: "healthy".to_string(),
            response_time_ms: Some(elapsed_ms),
            error_detail: None,
        })
    }
}
