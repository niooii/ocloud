CREATE TABLE IF NOT EXISTS media (
    id              BIGSERIAL PRIMARY KEY NOT NULL,
    uploaded_time   BIGINT NOT NULL,
    accessed_time   BIGINT NOT NULL,
    expiring_time   BIGINT NOT NULL,
    file_size       BIGINT NOT NULL,
    file_hash       TEXT NOT NULL
);