use std::{io::Write, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use axum::{body::Body, extract::{DefaultBodyLimit, Multipart, Path, Query, State}, http::{header, HeaderValue}, response::Response, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::FromRow;
use tokio::{fs::File, io::AsyncWriteExt};
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;
use crate::{cdn_settings, controller::{MediaCheckinResult, MediaDeleteResult}, error::{Error, Result}, model::{Media, MediaAccessInfo, MediaUploadInfo}};

use crate::controller::MediaController;

pub fn routes(mc: MediaController) -> Router{
    Router::new()
        .route(
            "/media", post(upload_media).layer(
            // TODO! file size limit
            DefaultBodyLimit::disable()
        ))
        .route("/media/:file_name", get(get_media).delete(delete_media))
        .route("/ping", get(ping))
        .with_state(mc)
}

async fn upload_media(
    State(mc): State<MediaController>,
    mut multipart: Multipart
) -> Result<String> {

    println!("{:?}", multipart);
    
    // Write the first field to the disk, ignore other fields.
    if let Some(mut field) = multipart.next_field().await
    .map_err(|e| Error::AxumError { why: format!("Multipart error: {}", e.body_text()) })? {
        
        let save_dir = PathBuf::from(&cdn_settings.read().await.save_dir);
        
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

        let info = MediaUploadInfo {
            file_name: name,
            file_size,
            file_hash,
            upload_start_time: uploaded_time
        };
        
        // Check-in file to database
        let checkin_result: MediaCheckinResult = mc.checkin_media(info).await?;

        let uploaded_media: &Media = &checkin_result.media;

        // Ensure the file handle is dropped before doing anything
        // ahem windows
        drop(file);

        if checkin_result.is_duplicate {
            tokio::fs::remove_file(&temp_path).await
                .map_err(|e| Error::IOError { why: e.to_string() })?;
            println!("Removed duplicate file..")
        } else {
            // No duplicate
            // Rename the file to have the proper filename
            let true_path: PathBuf = save_dir.join(
                uploaded_media.true_filename()
            );

            tokio::fs::rename(
                temp_path.as_path(), 
                true_path.as_path()
            ).await.map_err(|e| Error::IOError { why: e.to_string() })?;

            println!("Finalized filename..");  
        }

        Ok(uploaded_media.rel_endpoint())
    } else {
        // There were no fields.
        Err(Error::Error { why: "No content uploaded".to_string() })
    }
}

#[derive(Deserialize)]
struct AccessMediaQueryParams {
    id: i64,
    checksum: String
}

async fn get_media(
    State(mc): State<MediaController>,
    Path(file_name): Path<String>,
    Query(q_params): Query<AccessMediaQueryParams>
) -> Result<Response> {

    let info = MediaAccessInfo {
        id: q_params.id,
        file_hash: q_params.checksum,
        file_name: file_name.clone()
    };

    let media: Media = mc.get_media(&info).await?;

    let stream = media.reader_stream().await?;
    let body = Body::from_stream(stream);
    let mut res = Response::new(body);
    
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
}

async fn delete_media(
    State(mc): State<MediaController>,
    Path(file_name): Path<String>,
    Query(q_params): Query<AccessMediaQueryParams>
) -> Result<Json<Media>> {

    let info = MediaAccessInfo {
        id: q_params.id,
        file_hash: q_params.checksum,
        file_name: file_name.clone()
    };

    let delete_result: MediaDeleteResult = mc.delete_media(&info).await?;

    if delete_result.remaining_references == 0 {
        delete_result.deleted.delete_from_disk().await?;
    }
    
    Ok(Json(delete_result.deleted))
}

async fn ping() -> &'static str {
    "pong...?"
}           
