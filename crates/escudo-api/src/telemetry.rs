use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;

#[derive(Debug, Clone, Default)]
pub struct ClientTelemetry {
    pub ip: Option<IpAddr>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub user_agent: Option<String>,
    pub inferred_platform: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct IpWhoisResponse {
    success: bool,
    ip: Option<String>,
    country: Option<String>,
    #[serde(rename = "country_code")]
    country_code: Option<String>,
}

pub async fn resolve_request_telemetry(
    headers: &HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> ClientTelemetry {
    let ip = extract_client_ip(headers, connect_info);
    let user_agent = header_string(headers, "user-agent");
    let mut country = country_header(headers);
    let mut country_code = country_code_header(headers);

    if (country.is_none() || country_code.is_none()) && ip.is_some_and(is_public_ip) {
        if let Some(geo) = lookup_ip_location(ip.unwrap()).await {
            country = country.or(geo.country);
            country_code = country_code.or(geo.country_code);
        }
    }

    let inferred_platform = infer_platform(user_agent.as_deref());

    ClientTelemetry {
        ip,
        country,
        country_code,
        user_agent,
        inferred_platform,
    }
}

pub fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> Option<IpAddr> {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .and_then(|value| value.parse::<IpAddr>().ok());

    let real_ip = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<IpAddr>().ok());

    forwarded
        .or(real_ip)
        .or_else(|| connect_info.map(|info| info.0.ip()))
}

pub fn infer_platform(user_agent: Option<&str>) -> Option<String> {
    let ua = user_agent?.trim().to_ascii_lowercase();
    if ua.is_empty() {
        return None;
    }

    let platform = if ua.contains("android") {
        "android"
    } else if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ios") {
        "ios"
    } else if ua.contains("windows") {
        "windows"
    } else if ua.contains("mac os") || ua.contains("macintosh") {
        "macos"
    } else if ua.contains("linux") {
        "linux"
    } else {
        "unknown"
    };

    Some(platform.to_string())
}

pub fn normalize_country(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn country_header(headers: &HeaderMap) -> Option<String> {
    header_string(headers, "x-country-name")
        .or_else(|| header_string(headers, "cf-ipcountry"))
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.len() == 2 {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
}

fn country_code_header(headers: &HeaderMap) -> Option<String> {
    header_string(headers, "cf-ipcountry")
        .or_else(|| header_string(headers, "x-country-code"))
        .or_else(|| header_string(headers, "x-vercel-ip-country"))
        .map(|value| value.trim().to_uppercase())
        .filter(|value| value.len() == 2 && value != "XX")
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            !ipv4.is_private()
                && !ipv4.is_loopback()
                && !ipv4.is_link_local()
                && !ipv4.is_multicast()
                && !ipv4.is_broadcast()
                && !ipv4.is_documentation()
                && !ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            !ipv6.is_loopback()
                && !ipv6.is_unspecified()
                && !ipv6.is_multicast()
                && !ipv6.is_unique_local()
        }
    }
}

async fn lookup_ip_location(ip: IpAddr) -> Option<IpWhoisResponse> {
    let url = format!("https://ipwho.is/{ip}");
    let client = reqwest::Client::builder().build().ok()?;
    let response = client.get(url).send().await.ok()?;
    let body = response.json::<IpWhoisResponse>().await.ok()?;
    body.success.then_some(body)
}
