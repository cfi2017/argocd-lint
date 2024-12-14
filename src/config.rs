use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub entrypoints: Vec<String>,
    pub local_repos: HashMap<String, String>
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::File::with_name("config.override").required(false))
            .add_source(config::Environment::with_prefix("ADF").separator("__"))
            .build()?;

        cfg.try_deserialize()
    }
}