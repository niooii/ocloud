use std::{future::Future, path::{Path, PathBuf}, pin::Pin};

use axum::extract::multipart::Field;
use futures::{Stream, StreamExt};
use models::SFile;
use serde::{Deserialize, Serialize};
use sha2::{ Digest, Sha256};
use sqlx::{prelude::FromRow, PgPool};
use tokio::{fs::File, io::{AsyncRead, AsyncReadExt}};
use bytes::Bytes;
use tokio_util::io::ReaderStream;
use crate::error::Result;
use crate::error::Error;
use crate::CONFIG;

use super::{controller::StorageController};

// A row from the database.
#[derive(FromRow)]
pub struct Media {
    pub id: i64,
    // Unix timestamp in milliseconds.
    pub uploaded_time: i64,
    // Unix timestamp in milliseconds. Will be used to implement caching later.
    pub accessed_time: i64,
    // Unix timestamp in milliseconds. TODO!
    pub expiring_time: i64,
    // Size of the file in bytes.
    pub file_size: i64,
    // The SHA-256 checksum of the file.
    pub file_hash: String,
}

impl Media {
    /// Returns the path that the file should be at 
    /// on the host machine's filesystem.
    /// Is not guarenteed a file exists at this path.
    /// Storage path follows this format: 
    /// `save_dir/[first 2 chars of hash]/[next 2]/[rest of hash]`
    pub async fn true_path(&self) -> PathBuf {
        let save_dir = &CONFIG.read().await.save_dir;

        let hash = self.file_hash.clone();
        let first_dir = &hash[0..2];
        let second_dir  = &hash[2..4];
        let fname  = &hash[4..];

        save_dir.join(Path::new(&format!("{first_dir}/{second_dir}/{fname}")))
    }

    // Get a ReaderStream from the file, or an Err if it doesn't exist.
    pub async fn reader_stream(&self) -> Result<ReaderStream<File>> {
        let file = File::open(&self.true_path().await)
            .await.map_err(|e| Error::IOError { why: e.to_string() })?;

        Ok(ReaderStream::new(file))
    }

    // Attempts to delete the underlying file from the disk.
    pub async fn delete_from_disk(&self) -> Result<()> {
        Ok(tokio::fs::remove_file(self.true_path().await.as_path()).await
            .map_err(|e| Error::IOError { why: e.to_string() })?)
    }
}

// A row from the database.
#[derive(Serialize, FromRow)]
pub struct SFileRow {
    pub id: i64,
    pub path_parts: Vec<String>,
    pub is_dir: bool,
    pub full_path: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub media_id: Option<i64>,
}

impl From<SFileRow> for SFile {
    fn from(value: SFileRow) -> Self {
        Self {
            is_dir: value.is_dir,
            full_path: value.full_path.clone(),
            created_at: value.created_at,
            modified_at: value.modified_at,
            top_level_name: value.path_parts.last()
                .expect("if path parts is empty, something went very wrong").clone()
        }
    }
}

impl From<&SFileRow> for SFile {
    fn from(value: &SFileRow) -> Self {
        Self {
            is_dir: value.is_dir,
            full_path: value.full_path.clone(),
            created_at: value.created_at,
            modified_at: value.modified_at,
            top_level_name: value.path_parts.last()
                .expect("if path parts is empty, something went very wrong").clone()
        }
    }
}

fn check_is_dir(path: &PathBuf) -> bool {
    path.to_string_lossy().ends_with('/')
}

#[derive(Debug)]
pub struct VirtualPath {
    path: PathBuf,
    is_dir: bool
} 

impl VirtualPath {
    pub fn root() -> Self {
        Self {
            path: PathBuf::from("/"),
            is_dir: true
        }
    }

    pub fn path_parts(&self) -> Vec<String> {
        self.path.components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect()
    }
    
    pub fn path_parts_no_root(&self) -> Vec<String> {
        self.path.components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .filter(|c| c != "/")
            .collect()
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn err_if_dir(&self) -> Result<()> {
        if self.is_dir() {
            Err(Error::Error { why: "Bad path: did not expect directory".to_string() })
        } else {
            Ok(())
        }
    }

    pub fn err_if_file(&self) -> Result<()> {
        if !self.is_dir() {
            Err(Error::Error { why: "Bad path: did not expect file".to_string() })
        } else {
            Ok(())
        }
    }

    /// Only pushes if the path is currently a directory
    pub fn push_file(&mut self, file_name: String) -> Result<()> {
        self.err_if_file()?;
        self.path.push(file_name);
        self.is_dir = false;
        Ok(())
    }

    /// Only pushes if the path is currently a directory
    /// dir_name should not contain a trailing slash
    pub fn push_dir(&mut self, dir_name: String) -> Result<()> {
        self.err_if_file()?;
        self.path.push(
            format!("{dir_name}/")
        );
        Ok(())
    }

    pub fn file_name(&self) -> Option<String> {
        self.path.file_name().map(|s| s.to_string_lossy().to_string())
    }

    /// Like to_string, but keeps the trailing '/' if it is a directory.
    pub fn to_string_with_trailing(&self) -> String {
        self.path.as_os_str().to_string_lossy().into_owned()
    }
}

// i want the path strings to have a root, like
// '/school/photos' or something
impl<'de> Deserialize<'de> for VirtualPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        Ok(
            VirtualPath::from(
                format!("/{}", 
                String::deserialize(deserializer)?)
            )
        )
    }
}

impl From<String> for VirtualPath {
    fn from(value: String) -> Self {
        let path = PathBuf::from(value);
        Self { 
            is_dir: check_is_dir(&path),
            path
        }
    }
}

impl From<&str> for VirtualPath {
    fn from(value: &str) -> Self {
        let path = PathBuf::from(value);
        Self { 
            is_dir: check_is_dir(&path),
            path
        }
    }
}

impl ToString for VirtualPath {
    /// Does not include the trailing '/' whether it is a directory or not
    fn to_string(&self) -> String {
        if self.is_dir {
            let mut s = self.path.as_os_str().to_string_lossy().into_owned();
            s.pop();
            s
        } else {
            self.path.as_os_str().to_string_lossy().into_owned()
        }
    }
}

// Structs to be passed as info to the controller
pub struct FileUploadInfo {
    pub file_name: String,
    pub temp_path: PathBuf,
    pub file_size: i64,
    pub file_hash: String,
    pub upload_start_time: i64,
    pub vpath: VirtualPath
}           

// pub struct MediaAccessInfo {
//     pub id: i64,
//     pub file_name: String,
//     pub file_hash: String
// }

// Global configuration settings
#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    // TODO! should make an absolute dir maybe
    pub save_dir: PathBuf,
    pub max_filesize: Option<usize>
}
