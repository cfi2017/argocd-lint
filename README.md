# argocd-lint

Linting argocd manifests recursively.

This tool recursively parses and renders ArgoCD applications and checks for common
misconfigurations.

## Checks

- [x] Check if the applications repoURL is readable by the applications AppProject
- [x] Check if the applications destination namespace is writable by the applications AppProject
- [x] Check if the applications destination namespace exists

## Configuration

```ỳaml
entrypoints:
  - "/some/path/to/misc-apps.yaml"
local_repos:
  git@git.example.com:org/repo: "/some/path/to/local/git/repo"
```

## Usage

```bash
argocd-lint
```
