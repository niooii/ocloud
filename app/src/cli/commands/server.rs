use std::env::set_var;
use std::path::PathBuf;
use std::time::Duration;

use inquire::Confirm;
use sqlx::postgres::PgConnectOptions;

use crate::config::{CLI_CONFIG, CONFIG_DIR, DATA_DIR, PROGRAM_NAME, SERVER_CONFIG};
use crate::server;
use super::super::subcommands::{ServerCommand};
use super::super::error::CliResult;

pub async fn handler(command: ServerCommand) -> CliResult<()> {
    match command {
        ServerCommand::Run { host, port } => {
            let pg_conf = &SERVER_CONFIG.postgres;
            let connect_opts = PgConnectOptions::new()
                .host(&pg_conf.host)
                .port(pg_conf.port)
                .username(&pg_conf.user)
                .password(&pg_conf.pass)
                .database(&pg_conf.database);
            
            server::run(&host, port, connect_opts).await?;
        },
        ServerCommand::Wipe => {
            let proceed = Confirm::new("All saved data will be wiped. Continue?")
                .with_default(false)
                .with_help_message("This is a destructive action and CANNOT be undone.")
                .prompt()
                .unwrap_or(false);
            
            if proceed {
                let files = server::file_controller().await?;
                files.wipe().await?;
                println!("Finish.");
            } else {
                println!("Exit.");
            }
        },
    }
    Ok(())
}