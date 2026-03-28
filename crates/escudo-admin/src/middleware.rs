use crate::state::AdminState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use escudo_common::jwt::{decode_jwt, Claims, Role};
use escudo_common::EscudoError;

#[allow(dead_code)]
pub struct AdminUser(pub Claims);

#[async_trait::async_trait]
impl FromRequestParts<AdminState> for AdminUser {
    type Rejection = EscudoError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AdminState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(proxy_password) = std::env::var("ESCUDO_ADMIN_PROXY_PASSWORD")
            .ok()
            .filter(|v| !v.is_empty())
        {
            let trusted_header = parts
                .headers
                .get("x-escudo-ops-password")
                .and_then(|v| v.to_str().ok());
            if trusted_header == Some(proxy_password.as_str()) {
                return Ok(AdminUser(Claims {
                    sub: uuid::Uuid::nil(),
                    email: "nginx-dashboard-proxy@escudo.local".into(),
                    role: Role::Admin,
                    exp: i64::MAX,
                    iat: 0,
                }));
            }
        }

        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| EscudoError::Unauthorized("Missing authorization header".into()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| EscudoError::Unauthorized("Invalid authorization format".into()))?;

        let claims = decode_jwt(token, &state.jwt_secret)?;

        if claims.role != Role::Admin {
            return Err(EscudoError::Forbidden("Admin access required".into()));
        }

        Ok(AdminUser(claims))
    }
}
