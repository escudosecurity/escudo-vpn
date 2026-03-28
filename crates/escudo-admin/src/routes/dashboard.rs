use axum::response::Html;

const DASHBOARD_HTML: &str = include_str!("../../static/dashboard.html");

pub async fn dashboard_page() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}
