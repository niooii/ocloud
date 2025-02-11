use std::{path::PathBuf, process::exit};

use cli::CliConfig;
use inquire::Confirm;
use lazy_static::lazy_static;
use server::ServerConfig;

mod util;
mod cli;
mod server;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    DeserializeError,
    FileReadError { err: String },
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
    fn from(value: std::io::Error) -> Self {
        Error::FileReadError { err: value.to_string() }
    }
}

pub trait Config: Sized + Clone {
    fn read_or_create_default() -> Result<Self>;
    fn save(&self) -> Result<()>;
}

fn parse_or_prompt<T>(config_name: &str) -> T
where T: Config + Default {
    match T::read_or_create_default() {
        Ok(c) => c,
        Err(e) => {
            match e {
                Error::DeserializeError => {
                    let proceed = Confirm::new(
                        &format!("Error parsing {config_name} config. Restore default and load?")
                        )
                        .with_default(false)
                        .with_help_message(
                            &format!("File can be found in {}.", CONFIG_DIR.to_string_lossy())
                        )
                        .prompt()
                        .unwrap_or(false);

                    if proceed {
                        let def = T::default();
                        def.save().expect("Error saving default");
                        def
                    } else {
                        exit(1);
                    }
                },
                _ => panic!("{e:?}"),
            }
        }
    }
}

// Used for making config folders etc
pub const PROGRAM_NAME: &str = "ocloud";

lazy_static! {
    pub static ref CLI_CONFIG: CliConfig = {
        parse_or_prompt("CLI")
    };
    pub static ref SERVER_CONFIG: ServerConfig = {
        parse_or_prompt("server")
    };
    pub static ref CONFIG_DIR: PathBuf = {
        let dir = dirs::config_dir().unwrap_or_default().join(PROGRAM_NAME);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    };
    pub static ref DATA_DIR: PathBuf = {
        let dir = dirs::data_dir().unwrap_or_default().join(PROGRAM_NAME);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    };
}

pub fn init() {
    lazy_static::initialize(&CLI_CONFIG);
    lazy_static::initialize(&SERVER_CONFIG);
}