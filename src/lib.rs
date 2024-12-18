use std::collections::HashSet;
use std::fs::read_to_string;
use fancy::eprintcoln;
use rayon::prelude::*;
use yaml_rust2::{Yaml, YamlLoader};
use crate::config::Config;
use crate::model::State;
use crate::util::get_name;

pub mod config;
pub mod model;
pub mod checks;
pub mod util;
mod cli;
mod argo;

pub async fn check(config: Config) -> anyhow::Result<()> {

    let mut state = State::default();
    state.local_repos = config.local_repos.iter().map(|r| (r.repo.clone(), r.path.clone())).collect();
    state.config = config.clone();

    eprintcoln!("local repos:");
    for (url, path) in &state.local_repos {
        eprintcoln!("{} -> {}", url, path);
    }
    eprintcoln!("loading entrypoints");

    let entrypoints = load_entrypoints(&config.entrypoints);

    parse_yaml(&mut state, entrypoints)?;
    
    eprintcoln!("finished rendering");
    eprintcoln!("got {} applications", state.applications.len());
    eprintcoln!("got {} app projects", state.app_projects.len());
    eprintcoln!("got {} namespaces", state.namespaces.len());

    if let Ok(succeeded) = crate::checks::run_checks(&state) {
        if !succeeded {
            std::process::exit(1);
        }
    }

    Ok(())
    
}


fn load_entrypoints(eps: &Vec<String>) -> Vec<Yaml> {
    let mut documents = Vec::new();
    for ep in eps {
        let file_content = read_to_string(ep).unwrap();
        let docs = YamlLoader::load_from_str(&file_content).unwrap();
        documents.extend(docs);
    }
    documents
}

fn parse_yaml(state: &mut State, documents: Vec<Yaml>) -> anyhow::Result<()> {
    let applications: HashSet<String> = state.applications.keys().cloned().collect();

    for document in &documents {
        if let Some(kind) = document["kind"].as_str() {
            let name = get_name(document);
            match kind {
                "Application" => {
                    _ = state
                        .applications
                        .insert(name.to_owned(), document.to_owned().into())
                }
                "AppProject" => {
                    _ = state
                        .app_projects
                        .insert(name.to_owned(), document.to_owned().into())
                }
                "Namespace" => {
                    _ = state
                        .namespaces
                        .insert(name.to_owned(), document.to_owned().into())
                }
                _ => continue,
            };
        }
    }

    let new_applications: HashSet<String> = state
        .applications
        .keys()
        .cloned()
        .collect::<HashSet<_>>()
        .difference(&applications)
        .cloned()
        .collect();

    let new_templates = new_applications.into_iter()
        .map(|name| state.applications.get(&name).unwrap().to_owned())
        .inspect(|app| {
            eprintcoln!("[green]rendering application {}", app.name);
        })
        .par_bridge()
        .map(|app| app.render(state))
        .flatten()
        .map(|templates| YamlLoader::load_from_str(&templates))
        .flatten()
        .reduce(|| Vec::new(), |mut acc, mut templates| {
            acc.append(&mut templates);
            acc
        });

    if new_templates.is_empty() {
        return Ok(());
    }
    
    state.yaml.extend(new_templates.clone());

    eprintcoln!("rendered {} templates", new_templates.len());
    if let Err(err) = parse_yaml(state, new_templates) {
        eprintcoln!("could not parse rendered templates: {}", err);
    }

    Ok(())
}
