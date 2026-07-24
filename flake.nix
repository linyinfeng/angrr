{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;
      inherit (lib) fix listToAttrs nameValuePair;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      mkPkgs = system: import nixpkgs { inherit system; };
      mkPackages =
        system:
        fix (packages: {
          angrr = (mkPkgs system).callPackage ./package.nix { };
          default = packages.angrr;
        });
    in
    {
      packages = listToAttrs (map (system: nameValuePair system (mkPackages system)) systems);
      checks = self.packages;

      overlays.default = final: _prev: {
        angrr = final.callPackage ./package.nix { };
      };

      nixosModules.angrr = ./nixos/module.nix;
      darwinModules.angrr = ./darwin/module.nix;
    };
}
