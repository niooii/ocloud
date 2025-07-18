use inquire::Confirm;
use sqlx::postgres::PgConnectOptions;

use crate::config::SETTINGS;
use crate::server;
use super::super::subcommands::ServerCommand;
use super::super::error::CliResult;

pub async fn handler(command: ServerCommand) -> CliResult<()> {
    match command {
        ServerCommand::Run { host, port } => {
            let db_settings = &SETTINGS.database;
            let connect_opts = PgConnectOptions::new()
                .options([("timezone", "UTC")])
                .host(&db_settings.host)
                .port(db_settings.port)
                .username(&db_settings.username)
                .password(&db_settings.password)
                .database(&db_settings.database_name);
            
            server::run(&host, port, connect_opts).await?;
        },
        ServerCommand::Wipe => {
            let proceed = Confirm::new("All saved data will be wiped. Continue?")
                .with_default(false)
                .with_help_message("This is a destructive action and CANNOT be undone.")
                .prompt()
                .unwrap_or(false);
            
            if proceed {
                let files = server::file_controller_no_migrate().await?;
                files.nuke().await?;
                println!("Finish.");
            } else {
                println!("Exit.");
            }
        },
    }
    Ok(())
}