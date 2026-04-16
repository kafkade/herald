-- Add source_text column to preserve original text input for reflowing
ALTER TABLE messages ADD COLUMN source_text TEXT;
