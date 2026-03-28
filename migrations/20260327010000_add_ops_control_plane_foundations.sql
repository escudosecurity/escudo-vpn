ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS lifecycle_state TEXT NOT NULL DEFAULT 'healthy',
    ADD COLUMN IF NOT EXISTS assigned_user_cap INTEGER NOT NULL DEFAULT 350,
    ADD COLUMN IF NOT EXISTS active_session_soft_cap INTEGER NOT NULL DEFAULT 280,
    ADD COLUMN IF NOT EXISTS active_session_hard_cap INTEGER NOT NULL DEFAULT 350,
    ADD COLUMN IF NOT EXISTS throughput_soft_cap_mbps INTEGER,
    ADD COLUMN IF NOT EXISTS throughput_hard_cap_mbps INTEGER,
    ADD COLUMN IF NOT EXISTS routing_weight DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    ADD COLUMN IF NOT EXISTS health_score INTEGER NOT NULL DEFAULT 100,
    ADD COLUMN IF NOT EXISTS health_reasons JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS last_health_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_servers_lifecycle_state ON servers(lifecycle_state);
CREATE INDEX IF NOT EXISTS idx_servers_health_score ON servers(health_score);

CREATE TABLE IF NOT EXISTS node_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    cpu_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
    ram_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
    nic_in_mbps DOUBLE PRECISION NOT NULL DEFAULT 0,
    nic_out_mbps DOUBLE PRECISION NOT NULL DEFAULT 0,
    active_sessions INTEGER NOT NULL DEFAULT 0,
    assigned_users INTEGER NOT NULL DEFAULT 0,
    connect_success_pct DOUBLE PRECISION NOT NULL DEFAULT 100,
    median_connect_ms INTEGER NOT NULL DEFAULT 0,
    health_score INTEGER NOT NULL DEFAULT 100,
    health_state TEXT NOT NULL DEFAULT 'healthy',
    collected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_node_metrics_server_id_collected_at
    ON node_metrics(server_id, collected_at DESC);

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS signup_ip INET,
    ADD COLUMN IF NOT EXISTS signup_country TEXT,
    ADD COLUMN IF NOT EXISTS latest_login_ip INET,
    ADD COLUMN IF NOT EXISTS latest_login_country TEXT,
    ADD COLUMN IF NOT EXISTS abuse_score INTEGER NOT NULL DEFAULT 0;

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS device_install_id TEXT,
    ADD COLUMN IF NOT EXISTS platform TEXT,
    ADD COLUMN IF NOT EXISTS current_active_sessions INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS usage_bucket TEXT NOT NULL DEFAULT 'normal',
    ADD COLUMN IF NOT EXISTS preferred_class TEXT,
    ADD COLUMN IF NOT EXISTS dedicated_required BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS sensitive_route BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE INDEX IF NOT EXISTS idx_devices_device_install_id ON devices(device_install_id);
CREATE INDEX IF NOT EXISTS idx_devices_usage_bucket ON devices(usage_bucket);
