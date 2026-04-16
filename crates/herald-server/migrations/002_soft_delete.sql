-- Add soft-delete support: items are marked as deleted rather than removed
ALTER TABLE messages ADD COLUMN deleted_at TEXT;
ALTER TABLE countdowns ADD COLUMN deleted_at TEXT;
