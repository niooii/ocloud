use inquire::Confirm;

use crate::subcommands::{DbCommand, ServerCommand};
use crate::error::Result;

async fn db_handler(command: DbCommand) -> Result<()> {

    Ok(())
}

pub async fn handler(command: ServerCommand) -> Result<()> {
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
            }
        },
    }
    Ok(())
}