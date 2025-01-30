use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::util;
use crate::Config;
use crate::Result;

#[derive(Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub max_filesize: Option<usize>
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::data_dir().unwrap_or_default(),
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