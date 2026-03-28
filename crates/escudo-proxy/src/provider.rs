use anyhow::Result;
use async_trait::async_trait;

use crate::credential::ProxyCredential;

/// Options for acquiring a shared (rotating) residential proxy.
#[derive(Debug, Clone)]
pub struct SharedProxyRequest {
    /// ISO 3166-1 alpha-2 country code, e.g. "BR".
    pub country: String,
    /// How long the sticky session should last, in minutes.
    /// Pass `None` for a fully rotating (non-sticky) proxy.
    pub sticky_duration_mins: Option<u64>,
}

/// Options for acquiring a dedicated (static) IP proxy.
#[derive(Debug, Clone)]
pub struct DedicatedProxyRequest {
    /// ISO 3166-1 alpha-2 country code, e.g. "BR".
    pub country: String,
}

/// Core abstraction over an upstream residential proxy provider.
///
/// All methods are async and must be object-safe (hence `async_trait`).
#[async_trait]
pub trait ProxyProvider: Send + Sync {
    /// Acquire a shared (rotating / sticky-session) residential proxy for the given country.
    async fn acquire_shared_proxy(&self, request: SharedProxyRequest) -> Result<ProxyCredential>;

    /// Acquire a dedicated (static) IP proxy for the given country.
    async fn acquire_dedicated_ip(&self, request: DedicatedProxyRequest)
        -> Result<ProxyCredential>;

    /// Release / return a previously acquired proxy credential.
    ///
    /// For shared rotating proxies this is typically a no-op, but dedicated IPs
    /// may need to be explicitly deallocated on the provider side.
    async fn release_proxy(&self, credential: &ProxyCredential) -> Result<()>;

    /// Rotate the proxy associated with the given credential ID, returning a
    /// fresh credential.
    ///
    /// Providers that embed rotation context in the username (e.g. IPRoyal) may
    /// return an error directing the caller to use `ProxyPool::acquire_shared`
    /// with the correct country instead.
    async fn rotate_proxy(&self, credential: &ProxyCredential) -> Result<ProxyCredential>;

    /// List all proxies currently available / allocated on this provider account.
    async fn list_proxies(&self) -> Result<Vec<ProxyCredential>>;

    /// Perform a lightweight health check against the provider API.
    ///
    /// Returns `true` if the provider is reachable and the API token is valid.
    async fn health_check(&self) -> Result<bool>;
}
