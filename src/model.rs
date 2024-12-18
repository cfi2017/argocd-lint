use yaml_rust2::Yaml;
use std::collections::HashMap;
use crate::config::Config;
use crate::util::get_name;

#[derive(Debug, Default)]
pub struct State {
    pub local_repos: HashMap<String, String>,
    pub applications: HashMap<String, crate::argo::Application>,
    pub app_projects: HashMap<String, AppProject>,
    pub namespaces: HashMap<String, Namespace>,
    pub images: Vec<String>,
    pub yaml: Vec<Yaml>,
    pub config: Config
}


#[derive(Debug, Clone)]
pub struct AppProject {
    pub name: String,
    pub yaml: Yaml,
}

impl AppProject {
    pub fn writable_namespaces(&self) -> Vec<String> {
        self.yaml["spec"]["destinations"].as_vec().unwrap().iter().map(|d| d["namespace"].as_str().unwrap().to_owned()).collect()
    }
    
    pub fn source_repos(&self) -> Vec<String> {
        self.yaml["spec"]["sourceRepos"].as_vec().unwrap().iter().map(|d| d.as_str().unwrap().to_owned()).collect()
    }
}

impl From<Yaml> for AppProject {
    fn from(value: Yaml) -> Self {
        let name = get_name(&value);
        AppProject {
            name: name.to_owned(),
            yaml: value
        }
    }
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub yaml: Yaml,
}

impl From<Yaml> for Namespace {
    fn from(value: Yaml) -> Self {
        let name = get_name(&value);
        Namespace {
            name: name.to_owned(),
            yaml: value
        }
    }
}
