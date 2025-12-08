{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";

    # only used in checks
    nix-darwin.url = "github:nix-darwin/nix-darwin";
    nix-darwin.inputs.nixpkgs.follows = "nixpkgs";

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
          "x86_64-darwin"
          "aarch64-darwin"
        ];
        imports = [
          inputs.flake-parts.flakeModules.easyOverlay
          inputs.treefmt-nix.flakeModule
        ];
        flake = {
          nixosModules.angrr = ./nixos/module.nix;
          darwinModules.angrr = ./darwin/module.nix;
        };
        perSystem =
          {
            config,
            self',
            pkgs,
            system,
            ...
          }:
          let
            inherit (pkgs.stdenv.hostPlatform) isLinux isDarwin;
          in
          {
            packages = {
              angrr = pkgs.callPackage ./package.nix { };
              default = config.packages.angrr;
            };
            overlayAttrs = {
              inherit (config.packages) angrr;
            };
            checks = lib.mkMerge [
              # common checks
              { inherit (self'.packages) angrr; }

              # linux only
              (lib.mkIf isLinux {
                nixos-test-service = pkgs.testers.runNixOSTest {
                  imports = [ ./nixos/tests/angrr.nix ];
                  nodes.machine = {
                    imports = [ self.nixosModules.angrr ];
                  };
                  node.pkgs = lib.mkForce (pkgs.extend (self.overlays.default));
                };
                nixos-test-filter = pkgs.testers.runNixOSTest {
                  imports = [ ./nixos/tests/filter.nix ];
                  node.pkgs = lib.mkForce (pkgs.extend (self.overlays.default));
                };
              })

              (lib.mkIf isDarwin {
                system =
                  (inputs.nix-darwin.lib.darwinSystem {
                    modules = [
                      self.darwinModules.angrr
                      {
                        programs.direnv.enable = true;
                        system.stateVersion = 6; # required by nix-darwin
                      }
                    ];
                    pkgs = pkgs.extend (self.overlays.default);
                  }).system;
              })
            ];
            treefmt = {
              projectRootFile = "flake.nix";
              programs = {
                nixfmt.enable = true;
                rustfmt.enable = true;
                prettier.enable = true;
                taplo.enable = true;
                shellcheck.enable = true;
              };
            };
            devShells.default = pkgs.mkShell {
              inputsFrom = [ self'.packages.angrr ];
              packages = with pkgs; [
                rustup
                rust-analyzer
              ];
            };
          };
      }
    );
}
