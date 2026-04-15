-- Herald database schema v0.1.0

CREATE TABLE IF NOT EXISTS messages (
    id              TEXT PRIMARY KEY NOT NULL,
    grid            TEXT NOT NULL,  -- JSON: 6×22 array of CellContent
    h_align         TEXT NOT NULL DEFAULT 'center',
    v_align         TEXT NOT NULL DEFAULT 'middle',
    queue_position  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    expires_at      TEXT,
    CONSTRAINT valid_h_align CHECK (h_align IN ('left', 'center', 'right')),
    CONSTRAINT valid_v_align CHECK (v_align IN ('top', 'middle'))
);

CREATE INDEX IF NOT EXISTS idx_messages_queue_position ON messages(queue_position);
CREATE INDEX IF NOT EXISTS idx_messages_expires_at ON messages(expires_at) WHERE expires_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS countdowns (
    id              TEXT PRIMARY KEY NOT NULL,
    label           TEXT NOT NULL,
    target          TEXT NOT NULL,
    format_template TEXT NOT NULL DEFAULT '{D} DAYS  {HH}:{MM}:{SS}',
    zero_behavior   TEXT NOT NULL DEFAULT '{"action":"show_zero"}',
    queue_position  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    CONSTRAINT label_length CHECK (length(label) <= 44)
);

CREATE INDEX IF NOT EXISTS idx_countdowns_queue_position ON countdowns(queue_position);

CREATE TABLE IF NOT EXISTS configuration (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS rotation_state (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    current_index  INTEGER NOT NULL DEFAULT 0,
    last_rotation  TEXT NOT NULL
);

-- Seed rotation singleton
INSERT OR IGNORE INTO rotation_state (id, current_index, last_rotation)
VALUES (1, 0, datetime('now'));

-- Seed default configuration
INSERT OR IGNORE INTO configuration (key, value, updated_at) VALUES
    ('rotation_interval_seconds', '10',                          datetime('now')),
    ('countdown_refresh_seconds', '1',                           datetime('now')),
    ('default_h_align',          'center',                       datetime('now')),
    ('default_v_align',          'middle',                       datetime('now')),
    ('default_color',            'white',                        datetime('now')),
    ('countdown_zero_behavior',  '{"action":"show_zero"}',       datetime('now'));
