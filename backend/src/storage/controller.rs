use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};

use crate::{error::Result, storage::{filesystem::SFileCreateInfo, model::{FileUploadInfo, Media, SFileRow}}, Error, CONFIG};
use serde::{Serialize, Deserialize};
use sqlx::{postgres::PgRow, query, FromRow, PgPool, Postgres, Row, Transaction};
use tokio::{fs, sync::{Notify, RwLock}};
use key_mutex::tokio::KeyMutex;

use super::{filesystem::FileSystem, model::VirtualPath};
use models::SFile;

pub type StorageController = Arc<StorageControllerInner>;

#[derive(Clone)]
pub struct StorageControllerInner {
    db_pool: PgPool,
    pub fs: FileSystem,
    pub active_uploads: Arc<KeyMutex<String, ()>>
}

impl StorageControllerInner {
    pub async fn new(db_pool: PgPool) -> Self {
        Self {
            fs: FileSystem::new(db_pool.clone()).await,
            db_pool,
            active_uploads: Arc::new(KeyMutex::new())
        }
    }
}

impl StorageControllerInner {
    pub async fn finish_upload(&self, mut info: FileUploadInfo) -> Result<String> {
        let mut tx = self.db_pool.begin().await?;
        info.vpath.push_file(info.file_name)?;
        // stage 1: insert into media table if it doesnt exist
        let existing: Option<Media> = sqlx::query_as!(
            Media,
            "SELECT * FROM media
            WHERE file_hash = $1
            LIMIT 1",
            info.file_hash
        )
            .fetch_optional(&mut *tx).await?;

        let is_duplicate = existing.is_some();

        let media: Media = match existing {
            Some(s) => s,
            None => {
                sqlx::query_as!(
                    Media,
                    "INSERT INTO media (
                        uploaded_time,
                        accessed_time,
                        expiring_time,
                        file_size,
                        file_hash
                    )
                    VALUES ($1, $2, $3, $4, $5)
                    RETURNING *",
                    info.upload_start_time,
                    info.upload_start_time,
                    // TODO! expiring times maybe??
                    0,
                    info.file_size, 
                    info.file_hash
                )
                    .fetch_one(&mut *tx).await?
            }
        };
        
        // stage 2: finalize file location or delete dup file
        if is_duplicate {
            // dont matter if this fails or not ngl
            let _ = fs::remove_file(&info.temp_path).await
                .map_err(|e| Error::IOError { why: e.to_string() });
            println!("Removed duplicate file");
        } else {
            // No duplicate
            // Rename the file to its proper name
            let true_path: PathBuf = media.true_path().await;

            fs::create_dir_all(true_path.parent().unwrap()).await?;
            
            fs::rename(
                info.temp_path.as_path(), 
                true_path.as_path()
            ).await.map_err(|e| Error::IOError { why: e.to_string() })?;
            
            println!("Finalized filename..");  
        }
        // stage 3: insert the symbolic file into its table
        let vpath = 
            self.fs.make_file(
                &info.vpath,
                media.id,
                Some(&mut tx)
            ).await?;

        // finally commit transaction... phew
        tx.commit().await?;
    
        Ok(vpath.to_string())
    }

    pub async fn get_media(&self, vpath: &VirtualPath) -> Result<Media> {
        Ok(
            sqlx::query_as!(
                Media,
                "SELECT * FROM media
                WHERE id = $1",
                self.fs.get_media_id(vpath).await?,
            )
            .fetch_optional(&self.db_pool).await?
            .ok_or_else(|| Error::NoMediaFound)?
        )
    }

    // Deletes the symbolic file, aka the 'pointer' to the media.
    // Also will delete the media if there are no more references to it
    pub async fn delete_sfile(&self, vpath: &VirtualPath) -> Result<()> {
        vpath.err_if_dir()?;

        let full_path = vpath.to_string();

        let mut tx = self.db_pool.begin().await?;

        let media_id = sqlx::query!(
            r"DELETE FROM sfiles
            WHERE full_path = $1
            RETURNING media_id",
            full_path
        ).fetch_optional(&mut *tx)  
        .await?
        .map(|row| row.media_id)
        .ok_or(Error::NoMediaFound)?;
        
        let has_remaining_refs = sqlx::query!(
            r"SELECT 1 AS exists FROM sfiles 
            WHERE media_id = $1
            LIMIT 1",
            media_id
        )
            .fetch_optional(&mut *tx)
            .await?
            .is_some();

        if !has_remaining_refs {
            let deleted = sqlx::query_as!(
                Media,
                r"DELETE FROM media
                WHERE id = $1
                RETURNING *",
                media_id
            ).fetch_one(&mut *tx)
                .await?;
            deleted.delete_from_disk().await?;
        }

        // if anything fails (delete db entries or delete on disk) then
        // the transaction doesnt go through
        tx.commit().await?;
        Ok(())
    }
}