CREATE TABLE IF NOT EXISTS ip_health_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    netflix_status INTEGER,
    regional_status INTEGER,
    regional_service TEXT,
    latency_ms INTEGER,
    status TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ip_health_checks_proxy_checked
    ON ip_health_checks(proxy_ip_id, checked_at DESC);
