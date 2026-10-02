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

## Nix

Enter the Rust development environment with:

```bash
nix develop
```

Build or run the program with `nix build` and `nix run`, respectively. The
packaged program and development shell include Helm, which is used to render
Helm-based Argo CD applications. Run the build, tests, and Clippy checks with
`nix flake check`.
