// controller.rs
use std::{path::PathBuf, sync::Arc};

use sqlx::query;
use sqlx::query_as;
use sqlx::{postgres::{PgArguments, PgQueryResult}, query::{Query}, PgPool, Postgres, Transaction};
use tokio::fs;
use key_mutex::tokio::KeyMutex;
use tracing::trace;

use crate::{config::SETTINGS, server::{models::files::SFileRow, controllers::websocket::WebSocketController, error::{ServerError, ServerResult}}};

use crate::server::models::files::{FileUploadInfo, Media, SFile, VirtualPath};


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
    pub active_uploads: Arc<KeyMutex<String, ()>>,
    ws: Option<WebSocketController>,
}

impl FileControllerInner {
    pub async fn new_no_ws(db_pool: PgPool) -> Self {
        

        Self {
            db_pool,
            active_uploads: Arc::new(KeyMutex::new()),
            ws: None,
        }
    }

    pub async fn new(db_pool: PgPool, ws: WebSocketController) -> Self {
        

        Self {
            db_pool,
            active_uploads: Arc::new(KeyMutex::new()),
            ws: Some(ws),
        }
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
                .map_err(|e| ServerError::IOError { message: e.to_string() });
            trace!("Removed duplicate file {}", info.temp_path.to_string_lossy());
        } else {
            // No duplicate
            // Rename the file to its proper name
            let true_path: PathBuf = media.true_path().await;

            fs::create_dir_all(true_path.parent().unwrap()).await?;
            
            fs::rename(
                &info.temp_path, 
                &true_path
            ).await.map_err(|e| ServerError::IOError { message: e.to_string() })?;
            
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

        // notify ws clients of file creation and upload completion
        if let Some(ref ws) = self.ws {
            // TODO!
        }
    
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

    async fn resolve_path_to_sfile_id(&self, vpath: &VirtualPath) -> ServerResult<i64> {
        // TODO! when optimize multiple queries?
        if vpath.is_root() {
            return Ok(1); 
        }
        
        let parts = vpath.path_parts_no_root();
        let mut current_sfile_id: i64 = 1; // Start at root
        
        for part in parts {
            let result = query!(
                r"SELECT se.child_sfile_id, sf.is_dir 
                FROM sfile_entries se
                JOIN sfiles sf ON se.child_sfile_id = sf.id
                WHERE se.parent_sfile_id = $1 
                AND se.filename = $2",
                current_sfile_id,
                part
            ).fetch_optional(&self.db_pool).await?;
            
            match result {
                Some(row) => current_sfile_id = row.child_sfile_id,
                None => return Err(ServerError::PathDoesntExist),
            }
        }

        Ok(current_sfile_id)
    }

    async fn resolve_path_to_sfile_id_tx(
        &self, 
        vpath: &VirtualPath, 
        tx: &mut Transaction<'_, Postgres>
    ) -> ServerResult<i64> {
        if vpath.is_root() {
            return Ok(1);
        }
        let parts = vpath.path_parts_no_root();
        let mut current_sfile_id: i64 = 1; // Start at root
        for part in parts {
            let result = query!(
                r"SELECT se.child_sfile_id, sf.is_dir
                FROM sfile_entries se
                JOIN sfiles sf ON se.child_sfile_id = sf.id
                WHERE se.parent_sfile_id = $1
                AND se.filename = $2",
                current_sfile_id,
                part
            ).fetch_optional(&mut **tx).await?;
            match result {
                Some(row) => current_sfile_id = row.child_sfile_id,
                None => return Err(ServerError::PathDoesntExist),
            }
        }
        Ok(current_sfile_id)
    }

    /// Pretty much nukes everything. Very destructive (wow really?).
    pub async fn nuke(&self) -> ServerResult<()> {
        let mut tx = self.db_pool.begin().await?;
        
        sqlx::query!("DROP SCHEMA public CASCADE").execute(&mut *tx).await?;
        sqlx::query!("CREATE SCHEMA public").execute(&mut *tx).await?;
        sqlx::query!("GRANT ALL ON SCHEMA public TO postgres").execute(&mut *tx).await?;
        sqlx::query!("GRANT ALL ON SCHEMA public TO public").execute(&mut *tx).await?;
        
        sqlx::migrate!("./migrations").run(&mut *tx).await
            .expect("Failed to run migrations.");
        
        tx.commit().await?;
    
        fs::remove_dir_all(&SETTINGS.directories.files_dir).await?;
        fs::create_dir(&SETTINGS.directories.files_dir).await?;

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

        if vpath.is_root() {
            return Err(ServerError::BadOperation { details: "Cannot delete root directory.".into() })
        }

        let mut tx = self.db_pool.begin().await?;

        let sfile_id = self.resolve_path_to_sfile_id(vpath).await?;

        // stage 1: Get the media_id before deletion 
        let media_id = query!(
            r"SELECT media_id FROM sfiles WHERE id = $1",
            sfile_id
        ).fetch_optional(&mut *tx).await?
        .and_then(|row| row.media_id)
        .ok_or(ServerError::NoMediaFound)?;

        // stage 2: Delete the directory entry (removes the (filename, parent) -> sfile mapping)
        query!(
            r"DELETE FROM sfile_entries WHERE child_sfile_id = $1",
            sfile_id
        ).execute(&mut *tx).await?;

        // stage 3: Delete the sfile itself 
        query!(
            r"DELETE FROM sfiles WHERE id = $1",
            sfile_id
        ).execute(&mut *tx).await?;

        // stage 4: Check if any other sfiles still reference this media
        let has_remaining_refs = query!(
            r"SELECT 1 as _exists FROM sfiles WHERE media_id = $1 LIMIT 1",
            media_id
        ).fetch_optional(&mut *tx).await?
        .is_some();

        // stage 5: If no remaining references, delete the media and file from disk
        if !has_remaining_refs {
            let deleted_media = query_as!(
                Media,
                r"DELETE FROM media WHERE id = $1 RETURNING *",
                media_id
            ).fetch_one(&mut *tx).await?;
            
            deleted_media.delete_from_disk().await?;
        }

        // If anything fails (delete db entries or delete on disk) then
        // the transaction doesn't go through
        tx.commit().await?;

        if let Some(ref ws) = self.ws {
            // TODO!
        }

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

        let mut default_transaction = None;
        let tx = transaction
            .unwrap_or(default_transaction.get_or_insert(self.db_pool.begin().await?));

        let (parent_sfile_id, filename) = if path.is_root() {
            return Err(ServerError::BadOperation { details: "Cannot create another root directory.".into() });
        } else {
            let parent_path = path.parent().unwrap_or_else(VirtualPath::root);
            let parent_sfile_id = if parent_path.is_root() {
                // Find root sfile_id
                // TODO! this COULDD be just replaced with 1 tbh...
                query!(
                    "SELECT child_sfile_id FROM sfile_entries WHERE parent_sfile_id = 0 LIMIT 1"
                ).fetch_optional(&mut **tx).await?
                .map(|row| row.child_sfile_id)
                .ok_or(ServerError::PathDoesntExist)?
            } else {
                // Resolve parent path to its id
                self.resolve_path_to_sfile_id_tx(&parent_path, tx).await?
            };

            let filename = if path.is_root() {
                "".to_string()
            } else {
                path.name().unwrap_or("".into()).to_string()
            };

            (parent_sfile_id, filename)
        };

        // create the sfile 
        let sfile_row = query_as!(
            SFileRow,
            r"INSERT INTO sfiles (media_id, is_dir)
            VALUES ($1, $2)
            RETURNING *",
            media_id,
            is_dir
        ).fetch_one(&mut **tx).await?;

        // create directory entry 
        query!(
            r"INSERT INTO sfile_entries (parent_sfile_id, filename, child_sfile_id)
            VALUES ($1, $2, $3)",
            parent_sfile_id,
            filename,
            sfile_row.id
        ).execute(&mut **tx).await.map_err(|e| {
            let unique_violation = e.as_database_error()
                .and_then(|d| d.code())
                .is_some_and(|code| code == "23505");
            if unique_violation {
                ServerError::PathAlreadyExists
            } else {
                ServerError::from(e)
            }
        })?;
        
        if let Some(t) = default_transaction {
            t.commit().await?;
        }
        
        let mut sfile = SFile::from(sfile_row);
        sfile.full_path = path.to_string();
        sfile.top_level_name = path.name().unwrap();

        Ok(sfile)
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

        let dir_sfile_id = self.resolve_path_to_sfile_id(vpath).await?;

        // Single query_as! to get structured results directly
        let results = query!(
            r"SELECT 
                sf.id,
                sf.media_id, 
                sf.is_dir,
                sf.created_at,
                sf.modified_at,
                se.filename
            FROM sfile_entries se
            JOIN sfiles sf ON se.child_sfile_id = sf.id
            WHERE se.parent_sfile_id = $1
            ORDER BY se.filename",
            dir_sfile_id
        ).fetch_all(&self.db_pool).await?;

        let base_path = vpath.to_string();
        let sfiles = results.into_iter().map(|row| {
            SFile {
                id: row.id as u64,
                media_id: row.media_id.map(|id| id as u64),
                is_dir: row.is_dir,
                full_path: if vpath.is_root() {
                    format!("root/{}", row.filename)
                } else {
                    format!("{}/{}", base_path, row.filename)
                },
                created_at: row.created_at.and_utc(),
                modified_at: row.modified_at.and_utc(),
                top_level_name: row.filename,
            }
        }).collect();

        Ok(Some(sfiles))
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
            // check if directory already exists 
            let exists = self.path_info_transacted(&curr, transaction).await
            .map(|sfile| sfile.is_dir);

            let exists = match exists {
                Ok(bool) => bool,
                Err(e) => match e {
                    ServerError::PathDoesntExist => false,
                    _ => return Err(e)
                }
            };

            if exists {
                trace!("Directory already exists, skipping: {curr:?}");
                continue;
            }

            let create_res = self
                .make_dir(&curr, Some(transaction))
                .await;

            vec.push(create_res?);
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
        // it doesnt make sense to move a dir to a file, so treat the dest like a dir.
        let to = to.as_dir();
        let from_id = self.resolve_path_to_sfile_id(from).await?;

        let to_dir_id = self.resolve_path_to_sfile_id(&to.parent().unwrap_or(VirtualPath::root())).await?;
        
        if from.is_root() {
            return Err(ServerError::BadOperation { details: "Cannot move the root directory".into() })
        }

        if to.child_of(from) {
            return Err(ServerError::BadOperation { 
                details: "Cannot move directory into itself or its descendants".into() 
            });
        }

        let result = query_as!(
            SFileRow,
            r"WITH updated AS (
                UPDATE sfile_entries 
                SET filename = $1, parent_sfile_id = $2
                WHERE child_sfile_id = $3
                RETURNING child_sfile_id
            )
            SELECT sf.*
            FROM sfiles sf, updated
            WHERE sf.id = updated.child_sfile_id",
            to.name(),
            to_dir_id,
            from_id
        ).fetch_optional(&self.db_pool).await?
        .ok_or(ServerError::PathDoesntExist)?;

        let mut sfile = SFile::from(result);
        sfile.full_path = to.to_string();
        sfile.top_level_name = to.name().unwrap_or("".into()).to_string();

        // Notify WebSocket clients of file move
        if let Some(ref ws) = self.ws {
            // TODO!
        }
        
        Ok(sfile)
    }

    pub async fn path_info(
        &self, 
        vpath: &VirtualPath
    ) -> ServerResult<SFile> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath).await?;

        let result = query_as!(
            SFileRow,
            "SELECT * FROM sfiles WHERE id = $1",
            sfile_id
        ).fetch_optional(&self.db_pool).await?;

        match result {
            Some(sfile_row) => {
                let mut sfile = SFile::from(sfile_row);
                sfile.full_path = vpath.to_string();
                sfile.top_level_name = vpath.name().unwrap_or("".into()).to_string();
                Ok(sfile)
            },
            None => Err(ServerError::PathDoesntExist)
        }
    }

    pub async fn path_info_transacted(
        &self, 
        vpath: &VirtualPath,
        transaction: &mut Transaction<'_, Postgres>
    ) -> ServerResult<SFile> {
        let sfile_id = self.resolve_path_to_sfile_id_tx(vpath, transaction).await?;

        let result = query_as!(
            SFileRow,
            "SELECT * FROM sfiles WHERE id = $1",
            sfile_id
        ).fetch_optional(&mut **transaction).await?;

        match result {
            Some(sfile_row) => {
                let mut sfile = SFile::from(sfile_row);
                sfile.full_path = vpath.to_string();
                sfile.top_level_name = vpath.name().unwrap_or("".into()).to_string();
                Ok(sfile)
            },
            None => Err(ServerError::PathDoesntExist)
        }
    }

    pub async fn get_media_id(&self, vpath: &VirtualPath) -> ServerResult<Option<i64>> {
        vpath.err_if_dir()?;

        let id = self.resolve_path_to_sfile_id(vpath).await?;

        query!(
            r"SELECT media_id FROM sfiles WHERE id = $1",
            id
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