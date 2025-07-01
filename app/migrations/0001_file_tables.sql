-- basicallly data blocks
CREATE TABLE IF NOT EXISTS media (
    id              BIGSERIAL PRIMARY KEY NOT NULL,
    uploaded_time   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    accessed_time   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expiring_time   TIMESTAMP,
    file_size       BIGINT NOT NULL,
    file_hash       TEXT NOT NULL
);

-- symbolic files (inodes)
CREATE TABLE IF NOT EXISTS sfiles (
    id BIGSERIAL PRIMARY KEY NOT NULL, 
    -- NULL if directory
    media_id BIGINT, 
    is_dir BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE
);

-- SFile entries: (parent_id, name) → sfile
CREATE TABLE IF NOT EXISTS sfile_entries (
    id BIGSERIAL PRIMARY KEY NOT NULL,
    filename TEXT NOT NULL,
    
    -- What directory contains the child (0 = root)
    parent_sfile_id BIGINT NOT NULL,
    child_sfile_id BIGINT NOT NULL,
    
    FOREIGN KEY (parent_sfile_id) REFERENCES sfiles(id) ON DELETE CASCADE,
    FOREIGN KEY (child_sfile_id) REFERENCES sfiles(id) ON DELETE CASCADE,
    UNIQUE (parent_sfile_id, filename)
);

CREATE INDEX IF NOT EXISTS idx_sfile_entries_parent ON sfile_entries(parent_sfile_id);
CREATE INDEX IF NOT EXISTS idx_sfile_entries_child ON sfile_entries(child_sfile_id);

-- Create the root file manually
INSERT INTO sfiles (id, media_id, is_dir, created_at, modified_at)
VALUES (0, NULL, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

INSERT INTO sfiles (id, media_id, is_dir, created_at, modified_at)
VALUES (1, NULL, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

ALTER SEQUENCE sfiles_id_seq RESTART WITH 2;

INSERT INTO sfile_entries (id, filename, parent_sfile_id, child_sfile_id)
VALUES (0, 'root', 0, 1);