use std::path::PathBuf;
use std::time::Duration;

use inquire::Confirm;
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
use pg_embed::postgres::{PgEmbed, PgSettings};

use crate::config::{CLI_CONFIG, CONFIG_DIR, SERVER_CONFIG};
use crate::server;
use crate::subcommands::{ServerCommand};
use crate::error::CliResult;

pub async fn handler(command: ServerCommand) -> CliResult<()> {
    match command {
        ServerCommand::Run { host, port, embedded_db } => {
            let embedded: Option<PgEmbed> = if embedded_db {
                let pg_settings = PgSettings {
                    // Where to store the postgresql database
                    database_dir: PathBuf::from("data/db"),
                    port: CLI_CONFIG.local_postgres.port,
                    user: CLI_CONFIG.local_postgres.user.clone(),
                    password: CLI_CONFIG.local_postgres.pass.clone(),
                    auth_method: PgAuthMethod::Plain,
                    persistent: true,
                    timeout: Some(Duration::from_secs(15)),
                    migration_dir: None,
                };
                let fetch_settings = PgFetchSettings {
                    version: PG_V13,
                    ..Default::default()
                };

                let mut pg = PgEmbed::new(pg_settings, fetch_settings).await
                .expect("Failed to run embedded database");

                println!("Setting up embedded postgreSQL database..");
                pg.setup().await.unwrap();

                pg.start_db().await.unwrap();
                    
                Some(pg)
            } else {
                None
            };

            let db_url: String = embedded.as_ref()
                .map_or(SERVER_CONFIG.postgres.to_url(), |f| {
                    f.full_db_uri("postgres")
                });
            
            server::run(&host, port, &db_url).await?;
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