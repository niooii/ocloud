// model.rs

use std::{fmt, path::{Path, PathBuf}};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de::Error as err;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use crate::{config::SETTINGS, server::{controllers::websocket::WsIncomingEvent, error::{ServerError, ServerResult}, ServerState}};

// A row from the database.
#[derive(FromRow, Debug)]
pub struct Media {
    pub id: i64,
    pub uploaded_time: NaiveDateTime,
    // TODO! Will be used to implement caching later.
    pub accessed_time: NaiveDateTime,
    // uhh.. TODO!
    pub expiring_time: Option<NaiveDateTime>,
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
        let save_dir = &SETTINGS.directories.files_dir;

        let hash = self.file_hash.clone();
        let first_dir = &hash[0..2];
        let second_dir  = &hash[2..4];
        let fname  = &hash[4..];

        save_dir.join(Path::new(&format!("{first_dir}/{second_dir}/{fname}")))
    }

    // Get a ReaderStream from the file, or an Err if it doesn't exist.
    pub async fn reader_stream(&self) -> ServerResult<ReaderStream<File>> {
        let file = File::open(&self.true_path().await)
            .await.map_err(|e| ServerError::IOError { message: e.to_string() })?;

        Ok(ReaderStream::new(file))
    }

    // Attempts to delete the underlying file from the disk.
    pub async fn delete_from_disk(&self) -> ServerResult<()> {
        tokio::fs::remove_file(self.true_path().await.as_path()).await
            .map_err(|e| ServerError::IOError { message: e.to_string() })
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct SFile {
    pub id: u64,
    pub media_id: Option<u64>,
    pub is_dir: bool,
    // computed when needed, not stored in DB
    pub full_path: String,  
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    // Either the name of the directory or the file
    pub top_level_name: String
}

// A row from the sfiles table (new schema - no paths stored)
#[derive(Serialize, FromRow)]
pub struct SFileRow {
    pub id: i64,
    pub media_id: Option<i64>,
    pub is_dir: bool,
    // times will always be in UTC
    pub created_at: NaiveDateTime,
    pub modified_at: NaiveDateTime,
}

// A row from the sfile_entries table
#[derive(Serialize, FromRow)]
pub struct SFileEntryRow {
    pub id: i64,
    pub parent_sfile_id: Option<i64>,
    pub filename: String,
    pub child_sfile_id: i64,
}

// Helper struct for joining sfiles with their entry info
#[derive(FromRow)]
pub struct SFileWithEntry {
    // From sfiles table
    pub id: i64,
    pub media_id: Option<i64>,
    pub is_dir: bool,
    pub created_at: NaiveDateTime,
    pub modified_at: NaiveDateTime,
    // From sfile_entries table
    pub filename: String,
    pub parent_sfile_id: Option<i64>,
}

impl From<SFileRow> for SFile {
    fn from(value: SFileRow) -> Self {
        Self::from(&value)
    }
}

impl From<&SFileRow> for SFile {
    fn from(value: &SFileRow) -> Self {
        Self {
            id: value.id as u64,
            media_id: value.media_id.map(|id| id as u64),
            is_dir: value.is_dir,
            full_path: String::new(), // Will be populated separately
            created_at: value.created_at.and_utc(),
            modified_at: value.modified_at.and_utc(),
            top_level_name: String::new(), // Will be populated separately
        }
    }
}

impl From<SFileWithEntry> for SFile {
    fn from(value: SFileWithEntry) -> Self {
        Self {
            id: value.id as u64,
            media_id: value.media_id.map(|id| id as u64),
            is_dir: value.is_dir,
            full_path: String::new(), // Will be computed by controller
            created_at: value.created_at.and_utc(),
            modified_at: value.modified_at.and_utc(),
            top_level_name: value.filename,
        }
    }
}

fn check_is_dir(path: &PathBuf) -> bool {
    path.to_string_lossy().ends_with('/')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualPathError {
    InvalidPrefix,
    EmptyPath,
    InvalidCharacters,
}

impl fmt::Display for VirtualPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VirtualPathError::InvalidPrefix => write!(f, "Path must start with 'root/'"),
            VirtualPathError::EmptyPath => write!(f, "Path cannot be empty"),
            VirtualPathError::InvalidCharacters => write!(f, "Path contains invalid characters"),
        }
    }
}

impl std::error::Error for VirtualPathError {}

#[derive(Debug, PartialEq, Eq)]
/// A path that references a file on the host machines filesystem, used to access uploaded content.
/// The path can never be empty.
pub struct VirtualPath {
    path: PathBuf,
    is_dir: bool
} 

impl VirtualPath {
    pub fn root() -> Self {
        Self {
            path: PathBuf::from("root/"),
            is_dir: true
        }
    }

    /// Fallible constructor that validates the path
    pub fn try_from_string<S: AsRef<str>>(path: S) -> Result<Self, VirtualPathError> {
        let path_str = path.as_ref();
        
        if path_str.is_empty() {
            return Err(VirtualPathError::EmptyPath);
        }
        
        if !path_str.starts_with("root/") {
            return Err(VirtualPathError::InvalidPrefix);
        }
        
        // Check for invalid characters (null bytes, etc.)
        if path_str.contains('\0') {
            return Err(VirtualPathError::InvalidCharacters);
        }
        
        Ok(VirtualPath::from(path_str))
    }

    /// A path is never a child of itself.
    pub fn child_of(&self, other: &Self) -> bool {
        if !other.is_dir {
            return false;
        }
 
        let self_path = self.to_string();
        let other_path = other.to_string();

        if self_path.len() <= other_path.len() {
            return false;
        }

        // TODO! see if self contains other, if yes then
        // split self at others's length and see if it starts with a slash. if yes then
        // self is a child of other
        if self_path.contains(&other_path) {
            self_path[other_path.len()..].chars().next()
            .expect("Expected length check to work idiot") == '/'
        } else {
            false
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
            .skip(1)
            .collect()
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn err_if_dir(&self) -> ServerResult<()> {
        if self.is_dir() {
            Err(ServerError::InternalError { message: "Bad path: did not expect directory".to_string() })
        } else {
            Ok(())
        }
    }

    pub fn err_if_file(&self) -> ServerResult<()> {
        if !self.is_dir() {
            Err(ServerError::InternalError { message: "Bad path: did not expect file".to_string() })
        } else {
            Ok(())
        }
    }

    /// Only pushes if the path is currently a directory
    pub fn push_file(&mut self, file_name: String) -> ServerResult<()> {
        self.err_if_file()?;
        self.path.push(file_name);
        self.is_dir = false;
        Ok(())
    }

    /// Only pushes if the path is currently a directory
    /// dir_name should not contain a trailing slash
    pub fn push_dir(&mut self, dir_name: String) -> ServerResult<()> {
        self.err_if_file()?;
        self.path.push(
            format!("{dir_name}/")
        );
        Ok(())
    }

    pub fn to_dir(&mut self) {
        if !self.is_dir {
            self.is_dir = true;
            self.path = format!("{}/", self.to_string()).into();
        }
    }

    pub fn to_file(&mut self) {
        if self.is_dir {
            self.is_dir = false;
            self.path = self.to_string().into();
        }
    }

    pub fn as_dir(&self) -> Self {
        Self {
            path: format!("{}/", self.to_string()).into(),
            is_dir: true
        }
    }

    pub fn as_file(&self) -> Self {
        Self {
            path: self.to_string().into(),
            is_dir: false
        }
    }

    pub fn file_name(&self) -> Option<String> {
        self.path.file_name().map(|s| s.to_string_lossy().to_string())
    }

    /// Get just the name component (filename or directory name)
    pub fn name(&self) -> Option<String> {
        if self.is_root() {
            return Some("root".to_string());
        }
        
        let parts = self.path_parts_no_root();
        if parts.is_empty() {
            return None;
        }
        
        let last_part = parts.last().unwrap();
        if self.is_dir && last_part.is_empty() {
            // Handle trailing slash case
            if parts.len() > 1 {
                Some(parts[parts.len() - 2].clone())
            } else {
                None
            }
        } else {
            Some(last_part.clone())
        }
    }

    /// Get the parent directory path
    pub fn parent(&self) -> Option<VirtualPath> {
        if self.is_root() {
            return None;
        }
        
        let parts = self.path_parts_no_root();
        if parts.is_empty() {
            return None;
        }
        
        if parts.len() == 1 {
            return Some(VirtualPath::root());
        }
        
        let mut parent_parts = parts;
        if self.is_dir() {
            parent_parts.pop(); // Remove the current directory name
        } else {
            parent_parts.pop(); // Remove the file name
        }
        
        let parent_path = format!("root/{}/", parent_parts.join("/"));
        Some(VirtualPath::from(parent_path))
    }

    /// Like to_string, but keeps the trailing '/' if it is a directory.
    pub fn to_string_with_trailing(&self) -> String {
        self.path.as_os_str().to_string_lossy().into_owned()
    }

    pub fn is_root(&self) -> bool {
        self.path.to_string_lossy() == "root/"
    }

    /// Join with another path component, ensuring proper directory semantics
    pub fn join<S: AsRef<str>>(&self, component: S) -> Result<VirtualPath, ServerError> {
        self.err_if_file()?;
        let mut new_path = self.clone();
        new_path.push_dir(component.as_ref().to_string())?;
        Ok(new_path)
    }

    /// Join with a file name, ensuring this is a directory
    pub fn join_file<S: AsRef<str>>(&self, filename: S) -> Result<VirtualPath, ServerError> {
        self.err_if_file()?;
        let mut new_path = self.clone();
        new_path.push_file(filename.as_ref().to_string())?;
        Ok(new_path)
    }

    /// Get the extension of the file (if it's a file)
    pub fn extension(&self) -> Option<&str> {
        if self.is_dir {
            None
        } else {
            self.path.extension().and_then(|ext| ext.to_str())
        }
    }

    /// Get the file stem (filename without extension)
    pub fn file_stem(&self) -> Option<&str> {
        if self.is_dir {
            None
        } else {
            self.path.file_stem().and_then(|stem| stem.to_str())
        }
    }

    /// Get depth from root (root/ = 0, root/a/ = 1, root/a/b.txt = 2)
    pub fn depth(&self) -> usize {
        let parts = self.path_parts_no_root();
        if self.is_dir && !parts.is_empty() && parts.last().unwrap().is_empty() {
            parts.len() - 1
        } else {
            parts.len()
        }
    }
}

impl Clone for VirtualPath {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            is_dir: self.is_dir,
        }
    }
}

impl<'de> Deserialize<'de> for VirtualPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let val = String::deserialize(deserializer)?;
        if !val.starts_with("root/") {
            return Err(ServerError::InternalError { message: "Path should start with 'root/'.".into() })
                .map_err(D::Error::custom);
        }
        Ok(
            VirtualPath::from(
                val
            )
        )
    }
}

impl From<String> for VirtualPath {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&str> for VirtualPath {
    fn from(value: &str) -> Self {
        let path = PathBuf::from({
            // remove adjacent duplicate '/' characters.
            let mut dedup = String::new();
            for c in value.chars() {
                if !(dedup.ends_with('/') && c == '/') {
                    dedup.push(c);
                } 
            }
            dedup
        });
        Self { 
            is_dir: check_is_dir(&path),
            path
        }
    }
}

impl fmt::Display for VirtualPath {
    /// Does not include the trailing '/' whether it is a directory or not
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str = if self.is_dir {
            let mut s = self.path.as_os_str().to_string_lossy().into_owned();
            s.pop();
            s
        } else {
            self.path.as_os_str().to_string_lossy().into_owned()
        };
        write!(f, "{}", path_str)
    }
}

impl VirtualPath {
    /// Convenience method for getting string representation
    /// Does not include the trailing '/' whether it is a directory or not
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }
}

impl AsRef<Path> for VirtualPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<PathBuf> for VirtualPath {
    fn as_ref(&self) -> &PathBuf {
        &self.path
    }
}

// Structs to be passed as info to the controller
pub struct FileUploadInfo {
    pub file_name: String,
    pub temp_path: PathBuf,
    pub file_size: i64,
    pub file_hash: String,
    pub vpath: VirtualPath
}           

// Websocket (outgoing) events
#[derive(Debug, Clone, Serialize, ocloud_macros::WsOutEvent)]
pub struct FileCreatedEvent {
    pub path: String,
    pub file_id: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, ocloud_macros::WsOutEvent)]
pub struct FileDeletedEvent {
    pub path: String,
    pub file_id: u64,
}

#[derive(Debug, Clone, Serialize, ocloud_macros::WsOutEvent)]
pub struct FileMovedEvent {
    pub from_path: String,
    pub to_path: String,
    pub file_id: u64,
}

#[derive(Debug, Clone, Serialize, ocloud_macros::WsOutEvent)]
pub struct UploadProgressEvent {
    pub path: String,
    pub file_name: String,
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
}

#[derive(Debug, Clone, Serialize, ocloud_macros::WsOutEvent)]
pub struct UploadCompletedEvent {
    pub path: String,
    pub file_name: String,
    pub file_id: u64,
}

// Incoming websocket events
#[ocloud_macros::WsIncomingEvent]
pub struct CancelUploadEvent {
    pub temp: String
}

impl WsIncomingEvent for CancelUploadEvent {
    async fn handle(self, state: &ServerState, connection_id: Uuid) -> ServerResult<()> {
        todo!()
    }
}