mod api;
mod commands;
mod error;
mod subcommands;

use std::{path::{Path, PathBuf}, process::exit, str::FromStr};
use arboard::Clipboard;
use reqwest::Url;
use clap::{Parser, Subcommand};
use error::Result;
use config::{Config, CLI_CONFIG};
use subcommands::SubCommand;

// Used for making config folders etc
pub const PROGRAM_NAME: &str = "ocloud";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    config::init();

    // let mut clipboard: Clipboard = Clipboard::new()?;

    match cli.command {
        SubCommand::Upload { path, preserve, dir } => {
            let s = commands::upload::handler(path, preserve, dir).await?;
            println!("File can be found at {s}");
        },
        SubCommand::SetUrl { url } => {
            let mut config_new = CLI_CONFIG.clone();
            config_new.server_url = url.to_string().trim_matches('/').into();
            
            if let Err(e) = config_new.save() {
                eprintln!("Failed to save config changes: {e:?}");
                exit(1);
            }

            println!("Done.");
        },
        SubCommand::Server { command } => {
            commands::server::handler(command).await?;
        }
    }
    
    Ok(())
}
