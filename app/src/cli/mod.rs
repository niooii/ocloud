use std::process::exit;

use crate::config::DATA_DIR;
use crate::config::{self, YamlConfig, CLI_CONFIG, CONFIG_DIR};
use clap::Parser;
use error::CliResult;
use subcommands::SubCommand;
use tracing::error;
mod api;
mod commands;
pub mod error;
mod subcommands;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

pub async fn run() -> CliResult<()> {
    let cli = Cli::parse();

    config::init();

    // let mut clipboard: Clipboard = Clipboard::new()?;

    match cli.command {
        SubCommand::Upload {
            path,
            preserve,
            dir,
        } => {
            let s = commands::upload::handler(path, preserve, dir).await?;
            println!("File can be found at {s}");
        }
        SubCommand::SetUrl { url } => {
            let mut config_new = CLI_CONFIG.clone();
            config_new.server_url = url.to_string().trim_matches('/').into();

            if let Err(e) = config_new.save() {
                error!("Failed to save config changes: {e:?}");
                exit(1);
            }

            println!("Done.");
        }
        SubCommand::Server { command } => {
            commands::server::handler(command).await?;
        }
        SubCommand::Paths => {
            println!("Config files: {}", CONFIG_DIR.to_string_lossy());
            println!("Media and other data: {}", DATA_DIR.to_string_lossy());
        }
    }

    Ok(())
}
