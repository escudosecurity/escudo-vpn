use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::credential::{ProviderKind, ProxyCredential, ProxyType};
use crate::provider::{DedicatedProxyRequest, ProxyProvider, SharedProxyRequest};

const BASE_URL: &str = "https://apid.iproyal.com/v1/reseller";
const ISP_PRODUCT_NAME: &str = "ISP";
const ISP_PRODUCT_ID: i64 = 9;
const ISP_PLAN_ID_30_DAYS: i64 = 22;
const SOCKS5_PORT: u16 = 12324;

#[derive(Debug, Deserialize)]
struct ProductsResponse {
    data: Vec<Product>,
}

#[derive(Debug, Deserialize)]
struct Product {
    id: i64,
    name: String,
    plans: Vec<Plan>,
    locations: Vec<Location>,
}

#[derive(Debug, Deserialize)]
struct Plan {
    id: i64,
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Location {
    id: i64,
    name: String,
    out_of_stock: bool,
    #[serde(default)]
    child_locations: Vec<Location>,
}

#[derive(Debug, Deserialize)]
struct OrderListResponse {
    data: Vec<Order>,
}

#[derive(Debug, Deserialize)]
struct Order {
    id: i64,
    status: String,
    location: Option<String>,
    #[serde(default)]
    proxy_data: ProxyData,
}

#[derive(Debug, Deserialize, Default)]
struct ProxyData {
    #[serde(default)]
    ports: ProxyPorts,
    #[serde(default)]
    proxies: Vec<OrderProxy>,
}

#[derive(Debug, Deserialize, Default)]
struct ProxyPorts {
    socks5: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
struct OrderProxy {
    username: String,
    password: String,
    ip: String,
}

#[derive(Debug, Deserialize)]
struct CreateOrderResponse {
    id: i64,
    status: String,
}

#[derive(Debug, serde::Serialize)]
struct CreateOrderRequest {
    product_id: i64,
    product_plan_id: i64,
    product_location_id: i64,
    quantity: i64,
    auto_extend: bool,
}

pub struct IproyalClient {
    api_token: String,
    http: reqwest::Client,
}

impl IproyalClient {
    pub fn new(api_token: impl Into<String>) -> Result<Self> {
        let api_token = api_token.into();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Access-Token",
            HeaderValue::from_str(&api_token).context("invalid IPRoyal ISP access token")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build reqwest client")?;
        Ok(Self { api_token, http })
    }

    async fn load_isp_product(&self) -> Result<Product> {
        let url = format!("{BASE_URL}/products");
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("IPRoyal ISP products request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("IPRoyal ISP products error {}: {}", status, body);
        }

        let products: ProductsResponse = resp
            .json()
            .await
            .context("failed to parse IPRoyal ISP products response")?;

        products
            .data
            .into_iter()
            .find(|p| p.id == ISP_PRODUCT_ID || p.name == ISP_PRODUCT_NAME)
            .context("IPRoyal ISP product was not available on this account")
    }

    fn select_location(product: &Product, country: &str) -> Result<i64> {
        let country_name = country_code_to_name(country);
        let location = product
            .locations
            .iter()
            .find(|loc| loc.name.eq_ignore_ascii_case(&country_name))
            .with_context(|| format!("IPRoyal ISP country {country_name} is not available"))?;

        if !location.out_of_stock {
            return Ok(location.id);
        }

        location
            .child_locations
            .iter()
            .find(|child| !child.out_of_stock)
            .map(|child| child.id)
            .with_context(|| {
                format!("IPRoyal ISP country {country_name} is out of stock and has no live child locations")
            })
    }

    fn select_plan(product: &Product) -> Result<i64> {
        product
            .plans
            .iter()
            .find(|plan| plan.id == ISP_PLAN_ID_30_DAYS || plan.name == "30 Days")
            .map(|plan| plan.id)
            .context("IPRoyal ISP 30 Days plan was not available")
    }

    async fn create_isp_order(&self, country: &str) -> Result<CreateOrderResponse> {
        let product = self.load_isp_product().await?;
        let plan_id = Self::select_plan(&product)?;
        let location_id = Self::select_location(&product, country)?;
        let url = format!("{BASE_URL}/orders");
        let request = CreateOrderRequest {
            product_id: product.id,
            product_plan_id: plan_id,
            product_location_id: location_id,
            quantity: 1,
            auto_extend: true,
        };

        let resp = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("IPRoyal ISP create order request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("IPRoyal ISP create order error {}: {}", status, body);
        }

        resp.json()
            .await
            .context("failed to parse IPRoyal ISP create order response")
    }

    async fn wait_for_order_proxy(
        &self,
        order_id: i64,
        expected_country: &str,
        proxy_type: ProxyType,
    ) -> Result<ProxyCredential> {
        let url = format!("{BASE_URL}/orders/{order_id}");
        for attempt in 1..=30 {
            let resp = self
                .http
                .get(&url)
                .send()
                .await
                .with_context(|| format!("failed to poll IPRoyal ISP order {order_id}"))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("IPRoyal ISP order poll error {}: {}", status, body);
            }

            let order: Order = resp
                .json()
                .await
                .with_context(|| format!("failed to parse IPRoyal ISP order {order_id}"))?;

            if order.status.eq_ignore_ascii_case("confirmed") {
                if let Some(proxy) = order.proxy_data.proxies.into_iter().next() {
                    let mut credential = ProxyCredential::new(
                        ProviderKind::Iproyal,
                        proxy_type,
                        expected_country.to_uppercase(),
                        proxy.ip,
                        order.proxy_data.ports.socks5.unwrap_or(SOCKS5_PORT),
                        proxy.username,
                        proxy.password,
                        None,
                    );
                    credential.id = Uuid::new_v4();
                    debug!(order_id, attempt, host = %credential.host, "IPRoyal ISP order confirmed");
                    return Ok(credential);
                }
            }

            if order.status.eq_ignore_ascii_case("unpaid") {
                bail!("IPRoyal ISP order {order_id} is unpaid");
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        bail!("IPRoyal ISP order {order_id} did not issue a proxy within the timeout")
    }
}

#[async_trait]
impl ProxyProvider for IproyalClient {
    async fn acquire_shared_proxy(&self, request: SharedProxyRequest) -> Result<ProxyCredential> {
        let order = self.create_isp_order(&request.country).await?;
        self.wait_for_order_proxy(order.id, &request.country, ProxyType::Shared)
            .await
    }

    async fn acquire_dedicated_ip(
        &self,
        request: DedicatedProxyRequest,
    ) -> Result<ProxyCredential> {
        let order = self.create_isp_order(&request.country).await?;
        self.wait_for_order_proxy(order.id, &request.country, ProxyType::Dedicated)
            .await
    }

    async fn release_proxy(&self, _credential: &ProxyCredential) -> Result<()> {
        Ok(())
    }

    async fn rotate_proxy(&self, credential: &ProxyCredential) -> Result<ProxyCredential> {
        let request = SharedProxyRequest {
            country: credential.country.clone(),
            sticky_duration_mins: None,
        };
        self.acquire_shared_proxy(request).await
    }

    async fn list_proxies(&self) -> Result<Vec<ProxyCredential>> {
        let url = format!(
            "{BASE_URL}/orders?product_id={}&page=1&per_page=100",
            ISP_PRODUCT_ID
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("IPRoyal ISP list orders request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("IPRoyal ISP list orders error {}: {}", status, body);
        }

        let orders: OrderListResponse = resp
            .json()
            .await
            .context("failed to parse IPRoyal ISP orders response")?;

        let mut credentials = Vec::new();
        for order in orders.data {
            if !order.status.eq_ignore_ascii_case("confirmed") {
                continue;
            }
            let country = order
                .location
                .as_deref()
                .map(country_name_to_code)
                .unwrap_or_else(|| "XX".to_string());
            let port = order.proxy_data.ports.socks5.unwrap_or(SOCKS5_PORT);
            for proxy in order.proxy_data.proxies {
                let mut credential = ProxyCredential::new(
                    ProviderKind::Iproyal,
                    ProxyType::Dedicated,
                    country.clone(),
                    proxy.ip,
                    port,
                    proxy.username,
                    proxy.password,
                    None,
                );
                credential.id = Uuid::new_v4();
                credentials.push(credential);
            }
        }

        Ok(credentials)
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{BASE_URL}/products");
        let resp = self.http.get(&url).send().await;
        match resp {
            Ok(response) => {
                let ok = response.status().is_success();
                if !ok {
                    warn!(status = %response.status(), "IPRoyal ISP health check returned non-200");
                }
                Ok(ok)
            }
            Err(error) => {
                warn!(%error, "IPRoyal ISP health check failed");
                Ok(false)
            }
        }
    }
}

fn country_code_to_name(country: &str) -> String {
    match country.to_uppercase().as_str() {
        "BR" => "Brazil",
        "US" => "United States",
        "GB" | "UK" => "United Kingdom",
        "DE" => "Germany",
        "CA" => "Canada",
        other => other,
    }
    .to_string()
}

fn country_name_to_code(country: &str) -> String {
    match country {
        "Brazil" => "BR",
        "United States" => "US",
        "United Kingdom" => "GB",
        "Germany" => "DE",
        "Canada" => "CA",
        _ => "XX",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_country_code_to_name() {
        assert_eq!(country_code_to_name("BR"), "Brazil");
        assert_eq!(country_code_to_name("US"), "United States");
    }

    #[test]
    fn maps_country_name_to_code() {
        assert_eq!(country_name_to_code("Brazil"), "BR");
        assert_eq!(country_name_to_code("United States"), "US");
    }
}
