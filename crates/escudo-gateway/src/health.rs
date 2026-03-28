use axum::extract::State;
use axum::routing::get;
use axum::Router;
use prometheus::Encoder;

use crate::stats::Metrics;

async fn health() -> &'static str {
    "OK"
}

async fn metrics_handler(State(metrics): State<Metrics>) -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = metrics.registry.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("encode metrics");
    String::from_utf8(buffer).expect("metrics utf8")
}

pub fn health_router(metrics: Metrics) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .with_state(metrics)
}
