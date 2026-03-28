CREATE TABLE server_chains (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    entry_server_id UUID NOT NULL REFERENCES servers(id),
    exit_server_id UUID NOT NULL REFERENCES servers(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_different_servers CHECK (entry_server_id != exit_server_id)
);

CREATE INDEX idx_server_chains_active ON server_chains(is_active) WHERE is_active = true;
