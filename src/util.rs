use yaml_rust2::Yaml;

pub fn get_name(yaml: &Yaml) -> &str {
    yaml["metadata"]["name"].as_str().unwrap()
}

pub fn get_repo_urls(yaml: &Yaml) -> Vec<&str> {
    if let Some(sources) = yaml["spec"]["sources"].as_vec() {
        sources
            .iter()
            .filter_map(|source| source["repoURL"].as_str())
            .collect()
    } else {
        yaml["spec"]["source"]["repoURL"]
            .as_str()
            .into_iter()
            .collect()
    }
}

pub fn get_repo_url(source: &Yaml) -> Option<&str> {
    source["repoURL"].as_str()
}

pub fn get_chart_name(source: &Yaml) -> Option<&str> {
    source["chart"].as_str()
}

#[cfg(test)]
mod tests {
    use super::get_repo_urls;
    use yaml_rust2::YamlLoader;

    #[test]
    fn gets_repo_url_from_single_source_application() {
        let yaml = YamlLoader::load_from_str(
            "spec:\n  source:\n    repoURL: https://example.com/single.git\n",
        )
        .unwrap();

        assert_eq!(
            get_repo_urls(&yaml[0]),
            vec!["https://example.com/single.git"]
        );
    }

    #[test]
    fn gets_repo_urls_from_multi_source_application() {
        let yaml = YamlLoader::load_from_str(
            "spec:\n  sources:\n    - repoURL: https://example.com/chart.git\n    - repoURL: https://example.com/values.git\n",
        )
        .unwrap();

        assert_eq!(
            get_repo_urls(&yaml[0]),
            vec![
                "https://example.com/chart.git",
                "https://example.com/values.git"
            ]
        );
    }

    #[test]
    fn missing_repo_url_does_not_panic() {
        let yaml = YamlLoader::load_from_str("spec: {}\n").unwrap();

        assert!(get_repo_urls(&yaml[0]).is_empty());
    }
}
