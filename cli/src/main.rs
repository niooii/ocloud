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

#[derive(Subcommand, Debug)]
enum Commands {
    /// Upload files to the cloud.
    Upload { 
        path: PathBuf,
        /// Preserve the directory structure relative to the cwd.
        #[arg(short = 'p', long = "preserve")]
        preserve: bool,
    },
    /// Set the cloud's base url.
    SetUrl { url: String }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let config_path = dirs::config_dir().unwrap_or_default()
        .join("ocloud")
        .join("config.toml");
    
    let mut config = Config::from_file(&config_path).await?;
    let mut clipboard: Clipboard = Clipboard::new()?;

    // let pb: ProgressBar = ProgressBar::new(2000);
    // for n in 0..2000 {
    //     pb.inc(1);
    // }
    // pb.finish();

    match &cli.command {
        // https://stackoverflow.com/questions/70252995/how-to-monitor-reqwest-client-upload-progress
        Commands::Upload { path, preserve } => {
            if let Err(_) = Url::parse(&config.cloud_url) {
                eprintln!("Error: cloud url is invalid or does not exist.");
                eprintln!("Use the set-url command to set a cloud url.");
                exit(1);
            }

            // Fix url if the trailing slash is missing
            if !config.cloud_url.ends_with("/") {
                config.cloud_url.push_str("/");
                config.save_to(&config_path).await?;
            };

            match upload::upload_file(&Path::new(path), &config).await {
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

            config.cloud_url = url.clone();
            
            if let Err(e) = config.save_to(&config_path).await {
                eprintln!("Failed to save config changes: {e}");
                exit(1);
            }

            println!("Done.");
        }

    }
    
    Ok(())
}
