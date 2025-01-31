use std::{io::{Read, Write}, path::{Path, PathBuf}};
use toml::toml;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use crate::{Error, Result, CONFIG_DIR};

pub fn read_toml<T>(rel_path: impl AsRef<Path>) -> Result<T>
where T: DeserializeOwned {
    let path = CONFIG_DIR.join(rel_path);
    
    toml::from_str(
        &fs::read_to_string(path)?
    ).map_err(Error::from)
}

pub fn save_toml<T>(config: T, rel_path: impl AsRef<Path>) -> Result<()>
where T: Serialize {
    let path = CONFIG_DIR.join(rel_path);
    
    let content = toml::to_string_pretty(&config)?;
    fs::write(path, content)
        .map_err(Error::from)
}