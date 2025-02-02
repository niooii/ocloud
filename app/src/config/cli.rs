use serde::{Deserialize, Serialize};

use super::util;
use super::Config;
use super::Result;
use super::Error;

#[derive(Deserialize, Serialize, Clone)]
pub struct LocalPostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub database: String,
}

impl Default for LocalPostgresConfig {
    fn default() -> Self {
        Self { 
            host: "127.0.0.1".into(), 
            port: 7432, 
            user: "user".into(), 
            pass: "pass".into(), 
            database: "postgres".into(),
        }
    }
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct CliConfig {
    pub server_url: String,
    /// Configuration for starting a local postgres container
    pub local_postgres: LocalPostgresConfig
}

impl Config for CliConfig {
    fn read_or_create_default() -> Result<Self> {
        let path = "cli.toml";

        let res = util::read_toml::<Self>(path);
        if let Err(e) = res {
            match e {
                Error::FileReadError => {
                    let default = Self::default();
                    default.save()?;
                    return Ok(default);
                },
                _ => return Err(e)
            }
        } 

        res
    }

    fn save(&self) -> Result<()> {
        let path = "cli.toml";

        util::save_toml(self, path)
    }
}