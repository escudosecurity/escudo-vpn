CREATE TABLE IF NOT EXISTS server_proxy_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    proxy_target TEXT NOT NULL DEFAULT 'shared',
    assigned_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(server_id, proxy_target)
);
