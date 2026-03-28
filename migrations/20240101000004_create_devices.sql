CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id),
    name VARCHAR(100) NOT NULL DEFAULT 'default',
    public_key VARCHAR(44) NOT NULL UNIQUE,
    preshared_key VARCHAR(44) NOT NULL,
    assigned_ip VARCHAR(15) NOT NULL UNIQUE,
    private_key_encrypted VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_devices_user_id ON devices(user_id);
CREATE INDEX IF NOT EXISTS idx_devices_server_id ON devices(server_id);
CREATE INDEX IF NOT EXISTS idx_devices_assigned_ip ON devices(assigned_ip);
