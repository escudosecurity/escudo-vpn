CREATE TABLE IF NOT EXISTS ip_health_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    service TEXT NOT NULL,
    status TEXT NOT NULL,
    response_time_ms INTEGER,
    error_detail TEXT,
    checked_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_health_logs_proxy_checked ON ip_health_logs(proxy_ip_id, checked_at);
CREATE INDEX IF NOT EXISTS idx_health_logs_service ON ip_health_logs(service, checked_at);
