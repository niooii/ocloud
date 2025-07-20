use super::{Error, Result, CONFIG_DIR};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;

pub fn read_yaml<T>(rel_path: impl AsRef<Path> + ToString) -> Result<T>
where
    T: DeserializeOwned,
{
    let path = CONFIG_DIR.join(&rel_path);
    serde_yaml::from_str(&fs::read_to_string(path)?).map_err(|_| Error::DeserializeError)
}

pub fn save_yaml<T>(config: T, rel_path: impl AsRef<Path>) -> Result<()>
where
    T: Serialize,
{
    let path = CONFIG_DIR.join(rel_path);

    let content = serde_yaml::to_string(&config).map_err(|_| Error::SerializeError)?;
    fs::write(path, content).map_err(Error::from)
}
