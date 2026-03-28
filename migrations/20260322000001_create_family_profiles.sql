CREATE TABLE IF NOT EXISTS family_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL DEFAULT 'Default',
    block_porn BOOLEAN NOT NULL DEFAULT true,
    block_gambling BOOLEAN NOT NULL DEFAULT true,
    block_social_media BOOLEAN NOT NULL DEFAULT false,
    block_malware BOOLEAN NOT NULL DEFAULT true,
    block_drugs BOOLEAN NOT NULL DEFAULT true,
    block_violence BOOLEAN NOT NULL DEFAULT false,
    block_dating BOOLEAN NOT NULL DEFAULT false,
    block_gaming BOOLEAN NOT NULL DEFAULT false,
    custom_blocked_domains TEXT[] DEFAULT '{}',
    custom_allowed_domains TEXT[] DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_family_profiles_user ON family_profiles(user_id);

CREATE TABLE IF NOT EXISTS breach_monitors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    email TEXT NOT NULL,
    last_checked TIMESTAMPTZ,
    breach_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_breach_monitors_user_email ON breach_monitors(user_id, email);

CREATE TABLE IF NOT EXISTS breach_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    monitor_id UUID NOT NULL REFERENCES breach_monitors(id),
    breach_name TEXT NOT NULL,
    breach_date TEXT,
    breach_description TEXT,
    data_types TEXT[],
    first_seen TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_breach_results_monitor ON breach_results(monitor_id);
