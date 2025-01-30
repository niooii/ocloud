use std::{path::PathBuf, process::exit};
use crate::error::{Error, Result};
use std::{ffi::{OsStr, OsString}, path::Path};
use config::CLI_CONFIG;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use tokio::{fs::{read, File}, io::AsyncReadExt};
use reqwest::{multipart::{Form, Part}, Body, Client, StatusCode, Url};
use tokio_util::{bytes::Bytes, io::ReaderStream};
use futures_util::{stream::StreamExt, Stream};

pub async fn handler(path: PathBuf, preserve: bool, dir: String) -> Result<String> {
    if let Err(e) = Url::parse(&CLI_CONFIG.server_url) {
        eprintln!("Error: cloud url is invalid or does not exist.");
        eprintln!("Use the set-url command to set a cloud url.");
        return Err(e.into());
    }

    let upload_dir = PathBuf::from(
        format!(
            "root/{}",
            dir.trim_matches('/').to_string()
        )
    );

    let upload_path = upload_dir.join({
        if preserve {
            path.clone()
        } else {
            PathBuf::from(path.file_name().unwrap())
        }
    });

    println!("wiw chat: {}", upload_path.to_string_lossy());

    upload_file(&upload_path, &path).await
}

pub async fn upload_file(upload_path: &Path, file_path: &Path) -> Result<String> {
    println!("Uploading {:?}...", file_path.file_name().unwrap_or_default());

    let client = Client::new();

    let fname: String = file_path.file_name().unwrap().to_string_lossy().to_string();

    let multipart_form = Form::new()
        .part(fname, Part::stream(Body::wrap_stream(
            upload_stream(file_path).await?
        )));

    let endpoint: String = format!(
        "{}/media/{}/",
        CLI_CONFIG.server_url.clone(),
        upload_path.parent().unwrap().to_string_lossy()
    );
    println!("{endpoint}");
    let res = client.post(&endpoint)
        .multipart(multipart_form)
        .send().await?;

    if !res.status().is_success() {
        return Err(Error::FailStatusCode { status_code: res.status() });
    }

    let media_endpoint = res.text().await?.to_string();
    
    Ok(format!("{}/media/{}", CLI_CONFIG.server_url, media_endpoint))
}

pub async fn upload_stream(path: &Path) 
    -> Result<impl Stream<Item = std::result::Result<Bytes, std::io::Error>>> {
    let file = tokio::fs::File::open(path)
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