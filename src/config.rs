use std::collections::BTreeMap;
use std::path::PathBuf;
use serde::Deserialize;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalRepo {
    pub(crate) repo: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub local_repos: Vec<LocalRepo>,
    #[serde(default)]
    pub fuzz: bool,
    #[serde(default)]
    pub clusters: BTreeMap<String, ClusterConfig>,
    #[serde(default = "default_true")]
    pub check_namespaces: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClusterConfig {
    pub entrypoints: Vec<String>,
    #[serde(default = "default_true")]
    pub check_namespaces: bool,
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

    pub fn selected_clusters(
        &self,
        selected: Option<&str>,
    ) -> anyhow::Result<Vec<(String, ClusterConfig)>> {
        if self.clusters.is_empty() {
            if let Some(selected) = selected {
                anyhow::ensure!(
                    selected == "default",
                    "cluster '{}' does not exist (legacy configuration exposes only 'default')",
                    selected
                );
            }
            return Ok(vec![(
                "default".to_owned(),
                ClusterConfig {
                    entrypoints: self.entrypoints.clone(),
                    check_namespaces: self.check_namespaces,
                },
            )]);
        }

        match selected {
            Some(name) => self
                .clusters
                .get(name)
                .cloned()
                .map(|cluster| vec![(name.to_owned(), cluster)])
                .ok_or_else(|| anyhow::anyhow!("cluster '{}' does not exist", name)),
            None => Ok(self
                .clusters
                .iter()
                .map(|(name, cluster)| (name.clone(), cluster.clone()))
                .collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClusterConfig, Config};
    use std::collections::BTreeMap;

    fn multi_cluster_config() -> Config {
        Config {
            entrypoints: Vec::new(),
            local_repos: Vec::new(),
            fuzz: false,
            check_namespaces: true,
            clusters: BTreeMap::from([
                (
                    "development".to_owned(),
                    ClusterConfig {
                        entrypoints: vec!["dev.yaml".to_owned()],
                        check_namespaces: false,
                    },
                ),
                (
                    "production".to_owned(),
                    ClusterConfig {
                        entrypoints: vec!["prod.yaml".to_owned()],
                        check_namespaces: true,
                    },
                ),
            ]),
        }
    }

    #[test]
    fn no_selection_returns_all_clusters() {
        let selected = multi_cluster_config().selected_clusters(None).unwrap();

        assert_eq!(
            selected
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            vec!["development", "production"]
        );
    }

    #[test]
    fn cluster_selection_returns_only_requested_cluster() {
        let selected = multi_cluster_config()
            .selected_clusters(Some("development"))
            .unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].0, "development");
        assert!(!selected[0].1.check_namespaces);
    }
}
