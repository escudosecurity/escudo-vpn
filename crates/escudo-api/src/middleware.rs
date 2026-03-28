use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use escudo_common::jwt::{decode_jwt, Claims};
use escudo_common::EscudoError;
use tracing::{error, warn};

use crate::state::AppState;

pub struct AuthUser(pub Claims);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = EscudoError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| EscudoError::Unauthorized("Missing authorization header".into()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| EscudoError::Unauthorized("Invalid authorization format".into()))?;

        let claims = decode_jwt(token, &state.config.jwt.secret)?;

        let user_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
                .bind(claims.sub)
                .fetch_one(&state.db)
                .await
                .map_err(|e| {
                    error!("Auth user lookup failed for {}: {e}", claims.sub);
                    EscudoError::Internal("Authentication lookup failed".into())
                })?;

        if !user_exists {
            warn!("JWT accepted for missing user {}", claims.sub);
            return Err(EscudoError::Unauthorized("Account not found".into()));
        }

        Ok(AuthUser(claims))
    }
}
