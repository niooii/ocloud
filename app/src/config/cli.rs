use serde::{Deserialize, Serialize};

use super::YamlConfig;

#[derive(Deserialize, Serialize, Clone)]
pub struct CliConfig {
    pub server_url: String,
}

impl YamlConfig for CliConfig {
    const CONFIG_NAME: &'static str = "cli";
    const DEFAULT_YAML: &'static str = include_str!("../../configuration/cli.yaml");
}
