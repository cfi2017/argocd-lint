use yaml_rust2::Yaml;

pub fn get_name(yaml: &Yaml) -> Option<&str> {
    yaml["metadata"]["name"].as_str()
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

pub fn join_yaml_documents(documents: Vec<String>) -> String {
    documents
        .into_iter()
        .filter(|document| !document.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n---\n")
}

#[cfg(test)]
mod tests {
    use super::{get_name, get_repo_urls, join_yaml_documents};
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

    #[test]
    fn missing_metadata_name_does_not_panic() {
        let yaml = YamlLoader::load_from_str("apiVersion: v1\nkind: List\n").unwrap();

        assert_eq!(get_name(&yaml[0]), None);
    }

    #[test]
    fn joined_manifests_remain_separate_yaml_documents() {
        let joined = join_yaml_documents(vec![
            "apiVersion: v1\nkind: Service\n".to_owned(),
            "apiVersion: apps/v1\nkind: Deployment\n".to_owned(),
        ]);

        assert_eq!(YamlLoader::load_from_str(&joined).unwrap().len(), 2);
    }
}
