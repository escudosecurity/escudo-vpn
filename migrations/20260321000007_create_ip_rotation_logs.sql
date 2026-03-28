CREATE TABLE IF NOT EXISTS ip_rotation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    old_proxy_ip_id UUID NOT NULL,
    new_proxy_ip_id UUID NOT NULL,
    reason TEXT NOT NULL,
    country TEXT NOT NULL,
    provider TEXT NOT NULL,
    affected_servers INTEGER DEFAULT 0,
    affected_customers INTEGER DEFAULT 0,
    rotated_at TIMESTAMPTZ DEFAULT now()
);
