use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::util;
use super::Config;
use super::Result;
use super::Error;
use super::DATA_DIR;
use super::PROGRAM_NAME;

#[derive(Deserialize, Serialize, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub database: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self { 
            host: "127.0.0.1".into(), 
            port: 9432, 
            user: "user".into(), 
            pass: "pass".into(), 
            database: PROGRAM_NAME.to_string() 
        }
    }
}

impl PostgresConfig {
    pub fn to_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user,
            self.pass,
            self.host,
            self.port,
            self.database
        )
    }

    pub fn to_url_default_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.user,
            self.pass,
            self.host,
            self.port,
        )
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub files_dir: PathBuf,
    pub max_filesize: Option<usize>,
    pub postgres: PostgresConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            data_dir: DATA_DIR.clone().join("data"),
            files_dir: DATA_DIR.clone().join("files"),
            postgres: Default::default(),
            max_filesize: None
        }
    }
}

impl Config for ServerConfig {
    fn read_or_create_default() -> Result<Self> {
        let path = "server.toml";

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
        let path = "server.toml";

        util::save_toml(self, path)
    }
}