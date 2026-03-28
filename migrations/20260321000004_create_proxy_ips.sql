CREATE TABLE IF NOT EXISTS proxy_ips (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL,
    provider_proxy_id TEXT NOT NULL,
    proxy_type TEXT NOT NULL,
    country TEXT NOT NULL,
    city TEXT,
    socks5_host TEXT NOT NULL,
    socks5_port INTEGER NOT NULL,
    socks5_username TEXT NOT NULL,
    socks5_password TEXT NOT NULL,
    external_ip TEXT,
    status TEXT NOT NULL DEFAULT 'healthy',
    assigned_user_id UUID REFERENCES users(id),
    max_concurrent INTEGER DEFAULT 4,
    current_concurrent INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    last_health_check TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_proxy_ips_country_status ON proxy_ips(country, status);
CREATE INDEX IF NOT EXISTS idx_proxy_ips_assigned_user ON proxy_ips(assigned_user_id) WHERE assigned_user_id IS NOT NULL;
