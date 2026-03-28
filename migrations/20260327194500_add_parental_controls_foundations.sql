CREATE TABLE IF NOT EXISTS family_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL DEFAULT 'Default',
    block_porn BOOLEAN NOT NULL DEFAULT TRUE,
    block_gambling BOOLEAN NOT NULL DEFAULT TRUE,
    block_social_media BOOLEAN NOT NULL DEFAULT FALSE,
    block_malware BOOLEAN NOT NULL DEFAULT TRUE,
    block_drugs BOOLEAN NOT NULL DEFAULT TRUE,
    block_violence BOOLEAN NOT NULL DEFAULT FALSE,
    block_dating BOOLEAN NOT NULL DEFAULT FALSE,
    block_gaming BOOLEAN NOT NULL DEFAULT FALSE,
    custom_blocked_domains TEXT[] DEFAULT '{}',
    custom_allowed_domains TEXT[] DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_family_profiles_user ON family_profiles(user_id);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'family_profiles_user_id_key'
    ) THEN
        ALTER TABLE family_profiles
            ADD CONSTRAINT family_profiles_user_id_key UNIQUE (user_id);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS parental_children (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    child_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    access_code VARCHAR(16) NOT NULL UNIQUE,
    tier TEXT NOT NULL DEFAULT 'family',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    linked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_parental_children_parent_user_id
    ON parental_children(parent_user_id, created_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_parental_children_child_user_id
    ON parental_children(child_user_id)
    WHERE child_user_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS parental_child_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_id UUID NOT NULL REFERENCES parental_children(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    device_install_id TEXT,
    display_name TEXT NOT NULL DEFAULT 'Child Device',
    platform TEXT,
    notes TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    linked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_parental_child_devices_child_id
    ON parental_child_devices(child_id, linked_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_parental_child_devices_device_id
    ON parental_child_devices(device_id)
    WHERE device_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS parental_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_id UUID NOT NULL REFERENCES parental_children(id) ON DELETE CASCADE,
    target_device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    block_tiktok BOOLEAN NOT NULL DEFAULT FALSE,
    block_youtube BOOLEAN NOT NULL DEFAULT FALSE,
    block_social_media BOOLEAN NOT NULL DEFAULT FALSE,
    block_streaming BOOLEAN NOT NULL DEFAULT FALSE,
    bedtime_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    bedtime_start_minute INTEGER,
    bedtime_end_minute INTEGER,
    max_daily_minutes INTEGER,
    monitored_apps TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    blocked_apps TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    custom_blocked_domains TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    custom_allowed_domains TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_parental_policies_child_id
    ON parental_policies(child_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS parental_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_id UUID NOT NULL REFERENCES parental_children(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    days_of_week INTEGER[] NOT NULL DEFAULT ARRAY[1,2,3,4,5,6,7],
    start_minute INTEGER NOT NULL,
    end_minute INTEGER NOT NULL,
    blocked_categories TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    blocked_apps TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_parental_schedules_child_id
    ON parental_schedules(child_id, created_at DESC);

CREATE TABLE IF NOT EXISTS parental_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_id UUID NOT NULL REFERENCES parental_children(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    app_identifier TEXT,
    domain TEXT,
    action TEXT,
    detail TEXT,
    event_metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_parental_events_child_id
    ON parental_events(child_id, occurred_at DESC);
