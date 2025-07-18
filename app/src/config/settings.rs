use config::{Config, ConfigError, Environment as ConfigEnvironment};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::YamlConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub directories: DirectorySettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BaseConfig {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub directories: DirectorySettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DevelopmentConfig {
    pub application: DevelopmentApplicationSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProductionConfig {
    pub application: ProductionApplicationSettings,
    pub database: ProductionDatabaseSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestingConfig {
    pub application: TestingApplicationSettings,
    pub database: TestingDatabaseSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DevelopmentApplicationSettings {
    pub environment: Environment,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProductionApplicationSettings {
    pub environment: Environment,
    pub host: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProductionDatabaseSettings {
    pub require_ssl: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestingApplicationSettings {
    pub environment: Environment,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestingDatabaseSettings {
    pub database_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
    pub environment: Environment,
    pub max_filesize: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectorySettings {
    pub data_dir: PathBuf,
    pub files_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Environment {
    #[serde(alias="development", alias="dev")]
    Development,
    #[serde(alias="production", alias="prod")]
    Production,
    #[serde(alias="testing", alias="test")]
    Testing,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production", 
            Environment::Testing => "testing",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "production" | "prod" => Ok(Self::Production),
            "testing" | "test" => Ok(Self::Testing),
            other => Err(format!(
                "{} is not a supported environment. Use either `development`, `production`, or `testing`.",
                other
            )),
        }
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password,
            self.host,
            self.port,
            self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.username,
            self.password,
            self.host,
            self.port
        )
    }
}

impl Settings {
    pub fn try_from_configuration() -> Result<Settings, ConfigError> {
        use super::parse_or_prompt_yaml;
        
        let default_env = if cfg!(debug_assertions) { "development" } else { "production" };
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| default_env.into())
            .try_into()
            .map_err(|e| ConfigError::Message(e))?;

        let base_config: BaseConfig = parse_or_prompt_yaml();
        let environment_config = match environment {
            Environment::Development => {
                let dev_config: DevelopmentConfig = parse_or_prompt_yaml();
                serde_yaml::to_string(&dev_config).map_err(|_| ConfigError::Message("Failed to serialize development config".to_string()))?
            },
            Environment::Production => {
                let prod_config: ProductionConfig = parse_or_prompt_yaml();
                serde_yaml::to_string(&prod_config).map_err(|_| ConfigError::Message("Failed to serialize production config".to_string()))?
            },
            Environment::Testing => {
                let test_config: TestingConfig = parse_or_prompt_yaml();
                serde_yaml::to_string(&test_config).map_err(|_| ConfigError::Message("Failed to serialize testing config".to_string()))?
            },
        };

        let base_yaml = serde_yaml::to_string(&base_config).map_err(|_| ConfigError::Message("Failed to serialize base config".to_string()))?;

        let settings = Config::builder()
            .add_source(config::File::from_str(&base_yaml, config::FileFormat::Yaml))
            .add_source(config::File::from_str(&environment_config, config::FileFormat::Yaml))
            .add_source(
                ConfigEnvironment::with_prefix("APP")
                    .prefix_separator("_")
                    .separator("__"),
            )
            .build()?;

        settings.try_deserialize::<Settings>()
    }
}

impl YamlConfig for BaseConfig {
    const CONFIG_NAME: &'static str = "base";
    const DEFAULT_YAML: &'static str = include_str!("../../configuration/base.yaml");
}

impl YamlConfig for DevelopmentConfig {
    const CONFIG_NAME: &'static str = "development";
    const DEFAULT_YAML: &'static str = include_str!("../../configuration/development.yaml");
}

impl YamlConfig for ProductionConfig {
    const CONFIG_NAME: &'static str = "production";
    const DEFAULT_YAML: &'static str = include_str!("../../configuration/production.yaml");
}

impl YamlConfig for TestingConfig {
    const CONFIG_NAME: &'static str = "testing";
    const DEFAULT_YAML: &'static str = include_str!("../../configuration/testing.yaml");
}