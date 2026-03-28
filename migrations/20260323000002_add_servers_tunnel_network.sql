ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS tunnel_ipv4_cidr TEXT,
    ADD COLUMN IF NOT EXISTS tunnel_ipv4_gateway TEXT;
