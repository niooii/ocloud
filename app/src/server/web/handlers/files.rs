use std::{io::Write, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use axum::{body::Body, extract::{DefaultBodyLimit, Multipart, Path, State}, http::{header, HeaderValue}, response::Response, routing::{get, patch}, Json, Router};
use serde::Deserialize;
use crate::{config::SETTINGS, server::models::files::SFile};
use tokio::{fs::File, io::AsyncWriteExt};
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::{error, trace};

use crate::server::{controllers::files::FileController, models::files::{FileUploadInfo, Media, VirtualPath}};
use crate::server::error::{ServerError, ServerResult};

pub fn routes(controller: FileController) -> Router {
    Router::new()
        .route(
            "/files/*path", 
            get(get_file_or_list_dir)
            .delete(delete_file)
            .post(upload_or_mk_dirs)
            .layer(
                if let Some(s) = SETTINGS.application.max_filesize {
                    DefaultBodyLimit::max(s)
                } else {
                    DefaultBodyLimit::disable()
                }
            )
        ).route(
            "/files", 
            patch(move_files)
        )
        .with_state(controller)
}

#[derive(Deserialize)]
pub struct MoveInfo {
    pub from: VirtualPath,
    pub to: VirtualPath
}

pub async fn move_files(
    State(files): State<FileController>,
    Json(move_info): Json<MoveInfo>
) -> ServerResult<Json<SFile>> {
    files.mv(&move_info.from, &move_info.to).await
    .map(Json)
}

pub async fn upload_or_mk_dirs(
    State(files): State<FileController>, 
    Path(mut path): Path<VirtualPath>,
    multipart: Option<Multipart>
) -> ServerResult<Json<Vec<SFile>>> {
    path.err_if_file()?;
    // If it was multipart
    if let Some(mut multipart) = multipart {
        if let Some(mut field) = multipart.next_field().await
        .map_err(|e| ServerError::AxumError { message: format!("Multipart error: {}", e.body_text()) })? {
            
            let save_dir = &SETTINGS.directories.files_dir;
            
            // Name should be the name of the file, including the extension.
            let name: String = field.name().expect("File has no name??").to_string();
            // trace!("Got file: {name}");
            // trace!("for path: {}", path.to_string());
            
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
            println!("1");
            let mut file: File = File::create(&temp_path).await
                .map_err(|e| ServerError::IOError { message: e.to_string() } )?;
            // i64 type because postgres doesnt support unsigned gg
println!("2");
            let mut file_size: i64 = 0;
            const PROGRESS_THRESHOLD: u64 = 1024 * 1024; // 1MB
            let mut last_progress_report: u64 = 0;
            println!("3");
            while let Some(chunk) = field.chunk().await
                .map_err(|e| ServerError::AxumError { message: format!("Chunk error: {}", e.body_text()) })? {
                
                file.write_all(&chunk).await.map_err(|e| ServerError::IOError { message: e.to_string() } )?;
                file_size += chunk.len() as i64;
                hasher.write_all(&chunk).expect("Failed to hash shit");

                // Send progress updates every 1MB or so
                if file_size as u64 - last_progress_report >= PROGRESS_THRESHOLD {
                    // Note: We don't have total size available with multipart uploads in axum
                    // So we'll just report current bytes uploaded without percentage
                    last_progress_report = file_size as u64;
                }
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
                Ok(c) => {
                    // Notify upload completion via WebSocket if available
                    // This is handled in the FileController's finish_upload method
                    Ok(c)
                }
            };

            drop(mutex);

            return sfile.map(|s| Json(vec![s]));
        }
    }

    // either no content in multipart or not multipart. thats okay, just make the directory.
    files.make_all_dirs(&path, None).await.map(Json)
}

pub async fn get_file_or_list_dir(
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
                &format!("inline; filename=\"{file_name}\"")
            ).map_err(|_e| ServerError::InternalError { message: "Parse error".to_string() })?
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

pub async fn delete_file(
    State(files): State<FileController>,
    Path(path): Path<VirtualPath>,
) -> ServerResult<()> {
    files.delete_sfile(&path).await?;
    
    Ok(())
}