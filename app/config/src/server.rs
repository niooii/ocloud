use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::util;
use crate::Config;
use crate::Result;
use crate::DATA_DIR;

#[derive(Deserialize, Serialize, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub port: String,
    pub user: String,
    pub pass: String,
    pub database: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self { 
            host: "127.0.0.1".into(), 
            port: "9432".into(), 
            user: "user".into(), 
            pass: "pass".into(), 
            database: "postgres".into() 
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub postgres_config: PostgresConfig,
    pub max_filesize: Option<usize>
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            data_dir: DATA_DIR.clone().join("data"),
            postgres_config: Default::default(),
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
                crate::Error::FileReadError => {
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