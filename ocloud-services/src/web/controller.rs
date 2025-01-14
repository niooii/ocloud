use std::{path::Path, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};

use crate::{error::Result, storage::local::{Media, MediaAccessInfo, MediaUploadInfo}, Error};
use serde::{Serialize, Deserialize};
use sqlx::{postgres::PgRow, query, FromRow, PgPool, Row};

#[derive(Clone)]
pub struct MediaController {
    db_pool: PgPool,
}

impl MediaController {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool
        }
    }
}

async fn insert_info(db_pool: &PgPool, info: &MediaUploadInfo) -> Result<Media> {
    sqlx::query_as!(
        Media,
        "INSERT INTO media (
            uploaded_time,
            accessed_time,
            expiring_time,
            file_size,
            file_name,
            file_hash
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *",
        info.upload_start_time,
        info.upload_start_time,
        // TODO! expiring times maybe??
        0,
        info.file_size, 
        info.file_name,
        info.file_hash
    )
        .fetch_one(db_pool).await
        .map_err(|_| Error::DatabaseQueryError)
}

pub struct MediaCheckinResult {
    pub media: Media,
    pub is_duplicate: bool
}

pub struct MediaDeleteResult {
    pub deleted: Media,
    pub remaining_references: u32
}

impl MediaController {
    pub async fn checkin_media(&self, info: MediaUploadInfo) -> Result<MediaCheckinResult> {
        // TODO! how do i achieve this with one query bruh
        let existing: Option<Media> = sqlx::query_as!(
            Media,
            "SELECT * FROM media
            WHERE file_hash = $1
            LIMIT 1",
            info.file_hash
        )
            .fetch_optional(&self.db_pool).await
            .map_err(|_| Error::DatabaseQueryError)?;

        let media: Media = insert_info(&self.db_pool, &info).await?;

        Ok(MediaCheckinResult { media, is_duplicate: existing.is_some() })
    }

    pub async fn get_media(&self, info: &MediaAccessInfo) -> Result<Media> {
        Ok(
            sqlx::query_as!(
                Media,
                "SELECT * FROM media
                WHERE id = $1 AND file_hash = $2",
                info.id,
                info.file_hash
            )
            .fetch_optional(&self.db_pool).await
            .map_err(|_| Error::DatabaseQueryError)?
            .ok_or_else(|| Error::NoMediaFound)?
        )
    }

    pub async fn delete_media(&self, info: &MediaAccessInfo) -> Result<MediaDeleteResult> {
        // TODO! I don't need the count of references, only if there 
        // is a reference remaining. COUNT(*) is supposedly expensive.
        let query = 
            r#"WITH deleted AS (
                DELETE FROM media
                WHERE id = $1 AND file_hash = $2
                RETURNING *
            )
            SELECT
                deleted.*,
                (SELECT COUNT(*) FROM media WHERE file_hash = $2) as count
            FROM deleted"#; 

        let row = sqlx::query(
            query
        )
        .bind(info.id)
        .bind(info.file_hash.clone())
        .fetch_optional(&self.db_pool).await
        .map_err(|_| Error::DatabaseQueryError)?
        .ok_or_else(|| Error::NoMediaFound)?;

        let deleted = Media::from_row(&row)
            .map_err(|_e| Error::DatabaseQueryError)?;

        // TODO! assume this doesn't overflow.. right..?
        // For unknown reasons this includes the just deleted entry,
        // so subtract one for accurate count.
        let remaining_references: u32 = row.get::<i64, &str>("count") as u32 - 1;
        
        Ok(MediaDeleteResult { deleted, remaining_references })
    } 
}