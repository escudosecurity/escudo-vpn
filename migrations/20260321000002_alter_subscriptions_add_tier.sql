ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS tier TEXT NOT NULL DEFAULT 'free';

UPDATE subscriptions SET tier = 'free' WHERE plan = 'free' OR plan IS NULL;
UPDATE subscriptions SET tier = 'escudo' WHERE plan = 'pro';
UPDATE subscriptions SET tier = 'pro' WHERE plan = 'family';
