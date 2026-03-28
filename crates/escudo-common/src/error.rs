use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum EscudoError {
    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for EscudoError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            EscudoError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            EscudoError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            EscudoError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            EscudoError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            EscudoError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            EscudoError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            EscudoError::Database(e) => {
                tracing::error!("Database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            EscudoError::Jwt(e) => {
                tracing::warn!("JWT validation failed: {e}");
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired token".to_string(),
                )
            }
        };

        let body = json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, EscudoError>;
