use anyhow::Result;
use tracing::{info, warn};

use crate::credential::ProxyCredential;
use crate::provider::{DedicatedProxyRequest, ProxyProvider, SharedProxyRequest};

/// A pool that wraps a primary and an optional fallback [`ProxyProvider`].
///
/// All acquire / rotate operations attempt the primary first and
/// automatically fall back to the secondary provider on failure.
pub struct ProxyPool {
    primary: Box<dyn ProxyProvider>,
    fallback: Option<Box<dyn ProxyProvider>>,
}

impl ProxyPool {
    /// Create a pool with only a primary provider.
    pub fn new(primary: impl ProxyProvider + 'static) -> Self {
        Self {
            primary: Box::new(primary),
            fallback: None,
        }
    }

    /// Create a pool with a primary and a fallback provider.
    pub fn with_fallback(
        primary: impl ProxyProvider + 'static,
        fallback: impl ProxyProvider + 'static,
    ) -> Self {
        Self {
            primary: Box::new(primary),
            fallback: Some(Box::new(fallback)),
        }
    }

    /// Acquire a shared (rotating / sticky-session) proxy.
    ///
    /// Tries the primary provider first; on failure, tries the fallback if
    /// one is configured.
    pub async fn acquire_shared(&self, request: SharedProxyRequest) -> Result<ProxyCredential> {
        match self.primary.acquire_shared_proxy(request.clone()).await {
            Ok(cred) => {
                info!(
                    country = %request.country,
                    provider = "primary",
                    "acquired shared proxy"
                );
                Ok(cred)
            }
            Err(primary_err) => {
                warn!(
                    error = %primary_err,
                    "primary provider failed for acquire_shared; trying fallback"
                );
                match &self.fallback {
                    Some(fb) => {
                        let cred = fb.acquire_shared_proxy(request.clone()).await?;
                        info!(
                            country = %request.country,
                            provider = "fallback",
                            "acquired shared proxy via fallback"
                        );
                        Ok(cred)
                    }
                    None => Err(primary_err),
                }
            }
        }
    }

    /// Acquire a dedicated (static) IP proxy.
    ///
    /// Tries the primary provider first; on failure, tries the fallback if
    /// one is configured.
    pub async fn acquire_dedicated(
        &self,
        request: DedicatedProxyRequest,
    ) -> Result<ProxyCredential> {
        match self.primary.acquire_dedicated_ip(request.clone()).await {
            Ok(cred) => {
                info!(
                    country = %request.country,
                    provider = "primary",
                    "acquired dedicated IP"
                );
                Ok(cred)
            }
            Err(primary_err) => {
                warn!(
                    error = %primary_err,
                    "primary provider failed for acquire_dedicated; trying fallback"
                );
                match &self.fallback {
                    Some(fb) => {
                        let cred = fb.acquire_dedicated_ip(request.clone()).await?;
                        info!(
                            country = %request.country,
                            provider = "fallback",
                            "acquired dedicated IP via fallback"
                        );
                        Ok(cred)
                    }
                    None => Err(primary_err),
                }
            }
        }
    }

    /// Rotate the given proxy credential via the primary provider.
    ///
    /// Note: for IPRoyal this will return an error — use `acquire_shared`
    /// directly instead.
    pub async fn rotate(&self, credential: &ProxyCredential) -> Result<ProxyCredential> {
        self.primary.rotate_proxy(credential).await
    }

    /// Health-check both the primary and fallback providers.
    ///
    /// Returns `true` if the primary is healthy.  A fallback failure is
    /// logged as a warning but does not affect the return value.
    pub async fn validate_providers(&self) -> Result<bool> {
        let primary_ok = self.primary.health_check().await?;

        if primary_ok {
            info!("primary proxy provider is healthy");
        } else {
            warn!("primary proxy provider health check failed");
        }

        if let Some(fb) = &self.fallback {
            match fb.health_check().await {
                Ok(true) => info!("fallback proxy provider is healthy"),
                Ok(false) => warn!("fallback proxy provider health check returned false"),
                Err(e) => warn!(error = %e, "fallback proxy provider health check errored"),
            }
        }

        Ok(primary_ok)
    }
}
