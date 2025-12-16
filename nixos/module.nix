{ lib, ... }:
{
  # ./angrr.nix should be upstreamed to nixpkgs eventually
  # so the file should be self contained
  imports = [
    ./angrr.nix
    # See https://github.com/NixOS/nixpkgs/pull/471312#discussion_r2623237638
    (lib.mkRenamedOptionModule [ "services" "angrr" "config" ] [ "services" "angrr" "settings" ])
  ];
  disabledModules = [ "services/misc/angrr.nix" ];
}
