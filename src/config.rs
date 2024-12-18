use std::collections::HashMap;
use std::path::PathBuf;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LocalRepo {
    pub(crate) repo: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub entrypoints: Vec<String>,
    pub local_repos: Vec<LocalRepo>,
    #[serde(default)]
    pub fuzz: bool,
}

impl Config {
    pub fn load(file: Option<PathBuf>) -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::builder();
        
        if let Some(file) = file {
            cfg = cfg.add_source(config::File::from(file));
        }
        
        let cfg = cfg
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::File::with_name("config.override").required(false))
            .add_source(config::Environment::with_prefix("ADF").separator("__"))
            .build()?;

        cfg.try_deserialize()
    }
}