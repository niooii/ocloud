-- symbolic files that point to the media
CREATE TABLE IF NOT EXISTS sfiles (
    id              BIGSERIAL PRIMARY KEY NOT NULL,
    -- the media id the "file" at this path refers to,
    -- or NULL if its a dir
    media_id        BIGINT,
    is_dir          BOOLEAN NOT NULL,
    full_path       TEXT NOT NULL,
    path_parts      TEXT[] NOT NULL,
    created_at      BIGINT NOT NULL,
    modified_at     BIGINT NOT NULL,
    CONSTRAINT fk_media
        FOREIGN KEY (media_id)
        REFERENCES media(id)
        ON DELETE CASCADE,
    CONSTRAINT unique_path UNIQUE (full_path)
);
