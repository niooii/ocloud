use std::{ffi::{OsStr, OsString}, path::Path};
use crate::{config::Config, CONFIG};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use tokio::{fs::{read, File}, io::AsyncReadExt};
use reqwest::{multipart::{Form, Part}, Body, Client, StatusCode};
use tokio_util::{bytes::Bytes, io::ReaderStream};
use futures_util::{stream::StreamExt, Stream};
use tqdm::tqdm_async;

pub enum UploadError {
    NoFileFound,
    IoError { err: String },
    ReqwestError { err: reqwest::Error },
    FailStatusCode { status_code: StatusCode }
}

impl From<std::io::Error> for UploadError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError { err: value.to_string() }
    }
}

pub async fn upload_file(upload_path: &Path, file_path: &Path) -> Result<String, UploadError> {
    println!("Uploading {:?}...", file_path.file_name().unwrap_or_default());

    let client = Client::new();

    let fname: String = file_path.file_name().unwrap().to_string_lossy().to_string();

    let multipart_form = Form::new()
        .part(fname, Part::stream(Body::wrap_stream(
            upload_stream(file_path).await?
        )));

    

    let endpoint: String = format!(
        "{}/media/{}/",
        CONFIG.cloud_url.clone(),
        upload_path.parent().unwrap().to_string_lossy()
    );
    println!("{endpoint}");
    let res = client.post(&endpoint)
        .multipart(multipart_form)
        .send().await.map_err(|e| UploadError::ReqwestError { err: e })?;

    if !res.status().is_success() {
        return Err(UploadError::FailStatusCode { status_code: res.status() });
    }

    let media_endpoint = res.text().await
        .map_err(|e| UploadError::ReqwestError { err: e })?.to_string();
    
    Ok(format!("{}/media/{}", CONFIG.cloud_url, media_endpoint))
}

pub async fn upload_stream(path: &Path) -> Result<impl Stream<Item = Result<Bytes, std::io::Error>>, UploadError> {
    let file = File::open(path)
        .await?;
    let size = file.metadata().await?.len();
    
    let mut reader_stream = ReaderStream::new(file);
    Ok(
        async_stream::stream! {
            let pb = ProgressBar::new(size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap());
            while let Some(chunk) = reader_stream.next().await {
                if let Ok(ref chunk) = chunk {
                    pb.inc(chunk.len() as u64);
                }
                yield chunk;
            }
            pb.finish();
        }
    )
}