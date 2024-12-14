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

```

## Usage

```bash
argocd-lint
```
