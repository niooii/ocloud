use std::{io::Write, path::PathBuf, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};
use axum::{body::Body, extract::{DefaultBodyLimit, Multipart, Path, Query, State}, http::{header, HeaderValue}, response::Response, routing::get, Json, Router};
use crate::{config::SERVER_CONFIG, server::controllers::model::SFile};
use tokio::{fs::File, io::AsyncWriteExt, sync::Notify};
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::{error, trace};

use crate::{server::controllers::{files::{FileController}, model::{FileUploadInfo, Media, VirtualPath}}};
use crate::server::error::{ServerError, ServerResult};

pub fn routes(controller: FileController) -> Router {
    Router::new()
        .route(
            "/files/*path", 
            get(get_media)
            .delete(delete_media)
            .post(upload_media)
            .layer(
                if let Some(s) = SERVER_CONFIG.max_filesize {
                    DefaultBodyLimit::max(s)
                } else {
                    DefaultBodyLimit::disable()
                }
            )
        ).with_state(controller)
}

pub async fn upload_media(
    State(files): State<FileController>, 
    Path(mut path): Path<VirtualPath>,
    mut multipart: Multipart
) -> ServerResult<SFile> {
    path.err_if_file()?;

    // Write the first field to the disk, ignore other fields.
    if let Some(mut field) = multipart.next_field().await
    .map_err(|e| ServerError::AxumError { why: format!("Multipart error: {}", e.body_text()) })? {
        
        let save_dir = &SERVER_CONFIG.files_dir;
        
        // Name should be the name of the file, including the extension.
        let name: String = field.name().expect("File has no name??").to_string();
        trace!("Got file: {name}");
        trace!("for path: {}", path.to_string());
        
        if path.to_string_with_trailing().is_empty() {
            path = VirtualPath::root();
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut hasher = Sha256::new();

        let temp_path: PathBuf = save_dir.join(
            format!("./tmp_{now}_{name}")
        );

	    let mut file: File = File::create(&temp_path).await
            .map_err(|e| ServerError::IOError { why: e.to_string() } )?;
        // i64 type because postgres doesnt support unsigned gg

        let mut file_size: i64 = 0;
        while let Some(chunk) = field.chunk().await
            .map_err(|e| ServerError::AxumError { why: format!("Chunk error: {}", e.body_text()) })? {
            
            file.write_all(&chunk).await.map_err(|e| ServerError::IOError { why: e.to_string() } )?;
            file_size += chunk.len() as i64;
            hasher.write_all(&chunk).expect("Failed to hash shit");
        }

	    file.flush().await.expect("Bluh flushing file failed");

        let hash = hasher.finalize();
        let file_hash: String = format!("{hash:X}");  
        
        let info = FileUploadInfo {
            file_name: name,
            temp_path: temp_path.clone(),
            file_size,
            file_hash: file_hash.clone(),
            vpath: path,
        };
        
        // Ensure the file handle is dropped before doing anything
        // ahem windows
        drop(file);

        // HEHEHEHAW fix race condition 
        // just in case if two people upload the same file at the exact same time down to the millisecond...??
        let mutex = files.active_uploads.lock(file_hash.clone())
            .await;

        // Check-in file to database
        let sfile = match files.finish_upload(info).await {
            Err(e) => {
                // doesnt really have to be checked
                let _ = fs::remove_file(&temp_path).await;
                error!("(Tried) removed {} due to error: {e:?}", temp_path.to_string_lossy());
                Err(e)
            }
            Ok(c) => Ok(c)
        };

        drop(mutex);

        Ok(sfile?)
    } else {
        // There were no fields.
        Err(ServerError::Error { why: "No content uploaded".to_string() })
    }
}

pub async fn get_media(
    Path(path): Path<VirtualPath>,
    State(files): State<FileController>,
) -> ServerResult<Response> {
    if !path.is_dir() {
        let media: Media = files.get_media(&path).await?;

        let stream = media.reader_stream().await?;
        let body = Body::from_stream(stream);
        let mut res = Response::new(body);

        // error should be propogated from the storage.get_media call,
        // since there it has a directory or not check.
        let file_name = path.file_name().expect("Should not have gotten here.");
        
        let mime_type = mime_guess::from_path(&file_name).first_raw().unwrap_or("application/octet-stream");
        res.headers_mut().append(
            header::CONTENT_TYPE, 
            HeaderValue::from_static(
                mime_type
            )
        );
        
        res.headers_mut().append(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(
                &format!("inline; filename=\"{}\"", file_name)
            ).map_err(|_e| ServerError::Error { why: "Parse error".to_string() })?
        );

        res.headers_mut().append(
            header::ACCEPT_RANGES,
            HeaderValue::from_static("bytes")
        );

        Ok(res)
    } else {
        let list = files.list_dir(&path).await?
            .ok_or(ServerError::PathDoesntExist)?;
        Ok(Response::new(Body::from(serde_json::to_string(&list)?)))
    }

}

pub async fn delete_media(
    State(files): State<FileController>,
    Path(path): Path<VirtualPath>,
) -> ServerResult<()> {
    files.delete_sfile(&path).await?;
    
    Ok(())
}