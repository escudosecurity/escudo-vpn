use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use hickory_proto::op::{Header, ResponseCode};
use hickory_proto::rr::Record;
use hickory_resolver::TokioResolver;
use hickory_server::authority::MessageResponseBuilder;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use tracing::{debug, warn};

use crate::blocklist::SharedBlocklist;
use crate::policy::{match_policy_block, PolicyResolver};
use crate::stats::StatsRecorder;

pub struct DnsMetrics {
    pub queries_total: AtomicU64,
    pub blocked_total: AtomicU64,
}

impl DnsMetrics {
    pub fn new() -> Self {
        Self {
            queries_total: AtomicU64::new(0),
            blocked_total: AtomicU64::new(0),
        }
    }
}

pub struct EscudoHandler {
    blocklist: SharedBlocklist,
    resolver: TokioResolver,
    pub metrics: Arc<DnsMetrics>,
    stats_recorder: Option<StatsRecorder>,
    policy_resolver: Option<PolicyResolver>,
}

impl EscudoHandler {
    pub fn new(
        blocklist: SharedBlocklist,
        metrics: Arc<DnsMetrics>,
        stats_recorder: Option<StatsRecorder>,
        policy_resolver: Option<PolicyResolver>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let resolver = TokioResolver::builder_tokio()?.build();

        Ok(Self {
            blocklist,
            resolver,
            metrics,
            stats_recorder,
            policy_resolver,
        })
    }

    fn serve_failed() -> ResponseInfo {
        let mut header = Header::new();
        header.set_response_code(ResponseCode::ServFail);
        header.into()
    }

    fn matches_free_hosting_phishing(name: &str) -> bool {
        const FREE_HOSTING_SUFFIXES: &[&str] = &[
            "weebly.com",
            "wixsite.com",
            "blogspot.com",
            "000webhostapp.com",
            "sites.google.com",
        ];
        const FREE_HOSTING_PATTERNS: &[&str] = &[
            "outlook",
            "microsoft",
            "paypal",
            "banco",
            "itau",
            "bradesco",
            "nubank",
            "login",
            "verify",
            "secure",
            "account",
            "update",
            "confirm",
            "mail",
            "office",
            "onedrive",
            "hotmail",
        ];

        FREE_HOSTING_SUFFIXES.iter().any(|suffix| {
            name.ends_with(suffix)
                && FREE_HOSTING_PATTERNS
                    .iter()
                    .any(|pattern| name.contains(pattern))
        })
    }
}

#[async_trait::async_trait]
impl RequestHandler for EscudoHandler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        self.metrics.queries_total.fetch_add(1, Ordering::Relaxed);

        let client_ip = request.src().ip();

        let queries = request.queries();
        let query = match queries.first() {
            Some(q) => q,
            None => return Self::serve_failed(),
        };

        let name = query.name().to_string();
        let name_trimmed = name.trim_end_matches('.').to_lowercase();

        let parental_policy = if let Some(resolver) = &self.policy_resolver {
            resolver.resolve_for_ip(client_ip).await
        } else {
            None
        };

        // Check blocklist
        let base_blocked = {
            let bl = self.blocklist.read().await;
            bl.contains(&name_trimmed)
                || {
                    let parts: Vec<&str> = name_trimmed.split('.').collect();
                    (1..parts.len()).any(|i| {
                        let parent = parts[i..].join(".");
                        bl.contains(&parent)
                    })
                }
                || Self::matches_free_hosting_phishing(&name_trimmed)
        };
        let parental_reason = parental_policy
            .as_ref()
            .and_then(|policy| match_policy_block(policy, &name_trimmed));
        let is_blocked = base_blocked || parental_reason.is_some();

        // Record per-client stats (non-blocking; the buffer flushes to DB periodically)
        if let Some(recorder) = &self.stats_recorder {
            recorder.record(client_ip, is_blocked).await;
        }

        if is_blocked {
            if !base_blocked {
                if let (Some(resolver), Some(policy), Some(reason)) =
                    (&self.policy_resolver, parental_policy.as_ref(), parental_reason)
                {
                    resolver.record_blocked_event(policy, &name_trimmed, reason).await;
                }
            }
            self.metrics.blocked_total.fetch_add(1, Ordering::Relaxed);
            debug!("Blocked: {name_trimmed}");
            let builder = MessageResponseBuilder::from_message_request(request);
            let mut header = Header::response_from_request(request.header());
            header.set_response_code(ResponseCode::NXDomain);
            let response = builder.build_no_records(header);
            return response_handle
                .send_response(response)
                .await
                .unwrap_or_else(|_| Self::serve_failed());
        }

        // Forward all query types upstream
        let query_type = query.query_type();
        match self.resolver.lookup(&name_trimmed, query_type).await {
            Ok(lookup) => {
                let records: Vec<Record> = lookup.records().iter().cloned().collect();

                let builder = MessageResponseBuilder::from_message_request(request);
                let mut header = Header::response_from_request(request.header());
                header.set_response_code(ResponseCode::NoError);
                let response = builder.build(header, records.iter(), &[], &[], &[]);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|_| Self::serve_failed())
            }
            Err(e) => {
                warn!("Upstream resolution failed for {name_trimmed} ({query_type:?}): {e}");
                let builder = MessageResponseBuilder::from_message_request(request);
                let mut header = Header::response_from_request(request.header());
                header.set_response_code(ResponseCode::ServFail);
                let response = builder.build_no_records(header);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|_| Self::serve_failed())
            }
        }
    }
}
