use anyhow::bail;
use fancy::{colorize, eprintcoln};
use crate::model::State;
use crate::util::get_repo_urls;
use std::collections::{BTreeSet, HashSet};
use yaml_rust2::Yaml;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Resource {
    group: String,
    kind: String,
}

impl Resource {
    fn from_manifest(manifest: &Yaml) -> Option<Self> {
        let api_version = manifest["apiVersion"].as_str()?;
        let kind = manifest["kind"].as_str()?;
        let group = api_version
            .split_once('/')
            .map(|(group, _)| group)
            .unwrap_or("");

        Some(Self {
            group: group.to_owned(),
            kind: kind.to_owned(),
        })
    }

    fn matches(&self, resource: &Resource) -> bool {
        (self.group == "*" || self.group == resource.group)
            && (self.kind == "*" || self.kind == resource.kind)
    }
}

// checks
// 1. check if all projects that are referenced in applications exist
// 2. check if all namespaces that are referenced in applications exist
// 3. check if all namespaces that are referenced in applications are writable by the project
// 4. check if all source repos are accessible by the applications project
pub fn run_checks(
    state: &State,
    check_namespaces: bool,
    output_namespaced_resource_whitelist: bool,
) -> anyhow::Result<bool> {
    let mut succeeded = true;
    for (name, application) in &state.applications {
        if let Err(err) = run_check(state, name, application, check_namespaces) {
            eprintcoln!("[yellow]check failed for application [bold|red]{}[yellow]: {}", name, err);
            succeeded = false;
        }
    }

    if !check_resource_whitelists(state) {
        succeeded = false;
    }

    if !check_namespaced_resource_whitelists(state) {
        succeeded = false;
    }

    if output_namespaced_resource_whitelist {
        print_namespaced_resource_whitelists(state);
    }

    Ok(succeeded)
}

pub fn run_check(
    state: &State,
    _name: &str,
    application: &crate::argo::Application,
    check_namespaces: bool,
) -> anyhow::Result<()> {
    let project = application.project.clone();
    let namespace = application.destination_namespace.clone();

    if !state.app_projects.contains_key(&project) {
        bail!(colorize!("project [bold|red]{} [yellow]does not exist", project));
    }

    if check_namespaces && !state.namespaces.contains_key(&namespace) {
        bail!(colorize!("namespace [bold|red]{} [yellow]does not exist", namespace));
    }

    let app_project = state.app_projects.get(&project).unwrap();

    if !app_project
        .writable_namespaces()
        .iter()
        .any(|allowed| allowed == "*" || allowed == &namespace)
    {
        bail!(colorize!("project [bold|red]{} [yellow]is not allowed to write to namespace [bold|red]{}", project, namespace));
    }

    for repo in get_repo_urls(&application.yaml) {
        if !app_project
            .source_repos()
            .iter()
            .any(|allowed| allowed == "*" || allowed == repo)
        {
            bail!(colorize!("project [bold|red]{} [yellow]does not have access to repo [bold|red]{}", project, repo));
        }
    }
    
    Ok(())
}

fn project_resources(state: &State, project: &str) -> BTreeSet<Resource> {
    state
        .applications
        .values()
        .filter(|application| application.project == project)
        .filter_map(|application| state.rendered_manifests.get(&application.name))
        .flatten()
        .filter_map(Resource::from_manifest)
        .collect()
}

fn project_is_fully_rendered(state: &State, project: &str) -> bool {
    state
        .applications
        .values()
        .filter(|application| application.project == project)
        .all(|application| state.rendered_manifests.contains_key(&application.name))
}

fn custom_cluster_resources(state: &State) -> HashSet<Resource> {
    state
        .rendered_manifests
        .values()
        .flatten()
        .filter(|manifest| manifest["kind"].as_str() == Some("CustomResourceDefinition"))
        .filter(|manifest| manifest["spec"]["scope"].as_str() == Some("Cluster"))
        .filter_map(|manifest| {
            Some(Resource {
                group: manifest["spec"]["group"].as_str()?.to_owned(),
                kind: manifest["spec"]["names"]["kind"].as_str()?.to_owned(),
            })
        })
        .collect()
}

fn is_cluster_resource(resource: &Resource, custom_cluster: &HashSet<Resource>) -> bool {
    const CLUSTER_KINDS: &[&str] = &[
        "APIService",
        "CSIDriver",
        "CSINode",
        "CertificateSigningRequest",
        "ClusterTrustBundle",
        "ClusterRole",
        "ClusterRoleBinding",
        "CustomResourceDefinition",
        "DeviceClass",
        "FlowSchema",
        "GatewayClass",
        "IPAddress",
        "IngressClass",
        "MutatingWebhookConfiguration",
        "Namespace",
        "Node",
        "PersistentVolume",
        "PodSecurityPolicy",
        "PriorityClass",
        "PriorityLevelConfiguration",
        "ResourceSlice",
        "RuntimeClass",
        "ServiceCIDR",
        "StorageClass",
        "ValidatingAdmissionPolicy",
        "ValidatingAdmissionPolicyBinding",
        "ValidatingWebhookConfiguration",
        "VolumeAttachment",
        "VolumeAttributesClass",
    ];

    CLUSTER_KINDS.contains(&resource.kind.as_str()) || custom_cluster.contains(resource)
}

fn configured_whitelist(project: &Yaml, field: &str) -> Vec<Resource> {
    project["spec"][field]
        .as_vec()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(Resource {
                group: item["group"].as_str()?.to_owned(),
                kind: item["kind"].as_str()?.to_owned(),
            })
        })
        .collect()
}

fn check_resource_whitelists(state: &State) -> bool {
    let custom_cluster = custom_cluster_resources(state);
    let mut succeeded = true;

    for project in state.app_projects.values() {
        if !project_is_fully_rendered(state, &project.name) {
            eprintcoln!(
                "[yellow]warning for project [bold]{}[yellow]: resource whitelist checks skipped because not all applications rendered",
                project.name
            );
            continue;
        }
        let rendered: BTreeSet<_> = project_resources(state, &project.name)
            .into_iter()
            .filter(|resource| is_cluster_resource(resource, &custom_cluster))
            .collect();
        let whitelist = configured_whitelist(&project.yaml, "clusterResourceWhitelist");

        for resource in &rendered {
            if !whitelist.iter().any(|entry| entry.matches(resource)) {
                eprintcoln!(
                    "[yellow]check failed for project [bold|red]{}[yellow]: cluster resource [bold|red]{}/{}[yellow] is missing from clusterResourceWhitelist",
                    project.name,
                    resource.group,
                    resource.kind
                );
                succeeded = false;
            }
        }

        for entry in &whitelist {
            if !rendered.iter().any(|resource| entry.matches(resource)) {
                eprintcoln!(
                    "[yellow]warning for project [bold]{}[yellow]: unused clusterResourceWhitelist entry [bold]{}/{}",
                    project.name,
                    entry.group,
                    entry.kind
                );
            }
        }
    }

    succeeded
}

fn check_namespaced_resource_whitelists(state: &State) -> bool {
    let custom_cluster = custom_cluster_resources(state);
    let mut succeeded = true;

    for project in state.app_projects.values() {
        if project.yaml["spec"]["namespaceResourceWhitelist"].is_badvalue() {
            continue;
        }
        if !project_is_fully_rendered(state, &project.name) {
            eprintcoln!(
                "[yellow]warning for project [bold]{}[yellow]: namespace resource whitelist checks skipped because not all applications rendered",
                project.name
            );
            continue;
        }

        let rendered: BTreeSet<_> = project_resources(state, &project.name)
            .into_iter()
            .filter(|resource| !is_cluster_resource(resource, &custom_cluster))
            .collect();
        let whitelist = configured_whitelist(&project.yaml, "namespaceResourceWhitelist");

        for resource in &rendered {
            if !whitelist.iter().any(|entry| entry.matches(resource)) {
                eprintcoln!(
                    "[yellow]check failed for project [bold|red]{}[yellow]: namespaced resource [bold|red]{}/{}[yellow] is missing from namespaceResourceWhitelist",
                    project.name,
                    resource.group,
                    resource.kind
                );
                succeeded = false;
            }
        }

        for entry in &whitelist {
            if !rendered.iter().any(|resource| entry.matches(resource)) {
                eprintcoln!(
                    "[yellow]warning for project [bold]{}[yellow]: unused namespaceResourceWhitelist entry [bold]{}/{}",
                    project.name,
                    entry.group,
                    entry.kind
                );
            }
        }
    }

    succeeded
}

fn print_namespaced_resource_whitelists(state: &State) {
    let custom_cluster = custom_cluster_resources(state);

    for project in state.app_projects.values() {
        if !project_is_fully_rendered(state, &project.name) {
            eprintcoln!(
                "[yellow]warning for project [bold]{}[yellow]: namespaceResourceWhitelist output skipped because not all applications rendered",
                project.name
            );
            continue;
        }
        let resources: BTreeSet<_> = project_resources(state, &project.name)
            .into_iter()
            .filter(|resource| !is_cluster_resource(resource, &custom_cluster))
            .collect();

        println!("# AppProject: {}", project.name);
        if resources.is_empty() {
            println!("namespaceResourceWhitelist: []");
        } else {
            println!("namespaceResourceWhitelist:");
            for resource in resources {
                println!("  - group: {:?}", resource.group);
                println!("    kind: {:?}", resource.kind);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        check_namespaced_resource_whitelists, custom_cluster_resources, is_cluster_resource,
        Resource,
    };
    use crate::argo::Application;
    use crate::model::{AppProject, State};
    use yaml_rust2::YamlLoader;

    #[test]
    fn parses_core_and_grouped_api_versions() {
        let manifests = YamlLoader::load_from_str(
            "apiVersion: v1\nkind: Service\n---\napiVersion: apps/v1\nkind: Deployment\n",
        )
        .unwrap();

        assert_eq!(Resource::from_manifest(&manifests[0]).unwrap().group, "");
        assert_eq!(
            Resource::from_manifest(&manifests[1]).unwrap().group,
            "apps"
        );
    }

    #[test]
    fn whitelist_wildcards_match_resources() {
        let wildcard = Resource {
            group: "rbac.authorization.k8s.io".to_owned(),
            kind: "*".to_owned(),
        };
        let role = Resource {
            group: "rbac.authorization.k8s.io".to_owned(),
            kind: "ClusterRole".to_owned(),
        };

        assert!(wildcard.matches(&role));
    }

    #[test]
    fn rendered_crd_defines_custom_cluster_scope() {
        let manifests = YamlLoader::load_from_str(
            "apiVersion: apiextensions.k8s.io/v1\nkind: CustomResourceDefinition\nspec:\n  group: example.com\n  scope: Cluster\n  names:\n    kind: GlobalThing\n",
        )
        .unwrap();
        let mut state = State::default();
        state
            .rendered_manifests
            .insert("app".to_owned(), manifests);
        let resource = Resource {
            group: "example.com".to_owned(),
            kind: "GlobalThing".to_owned(),
        };

        assert!(is_cluster_resource(
            &resource,
            &custom_cluster_resources(&state)
        ));
    }

    #[test]
    fn existing_namespace_whitelist_must_include_rendered_resources() {
        let project_yaml = YamlLoader::load_from_str(
            "apiVersion: argoproj.io/v1alpha1\nkind: AppProject\nmetadata:\n  name: tenant\nspec:\n  namespaceResourceWhitelist:\n    - group: \"\"\n      kind: Service\n",
        )
        .unwrap()
        .remove(0);
        let application_yaml = YamlLoader::load_from_str(
            "apiVersion: argoproj.io/v1alpha1\nkind: Application\nmetadata:\n  name: app\n",
        )
        .unwrap()
        .remove(0);
        let manifests = YamlLoader::load_from_str(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: app\n",
        )
        .unwrap();
        let mut state = State::default();
        state
            .app_projects
            .insert("tenant".to_owned(), AppProject::from(project_yaml));
        state.applications.insert(
            "app".to_owned(),
            Application {
                name: "app".to_owned(),
                destination_namespace: "tenant".to_owned(),
                project: "tenant".to_owned(),
                yaml: application_yaml,
            },
        );
        state.rendered_manifests.insert("app".to_owned(), manifests);

        assert!(!check_namespaced_resource_whitelists(&state));
    }
}
