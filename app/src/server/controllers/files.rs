// controller.rs
use std::{path::PathBuf, sync::Arc};

use key_mutex::tokio::KeyMutex;
use sqlx::query;
use sqlx::query_as;
use sqlx::{
    postgres::{PgArguments, PgQueryResult},
    query::Query,
    PgPool, Postgres, Transaction,
};
use tokio::fs;
use tracing::trace;

use crate::{
    config::SETTINGS,
    server::{
        controllers::websocket::WebSocketController,
        error::{ServerError, ServerResult},
        models::files::SFileRow,
    },
};

use crate::server::models::auth::RelationshipType;
use crate::server::models::files::{FileUploadInfo, Media, SFile, VirtualPath};


/// File permission operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePermission {
    Read,
    Write,
    Delete,
    Admin,
}

impl FilePermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilePermission::Read => "read",
            FilePermission::Write => "write",
            FilePermission::Delete => "delete",
            FilePermission::Admin => "admin",
        }
    }
}

pub enum SFileCreateInfo<'a> {
    Dir {
        path: &'a VirtualPath,
        user_id: i64,
    },
    File {
        path: &'a VirtualPath,
        media_id: i64,
        user_id: i64,
    },
}

pub type FileController = Arc<FileControllerInner>;

#[derive(Clone)]
pub struct FileControllerInner {
    db_pool: PgPool,
    pub active_uploads: Arc<KeyMutex<String, ()>>,
    ws: Option<WebSocketController>,
}

impl FileControllerInner {
    pub fn db_pool(&self) -> &PgPool {
        &self.db_pool
    }

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
        .fetch_optional(&mut *tx)
        .await?;

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
                .fetch_one(&mut *tx)
                .await?
            }
        };

        // stage 2: finalize file location or delete dup file
        if is_duplicate {
            // dont matter if this fails or not ngl
            let _ = fs::remove_file(&info.temp_path)
                .await
                .map_err(|e| ServerError::IOError {
                    message: e.to_string(),
                });
            trace!(
                "Removed duplicate file {}",
                info.temp_path.to_string_lossy()
            );
        } else {
            // No duplicate
            // Rename the file to its proper name
            let true_path: PathBuf = media.true_path().await;

            fs::create_dir_all(true_path.parent().unwrap()).await?;

            fs::rename(&info.temp_path, &true_path)
                .await
                .map_err(|e| ServerError::IOError {
                    message: e.to_string(),
                })?;

            trace!("Finalized upload: {}", true_path.to_string_lossy());
        }

        // stage 3: insert the symbolic file into its table after creating all dirs
        self.make_all_dirs(&info.vpath, info.user_id, Some(&mut tx))
            .await?;
        let f = self
            .make_file(&info.vpath, media.id, info.user_id, Some(&mut tx))
            .await?;

        // Create resource and grant owner permission
        let resource_id = query!(
            r"INSERT INTO resources (resource_type, resource_id) 
            VALUES ($1, $2) 
            RETURNING id",
            "sfile",
            f.id as i64
        )
        .fetch_one(&mut *tx)
        .await?;

        query!(
            r"INSERT INTO user_resource_relationships (user_id, resource_id, relationship, granted_by) 
            VALUES ($1, $2, $3, $1)",
            info.user_id,
            resource_id.id,
            RelationshipType::Owner as RelationshipType
        ).execute(&mut *tx).await?;

        // finally commit transaction... phew
        tx.commit().await?;

        // notify ws clients of file creation and upload completion
        if let Some(ref ws) = self.ws {
            // TODO!
        }

        Ok(f)
    }

    pub async fn get_sfile(&self, vpath: &VirtualPath, user_id: i64) -> ServerResult<SFile> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath, user_id).await?;
        let row = query_as!(SFileRow, "SELECT * FROM sfiles WHERE id = $1", sfile_id,)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or(ServerError::PathDoesntExist)?;

        SFile::from_row(row, vpath)
    }

    pub async fn get_media(&self, vpath: &VirtualPath, user_id: i64) -> ServerResult<Media> {
        query_as!(
            Media,
            "SELECT * FROM media
            WHERE id = $1",
            self.get_media_id(vpath, user_id).await?,
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or(ServerError::NoMediaFound)
    }

    async fn resolve_path_to_sfile_id(
        &self,
        vpath: &VirtualPath,
        target_user_id: i64,
    ) -> ServerResult<i64> {
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
                AND se.filename = $2
                AND se.user_id = $3",
                current_sfile_id,
                part,
                target_user_id
            )
            .fetch_optional(&self.db_pool)
            .await?;

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
        target_user_id: i64,
        tx: &mut Transaction<'_, Postgres>,
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
                AND se.filename = $2
                AND se.user_id = $3",
                current_sfile_id,
                part,
                target_user_id
            )
            .fetch_optional(&mut **tx)
            .await?;
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

        sqlx::query!("DROP SCHEMA public CASCADE")
            .execute(&mut *tx)
            .await?;
        sqlx::query!("CREATE SCHEMA public")
            .execute(&mut *tx)
            .await?;
        sqlx::query!("GRANT ALL ON SCHEMA public TO postgres")
            .execute(&mut *tx)
            .await?;
        sqlx::query!("GRANT ALL ON SCHEMA public TO public")
            .execute(&mut *tx)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&mut *tx)
            .await
            .expect("Failed to run migrations.");

        tx.commit().await?;

        fs::remove_dir_all(&SETTINGS.directories.files_dir).await?;
        fs::create_dir(&SETTINGS.directories.files_dir).await?;

        Ok(())
    }

    pub async fn all_files(&self) -> ServerResult<Vec<SFile>> {
        query_as!(SFileRow, r#"SELECT * FROM sfiles"#)
            .fetch_all(&self.db_pool)
            .await
            .map_err(ServerError::from)
            .map(|rows| rows.into_iter().map(SFile::from_row_incomplete).collect())
    }

    // Deletes the symbolic file, aka the 'pointer' to the media.
    // Also will delete the media if there are no more references to it
    pub async fn delete_sfile(&self, vpath: &VirtualPath, user_id: i64) -> ServerResult<()> {
        vpath.err_if_dir()?;

        if vpath.is_root() {
            return Err(ServerError::BadOperation {
                details: "Cannot delete root directory.".into(),
            });
        }

        let mut tx = self.db_pool.begin().await?;

        let sfile_id = self.resolve_path_to_sfile_id(vpath, user_id).await?;

        // stage 1: Get the media_id before deletion
        let media_id = query!(r"SELECT media_id FROM sfiles WHERE id = $1", sfile_id)
            .fetch_optional(&mut *tx)
            .await?
            .and_then(|row| row.media_id)
            .ok_or(ServerError::NoMediaFound)?;

        // stage 2: Delete the directory entry (removes the (filename, parent) -> sfile mapping)
        query!(
            r"DELETE FROM sfile_entries WHERE child_sfile_id = $1",
            sfile_id
        )
        .execute(&mut *tx)
        .await?;

        // stage 3: Delete the sfile itself
        query!(r"DELETE FROM sfiles WHERE id = $1", sfile_id)
            .execute(&mut *tx)
            .await?;

        // stage 4: Check if any other sfiles still reference this media
        let has_remaining_refs = query!(
            r"SELECT 1 as _exists FROM sfiles WHERE media_id = $1 LIMIT 1",
            media_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        // stage 5: If no remaining references, delete the media and file from disk
        if !has_remaining_refs {
            let deleted_media = query_as!(
                Media,
                r"DELETE FROM media WHERE id = $1 RETURNING *",
                media_id
            )
            .fetch_one(&mut *tx)
            .await?;

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
        transaction: Option<&mut Transaction<'_, Postgres>>,
    ) -> ServerResult<SFile> {
        let (path, media_id, is_dir, user_id) = match info {
            SFileCreateInfo::Dir { path, user_id } => (path, None, true, user_id),
            SFileCreateInfo::File {
                path,
                media_id,
                user_id,
            } => (path, Some(media_id), false, user_id),
        };

        let mut default_transaction = None;
        let tx =
            transaction.unwrap_or(default_transaction.get_or_insert(self.db_pool.begin().await?));

        let (parent_sfile_id, filename) = if path.is_root() {
            return Err(ServerError::BadOperation {
                details: "Cannot create another root directory.".into(),
            });
        } else {
            let parent_path = path.parent().unwrap_or_else(VirtualPath::root);
            let parent_sfile_id = if parent_path.is_root() {
                // Find root sfile_id
                // TODO! this COULDD be just replaced with 1 tbh...
                query!("SELECT child_sfile_id FROM sfile_entries WHERE parent_sfile_id = 0 LIMIT 1")
                    .fetch_optional(&mut **tx)
                    .await?
                    .map(|row| row.child_sfile_id)
                    .ok_or(ServerError::PathDoesntExist)?
            } else {
                // Resolve parent path to its id
                self.resolve_path_to_sfile_id_tx(&parent_path, user_id, tx)
                    .await?
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
            r"INSERT INTO sfiles (media_id, is_dir, user_id, is_public)
            VALUES ($1, $2, $3, $4)
            RETURNING *",
            media_id,
            is_dir,
            user_id,
            false // Default to private
        )
        .fetch_one(&mut **tx)
        .await?;

        // create directory entry
        query!(
            r"INSERT INTO sfile_entries (parent_sfile_id, filename, child_sfile_id, user_id)
            VALUES ($1, $2, $3, $4)",
            parent_sfile_id,
            filename,
            sfile_row.id,
            user_id
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            let unique_violation = e
                .as_database_error()
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

        let sfile = SFile::from_row(sfile_row, path)?;

        Ok(sfile)
    }

    pub async fn make_file(
        &self,
        vpath: &VirtualPath,
        media_id: i64,
        user_id: i64,
        transaction: Option<&mut Transaction<'_, Postgres>>,
    ) -> ServerResult<SFile> {
        vpath.err_if_dir()?;
        self._create_file(
            SFileCreateInfo::File {
                path: vpath,
                media_id,
                user_id,
            },
            transaction,
        )
        .await
    }

    pub async fn make_dir(
        &self,
        vpath: &VirtualPath,
        user_id: i64,
        transaction: Option<&mut Transaction<'_, Postgres>>,
    ) -> ServerResult<SFile> {
        vpath.err_if_file()?;
        self._create_file(
            SFileCreateInfo::Dir {
                path: vpath,
                user_id,
            },
            transaction,
        )
        .await
    }

    pub async fn list_dir(
        &self,
        vpath: &VirtualPath,
        user_id: Option<i64>, // None for anonymous users browsing public dirs
        target_id: i64,       // The user whose files we want to list
    ) -> ServerResult<Option<Vec<SFile>>> {
        vpath.err_if_file()?;

        let dir_sfile_id = self.resolve_path_to_sfile_id(vpath, target_id).await?;

        // Use the provided target_id to determine whose files to show
        let target_user_id = target_id;

        // Get files belonging to the target user in this directory
        let results = query!(
            r"SELECT 
                sf.id,
                sf.media_id, 
                sf.is_dir,
                sf.created_at,
                sf.modified_at,
                sf.is_public,
                se.filename,
                sf.user_id
            FROM sfile_entries se
            JOIN sfiles sf ON se.child_sfile_id = sf.id
            WHERE se.parent_sfile_id = $1 
            AND se.user_id = $2
            ORDER BY se.filename",
            dir_sfile_id,
            target_user_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        let base_path = vpath.to_string();
        let sfiles = results
            .into_iter()
            .map(|row| SFile {
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
                is_public: row.is_public,
                user_id: row.user_id,
            })
            .collect();

        Ok(Some(sfiles))
    }

    /// Create all directories. If vpath is a directory, it will create that too, otherwise it will
    /// just create up to the deepest parent.
    /// Could do it in one query,
    /// Returns a vec of all the files newly created.
    pub async fn make_all_dirs(
        &self,
        vpath: &VirtualPath,
        user_id: i64,
        transaction: Option<&mut Transaction<'_, Postgres>>,
    ) -> ServerResult<Vec<SFile>> {
        let mut parts = vpath.path_parts_no_root();
        if !vpath.is_dir() {
            parts.pop();
        }
        let mut curr = VirtualPath::root();
        let mut vec = Vec::with_capacity(parts.len());

        let mut default_transaction = None;
        let transaction =
            transaction.unwrap_or(default_transaction.get_or_insert(self.db_pool.begin().await?));

        for part in parts {
            curr.push_dir(part).expect("should never happen, again");
            // check if directory already exists
            let exists = self
                .path_info_transacted(&curr, user_id, transaction)
                .await
                .map(|sfile| sfile.is_dir);

            let exists = match exists {
                Ok(bool) => bool,
                Err(e) => match e {
                    ServerError::PathDoesntExist => false,
                    _ => return Err(e),
                },
            };

            if exists {
                trace!("Directory already exists, skipping: {curr:?}");
                continue;
            }

            let create_res = self.make_dir(&curr, user_id, Some(transaction)).await;

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
        to: &VirtualPath,
        user_id: i64,
    ) -> ServerResult<SFile> {
        // it doesnt make sense to move a dir to a file, so treat the dest like a dir.
        let to = to.as_dir();
        let from_id = self.resolve_path_to_sfile_id(from, user_id).await?;

        let to_dir_id = self
            .resolve_path_to_sfile_id(&to.parent().unwrap_or(VirtualPath::root()), user_id)
            .await?;

        if from.is_root() {
            return Err(ServerError::BadOperation {
                details: "Cannot move the root directory".into(),
            });
        }

        if to.child_of(from) {
            return Err(ServerError::BadOperation {
                details: "Cannot move directory into itself or its descendants".into(),
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
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or(ServerError::PathDoesntExist)?;

        let sfile = SFile::from_row(result, &to)?;

        // Notify WebSocket clients of file move
        if let Some(ref ws) = self.ws {
            // TODO!
        }

        Ok(sfile)
    }

    pub async fn path_info(&self, vpath: &VirtualPath, user_id: i64) -> ServerResult<SFile> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath, user_id).await?;

        let result = query_as!(SFileRow, "SELECT * FROM sfiles WHERE id = $1", sfile_id)
            .fetch_optional(&self.db_pool)
            .await?;

        match result {
            Some(sfile_row) => {
                let sfile = SFile::from_row(sfile_row, vpath)?;
                Ok(sfile)
            }
            None => Err(ServerError::PathDoesntExist),
        }
    }

    pub async fn path_info_transacted(
        &self,
        vpath: &VirtualPath,
        user_id: i64,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> ServerResult<SFile> {
        let sfile_id = self
            .resolve_path_to_sfile_id_tx(vpath, user_id, transaction)
            .await?;

        let result = query_as!(SFileRow, "SELECT * FROM sfiles WHERE id = $1", sfile_id)
            .fetch_optional(&mut **transaction)
            .await?;

        match result {
            Some(sfile_row) => {
                let sfile = SFile::from_row(sfile_row, vpath)?;
                Ok(sfile)
            }
            None => Err(ServerError::PathDoesntExist),
        }
    }

    pub async fn get_media_id(
        &self,
        vpath: &VirtualPath,
        user_id: i64,
    ) -> ServerResult<Option<i64>> {
        vpath.err_if_dir()?;

        let id = self.resolve_path_to_sfile_id(vpath, user_id).await?;

        query!(r"SELECT media_id FROM sfiles WHERE id = $1", id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(ServerError::from)
            .map(|opt| opt.map(|rec| rec.media_id.unwrap()))
    }

    async fn execute_maybe_transacted<'a>(
        &self,
        q: Query<'a, Postgres, PgArguments>,
        transaction: Option<&mut Transaction<'_, Postgres>>,
    ) -> core::result::Result<PgQueryResult, sqlx::Error> {
        if let Some(t) = transaction {
            q.execute(&mut **t).await
        } else {
            q.execute(&self.db_pool).await
        }
    }

    /// Set the visibility status of a file or directory, returns the updated file
    pub async fn set_file_visibility(
        &self,
        vpath: &VirtualPath,
        is_public: bool,
        user_id: i64,
    ) -> ServerResult<SFile> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath, user_id).await?;

        let sfile = query_as!(
            SFileRow,
            "UPDATE sfiles SET is_public = $1 WHERE id = $2
            RETURNING *",
            is_public,
            sfile_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        SFile::from_row(sfile, vpath)
    }

    /// Get the visibility status of a file or directory
    pub async fn get_file_visibility(
        &self,
        vpath: &VirtualPath,
        user_id: i64,
    ) -> ServerResult<bool> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath, user_id).await?;

        let result = query!("SELECT is_public FROM sfiles WHERE id = $1", sfile_id)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(result.is_public)
    }

    /// Set file visibility by filename (helper for tests)
    pub async fn set_file_visibility_by_name(
        &self,
        filename: &str,
        is_public: bool,
    ) -> ServerResult<()> {

        query!(
            "UPDATE sfiles SET is_public = $1 WHERE id = (
                SELECT s.id FROM sfiles s 
                JOIN sfile_entries se ON s.id = se.child_sfile_id 
                WHERE se.filename = $2
            )",
            is_public,
            filename
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    /// Set directory visibility by name (helper for tests)
    pub async fn set_directory_visibility_by_name(
        &self,
        dirname: &str,
        is_public: bool,
    ) -> ServerResult<()> {

        query!(
            "UPDATE sfiles SET is_public = $1 WHERE id = (
                SELECT s.id FROM sfiles s 
                JOIN sfile_entries se ON s.id = se.child_sfile_id 
                WHERE se.filename = $2 AND s.is_dir = TRUE
            )",
            is_public,
            dirname
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    /// Set permissions for a file/directory for a target user
    pub async fn set_permissions_for(
        &self,
        vpath: &VirtualPath,
        target_user_id: u64,
        relationship: RelationshipType,
        granter_user_id: i64,
    ) -> ServerResult<()> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath, granter_user_id).await?;

        // Check if granter has permission to change permissions on this file
        // First check if user is the owner (direct ownership via sfiles.user_id)
        let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile_id)
            .fetch_optional(&self.db_pool)
            .await?
            .map(|row| row.user_id == Some(granter_user_id))
            .unwrap_or(false);

        // If not owner, check ReBAC permissions
        if !is_owner {
            // We need an auth context to check permissions
            // For now, we'll just require the granter to be the owner
            // This could be enhanced later to use AuthContext if needed
            return Err(ServerError::AuthorizationError {
                message: "Only file owners can grant permissions".to_string(),
            });
        }

        // Find or create resource entry for this file
        let resource_id = {
            // Try to find existing resource
            if let Some(resource) = query!(
                "SELECT id FROM resources WHERE resource_type = $1 AND resource_id = $2",
                "sfile",
                sfile_id
            )
            .fetch_optional(&self.db_pool)
            .await? {
                resource.id
            } else {
                // Create new resource
                query!(
                    r"INSERT INTO resources (resource_type, resource_id) 
                    VALUES ($1, $2) 
                    RETURNING id",
                    "sfile",
                    sfile_id
                )
                .fetch_one(&self.db_pool)
                .await?
                .id
            }
        };

        // Check if relationship already exists
        let existing = query!(
            "SELECT id FROM user_resource_relationships 
             WHERE user_id = $1 AND resource_id = $2 AND relationship = $3",
            target_user_id as i64,
            resource_id,
            relationship as RelationshipType
        )
        .fetch_optional(&self.db_pool)
        .await?;

        if existing.is_some() {
            return Err(ServerError::ValidationError {
                message: "Permission already exists for this user".to_string(),
            });
        }

        // Grant permission
        query!(
            r"INSERT INTO user_resource_relationships (user_id, resource_id, relationship, granted_by) 
            VALUES ($1, $2, $3, $4)",
            target_user_id as i64,
            resource_id,
            relationship as RelationshipType,
            granter_user_id
        ).execute(&self.db_pool).await?;

        Ok(())
    }

    /// Revoke permissions for a file/directory for a target user
    pub async fn revoke_permissions_for(
        &self,
        vpath: &VirtualPath,
        target_user_id: u64,
        relationship: RelationshipType,
        granter_user_id: i64,
    ) -> ServerResult<()> {
        let sfile_id = self.resolve_path_to_sfile_id(vpath, granter_user_id).await?;

        // Check if granter has permission to change permissions on this file
        let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile_id)
            .fetch_optional(&self.db_pool)
            .await?
            .map(|row| row.user_id == Some(granter_user_id))
            .unwrap_or(false);

        if !is_owner {
            return Err(ServerError::AuthorizationError {
                message: "Only file owners can revoke permissions".to_string(),
            });
        }

        // Find resource entry for this file
        let resource = query!(
            "SELECT r.id FROM resources r WHERE r.resource_type = $1 AND r.resource_id = $2",
            "sfile",
            sfile_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| ServerError::ValidationError {
            message: "No permissions found for this file".to_string(),
        })?;

        // Revoke permission
        let rows_affected = query!(
            "DELETE FROM user_resource_relationships 
             WHERE user_id = $1 AND resource_id = $2 AND relationship = $3",
            target_user_id as i64,
            resource.id,
            relationship as RelationshipType
        ).execute(&self.db_pool).await?.rows_affected();

        if rows_affected == 0 {
            return Err(ServerError::ValidationError {
                message: "Permission not found for this user".to_string(),
            });
        }

        Ok(())
    }
}
