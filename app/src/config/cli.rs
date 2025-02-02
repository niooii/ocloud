use serde::{Deserialize, Serialize};

use super::util;
use super::Config;
use super::Result;
use super::Error;
use super::PROGRAM_NAME;

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct CliConfig {
    pub server_url: String,
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