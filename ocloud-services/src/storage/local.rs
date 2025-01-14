use std::{future::Future, path::{Path, PathBuf}, pin::Pin};

use axum::extract::multipart::Field;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{ Digest, Sha256};
use sqlx::{prelude::FromRow, PgPool};
use tokio::{fs::File, io::{AsyncRead, AsyncReadExt}};
use bytes::Bytes;
use tokio_util::io::ReaderStream;
use crate::{cdn_settings, error::{Error, Result}};

use super::backend::{ByteStream, StorageBackend};

// A row from the database.
#[derive(Serialize, FromRow)]
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
    // The name of the file on the host machine, including the extension.
    pub file_name: String,
    // The SHA-256 checksum of the file.
    pub file_hash: String,
}

impl Media {
    // Returns a relative url to be attached to a base cdn url.
    // Format: "/fname.ext?id={id}&checksum={hash}"
    pub fn rel_endpoint(&self) -> String {
        format!(
            "{}?&id={}&checksum={}", 
            self.file_name, 
            self.id, 
            self.file_hash  
        )
    }

    // Returns what this file should be named.
    pub fn true_filename(&self) -> String {
        // Just use the file hash igs
        // If there's a hash collision using SHA-256,
        // I will throw my computer into the sun
        // and bring myself along
        self.file_hash.clone()
    }

    // Returns the path of this file on the host machine's filesystem.
    // Is not guarenteed a file exists at this path.
    pub async fn true_path(&self) -> PathBuf {
        let save_dir: &String = &cdn_settings.read().await.save_dir;
        let path: &Path = Path::new(save_dir);

        path.join(&self.true_filename())
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

// Structs to be passed as info to the controller

pub struct MediaUploadInfo {
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    pub upload_start_time: i64
}           

pub struct MediaAccessInfo {
    pub id: i64,
    pub file_name: String,
    pub file_hash: String
}

// Global configuration settings

#[derive(Deserialize, Debug)]
pub struct CdnSettings {
    pub save_dir: String
}

pub struct FilesystemStorage {
    db_pool: PgPool,
}

// The key object for storage on the local filesystem is the content hash.
impl StorageBackend for FilesystemStorage {
    async fn put_object(
        &self, 
        key: &str, 
        data: impl Stream<Item = Result<Bytes>> + Send + 'static
    ) -> Result<()> {
        todo!()
    }

    async fn stream_object(&self, key: &str) -> Result<ByteStream> {
        todo!()
    }

    async fn stream_object_range(
        &self, 
        key: &str,
        range: std::ops::Range<u64>
    ) -> Result<ByteStream> {
        todo!()
    }

    async fn delete_object(&self, key: &str) -> Result<()> {
        todo!()
    }
}