use std::path::Path;
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use super::{Error, Result, CONFIG_DIR};

pub fn read_toml<T>(rel_path: impl AsRef<Path> + ToString) -> Result<T>
where T: DeserializeOwned {
    let path = CONFIG_DIR.join(&rel_path);
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