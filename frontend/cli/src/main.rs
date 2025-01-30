mod upload;
mod config;
mod api;

use std::{path::{Path, PathBuf}, process::exit, str::FromStr};
use arboard::Clipboard;
use indicatif::ProgressBar;
use reqwest::Url;
use toml::toml;
use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use config::Config;
use tokio::fs::File;
use upload::UploadError;

// Used for making config folders etc
pub const PROGRAM_NAME: &str = "ocloud";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn get_config(config_path: &Path) -> Config {
    Config::from_file(config_path).unwrap()
}

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = {
        get_config(&CONFIG_PATH)
    };
    pub static ref CONFIG_PATH: PathBuf = {
        dirs::config_dir().unwrap_or_default()
        .join("ocloud")
        .join("config.toml")
    };
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Upload files to the cloud.
    Upload { 
        path: PathBuf,
        /// Preserve the directory structure relative to the cwd when uploading.
        /// Ex: ocloud upload -p ./books/fiction/AM.pdf will be uploaded to
        /// endpoint.com/media/books/fiction/AM.pdf
        /// rather than endpoint.com/media/AM.pdf without the preserve flag.
        #[arg(short = 'p', long = "preserve")]
        preserve: bool,
        /// The target directory to upload the file to.
        #[arg(short = 'd', long = "dir")]
        dir: Option<String>,
    },
    /// Set the cloud's base url.
    SetUrl { url: String }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut clipboard: Clipboard = Clipboard::new()?;

    match cli.command {
        Commands::Upload { path, preserve, dir } => {
            // if let Err(_) = Url::parse(&config.cloud_url) {
            //     eprintln!("Error: cloud url is invalid or does not exist.");
            //     eprintln!("Use the set-url command to set a cloud url.");
            //     exit(1);
            // }

            let upload_dir = PathBuf::from(
                format!(
                    "root/{}",
                    dir.map(|d| {
                        d.trim_matches('/').to_string()
                    }).unwrap_or_default()  
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

            match upload::upload_file(&upload_path, &path).await {
                Ok(url) => {
                    println!("{url}");
                    clipboard.set_text(url)?;
                    println!("Copied url to clipboard!");
                }
                Err(e) => {
                    match e {
                        UploadError::NoFileFound => 
                            eprintln!("File does not exist."),
                        UploadError::ReqwestError { err } => 
                            eprintln!("Reqwest error: {}", err),
                        UploadError::FailStatusCode { status_code } => 
                            eprintln!("Request failed with status code: {}", status_code),
                        UploadError::IoError { err } =>
                            eprintln!("IO failed: {err}")
                    }
                    exit(1);
                },
            }
        },

        Commands::SetUrl { url } => {
            // if let Err(_) = Url::parse(url) {
            //     eprintln!("Invalid cloud url: \"{}\". Check the input and try again.", url);
            //     exit(1);
            // } 
            let mut config_new = get_config(&CONFIG_PATH);
            config_new.cloud_url = url.trim_matches('/').into();
            
            if let Err(e) = CONFIG.save_to(&CONFIG_PATH) {
                eprintln!("Failed to save config changes: {e}");
                exit(1);
            }

            println!("Done.");
        }

    }
    
    Ok(())
}
