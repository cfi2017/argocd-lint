use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use anyhow::Context;
use fancy::eprintcoln;
use yaml_rust2::Yaml;
use crate::model::{State};
use crate::util::{get_chart_name, get_name, get_repo_url};

#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub destination_namespace: String,
    pub project: String,
    pub yaml: Yaml
}

impl From<Yaml> for Application {
    fn from(value: Yaml) -> Self {
        let name = get_name(&value);
        let namespace = value["spec"]["destination"]["namespace"].as_str().unwrap();
        let project = value["spec"]["project"].as_str().unwrap();
        Application {
            name: name.to_owned(),
            destination_namespace: namespace.to_owned(),
            project: project.to_owned(),
            yaml: value
        }
    }
}

impl Application {
    pub fn render(&self, state: &State) -> anyhow::Result<String> {
        let sources = self.yaml["spec"]["sources"].as_vec();
        if let Some(sources) = sources {
            return sources
                .iter()
                .map(|source| self.render_source(source, state))
                .collect::<anyhow::Result<Vec<_>>>()
                .map(|rendered| rendered.concat())
                .context("could not render multi-source application");
        }

        self.render_source(&self.yaml["spec"]["source"], state)
            .context("could not render application")
    }

    fn render_source(&self, source: &Yaml, state: &State) -> anyhow::Result<String> {
        if source["chart"].as_str().is_some() {
            self.render_helm_chart(source)
        } else if source["path"].as_str().is_some() {
            self.render_static_repo(source, state)
        } else {
            // A source with only `ref` supplies Helm values and renders no manifests itself.
            Ok(String::new())
        }
    }

    fn render_static_repo(&self, source: &Yaml, state: &State) -> anyhow::Result<String> {
        let repo_url = get_repo_url(source)
            .context("source with a path is missing repoURL")?;
        if state.local_repos.contains_key(repo_url) {
            let path = state.local_repos.get(repo_url).unwrap();
            let path = Path::new(path);
            let path = path.join(source["path"].as_str().unwrap());
            let files = std::fs::read_dir(path).context("could not read directory")?;
            let mut rendered_templates = String::new();
            for file in files {
                // if file is a directory, skip
                if file.as_ref().unwrap().file_type().unwrap().is_dir() {
                    continue;
                }
                let file = file.context("could not read file")?;
                let file = file.path();
                let mut file = File::open(file).context("could not open file")?;
                let mut contents = String::new();
                file.read_to_string(&mut contents).context("could not read file")?;
                rendered_templates.push_str(&contents);
            }
            Ok(rendered_templates)
        } else {
            eprintcoln!("[red]repo {} is a remote git repo and not found in local repos, skipping", repo_url);
            Ok("".to_string())
        }
    }

    fn render_helm_chart(&self, source: &Yaml) -> anyhow::Result<String> {
        let temp_dir = tempfile::tempdir().context("could not create temporary directory")?;
        let repo_url = get_repo_url(source)
            .context("Helm source is missing repoURL")?;
        let chart = get_chart_name(source)
            .context("Helm source is missing chart")?;
        let helm = &source["helm"];
        let release_name = helm["releaseName"].as_str().unwrap_or(&self.name);
        let values_file = temp_dir.path().join("values.yaml");
        if !helm["values"].is_badvalue() {
            let values = helm["values"].as_str().unwrap();
            let mut file = File::create(&values_file).context("could not create values file")?;
            file.write_all(values.as_bytes()).context("could not write values file")?;
        } else if !helm["valuesObject"].is_badvalue() {
            let values = &helm["valuesObject"];
            let mut values_str = String::new();
            yaml_rust2::YamlEmitter::new(&mut values_str).dump(values).context("could not dump values object")?;
            let mut file = File::create(&values_file).context("could not create values file")?;
            file.write_all(values_str.as_ref()).context("could not write values file")?;
        }

        let result = Command::new("helm")
            .arg("template")
            .arg("--repo")
            .arg(repo_url)
            .arg("-f")
            .arg(values_file)
            .arg(release_name)
            .arg(chart)
            .output()
            .unwrap().stdout;
        let result = String::from_utf8_lossy(&result);
        Ok(result.trim().to_string())
    }
    
}
