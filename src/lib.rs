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

pub async fn check(
    config: &Config,
    entrypoints: &[String],
    check_namespaces: bool,
    output_namespaced_resource_whitelist: bool,
) -> anyhow::Result<bool> {

    let mut state = State {
        local_repos: config
            .local_repos
            .iter()
            .map(|r| (r.repo.clone(), r.path.clone()))
            .collect(),
        ..State::default()
    };

    eprintcoln!("local repos:");
    for (url, path) in &state.local_repos {
        eprintcoln!("{} -> {}", url, path);
    }
    eprintcoln!("loading entrypoints");

    let entrypoints = load_entrypoints(entrypoints);

    parse_yaml(&mut state, entrypoints)?;
    
    eprintcoln!("finished rendering");
    eprintcoln!("got {} applications", state.applications.len());
    eprintcoln!("got {} app projects", state.app_projects.len());
    eprintcoln!("got {} namespaces", state.namespaces.len());

    crate::checks::run_checks(
        &state,
        check_namespaces,
        output_namespaced_resource_whitelist,
    )
    
}


fn load_entrypoints(eps: &[String]) -> Vec<Yaml> {
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
            match kind {
                "Application" => {
                    let Some(name) = get_name(document) else {
                        eprintcoln!("[yellow]ignoring Application without metadata.name");
                        continue;
                    };
                    _ = state
                        .applications
                        .insert(name.to_owned(), document.to_owned().into())
                }
                "AppProject" => {
                    let Some(name) = get_name(document) else {
                        eprintcoln!("[yellow]ignoring AppProject without metadata.name");
                        continue;
                    };
                    _ = state
                        .app_projects
                        .insert(name.to_owned(), document.to_owned().into())
                }
                "Namespace" => {
                    let Some(name) = get_name(document) else {
                        eprintcoln!("[yellow]ignoring Namespace without metadata.name");
                        continue;
                    };
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

    let rendered_applications = new_applications.into_iter()
        .map(|name| state.applications.get(&name).unwrap().to_owned())
        .inspect(|app| {
            eprintcoln!("[green]rendering application {}", app.name);
        })
        .par_bridge()
        .filter_map(|app| match app.render(state) {
            Ok(rendered) => Some((app.name, rendered)),
            Err(err) => {
                eprintcoln!("[red]could not render application {}: {:#}", app.name, err);
                None
            }
        })
        .filter_map(|(name, templates)| match YamlLoader::load_from_str(&templates) {
            Ok(documents) => Some((name, documents)),
            Err(err) => {
                eprintcoln!("[red]could not parse rendered YAML: {}", err);
                None
            }
        })
        .collect::<Vec<_>>();

    let mut new_templates = Vec::new();
    for (application, mut manifests) in rendered_applications {
        state
            .rendered_manifests
            .insert(application, manifests.clone());
        new_templates.append(&mut manifests);
    }

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
