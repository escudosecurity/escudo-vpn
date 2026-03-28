CREATE TABLE IF NOT EXISTS provider_servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID UNIQUE REFERENCES servers(id),
    provider TEXT NOT NULL,
    provider_instance_id TEXT NOT NULL,
    label TEXT NOT NULL UNIQUE,
    region TEXT NOT NULL,
    plan TEXT NOT NULL,
    public_ip TEXT,
    status TEXT NOT NULL DEFAULT 'provisioning',
    gateway_version TEXT,
    last_heartbeat TIMESTAMPTZ,
    monthly_cost_usd DECIMAL(8,2),
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(provider, provider_instance_id)
);
CREATE INDEX IF NOT EXISTS idx_provider_servers_status ON provider_servers(status);
