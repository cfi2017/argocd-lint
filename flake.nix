{
  description = "argocd-lint";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self, nixpkgs, crane }:
    let
      systems = [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          craneLib = crane.mkLib pkgs;
          src = craneLib.cleanCargoSource ./.;

          commonArgs = {
            inherit src;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.libyaml ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          argocd-lint = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.makeWrapper ];
              postInstall = ''
                wrapProgram $out/bin/argocd-lint \
                  --prefix PATH : ${nixpkgs.lib.makeBinPath [ pkgs.kubernetes-helm ]}
              '';
            }
          );
        in
        {
          inherit argocd-lint;
          default = argocd-lint;
        }
      );

      apps = forAllSystems (system: {
        argocd-lint = {
          type = "app";
          program = "${self.packages.${system}.argocd-lint}/bin/argocd-lint";
          meta.description = "Lint Argo CD manifests recursively";
        };
        default = self.apps.${system}.argocd-lint;
      });

      checks = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          craneLib = crane.mkLib pkgs;
          commonArgs = {
            src = craneLib.cleanCargoSource ./.;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.libyaml ];
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          inherit (self.packages.${system}) argocd-lint;

          tests = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets";
            }
          );
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          craneLib = crane.mkLib pkgs;
        in
        {
          default = craneLib.devShell {
            checks = self.checks.${system};
            packages = with pkgs; [
              cargo-edit
              cargo-watch
              git
              kubernetes-helm
              rust-analyzer
            ];
          };
        }
      );

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-tree);
    };
}
