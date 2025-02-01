use inquire::Confirm;

use crate::docker::start_pg_container;
use crate::server;
use crate::subcommands::{DbCommand, ServerCommand};
use crate::error::CliResult;

async fn db_handler(command: DbCommand) -> CliResult<()> {
    match command {
        DbCommand::Start => start_pg_container().await?,
        DbCommand::Stop => todo!(),
    }
    Ok(())
}

pub async fn handler(command: ServerCommand) -> CliResult<()> {
    match command {
        ServerCommand::Run { host, port } => {
            server::run(&host, port).await?;
        },
        ServerCommand::Db { command } => {
            db_handler(command).await?;
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
            } {
                println!("Exit.")
            }
        },
    }
    Ok(())
}