use anyhow::bail;
use async_trait::async_trait;

use crate::credential::ProxyCredential;
use crate::provider::{DedicatedProxyRequest, ProxyProvider, SharedProxyRequest};

/// Stub implementation of ProxyProvider for ProxyCheap.
///
/// This is a placeholder that returns errors for all methods that would
/// require a real API integration.  Replace with a full implementation
/// once ProxyCheap credentials are available.
pub struct ProxycheapClient;

impl ProxycheapClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProxycheapClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProxyProvider for ProxycheapClient {
    async fn acquire_shared_proxy(
        &self,
        _request: SharedProxyRequest,
    ) -> anyhow::Result<ProxyCredential> {
        bail!("ProxyCheap integration is not yet implemented");
    }

    async fn acquire_dedicated_ip(
        &self,
        _request: DedicatedProxyRequest,
    ) -> anyhow::Result<ProxyCredential> {
        bail!("ProxyCheap integration is not yet implemented");
    }

    async fn release_proxy(&self, _credential: &ProxyCredential) -> anyhow::Result<()> {
        Ok(())
    }

    async fn rotate_proxy(&self, _credential: &ProxyCredential) -> anyhow::Result<ProxyCredential> {
        bail!("ProxyCheap integration is not yet implemented");
    }

    async fn list_proxies(&self) -> anyhow::Result<Vec<ProxyCredential>> {
        Ok(vec![])
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(false)
    }
}
