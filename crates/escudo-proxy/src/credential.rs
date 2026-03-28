use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Which upstream proxy provider supplies this credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Iproyal,
    Proxycheap,
}

/// Whether the proxy is a rotating/shared residential IP or a static dedicated IP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyType {
    /// Rotating residential proxy with an optional sticky session.
    Shared,
    /// Static dedicated IP proxy.
    Dedicated,
}

/// All information needed to connect through an upstream proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCredential {
    /// Unique identifier for this credential (useful for pool tracking).
    pub id: Uuid,
    /// Which provider issued this credential.
    pub provider: ProviderKind,
    /// Shared (rotating) or Dedicated (static) IP.
    pub proxy_type: ProxyType,
    /// ISO 3166-1 alpha-2 country code, e.g. "BR".
    pub country: String,
    /// SOCKS5 proxy host.
    pub host: String,
    /// SOCKS5 proxy port.
    pub port: u16,
    /// Proxy username (may encode session/country in it for IPRoyal).
    pub username: String,
    /// Proxy password / API token.
    pub password: String,
    /// When this credential was issued.
    pub issued_at: DateTime<Utc>,
    /// When this credential expires, if known.
    pub expires_at: Option<DateTime<Utc>>,
}

impl ProxyCredential {
    /// Create a new credential with a freshly generated UUID and current timestamp.
    pub fn new(
        provider: ProviderKind,
        proxy_type: ProxyType,
        country: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            provider,
            proxy_type,
            country: country.into(),
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            issued_at: Utc::now(),
            expires_at,
        }
    }

    /// Format as a SOCKS5 URL: `socks5://username:password@host:port`
    pub fn socks5_url(&self) -> String {
        format!(
            "socks5://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }

    /// Return true if this credential has a known expiry and it has passed.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() >= expiry,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socks5_url_format() {
        let cred = ProxyCredential::new(
            ProviderKind::Iproyal,
            ProxyType::Shared,
            "BR",
            "geo.iproyal.com",
            32325,
            "user_token__country-br__session-abc__lifetime-60m",
            "secret",
            None,
        );
        assert_eq!(
            cred.socks5_url(),
            "socks5://user_token__country-br__session-abc__lifetime-60m:secret@geo.iproyal.com:32325"
        );
    }

    #[test]
    fn not_expired_when_no_expiry() {
        let cred = ProxyCredential::new(
            ProviderKind::Iproyal,
            ProxyType::Shared,
            "BR",
            "geo.iproyal.com",
            32325,
            "user",
            "pass",
            None,
        );
        assert!(!cred.is_expired());
    }
}
