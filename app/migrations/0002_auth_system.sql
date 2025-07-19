-- Users table for authentication
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY NOT NULL,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL, -- PHC format (Argon2id)
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_login TIMESTAMP
);

-- ReBAC: Resources that can have permissions
CREATE TABLE IF NOT EXISTS resources (
    id BIGSERIAL PRIMARY KEY NOT NULL,
    resource_type VARCHAR(50) NOT NULL, -- 'sfile', 'media', 'system', mostly sfile for now
    resource_id BIGINT, -- references the actual resource (sfile.id, media.id, etc)
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ReBAC: Relationship types between users and resources
CREATE TYPE relationship_type AS ENUM (
    'owner',
    'editor', 
    'viewer'
);

-- ReBAC: User-Resource relationships
CREATE TABLE IF NOT EXISTS user_resource_relationships (
    id BIGSERIAL PRIMARY KEY NOT NULL,
    user_id BIGINT NOT NULL,
    resource_id BIGINT NOT NULL,
    relationship relationship_type NOT NULL,
    granted_by BIGINT, -- user who granted this permission
    granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP, -- optional expiration
    
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE (user_id, resource_id, relationship)
);

-- Session management for web authentication
CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY NOT NULL,
    user_id BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    last_accessed TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Add user_id to existing tables
ALTER TABLE sfiles ADD COLUMN user_id BIGINT;
ALTER TABLE media ADD COLUMN user_id BIGINT;
ALTER TABLE sfile_entries ADD COLUMN user_id BIGINT;

-- Drop old unique constraint and add new one that includes user_id
ALTER TABLE sfile_entries DROP CONSTRAINT IF EXISTS sfile_entries_parent_sfile_id_filename_key;
ALTER TABLE sfile_entries ADD CONSTRAINT sfile_entries_parent_filename_user_unique 
    UNIQUE (parent_sfile_id, filename, user_id);

-- Add foreign key constraints
ALTER TABLE sfiles ADD CONSTRAINT fk_sfiles_user 
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE media ADD CONSTRAINT fk_media_user 
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE sfile_entries ADD CONSTRAINT fk_sfile_entries_user 
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;

-- Indexes for performance
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_user_resource_relationships_user ON user_resource_relationships(user_id);
CREATE INDEX idx_user_resource_relationships_resource ON user_resource_relationships(resource_id);
CREATE INDEX idx_user_sessions_user ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_expires ON user_sessions(expires_at);
CREATE INDEX idx_sfiles_user ON sfiles(user_id);
CREATE INDEX idx_media_user ON media(user_id);
CREATE INDEX idx_sfile_entries_user ON sfile_entries(user_id);

