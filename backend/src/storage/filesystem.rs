use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};
use sqlx::{postgres::PgArguments, query::Query};
use crate::{error::Result, storage::model::{FileUploadInfo, Media, SFileRow}, Error, CONFIG};
use serde::{Serialize, Deserialize};
use sqlx::{postgres::{PgQueryResult, PgRow}, query, FromRow, PgPool, Postgres, Row, Transaction};
use tokio::{fs, sync::{Notify, RwLock}};
use key_mutex::tokio::KeyMutex;
use super::model::VirtualPath;
use models::SFile;

pub enum SFileCreateInfo<'a> {
    Dir {
        path: &'a VirtualPath
    },
    File {
        path: &'a VirtualPath,
        media_id: i64,
    }
}

/// Manages "sfiles"
#[derive(Clone)]
pub struct FileSystem {
    db_pool: PgPool,
}

impl FileSystem {
    pub async fn new(db_pool: PgPool) -> Self {
        let fs = Self {
            db_pool
        };
        if let Err(e) = fs.make_dir(&VirtualPath::root(), None).await {
            match e {
                Error::PathAlreadyExists => println!("Root content directory exists."),
                _ => panic!("Failed to create root content directory.")
            }
        } else {
            println!("Created root content directory.")
        }
        fs
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

    // Returns the virtual path inserted into the database, in rare cases
    // it could be different.
    // If a transaction is passed in but fails for whatever reason
    // all the intermediate directories will still be created (not transacted).
    async fn mk_sfile(
        &self, 
        info: SFileCreateInfo<'_>, 
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> Result<VirtualPath> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let (path, media_id, is_dir) = match info {
            SFileCreateInfo::Dir { path } => (path, None, true),
            SFileCreateInfo::File { path, media_id } => (path, Some(media_id), false),
        };

        let path_parts = path.path_parts();
        let full_path = path.to_string();

        let q = sqlx::query!(
            r#"
            INSERT INTO sfiles (
                media_id,
                full_path,
                path_parts,
                created_at,
                modified_at,
                is_dir
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            media_id,
            full_path,
            &path_parts,
            current_time,
            current_time,
            is_dir,
        );
        self.execute_maybe_transacted(q, transaction).await
        .map_err(|e| {
            let unique_violation = e.as_database_error()
            .and_then(|d| d.code())
            .map_or(false, |code| code == "23505");
            if unique_violation {
                Error::PathAlreadyExists
            } else {
                Error::from(e)
            }
        })?;

        Ok(full_path.into())
    }

    pub async fn make_file(
        &self, 
        vpath: &VirtualPath, 
        media_id: i64,
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> Result<VirtualPath> {
        vpath.err_if_dir()?;
        self.make_all_dirs(vpath).await?;
        self.mk_sfile(
            SFileCreateInfo::File { path: vpath, media_id }, 
            transaction
        ).await
    }

    pub async fn make_dir(
        &self, 
        vpath: &VirtualPath, 
        transaction: Option<&mut Transaction<'_, Postgres>>
    ) -> Result<VirtualPath> {
        vpath.err_if_file()?;
        self.mk_sfile(
            SFileCreateInfo::Dir { path: vpath }, 
            transaction
        ).await
    }

    pub async fn list_dir(
        &self,
        vpath: &VirtualPath,
        
    ) -> Result<Option<Vec<SFile>>> {
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
        let res = sqlx::query_as!(
            SFileRow,
            "SELECT * FROM sfiles
            WHERE path_parts[1:$1] = $2 AND array_length(path_parts, 1) = $3",
            depth,
            &parts,
            depth + 1
        )
            .fetch_all(&mut *transaction).await
            .map_err(Error::from)
            .map(|v| Some(
                v.iter().map(SFile::from).collect()
            ));

        transaction.commit().await?;
        
        res
    }

    // Create all directories. If vpath is a directory, it will create that too, otherwise it will
    // just create up to the deepest parent.
    // Could do it in one query,
    // Also not adding a transacted variant since it shouldnt matter, and bad for concurrency
    pub async fn make_all_dirs(&self, vpath: &VirtualPath) -> Result<()> {
        let mut parts = vpath.path_parts_no_root();
        if !vpath.is_dir() {
            parts.pop();
        }
        let mut curr = VirtualPath::root();

        for part in parts {
            curr.push_dir(part).expect("should never happen, again");

            let create_res = self
                .make_dir(&curr, None)
                .await;

            if let Err(e) = create_res {
                match e {
                    Error::PathAlreadyExists => continue,
                    _ => return Err(e) 
                }
            }
        }

        Ok(())
    }

    pub async fn path_info(
        &self, 
        vpath: &VirtualPath
    ) -> Result<Option<SFile>> {
        let full_path = vpath.to_string();
        sqlx::query_as!(
            SFileRow,
            r"SELECT * FROM sfiles
            WHERE full_path = $1",
            full_path
        ).fetch_optional(&self.db_pool).await
            .map_err(Error::from)
            // oh hell no
            .map(|opt| opt.map(SFile::from))
    }

    pub async fn path_info_transacted(
        &self, 
        vpath: &VirtualPath,
        transaction: &mut Transaction<'_, Postgres>
    ) -> Result<Option<SFile>> {
        let full_path = vpath.to_string();
        sqlx::query_as!(
            SFileRow,
            r"SELECT * FROM sfiles
            WHERE full_path = $1",
            full_path
        ).fetch_optional(&mut **transaction).await
            .map_err(Error::from)
            .map(|opt| opt.map(SFile::from))
    }

    pub async fn get_media_id(&self, vpath: &VirtualPath) -> Result<Option<i64>> {
        vpath.err_if_dir()?;
        let full_path = vpath.to_string();
            sqlx::query!(
                r"SELECT media_id FROM sfiles
                WHERE full_path = $1 AND is_dir = false",
                full_path
            ).fetch_optional(&self.db_pool).await
                .map_err(Error::from)
                .map(|opt| opt.map(|rec| rec.media_id.unwrap()))
    }
}