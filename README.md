# argocd-lint

Linting argocd manifests recursively.

This tool recursively parses and renders ArgoCD applications and checks for common
misconfigurations.

## Checks

- [x] Check if the applications repoURL is readable by the applications AppProject
- [x] Check if the applications destination namespace is writable by the applications AppProject
- [x] Optionally check if the applications destination namespace exists
- [x] Check cluster and namespaced resource whitelists for missing and unused entries

## Configuration

```yaml
local_repos:
  - repo: git@git.example.com:org/repo
    path: "/some/path/to/local/git/repo"
clusters:
  development:
    entrypoints:
      - "/some/path/to/development-apps.yaml"
    check_namespaces: false
  production:
    entrypoints:
      - "/some/path/to/production-apps.yaml"
    check_namespaces: true
```

The legacy top-level `entrypoints` format remains supported as a cluster named
`default`. Namespace existence checks default to enabled; set `check_namespaces`
to `false` for clusters where namespaces are managed outside the rendered
manifests.

To print a per-project `namespaceResourceWhitelist` derived from the rendered
manifests, use:

```bash
argocd-lint --output-namespaced-resource-whitelist
```

## Usage

```bash
argocd-lint
```

With multiple configured clusters, the default command validates all of them.
Select one with:

```bash
argocd-lint --cluster development
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
