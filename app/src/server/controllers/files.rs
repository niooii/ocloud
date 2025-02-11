use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};

use sqlx::query;
use sqlx::query_as;
use serde::{Serialize, Deserialize};
use sqlx::{postgres::{PgArguments, PgQueryResult, PgRow}, query::{Query}, FromRow, PgPool, Postgres, Row, Transaction};
use tokio::{fs, sync::{Notify, RwLock}};
use key_mutex::tokio::KeyMutex;
use tracing::trace;

use crate::{config::SERVER_CONFIG, server::{controllers::model::SFileRow, error::{ServerError, ServerResult}}};

use super::model::{FileUploadInfo, Media, SFile, VirtualPath};

pub enum SFileCreateInfo<'a> {
    Dir {
        path: &'a VirtualPath
    },
    File {
        path: &'a VirtualPath,
        media_id: i64,
    }
}

pub type FileController = Arc<FileControllerInner>;

#[derive(Clone)]
pub struct FileControllerInner {
    db_pool: PgPool,
    pub active_uploads: Arc<KeyMutex<String, ()>>
}

impl FileControllerInner {
    pub async fn create_root(&self) {
        if let Err(e) = self.make_dir(&VirtualPath::root(), None).await {
            match e {
                ServerError::PathAlreadyExists => {},
                _ => panic!("Failed to create root content directory.")
            }
        } 
    }

    pub async fn new(db_pool: PgPool) -> Self {
        let fc = Self {
            db_pool,
            active_uploads: Arc::new(KeyMutex::new())
        };

        fc.create_root().await;

        fc
    }
}

impl FileControllerInner {
    pub async fn finish_upload(&self, mut info: FileUploadInfo) -> ServerResult<SFile> {
        let mut tx = self.db_pool.begin().await?;
        info.vpath.push_file(info.file_name)?;
        // stage 1: insert into media table if it doesnt exist
        let existing: Option<Media> = query_as!(
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
                query_as!(
                    Media,
                    "INSERT INTO media (
                        file_size,
                        file_hash
                    )
                    VALUES ($1, $2)
                    RETURNING *",
                    // TODO! expiring times maybe??
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
                .map_err(|e| ServerError::IOError { why: e.to_string() });
            trace!("Removed duplicate file {}", info.temp_path.to_string_lossy());
        } else {
            // No duplicate
            // Rename the file to its proper name
            let true_path: PathBuf = media.true_path().await;

            fs::create_dir_all(true_path.parent().unwrap()).await?;
            
            fs::rename(
                &info.temp_path, 
                &true_path
            ).await.map_err(|e| ServerError::IOError { why: e.to_string() })?;
            
            trace!("Finalized upload: {}", true_path.to_string_lossy());  
        }
        // stage 3: insert the symbolic file into its table after creating all dirs
        self.make_all_dirs(&info.vpath, Some(&mut tx)).await?;
        let f = 
            self.make_file(
                &info.vpath,
                media.id,
                Some(&mut tx)
            ).await?;

        // finally commit transaction... phew
        tx.commit().await?;
    
        Ok(f)
    }

    pub async fn get_media(&self, vpath: &VirtualPath) -> ServerResult<Media> {
        query_as!(
            Media,
            "SELECT * FROM media
            WHERE id = $1",
            self.get_media_id(vpath).await?,
        )
        .fetch_optional(&self.db_pool).await?
        .ok_or(ServerError::NoMediaFound)
    }

    /// Wipes all the files stored. Very destructive.
    pub async fn wipe(&self) -> ServerResult<()> {
        let mut tx = self.db_pool.begin().await?;
        
        query!(
            r#"DELETE FROM media"#
        ).execute(&mut *tx)
        .await?;
        query!(
            r#"DELETE FROM sfiles"#
        ).execute(&mut *tx)
        .await?;

        tx.commit().await?;
    
        fs::remove_dir_all(&SERVER_CONFIG.files_dir).await?;
        fs::create_dir(&SERVER_CONFIG.files_dir).await?;

        self.create_root().await;

        Ok(())
    }

    pub async fn all_files(&self) -> ServerResult<Vec<SFile>> {
        query_as!(
            SFileRow,
            r#"SELECT * FROM sfiles"#
        ).fetch_all(&self.db_pool).await
        .map_err(ServerError::from)
        .map(|rows| rows.into_iter().map(SFile::from).collect())
    }

    // Deletes the symbolic file, aka the 'pointer' to the media.
    // Also will delete the media if there are no more references to it
    pub async fn delete_sfile(&self, vpath: &VirtualPath) -> ServerResult<()> {
        vpath.err_if_dir()?;

        let full_path = vpath.to_string();

        let mut tx = self.db_pool.begin().await?;

        let media_id = query!(
            r"DELETE FROM sfiles
            WHERE full_path = $1
            RETURNING media_id",
            full_path
        ).fetch_optional(&mut *tx)  
        .await?
        .map(|row| row.media_id)
        .ok_or(ServerError::NoMediaFound)?;
        
        let has_remaining_refs = query!(
            r"SELECT 1 AS exists FROM sfiles 
            WHERE media_id = $1
            LIMIT 1",
            media_id
        )
            .fetch_optional(&mut *tx)
            .await?
            .is_some();

        if !has_remaining_refs {
            let deleted = query_as!(
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

    async fn _create_file(
        &self, 
        info: SFileCreateInfo<'_>, 
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> ServerResult<SFile> {
        let (path, media_id, is_dir) = match info {
            SFileCreateInfo::Dir { path } => (path, None, true),
            SFileCreateInfo::File { path, media_id } => (path, Some(media_id), false),
        };

        let full_path = path.to_string();

        let q = query_as!(
            SFileRow,
            r#"
            INSERT INTO sfiles (
                media_id,
                full_path,
                path_parts,
                is_dir
            )
            VALUES ($1, $2, STRING_TO_ARRAY($2, '/'), $3)
            RETURNING *
            "#,
            media_id,
            full_path,
            is_dir,
        );
        let file = if let Some(t) = transaction {
            q.fetch_one(&mut **t).await
        } else {
            q.fetch_one(&self.db_pool).await
        }
        .map_err(|e| {
            let unique_violation = e.as_database_error()
            .and_then(|d| d.code())
            .map_or(false, |code| code == "23505");
            if unique_violation {
                ServerError::PathAlreadyExists
            } else {
                ServerError::from(e)
            }
        })?;

        Ok(SFile::from(file))
    }

    pub async fn make_file(
        &self, 
        vpath: &VirtualPath, 
        media_id: i64,
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> ServerResult<SFile> {
        vpath.err_if_dir()?;
        self._create_file(
            SFileCreateInfo::File { path: vpath, media_id }, 
            transaction
        ).await
    }

    pub async fn make_dir(
        &self, 
        vpath: &VirtualPath, 
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> ServerResult<SFile> {
        vpath.err_if_file()?;
        self._create_file(
            SFileCreateInfo::Dir { path: vpath }, 
            transaction
        ).await
    }

    pub async fn list_dir(
        &self,
        vpath: &VirtualPath,
        
    ) -> ServerResult<Option<Vec<SFile>>> {
        vpath.err_if_file()?;
        let parts = vpath.path_parts();
        // TODO! unsafe cast
        let depth = parts.len() as i32;

        let mut transaction = self.db_pool.begin().await?;
        if let None = self.path_info_transacted(
            vpath, 
            &mut transaction
        ).await? {
            return Ok(None);
        }
        let res = query_as!(
            SFileRow,
            "SELECT * FROM sfiles
            WHERE path_parts[1:$1] = $2 AND array_length(path_parts, 1) = $3",
            depth,
            &parts,
            depth + 1
        )
            .fetch_all(&mut *transaction).await
            .map_err(ServerError::from)
            .map(|v| Some(
                v.iter().map(SFile::from).collect()
            ));

        transaction.commit().await?;
        
        res
    }

    /// Create all directories. If vpath is a directory, it will create that too, otherwise it will
    /// just create up to the deepest parent.
    /// Could do it in one query,
    /// Returns a vec of all the files newly created.
    pub async fn make_all_dirs(
        &self, 
        vpath: &VirtualPath, 
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> ServerResult<Vec<SFile>> {
        let mut parts = vpath.path_parts_no_root();
        if !vpath.is_dir() {
            parts.pop();
        }
        let mut curr = VirtualPath::root();
        let mut vec = Vec::with_capacity(parts.len());

        let mut default_transaction = None;
        let transaction = transaction
            .unwrap_or(default_transaction.get_or_insert(self.db_pool.begin().await?));

        for part in parts {
            curr.push_dir(part).expect("should never happen, again");

            let create_res = self
                .make_dir(&curr, Some(transaction))
                .await;

            match create_res {
                Err(e) => {
                    match e {
                        ServerError::PathAlreadyExists => continue,
                        _ => return Err(e) 
                    }
                },
                Ok(s) => vec.push(s)
            }
        }

        if let Some(t) = default_transaction {
            t.commit().await?;
        }

        Ok(vec)
    }

    /// Returns the file after the move.
    /// Works with directories and single files.
    pub async fn mv(
        &self,
        from: &VirtualPath,
        to: &VirtualPath
    ) -> ServerResult<SFile> {
        if from.is_dir() {
            // it doesnt make sense to move a dir to a file, so treat the dest like a dir.
            let to = to.as_dir();
            query!(
                ""
            );
            todo!();
        } else {
            query_as!(
                SFileRow, 
                r"UPDATE sfiles
                SET full_path = $1, path_parts = STRING_TO_ARRAY($1, '/')
                WHERE full_path = $2 AND is_dir = false
                RETURNING *",
                to.to_string(),
                from.to_string()
            )
                .fetch_optional(&self.db_pool).await
                .map_err(ServerError::from)
                .and_then(|opt| opt.ok_or(ServerError::PathDoesntExist))
                .map(SFile::from)
        }
    }

    pub async fn path_info(
        &self, 
        vpath: &VirtualPath
    ) -> ServerResult<Option<SFile>> {
        let full_path = vpath.to_string();
        query_as!(
            SFileRow,
            r"SELECT * FROM sfiles
            WHERE full_path = $1",
            full_path
        ).fetch_optional(&self.db_pool).await
            .map_err(ServerError::from)
            // oh hell no
            .map(|opt| opt.map(SFile::from))
    }

    pub async fn path_info_transacted(
        &self, 
        vpath: &VirtualPath,
        transaction: &mut Transaction<'_, Postgres>
    ) -> ServerResult<Option<SFile>> {
        let full_path = vpath.to_string();
        query_as!(
            SFileRow,
            r"SELECT * FROM sfiles
            WHERE full_path = $1",
            full_path
        ).fetch_optional(&mut **transaction).await
            .map_err(ServerError::from)
            .map(|opt| opt.map(SFile::from))
    }

    pub async fn get_media_id(&self, vpath: &VirtualPath) -> ServerResult<Option<i64>> {
        vpath.err_if_dir()?;
        let full_path = vpath.to_string();
            query!(
                r"SELECT media_id FROM sfiles
                WHERE full_path = $1 AND is_dir = false",
                full_path
            ).fetch_optional(&self.db_pool).await
                .map_err(ServerError::from)
                .map(|opt| opt.map(|rec| rec.media_id.unwrap()))
    }

    async fn execute_maybe_transacted<'a>(
        &self, 
        q: Query<'a, Postgres, PgArguments>,
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> core::result::Result<PgQueryResult, sqlx::Error> {
        if let Some(t) = transaction {
            q.execute(&mut **t).await
        } else {
            q.execute(&self.db_pool).await
        }
    }
}