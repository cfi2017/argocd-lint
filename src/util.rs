use yaml_rust::Yaml;

pub fn get_name(yaml: &Yaml) -> &str {
    yaml["metadata"]["name"].as_str().unwrap()
}

pub fn get_repo_url(yaml: &Yaml) -> &str {
    yaml["spec"]["source"]["repoURL"].as_str().unwrap()
}

pub fn get_chart_name(yaml: &Yaml) -> &str {
    yaml["spec"]["source"]["chart"].as_str().unwrap()
}
