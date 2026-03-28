CREATE TABLE IF NOT EXISTS dns_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_ip VARCHAR(18) NOT NULL,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    queries_total BIGINT NOT NULL DEFAULT 0,
    blocked_total BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(client_ip, date)
);

CREATE INDEX IF NOT EXISTS idx_dns_stats_client_ip ON dns_stats(client_ip);
CREATE INDEX IF NOT EXISTS idx_dns_stats_date ON dns_stats(date);
