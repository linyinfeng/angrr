{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";

    nix-darwin.url = "github:nix-darwin/nix-darwin";
    nix-darwin.inputs.nixpkgs.follows = "nixpkgs";

    nix-github-actions.url = "github:nix-community/nix-github-actions";
    nix-github-actions.inputs.nixpkgs.follows = "nixpkgs";

    flake-compat.url = "github:edolstra/flake-compat";
    flake-compat.flake = false;
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      {
        self,
        inputs,
        lib,
        ...
      }:
      {
        systems = [
          "x86_64-linux"
          "aarch64-linux"
          "aarch64-darwin"
        ];
        imports = [
          inputs.flake-parts.flakeModules.easyOverlay
          inputs.treefmt-nix.flakeModule
        ];
        perSystem =
          {
            config,
            self',
            pkgs,
            ...
          }:
          let
            inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
            sourceRoot = ../..;
          in
          {
            packages = {
              angrr = pkgs.callPackage ../../package.nix { };
              default = config.packages.angrr;
            };
            overlayAttrs = {
              inherit (config.packages) angrr;
            };
            checks = lib.mkMerge [
              { inherit (self'.packages) angrr; }

              (lib.mkIf isLinux (
                let
                  mkTest =
                    file:
                    pkgs.testers.runNixOSTest {
                      imports = [ file ];
                      nodes.machine.imports = [ ../../nixos/module.nix ];
                      node.pkgs = lib.mkForce (pkgs.extend self.overlays.default);
                    };
                in
                {
                  nixos-test-service = mkTest ../../nixos/tests/angrr.nix;
                  nixos-test-filter = mkTest ../../nixos/tests/filter.nix;
                  nixos-test-preset = mkTest ../../nixos/tests/preset.nix;
                  nixos-test-keep-n-per-bucket = mkTest ../../nixos/tests/keep-n-per-bucket.nix;
                  nixos-test-config-file = mkTest ../../nixos/tests/config-file.nix;
                }
              ))

              (lib.mkIf isDarwin {
                system =
                  (inputs.nix-darwin.lib.darwinSystem {
                    modules = [
                      ../../darwin/module.nix
                      {
                        services.angrr = {
                          enable = true;
                          settings = with builtins; fromTOML (readFile ../../etc/example-config.toml);
                        };
                        programs.direnv.enable = true;
                        system.stateVersion = 6;
                      }
                    ];
                    pkgs = pkgs.extend self.overlays.default;
                  }).system;
              })
            ];
            treefmt = {
              projectRoot = sourceRoot;
              projectRootFile = "Cargo.toml";
              programs = {
                nixfmt.enable = true;
                rustfmt.enable = true;
                prettier.enable = true;
                taplo.enable = true;
                shellcheck.enable = true;
                actionlint.enable = true;
              };
              settings.formatter.prettier.excludes = [ "docs/config.md" ];
            };
            devShells.default = pkgs.mkShell {
              inputsFrom = [ self'.packages.angrr ];
              packages = with pkgs; [
                rustup
                rust-analyzer
                go-md2man
                config.treefmt.build.wrapper
              ];
            };
          };
        flake.githubActions = inputs.nix-github-actions.lib.mkGithubMatrix {
          checks = self.checks;
        };
      }
    );
}
