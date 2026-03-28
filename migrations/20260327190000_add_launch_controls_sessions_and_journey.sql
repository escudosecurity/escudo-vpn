CREATE TABLE IF NOT EXISTS launch_controls (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    maintenance_mode BOOLEAN NOT NULL DEFAULT FALSE,
    allow_public_signup BOOLEAN NOT NULL DEFAULT TRUE,
    allow_anonymous_signup BOOLEAN NOT NULL DEFAULT TRUE,
    allow_connect BOOLEAN NOT NULL DEFAULT TRUE,
    allow_paid_checkout BOOLEAN NOT NULL DEFAULT FALSE,
    healthy_only_routing BOOLEAN NOT NULL DEFAULT TRUE,
    expose_paid_tiers BOOLEAN NOT NULL DEFAULT FALSE,
    free_beta_label TEXT NOT NULL DEFAULT 'free-beta',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO launch_controls (singleton)
VALUES (TRUE)
ON CONFLICT (singleton) DO NOTHING;

CREATE TABLE IF NOT EXISTS invite_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL UNIQUE,
    tier TEXT NOT NULL DEFAULT 'free',
    plan TEXT NOT NULL DEFAULT 'free',
    duration_days INTEGER NOT NULL DEFAULT 30,
    max_uses INTEGER NOT NULL DEFAULT 1,
    used_count INTEGER NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    cohort TEXT,
    notes TEXT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_invite_codes_active ON invite_codes(active, expires_at);

CREATE TABLE IF NOT EXISTS invite_code_redemptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invite_code_id UUID NOT NULL REFERENCES invite_codes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redeemed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (invite_code_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_invite_code_redemptions_user_id ON invite_code_redemptions(user_id, redeemed_at DESC);

CREATE TABLE IF NOT EXISTS vpn_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    tier TEXT NOT NULL DEFAULT 'free',
    connect_country TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    disconnect_reason TEXT,
    bytes_in BIGINT NOT NULL DEFAULT 0,
    bytes_out BIGINT NOT NULL DEFAULT 0,
    session_metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_vpn_sessions_user_id_started_at ON vpn_sessions(user_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_vpn_sessions_device_id_started_at ON vpn_sessions(device_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_vpn_sessions_server_id_started_at ON vpn_sessions(server_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_vpn_sessions_active ON vpn_sessions(device_id) WHERE ended_at IS NULL;

CREATE TABLE IF NOT EXISTS journey_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    server_id UUID REFERENCES servers(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    outcome TEXT NOT NULL DEFAULT 'success',
    detail TEXT,
    event_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_journey_events_created_at ON journey_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_journey_events_user_id_created_at ON journey_events(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_journey_events_event_type_created_at ON journey_events(event_type, created_at DESC);
