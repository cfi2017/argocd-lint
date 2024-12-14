use anyhow::Context;
use argocd_lint::model::State;
use argocd_lint::util::{get_chart_name, get_name, get_repo_url};
use fancy::eprintcoln;
use log::info;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::{File, read_to_string};
use std::hash::Hash;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::Command;
use rayon::prelude::*;
use tempfile::tempfile;
use yaml_rust::{Yaml, YamlLoader};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = argocd_lint::config::Config::load().context("could not load config")?;

    let mut state = State::default();
    state.local_repos = config.local_repos;

    eprintcoln!("loading entrypoints");

    let entrypoints = load_entrypoints(&config.entrypoints);

    parse_yaml(&mut state, entrypoints)?;

    if let Ok(succeeded) = argocd_lint::checks::run_checks(&state) {
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

    eprintcoln!("rendered {} templates", new_templates.len());
    if let Err(err) = parse_yaml(state, new_templates) {
        eprintcoln!("could not parse rendered templates: {}", err);
    }

    Ok(())
}
