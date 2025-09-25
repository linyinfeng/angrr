{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";

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
          {
            packages = {
              angrr = pkgs.angrr.overrideAttrs (old: {
                src = ./.;
                cargoDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
                nativeCheckInputs = (old.nativeCheckInputs or [ ]) ++ [ pkgs.clippy ];
                postCheck = (old.postCheck or "") + ''
                  cargo clippy --all-targets -- --deny warnings
                '';
              });
              # TODO upstream
              angrr-direnv = pkgs.resholve.mkDerivation {
                pname = "angrr-direnv";
                version = "unstable";
                src = ./direnv;
                # nix-direnv like installation
                installPhase = ''
                  runHook preInstall
                  install -m400 -D angrr.sh $out/share/direnv/lib/angrr.sh
                  runHook postInstall
                '';
                solutions = {
                  default = {
                    scripts = [ "share/direnv/lib/angrr.sh" ];
                    interpreter = "none";
                    inputs = [ ]; # use external angrr from PATH
                    fake = {
                      function = [
                        "has"
                        "direnv_layout_dir"
                        "log_error"
                        "log_status"
                      ];
                      external = [
                        "angrr"
                      ];
                    };
                  };
                };
              };
              default = config.packages.angrr;
            };
            overlayAttrs = {
              inherit (config.packages) angrr angrr-direnv;
            };
            checks = {
              inherit (self'.packages) angrr;
              module = pkgs.testers.runNixOSTest {
                imports = [ "${inputs.nixpkgs}/nixos/tests/angrr.nix" ];
                nodes.machine = {
                  imports = [ self.nixosModules.angrr ];
                };
                node.pkgs = lib.mkForce (pkgs.extend (self.overlays.default)).pkgsLinux;
              };
              upstreamModule = pkgs.testers.runNixOSTest {
                imports = [ "${inputs.nixpkgs}/nixos/tests/angrr.nix" ];
                node.pkgs = lib.mkForce (pkgs.extend (self.overlays.default)).pkgsLinux;
              };
            };
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
