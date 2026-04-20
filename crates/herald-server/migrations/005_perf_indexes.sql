-- Performance indexes for common query patterns

-- Messages and countdowns are frequently filtered by deleted_at IS NULL
CREATE INDEX IF NOT EXISTS idx_messages_deleted_at ON messages(deleted_at);
CREATE INDEX IF NOT EXISTS idx_countdowns_deleted_at ON countdowns(deleted_at);

-- Message scheduling queries filter on display_at
CREATE INDEX IF NOT EXISTS idx_messages_display_at ON messages(display_at) WHERE display_at IS NOT NULL;
