-- Add public access support to files and folders
-- This allows files/folders to be accessible without authentication

-- Add is_public column to sfiles table
ALTER TABLE sfiles ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT FALSE;

-- Add index for efficient public file queries
CREATE INDEX idx_sfiles_public ON sfiles (is_public) WHERE is_public = TRUE;

-- Add index for public files by user (for listing user's public files)
CREATE INDEX idx_sfiles_user_public ON sfiles (user_id, is_public) WHERE is_public = TRUE;