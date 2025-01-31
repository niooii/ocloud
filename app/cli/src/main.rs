mod api;
mod commands;
mod error;

use std::{path::{Path, PathBuf}, process::exit, str::FromStr};
use arboard::Clipboard;
use reqwest::Url;
use clap::{Parser, Subcommand};
use error::Result;
use config::{Config, CLI_CONFIG};

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
        /// Preserve the directory structure relative to the cwd when uploading.
        /// Ex: ocloud upload -p ./books/fiction/AM.pdf will be uploaded to
        /// endpoint.com/media/root/books/fiction/AM.pdf
        /// rather than endpoint.com/media/root/AM.pdf without the preserve flag.
        #[arg(short = 'p', long = "preserve")]
        preserve: bool,
        /// The target directory to upload the file to.
        #[arg(short = 'd', long = "dir", default_value = "")]
        dir: String,
    },
    /// Set the cloud's base url.
    SetUrl { url: Url },
    /// Host the cloud with the given parameters.
    Server {
        /// The host to use when starting the server.
        #[arg(short = 'H', long = "host", default_value="0.0.0.0")]
        host: String,
        /// The port to use when starting the server.
        #[arg(short = 'p', long = "port", default_value="443")]
        port: u16 
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    config::init();

    // let mut clipboard: Clipboard = Clipboard::new()?;

    match cli.command {
        Commands::Upload { path, preserve, dir } => {
            let s = commands::upload::handler(path, preserve, dir).await?;
            println!("File can be found at {s}");
        },
        Commands::SetUrl { url } => {
            let mut config_new = CLI_CONFIG.clone();
            config_new.server_url = url.to_string().trim_matches('/').into();
            
            if let Err(e) = config_new.save() {
                eprintln!("Failed to save config changes: {e:?}");
                exit(1);
            }

            println!("Done.");
        },
        Commands::Server { host, port } => {
            server::run(&host, port).await;
        }
    }
    
    Ok(())
}
