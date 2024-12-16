use anyhow::bail;
use fancy::{colorize, eprintcoln};
use crate::model::State;

// checks
// 1. check if all projects that are referenced in applications exist
// 2. check if all namespaces that are referenced in applications exist
// 3. check if all namespaces that are referenced in applications are writable by the project
// 4. check if all source repos are accessible by the applications project
pub fn run_checks(state: &State) -> anyhow::Result<bool> {
    let mut succeeded = true;
    for (name, application) in &state.applications {
        if let Err(err) = run_check(state, name, application) {
            eprintcoln!("[yellow]check failed for application [bold|red]{}[yellow]: {}", name, err);
            succeeded = false;
        }
    }
    Ok(succeeded)
}

pub fn run_check(state: &State, name: &str, application: &crate::argo::Application) -> anyhow::Result<()> {
    let project = application.project.clone();
    let namespace = application.destination_namespace.clone();

    if !state.app_projects.contains_key(&project) {
        bail!(colorize!("project [bold|red]{} [yellow]does not exist", project));
    }

    if !state.namespaces.contains_key(&namespace) {
        bail!(colorize!("namespace [bold|red]{} [yellow]does not exist", namespace));
    }

    let app_project = state.app_projects.get(&project).unwrap();
    let namespace = state.namespaces.get(&namespace).unwrap();

    if !app_project.writable_namespaces().contains(&namespace.name) {
        bail!(colorize!("project [bold|red]{} [yellow]is not allowed to write to namespace [bold|red]{}", project, namespace.name));
    }

    let repo = application.yaml["spec"]["source"]["repoURL"].as_str().unwrap();
    if !app_project.source_repos().contains(&repo.to_owned()) {
        bail!(colorize!("project [bold|red]{} [yellow]does not have access to repo [bold|red]{}", project, repo));
    }
    
    Ok(())
}