-- Your SQL goes here
-- Drop the unique constraint on user_id in the presents table
ALTER TABLE presents DROP CONSTRAINT IF EXISTS presents_user_id_key;
