use std::{io::Write, path::PathBuf, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};
use axum::{body::Body, extract::{Multipart, Path, Query, State}, http::{header, HeaderValue}, response::Response, Json};
use serde::Deserialize;
use tokio::{fs::File, io::AsyncWriteExt, sync::Notify};
use sha2::{Digest, Sha256};
use tokio::fs;

use crate::{error::Error, storage::{controller::{StorageController}, model::{FileUploadInfo, Media, VirtualPath}}, CONFIG};
use crate::error::Result;

pub async fn upload_media(
    State(mc): State<StorageController>, 
    Path(path): Path<VirtualPath>,
    mut multipart: Multipart
) -> Result<String> {
    path.err_if_file()?;

    // Write the first field to the disk, ignore other fields.
    if let Some(mut field) = multipart.next_field().await
    .map_err(|e| Error::AxumError { why: format!("Multipart error: {}", e.body_text()) })? {
        
        let save_dir = PathBuf::from(&CONFIG.read().await.save_dir);
        
        // Name should be the name of the file, including the extension.
        let name: String = field.name().expect("File has no name??").to_string();
        println!("Got file: {name}");

        let uploaded_time: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time itself is against you today, it seems..")
            .as_millis() as i64;

        let mut hasher = Sha256::new();

        let temp_path: PathBuf = save_dir.join(
            format!("./tmp_{uploaded_time}_{name}")
        );

	    let mut file: File = File::create(&temp_path).await
            .map_err(|e| Error::IOError { why: e.to_string() } )?;
        // i64 type because postgres doesnt support unsigned gg
        let mut file_size: i64 = 0;
        while let Some(chunk) = field.chunk().await
            .map_err(|e| Error::AxumError { why: format!("Chunk error: {}", e.body_text()) })? {
            
            file.write_all(&chunk).await.map_err(|e| Error::IOError { why: e.to_string() } )?;
            file_size += chunk.len() as i64;
            hasher.write(&chunk).expect("Failed to hash shit");
        }

	    file.flush().await.expect("Bluh flushing file failed");

        let hash = hasher.finalize();
        let file_hash: String = format!("{:X}", hash);  
        
        let info = FileUploadInfo {
            file_name: name,
            temp_path: temp_path.clone(),
            file_size,
            file_hash: file_hash.clone(),
            vpath: path,
            upload_start_time: uploaded_time
        };
        
        // Ensure the file handle is dropped before doing anything
        // ahem windows
        drop(file);

        // HEHEHEHAW fix race condition 
        // just in case if two people upload the same file at the exact same time down to the millisecond...??
        let mutex = mc.active_uploads.lock(file_hash.clone())
            .await;

        // Check-in file to database
        let vpathstr = match mc.finish_upload(info).await {
            Err(e) => {
                // doesnt really have to be checked
                let _ = fs::remove_file(&temp_path).await;
                println!("(Tried) removed temp file due to db error: {e:?}");
                Err(e)
            }
            Ok(c) => Ok(c)
        };

        drop(mutex);

        Ok(vpathstr?)
    } else {
        // There were no fields.
        Err(Error::Error { why: "No content uploaded".to_string() })
    }
}

pub async fn get_media(
    State(storage): State<StorageController>,
    Path(path): Path<VirtualPath>,
) -> Result<Response> {
    if !path.is_dir() {
        let media: Media = storage.get_media(&path).await?;

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
            ).map_err(|_e| Error::Error { why: "Parse error".to_string() })?
        );

        res.headers_mut().append(
            header::ACCEPT_RANGES,
            HeaderValue::from_static("bytes")
        );

        Ok(res)
    } else {
        let list = storage.fs.list_dir(&path).await?
            .ok_or(Error::PathDoesntExist)?;
        Ok(Response::new(Body::from(serde_json::to_string(&list)?)))
    }

}

pub async fn delete_media(
    State(storage): State<StorageController>,
    Path(path): Path<VirtualPath>,
) -> Result<()> {
    storage.delete_sfile(&path).await?;
    
    Ok(())
}