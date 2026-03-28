use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tracing::warn;

use crate::middleware::AuthUser;
use crate::state::gateway::GetStatsRequest;
use crate::state::AppState;

pub async fn stats_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    _auth: AuthUser,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_stats_socket(socket, state))
}

async fn handle_stats_socket(mut socket: WebSocket, state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        interval.tick().await;

        let mut gateway = state.gateway.clone();
        let stats = match gateway.get_stats(GetStatsRequest {}).await {
            Ok(resp) => resp.into_inner(),
            Err(e) => {
                warn!("Failed to get stats for WS: {e}");
                continue;
            }
        };

        let json = serde_json::json!({
            "total_peers": stats.total_peers,
            "total_rx_bytes": stats.total_rx_bytes,
            "total_tx_bytes": stats.total_tx_bytes,
            "uptime_seconds": stats.uptime_seconds,
        });

        if socket
            .send(Message::Text(json.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
