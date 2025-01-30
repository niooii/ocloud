use serde::{Deserialize, Serialize};

use crate::util;
use crate::Config;
use crate::Result;

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct CliConfig {
    pub server_url: String
}

impl Config for CliConfig {
    fn read_or_create_default() -> Result<Self> {
        let path = "cli.toml";

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
        let path = "cli.toml";

        util::save_toml(self, path)
    }
}