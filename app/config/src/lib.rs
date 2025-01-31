use std::path::PathBuf;

use cli::CliConfig;
use lazy_static::lazy_static;
use server::ServerConfig;

mod util;
mod cli;
mod server;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug)]
pub enum Error {
    FileReadError,
    DeserializeError,
    SerializeError
}

impl From<toml::de::Error> for Error {
    fn from(_value: toml::de::Error) -> Self {
        Error::DeserializeError
    }
}

impl From<toml::ser::Error> for Error {
    fn from(_value: toml::ser::Error) -> Self {
        Error::SerializeError
    }
}

impl From<std::io::Error> for Error {
    fn from(_value: std::io::Error) -> Self {
        Error::FileReadError
    }
}

pub trait Config: Sized + Clone {
    fn read_or_create_default() -> Result<Self>;
    fn save(&self) -> Result<()>;
}

lazy_static! {
    pub static ref CLI_CONFIG: CliConfig = {
        CliConfig::read_or_create_default().expect("Failed to read/create cli config")
    };
    pub static ref SERVER_CONFIG: ServerConfig = {
        ServerConfig::read_or_create_default().expect("Failed to read/create server config")
    };
    pub static ref CONFIG_DIR: PathBuf = {
        dirs::config_dir().unwrap_or_default().join("ocloud")
    };
    pub static ref DATA_DIR: PathBuf = {
        dirs::data_dir().unwrap_or_default().join("ocloud")
    };
}

pub fn init() {
    lazy_static::initialize(&CLI_CONFIG);
    lazy_static::initialize(&SERVER_CONFIG);
}