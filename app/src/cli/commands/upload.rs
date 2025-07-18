use std::path::PathBuf;
use crate::{config::CLI_CONFIG, cli::error::{CliError, CliResult}};
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{multipart::{Form, Part}, Body, Client, Url};
use tokio_util::{bytes::Bytes, io::ReaderStream};
use futures_util::{stream::StreamExt, Stream};
use tracing::{error, trace};

pub async fn handler(path: PathBuf, preserve: bool, dir: String) -> CliResult<String> {
    if let Err(e) = Url::parse(&CLI_CONFIG.server_url) {
        error!("Error: cloud url is invalid or does not exist. Use the set-url command to set a cloud url.");
        return Err(e.into());
    }

    let upload_dir = PathBuf::from(
        format!(
            "root/{}",
            dir.trim_matches('/')
        )
    );

    let upload_path = upload_dir.join({
        if preserve {
            path.clone()
        } else {
            PathBuf::from(path.file_name().unwrap())
        }
    });

    trace!("Uploading file to: {}", upload_path.to_string_lossy());

    upload_file(&upload_path, &path).await
}

// TODO! this should be in api wrapper
pub async fn upload_file(upload_path: &Path, file_path: &Path) -> CliResult<String> {
    trace!("Uploading {:?}...", file_path.file_name().unwrap_or_default());

    let client = Client::new();

    let fname: String = file_path.file_name().unwrap().to_string_lossy().to_string();

    let multipart_form = Form::new()
        .part(fname, Part::stream(Body::wrap_stream(
            upload_stream(file_path).await?
        )));

    let endpoint: String = format!(
        "{}/files/{}/",
        CLI_CONFIG.server_url.clone(),
        upload_path.parent().unwrap().to_string_lossy()
    );

    trace!("Uploading to url {endpoint}");

    let res = client.post(&endpoint)
        .multipart(multipart_form)
        .send().await?;

    if !res.status().is_success() {
        return Err(CliError::FailStatusCode { status_code: res.status() });
    }

    let media_endpoint = res.text().await?.to_string();
    
    Ok(format!("{}/files/{}", CLI_CONFIG.server_url, media_endpoint))
}

async fn upload_stream(path: &Path) 
    -> CliResult<impl Stream<Item = std::result::Result<Bytes, std::io::Error>>> {
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