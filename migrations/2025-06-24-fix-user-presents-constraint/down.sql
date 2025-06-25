-- This file should undo anything in `up.sql`
-- Add back the unique constraint if needed in the future
ALTER TABLE presents ADD CONSTRAINT presents_user_id_key UNIQUE (user_id);
