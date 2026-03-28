use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Admin,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub role: Role,
    pub exp: i64,
    pub iat: i64,
}

pub fn encode_jwt(
    user_id: Uuid,
    email: &str,
    role: Role,
    secret: &str,
    expiration_hours: i64,
) -> crate::error::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        role,
        exp: (now + Duration::hours(expiration_hours)).timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn decode_jwt(token: &str, secret: &str) -> crate::error::Result<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation
        .required_spec_claims
        .extend(["exp".into(), "iat".into(), "sub".into()]);
    validation.validate_exp = true;
    validation.leeway = 5;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let user_id = Uuid::new_v4();
        let secret = "test-secret-key-for-jwt-testing";
        let token = encode_jwt(user_id, "test@example.com", Role::User, secret, 1).unwrap();
        let claims = decode_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, Role::User);
    }

    #[test]
    fn test_admin_role_roundtrip() {
        let user_id = Uuid::new_v4();
        let secret = "admin-secret";
        let token = encode_jwt(user_id, "admin@example.com", Role::Admin, secret, 1).unwrap();
        let claims = decode_jwt(&token, secret).unwrap();
        assert_eq!(claims.role, Role::Admin);
    }

    #[test]
    fn test_wrong_secret_fails() {
        let token = encode_jwt(Uuid::new_v4(), "x@y.com", Role::User, "secret1", 1).unwrap();
        assert!(decode_jwt(&token, "secret2").is_err());
    }

    #[test]
    fn test_expired_token_fails() {
        let token = encode_jwt(Uuid::new_v4(), "x@y.com", Role::User, "sec", -1).unwrap();
        assert!(decode_jwt(&token, "sec").is_err());
    }

    #[test]
    fn test_garbage_token_fails() {
        assert!(decode_jwt("not.a.jwt", "secret").is_err());
    }
}
