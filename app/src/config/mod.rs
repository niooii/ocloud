use std::{path::PathBuf, process::exit};

use cli::CliConfig;
use inquire::Confirm;
use lazy_static::lazy_static;
pub use settings::Settings;

mod cli;
pub mod settings;
mod util;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    DeserializeError,
    FileReadError { err: String },
    SerializeError,
}

impl From<serde_yaml::Error> for Error {
    fn from(_value: serde_yaml::Error) -> Self {
        Error::DeserializeError
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::FileReadError {
            err: value.to_string(),
        }
    }
}

pub trait YamlConfig: Sized + Clone + for<'de> serde::Deserialize<'de> + serde::Serialize {
    const CONFIG_NAME: &'static str;
    const DEFAULT_YAML: &'static str;

    fn read_or_create_default() -> Result<Self> {
        let path = format!("{}.yaml", Self::CONFIG_NAME);

        let res = util::read_yaml::<Self>(&path);
        if let Err(e) = res {
            match e {
                Error::FileReadError { .. } => {
                    // File doesn't exist, create default from embedded YAML
                    let default: Self = serde_yaml::from_str(Self::DEFAULT_YAML)
                        .map_err(|_| Error::DeserializeError)?;
                    default.save()?;
                    return Ok(default);
                }
                _ => return Err(e),
            }
        }

        res
    }

    fn save(&self) -> Result<()> {
        let path = format!("{}.yaml", Self::CONFIG_NAME);
        util::save_yaml(self, path)
    }
}

fn parse_or_prompt_yaml<T>() -> T
where
    T: YamlConfig,
{
    match T::read_or_create_default() {
        Ok(c) => c,
        Err(e) => match e {
            Error::DeserializeError => {
                let proceed = Confirm::new(&format!(
                    "Error parsing {} config. Restore default and load?",
                    T::CONFIG_NAME
                ))
                .with_default(false)
                .with_help_message(&format!(
                    "File can be found in {}.",
                    CONFIG_DIR.to_string_lossy()
                ))
                .prompt()
                .unwrap_or(false);

                if proceed {
                    let def: T =
                        serde_yaml::from_str(T::DEFAULT_YAML).expect("Invalid default YAML");
                    def.save().expect("Error saving default");
                    def
                } else {
                    exit(1);
                }
            }
            _ => {
                eprintln!("Configuration error: {e:?}");
                exit(1);
            }
        },
    }
}

// Used for making config folders etc
pub const PROGRAM_NAME: &str = "ocloud";

lazy_static! {
    pub static ref CLI_CONFIG: CliConfig = parse_or_prompt_yaml();
    pub static ref SETTINGS: Settings =
        Settings::try_from_configuration().expect("Failed to load configuration");
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
    lazy_static::initialize(&SETTINGS);
}
