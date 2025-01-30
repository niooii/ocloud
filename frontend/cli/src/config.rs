use std::{io::{Read, Write}, path::Path};
use toml::toml;
use serde::{Deserialize, Serialize};
use std::fs::File;
use anyhow::Result;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub cloud_url: String
}

impl Config {
    /// Creates a default configuration file at the specified path.
    pub fn create_at_path(path: &Path) -> Result<Self> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }

        let mut file = File::create(path)?;

        let toml = 
        toml! {
            cloud_url = ""            
        };

        let toml_str = toml::to_string(&toml)?;

        file.write_all(toml_str.as_bytes())?;

        Ok(
            toml::from_str(&toml_str)?
        )
     }

    pub fn from_file(path: &Path) -> Result<Self> {
        // If file doesn't exist create.
        let mut file = if let Ok(f) = File::open(path) {
            f
        } else {
            return Self::create_at_path(path);
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        drop(file);
        
        Ok(
            match toml::from_str::<Config>(&contents) {
                // If deserialize error then just override with default stuff.
                Err(e) => Self::create_at_path(path)?,
                Ok(t) => t
            }
        )
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string(self)?;

        let mut file = File::create(path)?;

        file.write_all(contents.as_bytes())?;

        Ok(())
    }
}