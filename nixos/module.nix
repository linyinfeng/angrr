{
  # ./angrr.nix should be upstreamed to nixpkgs eventually
  # so the file should be self contained
  imports = [ ./angrr.nix ];
  disabledModules = [ "services/misc/angrr.nix" ];
}
